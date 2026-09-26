//! Public foreground-program identity and restoring our own template menu target.
//! No third-party control tree, window title or document content is inspected.

use serde::{Deserialize, Serialize};
use std::sync::{Mutex, MutexGuard};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplicationAdapterKind {
    Codex,
    Browser,
    WeChat,
    Generic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowToken {
    application_id: String,
    adapter: ApplicationAdapterKind,
    process_id: u32,
    window_id: u64,
    generation: u64,
}

impl WindowToken {
    #[cfg(test)]
    pub(crate) fn from_identity(
        application_id: String,
        adapter: ApplicationAdapterKind,
        process_id: u32,
        window_id: u64,
        generation: u64,
    ) -> Self {
        Self {
            application_id,
            adapter,
            process_id,
            window_id,
            generation,
        }
    }

    pub fn application_id(&self) -> &str {
        &self.application_id
    }

    pub fn adapter(&self) -> ApplicationAdapterKind {
        self.adapter
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn process_id(&self) -> u32 {
        self.process_id
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ApplicationControlError {
    #[error("当前没有可控制的前台窗口")]
    NoForegroundWindow,
    #[error("当前平台不支持 Windows 应用控制")]
    UnsupportedPlatform,
    #[error("前台应用身份不可识别")]
    IdentityUnavailable,
    #[error("应用控制令牌已失效")]
    StaleToken,
    #[error("公开窗口操作不可用")]
    WindowOperationFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ForegroundIdentity {
    application_id: String,
    adapter: ApplicationAdapterKind,
    process_id: u32,
    window_id: u64,
}

#[derive(Debug, Default)]
struct TokenState {
    current: Option<ForegroundIdentity>,
    generation: u64,
}

impl TokenState {
    fn observe(&mut self, identity: ForegroundIdentity) -> WindowToken {
        if self.current.as_ref() != Some(&identity) {
            self.generation = self.generation.saturating_add(1).max(1);
            self.current = Some(identity.clone());
        }
        WindowToken {
            application_id: identity.application_id,
            adapter: identity.adapter,
            process_id: identity.process_id,
            window_id: identity.window_id,
            generation: self.generation,
        }
    }

    fn invalidate(&mut self) {
        if self.current.take().is_some() {
            self.generation = self.generation.saturating_add(1).max(1);
        }
    }

    fn accepts(&self, token: &WindowToken, identity: &ForegroundIdentity) -> bool {
        self.generation == token.generation
            && self.current.as_ref() == Some(identity)
            && token.application_id == identity.application_id
            && token.adapter == identity.adapter
            && token.process_id == identity.process_id
            && token.window_id == identity.window_id
    }
}

/// Stateful foreground controller.  Callers keep the returned token and pass
/// it back only when restoring the explicitly opened menu target.
#[derive(Debug, Default)]
pub struct ApplicationController {
    state: Mutex<TokenState>,
}

/// Injectable boundary used by the scene runtime.  Tests can provide a fake
/// without touching the desktop, while production uses [`ApplicationController`].
pub trait ApplicationControlBackend: Send + Sync {
    fn identify_foreground(&self) -> Result<WindowToken, ApplicationControlError>;
    fn restore_foreground(&self, token: &WindowToken) -> Result<(), ApplicationControlError>;
}

impl ApplicationController {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn identify_foreground(&self) -> Result<WindowToken, ApplicationControlError> {
        match platform::foreground_identity() {
            Ok(identity) => {
                let token = lock(&self.state).observe(identity);
                Ok(token)
            }
            Err(error) => {
                lock(&self.state).invalidate();
                Err(error)
            }
        }
    }

    pub fn restore_foreground(&self, token: &WindowToken) -> Result<(), ApplicationControlError> {
        lock(&self.state)
            .current
            .as_ref()
            .filter(|identity| {
                identity.process_id == token.process_id && identity.window_id == token.window_id
            })
            .ok_or(ApplicationControlError::StaleToken)?;
        platform::restore_foreground(token)
    }
}

impl ApplicationControlBackend for ApplicationController {
    fn identify_foreground(&self) -> Result<WindowToken, ApplicationControlError> {
        ApplicationController::identify_foreground(self)
    }

    fn restore_foreground(&self, token: &WindowToken) -> Result<(), ApplicationControlError> {
        ApplicationController::restore_foreground(self, token)
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|p| p.into_inner())
}

#[cfg(windows)]
mod platform {
    use super::*;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowThreadProcessId, IsWindow, SetForegroundWindow,
    };

    pub(super) fn foreground_identity() -> Result<ForegroundIdentity, ApplicationControlError> {
        let hwnd = unsafe { GetForegroundWindow() };
        if hwnd.0.is_null() || !unsafe { IsWindow(Some(hwnd)) }.as_bool() {
            return Err(ApplicationControlError::NoForegroundWindow);
        }
        let mut process_id = 0u32;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut process_id)) };
        if process_id == 0 {
            return Err(ApplicationControlError::IdentityUnavailable);
        }
        let executable = crate::app_launcher::process_executable_path(process_id)
            .ok_or(ApplicationControlError::IdentityUnavailable)?;
        let (application_id, adapter) = application_identity_for_executable(&executable);
        Ok(ForegroundIdentity {
            application_id,
            adapter,
            process_id,
            window_id: hwnd.0 as usize as u64,
        })
    }

    pub(super) fn restore_foreground(token: &WindowToken) -> Result<(), ApplicationControlError> {
        let hwnd = HWND(token.window_id as usize as *mut _);
        if !unsafe { IsWindow(Some(hwnd)) }.as_bool() {
            return Err(ApplicationControlError::StaleToken);
        }
        let mut owner = 0;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut owner)) };
        if owner != token.process_id {
            return Err(ApplicationControlError::StaleToken);
        }
        if unsafe { SetForegroundWindow(hwnd) }.as_bool()
            && unsafe { GetForegroundWindow() } == hwnd
        {
            Ok(())
        } else {
            Err(ApplicationControlError::WindowOperationFailed)
        }
    }
}

