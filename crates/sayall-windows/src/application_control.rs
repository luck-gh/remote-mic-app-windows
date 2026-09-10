//! Public Windows application-control adapters.
//!
//! The controller deliberately treats UI Automation as capability evidence,
//! rather than assuming that a process name implies a usable control.  It does
//! not read control names or document values.  A semantic action runs only
//! while the original foreground-window token is still current.

use crate::templates::{ControlRegion, SemanticAction};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FocusRegion {
    ApplicationList,
    Content,
    Input,
    ImeCandidate,
    Modal,
    Unknown,
}

impl FocusRegion {
    pub fn control_region(self) -> Option<ControlRegion> {
        match self {
            Self::ApplicationList => Some(ControlRegion::ApplicationList),
            Self::Content => Some(ControlRegion::Content),
            Self::Input => Some(ControlRegion::Input),
            Self::ImeCandidate | Self::Modal | Self::Unknown => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityState {
    Available,
    Unavailable,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityReason {
    PublicShortcut,
    UiaPattern,
    MissingUiaPattern,
    UnsupportedApplication,
    WrongRegion,
    UnknownFocus,
    ImeCandidateActive,
    ModalActive,
    SensitiveInput,
    AmbiguousTarget,
    InternalRuntimeAction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionCapability {
    pub action: SemanticAction,
    pub state: CapabilityState,
    pub reason: CapabilityReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilitySnapshot {
    pub application_id: String,
    pub adapter: ApplicationAdapterKind,
    pub generation: u64,
    pub focus: FocusSnapshot,
    pub actions: Vec<ActionCapability>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusSnapshot {
    pub region: FocusRegion,
    pub control_region: Option<ControlRegion>,
    pub generation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionResult {
    Performed,
    Unavailable,
    Blocked,
    Stale,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionOutcome {
    pub action: SemanticAction,
    pub result: ActionResult,
    pub reason: Option<CapabilityReason>,
    pub generation: u64,
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
    #[error("公开 UI Automation 接口不可用")]
    AutomationUnavailable,
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
/// it back for every capability query and action.  Moving to another window
/// invalidates the old token before an action is dispatched.
#[derive(Debug, Default)]
pub struct ApplicationController {
    state: Mutex<TokenState>,
}

/// Injectable boundary used by the scene runtime.  Tests can provide a fake
/// without touching the desktop, while production uses [`ApplicationController`].
pub trait ApplicationControlBackend: Send + Sync {
    fn identify_foreground(&self) -> Result<WindowToken, ApplicationControlError>;
    fn classify_focus(&self, token: &WindowToken)
        -> Result<FocusSnapshot, ApplicationControlError>;
    fn capabilities(
        &self,
        token: &WindowToken,
    ) -> Result<CapabilitySnapshot, ApplicationControlError>;
    fn perform(&self, token: &WindowToken, action: SemanticAction) -> ActionOutcome;
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
                log_observation(&token, "identified", None);
                Ok(token)
            }
            Err(error) => {
                lock(&self.state).invalidate();
                Err(error)
            }
        }
    }

    pub fn classify_focus(
        &self,
        token: &WindowToken,
    ) -> Result<FocusSnapshot, ApplicationControlError> {
        self.validate(token)?;
        let evidence = platform::probe_evidence(token)?;
        log_observation(token, "focus_classified", Some(evidence.region));
        Ok(FocusSnapshot {
            region: evidence.region,
            control_region: evidence.region.control_region(),
            generation: token.generation,
        })
    }

    pub fn capabilities(
        &self,
        token: &WindowToken,
    ) -> Result<CapabilitySnapshot, ApplicationControlError> {
        self.validate(token)?;
        let evidence = platform::probe_evidence(token).unwrap_or_default();
        let focus = FocusSnapshot {
            region: evidence.region,
            control_region: evidence.region.control_region(),
            generation: token.generation,
        };
        let snapshot = CapabilitySnapshot {
            application_id: token.application_id.clone(),
            adapter: token.adapter,
            generation: token.generation,
            focus,
            actions: all_actions()
                .into_iter()
                .map(|action| capability_for(token.adapter, &evidence, action))
                .collect(),
        };
        log_capabilities(token, &snapshot);
        Ok(snapshot)
    }

    pub fn perform(&self, token: &WindowToken, action: SemanticAction) -> ActionOutcome {
        if self.validate(token).is_err() {
            log_action(token, &action, ActionResult::Stale, None);
            return outcome(token, action, ActionResult::Stale, None);
        }
        let evidence = match platform::probe_evidence(token) {
            Ok(evidence) => evidence,
            Err(_) => Evidence::default(),
        };
        let capability = capability_for(token.adapter, &evidence, action.clone());
        if capability.state != CapabilityState::Available {
            let result = if capability.state == CapabilityState::Blocked {
                ActionResult::Blocked
            } else {
                ActionResult::Unavailable
            };
            log_action(token, &action, result, Some(capability.reason));
            return outcome(token, action, result, Some(capability.reason));
        }
        // Recheck immediately before a state-changing public API call.
        if self.validate(token).is_err() {
            log_action(token, &action, ActionResult::Stale, None);
            return outcome(token, action, ActionResult::Stale, None);
        }
        let result = if platform::perform(token, &evidence, &action).is_ok() {
            ActionResult::Performed
        } else {
            ActionResult::Failed
        };
        log_action(token, &action, result, Some(capability.reason));
        outcome(token, action, result, Some(capability.reason))
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

    fn validate(&self, token: &WindowToken) -> Result<(), ApplicationControlError> {
        let identity = platform::foreground_identity()?;
        if lock(&self.state).accepts(token, &identity) {
            Ok(())
        } else {
            Err(ApplicationControlError::StaleToken)
        }
    }
}

impl ApplicationControlBackend for ApplicationController {
    fn identify_foreground(&self) -> Result<WindowToken, ApplicationControlError> {
        ApplicationController::identify_foreground(self)
    }

    fn classify_focus(
        &self,
        token: &WindowToken,
    ) -> Result<FocusSnapshot, ApplicationControlError> {
        ApplicationController::classify_focus(self, token)
    }

    fn capabilities(
        &self,
        token: &WindowToken,
    ) -> Result<CapabilitySnapshot, ApplicationControlError> {
        ApplicationController::capabilities(self, token)
    }

    fn perform(&self, token: &WindowToken, action: SemanticAction) -> ActionOutcome {
        ApplicationController::perform(self, token, action)
    }

    fn restore_foreground(&self, token: &WindowToken) -> Result<(), ApplicationControlError> {
        ApplicationController::restore_foreground(self, token)
    }
}

#[derive(Debug, Clone, Default)]
struct Evidence {
    region: FocusRegion,
    modal: bool,
    ime_candidate: bool,
    sensitive_input: bool,
    list_protocol: bool,
    focused_invokable: bool,
    input_target: bool,
    content_target: bool,
    scroll_target: bool,
    send_target: bool,
    zoom_target: bool,
    parent_item: bool,
    expandable_item: bool,
}

impl Default for FocusRegion {
    fn default() -> Self {
        Self::Unknown
    }
}

fn capability_for(
    adapter: ApplicationAdapterKind,
    evidence: &Evidence,
    action: SemanticAction,
) -> ActionCapability {
    use CapabilityReason::*;
    use CapabilityState::*;
    use SemanticAction::*;

    if evidence.modal && !matches!(action, Escape | VolumeUp | VolumeDown) {
        return capability(action, Blocked, ModalActive);
    }
    if evidence.ime_candidate && matches!(action, Send) {
        return capability(action, Blocked, ImeCandidateActive);
    }
    if evidence.sensitive_input && matches!(action, NativeEnter | Newline | Send | Backspace) {
        return capability(action, Blocked, SensitiveInput);
    }

    let (state, reason) = match action {
        Disabled => (Unavailable, InternalRuntimeAction),
        OpenApplicationMenu | OpenAdjustmentMenu => (Unavailable, InternalRuntimeAction),
        VolumeUp | VolumeDown | Escape => (Available, PublicShortcut),
        PreviousTab | NextTab if adapter == ApplicationAdapterKind::Browser => {
            (Available, PublicShortcut)
        }
        BrowserBack if adapter == ApplicationAdapterKind::Browser => match evidence.region {
            FocusRegion::Content => (Available, PublicShortcut),
            FocusRegion::Unknown => (Blocked, UnknownFocus),
            _ => (Blocked, WrongRegion),
        },
        FocusInput if adapter == ApplicationAdapterKind::Browser => (Available, PublicShortcut),
        FocusInput if evidence.input_target => (Available, UiaPattern),
        FocusContent if evidence.content_target => (Available, UiaPattern),
        FocusApplicationList if evidence.list_protocol => (Available, UiaPattern),
        SelectPrevious | SelectNext | CancelSelection if evidence.list_protocol => {
            (Available, UiaPattern)
        }
        SelectParent if evidence.list_protocol && evidence.parent_item => (Available, UiaPattern),
        ExpandSelection if evidence.list_protocol && evidence.expandable_item => {
            (Available, UiaPattern)
        }
        ActivateSelection if evidence.focused_invokable => (Available, UiaPattern),
        ScrollUp | ScrollDown | PageUp | PageDown if evidence.scroll_target => {
            (Available, UiaPattern)
        }
        NativeEnter | Newline | Backspace
            if matches!(
                evidence.region,
                FocusRegion::Input | FocusRegion::ImeCandidate
            ) =>
        {
            (Available, PublicShortcut)
        }
        Send if evidence.region == FocusRegion::Input && evidence.send_target => {
            (Available, UiaPattern)
        }
        ZoomIn | ZoomOut if evidence.zoom_target => (Available, UiaPattern),
        PreviousTab | NextTab | BrowserBack => (Unavailable, UnsupportedApplication),
        FocusInput | FocusContent | FocusApplicationList | SelectPrevious | SelectNext
        | SelectParent | ExpandSelection | ActivateSelection | CancelSelection | ScrollUp
        | ScrollDown | NativeEnter | Newline | Send | Backspace | PageUp | PageDown | ZoomIn
        | ZoomOut => {
            if evidence.region == FocusRegion::Unknown {
                (Blocked, UnknownFocus)
            } else {
                (Unavailable, MissingUiaPattern)
            }
        }
    };
    capability(action, state, reason)
}

fn capability(
    action: SemanticAction,
    state: CapabilityState,
    reason: CapabilityReason,
) -> ActionCapability {
    ActionCapability {
        action,
        state,
        reason,
    }
}

fn all_actions() -> Vec<SemanticAction> {
    use SemanticAction::*;
    vec![
        Disabled,
        SelectPrevious,
        SelectNext,
        SelectParent,
        ExpandSelection,
        ActivateSelection,
        CancelSelection,
        FocusApplicationList,
        FocusContent,
        FocusInput,
        ScrollUp,
        ScrollDown,
        PreviousTab,
        NextTab,
        BrowserBack,
        NativeEnter,
        Newline,
        Send,
        Backspace,
        PageUp,
        PageDown,
        ZoomIn,
        ZoomOut,
        VolumeUp,
        VolumeDown,
        OpenApplicationMenu,
        OpenAdjustmentMenu,
        Escape,
    ]
}

fn outcome(
    token: &WindowToken,
    action: SemanticAction,
    result: ActionResult,
    reason: Option<CapabilityReason>,
) -> ActionOutcome {
    ActionOutcome {
        action,
        result,
        reason,
        generation: token.generation,
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(windows)]
fn log_action(
    token: &WindowToken,
    action: &SemanticAction,
    result: ActionResult,
    reason: Option<CapabilityReason>,
) {
    crate::gatt_note(format!(
        "application_control adapter={:?} action={:?} result={:?} reason={:?} generation={}",
        token.adapter, action, result, reason, token.generation
    ));
}

#[cfg(windows)]
fn log_observation(token: &WindowToken, event: &str, region: Option<FocusRegion>) {
    crate::gatt_note(format!(
        "application_control event={event} adapter={:?} region={region:?} generation={}",
        token.adapter, token.generation
    ));
}

#[cfg(not(windows))]
fn log_observation(_token: &WindowToken, _event: &str, _region: Option<FocusRegion>) {}

#[cfg(windows)]
fn log_capabilities(token: &WindowToken, snapshot: &CapabilitySnapshot) {
    let available = snapshot
        .actions
        .iter()
        .filter(|capability| capability.state == CapabilityState::Available)
        .count();
    let blocked = snapshot
        .actions
        .iter()
        .filter(|capability| capability.state == CapabilityState::Blocked)
        .count();
    crate::gatt_note(format!(
        "application_control event=capabilities adapter={:?} region={:?} available={} blocked={} generation={}",
        token.adapter, snapshot.focus.region, available, blocked, token.generation
    ));
}

#[cfg(not(windows))]
fn log_capabilities(_token: &WindowToken, _snapshot: &CapabilitySnapshot) {}

#[cfg(not(windows))]
fn log_action(
    _token: &WindowToken,
    _action: &SemanticAction,
    _result: ActionResult,
    _reason: Option<CapabilityReason>,
) {
}

#[cfg(windows)]
mod platform {
    use super::*;
    use crate::send_input::{KeyChord, KeyCode};
    use windows::core::Interface;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
        COINIT_MULTITHREADED,
    };
    use windows::Win32::UI::Accessibility::{
        CUIAutomation, IUIAutomation, IUIAutomation2, IUIAutomationElement,
        IUIAutomationExpandCollapsePattern, IUIAutomationInvokePattern,
        IUIAutomationScrollItemPattern, IUIAutomationScrollPattern,
        IUIAutomationSelectionItemPattern, IUIAutomationTransformPattern2, ScrollAmount,
        ScrollAmount_LargeDecrement, ScrollAmount_LargeIncrement, ScrollAmount_NoAmount,
        ScrollAmount_SmallDecrement, ScrollAmount_SmallIncrement, TreeScope_Descendants,
        UIA_DocumentControlTypeId, UIA_EditControlTypeId, UIA_ExpandCollapsePatternId,
        UIA_InvokePatternId, UIA_ScrollItemPatternId, UIA_ScrollPatternId,
        UIA_SelectionItemPatternId, UIA_TransformPattern2Id,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowThreadProcessId, IsWindow, SetForegroundWindow,
    };

    const UIA_TIMEOUT_MS: u32 = 750;

    struct ComGuard(bool);

    impl Drop for ComGuard {
        fn drop(&mut self) {
            if self.0 {
                unsafe { CoUninitialize() };
            }
        }
    }

    struct UiaSession {
        automation: IUIAutomation,
        root: IUIAutomationElement,
        focused: Option<IUIAutomationElement>,
        elements: Vec<IUIAutomationElement>,
    }

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
        let executable = process_executable_name(process_id)
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
        if unsafe { SetForegroundWindow(hwnd) }.as_bool() {
            Ok(())
        } else {
            Err(ApplicationControlError::AutomationUnavailable)
        }
    }

    pub(super) fn probe_evidence(token: &WindowToken) -> Result<Evidence, ApplicationControlError> {
        let (_guard, session) = UiaSession::connect(token)?;
        session.evidence()
    }

    pub(super) fn perform(
        token: &WindowToken,
        evidence: &Evidence,
        action: &SemanticAction,
    ) -> Result<(), ApplicationControlError> {
        use SemanticAction::*;
        match action {
            VolumeUp => tap(&[KeyCode::VolumeUp]),
            VolumeDown => tap(&[KeyCode::VolumeDown]),
            Escape => tap(&[KeyCode::Escape]),
            PreviousTab if token.adapter == ApplicationAdapterKind::Browser => {
                tap(&[KeyCode::Control, KeyCode::PageUp])
            }
            NextTab if token.adapter == ApplicationAdapterKind::Browser => {
                tap(&[KeyCode::Control, KeyCode::PageDown])
            }
            BrowserBack if token.adapter == ApplicationAdapterKind::Browser => {
                tap(&[KeyCode::Alt, KeyCode::Left])
            }
            FocusInput if token.adapter == ApplicationAdapterKind::Browser => {
                tap(&[KeyCode::Control, KeyCode::L])
            }
            NativeEnter | Newline => tap(&[KeyCode::Enter]),
            Backspace => tap(&[KeyCode::Backspace]),
            _ => {
                let (_guard, session) = UiaSession::connect(token)?;
                session.perform(evidence, action)
            }
        }
    }

    fn tap(keys: &[KeyCode]) -> Result<(), ApplicationControlError> {
        crate::send_input_windows::SendInputRuntime::new()
            .tap(KeyChord {
                keys: keys.to_vec(),
            })
            .map(|_| ())
            .map_err(|_| ApplicationControlError::AutomationUnavailable)
    }

    impl UiaSession {
        fn connect(token: &WindowToken) -> Result<(ComGuard, Self), ApplicationControlError> {
            let initialized = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.is_ok();
            let guard = ComGuard(initialized);
            let automation: IUIAutomation = unsafe {
                CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
                    .map_err(|_| ApplicationControlError::AutomationUnavailable)?
            };
            if let Ok(automation2) = automation.cast::<IUIAutomation2>() {
                unsafe {
                    let _ = automation2.SetConnectionTimeout(UIA_TIMEOUT_MS);
                    let _ = automation2.SetTransactionTimeout(UIA_TIMEOUT_MS);
                }
            }
            let hwnd = HWND(token.window_id as usize as *mut core::ffi::c_void);
            let root = unsafe { automation.ElementFromHandle(hwnd) }
                .map_err(|_| ApplicationControlError::AutomationUnavailable)?;
            let focused = unsafe { automation.GetFocusedElement() }
                .ok()
                .filter(|element| is_descendant(&automation, &root, element));
            let condition = unsafe { automation.CreateTrueCondition() }
                .map_err(|_| ApplicationControlError::AutomationUnavailable)?;
            let array = unsafe { root.FindAll(TreeScope_Descendants, &condition) }
                .map_err(|_| ApplicationControlError::AutomationUnavailable)?;
            let length = unsafe { array.Length() }.unwrap_or(0).clamp(0, 20_000);
            let mut elements = Vec::with_capacity(length as usize);
            for index in 0..length {
                if let Ok(element) = unsafe { array.GetElement(index) } {
                    elements.push(element);
                }
            }
            Ok((
                guard,
                Self {
                    automation,
                    root,
                    focused,
                    elements,
                },
            ))
        }

        fn evidence(&self) -> Result<Evidence, ApplicationControlError> {
            let modal = self.has_modal();
            let outside_focus = unsafe { self.automation.GetFocusedElement() }.ok();
            let ime_candidate = outside_focus
                .as_ref()
                .filter(|focused| !is_descendant(&self.automation, &self.root, focused))
                .and_then(|focused| unsafe { focused.CurrentProcessId() }.ok())
                .and_then(|pid| process_executable_name(pid as u32))
                .is_some_and(|name| is_ime_executable(&name));
            let region = if modal {
                FocusRegion::Modal
            } else if ime_candidate {
                FocusRegion::ImeCandidate
            } else {
                self.focus_region()
            };
            let sensitive_input = self.focused.as_ref().is_some_and(|element| {
                unsafe { element.CurrentIsPassword() }.is_ok_and(|v| v.as_bool())
            });
            let list_protocol = self.list_context().is_some();
            let focused_invokable = if region == FocusRegion::ApplicationList {
                self.focused_list_item().is_some()
            } else {
                self.focused_or_ancestor(|element| {
                    has_pattern::<IUIAutomationInvokePattern>(element, UIA_InvokePatternId)
                })
                .is_some()
            };
            let input_target = self.unique_input().is_some();
            let content_target = self.unique_content().is_some();
            let scroll_target = self.scroll_target().is_some();
            let send_target = self.send_target().is_some();
            let zoom_target = self.zoom_target().is_some();
            let parent_item = self.parent_selection_item().is_some();
            let expandable_item = self.focused_list_item().is_some_and(|element| {
                has_pattern::<IUIAutomationExpandCollapsePattern>(
                    &element,
                    UIA_ExpandCollapsePatternId,
                )
            });
            Ok(Evidence {
                region,
                modal,
                ime_candidate,
                sensitive_input,
                list_protocol,
                focused_invokable,
                input_target,
                content_target,
                scroll_target,
                send_target,
                zoom_target,
                parent_item,
                expandable_item,
            })
        }

        fn perform(
            &self,
            _evidence: &Evidence,
            action: &SemanticAction,
        ) -> Result<(), ApplicationControlError> {
            use SemanticAction::*;
            match action {
                FocusApplicationList => self.focus_active_list_item(),
                FocusContent => self
                    .unique_content()
                    .ok_or(ApplicationControlError::AutomationUnavailable)
                    .and_then(set_focus),
                FocusInput => self
                    .unique_input()
                    .ok_or(ApplicationControlError::AutomationUnavailable)
                    .and_then(set_focus),
                SelectPrevious => self.move_list_focus(-1),
                SelectNext => self.move_list_focus(1),
                SelectParent => self
                    .parent_selection_item()
                    .ok_or(ApplicationControlError::AutomationUnavailable)
                    .and_then(set_focus),
                ExpandSelection => {
                    let element = self
                        .focused_list_item()
                        .ok_or(ApplicationControlError::AutomationUnavailable)?;
                    let pattern: IUIAutomationExpandCollapsePattern =
                        unsafe { element.GetCurrentPatternAs(UIA_ExpandCollapsePatternId) }
                            .map_err(|_| ApplicationControlError::AutomationUnavailable)?;
                    unsafe { pattern.Expand() }
                        .map_err(|_| ApplicationControlError::AutomationUnavailable)
                }
                ActivateSelection => {
                    let element = if self.focus_region() == FocusRegion::ApplicationList {
                        self.focused_list_item()
                    } else {
                        self.focused_or_ancestor(|element| {
                            has_pattern::<IUIAutomationInvokePattern>(element, UIA_InvokePatternId)
                        })
                    }
                    .ok_or(ApplicationControlError::AutomationUnavailable)?;
                    invoke(&element)
                }
                CancelSelection => self.focus_active_list_item(),
                ScrollUp => self.scroll(ScrollAmount_SmallDecrement),
                ScrollDown => self.scroll(ScrollAmount_SmallIncrement),
                PageUp => self.scroll(ScrollAmount_LargeDecrement),
                PageDown => self.scroll(ScrollAmount_LargeIncrement),
                Send => self
                    .send_target()
                    .ok_or(ApplicationControlError::AutomationUnavailable)
                    .and_then(|button| invoke(&button)),
                ZoomIn => self.zoom(true),
                ZoomOut => self.zoom(false),
                _ => Err(ApplicationControlError::AutomationUnavailable),
            }
        }

        fn focus_region(&self) -> FocusRegion {
            let Some(focused) = self.focused.as_ref() else {
                return FocusRegion::Unknown;
            };
            if unsafe { focused.CurrentIsPassword() }.is_ok_and(|v| v.as_bool()) {
                return FocusRegion::Input;
            }
            if self
                .focused_or_ancestor(|element| {
                    unsafe { element.CurrentControlType() }.ok() == Some(UIA_EditControlTypeId)
                })
                .is_some()
            {
                return FocusRegion::Input;
            }
            if self.focused_list_item().is_some() {
                return FocusRegion::ApplicationList;
            }
            if self
                .focused_or_ancestor(|element| {
                    unsafe { element.CurrentControlType() }.ok() == Some(UIA_DocumentControlTypeId)
                        || has_pattern::<IUIAutomationScrollPattern>(element, UIA_ScrollPatternId)
                })
                .is_some()
            {
                return FocusRegion::Content;
            }
            FocusRegion::Unknown
        }

        fn has_modal(&self) -> bool {
            std::iter::once(&self.root)
                .chain(self.elements.iter())
                .any(|element| {
                    let Ok(pattern) = (unsafe {
                        element.GetCurrentPatternAs::<
                        windows::Win32::UI::Accessibility::IUIAutomationWindowPattern,
                    >(windows::Win32::UI::Accessibility::UIA_WindowPatternId)
                    }) else {
                        return false;
                    };
                    unsafe { pattern.CurrentIsModal() }.is_ok_and(|value| value.as_bool())
                })
        }

        fn list_items(&self) -> Vec<IUIAutomationElement> {
            self.elements
                .iter()
                .filter(|element| {
                    is_enabled(element)
                        && !is_offscreen(element)
                        && has_pattern::<IUIAutomationSelectionItemPattern>(
                            element,
                            UIA_SelectionItemPatternId,
                        )
                        && has_pattern::<IUIAutomationInvokePattern>(element, UIA_InvokePatternId)
                })
                .cloned()
                .collect()
        }

        fn focused_list_item(&self) -> Option<IUIAutomationElement> {
            self.focused_or_ancestor(|element| {
                has_pattern::<IUIAutomationSelectionItemPattern>(
                    element,
                    UIA_SelectionItemPatternId,
                ) && has_pattern::<IUIAutomationInvokePattern>(element, UIA_InvokePatternId)
            })
        }

        fn active_list_item(&self) -> Option<IUIAutomationElement> {
            let mut selected = self.list_items().into_iter().filter(|element| {
                unsafe {
                    element
                        .GetCurrentPatternAs::<IUIAutomationSelectionItemPattern>(
                            UIA_SelectionItemPatternId,
                        )
                        .and_then(|pattern| pattern.CurrentIsSelected())
                }
                .is_ok_and(|selected| selected.as_bool())
            });
            let first = selected.next()?;
            selected.next().is_none().then_some(first)
        }

        fn focus_active_list_item(&self) -> Result<(), ApplicationControlError> {
            self.active_list_item()
                .ok_or(ApplicationControlError::AutomationUnavailable)
                .and_then(set_focus)
        }

        fn move_list_focus(&self, delta: isize) -> Result<(), ApplicationControlError> {
            let (current, items) = self
                .list_context()
                .ok_or(ApplicationControlError::AutomationUnavailable)?;
            let index = current
                .as_ref()
                .and_then(|current| {
                    items.iter().position(|candidate| {
                        unsafe { self.automation.CompareElements(current, candidate) }
                            .is_ok_and(|same| same.as_bool())
                    })
                })
                .unwrap_or(if delta < 0 { items.len() - 1 } else { 0 });
            let next = (index as isize + delta).clamp(0, items.len() as isize - 1) as usize;
            if let Ok(scroll_item) = unsafe {
                items[next]
                    .GetCurrentPatternAs::<IUIAutomationScrollItemPattern>(UIA_ScrollItemPatternId)
            } {
                unsafe { scroll_item.ScrollIntoView() }
                    .map_err(|_| ApplicationControlError::AutomationUnavailable)?;
            }
            set_focus(items[next].clone())
        }

        fn list_context(
            &self,
        ) -> Option<(Option<IUIAutomationElement>, Vec<IUIAutomationElement>)> {
            let current = self
                .focused_list_item()
                .or_else(|| self.active_list_item())?;
            let pattern: IUIAutomationSelectionItemPattern =
                unsafe { current.GetCurrentPatternAs(UIA_SelectionItemPatternId) }.ok()?;
            let container = unsafe { pattern.CurrentSelectionContainer() }.ok()?;
            let items: Vec<_> = self
                .list_items()
                .into_iter()
                .filter(|candidate| {
                    let candidate_container = unsafe {
                        candidate
                            .GetCurrentPatternAs::<IUIAutomationSelectionItemPattern>(
                                UIA_SelectionItemPatternId,
                            )
                            .and_then(|item| item.CurrentSelectionContainer())
                    };
                    candidate_container.is_ok_and(|candidate_container| {
                        unsafe {
                            self.automation
                                .CompareElements(&container, &candidate_container)
                        }
                        .is_ok_and(|same| same.as_bool())
                    })
                })
                .collect();
            (!items.is_empty()).then_some((Some(current), items))
        }

        fn parent_selection_item(&self) -> Option<IUIAutomationElement> {
            let item = self.focused_list_item()?;
            let walker = unsafe { self.automation.ControlViewWalker() }.ok()?;
            let mut cursor = unsafe { walker.GetParentElement(&item) }.ok()?;
            for _ in 0..32 {
                if has_pattern::<IUIAutomationSelectionItemPattern>(
                    &cursor,
                    UIA_SelectionItemPatternId,
                ) && has_pattern::<IUIAutomationInvokePattern>(&cursor, UIA_InvokePatternId)
                {
                    return Some(cursor);
                }
                cursor = unsafe { walker.GetParentElement(&cursor) }.ok()?;
            }
            None
        }

        fn unique_input(&self) -> Option<IUIAutomationElement> {
            unique(self.elements.iter().filter(|element| {
                unsafe { element.CurrentControlType() }.ok() == Some(UIA_EditControlTypeId)
                    && is_enabled(element)
                    && is_focusable(element)
                    && !unsafe { element.CurrentIsPassword() }.is_ok_and(|v| v.as_bool())
            }))
        }

        fn unique_content(&self) -> Option<IUIAutomationElement> {
            unique(self.elements.iter().filter(|element| {
                unsafe { element.CurrentControlType() }.ok() == Some(UIA_DocumentControlTypeId)
                    && is_enabled(element)
                    && is_focusable(element)
            }))
        }

        fn scroll_target(&self) -> Option<IUIAutomationElement> {
            self.focused_or_ancestor(|element| is_vertical_scroll_target(element))
                .or_else(|| {
                    unique(
                        self.elements
                            .iter()
                            .filter(|element| is_vertical_scroll_target(element)),
                    )
                })
        }

        fn scroll(&self, amount: ScrollAmount) -> Result<(), ApplicationControlError> {
            let element = self
                .scroll_target()
                .ok_or(ApplicationControlError::AutomationUnavailable)?;
            let pattern: IUIAutomationScrollPattern =
                unsafe { element.GetCurrentPatternAs(UIA_ScrollPatternId) }
                    .map_err(|_| ApplicationControlError::AutomationUnavailable)?;
            if !unsafe { pattern.CurrentVerticallyScrollable() }.is_ok_and(|value| value.as_bool())
            {
                return Err(ApplicationControlError::AutomationUnavailable);
            }
            unsafe { pattern.Scroll(ScrollAmount_NoAmount, amount) }
                .map_err(|_| ApplicationControlError::AutomationUnavailable)
        }

        fn send_target(&self) -> Option<IUIAutomationElement> {
            let composer = self.focused_or_ancestor(|element| {
                unsafe { element.CurrentControlType() }.ok() == Some(UIA_EditControlTypeId)
            })?;
            if !is_enabled(&composer)
                || unsafe { composer.CurrentIsPassword() }.is_ok_and(|v| v.as_bool())
            {
                return None;
            }
            let mut related = Vec::new();
            for candidate in self.elements.iter().filter(|element| {
                is_enabled(element)
                    && has_pattern::<IUIAutomationInvokePattern>(element, UIA_InvokePatternId)
            }) {
                let controls_composer = unsafe { candidate.CurrentControllerFor() }
                    .ok()
                    .is_some_and(|array| array_contains(&self.automation, &array, &composer));
                let flows_from_composer = unsafe { composer.CurrentFlowsTo() }
                    .ok()
                    .is_some_and(|array| array_contains(&self.automation, &array, candidate));
                if controls_composer || flows_from_composer {
                    related.push(candidate.clone());
                }
            }
            if related.len() == 1 {
                related.pop()
            } else {
                None
            }
        }

        fn zoom_target(&self) -> Option<IUIAutomationElement> {
            unique(self.elements.iter().filter(|element| {
                unsafe {
                    element
                        .GetCurrentPatternAs::<IUIAutomationTransformPattern2>(
                            UIA_TransformPattern2Id,
                        )
                        .and_then(|pattern| pattern.CurrentCanZoom())
                }
                .is_ok_and(|value| value.as_bool())
            }))
        }

        fn zoom(&self, increase: bool) -> Result<(), ApplicationControlError> {
            let element = self
                .zoom_target()
                .ok_or(ApplicationControlError::AutomationUnavailable)?;
            let pattern: IUIAutomationTransformPattern2 =
                unsafe { element.GetCurrentPatternAs(UIA_TransformPattern2Id) }
                    .map_err(|_| ApplicationControlError::AutomationUnavailable)?;
            let current = unsafe { pattern.CurrentZoomLevel() }
                .map_err(|_| ApplicationControlError::AutomationUnavailable)?;
            let minimum = unsafe { pattern.CurrentZoomMinimum() }
                .map_err(|_| ApplicationControlError::AutomationUnavailable)?;
            let maximum = unsafe { pattern.CurrentZoomMaximum() }
                .map_err(|_| ApplicationControlError::AutomationUnavailable)?;
            let delta = if increase { 10.0 } else { -10.0 };
            let requested = (current + delta).clamp(minimum, maximum);
            unsafe { pattern.Zoom(requested) }
                .map_err(|_| ApplicationControlError::AutomationUnavailable)
        }

        fn focused_or_ancestor(
            &self,
            predicate: impl Fn(&IUIAutomationElement) -> bool,
        ) -> Option<IUIAutomationElement> {
            let walker = unsafe { self.automation.ControlViewWalker() }.ok()?;
            let mut cursor = self.focused.clone()?;
            for _ in 0..64 {
                if predicate(&cursor) {
                    return Some(cursor);
                }
                if unsafe { self.automation.CompareElements(&cursor, &self.root) }
                    .is_ok_and(|same| same.as_bool())
                {
                    break;
                }
                cursor = unsafe { walker.GetParentElement(&cursor) }.ok()?;
            }
            None
        }
    }

    fn has_pattern<T: Interface>(
        element: &IUIAutomationElement,
        id: windows::Win32::UI::Accessibility::UIA_PATTERN_ID,
    ) -> bool {
        unsafe { element.GetCurrentPatternAs::<T>(id) }.is_ok()
    }

    fn unique<'a>(
        mut elements: impl Iterator<Item = &'a IUIAutomationElement>,
    ) -> Option<IUIAutomationElement> {
        let first = elements.next()?.clone();
        if elements.next().is_none() {
            Some(first)
        } else {
            None
        }
    }

    fn set_focus(element: IUIAutomationElement) -> Result<(), ApplicationControlError> {
        unsafe { element.SetFocus() }.map_err(|_| ApplicationControlError::AutomationUnavailable)
    }

    fn invoke(element: &IUIAutomationElement) -> Result<(), ApplicationControlError> {
        let pattern: IUIAutomationInvokePattern =
            unsafe { element.GetCurrentPatternAs(UIA_InvokePatternId) }
                .map_err(|_| ApplicationControlError::AutomationUnavailable)?;
        unsafe { pattern.Invoke() }.map_err(|_| ApplicationControlError::AutomationUnavailable)
    }

    fn is_enabled(element: &IUIAutomationElement) -> bool {
        unsafe { element.CurrentIsEnabled() }.is_ok_and(|value| value.as_bool())
    }

    fn is_focusable(element: &IUIAutomationElement) -> bool {
        unsafe { element.CurrentIsKeyboardFocusable() }.is_ok_and(|value| value.as_bool())
    }

    fn is_offscreen(element: &IUIAutomationElement) -> bool {
        unsafe { element.CurrentIsOffscreen() }.map_or(true, |value| value.as_bool())
    }

    fn is_vertical_scroll_target(element: &IUIAutomationElement) -> bool {
        unsafe {
            element
                .GetCurrentPatternAs::<IUIAutomationScrollPattern>(UIA_ScrollPatternId)
                .and_then(|pattern| pattern.CurrentVerticallyScrollable())
        }
        .is_ok_and(|value| value.as_bool())
    }

    fn array_contains(
        automation: &IUIAutomation,
        array: &windows::Win32::UI::Accessibility::IUIAutomationElementArray,
        wanted: &IUIAutomationElement,
    ) -> bool {
        let length = unsafe { array.Length() }.unwrap_or(0).clamp(0, 256);
        (0..length).any(|index| {
            unsafe { array.GetElement(index) }
                .ok()
                .and_then(|element| unsafe { automation.CompareElements(&element, wanted) }.ok())
                .is_some_and(|same| same.as_bool())
        })
    }

    fn is_descendant(
        automation: &IUIAutomation,
        root: &IUIAutomationElement,
        element: &IUIAutomationElement,
    ) -> bool {
        let Ok(walker) = (unsafe { automation.ControlViewWalker() }) else {
            return false;
        };
        let mut cursor = element.clone();
        for _ in 0..128 {
            if unsafe { automation.CompareElements(&cursor, root) }.is_ok_and(|same| same.as_bool())
            {
                return true;
            }
            let Ok(parent) = (unsafe { walker.GetParentElement(&cursor) }) else {
                break;
            };
            cursor = parent;
        }
        false
    }

    fn process_executable_name(process_id: u32) -> Option<String> {
        use windows::Win32::System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
            TH32CS_SNAPPROCESS,
        };
        let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }.ok()?;
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut found = None;
        let mut more = unsafe { Process32FirstW(snapshot, &mut entry) }.is_ok();
        while more {
            if entry.th32ProcessID == process_id {
                let length = entry
                    .szExeFile
                    .iter()
                    .position(|character| *character == 0)
                    .unwrap_or(entry.szExeFile.len());
                found = Some(String::from_utf16_lossy(&entry.szExeFile[..length]));
                break;
            }
            more = unsafe { Process32NextW(snapshot, &mut entry) }.is_ok();
        }
        unsafe {
            let _ = windows::Win32::Foundation::CloseHandle(snapshot);
        }
        found
    }

    fn is_ime_executable(executable: &str) -> bool {
        matches!(
            executable.to_ascii_lowercase().as_str(),
            "textinputhost.exe" | "ctfmon.exe" | "inputapp.exe"
        )
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

    pub(super) fn probe_evidence(
        _token: &WindowToken,
    ) -> Result<Evidence, ApplicationControlError> {
        Err(ApplicationControlError::UnsupportedPlatform)
    }

    pub(super) fn perform(
        _token: &WindowToken,
        _evidence: &Evidence,
        _action: &SemanticAction,
    ) -> Result<(), ApplicationControlError> {
        Err(ApplicationControlError::UnsupportedPlatform)
    }
}

fn application_identity_for_executable(executable: &str) -> (String, ApplicationAdapterKind) {
    let executable = executable.to_ascii_lowercase();
    match executable.as_str() {
        "codex.exe" => ("codex".to_owned(), ApplicationAdapterKind::Codex),
        "msedge.exe" => ("edge".to_owned(), ApplicationAdapterKind::Browser),
        "chrome.exe" => ("chrome".to_owned(), ApplicationAdapterKind::Browser),
        "wechat.exe" | "weixin.exe" => ("wechat".to_owned(), ApplicationAdapterKind::WeChat),
        _ => (
            format!("executable:{executable}"),
            ApplicationAdapterKind::Generic,
        ),
    }
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
            application_identity_for_executable("Weixin.exe"),
            ("wechat".to_owned(), ApplicationAdapterKind::WeChat)
        );
        assert_eq!(
            application_identity_for_executable("msedge.exe").1,
            ApplicationAdapterKind::Browser
        );
    }

    #[test]
    fn send_is_blocked_for_candidate_modal_and_unknown_focus() {
        let candidate = Evidence {
            region: FocusRegion::ImeCandidate,
            ime_candidate: true,
            send_target: true,
            ..Default::default()
        };
        assert_eq!(
            capability_for(
                ApplicationAdapterKind::WeChat,
                &candidate,
                SemanticAction::Send
            )
            .state,
            CapabilityState::Blocked
        );
        let modal = Evidence {
            region: FocusRegion::Modal,
            modal: true,
            send_target: true,
            ..Default::default()
        };
        assert_eq!(
            capability_for(ApplicationAdapterKind::Codex, &modal, SemanticAction::Send).state,
            CapabilityState::Blocked
        );
        let unknown = Evidence::default();
        assert_ne!(
            capability_for(
                ApplicationAdapterKind::Codex,
                &unknown,
                SemanticAction::Send
            )
            .state,
            CapabilityState::Available
        );
    }

    #[test]
    fn browser_shortcuts_still_require_the_right_region() {
        let unknown = Evidence::default();
        assert_eq!(
            capability_for(
                ApplicationAdapterKind::Browser,
                &unknown,
                SemanticAction::PreviousTab,
            )
            .state,
            CapabilityState::Available
        );
        assert_eq!(
            capability_for(
                ApplicationAdapterKind::Browser,
                &unknown,
                SemanticAction::BrowserBack,
            )
            .state,
            CapabilityState::Blocked
        );
    }

    #[test]
    fn list_navigation_requires_selection_item_and_invoke_evidence() {
        let unavailable = Evidence {
            region: FocusRegion::ApplicationList,
            ..Default::default()
        };
        assert_eq!(
            capability_for(
                ApplicationAdapterKind::Codex,
                &unavailable,
                SemanticAction::SelectNext,
            )
            .state,
            CapabilityState::Unavailable
        );
        let available = Evidence {
            region: FocusRegion::ApplicationList,
            list_protocol: true,
            ..Default::default()
        };
        assert_eq!(
            capability_for(
                ApplicationAdapterKind::Codex,
                &available,
                SemanticAction::SelectNext,
            )
            .state,
            CapabilityState::Available
        );
    }
}