#[cfg(not(windows))]
mod platform {
    use super::*;
    pub(super) fn foreground_identity() -> Result<ForegroundIdentity, ApplicationControlError> {
        Err(ApplicationControlError::UnsupportedPlatform)
    }
    pub(super) fn restore_foreground(_token: &WindowToken) -> Result<(), ApplicationControlError> {
        Err(ApplicationControlError::UnsupportedPlatform)
    }
}

fn application_identity_for_executable(executable: &str) -> (String, ApplicationAdapterKind) {
    let application_id = crate::app_launcher::application_identity_for_path(executable);
    let adapter = match application_id.as_str() {
        "codex" => ApplicationAdapterKind::Codex,
        "edge" | "chrome" => ApplicationAdapterKind::Browser,
        "wechat" => ApplicationAdapterKind::WeChat,
        _ => ApplicationAdapterKind::Generic,
    };
    (application_id, adapter)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity(application_id: &str, window_id: u64) -> ForegroundIdentity {
        ForegroundIdentity {
            application_id: application_id.to_owned(),
            adapter: ApplicationAdapterKind::Generic,
            process_id: 42,
            window_id,
        }
    }

    #[test]
    fn foreground_generation_changes_only_with_identity() {
        let mut state = TokenState::default();
        let first = state.observe(identity("one", 10));
        let same = state.observe(identity("one", 10));
        let changed = state.observe(identity("two", 11));
        assert_eq!(first.generation(), same.generation());
        assert!(changed.generation() > same.generation());
        assert!(!state.accepts(&first, &identity("one", 10)));
        assert!(state.accepts(&changed, &identity("two", 11)));
    }

    #[test]
    fn invalidation_rejects_old_token() {
        let mut state = TokenState::default();
        let token = state.observe(identity("one", 10));
        state.invalidate();
        assert!(!state.accepts(&token, &identity("one", 10)));
    }

    #[test]
    fn executable_identity_uses_public_process_names() {
        assert_eq!(
            application_identity_for_executable("Codex.exe"),
            ("codex".to_owned(), ApplicationAdapterKind::Codex)
        );
        assert_eq!(
            application_identity_for_executable(
                r"C:\Program Files\WindowsApps\OpenAI.Codex_26.903.8094.0_x64__2p2nqsd0c76g0\app\ChatGPT.exe"
            ),
            ("codex".to_owned(), ApplicationAdapterKind::Codex)
        );
        assert_eq!(
            application_identity_for_executable(
                r"C:\Program Files\WindowsApps\OpenAI.ChatGPT-Desktop_1.2.3.0_x64__2p2nqsd0c76g0\app\ChatGPT.exe"
            ),
            (
                r"c:\program files\windowsapps\openai.chatgpt-desktop_1.2.3.0_x64__2p2nqsd0c76g0\app\chatgpt.exe".to_owned(),
                ApplicationAdapterKind::Generic,
            )
        );
        assert_eq!(
            application_identity_for_executable("Weixin.exe"),
            ("wechat".to_owned(), ApplicationAdapterKind::WeChat)
        );
        assert_eq!(
            application_identity_for_executable(r"C:\Program Files\Microsoft\Edge\msedge.exe").1,
            ApplicationAdapterKind::Browser
        );
        assert_eq!(
            application_identity_for_executable(r"D:\Tools\Reader.exe").0,
            r"d:\tools\reader.exe"
        );
    }
}
