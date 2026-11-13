//! 按键映射引擎：语义边沿 → 手势识别 → 动作注入。
//!
//! RC003 由报告层捕获独占映射输入；以下键盘/HID 双源仅用于 RC001（见 key_gate.rs 与
//! docs/investigations/2026-09-05-ll-swallow-vs-raw-input.md 的实证）：
//! 1. Raw Input 监听线程：HID 报文（usage 集合，绝对状态）与未被吞的键盘事件；
//! 2. key_gate 钩子线程：被吞键盘事件的边沿。
//! 两源汇入本引擎线程的 `ButtonStateMerger`（键盘/ HID 双源并集去重），
//! 输出语义边沿驱动 [`GestureRecognizer`]。
//!
//! 动作语义对齐 Mac 原版：全部为 tap（DOWN+UP 连发），无按住保持；
//! 按住 = 长按动作（一次 tap）或连发 tap。注入失败记录到快照，不中断引擎。
//!
//! 护栏：
//! - 注入只在门控存活时进行（key_gate 钩子线程未运行 → 不吞键 → 原始键照常
//!   进系统；此时注入会造成双输入，因此引擎保持观察模式）；
//! - 监听器停止/设备移除 → 释放全部按住状态并取消计时（不触发动作）；
//! - 语音键不参与映射（RemoteButton 无语音键条目，保持 ATVV 实时生命周期）。
//!
//! 泄漏对冲（2026-09-06 调查档案修复记录，结构性武装死锁的缓解）：常见
//! 物理 VK（方向/Enter/Home/TV）不能直接归因（见 key_gate.rs），孤立首按
//! 的原始键必泄漏进 OS。泄漏路径（[`EngineMessage::Keyboard`]，监听器按
//! 设备路径过滤，只含遥控器事件）的按压会把该键标记为"原生已交付"：
//! 若映射动作与原生动作相同（右→右 等，见 [`native_key`]），该次 Single
//! 跳过注入——冷首按单响应；Long/Double 与按住连发始终注入（原生无法
//! 交付组合语义/连发）。

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, RwLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::app_switcher::{self, AppSwitcherCommand, AppSwitcherIo, AppSwitcherSession, EndReason};
use crate::button_gestures::GestureRecognizer;
use crate::key_gate;
use crate::raw_input::{
    ButtonEdge, ButtonStateMerger, RawInputSnapshot, RawKeyboardEvent, RemoteButton,
};
use crate::send_input::{
    native_key, ButtonAction, ButtonActions, ButtonMappings, ButtonTrigger, HoldRepeat, KeyChord,
    KeyCode, MouseClickKind, MoveDirection, ScrollDirection,
};
use crate::UsageCounters;

fn mapping_note(note: String) {
    #[cfg(test)]
    if let Some(notes) = MAPPING_TEST_NOTES
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .as_mut()
    {
        notes.push(note.clone());
    }
    crate::ble::gatt_note(note);
}

#[cfg(test)]
static MAPPING_TEST_NOTES: Mutex<Option<Vec<String>>> = Mutex::new(None);
/// 「打开应用」实际使用的聚焦档案。
///
/// 预设应用（含用户手动添加的预设路径）直接使用产品默认聚焦方式
/// （Chrome / 网易云用快捷键，其余自动选择输入框）。
/// 自定义应用仍按用户保存的档案执行，没有档案时只打开。
fn open_app_effective_profile(
    mappings: &ButtonMappings,
    target: &str,
) -> Option<crate::focus::AppFocusProfile> {
    focus_profile_for_target(&mappings.focus_profiles, target)
}

/// 目标对应的实际聚焦档案：预设优先（含手动添加的预设路径），其余读取用户档案。
///
/// 组合动作的聚焦步骤与按键页的「打开应用 + 聚焦」必须使用同一判定，
/// 否则手动添加的预设路径会在组合动作里退化成「只打开」。
pub(crate) fn focus_profile_for_target(
    profiles: &std::collections::BTreeMap<String, crate::focus::AppFocusProfile>,
    target: &str,
) -> Option<crate::focus::AppFocusProfile> {
    if let Some(app) = crate::app_launcher::preset_app_for_target(target) {
        return Some(preset_focus_profile(app.id));
    }
    profiles.get(target).cloned()
}

/// 预设应用的默认聚焦入口。
///
/// 有稳定公开快捷键的应用优先用快捷键（不依赖无障碍树）：
/// - Chrome：`Ctrl+L` 聚焦地址栏；
/// - 网易云音乐：`Ctrl+F` 打开搜索页并聚焦搜索框。
///
/// 其余预设仍走通用输入框选择。快捷键在目标应用成为前台后才发送。
pub(crate) fn preset_focus_profile(target: &str) -> crate::focus::AppFocusProfile {
    let shortcut = match target {
        "chrome" => Some(KeyChord {
            keys: vec![KeyCode::LeftControl, KeyCode::L],
        }),
        "netease_music" => Some(KeyChord {
            keys: vec![KeyCode::LeftControl, KeyCode::F],
        }),
        _ => None,
    };
    match shortcut {
        Some(shortcut) => crate::focus::AppFocusProfile {
            strategy: crate::focus::FocusStrategy::AppShortcut,
            shortcut: Some(shortcut),
            ..Default::default()
        },
        None => crate::focus::AppFocusProfile {
            strategy: crate::focus::FocusStrategy::AutoComposer,
            ..Default::default()
        },
    }
}

/// 每次「只打开」请求递增；后台前台保障任务据此判断自己是否已被更新的一次
/// 打开取代，避免旧任务在用户已经切走后继续抢焦点。
static OPEN_ONLY_FOREGROUND_GENERATION: AtomicU64 = AtomicU64::new(0);

/// 「只打开应用」策略的冷启动前台保障：先有界等；仍未到前台时**只激活已存在的
/// 可见窗口**，绝不重新启动应用。
///
/// 不依赖私有键位方案宿主是否可用——公开构建也必须执行这段等待，否则冷启动
/// `launched_without_foreground` 后窗口会停在后台（2026-11-12 飞书现场）。
///
/// 重试必须是“激活”而不是“启动”：用户可能已经把刚打开的窗口关掉，再次调用
/// `launch_app` 会把应用重新拉起，表现为“关掉后又自己打开”（2026-11-12 现场）。
///
/// `canceled` 每次循环前检查：产生更新的一次打开后，旧任务不再激活任何窗口。
fn ensure_open_only_foreground(
    target: &str,
    rounds: u32,
    mut canceled: impl FnMut() -> bool,
    mut wait: impl FnMut(&str, std::time::Duration) -> bool,
    mut activate_existing: impl FnMut(&str) -> bool,
) -> bool {
    // 先做一次零等待检查：正常托盘恢复/已在前台时直接返回，不引入额外延迟。
    if wait(target, std::time::Duration::ZERO) {
        crate::ble::gatt_note(
            "open_app_profile_foreground phase=completed target_result=passed".to_owned(),
        );
        return true;
    }
    // 冷启动时不要先干等固定超时：每 500ms 检查一次，只要出现可见窗口就立刻
    // 激活；没有可见窗口（应用还没出现，或用户已经关掉）就只继续观察，绝不启动。
    for _ in 0..rounds {
        if canceled() {
            crate::ble::gatt_note(
                "open_app_profile_foreground phase=completed target_result=superseded".to_owned(),
            );
            return false;
        }
        if activate_existing(target) {
            crate::ble::gatt_note(
                "open_app_profile_foreground phase=retry_activate target_result=passed".to_owned(),
            );
        }
        if canceled() {
            crate::ble::gatt_note(
                "open_app_profile_foreground phase=completed target_result=superseded".to_owned(),
            );
            return false;
        }
        if wait(target, std::time::Duration::from_millis(500)) {
            crate::ble::gatt_note(
                "open_app_profile_foreground phase=completed target_result=passed".to_owned(),
            );
            return true;
        }
    }
    crate::ble::gatt_note(
        "open_app_profile_foreground phase=completed target_result=skipped".to_owned(),
    );
    false
}

/// 在后台执行「只打开」前台保障。最长 4 秒的等待不能占住引擎线程，否则后续
/// 按键会排队（2026-11-12 评审）。每次调用递增 generation，旧任务在下一轮
/// 检查时退出，不会在新一次打开之后继续抢焦点。
fn schedule_open_only_foreground(target: &str) {
    let generation = OPEN_ONLY_FOREGROUND_GENERATION.fetch_add(1, Ordering::Relaxed) + 1;
    let target = target.to_owned();
    let spawned = std::thread::Builder::new()
        .name("sayall-open-only-fg".to_owned())
        .spawn(move || {
            ensure_open_only_foreground(
                &target,
                8,
                || OPEN_ONLY_FOREGROUND_GENERATION.load(Ordering::Relaxed) != generation,
                |target, timeout| crate::app_launcher::wait_for_target_foreground(target, timeout),
                |target| crate::app_launcher::activate_existing_window(target),
            );
        });
    if spawned.is_err() {
        crate::ble::gatt_note(
            "open_app_profile_foreground phase=completed target_result=skipped reason=thread_unavailable"
                .to_owned(),
        );
    }
}

/// 引擎消息（监听器/门控/宿主 → 引擎线程）。
#[derive(Debug)]
pub enum EngineMessage {
    /// 监听器观察到的遥控器键盘事件（未被吞的；被吞的走 [`Self::GateEdge`]）。
    Keyboard(RawKeyboardEvent),
    /// 监听器观察到的一份 HID 报文 usage 集合（绝对状态）。
    HidUsages(BTreeSet<u16>),
    /// 门控吞下的键盘边沿（已归因到遥控器）。
    GateEdge(ButtonEdge),
    /// Device-bound report capture; these edges were removed before kbdhid.
    DriverEdge(ButtonEdge),
    /// Verified physical state for UI only; never enters the gesture/execution merger.
    HidObservation(u64),
    CaptureContinuityReset {
        old_owned_mask: u64,
    },
    /// Raw Input 监听器已停止：释放全部按住状态。
    ListenerStopped,
    /// 匹配的遥控器 HID 设备被移除（断连/睡眠）：释放全部按住状态。
    DeviceRemoved,
    /// 按键映射已更新：重建手势配置。
    MappingsChanged {
        execution: ButtonMappings,
        recognition: ButtonMappings,
        enhanced: bool,
    },
    /// Ordered barrier used by normal shutdown to prove every earlier
    /// reconfiguration/cancellation message has completed.
    Barrier(Sender<bool>),
    /// UI metadata is published only after earlier mapping updates were consumed.
    MappingNotice(MappingNoticeCallback),
    /// 临时暂挂（向导第⑥步"只看不动"）：仍观察边沿与手势，但不执行任何动作。
    /// 内存态、不落盘；进程结束自然消失。
    CaptureMappingsChanged {
        execution: ButtonMappings,
        recognition: ButtonMappings,
        enhanced: bool,
    },
    SetSuspended(bool),
    /// 语音键开始（MIC_OPEN / STREAM_START）：应用切换器会话必须释放 Alt，
    /// 否则按住说话快捷键会带着按住的 Alt 进入输入法。
    VoiceSessionStarted(Sender<bool>),
    /// 组合动作步骤请求启动应用切换器会话（Alt+Tab）。
    StartAppSwitcher,
    /// 退出前同步收尾应用切换器（带 ack：宿主必须等到 Alt 真的释放）。
    ReleaseAppSwitcherForExit(Sender<bool>),
    Shutdown,
}

pub struct MappingNoticeCallback(pub Box<dyn Fn(bool) + Send>);

impl std::fmt::Debug for MappingNoticeCallback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MappingNoticeCallback")
    }
}

/// 动作注入器抽象（生产实现包装 `SendInputRuntime`，测试实现记录调用）。
pub trait MappingInjector: Send + Sync {
    fn tap(&self, chord: &KeyChord) -> Result<(), String>;
    fn scroll(&self, direction: ScrollDirection, steps: u16) -> Result<(), String>;
    /// APP 级翻页：优先 `ScrollPattern`，没有档案时由后端找当前前台窗口的
    /// 最大可滚动区域；最后才退到不移动光标的滚轮消息。
    fn scroll_region(
        &self,
        _target: Option<&crate::scroll_region::ScrollRegionTarget>,
        _command: crate::scroll_region::ScrollCommand,
    ) -> Result<crate::scroll_region::ScrollOutcome, String> {
        Err("当前平台不支持翻页".to_owned())
    }
    fn mouse_click(&self, kind: MouseClickKind) -> Result<(), String>;
    fn mouse_move(&self, direction: MoveDirection, distance: u16) -> Result<(), String>;
    /// 打开/激活预设应用（生产实现调用 app_launcher）。
    fn launch_app(&self, target: &str) -> Result<(), String>;
    /// 聚焦当前前台应用的可编辑输入框。
    ///
    /// **异步受理**：返回 `Ok` 只表示请求已交给聚焦服务（它会按时间表重试并
    /// 独立汇报结果），不代表已经聚焦成功——聚焦结果见 [`Self::focus_report`]。
    fn focus_frontmost(&self) -> Result<(), String>;
    /// 打开/激活应用后按聚焦档案聚焦（`None` 或 `open_only` 等同只打开）。
    fn launch_app_and_focus(
        &self,
        target: &str,
        profile: Option<&crate::focus::AppFocusProfile>,
        cancellation: Option<&AtomicBool>,
    ) -> Result<(), String>;
    /// 取消异步聚焦/学习；没有后台资源的实现无需操作。
    fn cancel_focus(&self) {}
    /// 正常退出后永久关闭当前平台实例的聚焦入口。
    fn close_focus(&self) {
        self.cancel_focus();
    }
    /// 最近一次聚焦结果（供 UI 状态与诊断）；无该能力的实现返回 `None`。
    fn focus_report(&self) -> Option<crate::focus_service::FocusReport> {
        None
    }
    /// 「学习输入框」：在 3 秒窗口内轮询系统焦点，返回稳定命中的可编辑目标。
    ///
    /// 同步阻塞（窗口期内），调用方必须放在后台任务里。
    fn learn_focus_target(&self) -> Result<crate::focus::RecordedFocusTarget, String>;

    /// 「记录点击位置」：读取当前物理光标并要求它落在目标应用窗口内。
    ///
    /// 面向完全不暴露无障碍接口的应用（微信 4.x）。同步、无阻塞。
    fn learn_focus_point(&self, target: &str) -> Result<crate::focus::RecordedPoint, String> {
        crate::app_launcher::learn_focus_point(target)
    }

    /// 可选常用语组件（私有实现，构建期由宿主安装到进程级槽位）。
    /// 未安装时返回空实现：常用语不可用，且不影响任何按键行为。
    fn common_phrase_host(&self) -> Arc<dyn crate::common_phrases::CommonPhraseHost> {
        crate::common_phrases::current_common_phrase_host()
    }

    /// 应用切换器会话的原始按键注入（默认不支持；生产实现走 SendInput）。
    fn app_switcher_submit(&self, _command: AppSwitcherCommand) -> Result<(), String> {
        Err("当前平台不支持应用切换".to_owned())
    }

    /// 系统 Alt+Tab 切换器当前是否存在（布尔判定，不记录窗口身份）。
    fn app_switcher_present(&self) -> bool {
        false
    }

    /// 左 Alt 是否仍在系统键态（释放后的读回判据）。
    fn app_switcher_alt_down(&self) -> bool {
        false
    }

    /// 组合动作里的 Alt+Tab：请求映射引擎启动应用切换器会话。
    ///
    /// 目标会话在引擎线程中执行；该方法只保证启动请求已送达。
    fn start_app_switcher(&self) -> Result<(), String> {
        Err("当前平台不支持组合动作里的应用切换".to_owned())
    }
}

/// 把注入器接缝适配成 [`AppSwitcherIo`]，避免状态机依赖整个映射引擎。
struct InjectorAppSwitcherIo<'a>(&'a dyn MappingInjector);

impl AppSwitcherIo for InjectorAppSwitcherIo<'_> {
    fn submit(&self, command: AppSwitcherCommand) -> Result<(), String> {
        self.0.app_switcher_submit(command)
    }

    fn switcher_present(&self) -> bool {
        self.0.app_switcher_present()
    }

    fn alt_down(&self) -> bool {
        self.0.app_switcher_alt_down()
    }
}

/// 生产注入器：批量 SendInput tap（DOWN+UP），部分交付时由 send_input 层回滚。
pub struct SendInputInjector {
    runtime: Arc<crate::send_input_windows::SendInputRuntime>,
    /// 聚焦服务：单线程串行执行 UIA 聚焦（不阻塞按键/手势线程）。
    focus: Mutex<Option<crate::focus_service::FocusRunner>>,
    focus_cancel: Mutex<Option<Arc<AtomicBool>>>,
    focus_generation: AtomicU64,
    focus_closed: AtomicBool,
    focus_policy: crate::focus_service::FocusRetryPolicy,
    /// 可选常用语组件（私有实现注入；默认不可用）。
    common_phrase_host: RwLock<Arc<dyn crate::common_phrases::CommonPhraseHost>>,
}

impl SendInputInjector {
    #[cfg(windows)]
    pub fn new(runtime: Arc<crate::send_input_windows::SendInputRuntime>) -> Self {
        Self {
            runtime,
            focus: Mutex::new(None),
            focus_cancel: Mutex::new(None),
            focus_generation: AtomicU64::new(0),
            focus_closed: AtomicBool::new(false),
            focus_policy: crate::focus_service::FocusRetryPolicy::default(),
            common_phrase_host: RwLock::new(Arc::new(
                crate::common_phrases::DisabledCommonPhraseHost,
            )),
        }
    }

    /// 绑定可选常用语组件（私有实现，构建期注入；未绑定即不可用）。
    ///
    /// 绑定后：`OpenCommonPhrases` 动作可打开面板，面板打开期间六个按键边沿由
    /// `CommonPhraseHost::handle` 在**手势识别之前**认领。
    pub fn set_common_phrase_host(&self, host: Arc<dyn crate::common_phrases::CommonPhraseHost>) {
        let mut slot = self
            .common_phrase_host
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *slot = host;
    }

    /// 惰性启动聚焦服务（首次用到时才创建线程）。
    #[cfg(windows)]
    fn with_focus_runner<T>(
        &self,
        task: impl FnOnce(&crate::focus_service::FocusRunner) -> T,
    ) -> Option<T> {
        let generation = self.focus_generation.load(Ordering::Acquire);
        self.with_focus_runner_generation(generation, task)
    }

    #[cfg(windows)]
    fn with_focus_runner_generation<T>(
        &self,
        generation: u64,
        task: impl FnOnce(&crate::focus_service::FocusRunner) -> T,
    ) -> Option<T> {
        let mut slot = self
            .focus
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if self.focus_closed.load(Ordering::Acquire)
            || self.focus_generation.load(Ordering::Acquire) != generation
        {
            crate::ble::gatt_note(
                "focus_request phase=cancelled reason=stale_generation".to_owned(),
            );
            return None;
        }
        if slot.as_ref().is_some_and(|runner| runner.is_cancelled()) {
            slot.take();
        }
        if slot.is_none() {
            let backend: Arc<dyn crate::focus_service::FocusBackend> =
                Arc::new(crate::focus_windows::WindowsFocusBackend::new());
            let runner = crate::focus_service::FocusRunner::spawn(backend, self.focus_policy);
            *self.focus_cancel.lock().unwrap_or_else(|p| p.into_inner()) =
                Some(runner.cancellation_signal());
            *slot = Some(runner);
        }
        // 创建线程期间发生取消：其 signal 可能尚未发布，必须自行退役。
        if self.focus_closed.load(Ordering::Acquire)
            || self.focus_generation.load(Ordering::Acquire) != generation
        {
            slot.take();
            crate::ble::gatt_note(
                "focus_request phase=cancelled reason=stale_generation".to_owned(),
            );
            return None;
        }
        slot.as_ref().map(task)
    }

    #[cfg(not(windows))]
    fn with_focus_runner_generation<T>(
        &self,
        _generation: u64,
        _task: impl FnOnce(&crate::focus_service::FocusRunner) -> T,
    ) -> Option<T> {
        None
    }

    /// 非 Windows 平台没有 UIA 后端：聚焦服务不可用。
    #[cfg(not(windows))]
    fn with_focus_runner<T>(
        &self,
        _task: impl FnOnce(&crate::focus_service::FocusRunner) -> T,
    ) -> Option<T> {
        None
    }
}

impl MappingInjector for SendInputInjector {
    fn close_focus(&self) {
        self.focus_closed.store(true, Ordering::Release);
        self.cancel_focus();
    }

    fn cancel_focus(&self) {
        self.focus_generation.fetch_add(1, Ordering::AcqRel);
        let signal = self
            .focus_cancel
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone();
        if let Some(signal) = &signal {
            signal.store(true, Ordering::Relaxed);
        }
        // 学习调用持有 focus 锁等待服务结果，取消不得等待该锁。
        if let Ok(mut slot) = self.focus.try_lock() {
            slot.take();
        }
        crate::ble::gatt_note(format!(
            "focus_cancel phase=requested runner_present={}",
            signal.is_some()
        ));
    }

    fn scroll(&self, direction: ScrollDirection, steps: u16) -> Result<(), String> {
        self.runtime
            .scroll(direction, steps)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    fn scroll_region(
        &self,
        target: Option<&crate::scroll_region::ScrollRegionTarget>,
        command: crate::scroll_region::ScrollCommand,
    ) -> Result<crate::scroll_region::ScrollOutcome, String> {
        #[cfg(windows)]
        {
            crate::scroll_region_windows::scroll_foreground(target.cloned(), command)
                .map_err(|error| error.to_string())
        }
        #[cfg(not(windows))]
        {
            let _ = (target, command);
            Err("当前平台不支持翻页".to_owned())
        }
    }

    fn mouse_click(&self, kind: MouseClickKind) -> Result<(), String> {
        self.runtime
            .mouse_click(kind)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    fn mouse_move(&self, direction: MoveDirection, distance: u16) -> Result<(), String> {
        self.runtime
            .mouse_move(direction, distance)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    fn tap(&self, chord: &KeyChord) -> Result<(), String> {
        self.runtime
            .tap(chord.clone())
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    fn launch_app(&self, target: &str) -> Result<(), String> {
        crate::app_launcher::activate_or_launch(target)
    }

    fn focus_frontmost(&self) -> Result<(), String> {
        self.with_focus_runner(|runner| runner.submit(crate::focus_service::FocusTask::Frontmost))
            .flatten()
            .map(|_| ())
            .ok_or_else(|| "聚焦服务不可用".to_owned())
    }

    fn launch_app_and_focus(
        &self,
        target: &str,
        profile: Option<&crate::focus::AppFocusProfile>,
        cancellation: Option<&AtomicBool>,
    ) -> Result<(), String> {
        let generation = self.focus_generation.load(Ordering::Acquire);
        if self.focus_closed.load(Ordering::Acquire)
            || cancellation.is_some_and(|signal| signal.load(Ordering::Acquire))
        {
            return Err("应用正在退出".to_owned());
        }
        self.launch_app(target)?;
        if self.focus_closed.load(Ordering::Acquire)
            || self.focus_generation.load(Ordering::Acquire) != generation
            || cancellation.is_some_and(|signal| signal.load(Ordering::Acquire))
        {
            return Err("聚焦请求已取消".to_owned());
        }
        let strategy = profile
            .map(|profile| profile.strategy)
            .unwrap_or(crate::focus::FocusStrategy::OpenOnly);
        match strategy {
            crate::focus::FocusStrategy::OpenOnly => {
                // 冷启动窗口常晚于 activate_or_launch 的内建 1.5 秒预算
                // （2026-10-23 网易云音乐实测 4.2 秒）。按键方案要在下一按前
                // 命中目标 App，这里再有界等一次；只等前台，不抢焦点。
                //
                // 2026-10-28 真机回归：无线麦自己在前台时，Windows 的前台锁
                // 会让新窗口停在后面，单靠等待永远等不到；已运行应用的第二次
                // activate_or_launch 会走「恢复已有窗口并置前」，与 AppShortcut
                // 分支同一路径，能把目标真正拿到前台。
                //
                // 这段等待**不能**按「私有键位方案宿主是否可用」门控：公开构建
                // 不注入私有宿主，门控会让冷启动首按完全跳过等待/重试。2026-11-12
                // 飞书现场：菜单键冷启动 `launched_without_foreground` 后无任何
                // `focus_wait_foreground`/重试日志，窗口停在后台。
                // 等待与激活放到后台，避免最长 4 秒阻塞引擎线程；每次打开递增
                // generation，旧任务在用户发起下一次打开后退出。
                schedule_open_only_foreground(target);
                Ok(())
            }
            crate::focus::FocusStrategy::AutoComposer => {
                // 激活是异步完成的：先等目标成为前台，否则前几次尝试会打在旧前台
                // 应用上（与「聚焦已记录的输入框」同一理由）。
                let foreground = crate::app_launcher::wait_for_target_foreground(
                    target,
                    std::time::Duration::from_millis(2500),
                );
                if !foreground {
                    // 打开本身已经成功。窗口迟迟没到前台时放弃自动聚焦，
                    // 不把焦点抢到用户当前正在用的另一个应用上。
                    return Ok(());
                }
                self.with_focus_runner(|runner| {
                    runner.submit(crate::focus_service::FocusTask::LaunchThenFocus(
                        crate::focus::FocusChoice::BestComposer,
                    ))
                })
                .flatten()
                .map(|_| ())
                .ok_or_else(|| "聚焦服务不可用".to_owned())
            }
            crate::focus::FocusStrategy::AppShortcut => {
                let chord = profile
                    .and_then(|profile| profile.shortcut.clone())
                    .ok_or_else(|| "该应用未录入聚焦快捷键".to_owned())?;
                // 快捷键必须落在目标应用上：激活是异步的，先等目标成为前台，
                // 否则 Ctrl+L / Ctrl+F 会发给用户当前正在用的另一个应用。
                let mut foreground = crate::app_launcher::wait_for_target_foreground(
                    target,
                    std::time::Duration::from_millis(2500),
                );
                if !foreground {
                    // 冷启动可能超过 activate_or_launch 的 1.5 秒前台预算：窗口在主窗口
                    // 出现后仍留在后台。再走一次“已运行→前置”，避免冷启动首按丢焦点
                    // （2026-10-23 网易云音乐冷启动实测：4.2 秒才出现主窗口）。
                    crate::ble::gatt_note(
                        "open_app_focus_retry phase=started reason=target_not_foreground"
                            .to_owned(),
                    );
                    let _ = self.launch_app(target);
                    foreground = crate::app_launcher::wait_for_target_foreground(
                        target,
                        std::time::Duration::from_millis(2500),
                    );
                    crate::ble::gatt_note(format!(
                        "open_app_focus_retry phase=completed terminal_result={}",
                        if foreground { "passed" } else { "failed" }
                    ));
                }
                if !foreground {
                    crate::ble::gatt_note(
                        "open_app_focus_shortcut phase=skipped reason=target_not_foreground"
                            .to_owned(),
                    );
                    return Ok(());
                }
                self.tap(&chord)?;
                crate::ble::gatt_note("open_app_focus_shortcut phase=completed".to_owned());
                // 网易云音乐的 Ctrl+F 只打开搜索页，不把焦点放进搜索框；
                // 点击搜索框并全选旧查询，后续语音输入才会替换而不是追加。
                if let Some((x, y)) = crate::app_launcher::preset_focus_click_point(target) {
                    std::thread::sleep(std::time::Duration::from_millis(450));
                    if !crate::app_launcher::wait_for_target_foreground(
                        target,
                        std::time::Duration::from_millis(500),
                    ) {
                        crate::ble::gatt_note(
                            "open_app_focus_click phase=skipped reason=target_not_foreground"
                                .to_owned(),
                        );
                        return Ok(());
                    }
                    if self.runtime.move_cursor_and_click(x, y).is_err() {
                        crate::ble::gatt_note(
                            "open_app_focus_click phase=failed error_domain=send_input error_code=click_failed retryable=true"
                                .to_owned(),
                        );
                        return Ok(());
                    }
                    std::thread::sleep(std::time::Duration::from_millis(120));
                    let select_all = KeyChord {
                        keys: vec![KeyCode::LeftControl, KeyCode::A],
                    };
                    if self.tap(&select_all).is_err() {
                        crate::ble::gatt_note(
                            "open_app_focus_click phase=failed error_domain=send_input error_code=select_all_failed retryable=true"
                                .to_owned(),
                        );
                        return Ok(());
                    }
                    crate::ble::gatt_note("open_app_focus_click phase=completed".to_owned());
                }
                Ok(())
            }
            crate::focus::FocusStrategy::RecordedElement => {
                let recorded = profile
                    .and_then(|profile| profile.recorded.clone())
                    .ok_or_else(|| "该应用还没有学习过输入框".to_owned())?;
                // 激活是异步完成的：先等目标成为前台，否则聚焦服务的前几次尝试
                // 会打在上一个前台应用上（嵌套动作里尤其明显）。
                let _ = crate::app_launcher::wait_for_target_foreground(
                    target,
                    std::time::Duration::from_millis(2500),
                );
                self.with_focus_runner_generation(generation, |runner| {
                    runner.submit(crate::focus_service::FocusTask::LaunchThenFocus(
                        crate::focus::FocusChoice::Recorded(recorded),
                    ))
                })
                .flatten()
                .map(|_| ())
                .ok_or_else(|| "聚焦服务不可用".to_owned())
            }
            crate::focus::FocusStrategy::RecordedPoint => {
                let point = profile
                    .and_then(|profile| profile.point)
                    .ok_or_else(|| "该应用还没有记录点击位置".to_owned())?;
                let (x, y) = crate::app_launcher::recorded_point_absolute(target, &point)?;
                if cancellation.is_some_and(|signal| signal.load(Ordering::Acquire)) {
                    return Err("聚焦请求已取消".to_owned());
                }
                self.runtime
                    .move_cursor_and_click(x, y)
                    .map(|_| ())
                    .map_err(|error| error.to_string())
            }
        }
    }

    fn focus_report(&self) -> Option<crate::focus_service::FocusReport> {
        let slot = self
            .focus
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        slot.as_ref().and_then(|runner| runner.last_report())
    }

    fn learn_focus_target(&self) -> Result<crate::focus::RecordedFocusTarget, String> {
        self.with_focus_runner(|runner| runner.learn_target())
            .unwrap_or(Err(crate::focus_service::FocusFailure::Cancelled))
            .map_err(|reason| reason.as_str().to_owned())
    }

    fn common_phrase_host(&self) -> Arc<dyn crate::common_phrases::CommonPhraseHost> {
        let local = self
            .common_phrase_host
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        // 组件通常由宿主启动时安装到进程级槽位；本地字段为空实现时读槽位。
        if matches!(
            local.access(),
            crate::common_phrases::CommonPhraseAccess::Unavailable
        ) {
            return crate::common_phrases::current_common_phrase_host();
        }
        local
    }

    fn app_switcher_submit(&self, command: AppSwitcherCommand) -> Result<(), String> {
        if command == AppSwitcherCommand::AltDown && crate::ime::voice_session_active() {
            crate::ble::gatt_note(
                "app_switcher_inject phase=start result=rejected reason=voice_active".to_owned(),
            );
            return Err("语音会话中暂不打开应用切换器".to_owned());
        }

        let chord = match command {
            AppSwitcherCommand::AltDown | AppSwitcherCommand::AltUp => KeyChord {
                keys: vec![KeyCode::LeftAlt],
            },
            AppSwitcherCommand::Tab => KeyChord {
                keys: vec![KeyCode::Tab],
            },
            AppSwitcherCommand::ReverseTab => KeyChord {
                keys: vec![KeyCode::LeftShift, KeyCode::Tab],
            },
            AppSwitcherCommand::Escape => KeyChord {
                keys: vec![KeyCode::Escape],
            },
        };
        let result = match command {
            AppSwitcherCommand::AltDown => self.runtime.press(&chord),
            AppSwitcherCommand::AltUp => self.runtime.release(&chord),
            AppSwitcherCommand::Tab
            | AppSwitcherCommand::ReverseTab
            | AppSwitcherCommand::Escape => self.runtime.tap(chord),
        };
        match &result {
            Ok(_) => crate::ble::gatt_note(format!(
                "app_switcher_inject command={command:?} result=submitted"
            )),
            Err(error) => crate::ble::gatt_note(format!(
                "app_switcher_inject command={command:?} result=failed error_domain=send_input retryable=true error={error}"
            )),
        }
        result.map(|_| ()).map_err(|error| error.to_string())
    }

    fn app_switcher_present(&self) -> bool {
        crate::foreground::alt_tab_switcher_present()
    }

    fn app_switcher_alt_down(&self) -> bool {
        crate::send_input_windows::left_alt_down()
    }

    fn start_app_switcher(&self) -> Result<(), String> {
        request_app_switcher_start()
    }
}

pub type ButtonEdgeCallback = Arc<dyn Fn(ButtonEdge) + Send + Sync>;
pub type ButtonGestureCallback = Arc<dyn Fn(FiredGesture) + Send + Sync>;
pub type GestureHandler = Arc<dyn Fn(RoutedGesture) -> GestureDisposition + Send + Sync>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GestureDisposition {
    Handled,
    PassThrough,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoutedGesture {
    pub gesture: FiredGesture,
    /// The remote's native key already reached Windows before source attribution.
    /// Semantic routing must not add a second action in this case.
    pub native_delivered: bool,
}

/// 一次触发的手势（用于 UI 反馈与事件推送）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FiredGesture {
    pub button: RemoteButton,
    pub trigger: ButtonTrigger,
}

/// 按键映射运行时快照（UI 状态与诊断）。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ButtonMappingSnapshot {
    pub observed_buttons: Vec<RemoteButton>,
    pub enabled: bool,
    pub gate_active: bool,
    pub listener_active: bool,
    pub swallowed_edges: u64,
    pub leaked_downs: u64,
    pub fired_gestures: u64,
    pub last_fired: Option<FiredGesture>,
    pub last_error: Option<String>,
    /// 最近一次「聚焦输入框」的结果（未使用过该功能时为 `None`）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_focus: Option<crate::focus_service::FocusReport>,
}

#[derive(Debug, Default)]
struct EngineState {
    fired_gestures: u64,
    last_fired: Option<FiredGesture>,
    last_error: Option<String>,
    /// 向导等场景的临时暂挂：观察照旧，动作不执行。
    suspended: bool,
}

/// UI observation is a union of normal input and verified host reports. Mapping
/// cancellation must not make a physically held host key appear released.
#[derive(Default)]
struct ObservationState {
    mapped: BTreeSet<RemoteButton>,
    physical: BTreeSet<RemoteButton>,
}

impl ObservationState {
    fn active(&self) -> BTreeSet<RemoteButton> {
        self.mapped.union(&self.physical).copied().collect()
    }
}

#[derive(Default)]
struct ButtonObservation {
    state: Mutex<ObservationState>,
    callbacks: RwLock<Vec<ButtonEdgeCallback>>,
    connected: std::sync::atomic::AtomicBool,
}

impl ButtonObservation {
    fn update(&self, update: impl FnOnce(&mut ObservationState)) {
        let edges = {
            let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
            let previous = state.active();
            update(&mut state);
            let current = state.active();
            previous
                .symmetric_difference(&current)
                .map(|button| ButtonEdge {
                    button: *button,
                    is_pressed: current.contains(button),
                })
                .collect::<Vec<_>>()
        };
        for edge in edges {
            crate::gatt_note(format!(
                "button_observation button={:?} pressed={} execution=unchanged",
                edge.button, edge.is_pressed
            ));
            for callback in read_callbacks(&self.callbacks).iter() {
                callback(edge);
            }
        }
    }

    fn mapped_edge(&self, edge: ButtonEdge) {
        self.update(|state| {
            if edge.is_pressed {
                state.mapped.insert(edge.button);
            } else {
                state.mapped.remove(&edge.button);
            }
        });
    }

    fn physical(&self, buttons: u64) {
        self.update(|state| {
            state.physical = crate::raw_input::ALL_BUTTONS
                .iter()
                .enumerate()
                .filter_map(|(index, button)| (buttons & (1 << index) != 0).then_some(*button))
                .collect();
        });
    }

    fn clear(&self) {
        self.update(|state| *state = ObservationState::default());
    }

    fn active(&self) -> Vec<RemoteButton> {
        self.state
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .active()
            .into_iter()
            .collect()
    }
}

/// 基础映射 + 键位方案覆盖层。
///
/// 覆盖层赢过同键基础动作；未出现在覆盖层的键继续走基础映射。
/// 基础总开关关闭时覆盖层不生效。
fn merged_mappings(base: &ButtonMappings, overlay: Option<&ButtonMappings>) -> ButtonMappings {
    let mut merged = base.clone();
    if !merged.enabled {
        return merged;
    }
    if let Some(overlay) = overlay {
        for (button, actions) in &overlay.actions {
            merged.actions.insert(*button, actions.clone());
        }
        for (target, profile) in &overlay.focus_profiles {
            merged
                .focus_profiles
                .insert(target.clone(), profile.clone());
        }
    }
    merged
}

/// key_gate 需要吞掉的按键集合。
///
/// 覆盖层里的 `Disabled` 是「消费本次按键」而不是「回落到基础映射」：
/// 即使 `any_configured()` 为假，也必须把该键并入吞键掩码，否则原始键会
/// 漏进前台应用。
fn effective_button_mask(base: &ButtonMappings, overlay: Option<&ButtonMappings>) -> u64 {
    let mut mask = merged_mappings(base, overlay).mapped_mask();
    if base.enabled {
        if let Some(overlay) = overlay {
            for button in overlay.actions.keys() {
                mask |= 1u64 << button.ordinal();
            }
        }
    }
    mask
}

/// 单个手势的有效动作：覆盖层优先，否则基础映射。
fn effective_action(
    base: &ButtonMappings,
    overlay: Option<&ButtonMappings>,
    button: RemoteButton,
    trigger: ButtonTrigger,
) -> ButtonAction {
    if base.enabled {
        if let Some(overlay) = overlay {
            if let Some(actions) = overlay.actions.get(&button) {
                return actions.trigger(trigger).clone();
            }
        }
    }
    base.action_for(button, trigger)
}

/// 单个按键的有效三列配置（覆盖层优先，否则基础映射）。
///
/// 应用切换器会话在按下沿即时认领“任何一列绑定为切换应用”的键，不能只看
/// 当前触发方式——按住的方向键没有手势判定时间。
fn effective_actions(
    base: &ButtonMappings,
    overlay: Option<&ButtonMappings>,
    button: RemoteButton,
) -> ButtonActions {
    if base.enabled {
        if let Some(overlay) = overlay {
            if let Some(actions) = overlay.actions.get(&button) {
                return actions.clone();
            }
        }
    }
    base.actions(button)
}

fn action_is_app_switcher(actions: &ButtonActions) -> bool {
    [&actions.single, &actions.double, &actions.long]
        .iter()
        .any(|action| matches!(action, ButtonAction::AppSwitcher))
}

/// 按键映射引擎运行时。持有句柄即运行；线程在 `Shutdown` 或通道关闭时退出。
pub struct ButtonMappingRuntime {
    observation: Arc<ButtonObservation>,
    configuration: Arc<Mutex<InputConfiguration>>,
    capture_bridge: Arc<Mutex<std::sync::Weak<crate::rc003_bridge::Rc003Bridge>>>,
    capture_enabled: Arc<std::sync::atomic::AtomicBool>,
    sender: Sender<EngineMessage>,
    receiver: Mutex<Option<Receiver<EngineMessage>>>,
    state: Arc<Mutex<EngineState>>,
    edge_callbacks: Arc<RwLock<Vec<ButtonEdgeCallback>>>,
    gesture_callbacks: Arc<RwLock<Vec<ButtonGestureCallback>>>,
    gesture_handler: Arc<RwLock<Option<GestureHandler>>>,
    /// 与引擎工作线程共享的注入器句柄（用于读取最近一次聚焦结果、触发测试聚焦）。
    injector: Arc<dyn MappingInjector>,
    /// 组合动作运行时（RunMacro 手势分发与 UI 测试入口共用）。
    macro_runtime: Arc<crate::macro_runtime::MacroRuntime>,
    worker: Option<JoinHandle<()>>,
}

#[derive(Default)]
struct InputConfiguration {
    mappings: ButtonMappings,
    profile_mappings: Option<ButtonMappings>,
    profile_overlay: Option<ButtonMappings>,
    scene_mappings: ButtonMappings,
    model: crate::RemoteModel,
    connected: bool,
    enhanced: bool,
    captured_mask: u64,
    capability_diagnostic: Mutex<Option<String>>,
}

impl InputConfiguration {
    fn requested_mask(&self) -> u64 {
        merged_mappings(
            self.profile_mappings.as_ref().unwrap_or(&self.mappings),
            self.profile_overlay.as_ref(),
        )
        .mapped_mask()
            | self.scene_mappings.mapped_mask()
            | key_gate::app_switcher_mask()
    }

    fn apply_input_capabilities(&self, mut mappings: ButtonMappings) -> ButtonMappings {
        mappings.enabled &= self.connected;
        if !self.connected || self.model == crate::RemoteModel::Unknown {
            mappings.actions.clear();
        } else if self.model == crate::RemoteModel::Rc003 {
            mappings
                .actions
                .retain(|button, _| self.captured_mask & (1 << button.ordinal()) != 0);
        } else if self.model != crate::RemoteModel::Rc001 {
            mappings.actions.clear();
        }
        mappings
    }

    fn effective_mappings(&self) -> (ButtonMappings, ButtonMappings) {
        let execution = self.apply_input_capabilities(merged_mappings(
            self.profile_mappings.as_ref().unwrap_or(&self.mappings),
            self.profile_overlay.as_ref(),
        ));
        let scene = self.apply_input_capabilities(self.scene_mappings.clone());
        let recognition = merge_recognition_mappings(&execution, &scene);
        let note = format!(
            "map_capabilities model={:?} connected={} captured_mask={} requested_mask={} execution_mask={} recognition_mask={}",
            self.model,
            self.connected,
            self.captured_mask,
            self.requested_mask(),
            execution.mapped_mask(),
            recognition.mapped_mask(),
        );
        let mut last = self
            .capability_diagnostic
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        if last.as_ref() != Some(&note) {
            mapping_note(note.clone());
            *last = Some(note);
        }
        (execution, recognition)
    }
}

fn merge_recognition_mappings(
    execution: &ButtonMappings,
    scene: &ButtonMappings,
) -> ButtonMappings {
    let mut merged = execution.clone();
    merged.enabled = execution.enabled || scene.enabled;
    for (button, scene_actions) in &scene.actions {
        let actions = merged.actions.entry(*button).or_default();
        // Recognition markers are not user actions. Filling an empty Single
        // must not turn automatic repeat on for a scene-only control.
        if actions.single == ButtonAction::Disabled && actions.hold_repeat.is_none() {
            actions.hold_repeat = Some(HoldRepeat::Off);
        }
        // Scene controls own this button before common contextual clicks.
        actions.ok_context_click = false;
        merge_recognition_action(&mut actions.single, &scene_actions.single);
        merge_recognition_action(&mut actions.double, &scene_actions.double);
        merge_recognition_action(&mut actions.long, &scene_actions.long);
    }
    merged
}

fn merge_recognition_action(target: &mut ButtonAction, source: &ButtonAction) {
    if *target == ButtonAction::Disabled && *source != ButtonAction::Disabled {
        *target = ButtonAction::Shortcut {
            chord: KeyChord {
                keys: vec![KeyCode::Escape],
            },
        };
    }
}

/// A cancelled physical hold cannot become a new gesture after reconfiguration.
/// Releases with no accepted press must not manufacture a single/double click.
#[derive(Default)]
struct GestureInputState {
    blocked: BTreeSet<RemoteButton>,
    accepted: BTreeSet<RemoteButton>,
}

impl GestureInputState {
    fn cancel(&mut self, active: BTreeSet<RemoteButton>) {
        self.blocked.extend(active);
        self.accepted.clear();
    }

    fn accept(&mut self, edge: ButtonEdge) -> bool {
        if self.blocked.contains(&edge.button) {
            if !edge.is_pressed {
                self.blocked.remove(&edge.button);
            }
            return false;
        }
        if edge.is_pressed {
            self.accepted.insert(edge.button)
        } else {
            self.accepted.remove(&edge.button)
        }
    }
}

impl ButtonMappingRuntime {
    pub fn new(
        injector: Arc<dyn MappingInjector>,
        usage: Arc<UsageCounters>,
        snapshot: Arc<Mutex<RawInputSnapshot>>,
        macro_runtime: Arc<crate::macro_runtime::MacroRuntime>,
    ) -> Self {
        let (sender, receiver) = mpsc::channel();
        // 门控把被吞键盘边沿直接投递到引擎通道（钩子线程闭包投递，无阻塞）。
        key_gate::set_edge_sink(Arc::new({
            let sender = sender.clone();
            move |edge| {
                let _ = sender.send(EngineMessage::GateEdge(edge));
            }
        }));
        let mappings = Arc::new(RwLock::new(ButtonMappings::default()));
        let state = Arc::new(Mutex::new(EngineState::default()));
        let observation = Arc::new(ButtonObservation::default());
        let observation_sink = Arc::clone(&observation);
        let edge_callbacks = Arc::new(RwLock::new(vec![Arc::new(move |edge| {
            if key_gate::listener_active() {
                observation_sink.mapped_edge(edge);
            } else {
                observation_sink.clear();
            }
        }) as ButtonEdgeCallback]));
        let gesture_callbacks = Arc::new(RwLock::new(Vec::new()));
        let gesture_handler = Arc::new(RwLock::new(None));

        // BLE 语音开始（MIC_OPEN / STREAM_START）经这个端点取消应用切换器会话。
        // 进程内只有一个映射引擎；测试并行创建时后创建的会覆盖，发送失败是安全的空操作。
        if let Ok(mut slot) = voice_session_sender_slot().lock() {
            *slot = Some(sender.clone());
        }
        // 组合动作里的 Alt+Tab 经同一引擎通道启动应用切换器会话。
        if let Ok(mut slot) = app_switcher_sender_slot().lock() {
            *slot = Some(sender.clone());
        }

        let runtime = Self {
            observation: Arc::clone(&observation),
            configuration: Arc::new(Mutex::new(InputConfiguration::default())),
            capture_bridge: Arc::new(Mutex::new(std::sync::Weak::new())),
            capture_enabled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            sender,
            receiver: Mutex::new(Some(receiver)),
            state: Arc::clone(&state),
            edge_callbacks: Arc::clone(&edge_callbacks),
            gesture_callbacks: Arc::clone(&gesture_callbacks),
            gesture_handler: Arc::clone(&gesture_handler),
            injector: Arc::clone(&injector),
            macro_runtime: Arc::clone(&macro_runtime),
            worker: None,
        };

        key_gate::set_capture_mask_sink(Arc::new({
            let configuration = Arc::clone(&runtime.configuration);
            let capture_bridge = Arc::clone(&runtime.capture_bridge);
            let enabled = Arc::clone(&runtime.capture_enabled);
            move || {
                let bridge = capture_bridge.lock().unwrap().upgrade();
                if let Some(bridge) = bridge {
                    let configuration = configuration.lock().unwrap();
                    let mask = if configuration.connected
                        && configuration.model == crate::RemoteModel::Rc003
                    {
                        configuration.requested_mask()
                    } else {
                        0
                    };
                    drop(configuration);
                    bridge.set_capture_targets(enabled.load(Ordering::Acquire), mask);
                }
            }
        }));
        let worker = std::thread::Builder::new()
            .name("sayall-button-mapping".to_owned())
            .spawn({
                let mappings = Arc::clone(&mappings);
                let state = Arc::clone(&state);
                let snapshot = Arc::clone(&snapshot);
                let edge_callbacks = Arc::clone(&edge_callbacks);
                let gesture_callbacks = Arc::clone(&gesture_callbacks);
                let receiver = runtime
                    .receiver
                    .lock()
                    .unwrap()
                    .take()
                    .expect("engine receiver is taken exactly once");
                move || {
                    engine_worker(
                        receiver,
                        Arc::clone(&mappings),
                        Arc::new(RwLock::new(ButtonMappings::default())),
                        state,
                        snapshot,
                        edge_callbacks,
                        gesture_callbacks,
                        gesture_handler,
                        injector,
                        usage,
                        observation,
                        macro_runtime,
                    )
                }
            })
            .ok();
        let mut runtime = runtime;
        runtime.worker = worker;
        runtime
    }

    /// 监听器与门控向引擎投递消息的通道端点。
    pub fn sender(&self) -> Sender<EngineMessage> {
        self.sender.clone()
    }

    pub(crate) fn publish_mapping_notice(&self, callback: impl Fn(bool) + Send + 'static) {
        let _ = self
            .sender
            .send(EngineMessage::MappingNotice(MappingNoticeCallback(
                Box::new(callback),
            )));
    }

    /// 退出前同步释放应用切换器会话（等引擎确认已经释放 Alt）。
    ///
    /// Tauri 退出收尾是 `std::process::exit`，不执行析构；宿主必须在进程结束前
    /// 调用本方法并等待 ack，否则应用切换器会话可能把 Alt 留在系统键态。
    pub fn release_app_switcher_for_exit(&self, timeout: Duration) -> bool {
        let (sender, receiver) = mpsc::channel();
        if self
            .sender
            .send(EngineMessage::ReleaseAppSwitcherForExit(sender))
            .is_err()
        {
            return false;
        }
        receiver.recv_timeout(timeout).unwrap_or(false)
    }

    /// 更新按键映射：热加载到引擎 + 同步门控吞键配置。
    pub fn set_mappings(&self, mappings: ButtonMappings) {
        let mut configuration = self.configuration.lock().unwrap_or_else(|p| p.into_inner());
        let previous = configuration.effective_mappings();
        configuration.mappings = mappings;
        let (execution, recognition) = configuration.effective_mappings();
        if previous == (execution.clone(), recognition.clone()) {
            drop(configuration);
            self.sync_capture_targets();
            return;
        }
        let _ = self.sender.send(EngineMessage::MappingsChanged {
            execution,
            recognition,
            enhanced: configuration.enhanced,
        });
        drop(configuration);
        self.sync_capture_targets();
    }

    /// Selects an application-specific ordinary-key profile. `None` restores
    /// the persisted common mappings. Reconfiguration cancels any active hold;
    /// the engine then ignores repeated DOWN/UP edges until that hold is fully
    /// released, so gestures never cross profile boundaries.
    pub fn set_profile_mappings(&self, mappings: Option<ButtonMappings>) {
        let profile_active = mappings.is_some();
        let changed = {
            let mut configuration = self.configuration.lock().unwrap_or_else(|p| p.into_inner());
            let previous = configuration.effective_mappings();
            configuration.profile_mappings = mappings;
            let (execution, recognition) = configuration.effective_mappings();
            (previous != (execution.clone(), recognition.clone())).then_some((
                execution,
                recognition,
                configuration.enhanced,
            ))
        };
        let Some((execution, recognition, enhanced)) = changed else {
            self.sync_capture_targets();
            return;
        };
        crate::ble::gatt_note(format!(
            "map_profile_switch phase=requested profile_active={profile_active} active_hold_policy=cancel_then_gate_until_all_up"
        ));
        let result = self.sender.send(EngineMessage::MappingsChanged {
            execution,
            recognition,
            enhanced,
        });
        crate::ble::gatt_note(format!(
            "map_profile_switch phase=completed profile_active={profile_active} terminal_result={}",
            if result.is_ok() { "passed" } else { "failed" }
        ));
        self.sync_capture_targets();
    }

    /// Atomically switches the current direct profile and semantic-recognition
    /// profile. A foreground transition therefore produces one cancellation
    /// barrier and never exposes an intermediate mixed template kind.
    pub fn set_application_mappings(
        &self,
        profile_mappings: Option<ButtonMappings>,
        scene_mappings: ButtonMappings,
    ) {
        let profile_active = profile_mappings.is_some();
        let scene_active = scene_mappings.enabled;
        let changed = {
            let mut configuration = self.configuration.lock().unwrap_or_else(|p| p.into_inner());
            let previous = configuration.effective_mappings();
            configuration.profile_mappings = profile_mappings;
            configuration.scene_mappings = scene_mappings;
            let (execution, recognition) = configuration.effective_mappings();
            (previous != (execution.clone(), recognition.clone())).then_some((
                execution,
                recognition,
                configuration.enhanced,
            ))
        };
        let Some((execution, recognition, enhanced)) = changed else {
            self.sync_capture_targets();
            return;
        };
        crate::ble::gatt_note(format!(
            "map_application_switch phase=requested profile_active={profile_active} scene_active={scene_active} active_hold_policy=cancel_then_gate_until_all_up"
        ));
        let result = self.sender.send(EngineMessage::MappingsChanged {
            execution,
            recognition,
            enhanced,
        });
        crate::ble::gatt_note(format!(
            "map_application_switch phase=completed profile_active={profile_active} scene_active={scene_active} terminal_result={}",
            if result.is_ok() { "passed" } else { "failed" }
        ));
        self.sync_capture_targets();
    }

    /// Adds semantic gesture shapes to recognition without replacing the
    /// generic mappings used when the scene router passes through.
    pub fn set_scene_mappings(&self, mappings: ButtonMappings) {
        let mut configuration = self.configuration.lock().unwrap_or_else(|p| p.into_inner());
        let previous = configuration.effective_mappings();
        configuration.scene_mappings = mappings;
        let (execution, recognition) = configuration.effective_mappings();
        if previous == (execution.clone(), recognition.clone()) {
            drop(configuration);
            self.sync_capture_targets();
            return;
        }
        let _ = self.sender.send(EngineMessage::MappingsChanged {
            execution,
            recognition,
            enhanced: configuration.enhanced,
        });
        drop(configuration);
        self.sync_capture_targets();
    }

    /// Call only with the current, identified connection; reconnecting is unavailable.
    /// The basic channel supports the three vendor keys on RC001 only.
    pub fn set_input_context(&self, model: crate::RemoteModel, connected: bool) {
        let mut configuration = self.configuration.lock().unwrap_or_else(|p| p.into_inner());
        if configuration.model == model && configuration.connected == connected {
            return;
        }
        configuration.model = model;
        configuration.connected = connected;
        key_gate::set_report_capture_enabled(model == crate::RemoteModel::Rc003);
        if !connected {
            configuration.captured_mask = 0;
            configuration.enhanced = false;
        }
        self.observation.connected.store(
            connected && model == crate::RemoteModel::Rc003,
            std::sync::atomic::Ordering::Release,
        );
        if !connected {
            let _ = self.sender.send(EngineMessage::HidObservation(0));
        }
        crate::ble::gatt_note(format!(
            "map_input_context model={model:?} connected={connected} vendor_keys_available={}",
            connected && model == crate::RemoteModel::Rc001
        ));
        let (execution, recognition) = configuration.effective_mappings();
        let _ = self.sender.send(EngineMessage::MappingsChanged {
            execution,
            recognition,
            enhanced: configuration.enhanced,
        });
        drop(configuration);
        self.sync_capture_targets();
    }

    /// Grant execution only for the current report-capture ownership acknowledgement.
    pub(crate) fn set_capture_owned(&self, mask: u64) -> Option<u64> {
        let mut configuration = self.configuration.lock().unwrap_or_else(|p| p.into_inner());
        let mask = if configuration.connected && configuration.model == crate::RemoteModel::Rc003 {
            mask & configuration.requested_mask()
        } else {
            0
        };
        if configuration.captured_mask == mask {
            return None;
        }
        configuration.captured_mask = mask;
        configuration.enhanced = mask != 0;
        let (execution, recognition) = configuration.effective_mappings();
        let _ = self.sender.send(EngineMessage::CaptureMappingsChanged {
            execution,
            recognition,
            enhanced: mask != 0,
        });
        Some(mask)
    }

    #[cfg(test)]
    fn set_input_enhancement(&self, available: bool) {
        let _ = self.set_capture_owned(if available { (1 << 13) - 1 } else { 0 });
    }

    pub(crate) fn requested_capture_mask(&self) -> u64 {
        let c = self.configuration.lock().unwrap_or_else(|p| p.into_inner());
        if !c.connected || c.model != crate::RemoteModel::Rc003 {
            return 0;
        }
        c.requested_mask()
    }

    pub(crate) fn attach_capture_bridge(&self, bridge: &Arc<crate::rc003_bridge::Rc003Bridge>) {
        *self
            .capture_bridge
            .lock()
            .unwrap_or_else(|p| p.into_inner()) = Arc::downgrade(bridge);
        self.sync_capture_targets();
    }

    pub(crate) fn set_capture_enabled(&self, enabled: bool) {
        self.capture_enabled
            .store(enabled, std::sync::atomic::Ordering::Release);
        self.sync_capture_targets();
    }

    fn sync_capture_targets(&self) {
        let bridge = self
            .capture_bridge
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .upgrade();
        if let Some(bridge) = bridge {
            bridge.set_capture_targets(
                self.capture_enabled
                    .load(std::sync::atomic::Ordering::Acquire),
                self.requested_capture_mask(),
            );
        }
    }

    /// 更新键位方案覆盖层。
    ///
    /// 私有组件只在前后台切换/方案保存时调用；公开构建没有私有组件，永远不会调用。
    /// `None` 表示没有方案生效，引擎完全走基础映射。
    pub fn set_profile_overlay(&self, overlay: Option<ButtonMappings>) {
        let overlay = overlay.map(ButtonMappings::promote_app_switcher_shortcuts);
        let mut configuration = self.configuration.lock().unwrap_or_else(|p| p.into_inner());
        if configuration.profile_overlay == overlay {
            return;
        }
        configuration.profile_overlay = overlay;
        let (execution, recognition) = configuration.effective_mappings();
        let _ = self.sender.send(EngineMessage::MappingsChanged {
            execution,
            recognition,
            enhanced: configuration.enhanced,
        });
        drop(configuration);
        self.sync_capture_targets();
    }

    /// 当前覆盖层（测试/诊断用）。
    pub fn profile_overlay(&self) -> Option<ButtonMappings> {
        self.configuration
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .profile_overlay
            .clone()
    }

    /// 临时暂挂：观察照旧（边沿与手势回调不受影响），动作不执行。
    /// 只存在于引擎内存、不写入用户配置；用于向导第⑥步的"只看不动"。
    pub fn set_suspended(&self, suspended: bool) {
        let _ = self.sender.send(EngineMessage::SetSuspended(suspended));
    }

    pub fn mappings(&self) -> ButtonMappings {
        self.configuration
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .mappings
            .clone()
    }

    /// 组合动作运行时句柄（平台层在加载/保存组合动作后经它同步库快照）。
    pub fn macro_runtime(&self) -> Arc<crate::macro_runtime::MacroRuntime> {
        Arc::clone(&self.macro_runtime)
    }

    pub fn snapshot(&self) -> ButtonMappingSnapshot {
        let state = lock_state(&self.state);
        let enabled = {
            let configuration = self.configuration.lock().unwrap_or_else(|p| p.into_inner());
            configuration
                .profile_mappings
                .as_ref()
                .unwrap_or(&configuration.mappings)
                .enabled
        };
        ButtonMappingSnapshot {
            observed_buttons: self.observation.active(),
            enabled,
            gate_active: key_gate::is_gate_thread_alive(),
            listener_active: key_gate::listener_active(),
            swallowed_edges: key_gate::swallowed_edge_count(),
            leaked_downs: key_gate::leaked_down_count(),
            fired_gestures: state.fired_gestures,
            last_fired: state.last_fired,
            last_error: state.last_error.clone(),
            last_focus: self.injector.focus_report(),
        }
    }

    /// UI-only observations; these callbacks must never drive scene gestures.
    pub fn subscribe_button_observations(&self, callback: ButtonEdgeCallback) {
        self.observation
            .callbacks
            .write()
            .unwrap_or_else(|p| p.into_inner())
            .push(callback);
    }

    /// 订阅执行边沿，供菜单生命周期使用；不含只观察的增强报告。
    /// UI「测试」按钮：走与手势分发相同的注入器路径（异步受理，结果见快照）。
    pub fn close_focus(&self) {
        self.injector.close_focus();
    }

    pub fn focus_frontmost_now(&self) -> Result<(), String> {
        self.injector.focus_frontmost()
    }

    /// UI「测试打开与聚焦」：先打开/激活目标应用，再按其聚焦档案聚焦。
    pub fn launch_app_and_focus_now(&self, target: &str) -> Result<(), String> {
        let mappings = self.mappings();
        let profile = open_app_effective_profile(&mappings, target);
        self.injector
            .launch_app_and_focus(target, profile.as_ref(), None)
    }

    /// UI「开始学习输入框」：阻塞至多 3 秒，返回捕获到的输入框特征。
    pub fn learn_focus_target(&self) -> Result<crate::focus::RecordedFocusTarget, String> {
        self.injector.learn_focus_target()
    }

    /// UI「记录点击位置」：读取物理光标并要求其落在目标应用窗口内。
    pub fn learn_focus_point(&self, target: &str) -> Result<crate::focus::RecordedPoint, String> {
        self.injector.learn_focus_point(target)
    }

    /// 订阅语义按键边沿（Tauri 层转发为前端事件；画布高亮数据源）。
    pub fn subscribe_button_edges(&self, callback: ButtonEdgeCallback) {
        self.edge_callbacks
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(callback);
    }

    /// 订阅已触发手势（前端"单击/双击/长按"反馈）。
    pub fn subscribe_button_gestures(&self, callback: ButtonGestureCallback) {
        self.gesture_callbacks
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(callback);
    }

    pub fn set_gesture_handler(&self, handler: Option<GestureHandler>) {
        *self
            .gesture_handler
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = handler;
    }

    /// Wait until the engine has applied every message queued before this call.
    /// The caller supplies the bound so application exit cannot wait forever.
    pub fn wait_for_idle(&self, timeout: Duration) -> bool {
        self.host_raw_released(timeout).is_some()
    }

    /// Ordered snapshot of the selected device's existing native holds.
    /// A missing reply is unknown, never permission to reuse an all-up proof.
    pub(crate) fn host_raw_released(&self, timeout: Duration) -> Option<bool> {
        let (sender, receiver) = mpsc::channel();
        self.sender.send(EngineMessage::Barrier(sender)).ok()?;
        receiver.recv_timeout(timeout).ok()
    }
}

impl Drop for ButtonMappingRuntime {
    fn drop(&mut self) {
        let _ = self.sender.send(EngineMessage::Shutdown);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        // 引擎已经退出：任何应用切换器遮蔽都必须 fail-open。
        key_gate::set_app_switcher_mask(0);
        if let Ok(mut slot) = voice_session_sender_slot().lock() {
            *slot = None;
        }
    }
}

fn ignore_host_owned_keyboard(
    event: RawKeyboardEvent,
    merger: &ButtonStateMerger,
    owned_mask: u64,
) -> bool {
    event.button().is_some_and(|button| {
        // These messages already passed raw_input_windows' exact selected-device
        // filter. This only deduplicates the application's event queue; it never
        // suppresses Windows input. New keys require current capture ownership.
        owned_mask & (1 << button.ordinal()) != 0
            && (event.is_pressed() || !merger.keyboard_button_is_pressed(button))
    })
}

/// 「按住连续触发」一趟的日志聚合（按钮 → 触发方式、拍数、起始时刻）。
type RepeatRuns = BTreeMap<RemoteButton, (ButtonTrigger, u64, Instant)>;

/// OK 键「移动光标后按 OK 点击」的判定窗口：用遥控器移动过光标后的 5 秒内，
/// 按 OK 直接点击光标位置（方案见 `docs/plan/2026-10-06-ok-context-click.md`）。
const OK_CONTEXT_CLICK_WINDOW: Duration = Duration::from_secs(5);

/// 上下文点击是否命中窗口（纯函数，便于按合成时间测试边界）。
fn ok_context_click_window_hit(enabled: bool, last_move: Option<Instant>, now: Instant) -> bool {
    enabled
        && last_move.is_some_and(|at| now.saturating_duration_since(at) <= OK_CONTEXT_CLICK_WINDOW)
}

/// OK 按下沿的上下文点击判定与注入：命中窗口时注入一次左键单击并返回 `true`
/// （调用方据此吞掉本次按压）；未命中或注入失败返回 `false`（走原本的手势路径，
/// 注入失败时用户至少还能拿到原动作）。
///
/// 只对 OK 键生效；开关关闭时不留日志，避免每次按 OK 都刷一行。
fn ok_context_click_applies(
    edge: &ButtonEdge,
    mappings: &Arc<RwLock<ButtonMappings>>,
    last_remote_move_at: Option<Instant>,
    now: Instant,
    injector: &Arc<dyn MappingInjector>,
) -> bool {
    if !edge.is_pressed || edge.button != RemoteButton::Ok {
        return false;
    }
    let (mappings_enabled, switch_on) = {
        let mappings = read_lock(mappings);
        let actions = mappings.actions.get(&RemoteButton::Ok);
        (
            mappings.enabled,
            actions
                .map(|actions| actions.ok_context_click)
                .unwrap_or(false),
        )
    };
    if !mappings_enabled || !switch_on {
        return false;
    }
    let elapsed = last_remote_move_at.map(|at| now.saturating_duration_since(at));
    if !ok_context_click_window_hit(true, last_remote_move_at, now) {
        crate::ble::gatt_note(match elapsed {
            Some(elapsed) => format!(
                "map_ok_click action=skip reason=window_expired elapsed_ms={} window_ms={}",
                elapsed.as_millis(),
                OK_CONTEXT_CLICK_WINDOW.as_millis()
            ),
            None => "map_ok_click action=skip reason=no_recent_move".to_owned(),
        });
        return false;
    }
    // 门控未运行时不注入：原始键可能未被吞（同 map_skip_inject 的理由）。
    if !key_gate::is_gate_thread_alive() {
        crate::ble::gatt_note("map_ok_click action=skip reason=gate_not_alive".to_owned());
        return false;
    }
    match injector.mouse_click(MouseClickKind::Left) {
        Ok(()) => {
            crate::ble::gatt_note(format!(
                "map_ok_click action=click elapsed_ms={}",
                elapsed.map(|value| value.as_millis()).unwrap_or(0)
            ));
            true
        }
        Err(error) => {
            crate::ble::gatt_note(format!(
                "map_ok_click action=skip reason=inject_failed error_domain=send_input error={error}"
            ));
            false
        }
    }
}

/// 记录一次连续触发拍：首拍落 start 日志，尾拍在结束日志里带出总拍数。
fn note_repeat_beat(repeat_runs: &mut RepeatRuns, button: RemoteButton, trigger: ButtonTrigger) {
    if !repeat_runs.contains_key(&button) {
        crate::ble::gatt_note(format!(
            "map_repeat start button={button:?} trigger={trigger:?} interval_ms={}",
            button
                .repeat_interval()
                .map(|interval| interval.as_millis())
                .unwrap_or(0)
        ));
        repeat_runs.insert(button, (trigger, 0, Instant::now()));
    }
    if let Some(run) = repeat_runs.get_mut(&button) {
        run.1 = run.1.saturating_add(1);
    }
}

/// 结束一趟连续触发并落聚合日志（`reason` = release / reset / reconfigure / shutdown）。
fn finish_repeat_run(repeat_runs: &mut RepeatRuns, button: RemoteButton, reason: &str) {
    if let Some((trigger, beats, started)) = repeat_runs.remove(&button) {
        crate::ble::gatt_note(format!(
            "map_repeat end button={button:?} trigger={trigger:?} beats={beats} duration_ms={} reason={reason}",
            started.elapsed().as_millis()
        ));
    }
}

fn finish_all_repeat_runs(repeat_runs: &mut RepeatRuns, reason: &str) {
    let buttons: Vec<RemoteButton> = repeat_runs.keys().copied().collect();
    for button in buttons {
        finish_repeat_run(repeat_runs, button, reason);
    }
}

/// 应用切换器会话收尾：写终态日志、清门控遮蔽，并在 Alt 未释放时留下错误状态。
///
/// 每次引擎调用会话后都要调用一次；`EndReport` 每个终态只产生一次。
fn drain_app_switcher_end(session: &mut AppSwitcherSession, state: &Arc<Mutex<EngineState>>) {
    let Some(end) = session.take_last_end() else {
        return;
    };
    key_gate::set_app_switcher_mask(0);
    crate::ble::gatt_note(format!(
        "map_app_switcher operation={} phase=end reason={} escape={} alt_release={} alt_down_after={} switcher_opened={} duration_ms={} tab_count={} previous_count={} confirm_count={}",
        end.operation_id,
        end.reason.as_str(),
        end.escape_submitted,
        if end.alt_release_ok { "submitted" } else { "failed" },
        end.alt_still_down,
        end.switcher_opened,
        end.duration.as_millis(),
        end.tabs,
        end.previous,
        end.confirmations
    ));
    if end.alt_still_down {
        lock_state(state).last_error =
            Some("应用切换器没能释放 Alt，请按一次键盘 Alt 键恢复".to_owned());
    }
}

/// BLE 层在语音键 `MIC_OPEN` / `STREAM_START` 时调用：取消应用切换器会话。
///
/// 为什么需要：语音键不经过按键映射引擎；按住说话快捷键必须在没有 Alt 按住的
/// 状态下注入，否则输入法可能把组合键判成 Alt 系快捷键。未创建引擎时是空操作。
pub(crate) fn note_voice_session_started() -> Result<(), String> {
    let sender = voice_session_sender_slot().lock().unwrap().clone();
    let Some(sender) = sender else {
        return Ok(());
    };
    release_switcher_before_voice(&sender, Duration::from_millis(60))
}

fn release_switcher_before_voice(
    sender: &Sender<EngineMessage>,
    timeout: Duration,
) -> Result<(), String> {
    let (ack, receiver) = mpsc::channel();
    sender
        .send(EngineMessage::VoiceSessionStarted(ack))
        .map_err(|_| "语音开始前无法联系应用切换器".to_owned())?;
    match receiver.recv_timeout(timeout) {
        Ok(true) => Ok(()),
        Ok(false) => Err("语音开始前 Alt 尚未释放".to_owned()),
        Err(_) => Err("语音开始前应用切换器释放确认超时".to_owned()),
    }
}

static VOICE_SESSION_SENDER: std::sync::OnceLock<Mutex<Option<Sender<EngineMessage>>>> =
    std::sync::OnceLock::new();

fn voice_session_sender_slot() -> &'static Mutex<Option<Sender<EngineMessage>>> {
    VOICE_SESSION_SENDER.get_or_init(|| Mutex::new(None))
}

/// 组合动作（宏）启动应用切换器的引擎通道。
///
/// 进程内只有一个映射引擎；测试并行创建时后创建的会覆盖，发送失败是
/// 安全的空操作（组合动作步骤报失败，不影响按键与语音）。
static APP_SWITCHER_SENDER: std::sync::OnceLock<Mutex<Option<Sender<EngineMessage>>>> =
    std::sync::OnceLock::new();

fn app_switcher_sender_slot() -> &'static Mutex<Option<Sender<EngineMessage>>> {
    APP_SWITCHER_SENDER.get_or_init(|| Mutex::new(None))
}

fn request_app_switcher_start() -> Result<(), String> {
    let slot = app_switcher_sender_slot()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let Some(sender) = slot.as_ref() else {
        return Err("app_switcher_engine_unavailable".to_owned());
    };
    sender
        .send(EngineMessage::StartAppSwitcher)
        .map_err(|_| "app_switcher_engine_disconnected".to_owned())
}

/// 把定时器到期的**连续拍**并入聚合；`advance_with_ticks` 的第二返回值以外
/// 的手势（双击窗补发的单击、首次长按）是一次性手势，不进聚合
/// （2026-10-05 评审发现 1：避免把点按记成 `map_repeat`，污染拍数与时长）。
fn note_advance_ticks(repeat_runs: &mut RepeatRuns, ticks: Vec<(RemoteButton, ButtonTrigger)>) {
    for (button, trigger) in ticks {
        note_repeat_beat(repeat_runs, button, trigger);
    }
}

#[allow(clippy::too_many_arguments)]
fn engine_worker(
    receiver: Receiver<EngineMessage>,
    mappings: Arc<RwLock<ButtonMappings>>,
    recognition_mappings: Arc<RwLock<ButtonMappings>>,
    state: Arc<Mutex<EngineState>>,
    snapshot: Arc<Mutex<RawInputSnapshot>>,
    edge_callbacks: Arc<RwLock<Vec<ButtonEdgeCallback>>>,
    gesture_callbacks: Arc<RwLock<Vec<ButtonGestureCallback>>>,
    gesture_handler: Arc<RwLock<Option<GestureHandler>>>,
    injector: Arc<dyn MappingInjector>,
    usage: Arc<UsageCounters>,
    observation: Arc<ButtonObservation>,
    macro_runtime: Arc<crate::macro_runtime::MacroRuntime>,
) {
    let mut merger = ButtonStateMerger::default();
    let mut recognizer = GestureRecognizer::new();
    recognizer.configure(&read_lock(&recognition_mappings).clone());
    // 泄漏对冲标记：本次按住的原始键已泄漏进 OS（原生动作已交付）的按键。
    // 由 [`EngineMessage::Keyboard`]（泄漏路径）的按压边沿置位，Single 同键
    // 映射触发时消费并跳过注入；门控吞下的按压（[`EngineMessage::GateEdge`]）
    // 置位前清除。见模块文档"泄漏对冲"。
    let mut native_pending: BTreeSet<RemoteButton> = BTreeSet::new();
    let mut gesture_input = GestureInputState::default();
    let mut enhancement_active = false;
    let mut mapping_revision = 0u64;
    let mut mapping_notice: Option<MappingNoticeCallback> = None;
    // 「按住连续触发」统计：起拍建条目，释放/复位时聚合落一行。
    let mut repeat_runs: RepeatRuns = BTreeMap::new();
    // 常用语面板认领的按下（公开侧自持配对，2026-10-15 评审 M2）：被认领的
    // 按下不进手势识别，其释放沿也由引擎直接消费——不依赖组件在释放沿的
    // 返回值（那一瞬面板可能已关闭）。见 `CommonPhraseHost::handle` 的配对约定。
    let mut phrase_claimed: BTreeSet<RemoteButton> = BTreeSet::new();
    // OK 键上下文点击：最近一次由遥控器注入成功的鼠标移动时刻（5 秒窗口）。
    let mut last_remote_move_at: Option<Instant> = None;
    // 应用切换器（Alt+Tab）临时会话：状态与计时都留在引擎线程，避免跨线程锁。
    let mut app_switcher = AppSwitcherSession::new();

    loop {
        // 消息密集时 recv_timeout 可能一直不超时；在每条消息前补一次会话心跳，
        // 保证 15 秒自动取消与切换器存在性轮询不会因为消息不断而饿死。
        let now = Instant::now();
        if app_switcher
            .next_deadline()
            .is_some_and(|deadline| deadline <= now)
        {
            let io = InjectorAppSwitcherIo(injector.as_ref());
            let tick = app_switcher.tick(now, &io);
            if tick.opened_now {
                crate::ble::gatt_note(format!(
                    "map_app_switcher operation={} phase=open switcher_seen=true",
                    app_switcher.operation_id()
                ));
            }
            drain_app_switcher_end(&mut app_switcher, &state);
        }

        let timeout = match (recognizer.next_deadline(), app_switcher.next_deadline()) {
            (Some(left), Some(right)) => Some(left.min(right)),
            (Some(deadline), None) | (None, Some(deadline)) => Some(deadline),
            (None, None) => None,
        }
        .map(|deadline| deadline.saturating_duration_since(Instant::now()));
        let message = match timeout {
            Some(timeout) => match receiver.recv_timeout(timeout) {
                Ok(message) => message,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    let now = Instant::now();
                    // 只有连续触发计时器产生的拍才计入 map_repeat 聚合；
                    // 双击窗补发的单击与首次长按是一次性手势（2026-10-05 评审发现 1）。
                    let (fired, ticks) = recognizer.advance_with_ticks(now);
                    note_advance_ticks(&mut repeat_runs, ticks);
                    for (button, trigger) in fired {
                        if fire_gesture(
                            button,
                            trigger,
                            now,
                            &mappings,
                            &recognition_mappings,
                            &state,
                            &gesture_callbacks,
                            &gesture_handler,
                            &injector,
                            &mut native_pending,
                            &mut last_remote_move_at,
                            &macro_runtime,
                            &mut app_switcher,
                        ) {
                            reset_after_terminal_action(
                                "lock_workstation",
                                &mut merger,
                                &mut recognizer,
                                &mut repeat_runs,
                                &snapshot,
                                &edge_callbacks,
                                &mut native_pending,
                            );
                            break;
                        }
                    }
                    {
                        let io = InjectorAppSwitcherIo(injector.as_ref());
                        let tick = app_switcher.tick(now, &io);
                        if tick.opened_now {
                            crate::ble::gatt_note(format!(
                                "map_app_switcher operation={} phase=open switcher_seen=true",
                                app_switcher.operation_id()
                            ));
                        }
                        drain_app_switcher_end(&mut app_switcher, &state);
                    }
                    continue;
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    // 引擎线程收尾：先释放应用切换器的 Alt，再补齐连续触发聚合日志。
                    let io = InjectorAppSwitcherIo(injector.as_ref());
                    app_switcher.cancel(EndReason::EngineStopped, Instant::now(), &io);
                    drain_app_switcher_end(&mut app_switcher, &state);
                    key_gate::set_app_switcher_mask(0);
                    finish_all_repeat_runs(&mut repeat_runs, "shutdown");
                    break;
                }
            },
            None => match receiver.recv() {
                Ok(message) => message,
                Err(_) => {
                    let io = InjectorAppSwitcherIo(injector.as_ref());
                    app_switcher.cancel(EndReason::EngineStopped, Instant::now(), &io);
                    drain_app_switcher_end(&mut app_switcher, &state);
                    key_gate::set_app_switcher_mask(0);
                    finish_all_repeat_runs(&mut repeat_runs, "shutdown");
                    break;
                }
            },
        };

        let capture_reconfig = matches!(&message, EngineMessage::CaptureMappingsChanged { .. });
        match message {
            EngineMessage::CaptureContinuityReset { old_owned_mask } => {
                let active = merger.active_button_set();
                let mut cancelled = 0;
                for button in crate::raw_input::ALL_BUTTONS {
                    if old_owned_mask & (1 << button.ordinal()) == 0 {
                        continue;
                    }
                    recognizer.cancel_press(button);
                    finish_repeat_run(&mut repeat_runs, button, "capture_handoff");
                    native_pending.remove(&button);
                    if !phrase_claimed.contains(&button) {
                        gesture_input.accepted.remove(&button);
                        if active.contains(&button) {
                            gesture_input.blocked.insert(button);
                        }
                    }
                    cancelled += 1;
                }
                crate::ble::gatt_note(format!(
                    "map_capture_handoff cancelled_keys={cancelled} waiting_release={} available={enhancement_active}",
                    gesture_input.blocked.len(),
                ));
            }
            EngineMessage::HidObservation(buttons) => {
                if key_gate::listener_active()
                    && observation
                        .connected
                        .load(std::sync::atomic::Ordering::Acquire)
                {
                    observation.physical(buttons & ((1 << 13) - 1));
                } else {
                    observation.clear();
                }
            }
            EngineMessage::MappingNotice(callback) => {
                (callback.0)(read_lock(&recognition_mappings).mapped_mask() != 0);
                mapping_notice = Some(callback);
            }
            EngineMessage::Barrier(reply) => {
                let _ = reply.send(
                    crate::raw_input::ALL_BUTTONS
                        .iter()
                        .all(|button| !merger.keyboard_button_is_pressed(*button)),
                );
            }
            EngineMessage::Keyboard(event) => {
                if enhancement_active
                    && ignore_host_owned_keyboard(event, &merger, key_gate::enhanced_owned_mask())
                {
                    continue;
                }
                // Keyboard messages have already passed raw_input_windows' exact
                // selected-device path check. Drain only that source's existing
                // pre-takeover DOWN; a raw UP cannot remove a driver-owned hold.
                let now = Instant::now();
                let edges = merger.update_keyboard(event);
                // 泄漏路径的按压边沿：原生动作已进 OS，标记待对冲。
                for edge in &edges {
                    if edge.is_pressed {
                        native_pending.insert(edge.button);
                    }
                }
                handle_edges(
                    edges,
                    now,
                    &mut merger,
                    &mut recognizer,
                    &mappings,
                    &recognition_mappings,
                    &state,
                    &snapshot,
                    &edge_callbacks,
                    &gesture_callbacks,
                    &gesture_handler,
                    &injector,
                    &usage,
                    &mut native_pending,
                    &mut gesture_input,
                    &mut repeat_runs,
                    &mut phrase_claimed,
                    &mut last_remote_move_at,
                    &macro_runtime,
                    &mut app_switcher,
                );
            }
            EngineMessage::HidUsages(usages) => {
                // A complete, decoded keyboard report with no usages is direct
                // all-up evidence, including a release missed while disconnected.
                if usages.is_empty() && !enhancement_active && !gesture_input.blocked.is_empty() {
                    recognizer.release_all();
                    merger.release_all();
                    gesture_input = GestureInputState::default();
                    native_pending.clear();
                    lock_snapshot(&snapshot).active_buttons.clear();
                    crate::ble::gatt_note(
                        "map_resync evidence=hid_all_up waiting_for_release=false".to_owned(),
                    );
                    continue;
                }
                let now = Instant::now();
                let edges = merger.update_hid_usages(usages);
                handle_edges(
                    edges,
                    now,
                    &mut merger,
                    &mut recognizer,
                    &mappings,
                    &recognition_mappings,
                    &state,
                    &snapshot,
                    &edge_callbacks,
                    &gesture_callbacks,
                    &gesture_handler,
                    &injector,
                    &usage,
                    &mut native_pending,
                    &mut gesture_input,
                    &mut repeat_runs,
                    &mut phrase_claimed,
                    &mut last_remote_move_at,
                    &macro_runtime,
                    &mut app_switcher,
                );
            }
            EngineMessage::GateEdge(edge) | EngineMessage::DriverEdge(edge) => {
                let now = Instant::now();
                let edges = if matches!(message, EngineMessage::DriverEdge(_)) {
                    merger.apply_driver_button_edge(edge)
                } else {
                    merger.apply_keyboard_button_edge(edge.button, edge.is_pressed)
                };
                // 门控吞下的按压：原生动作未进 OS，清除待对冲标记。
                if edge.is_pressed {
                    native_pending.remove(&edge.button);
                }
                handle_edges(
                    edges,
                    now,
                    &mut merger,
                    &mut recognizer,
                    &mappings,
                    &recognition_mappings,
                    &state,
                    &snapshot,
                    &edge_callbacks,
                    &gesture_callbacks,
                    &gesture_handler,
                    &injector,
                    &usage,
                    &mut native_pending,
                    &mut gesture_input,
                    &mut repeat_runs,
                    &mut phrase_claimed,
                    &mut last_remote_move_at,
                    &macro_runtime,
                    &mut app_switcher,
                );
            }
            EngineMessage::ListenerStopped | EngineMessage::DeviceRemoved => {
                macro_runtime.cancel_all();
                injector.cancel_focus();
                observation.clear();
                let reason = match message {
                    EngineMessage::ListenerStopped => "listener_stopped",
                    _ => "device_removed",
                };
                crate::ble::gatt_note(format!("map_reset source={reason}"));
                // 应用切换器：来源失效时先取消会话并释放 Alt（Esc 只在切换器仍
                // 存在时发送），再把按住状态复位。
                {
                    let io = InjectorAppSwitcherIo(injector.as_ref());
                    let end_reason = match message {
                        EngineMessage::ListenerStopped => EndReason::ListenerStopped,
                        _ => EndReason::DeviceRemoved,
                    };
                    app_switcher.cancel(end_reason, Instant::now(), &io);
                    drain_app_switcher_end(&mut app_switcher, &state);
                }
                // 常用语面板（可选私有组件）：来源失效（断连、监听停止、睡眠
                // 唤醒后的 HID 接口移除）时关闭面板并清认领状态；未接入时空操作。
                injector.common_phrase_host().reset(reason);
                // 释放全部按住状态：取消所有手势计时，不触发动作。
                recognizer.release_all();
                cancel_gesture_input(&merger, &mut gesture_input, &snapshot, &edge_callbacks);
                native_pending.clear();
                key_gate::cancel_pending_holds();
                finish_all_repeat_runs(&mut repeat_runs, "reset");
                phrase_claimed.clear();
            }
            EngineMessage::MappingsChanged {
                execution,
                recognition,
                enhanced,
            }
            | EngineMessage::CaptureMappingsChanged {
                execution,
                recognition,
                enhanced,
            } => {
                if !capture_reconfig {
                    let io = InjectorAppSwitcherIo(injector.as_ref());
                    app_switcher.cancel(EndReason::MappingsChanged, Instant::now(), &io);
                    drain_app_switcher_end(&mut app_switcher, &state);
                }
                mapping_revision = mapping_revision.wrapping_add(1);
                let (repeat_single, repeat_long, repeat_auto, repeat_off) =
                    crate::raw_input::ALL_BUTTONS.iter().fold(
                        (0u32, 0u32, 0u32, 0u32),
                        |(single, long, automatic, off), button| {
                            let mask = 1 << button.ordinal();
                            let config = crate::button_gestures::GestureConfig::for_button(
                                &recognition,
                                *button,
                            );
                            let single_enabled = config.is_some_and(|c| c.repeat_single.is_some());
                            let long_enabled = config.is_some_and(|c| c.repeat_long.is_some());
                            let choice = execution.actions(*button).hold_repeat;
                            (
                                single | if single_enabled { mask } else { 0 },
                                long | if long_enabled { mask } else { 0 },
                                automatic
                                    | if single_enabled && choice.is_none() {
                                        mask
                                    } else {
                                        0
                                    },
                                off | if choice == Some(HoldRepeat::Off) {
                                    mask
                                } else {
                                    0
                                },
                            )
                        },
                    );
                crate::ble::gatt_note(format!(
                    "map_reconfig revision={mapping_revision} execution_enabled={} execution_mask={} recognition_mask={} enhanced={enhanced} repeat_single_mask={repeat_single} repeat_long_mask={repeat_long} repeat_auto_mask={repeat_auto} repeat_explicit_off_mask={repeat_off}",
                    execution.enabled, execution.mapped_mask(), recognition.mapped_mask(),
                ));
                if !recognition.enabled {
                    injector.cancel_focus();
                }
                enhancement_active = enhanced;
                cancel_gesture_input(&merger, &mut gesture_input, &snapshot, &edge_callbacks);
                recognizer.release_all();
                recognizer.configure(&recognition);
                let mut mapped_mask = recognition.mapped_mask();
                if enhanced {
                    for button in [
                        RemoteButton::Back,
                        RemoteButton::VolumeUp,
                        RemoteButton::VolumeDown,
                    ] {
                        mapped_mask &= !(1 << button.ordinal());
                    }
                }
                key_gate::cancel_pending_holds();
                key_gate::configure(recognition.enabled, mapped_mask);
                *mappings.write().unwrap_or_else(|p| p.into_inner()) = execution;
                *recognition_mappings
                    .write()
                    .unwrap_or_else(|p| p.into_inner()) = recognition;
                let mappings = read_lock(&recognition_mappings).clone();
                // 配置变化重置全部手势状态：挂起的泄漏对冲标记一并失效。
                // 常用语认领集合保留：正被认领按住的键仍靠它的释放沿配对消费。
                native_pending.clear();
                finish_all_repeat_runs(&mut repeat_runs, "reconfigure");
                let configured = crate::raw_input::ALL_BUTTONS
                    .iter()
                    .filter(|button| {
                        crate::button_gestures::GestureConfig::for_button(&mappings, **button)
                            .is_some()
                    })
                    .count();
                crate::ble::gatt_note(format!(
                    "map_reconfig enabled={} buttons_configured={}",
                    mappings.enabled, configured,
                ));
                if let Some(callback) = &mapping_notice {
                    (callback.0)(mappings.mapped_mask() != 0);
                }
            }
            EngineMessage::Shutdown => {
                let io = InjectorAppSwitcherIo(injector.as_ref());
                app_switcher.cancel(EndReason::Shutdown, Instant::now(), &io);
                drain_app_switcher_end(&mut app_switcher, &state);
                injector.close_focus();
                finish_all_repeat_runs(&mut repeat_runs, "shutdown");
                observation.clear();
                recognizer.release_all();
                cancel_gesture_input(&merger, &mut gesture_input, &snapshot, &edge_callbacks);
                key_gate::cancel_pending_holds();
                key_gate::configure(false, 0);
                break;
            }
            EngineMessage::SetSuspended(suspended) => {
                if suspended {
                    injector.cancel_focus();
                    // 暂挂（向导“只看不动”）不得遗留按住的 Alt。
                    let io = InjectorAppSwitcherIo(injector.as_ref());
                    app_switcher.cancel(EndReason::Suspended, Instant::now(), &io);
                    drain_app_switcher_end(&mut app_switcher, &state);
                }
                lock_state(&state).suspended = suspended;
                crate::ble::gatt_note(format!("map_suspension suspended={suspended}"));
            }
            EngineMessage::VoiceSessionStarted(ack) => {
                let io = InjectorAppSwitcherIo(injector.as_ref());
                if app_switcher.cancel(EndReason::VoiceStarted, Instant::now(), &io) {
                    drain_app_switcher_end(&mut app_switcher, &state);
                }
                let _ = ack.send(app_switcher.release_confirmed());
            }
            EngineMessage::StartAppSwitcher => {
                if lock_state(&state).suspended {
                    crate::ble::gatt_note(
                        "map_app_switcher phase=start result=ignored reason=suspended source=macro"
                            .to_owned(),
                    );
                } else {
                    let now = Instant::now();
                    let io = InjectorAppSwitcherIo(injector.as_ref());
                    let was_active = app_switcher.is_active();
                    if !was_active {
                        key_gate::set_app_switcher_mask(app_switcher::control_mask());
                        injector.common_phrase_host().close("app_switcher");
                    }
                    let result = if was_active {
                        app_switcher.advance(now, &io)
                    } else {
                        // 组合动作没有物理触发键；用 TV 作为会话来源占位，
                        // 日志用 source=macro 区分真实按键触发。
                        app_switcher.start(RemoteButton::Tv, now, &io)
                    };
                    match result {
                        Ok(operation) => crate::ble::gatt_note(format!(
                            "map_app_switcher phase={} operation={operation} result=submitted source=macro",
                            if was_active { "tab" } else { "start" }
                        )),
                        Err(error) => {
                            crate::ble::gatt_note(format!(
                                "map_app_switcher phase={} result=failed source=macro error_domain=send_input retryable=true error={error}",
                                if was_active { "tab" } else { "start" }
                            ));
                            lock_state(&state).last_error =
                                Some("打开应用切换器失败，请重试".to_owned());
                        }
                    }
                    drain_app_switcher_end(&mut app_switcher, &state);
                    if !app_switcher.is_active() {
                        // AltDown/Tab 注入失败或会话立即收尾：遮蔽必须 fail-open。
                        key_gate::set_app_switcher_mask(0);
                    }
                }
            }
            EngineMessage::ReleaseAppSwitcherForExit(ack) => {
                let io = InjectorAppSwitcherIo(injector.as_ref());
                app_switcher.cancel(EndReason::Shutdown, Instant::now(), &io);
                drain_app_switcher_end(&mut app_switcher, &state);
                key_gate::set_app_switcher_mask(0);
                let _ = ack.send(app_switcher.release_confirmed());
            }
        }
    }
    observation.clear();
}

fn cancel_gesture_input(
    merger: &ButtonStateMerger,
    input: &mut GestureInputState,
    snapshot: &Arc<Mutex<RawInputSnapshot>>,
    callbacks: &Arc<RwLock<Vec<ButtonEdgeCallback>>>,
) {
    let active = merger.active_button_set();
    input.cancel(active.clone());
    lock_snapshot(snapshot).active_buttons.clear();
    for callback in read_callbacks(callbacks).iter() {
        for button in &active {
            callback(ButtonEdge {
                button: *button,
                is_pressed: false,
            });
        }
    }
    crate::ble::gatt_note(format!(
        "map_cancel held_keys={} waiting_release={} waiting_for_release={}",
        active.len(),
        input.blocked.len(),
        !input.blocked.is_empty()
    ));
}

#[allow(clippy::too_many_arguments)]
fn handle_edges(
    edges: Vec<ButtonEdge>,
    now: Instant,
    merger: &mut ButtonStateMerger,
    recognizer: &mut GestureRecognizer,
    mappings: &Arc<RwLock<ButtonMappings>>,
    recognition_mappings: &Arc<RwLock<ButtonMappings>>,
    state: &Arc<Mutex<EngineState>>,
    snapshot: &Arc<Mutex<RawInputSnapshot>>,
    edge_callbacks: &Arc<RwLock<Vec<ButtonEdgeCallback>>>,
    gesture_callbacks: &Arc<RwLock<Vec<ButtonGestureCallback>>>,
    gesture_handler: &Arc<RwLock<Option<GestureHandler>>>,
    injector: &Arc<dyn MappingInjector>,
    usage: &Arc<UsageCounters>,
    native_pending: &mut BTreeSet<RemoteButton>,
    gesture_input: &mut GestureInputState,
    repeat_runs: &mut RepeatRuns,
    phrase_claimed: &mut BTreeSet<RemoteButton>,
    last_remote_move_at: &mut Option<Instant>,
    macro_runtime: &Arc<crate::macro_runtime::MacroRuntime>,
    app_switcher: &mut AppSwitcherSession,
) {
    if edges.is_empty() {
        return;
    }
    crate::ble::gatt_note(format!(
        "map_edges count={} detail={} gate(sw={} lk={})",
        edges.len(),
        edges
            .iter()
            .map(|edge| format!("{:?}={}", edge.button, edge.is_pressed))
            .collect::<Vec<_>>()
            .join(","),
        key_gate::swallowed_edge_count(),
        key_gate::leaked_down_count()
    ));
    for edge in edges.iter().filter(|edge| edge.is_pressed) {
        usage.record_button_press(edge.button);
    }
    {
        let mut snapshot = lock_snapshot(snapshot);
        snapshot.semantic_edge_count = snapshot
            .semantic_edge_count
            .saturating_add(edges.len() as u64);
        snapshot.active_buttons = merger
            .active_button_set()
            .difference(&gesture_input.blocked)
            .copied()
            .collect();
        if let Some(last) = edges.last() {
            snapshot.last_button = Some(last.button);
            snapshot.last_is_pressed = Some(last.is_pressed);
        }
    }
    for callback in read_callbacks(edge_callbacks).iter() {
        for edge in &edges {
            callback(*edge);
        }
    }

    for edge in edges {
        if !gesture_input.accept(edge) {
            mapping_note(format!(
                "map_gesture button={:?} pressed={} accepted=false fired_count=0 reason=waiting_release",
                edge.button, edge.is_pressed,
            ));
            native_pending.remove(&edge.button);
            if !edge.is_pressed {
                app_switcher.release(edge.button);
            }
            continue;
        }
        #[cfg(windows)]
        if edge.button == RemoteButton::Tv && edge.is_pressed {
            crate::lock_open_with_guard::note_tv_press();
        }
        if edge.is_pressed && recognizer.defers_single_until_release(edge.button) {
            crate::ble::gatt_note(format!(
                "map_terminal_wait button={:?} action=lock_workstation phase=armed release_required=true",
                edge.button
            ));
        }
        let fired = if edge.is_pressed {
            // 应用切换器（Alt+Tab）：会话期间在按下沿即时认领 OK/返回/方向/TV、
            // 启动键以及任何绑定为“切换应用”的键；不进入手势识别，也不执行
            // 原映射。门控遮蔽（key_gate::set_app_switcher_mask）保证原生键
            // 不会漏进系统切换器或前台应用。
            if app_switcher.is_active() {
                let is_trigger = app_switcher::is_control_button(edge.button)
                    || app_switcher.origin_button() == Some(edge.button)
                    || {
                        let base = read_lock(mappings);
                        action_is_app_switcher(&base.actions(edge.button))
                    };
                if is_trigger {
                    let io = InjectorAppSwitcherIo(injector.as_ref());
                    let result = app_switcher.press(edge.button, now, &io);
                    if let Some(action) = result.action {
                        crate::ble::gatt_note(format!(
                            "map_app_switcher operation={} phase=press button={:?} action={action:?}",
                            app_switcher.operation_id(),
                            edge.button
                        ));
                    }
                    if let Some(error) = result.error {
                        crate::ble::gatt_note(format!(
                            "map_app_switcher operation={} phase=press button={:?} result=failed error_domain=send_input retryable=true error={error}",
                            app_switcher.operation_id(),
                            edge.button
                        ));
                        lock_state(state).last_error =
                            Some(format!("应用切换器按键发送失败：{error}"));
                    }
                    drain_app_switcher_end(app_switcher, state);
                    if result.claimed {
                        // 被会话认领的按下不继承旧手势状态；其释放沿由会话的
                        // claimed 集合配对消费（见下方释放分支）。
                        phrase_claimed.remove(&edge.button);
                        recognizer.cancel_press(edge.button);
                        continue;
                    }
                }
            }
            // 常用语面板（可选私有组件）：在手势识别之前认领本次边沿。
            // 面板打开期间这六个键不区分单击/双击/长按，按下即响应；
            // 命中即结束，不进入 recognizer，也不执行原映射。
            if injector.common_phrase_host().handle(
                edge.button,
                true,
                crate::common_phrases::PHRASE_SOURCE_DEFAULT,
            ) {
                // 公开侧自持配对（2026-10-15 评审 M2）：记下被认领的按下，
                // 其释放沿由本引擎直接消费——组件在释放沿的返回值不作数
                //（那一瞬面板可能已关闭，释放沿绝不能漏进手势识别）。
                phrase_claimed.insert(edge.button);
                // 认领的按压不继承旧手势状态：上一轮双击窗若挂起，残留会在
                // 窗口超时后补发一次幽灵单击。
                recognizer.cancel_press(edge.button);
                Vec::new()
            } else {
                // 未认领的按下会进入手势识别：清掉该键可能残留的旧认领记录，
                // 让其释放沿回到手势识别（与上面的配对消费互补）。
                phrase_claimed.remove(&edge.button);
                // OK 上下文点击：命中窗口时立即点击光标位置，并吞掉本次按压
                //（单击/双击/长按都不执行）；注入失败则回落到正常路径。
                if !lock_state(state).suspended
                    && !native_pending.contains(&edge.button)
                    && read_lock(recognition_mappings)
                        .actions(edge.button)
                        .ok_context_click
                    && ok_context_click_applies(
                        &edge,
                        &mappings,
                        *last_remote_move_at,
                        now,
                        injector,
                    )
                {
                    recognizer.swallow_press(edge.button);
                    Vec::new()
                } else {
                    recognizer.press(edge.button, now)
                }
            }
        } else {
            // 释放沿：应用切换器会话认领过的按下必须配对消费（即使会话已在
            // 按下沿结束，例如 OK 确认 / 返回取消）。
            if app_switcher.release(edge.button) {
                finish_repeat_run(repeat_runs, edge.button, "app_switcher");
                continue;
            }
            // 释放沿：先问面板（保持组件内部在按状态同步），再按公开侧记账
            // 决定是否消费——被认领的按下，其释放沿必须配对吞下。
            let claimed_press = phrase_claimed.remove(&edge.button);
            let panel_handled = injector.common_phrase_host().handle(
                edge.button,
                false,
                crate::common_phrases::PHRASE_SOURCE_DEFAULT,
            );
            if claimed_press || panel_handled {
                if panel_handled && !claimed_press {
                    // 面板认领了「没有认领记录」的释放沿：该键的按下沿当时
                    // 进了手势识别（长按/双击计时在跑），先清掉残留状态再消费，
                    // 否则计时到期会引爆幽灵手势。
                    recognizer.cancel_press(edge.button);
                }
                finish_repeat_run(repeat_runs, edge.button, "phrase_panel");
                continue;
            }
            recognizer.release(edge.button, now)
        };
        if !edge.is_pressed {
            // 含「按住连续触发」的一趟结束：落聚合日志（拍数/时长）。
            finish_repeat_run(repeat_runs, edge.button, "release");
        }
        let recognition = read_lock(recognition_mappings);
        let configured =
            crate::button_gestures::GestureConfig::for_button(&recognition, edge.button).is_some();
        let reason = if !recognition.enabled {
            "recognition_disabled"
        } else if !configured {
            "unconfigured"
        } else if !fired.is_empty() {
            "fired"
        } else if edge.is_pressed {
            "waiting_gesture"
        } else {
            "released_without_gesture"
        };
        mapping_note(format!(
            "map_gesture button={:?} pressed={} accepted=true recognition_enabled={} configured={} fired_count={} reason={reason}",
            edge.button, edge.is_pressed, recognition.enabled, configured, fired.len(),
        ));
        drop(recognition);
        for trigger in fired {
            if fire_gesture(
                edge.button,
                trigger,
                now,
                mappings,
                recognition_mappings,
                state,
                gesture_callbacks,
                gesture_handler,
                injector,
                native_pending,
                last_remote_move_at,
                macro_runtime,
                app_switcher,
            ) {
                reset_after_terminal_action(
                    "lock_workstation",
                    merger,
                    recognizer,
                    repeat_runs,
                    snapshot,
                    edge_callbacks,
                    native_pending,
                );
                return;
            }
        }
    }
}

/// 会切换 Windows 会话的动作可能让遥控器释放沿延迟到解锁之后。动作已被系统
/// 接受时立即结束本轮按住状态；迟到的 UP 随后只会成为幂等输入，下一次真实 DOWN
/// 可立刻开始新一轮手势。
fn reset_after_terminal_action(
    reason: &str,
    merger: &mut ButtonStateMerger,
    recognizer: &mut GestureRecognizer,
    repeat_runs: &mut RepeatRuns,
    snapshot: &Arc<Mutex<RawInputSnapshot>>,
    edge_callbacks: &Arc<RwLock<Vec<ButtonEdgeCallback>>>,
    native_pending: &mut BTreeSet<RemoteButton>,
) {
    recognizer.release_all();
    let releases = merger.release_all();
    native_pending.clear();
    finish_all_repeat_runs(repeat_runs, "reset");
    crate::ble::gatt_note(format!(
        "map_reset source=terminal_action reason={reason} synthetic_releases={}",
        releases.len()
    ));
    if releases.is_empty() {
        return;
    }
    {
        let mut snapshot = lock_snapshot(snapshot);
        snapshot.semantic_edge_count = snapshot
            .semantic_edge_count
            .saturating_add(releases.len() as u64);
        snapshot.active_buttons.clear();
        if let Some(last) = releases.last() {
            snapshot.last_button = Some(last.button);
            snapshot.last_is_pressed = Some(false);
        }
    }
    for callback in read_callbacks(edge_callbacks).iter() {
        for edge in &releases {
            callback(*edge);
        }
    }
}

/// 当前前台 APP 对应的滚动区域档案；只在 Windows 生产路径使用。
#[cfg(windows)]
fn scroll_target_for_foreground(
    mappings: &ButtonMappings,
) -> Option<crate::scroll_region::ScrollRegionTarget> {
    let executable = crate::focus_windows::foreground_executable_path()?;
    mappings
        .scroll_regions
        .iter()
        .find(|(target, _)| crate::app_launcher::target_matches_executable(target, &executable))
        .map(|(_, target)| target.clone())
}

#[allow(clippy::too_many_arguments)]
fn fire_gesture(
    button: RemoteButton,
    trigger: ButtonTrigger,
    now: Instant,
    mappings: &Arc<RwLock<ButtonMappings>>,
    recognition_mappings: &Arc<RwLock<ButtonMappings>>,
    state: &Arc<Mutex<EngineState>>,
    gesture_callbacks: &Arc<RwLock<Vec<ButtonGestureCallback>>>,
    gesture_handler: &Arc<RwLock<Option<GestureHandler>>>,
    injector: &Arc<dyn MappingInjector>,
    native_pending: &mut BTreeSet<RemoteButton>,
    last_remote_move_at: &mut Option<Instant>,
    macro_runtime: &Arc<crate::macro_runtime::MacroRuntime>,
    app_switcher: &mut AppSwitcherSession,
) -> bool {
    let fired = FiredGesture { button, trigger };
    let suspended = {
        let mut state = lock_state(state);
        state.fired_gestures = state.fired_gestures.saturating_add(1);
        state.last_fired = Some(fired);
        state.suspended
    };
    for callback in read_callbacks(gesture_callbacks).iter() {
        callback(fired);
    }
    // 暂挂（向导第⑥步）：手势仍被观察与广播，但不执行任何注入/动作。
    if suspended {
        return false;
    }

    let recognition = read_lock(recognition_mappings).clone();
    // 门控未运行时不注入：原始键未被吞（或无法归因），注入会造成双输入。
    if !recognition.enabled || !key_gate::is_gate_thread_alive() {
        if recognition.enabled {
            mapping_note(format!(
                "map_skip_inject reason=gate_not_alive enabled={} gate_alive=false button={:?} trigger={:?}",
                recognition.enabled, button, trigger
            ));
            let mut state = lock_state(state);
            state.last_error =
                Some("按键映射门控未运行，已保持观察模式（不注入，避免双输入）".to_owned());
        }
        return false;
    }
    let native_delivered = native_pending.remove(&button);
    let handler = read_lock(gesture_handler).clone();
    if let Some(handler) = handler {
        let disposition = handler(RoutedGesture {
            gesture: fired,
            native_delivered,
        });
        mapping_note(format!(
            "map_route button={button:?} trigger={trigger:?} disposition={disposition:?} native_delivered={native_delivered}"
        ));
        if disposition != GestureDisposition::PassThrough {
            return false;
        }
    }
    let mappings = read_lock(mappings).clone();
    if !mappings.enabled {
        mapping_note(format!(
            "map_skip_inject reason=mapping_disabled button={button:?} trigger={trigger:?}"
        ));
        return false;
    }
    let action = mappings.action_for(button, trigger);
    let scroll_target: Option<crate::scroll_region::ScrollRegionTarget> = {
        #[cfg(windows)]
        {
            scroll_target_for_foreground(&mappings)
        }
        #[cfg(not(windows))]
        {
            None
        }
    };
    if action == ButtonAction::Disabled {
        mapping_note(format!(
            "map_skip_inject reason=action_disabled button={:?} trigger={:?}",
            button, trigger
        ));
        return false;
    }
    // 泄漏对冲：该按住的原始键已泄漏进 OS（原生动作已交付）。Single 且映射
    // 动作与原生动作相同（右→右 等）时跳过注入（原生已交付，注入即双响应）；
    // 其余触发（Long/Double/连发/不同动作）始终注入——原生无法交付组合
    // 语义与连发。标记在此消费，对冲只作用于本次按住的首个 Single。
    if native_delivered {
        if trigger == ButtonTrigger::Single {
            let native_covers = matches!(&action, ButtonAction::Shortcut { chord }
                if chord.keys.len() == 1
                    && native_key(button).is_some_and(|native| chord.keys[0] == native));
            if native_covers {
                crate::ble::gatt_note(format!(
                    "map_skip_inject reason=native_covers_action button={:?} trigger=single",
                    button
                ));
                return false;
            }
        }
    }
    match action {
        ButtonAction::Disabled => {}
        ButtonAction::TaskSwitch { .. } => {
            // Only the foreground-guarded system-task router may execute this action.
            crate::gatt_note("task_switch phase=rejected reason=router_unavailable".to_owned());
            lock_state(state).last_error = Some("任务切换控制器不可用".to_owned());
        }
        ButtonAction::Unsupported(raw) => {
            crate::ble::gatt_note(format!(
                "map_skip_inject reason=unsupported_action_type button={button:?} trigger={trigger:?} action_type={}",
                raw.log_token()
            ));
        }
        ButtonAction::Scroll { direction, steps } => {
            crate::ble::gatt_note(format!(
                "map_fire button={button:?} trigger={trigger:?} action=scroll direction={direction:?} steps={steps}"
            ));
            if let Err(error) = injector.scroll(direction, steps) {
                lock_state(state).last_error = Some(format!("滚轮事件发送失败：{error}"));
            }
        }
        ButtonAction::PageUp
        | ButtonAction::PageDown
        | ButtonAction::ScrollToTop
        | ButtonAction::ScrollToBottom => {
            let command = match &action {
                ButtonAction::PageUp => crate::scroll_region::ScrollCommand::PageUp,
                ButtonAction::PageDown => crate::scroll_region::ScrollCommand::PageDown,
                ButtonAction::ScrollToTop => crate::scroll_region::ScrollCommand::ToTop,
                _ => crate::scroll_region::ScrollCommand::ToBottom,
            };
            crate::ble::gatt_note(format!(
                "map_fire button={button:?} trigger={trigger:?} action={} target_recorded={}",
                command.as_str(),
                scroll_target.is_some()
            ));
            match injector.scroll_region(scroll_target.as_ref(), command) {
                Ok(outcome) => crate::ble::gatt_note(format!(
                    "map_scroll result=ok command={} method={} moved={} at_edge={}",
                    command.as_str(),
                    outcome.method.as_str(),
                    outcome.moved,
                    outcome.at_edge
                )),
                Err(error) => {
                    crate::ble::gatt_note(format!(
                        "map_scroll result=err command={} reason={error}",
                        command.as_str()
                    ));
                    lock_state(state).last_error = Some(format!("翻页失败：{error}"));
                }
            }
        }
        ButtonAction::MouseClick { kind } => {
            crate::ble::gatt_note(format!(
                "map_fire button={button:?} trigger={trigger:?} action=mouse_click kind={kind:?}"
            ));
            if let Err(error) = injector.mouse_click(kind) {
                lock_state(state).last_error = Some(format!("鼠标点击失败：{error}"));
            }
        }
        ButtonAction::MouseMove {
            direction,
            distance,
        } => {
            crate::ble::gatt_note(format!(
                "map_fire button={button:?} trigger={trigger:?} action=mouse_move direction={direction:?} distance={distance}"
            ));
            // 只有注入成功才算"用户刚刚用遥控器移动过光标"：OK 上下文点击
            // 的窗口据此刷新（失败不刷新，见 map_ok_click 的 skip 日志）。
            match injector.mouse_move(direction, distance) {
                Ok(()) => *last_remote_move_at = Some(Instant::now()),
                Err(error) => lock_state(state).last_error = Some(format!("鼠标移动失败：{error}")),
            }
        }
        ButtonAction::Shortcut { chord } => {
            let terminal_action = chord.is_lock_workstation();
            mapping_note(format!(
                "map_fire button={:?} trigger={:?} action=shortcut key_count={}",
                button,
                trigger,
                chord.keys.len(),
            ));
            // 成功判据分两层（Issue #195，2026-10-08）：`tap` 的 Ok 只表示事件
            // 已入队（SendInput 提交数 = 计划数），不能当成"前台收到了键"——
            // 0.5.0 现场就是 `map_inject result=ok` 与"测试页一个键都没有"同时
            // 成立。这里补上第二层：本进程键钩子宿主看到的注入事件回声
            // （`echo_delta`/`last_*`，见 key_gate::InjectedEcho）。
            let echo_before = crate::key_gate::injected_event_count();
            match injector.tap(&chord) {
                Ok(()) => {
                    // 锁屏动作走 Win32 API（LockWorkStation），不进输入流：
                    // 没有可观测的注入回声，不得套用 SendInput 分支的判据。
                    if terminal_action {
                        mapping_note(format!(
                            "{LOCK_WORKSTATION_INJECT_NOTE} button={button:?} trigger={trigger:?}"
                        ));
                    } else {
                        let echo = observe_injection_echo(echo_before);
                        mapping_note(format!(
                            "{} button={button:?} trigger={trigger:?}",
                            shortcut_injection_note(echo)
                        ));
                    }
                    return terminal_action;
                }
                Err(error) => {
                    mapping_note(format!("map_inject result=err button={button:?} trigger={trigger:?} error_domain=send_input error_code=injection_failed reason=backend_rejected retryable=true"));
                    lock_state(state).last_error = Some(format!("注入快捷键失败：{error}"));
                }
            }
        }
        ButtonAction::OpenApp { target } => {
            let target_kind = if target.contains('\\') || target.contains('/') {
                "custom"
            } else {
                "preset"
            };
            let profile = open_app_effective_profile(&mappings, &target);
            let strategy = profile
                .as_ref()
                .map(|profile| profile.strategy.as_str())
                .unwrap_or("none");
            crate::ble::gatt_note(format!(
                "map_fire button={:?} trigger={:?} action=open_app target_kind={target_kind} focus_strategy={strategy}",
                button, trigger,
            ));
            let launched = match injector.launch_app_and_focus(&target, profile.as_ref(), None) {
                Ok(()) => {
                    crate::ble::gatt_note(format!(
                        "map_launch result=ok target_kind={} focus_strategy={}",
                        target_kind, strategy
                    ));
                    true
                }
                Err(error) => {
                    crate::ble::gatt_note(format!(
                        "map_launch result=err target_kind={} focus_strategy={} error_domain=shell error_code=launch_failed reason=target_unavailable retryable=true",
                        target_kind, strategy
                    ));
                    lock_state(state).last_error = Some(format!("打开应用失败：{error}"));
                    false
                }
            };
            // 前台切换是异步的：先按实际前台同步一次，再按刚打开的目标应用
            // 预备覆盖层。否则用户在窗口还没拿到前台时按下第一个键，仍会
            // 落回上一个应用的基础/专用方案（2026-10-28 网易云音乐冷启动）。
            let host = crate::button_profiles::current_button_profile_host();
            host.sync_foreground();
            if launched {
                host.target_opened(&target);
            }
        }
        ButtonAction::RunMacro { macro_id, .. } => {
            // 组合动作由运行时在独立线程异步执行（不阻塞引擎线程）；引用按
            // **最新已保存版本**解析（Mac 语义：编辑后所有引用自动生效），
            // 保存时所指的 macro_version 只作审计。
            crate::ble::gatt_note(format!(
                "map_fire button={:?} trigger={:?} action=run_macro macro_id={macro_id}",
                button, trigger
            ));
            match macro_runtime.start(&macro_id) {
                Ok(()) => crate::ble::gatt_note(format!(
                    "map_macro macro_id={macro_id} phase=accepted terminal_result=deferred"
                )),
                Err(error) => {
                    let reason = match error {
                        crate::macro_runtime::MacroStartError::ShutDown => "shut_down",
                        crate::macro_runtime::MacroStartError::AlreadyRunning => "already_running",
                        crate::macro_runtime::MacroStartError::NotFound => "not_found",
                        crate::macro_runtime::MacroStartError::Invalid(_) => "invalid_macro",
                    };
                    crate::ble::gatt_note(format!(
                        "map_macro macro_id={macro_id} phase=requested terminal_result=failed reason={reason}"
                    ));
                    lock_state(state).last_error = Some("组合动作未能开始执行".to_owned());
                }
            }
        }
        ButtonAction::FocusInput => {
            crate::ble::gatt_note(format!(
                "map_fire button={:?} trigger={:?} action=focus_input",
                button, trigger
            ));
            // 异步受理：聚焦要重试并读回，结果由聚焦服务经 focus_report 汇报，
            // 这里只记录「已受理/无法受理」。
            match injector.focus_frontmost() {
                Ok(()) => crate::ble::gatt_note(
                    "map_focus_input phase=accepted terminal_result=deferred".to_owned(),
                ),
                Err(error) => {
                    crate::ble::gatt_note(
                        "map_focus_input phase=requested terminal_result=failed reason=service_unavailable"
                            .to_owned(),
                    );
                    lock_state(state).last_error = Some(format!("聚焦输入框失败：{error}"));
                }
            }
        }
        ButtonAction::AppSwitcher => {
            let io = InjectorAppSwitcherIo(injector.as_ref());
            let was_active = app_switcher.is_active();
            if !was_active {
                // 会话期间 OK/返回/方向/TV 由门控直接吞下：这些键即使没有
                // 映射，原生动作也会漏进系统切换器或目标应用。
                key_gate::set_app_switcher_mask(app_switcher::control_mask());
                // 常用语面板也认领方向/OK；两个短时窗口不能同时存在。
                injector.common_phrase_host().close("app_switcher");
            }
            let result = if was_active {
                app_switcher.advance(now, &io)
            } else {
                app_switcher.start(button, now, &io)
            };
            match result {
                Ok(operation) => crate::ble::gatt_note(format!(
                    "map_fire button={button:?} trigger={trigger:?} action=app_switcher phase={} operation={operation} result=submitted",
                    if was_active { "tab" } else { "start" }
                )),
                Err(error) => {
                    crate::ble::gatt_note(format!(
                        "map_fire button={button:?} trigger={trigger:?} action=app_switcher phase={} result=failed error_domain=send_input retryable=true error={error}",
                        if was_active { "tab" } else { "start" }
                    ));
                    lock_state(state).last_error = Some("打开应用切换器失败，请重试".to_owned());
                }
            }
            drain_app_switcher_end(app_switcher, state);
            if !app_switcher.is_active() {
                // AltDown/Tab 注入失败或会话立即收尾：遮蔽必须 fail-open。
                key_gate::set_app_switcher_mask(0);
            }
        }
        ButtonAction::OpenCommonPhrases => {
            if app_switcher.is_active() {
                // 切换器显示期间打开常用语面板会让面板与系统切换器争夺
                // 方向/OK；直接忽略本次动作，会话仍可用返回键取消。
                crate::ble::gatt_note(format!(
                    "map_app_switcher operation={} phase=panel_ignored reason=session_active button={button:?} trigger={trigger:?}",
                    app_switcher.operation_id()
                ));
                return false;
            }
            let host = injector.common_phrase_host();
            // 同一按键再按 = 收起面板（对齐 macOS：isVisible 且来源相同 → close(toggle_button)）。
            // 面板打开时也消费这次按键（不重复触发打开动作）。
            let toggled_closed = host.is_panel_open();
            let opened = if toggled_closed {
                host.close("toggle_button");
                false
            } else {
                host.open(crate::common_phrases::PHRASE_SOURCE_DEFAULT)
            };
            crate::ble::gatt_note(format!(
                "map_fire button={button:?} trigger={trigger:?} action=open_common_phrases result={} access={:?}",
                if toggled_closed {
                    "closed"
                } else if opened {
                    "opened"
                } else {
                    "unavailable"
                },
                host.access()
            ));
            if !opened && !toggled_closed {
                lock_state(state).last_error =
                    Some("常用语当前不可用（需要会员，或未接入常用语组件）".to_owned());
            }
        }
    }
    false
}

/// 锁屏动作（Win+L）不走 SendInput：`map_inject` 行必须显式说明"无回声是
/// 设计如此"，否则会被读成"钩子没看到注入"（那是 SendInput 分支的失败判据）。
const LOCK_WORKSTATION_INJECT_NOTE: &str =
    "map_inject result=queued note=lock_workstation_via_api_no_sendinput_echo";

/// 注入回声的有界等待预算：与门控的 60ms 有界等待同源——注入事件要先经
/// 系统输入流回到宿主钩子，再由宿主经本机 IPC 送回主进程读线程；引擎线程
/// 在这里做有界轮询，等到即返回（正常路径首轮即命中，不引入固定延迟）。
const INJECTION_ECHO_WAIT_MS: u64 = 60;

/// 等待并读取本次注入的回声读数。门控未运行（宿主钩子未安装/已断开）时
/// 不等待：此时"没有回声"是预期的，直接记录读数即可（避免白等 60ms）。
fn observe_injection_echo(before: u64) -> crate::key_gate::InjectedEcho {
    // 单测环境没有真实宿主钩子，等也等不到回声：直接读数，不给每个注入
    // 用例强加 60ms（诊断路径的测试豁免，不影响生产行为）。
    if cfg!(test) || !crate::key_gate::is_gate_thread_alive() {
        return crate::key_gate::injected_echo_since(before);
    }
    let deadline = Instant::now() + std::time::Duration::from_millis(INJECTION_ECHO_WAIT_MS);
    loop {
        let echo = crate::key_gate::injected_echo_since(before);
        if echo.count > 0 || Instant::now() >= deadline {
            return echo;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
}

/// `map_inject` 日志行（纯函数，含回声分支）：
/// - `result=queued`：事件已入队，**不声称成功**；
/// - `echo_delta>0`：宿主钩子确实看到注入事件经过系统输入流（`last_*` 是
///   最后一条的 vk/扫描码/flags，可当场核对目标键与扫描码）；
/// - `echo_delta=0`：这次注入没有被宿主钩子看到——要么宿主/钩子不在，
///   要么事件在到达本进程钩子之前就被别的钩子丢掉了。前台是否可见都不能
///   由本行推出，仍需真机验收。
fn shortcut_injection_note(echo: crate::key_gate::InjectedEcho) -> String {
    if echo.count == 0 {
        "map_inject result=queued echo_delta=0 note=hook_did_not_observe_injected_key".to_owned()
    } else {
        format!(
            "map_inject result=queued echo_delta={} last_vk=0x{:02X} last_scan=0x{:02X} last_flags=0x{:X}",
            echo.count,
            echo.vk & 0xFF,
            echo.scan & 0xFF,
            echo.flags
        )
    }
}

fn read_lock<T>(mutex: &RwLock<T>) -> std::sync::RwLockReadGuard<'_, T> {
    mutex
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn read_callbacks<T>(mutex: &RwLock<Vec<T>>) -> std::sync::RwLockReadGuard<'_, Vec<T>> {
    read_lock(mutex)
}

fn lock_state(state: &Mutex<EngineState>) -> std::sync::MutexGuard<'_, EngineState> {
    state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn lock_snapshot(
    snapshot: &Mutex<RawInputSnapshot>,
) -> std::sync::MutexGuard<'_, RawInputSnapshot> {
    snapshot
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raw_input::RemoteButton;
    use crate::send_input::{ButtonAction, ButtonActions, KeyCode};
    use std::sync::Mutex as StdMutex;
    use std::time::{Duration, Instant};

    // Production has one process-wide gate; test runtimes must own it exclusively.
    use crate::key_gate::GATE_TEST_LOCK;

    #[test]
    fn switcher_controls_extend_report_targets_without_changing_saved_mappings() {
        let _gate_lock = crate::key_gate::lock_gate_tests();
        key_gate::set_app_switcher_mask(0);
        let configuration = InputConfiguration::default();
        let before = configuration.requested_mask();
        key_gate::set_app_switcher_mask(app_switcher::control_mask());
        assert_eq!(
            configuration.requested_mask(),
            before | app_switcher::control_mask()
        );
        assert_eq!(configuration.mappings, ButtonMappings::default());
        key_gate::set_app_switcher_mask(0);
        assert_eq!(configuration.requested_mask(), before);
    }

    #[test]
    fn voice_barrier_requires_release_ack_and_rejects_timeout() {
        let (sender, receiver) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            if let EngineMessage::VoiceSessionStarted(ack) = receiver.recv().unwrap() {
                ack.send(false).unwrap();
            } else {
                panic!("expected voice release barrier");
            }
        });
        assert!(release_switcher_before_voice(&sender, Duration::from_millis(60)).is_err());
        worker.join().unwrap();
        let (sender, _blocked_receiver) = mpsc::channel();
        assert!(release_switcher_before_voice(&sender, Duration::ZERO).is_err());
        let (sender, receiver) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            if let EngineMessage::VoiceSessionStarted(ack) = receiver.recv().unwrap() {
                ack.send(true).unwrap();
            }
        });
        assert!(release_switcher_before_voice(&sender, Duration::from_millis(60)).is_ok());
        worker.join().unwrap();
    }

    #[test]
    fn mapping_diagnostics_explain_each_received_edge_without_changing_execution() {
        let _gate_test = GATE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let gate = crate::key_gate::KeyGate::start();
        for scenario in [
            "unowned", "waiting", "handled", "blocked", "disabled", "ok", "err",
        ] {
            *MAPPING_TEST_NOTES.lock().unwrap() = Some(Vec::new());
            let injector = Arc::new(RecordingInjector {
                fail: scenario == "err",
                ..Default::default()
            });
            let runtime = ButtonMappingRuntime::new(
                injector.clone(),
                Arc::new(UsageCounters::default()),
                Arc::new(StdMutex::new(RawInputSnapshot::default())),
                Arc::new(crate::macro_runtime::MacroRuntime::new(Arc::new(
                    crate::macro_runtime::UnsupportedStepperSource,
                ))),
            );
            runtime.set_input_context(crate::RemoteModel::Rc003, true);
            let mut mapping = mappings_with_single(RemoteButton::Back, KeyCode::Backspace);
            if scenario == "waiting" {
                let actions = mapping.actions.get_mut(&RemoteButton::Back).unwrap();
                actions.double = actions.single.clone();
            }
            if scenario == "disabled" {
                runtime.set_application_mappings(None, mapping);
            } else {
                runtime.set_mappings(mapping);
            }
            if scenario != "unowned" {
                runtime.set_input_enhancement(true);
            }
            if matches!(scenario, "handled" | "blocked") {
                let disposition = if scenario == "handled" {
                    GestureDisposition::Handled
                } else {
                    GestureDisposition::Blocked
                };
                runtime.set_gesture_handler(Some(Arc::new(move |_| disposition)));
            }
            flush(&runtime);
            runtime
                .sender()
                .send(EngineMessage::DriverEdge(ButtonEdge {
                    button: RemoteButton::Back,
                    is_pressed: true,
                }))
                .unwrap();
            flush(&runtime);
            let notes = MAPPING_TEST_NOTES
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .join("\n");
            assert!(
                notes.contains("map_gesture button=Back pressed=true"),
                "{scenario}: missing recognizer evidence: {notes}"
            );
            match scenario {
                "unowned" => {
                    assert!(
                        notes.contains(
                            "captured_mask=0 requested_mask=1 execution_mask=0 recognition_mask=0"
                        ),
                        "{notes}"
                    );
                    assert!(
                        notes.contains("configured=false fired_count=0 reason=unconfigured"),
                        "{notes}"
                    );
                }
                "waiting" => assert!(
                    notes.contains("configured=true fired_count=0 reason=waiting_gesture"),
                    "{notes}"
                ),
                "handled" => assert!(notes.contains("disposition=Handled"), "{notes}"),
                "blocked" => assert!(notes.contains("disposition=Blocked"), "{notes}"),
                "disabled" => assert!(
                    notes.contains("reason=action_disabled button=Back trigger=Single"),
                    "{notes}"
                ),
                "ok" => assert!(
                    notes
                        .lines()
                        .any(|line| line.starts_with("map_inject result=queued ")
                            && line.contains("button=Back trigger=Single")),
                    "{notes}"
                ),
                "err" => assert!(
                    notes.contains("map_inject result=err button=Back trigger=Single"),
                    "{notes}"
                ),
                _ => unreachable!(),
            }
            assert!(
                !notes.contains("chord="),
                "diagnostics must not disclose configured keys: {notes}"
            );
            assert_eq!(
                injector.taps.lock().unwrap().len(),
                usize::from(scenario == "ok")
            );
            runtime
                .sender()
                .send(EngineMessage::DriverEdge(ButtonEdge {
                    button: RemoteButton::Back,
                    is_pressed: false,
                }))
                .unwrap();
            flush(&runtime);
            drop(runtime);
        }
        drop(gate);
        *MAPPING_TEST_NOTES.lock().unwrap() = None;
    }

    #[test]
    fn mapping_diagnostics_deduplicate_unchanged_state_and_explain_cancelled_release() {
        let _gate_test = GATE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let gate = crate::key_gate::KeyGate::start();
        *MAPPING_TEST_NOTES.lock().unwrap() = Some(Vec::new());
        let injector = Arc::new(RecordingInjector::default());
        let runtime = ButtonMappingRuntime::new(
            injector.clone(),
            Arc::new(UsageCounters::default()),
            Arc::new(StdMutex::new(RawInputSnapshot::default())),
            Arc::new(crate::macro_runtime::MacroRuntime::new(Arc::new(
                crate::macro_runtime::UnsupportedStepperSource,
            ))),
        );
        runtime.set_input_context(crate::RemoteModel::Rc003, true);
        let mut mapping = ButtonMappings::default();
        let buttons = [
            RemoteButton::Back,
            RemoteButton::Menu,
            RemoteButton::VolumeUp,
            RemoteButton::VolumeDown,
        ];
        for button in buttons {
            mapping
                .actions
                .extend(mappings_with_single(button, KeyCode::Backspace).actions);
        }
        runtime.set_mappings(mapping.clone());
        runtime.set_input_enhancement(true);
        flush(&runtime);
        let before = MAPPING_TEST_NOTES.lock().unwrap().as_ref().unwrap().len();
        for _ in 0..10 {
            runtime.set_mappings(mapping.clone());
        }
        flush(&runtime);
        assert_eq!(
            MAPPING_TEST_NOTES.lock().unwrap().as_ref().unwrap().len(),
            before,
            "unchanged capability state must remain quiet"
        );
        for button in buttons {
            for is_pressed in [true, true, false, false] {
                runtime
                    .sender()
                    .send(EngineMessage::DriverEdge(ButtonEdge { button, is_pressed }))
                    .unwrap();
            }
        }
        flush(&runtime);
        assert_eq!(injector.taps.lock().unwrap().len(), buttons.len());
        let notes = MAPPING_TEST_NOTES
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .join("\n");
        assert_eq!(
            notes
                .lines()
                .filter(|line| line.starts_with("map_gesture "))
                .count(),
            8,
            "only real logical edges should produce recognizer diagnostics"
        );
        for button in buttons {
            assert!(
                notes.contains(&format!("map_gesture button={button:?} pressed=true")),
                "{notes}"
            );
        }
        runtime
            .sender()
            .send(EngineMessage::DriverEdge(ButtonEdge {
                button: RemoteButton::Back,
                is_pressed: true,
            }))
            .unwrap();
        flush(&runtime);
        mapping.actions.get_mut(&RemoteButton::Back).unwrap().single = ButtonAction::Shortcut {
            chord: KeyChord {
                keys: vec![KeyCode::Escape],
            },
        };
        runtime.set_mappings(mapping);
        flush(&runtime);
        runtime
            .sender()
            .send(EngineMessage::DriverEdge(ButtonEdge {
                button: RemoteButton::Back,
                is_pressed: false,
            }))
            .unwrap();
        flush(&runtime);
        let notes = MAPPING_TEST_NOTES
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .join("\n");
        assert!(notes.contains("map_gesture button=Back pressed=false accepted=false fired_count=0 reason=waiting_release"), "{notes}");
        assert_eq!(
            injector.taps.lock().unwrap().len(),
            buttons.len() + 1,
            "cancelled UP must not execute the replacement mapping"
        );
        drop(runtime);
        drop(gate);
        *MAPPING_TEST_NOTES.lock().unwrap() = None;
    }

    #[test]
    fn unified_capture_targets_include_profile_and_menu_without_common_actions() {
        let _gate_test = GATE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let runtime = ButtonMappingRuntime::new(
            Arc::new(RecordingInjector::default()),
            Arc::new(UsageCounters::default()),
            Arc::new(StdMutex::new(RawInputSnapshot::default())),
            Arc::new(crate::macro_runtime::MacroRuntime::new(Arc::new(
                crate::macro_runtime::UnsupportedStepperSource,
            ))),
        );
        runtime.set_input_context(crate::RemoteModel::Rc003, true);
        let profile = mappings_with_single(RemoteButton::Ok, KeyCode::Enter);
        let scene = mappings_with_single(RemoteButton::Menu, KeyCode::Escape);
        runtime.set_application_mappings(Some(profile), scene);
        assert_eq!(
            runtime.requested_capture_mask() as u64,
            (1 << RemoteButton::Ok.ordinal()) | (1 << RemoteButton::Menu.ordinal())
        );
    }

    #[test]
    fn unified_capture_capabilities_follow_exact_current_targets_and_connection() {
        let _gate_test = GATE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let runtime = ButtonMappingRuntime::new(
            Arc::new(RecordingInjector::default()),
            Arc::new(UsageCounters::default()),
            Arc::new(StdMutex::new(RawInputSnapshot::default())),
            Arc::new(crate::macro_runtime::MacroRuntime::new(Arc::new(
                crate::macro_runtime::UnsupportedStepperSource,
            ))),
        );
        runtime.set_input_context(crate::RemoteModel::Rc003, true);
        runtime.set_application_mappings(
            Some(mappings_with_single(RemoteButton::Ok, KeyCode::Enter)),
            mappings_with_single(RemoteButton::Menu, KeyCode::Escape),
        );
        // Ordinary native events remain available but cannot authorize any mapping.
        assert_eq!(
            runtime
                .configuration
                .lock()
                .unwrap()
                .effective_mappings()
                .1
                .mapped_mask(),
            0
        );
        let _ = runtime.set_capture_owned(1 << RemoteButton::Menu.ordinal());
        assert_eq!(
            runtime
                .configuration
                .lock()
                .unwrap()
                .effective_mappings()
                .1
                .mapped_mask(),
            1 << RemoteButton::Menu.ordinal()
        );
        runtime.set_application_mappings(
            Some(mappings_with_single(RemoteButton::Right, KeyCode::Right)),
            mappings_with_single(RemoteButton::Menu, KeyCode::Escape),
        );
        let _ = runtime.set_capture_owned(1 << RemoteButton::Ok.ordinal()); // stale target ACK
        assert_eq!(
            runtime
                .configuration
                .lock()
                .unwrap()
                .effective_mappings()
                .1
                .mapped_mask(),
            0
        );
        runtime.set_input_context(crate::RemoteModel::Unknown, false);
        let _ = runtime.set_capture_owned((1 << 13) - 1); // late callback after disconnect
        assert_eq!(
            runtime
                .configuration
                .lock()
                .unwrap()
                .effective_mappings()
                .1
                .mapped_mask(),
            0
        );
    }

    #[test]
    fn observed_host_hold_survives_mapping_cancel_without_duplicate_edges() {
        let _gate_test = GATE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let observation = ButtonObservation::default();
        let events = Arc::new(StdMutex::new(Vec::new()));
        let sink = events.clone();
        observation
            .callbacks
            .write()
            .unwrap()
            .push(Arc::new(move |edge| sink.lock().unwrap().push(edge)));
        let down = ButtonEdge {
            button: RemoteButton::Back,
            is_pressed: true,
        };
        let up = ButtonEdge {
            is_pressed: false,
            ..down
        };
        observation.physical(1);
        observation.physical(1);
        observation.mapped_edge(down);
        observation.mapped_edge(up); // configuration cancels the action, not the physical hold
        assert_eq!(observation.active(), vec![RemoteButton::Back]);
        assert_eq!(*events.lock().unwrap(), vec![down]);
        observation.physical(0);
        observation.clear();
        assert_eq!(*events.lock().unwrap(), vec![down, up]);
        // Reverse arrival order is also a union, including existing Raw Input TV/Home.
        observation.mapped_edge(down);
        observation.physical(1);
        observation.physical(0);
        observation.mapped_edge(up);
        assert_eq!(*events.lock().unwrap(), vec![down, up, down, up]);
    }

    #[test]
    fn host_observation_without_actions_does_not_execute_or_feed_scene_edges() {
        let _gate_test = GATE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let gate = crate::key_gate::KeyGate::start();
        key_gate::set_listener_active(true);
        let injector = Arc::new(RecordingInjector::default());
        let runtime = ButtonMappingRuntime::new(
            injector.clone(),
            Arc::new(UsageCounters::default()),
            Arc::new(StdMutex::new(RawInputSnapshot::default())),
            Arc::new(crate::macro_runtime::MacroRuntime::new(Arc::new(
                crate::macro_runtime::UnsupportedStepperSource,
            ))),
        );
        runtime.set_input_context(crate::RemoteModel::Rc003, true);
        let observed = Arc::new(StdMutex::new(Vec::new()));
        let sink = observed.clone();
        runtime
            .subscribe_button_observations(Arc::new(move |edge| sink.lock().unwrap().push(edge)));
        let execution = Arc::new(StdMutex::new(Vec::new()));
        let sink = execution.clone();
        runtime.subscribe_button_edges(Arc::new(move |edge| sink.lock().unwrap().push(edge)));
        let sender = runtime.sender();
        sender.send(EngineMessage::HidObservation(7)).unwrap();
        sender.send(EngineMessage::HidObservation(7)).unwrap();
        flush(&runtime);
        assert_eq!(runtime.requested_capture_mask(), 0);
        assert_eq!(runtime.snapshot().observed_buttons.len(), 3);
        assert_eq!(observed.lock().unwrap().len(), 3);
        assert!(execution.lock().unwrap().is_empty());
        assert_eq!(runtime.snapshot().fired_gestures, 0);
        assert!(injector.taps.lock().unwrap().is_empty());
        key_gate::set_listener_active(false);
        sender.send(EngineMessage::ListenerStopped).unwrap();
        sender.send(EngineMessage::HidObservation(7)).unwrap();
        flush(&runtime);
        assert!(runtime.snapshot().observed_buttons.is_empty());
        assert_eq!(observed.lock().unwrap().len(), 6);
        drop(gate);
    }

    #[test]
    fn host_observation_and_mapped_edges_publish_once_and_execute_once() {
        let _gate_test = GATE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let gate = crate::key_gate::KeyGate::start();
        key_gate::set_listener_active(true);
        let injector = Arc::new(RecordingInjector::default());
        let runtime = ButtonMappingRuntime::new(
            injector.clone(),
            Arc::new(UsageCounters::default()),
            Arc::new(StdMutex::new(RawInputSnapshot::default())),
            Arc::new(crate::macro_runtime::MacroRuntime::new(Arc::new(
                crate::macro_runtime::UnsupportedStepperSource,
            ))),
        );
        runtime.set_input_context(crate::RemoteModel::Rc003, true);
        runtime.set_mappings(mappings_with_single(RemoteButton::Back, KeyCode::Backspace));
        runtime.set_input_enhancement(true);
        let observed = Arc::new(StdMutex::new(Vec::new()));
        let sink = observed.clone();
        runtime
            .subscribe_button_observations(Arc::new(move |edge| sink.lock().unwrap().push(edge)));
        let sender = runtime.sender();
        for pressed in [true, false] {
            sender
                .send(EngineMessage::HidObservation(u64::from(pressed)))
                .unwrap();
            sender
                .send(EngineMessage::DriverEdge(ButtonEdge {
                    button: RemoteButton::Back,
                    is_pressed: pressed,
                }))
                .unwrap();
        }
        flush(&runtime);
        assert_eq!(observed.lock().unwrap().len(), 2);
        assert_eq!(injector.taps.lock().unwrap().len(), 1);
        assert_eq!(runtime.snapshot().fired_gestures, 1);
        sender.send(EngineMessage::HidObservation(2)).unwrap();
        flush(&runtime); // establish the observed hold before disconnecting
        runtime.set_input_context(crate::RemoteModel::Rc003, false);
        sender.send(EngineMessage::DeviceRemoved).unwrap();
        flush(&runtime);
        assert!(runtime.snapshot().observed_buttons.is_empty());
        assert_eq!(observed.lock().unwrap().len(), 4);
        sender.send(EngineMessage::HidObservation(4)).unwrap();
        flush(&runtime);
        assert_eq!(observed.lock().unwrap().len(), 4); // late old report cannot relight
        runtime.set_input_context(crate::RemoteModel::Rc003, false);
        runtime.set_input_context(crate::RemoteModel::Rc003, true);
        sender.send(EngineMessage::HidObservation(4)).unwrap();
        flush(&runtime);
        drop(runtime);
        assert_eq!(observed.lock().unwrap().len(), 6);
        drop(gate);
    }

    fn home_keyboard(pressed: bool) -> RawKeyboardEvent {
        RawKeyboardEvent {
            virtual_key: 0x24,
            make_code: 0x47,
            flags: if pressed { 0 } else { 1 },
            message: if pressed { 0x0100 } else { 0x0101 },
        }
    }

    #[test]
    fn unified_capture_never_filters_unowned_raw_events() {
        let merger = ButtonStateMerger::default();
        for pressed in [true, false] {
            assert!(!ignore_host_owned_keyboard(
                home_keyboard(pressed),
                &merger,
                0
            ));
            assert!(!ignore_host_owned_keyboard(
                home_keyboard(pressed),
                &merger,
                1 << RemoteButton::Menu.ordinal()
            ));
        }
        let menu = RawKeyboardEvent {
            virtual_key: 0x5D,
            ..home_keyboard(true)
        };
        assert!(ignore_host_owned_keyboard(
            menu,
            &merger,
            1 << RemoteButton::Menu.ordinal()
        ));
        assert!(!ignore_host_owned_keyboard(
            menu,
            &merger,
            1 << RemoteButton::Home.ordinal()
        ));
    }

    #[test]
    fn host_takeover_drains_only_the_selected_raw_source_existing_down() {
        let _gate_test = GATE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        for _ in 0..32 {
            let mut merger = ButtonStateMerger::default();
            assert_eq!(merger.update_keyboard(home_keyboard(true)).len(), 1);
            // The source becomes ready between its original native DOWN and UP.
            assert!(!ignore_host_owned_keyboard(
                home_keyboard(false),
                &merger,
                1 << RemoteButton::Home.ordinal()
            ));
            let released = merger.update_keyboard(home_keyboard(false));
            assert_eq!(
                released,
                vec![ButtonEdge {
                    button: RemoteButton::Home,
                    is_pressed: false
                }]
            );
            assert!(merger.active_button_set().is_empty());
            assert!(ignore_host_owned_keyboard(
                home_keyboard(false),
                &merger,
                1 << RemoteButton::Home.ordinal()
            ));
            assert!(ignore_host_owned_keyboard(
                home_keyboard(true),
                &merger,
                1 << RemoteButton::Home.ordinal()
            ));
        }
    }

    #[test]
    fn host_takeover_raw_release_cannot_cancel_or_duplicate_driver_release() {
        let _gate_test = GATE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut merger = ButtonStateMerger::default();
        let down = ButtonEdge {
            button: RemoteButton::Home,
            is_pressed: true,
        };
        let up = ButtonEdge {
            button: RemoteButton::Home,
            is_pressed: false,
        };
        merger.apply_driver_button_edge(down);
        // A same-VK UP with no held raw DOWN is rejected, not lent to the driver source.
        assert!(ignore_host_owned_keyboard(
            home_keyboard(false),
            &merger,
            1 << RemoteButton::Home.ordinal()
        ));
        assert!(merger.active_button_set().contains(&RemoteButton::Home));
        assert_eq!(merger.apply_driver_button_edge(up), vec![up]);

        merger.update_keyboard(home_keyboard(true));
        merger.apply_driver_button_edge(down);
        assert!(!ignore_host_owned_keyboard(
            home_keyboard(false),
            &merger,
            1 << RemoteButton::Home.ordinal()
        ));
        assert!(merger.update_keyboard(home_keyboard(false)).is_empty());
        assert!(merger.active_button_set().contains(&RemoteButton::Home));
        assert_eq!(merger.apply_driver_button_edge(up), vec![up]);
        assert!(ignore_host_owned_keyboard(
            home_keyboard(false),
            &merger,
            1 << RemoteButton::Home.ordinal()
        ));
    }

    fn flush(runtime: &ButtonMappingRuntime) {
        assert!(runtime.wait_for_idle(Duration::from_secs(2)));
    }

    #[test]
    fn mapping_notice_follows_consumed_mapping_and_connection_changes() {
        let _gate_test = GATE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let runtime = ButtonMappingRuntime::new(
            Arc::new(RecordingInjector::default()),
            Arc::new(UsageCounters::default()),
            Arc::new(StdMutex::new(RawInputSnapshot::default())),
            Arc::new(crate::macro_runtime::MacroRuntime::new(Arc::new(
                crate::macro_runtime::UnsupportedStepperSource,
            ))),
        );
        let values = Arc::new(StdMutex::new(Vec::new()));
        runtime.set_input_context(crate::RemoteModel::Rc001, true);
        let mut mappings = ButtonMappings::default();
        mappings.actions.insert(
            RemoteButton::Back,
            ButtonActions {
                single: ButtonAction::Shortcut {
                    chord: KeyChord {
                        keys: vec![KeyCode::Backspace],
                    },
                },
                ..ButtonActions::default()
            },
        );
        runtime.set_mappings(mappings);
        let observed = Arc::clone(&values);
        runtime.publish_mapping_notice(move |available| observed.lock().unwrap().push(available));
        flush(&runtime);
        assert_eq!(*values.lock().unwrap(), vec![true]);
        runtime.set_input_context(crate::RemoteModel::Rc001, false);
        flush(&runtime);
        assert_eq!(*values.lock().unwrap(), vec![true, false]);
    }

    #[test]
    fn host_configuration_barrier_observes_old_raw_down_and_its_paired_release() {
        let _gate_test = GATE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let runtime = ButtonMappingRuntime::new(
            Arc::new(RecordingInjector::default()),
            Arc::new(UsageCounters::default()),
            Arc::new(StdMutex::new(RawInputSnapshot::default())),
            Arc::new(crate::macro_runtime::MacroRuntime::new(Arc::new(
                crate::macro_runtime::UnsupportedStepperSource,
            ))),
        );
        let timeout = Duration::from_secs(2);
        assert_eq!(runtime.host_raw_released(timeout), Some(true));
        runtime
            .sender()
            .send(EngineMessage::Keyboard(home_keyboard(true)))
            .unwrap();
        // The ordinary shutdown barrier still succeeds with a physically held key.
        assert!(runtime.wait_for_idle(timeout));
        assert_eq!(runtime.host_raw_released(timeout), Some(false));
        runtime
            .sender()
            .send(EngineMessage::Keyboard(home_keyboard(false)))
            .unwrap();
        assert_eq!(runtime.host_raw_released(timeout), Some(true));
        runtime
            .sender()
            .send(EngineMessage::DriverEdge(ButtonEdge {
                button: RemoteButton::Home,
                is_pressed: true,
            }))
            .unwrap();
        // Driver ownership is separately checked by the Helper, not confused with raw input.
        assert_eq!(runtime.host_raw_released(timeout), Some(true));
        runtime
            .sender()
            .send(EngineMessage::DriverEdge(ButtonEdge {
                button: RemoteButton::Home,
                is_pressed: false,
            }))
            .unwrap();
        drop(runtime);
    }

    #[test]
    fn input_context_gates_vendor_keys_without_erasing_configuration() {
        let _gate_test = GATE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut configuration = InputConfiguration::default();
        for button in [
            RemoteButton::Back,
            RemoteButton::VolumeUp,
            RemoteButton::VolumeDown,
        ] {
            configuration.mappings.actions.insert(
                button,
                ButtonActions {
                    single: ButtonAction::Shortcut {
                        chord: KeyChord {
                            keys: vec![KeyCode::Escape],
                        },
                    },
                    ..ButtonActions::default()
                },
            );
        }
        let saved = configuration.mappings.clone();
        for (model, connected, available) in [
            (crate::RemoteModel::Unknown, true, false),
            (crate::RemoteModel::Rc003, true, false),
            (crate::RemoteModel::Rc001, false, false),
            (crate::RemoteModel::Rc001, true, true),
        ] {
            configuration.model = model;
            configuration.connected = connected;
            let effective = configuration.effective_mappings();
            for button in [
                RemoteButton::Back,
                RemoteButton::VolumeUp,
                RemoteButton::VolumeDown,
            ] {
                assert_eq!(
                    effective.0.action_for(button, ButtonTrigger::Single) != ButtonAction::Disabled,
                    available
                );
            }
            assert_eq!(configuration.mappings, saved);
            assert_eq!(effective.0.enabled, connected && saved.enabled);
        }
    }

    #[test]
    fn driver_capability_requires_known_connection_and_preserves_saved_actions() {
        let _gate_test = GATE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let runtime = ButtonMappingRuntime::new(
            Arc::new(RecordingInjector::default()),
            Arc::new(UsageCounters::default()),
            Arc::new(StdMutex::new(RawInputSnapshot::default())),
            Arc::new(crate::macro_runtime::MacroRuntime::new(Arc::new(
                crate::macro_runtime::UnsupportedStepperSource,
            ))),
        );
        let saved = mappings_with_single(RemoteButton::Back, KeyCode::Backspace);
        runtime.set_mappings(saved.clone());
        assert_eq!(runtime.requested_capture_mask(), 0);
        runtime.set_input_context(crate::RemoteModel::Rc003, true);
        assert_eq!(runtime.requested_capture_mask(), 1);
        runtime.set_input_enhancement(true);
        assert!(runtime
            .configuration
            .lock()
            .unwrap()
            .effective_mappings()
            .0
            .actions
            .contains_key(&RemoteButton::Back));
        runtime.set_input_context(crate::RemoteModel::Unknown, true);
        assert_eq!(runtime.requested_capture_mask(), 0);
        assert!(!runtime
            .configuration
            .lock()
            .unwrap()
            .effective_mappings()
            .0
            .actions
            .contains_key(&RemoteButton::Back));
        runtime.set_input_enhancement(false);
        assert_eq!(runtime.mappings(), saved);
    }

    #[test]
    fn fixed_template_keys_and_combinations_fire_on_down_without_gesture_delay() {
        let _gate_test = GATE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let gate = crate::key_gate::KeyGate::start();
        let injector = Arc::new(RecordingInjector::default());
        let snapshot = Arc::new(StdMutex::new(RawInputSnapshot::default()));
        let runtime = ButtonMappingRuntime::new(
            injector.clone(),
            Arc::new(UsageCounters::default()),
            snapshot.clone(),
            Arc::new(crate::macro_runtime::MacroRuntime::new(Arc::new(
                crate::macro_runtime::UnsupportedStepperSource,
            ))),
        );
        runtime.set_input_context(crate::RemoteModel::Rc003, true);
        let mappings = crate::templates::MappingConfiguration::recommended_templates()
            .remove(0)
            .mappings;
        runtime.set_mappings(mappings.clone());
        runtime.set_input_enhancement(true);
        flush(&runtime);
        let mut expected = Vec::new();
        for _ in 0..7 {
            for button in [
                RemoteButton::Left,
                RemoteButton::Right,
                RemoteButton::Back,
                RemoteButton::VolumeUp,
                RemoteButton::VolumeDown,
                RemoteButton::Ok,
            ] {
                let ButtonAction::Shortcut { chord } =
                    mappings.action_for(button, ButtonTrigger::Single)
                else {
                    panic!("fixed key missing")
                };
                runtime
                    .sender()
                    .send(EngineMessage::GateEdge(ButtonEdge {
                        button,
                        is_pressed: true,
                    }))
                    .unwrap();
                flush(&runtime);
                expected.push(chord);
                assert_eq!(
                    *injector.taps.lock().unwrap(),
                    expected,
                    "each DOWN executes before UP without long/double delay"
                );
                runtime
                    .sender()
                    .send(EngineMessage::GateEdge(ButtonEdge {
                        button,
                        is_pressed: false,
                    }))
                    .unwrap();
                flush(&runtime);
                assert_eq!(
                    *injector.taps.lock().unwrap(),
                    expected,
                    "UP does not duplicate the mapping"
                );
            }
        }
        assert!(snapshot.lock().unwrap().active_buttons.is_empty());
        drop(runtime);
        drop(gate);
    }

    #[test]
    fn builtin_confirm_native_first_then_captured_presses_each_deliver_one_enter() {
        let _gate_test = GATE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let gate = crate::key_gate::KeyGate::start();
        let injector = Arc::new(RecordingInjector::default());
        let raw_snapshot = Arc::new(StdMutex::new(RawInputSnapshot::default()));
        let runtime = ButtonMappingRuntime::new(
            injector.clone(),
            Arc::new(UsageCounters::default()),
            raw_snapshot.clone(),
            Arc::new(crate::macro_runtime::MacroRuntime::new(Arc::new(
                crate::macro_runtime::UnsupportedStepperSource,
            ))),
        );
        runtime.set_input_context(crate::RemoteModel::Rc003, true);
        runtime.set_mappings(
            crate::templates::MappingConfiguration::recommended_templates()
                .remove(1)
                .mappings,
        );
        let routed = Arc::new(StdMutex::new(Vec::new()));
        let capture = routed.clone();
        runtime.set_gesture_handler(Some(Arc::new(move |event| {
            capture.lock().unwrap().push(event);
            GestureDisposition::PassThrough
        })));
        flush(&runtime);
        runtime
            .sender()
            .send(EngineMessage::Keyboard(keyboard_event(0x0D, 0x100)))
            .unwrap();
        flush(&runtime);
        assert!(
            injector.taps.lock().unwrap().is_empty(),
            "first native Enter must not be duplicated"
        );
        runtime
            .sender()
            .send(EngineMessage::Keyboard(keyboard_event(0x0D, 0x101)))
            .unwrap();
        flush(&runtime);
        for _ in 0..6 {
            let _ = runtime.set_capture_owned(1 << RemoteButton::Ok.ordinal());
            flush(&runtime);
            runtime
                .sender()
                .send(EngineMessage::DriverEdge(ButtonEdge {
                    button: RemoteButton::Ok,
                    is_pressed: true,
                }))
                .unwrap();
            flush(&runtime);
            runtime
                .sender()
                .send(EngineMessage::DriverEdge(ButtonEdge {
                    button: RemoteButton::Ok,
                    is_pressed: false,
                }))
                .unwrap();
            flush(&runtime);
        }
        assert_eq!(
            injector.taps.lock().unwrap().as_slice(),
            vec![
                KeyChord {
                    keys: vec![KeyCode::Enter]
                };
                6
            ]
        );
        let seen = routed.lock().unwrap();
        assert_eq!(seen.len(), 6);
        assert!(seen.iter().all(|v| !v.native_delivered));
        assert!(raw_snapshot.lock().unwrap().active_buttons.is_empty());
        drop(gate);
    }

    #[test]
    fn driver_channel_three_keys_execute_once_and_cancel_without_native_injection() {
        let _gate_test = GATE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let gate = crate::key_gate::KeyGate::start();
        let injector = Arc::new(RecordingInjector::default());
        let snapshot = Arc::new(StdMutex::new(RawInputSnapshot::default()));
        let runtime = ButtonMappingRuntime::new(
            injector.clone(),
            Arc::new(UsageCounters::default()),
            snapshot.clone(),
            Arc::new(crate::macro_runtime::MacroRuntime::new(Arc::new(
                crate::macro_runtime::UnsupportedStepperSource,
            ))),
        );
        runtime.set_input_context(crate::RemoteModel::Rc003, true);
        let mut mappings = ButtonMappings::default();
        for button in [
            RemoteButton::Back,
            RemoteButton::VolumeUp,
            RemoteButton::VolumeDown,
        ] {
            mappings.actions.insert(
                button,
                ButtonActions {
                    single: ButtonAction::Shortcut {
                        chord: KeyChord {
                            keys: vec![KeyCode::Escape],
                        },
                    },
                    ..ButtonActions::default()
                },
            );
        }
        runtime.set_mappings(mappings);
        runtime.set_input_enhancement(true);
        flush(&runtime);
        for button in [
            RemoteButton::Back,
            RemoteButton::VolumeUp,
            RemoteButton::VolumeDown,
        ] {
            for is_pressed in [true, true, false] {
                runtime
                    .sender()
                    .send(EngineMessage::DriverEdge(ButtonEdge { button, is_pressed }))
                    .unwrap();
            }
        }
        flush(&runtime);
        assert_eq!(injector.taps.lock().unwrap().len(), 3);
        assert!(snapshot.lock().unwrap().active_buttons.is_empty());
        runtime
            .sender()
            .send(EngineMessage::DriverEdge(ButtonEdge {
                button: RemoteButton::Back,
                is_pressed: true,
            }))
            .unwrap();
        flush(&runtime);
        let count = injector.taps.lock().unwrap().len();
        runtime.set_input_enhancement(false);
        runtime
            .sender()
            .send(EngineMessage::DriverEdge(ButtonEdge {
                button: RemoteButton::Back,
                is_pressed: false,
            }))
            .unwrap();
        flush(&runtime);
        assert_eq!(injector.taps.lock().unwrap().len(), count);
        assert!(snapshot.lock().unwrap().active_buttons.is_empty());
        drop(gate);
    }

    #[test]
    fn connection_context_enables_selected_profile_and_reconnect_restores_it() {
        let _gate_test = GATE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let runtime = ButtonMappingRuntime::new(
            Arc::new(RecordingInjector::default()),
            Arc::new(UsageCounters::default()),
            Arc::new(StdMutex::new(RawInputSnapshot::default())),
            Arc::new(crate::macro_runtime::MacroRuntime::new(Arc::new(
                crate::macro_runtime::UnsupportedStepperSource,
            ))),
        );
        let profile = mappings_with_single(RemoteButton::Up, KeyCode::Backspace);
        runtime.set_profile_mappings(Some(profile.clone()));
        assert!(
            !runtime
                .configuration
                .lock()
                .unwrap()
                .effective_mappings()
                .0
                .enabled
        );

        runtime.set_input_context(crate::RemoteModel::Rc001, true);
        runtime.set_input_context(crate::RemoteModel::Rc001, true);
        assert_eq!(
            runtime.configuration.lock().unwrap().effective_mappings().0,
            profile
        );

        runtime.set_input_context(crate::RemoteModel::Rc001, false);
        assert!(
            !runtime
                .configuration
                .lock()
                .unwrap()
                .effective_mappings()
                .0
                .enabled
        );

        runtime.set_input_context(crate::RemoteModel::Rc001, true);
        assert_eq!(
            runtime.configuration.lock().unwrap().effective_mappings().0,
            profile
        );
    }

    #[test]
    fn application_template_kind_switch_is_atomic_and_keeps_direct_execution_separate() {
        let _gate_test = GATE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let runtime = ButtonMappingRuntime::new(
            Arc::new(RecordingInjector::default()),
            Arc::new(UsageCounters::default()),
            Arc::new(StdMutex::new(RawInputSnapshot::default())),
            Arc::new(crate::macro_runtime::MacroRuntime::new(Arc::new(
                crate::macro_runtime::UnsupportedStepperSource,
            ))),
        );
        runtime.set_input_context(crate::RemoteModel::Rc001, true);

        let common = mappings_with_single(RemoteButton::Up, KeyCode::Backspace);
        let direct = mappings_with_single(RemoteButton::Left, KeyCode::Escape);
        let semantic = mappings_with_single(RemoteButton::Ok, KeyCode::Enter);
        let mut disabled_scene = ButtonMappings::default();
        disabled_scene.enabled = false;
        runtime.set_mappings(common.clone());

        runtime.set_application_mappings(Some(direct.clone()), disabled_scene.clone());
        {
            let configuration = runtime.configuration.lock().unwrap();
            let (execution, recognition) = configuration.effective_mappings();
            assert_eq!(execution, direct);
            assert_eq!(recognition, direct);
            assert_eq!(
                execution.action_for(RemoteButton::Ok, ButtonTrigger::Single),
                ButtonAction::Disabled
            );
        }

        runtime.set_application_mappings(None, semantic.clone());
        {
            let configuration = runtime.configuration.lock().unwrap();
            let (execution, recognition) = configuration.effective_mappings();
            assert_eq!(execution, common);
            assert_eq!(
                execution.action_for(RemoteButton::Ok, ButtonTrigger::Single),
                ButtonAction::Disabled
            );
            assert_eq!(
                recognition.action_for(RemoteButton::Ok, ButtonTrigger::Single),
                ButtonAction::Shortcut {
                    chord: KeyChord {
                        keys: vec![KeyCode::Escape]
                    }
                }
            );
        }

        runtime.set_application_mappings(None, disabled_scene);
        let configuration = runtime.configuration.lock().unwrap();
        let (execution, recognition) = configuration.effective_mappings();
        assert_eq!(execution, common);
        assert_eq!(recognition, common);
    }

    #[test]
    fn cancelled_input_holds_do_not_transfer_or_fabricate_clicks() {
        let _gate_test = GATE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        for cancellation in ["disable", "disconnect", "listener_restart", "model_switch"] {
            let runtime = ButtonMappingRuntime::new(
                Arc::new(RecordingInjector::default()),
                Arc::new(UsageCounters::default()),
                Arc::new(StdMutex::new(RawInputSnapshot::default())),
                Arc::new(crate::macro_runtime::MacroRuntime::new(Arc::new(
                    crate::macro_runtime::UnsupportedStepperSource,
                ))),
            );
            runtime.set_input_context(crate::RemoteModel::Rc001, true);
            let mut mappings = mappings_with_single(RemoteButton::Ok, KeyCode::Enter);
            mappings.actions.get_mut(&RemoteButton::Ok).unwrap().long = ButtonAction::Shortcut {
                chord: KeyChord {
                    keys: vec![KeyCode::Escape],
                },
            };
            runtime.set_mappings(mappings.clone());
            let sender = runtime.sender();
            sender
                .send(EngineMessage::GateEdge(ButtonEdge {
                    button: RemoteButton::Ok,
                    is_pressed: true,
                }))
                .unwrap();
            flush(&runtime);
            match cancellation {
                "disable" => {
                    let mut disabled = mappings.clone();
                    disabled.enabled = false;
                    runtime.set_mappings(disabled);
                }
                "disconnect" => runtime.set_input_context(crate::RemoteModel::Rc001, false),
                "listener_restart" => {
                    sender.send(EngineMessage::ListenerStopped).unwrap();
                }
                _ => runtime.set_input_context(crate::RemoteModel::Rc003, true),
            }
            runtime.set_input_context(crate::RemoteModel::Rc001, true);
            runtime.set_mappings(mappings);
            // A repeat from the old physical hold and its eventual UP cannot click.
            sender
                .send(EngineMessage::GateEdge(ButtonEdge {
                    button: RemoteButton::Ok,
                    is_pressed: true,
                }))
                .unwrap();
            sender
                .send(EngineMessage::GateEdge(ButtonEdge {
                    button: RemoteButton::Ok,
                    is_pressed: false,
                }))
                .unwrap();
            flush(&runtime);
            assert_eq!(runtime.snapshot().fired_gestures, 0, "{cancellation}");
            sender
                .send(EngineMessage::GateEdge(ButtonEdge {
                    button: RemoteButton::Ok,
                    is_pressed: true,
                }))
                .unwrap();
            sender
                .send(EngineMessage::GateEdge(ButtonEdge {
                    button: RemoteButton::Ok,
                    is_pressed: false,
                }))
                .unwrap();
            flush(&runtime);
            assert_eq!(
                runtime.snapshot().fired_gestures,
                1,
                "{cancellation}: next press"
            );
            sender
                .send(EngineMessage::GateEdge(ButtonEdge {
                    button: RemoteButton::Ok,
                    is_pressed: true,
                }))
                .unwrap();
            sender.send(EngineMessage::ListenerStopped).unwrap();
            // A verified full all-up report recovers a release missed offline.
            sender
                .send(EngineMessage::HidUsages(BTreeSet::new()))
                .unwrap();
            sender
                .send(EngineMessage::GateEdge(ButtonEdge {
                    button: RemoteButton::Ok,
                    is_pressed: true,
                }))
                .unwrap();
            sender
                .send(EngineMessage::GateEdge(ButtonEdge {
                    button: RemoteButton::Ok,
                    is_pressed: false,
                }))
                .unwrap();
            flush(&runtime);
            assert_eq!(
                runtime.snapshot().fired_gestures,
                2,
                "{cancellation}: all-up resync"
            );
        }
        let mut restarted = GestureInputState::default();
        assert!(!restarted.accept(ButtonEdge {
            button: RemoteButton::Ok,
            is_pressed: false
        }));
        assert!(restarted.accept(ButtonEdge {
            button: RemoteButton::Ok,
            is_pressed: true
        }));
        restarted.cancel(BTreeSet::from([RemoteButton::Ok]));
        assert!(restarted.accept(ButtonEdge {
            button: RemoteButton::Up,
            is_pressed: true
        }));
        assert!(!restarted.accept(ButtonEdge {
            button: RemoteButton::Ok,
            is_pressed: false
        }));
        assert!(restarted.accept(ButtonEdge {
            button: RemoteButton::Up,
            is_pressed: false
        }));
        assert!(restarted.accept(ButtonEdge {
            button: RemoteButton::Up,
            is_pressed: true
        }));
    }

    #[test]
    fn profile_switch_during_hold_cancels_old_gesture_and_waits_for_full_release() {
        let _gate_test = GATE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let injector = Arc::new(RecordingInjector::default());
        let runtime = ButtonMappingRuntime::new(
            Arc::clone(&injector) as Arc<dyn MappingInjector>,
            Arc::new(UsageCounters::default()),
            Arc::new(StdMutex::new(RawInputSnapshot::default())),
            Arc::new(crate::macro_runtime::MacroRuntime::new(Arc::new(
                crate::macro_runtime::UnsupportedStepperSource,
            ))),
        );
        runtime.set_input_context(crate::RemoteModel::Rc001, true);
        let mut common = ButtonMappings::default();
        common.actions.insert(
            RemoteButton::Ok,
            ButtonActions {
                single: ButtonAction::Shortcut {
                    chord: KeyChord {
                        keys: vec![KeyCode::Enter],
                    },
                },
                long: ButtonAction::Shortcut {
                    chord: KeyChord {
                        keys: vec![KeyCode::Enter],
                    },
                },
                ..ButtonActions::default()
            },
        );
        let mut profile = ButtonMappings::default();
        profile.actions.insert(
            RemoteButton::Ok,
            ButtonActions {
                single: ButtonAction::Shortcut {
                    chord: KeyChord {
                        keys: vec![KeyCode::Escape],
                    },
                },
                long: ButtonAction::Shortcut {
                    chord: KeyChord {
                        keys: vec![KeyCode::Escape],
                    },
                },
                ..ButtonActions::default()
            },
        );
        runtime.set_mappings(common.clone());
        let sender = runtime.sender();
        sender
            .send(EngineMessage::GateEdge(ButtonEdge {
                button: RemoteButton::Ok,
                is_pressed: true,
            }))
            .unwrap();
        flush(&runtime);

        runtime.set_profile_mappings(Some(profile.clone()));
        assert_eq!(
            runtime.configuration.lock().unwrap().effective_mappings().0,
            profile
        );
        sender
            .send(EngineMessage::GateEdge(ButtonEdge {
                button: RemoteButton::Ok,
                is_pressed: false,
            }))
            .unwrap();
        flush(&runtime);
        assert_eq!(runtime.snapshot().fired_gestures, 0);
        assert_eq!(
            runtime.mappings(),
            common,
            "profile must not overwrite common settings"
        );

        sender
            .send(EngineMessage::GateEdge(ButtonEdge {
                button: RemoteButton::Ok,
                is_pressed: true,
            }))
            .unwrap();
        sender
            .send(EngineMessage::GateEdge(ButtonEdge {
                button: RemoteButton::Ok,
                is_pressed: false,
            }))
            .unwrap();
        flush(&runtime);
        assert_eq!(runtime.snapshot().fired_gestures, 1);

        runtime.set_profile_mappings(None);
        sender
            .send(EngineMessage::GateEdge(ButtonEdge {
                button: RemoteButton::Ok,
                is_pressed: true,
            }))
            .unwrap();
        sender
            .send(EngineMessage::GateEdge(ButtonEdge {
                button: RemoteButton::Ok,
                is_pressed: false,
            }))
            .unwrap();
        flush(&runtime);
        assert_eq!(runtime.snapshot().fired_gestures, 2);
    }

    /// 测试注入器：记录 tap 的和弦与打开应用的目标。
    #[derive(Debug, Default)]
    struct RecordingInjector {
        taps: StdMutex<Vec<KeyChord>>,
        launches: StdMutex<Vec<String>>,
        scrolls: StdMutex<Vec<(ScrollDirection, u16)>>,
        clicks: StdMutex<Vec<MouseClickKind>>,
        moves: StdMutex<Vec<(MoveDirection, u16)>>,
        focus_requests: StdMutex<Vec<FocusCall>>,
        focus_cancellations: StdMutex<u32>,
        learn_results: StdMutex<Vec<Result<crate::focus::RecordedFocusTarget, String>>>,
        /// 常用语组件槽：测试独立注入，不碰进程级槽位（避免用例间互相污染）。
        phrase_host: PhraseHostSlot,
        app_switcher_commands: StdMutex<Vec<AppSwitcherCommand>>,
        app_switcher_present: std::sync::atomic::AtomicBool,
        app_switcher_alt_down: std::sync::atomic::AtomicBool,
        fail: bool,
    }

    /// 常用语组件槽（`Arc<dyn CommonPhraseHost>` 无 Debug 实现，包一层，
    /// 免得为它手写整个 [`RecordingInjector`] 的 Debug）。
    #[derive(Default)]
    struct PhraseHostSlot(StdMutex<Option<Arc<dyn crate::common_phrases::CommonPhraseHost>>>);

    impl std::fmt::Debug for PhraseHostSlot {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("PhraseHostSlot")
        }
    }

    impl RecordingInjector {
        /// 绑定本用例专用的常用语组件。
        fn set_phrase_host(&self, host: Arc<dyn crate::common_phrases::CommonPhraseHost>) {
            *self.phrase_host.0.lock().unwrap() = Some(host);
        }
    }

    /// 测试注入器收到的聚焦类调用（区分「只聚焦」与「打开+聚焦」）。
    #[derive(Debug, Clone, PartialEq, Eq)]
    enum FocusCall {
        Frontmost,
        LaunchThenFocus {
            target: String,
            strategy: &'static str,
        },
    }

    impl MappingInjector for RecordingInjector {
        fn cancel_focus(&self) {
            *self.focus_cancellations.lock().unwrap() += 1;
        }

        fn scroll(&self, direction: ScrollDirection, steps: u16) -> Result<(), String> {
            if self.fail {
                return Err("wheel injection failed (test)".to_owned());
            }
            self.scrolls.lock().unwrap().push((direction, steps));
            Ok(())
        }

        fn mouse_click(&self, kind: MouseClickKind) -> Result<(), String> {
            if self.fail {
                return Err("mouse click failed (test)".to_owned());
            }
            self.clicks.lock().unwrap().push(kind);
            Ok(())
        }

        fn mouse_move(&self, direction: MoveDirection, distance: u16) -> Result<(), String> {
            if self.fail {
                return Err("mouse move failed (test)".to_owned());
            }
            self.moves.lock().unwrap().push((direction, distance));
            Ok(())
        }

        fn tap(&self, chord: &KeyChord) -> Result<(), String> {
            if self.fail {
                return Err("注入失败（测试）".to_owned());
            }
            self.taps.lock().unwrap().push(chord.clone());
            Ok(())
        }

        fn launch_app(&self, target: &str) -> Result<(), String> {
            if self.fail {
                return Err("打开应用失败（测试）".to_owned());
            }
            self.launches.lock().unwrap().push(target.to_owned());
            Ok(())
        }

        fn focus_frontmost(&self) -> Result<(), String> {
            if self.fail {
                return Err("聚焦服务不可用（测试）".to_owned());
            }
            self.focus_requests
                .lock()
                .unwrap()
                .push(FocusCall::Frontmost);
            Ok(())
        }

        fn launch_app_and_focus(
            &self,
            target: &str,
            profile: Option<&crate::focus::AppFocusProfile>,
            _cancellation: Option<&AtomicBool>,
        ) -> Result<(), String> {
            if self.fail {
                return Err("打开应用失败（测试）".to_owned());
            }
            let strategy = profile
                .map(|profile| profile.strategy.as_str())
                .unwrap_or("none");
            self.launches.lock().unwrap().push(target.to_owned());
            self.focus_requests
                .lock()
                .unwrap()
                .push(FocusCall::LaunchThenFocus {
                    target: target.to_owned(),
                    strategy,
                });
            Ok(())
        }
        fn learn_focus_target(&self) -> Result<crate::focus::RecordedFocusTarget, String> {
            if self.fail {
                return Err("学习输入框失败（测试）".to_owned());
            }
            self.learn_results.lock().unwrap().pop().unwrap_or_else(|| {
                Ok(crate::focus::RecordedFocusTarget {
                    control_type: "Edit".to_owned(),
                    automation_id: "chat-input".to_owned(),
                    ..Default::default()
                })
            })
        }

        fn common_phrase_host(&self) -> Arc<dyn crate::common_phrases::CommonPhraseHost> {
            self.phrase_host
                .0
                .lock()
                .unwrap()
                .clone()
                .unwrap_or_else(|| Arc::new(crate::common_phrases::DisabledCommonPhraseHost))
        }

        fn app_switcher_submit(&self, command: AppSwitcherCommand) -> Result<(), String> {
            if self.fail {
                return Err("app switcher injection failed (test)".to_owned());
            }
            self.app_switcher_commands.lock().unwrap().push(command);
            match command {
                AppSwitcherCommand::AltDown => self
                    .app_switcher_alt_down
                    .store(true, std::sync::atomic::Ordering::SeqCst),
                AppSwitcherCommand::AltUp => self
                    .app_switcher_alt_down
                    .store(false, std::sync::atomic::Ordering::SeqCst),
                _ => {}
            }
            Ok(())
        }

        fn app_switcher_present(&self) -> bool {
            self.app_switcher_present
                .load(std::sync::atomic::Ordering::SeqCst)
        }

        fn app_switcher_alt_down(&self) -> bool {
            self.app_switcher_alt_down
                .load(std::sync::atomic::Ordering::SeqCst)
        }
    }

    fn mappings_with_single(button: RemoteButton, key: KeyCode) -> ButtonMappings {
        let mut mappings = ButtonMappings::default();
        mappings.actions.insert(
            button,
            ButtonActions {
                single: ButtonAction::Shortcut {
                    chord: KeyChord { keys: vec![key] },
                },
                ..ButtonActions::default()
            },
        );
        mappings
    }

    fn hid_usages_of(button: RemoteButton) -> BTreeSet<u16> {
        let usage = match button {
            RemoteButton::Ok => 0x0028,
            RemoteButton::Tv => 0x0035,
            RemoteButton::Left => 0x0050,
            RemoteButton::Right => 0x004F,
            RemoteButton::Up => 0x0052,
            RemoteButton::Down => 0x0051,
            RemoteButton::Back => 0x00F1,
            _ => 0x0028,
        };
        BTreeSet::from([usage])
    }

    /// 泄漏路径的遥控器键盘事件（监听器按设备路径过滤后投递给引擎的形态）。
    fn keyboard_event(virtual_key: u16, message: u32) -> RawKeyboardEvent {
        RawKeyboardEvent {
            make_code: 0,
            flags: 0,
            virtual_key,
            message,
        }
    }

    const KEYDOWN: u32 = 0x0100;
    const KEYUP: u32 = 0x0101;

    /// 轮询等待谓词成立（真时钟测试的统一等待原语），预算内不成立返回 false。
    ///
    /// 为什么不用固定 sleep：引擎是单线程循环，消息处理与连发/双击定时器共用
    /// 一条队列，CI 慢机或全量并行下排队延迟可达本地的数倍——固定 sleep 是
    /// 「本地刚好够、CI 必然压线」的 flaky 来源（2026-09-28 `leak_suppression_suite`
    /// 实证：700ms 窗口断言 4 拍连发，负载下第 4 拍在窗口后才到）。轮询把
    /// 「断言时机」换成「条件成立」，判据不变、余量放大；超时后由调用处的
    /// assert 以实际状态给出可诊断的失败。
    fn wait_until(mut pred: impl FnMut() -> bool, budget: Duration) -> bool {
        let deadline = Instant::now() + budget;
        while Instant::now() < deadline {
            if pred() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        pred()
    }

    /// 泄漏对冲套件（2026-09-06 调查档案修复记录）：泄漏路径
    /// （[`EngineMessage::Keyboard`]，监听器按设备路径过滤=遥控器专用）的
    /// 按压边沿把该键标记为"原生已交付"——同键映射（上→上）的 Single
    /// 跳过注入（原生动作已进 OS），连发/不同键映射/门控路径照常注入。
    ///
    /// 测试锁覆盖完整 runtime/gate 生命周期，避免独立夹具互相改变进程全局状态。
    #[test]
    fn leak_suppression_suite() {
        let _gate_test = GATE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let gate = crate::key_gate::KeyGate::start();

        let injector = Arc::new(RecordingInjector::default());
        let runtime = ButtonMappingRuntime::new(
            Arc::clone(&injector) as Arc<dyn MappingInjector>,
            Arc::new(UsageCounters::default()),
            Arc::new(StdMutex::new(RawInputSnapshot::default())),
            Arc::new(crate::macro_runtime::MacroRuntime::new(Arc::new(
                crate::macro_runtime::UnsupportedStepperSource,
            ))),
        );
        runtime.set_input_context(crate::RemoteModel::Rc001, true);
        let single = |key: KeyCode| ButtonAction::Shortcut {
            chord: KeyChord { keys: vec![key] },
        };
        let mut mappings = ButtonMappings::default();
        // 上→上：同键映射（泄漏对冲目标）。
        mappings.actions.insert(
            RemoteButton::Up,
            ButtonActions {
                single: single(KeyCode::Up),
                // 场景 5 同时验证新缺省：原生首拍只对冲一次，后续自动连发仍注入。
                hold_repeat: None,
                ..ButtonActions::default()
            },
        );
        // 右→右：同键映射，作为门控路径的对照组。
        mappings.actions.insert(
            RemoteButton::Right,
            ButtonActions {
                single: single(KeyCode::Right),
                ..ButtonActions::default()
            },
        );
        // 左→退格：不同键映射（泄漏路径仍须注入配置动作）。
        mappings.actions.insert(
            RemoteButton::Left,
            ButtonActions {
                single: single(KeyCode::Backspace),
                ..ButtonActions::default()
            },
        );
        // 确定→Enter 单击 + 空格 双击：双击窗口补发单击的对冲场景。
        mappings.actions.insert(
            RemoteButton::Ok,
            ButtonActions {
                single: single(KeyCode::Enter),
                double: single(KeyCode::Space),
                long: ButtonAction::Disabled,
                hold_repeat: None,
                ok_context_click: false,
                extra: BTreeMap::new(),
                ..ButtonActions::default()
            },
        );
        // 电源→Win+L：锁屏会让真实 UP 延迟到解锁后，引擎须在成功请求锁屏后
        // 立即清理按住态，保证下一次 DOWN 不依赖旧 UP。
        mappings.actions.insert(
            RemoteButton::Power,
            ButtonActions {
                single: ButtonAction::Shortcut {
                    chord: KeyChord {
                        keys: vec![KeyCode::LeftWindows, KeyCode::L],
                    },
                },
                ..ButtonActions::default()
            },
        );
        runtime.set_mappings(mappings);

        let sender = runtime.sender();
        let taps = || injector.taps.lock().unwrap().clone();

        // 场景 1：泄漏路径的同键映射（上→上）首击不注入（原生已交付）。
        assert!(crate::key_gate::is_gate_thread_alive());
        sender
            .send(EngineMessage::Keyboard(keyboard_event(0x26, KEYDOWN)))
            .unwrap();
        std::thread::sleep(Duration::from_millis(120));
        assert!(
            taps().is_empty(),
            "泄漏路径同键映射的首击应由原生覆盖，不注入"
        );
        // 连发起始（350ms）前释放，避免连发干扰后续断言。
        sender
            .send(EngineMessage::Keyboard(keyboard_event(0x26, KEYUP)))
            .unwrap();
        std::thread::sleep(Duration::from_millis(50));

        // 场景 2（对照）：门控路径的同键映射（右→右）照常注入。
        assert!(crate::key_gate::is_gate_thread_alive());
        let base = taps().len();
        sender
            .send(EngineMessage::GateEdge(ButtonEdge {
                button: RemoteButton::Right,
                is_pressed: true,
            }))
            .unwrap();
        wait_until(|| taps().len() > base, Duration::from_millis(600));
        assert_eq!(
            taps()[base..].to_vec(),
            vec![KeyChord {
                keys: vec![KeyCode::Right]
            }],
            "门控路径（已吞键）的同键映射必须注入"
        );
        sender
            .send(EngineMessage::GateEdge(ButtonEdge {
                button: RemoteButton::Right,
                is_pressed: false,
            }))
            .unwrap();
        std::thread::sleep(Duration::from_millis(50));

        // 场景 3：泄漏路径的不同键映射（左→退格）照常注入。冷首按会
        // 同时包含原生左移，这是与上/下/右/确定相同的结构性边界。
        assert!(crate::key_gate::is_gate_thread_alive());
        let base = taps().len();
        sender
            .send(EngineMessage::Keyboard(keyboard_event(0x25, KEYDOWN)))
            .unwrap();
        wait_until(|| taps().len() > base, Duration::from_millis(600));
        assert_eq!(
            taps()[base..].to_vec(),
            vec![KeyChord {
                keys: vec![KeyCode::Backspace]
            }],
            "泄漏路径的左键不同键映射必须注入"
        );
        sender
            .send(EngineMessage::Keyboard(keyboard_event(0x25, KEYUP)))
            .unwrap();
        std::thread::sleep(Duration::from_millis(50));

        // 场景 4：泄漏路径的双击窗口补发单击（确定→Enter）由原生覆盖，不注入。
        assert!(crate::key_gate::is_gate_thread_alive());
        let base = taps().len();
        sender
            .send(EngineMessage::Keyboard(keyboard_event(0x0D, KEYDOWN)))
            .unwrap();
        std::thread::sleep(Duration::from_millis(80));
        sender
            .send(EngineMessage::Keyboard(keyboard_event(0x0D, KEYUP)))
            .unwrap();
        std::thread::sleep(Duration::from_millis(650));
        let after_window = &taps()[base..];
        assert!(
            after_window.is_empty(),
            "双击窗口超时补发的同键单击应由原生覆盖：{after_window:?}"
        );

        // 场景 5：泄漏按住的连发照常注入（遥控器不自动重复，连发由引擎交付）。
        assert!(crate::key_gate::is_gate_thread_alive());
        let base = taps().len();
        sender
            .send(EngineMessage::Keyboard(keyboard_event(0x26, KEYDOWN)))
            .unwrap();
        let got_four = wait_until(|| taps().len() >= base + 4, Duration::from_millis(2000));
        let count = taps().len() - base;
        assert!(
            got_four,
            "泄漏按住的连发应注入（350/450/550/650ms），实际 {count} 次"
        );
        sender
            .send(EngineMessage::Keyboard(keyboard_event(0x26, KEYUP)))
            .unwrap();
        std::thread::sleep(Duration::from_millis(50));

        // 场景 6：Win+L 会切换交互桌面，必须在实体 UP 到达后才调用锁屏，
        // 确保门控先成对消费 DOWN/UP；下一次完整按压仍可再次触发。
        assert!(crate::key_gate::is_gate_thread_alive());
        let before_lock = taps().len();
        for _ in 0..2 {
            let before_press = taps().len();
            sender
                .send(EngineMessage::GateEdge(ButtonEdge {
                    button: RemoteButton::Power,
                    is_pressed: true,
                }))
                .unwrap();
            // 「按住期间不锁屏」是否定性断言：观察窗从 50ms 拉长到 150ms，
            // 给引擎更充分的时间证明"没有发生"（越久越强，且仍远小于测试预算）。
            std::thread::sleep(Duration::from_millis(150));
            assert_eq!(
                taps().len(),
                before_press,
                "Win+L 不得在实体按键仍按住时切换桌面"
            );
            sender
                .send(EngineMessage::GateEdge(ButtonEdge {
                    button: RemoteButton::Power,
                    is_pressed: false,
                }))
                .unwrap();
            wait_until(|| taps().len() > before_press, Duration::from_millis(600));
            assert_eq!(
                taps().len(),
                before_press + 1,
                "Win+L 应在本轮实体按键释放后执行一次"
            );
        }
        let after_lock = taps();
        assert_eq!(
            after_lock.len(),
            before_lock + 2,
            "Win+L 终端动作成功后须立即释放引擎状态：{after_lock:?}"
        );
        assert!(after_lock[before_lock].is_lock_workstation());
        assert!(after_lock[before_lock + 1].is_lock_workstation());

        drop(runtime);
        drop(gate);
    }

    /// Issue #195：`map_inject` 的成功判据分两层——入队（queued）与回声
    /// （宿主钩子是否看到这次注入）。日志模板必须同时覆盖"看到"与"没看到"
    /// 两种形态，且不得再出现「result=ok」这种只证明入队的措辞。
    #[test]
    fn shortcut_injection_note_reports_queue_and_hook_echo() {
        let observed = shortcut_injection_note(crate::key_gate::InjectedEcho {
            count: 2,
            vk: 0x1B,
            scan: 0x01,
            flags: 0x90,
        });
        assert_eq!(
            observed,
            "map_inject result=queued echo_delta=2 last_vk=0x1B last_scan=0x01 last_flags=0x90"
        );
        assert!(
            !observed.contains("result=ok"),
            "不得再把入队当成功：{observed}"
        );

        let missing = shortcut_injection_note(crate::key_gate::InjectedEcho::default());
        assert_eq!(
            missing,
            "map_inject result=queued echo_delta=0 note=hook_did_not_observe_injected_key"
        );

        // 锁屏动作没有 SendInput 回声，措辞必须自证"设计如此"。
        assert_eq!(
            LOCK_WORKSTATION_INJECT_NOTE,
            "map_inject result=queued note=lock_workstation_via_api_no_sendinput_echo"
        );
    }

    #[test]
    fn hid_press_release_drives_single_action_tap() {
        let _gate_test = GATE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let injector = Arc::new(RecordingInjector::default());
        let snapshot = Arc::new(StdMutex::new(RawInputSnapshot::default()));
        let runtime = ButtonMappingRuntime::new(
            Arc::clone(&injector) as Arc<dyn MappingInjector>,
            Arc::new(UsageCounters::default()),
            Arc::clone(&snapshot),
            Arc::new(crate::macro_runtime::MacroRuntime::new(Arc::new(
                crate::macro_runtime::UnsupportedStepperSource,
            ))),
        );
        runtime.set_input_context(crate::RemoteModel::Rc001, true);
        // 注意：单元测试环境没有真实 key_gate 线程（is_gate_thread_alive=false），
        // 引擎按设计保持观察模式（不注入）。此处先验证边沿→手势→回调链路。
        runtime.set_mappings(mappings_with_single(RemoteButton::Ok, KeyCode::Enter));

        let fired = Arc::new(StdMutex::new(Vec::new()));
        let fired_sink = Arc::clone(&fired);
        runtime.subscribe_button_gestures(Arc::new(move |gesture| {
            fired_sink.lock().unwrap().push(gesture);
        }));

        let sender = runtime.sender();
        sender
            .send(EngineMessage::HidUsages(hid_usages_of(RemoteButton::Ok)))
            .unwrap();
        sender
            .send(EngineMessage::HidUsages(BTreeSet::new()))
            .unwrap();
        assert!(wait_until(
            || !fired.lock().unwrap().is_empty(),
            Duration::from_secs(2)
        ));
        let fired = fired.lock().unwrap();
        assert_eq!(
            fired.as_slice(),
            &[FiredGesture {
                button: RemoteButton::Ok,
                trigger: ButtonTrigger::Single
            }],
            "OK 只配置单击：HID 按下/释放应触发一次单击手势"
        );
        assert!(
            injector.taps.lock().unwrap().is_empty(),
            "门控未运行（测试环境）时不得注入"
        );
        drop(fired);

        let snapshot = snapshot.lock().unwrap();
        assert_eq!(snapshot.active_buttons, Vec::new());
        assert_eq!(snapshot.semantic_edge_count, 2);
        assert_eq!(snapshot.last_button, Some(RemoteButton::Ok));
    }

    /// 向导第⑥步：暂挂期间手势仍被观察（回调照发），但不执行注入；解除后恢复。
    #[test]
    fn suspended_engine_observes_gestures_without_injecting() {
        let _gate_lock = GATE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let gate = crate::key_gate::KeyGate::start();
        let injector = Arc::new(RecordingInjector::default());
        let runtime = ButtonMappingRuntime::new(
            Arc::clone(&injector) as Arc<dyn MappingInjector>,
            Arc::new(UsageCounters::default()),
            Arc::new(StdMutex::new(RawInputSnapshot::default())),
            Arc::new(crate::macro_runtime::MacroRuntime::new(Arc::new(
                crate::macro_runtime::UnsupportedStepperSource,
            ))),
        );
        runtime.set_input_context(crate::RemoteModel::Rc001, true);
        runtime.set_mappings(mappings_with_single(RemoteButton::Ok, KeyCode::Enter));
        let fired = Arc::new(StdMutex::new(Vec::new()));
        let fired_sink = Arc::clone(&fired);
        runtime.subscribe_button_gestures(Arc::new(move |gesture| {
            fired_sink.lock().unwrap().push(gesture);
        }));

        let sender = runtime.sender();
        let tap_count = || injector.taps.lock().unwrap().len();

        runtime.set_suspended(true);
        sender
            .send(EngineMessage::HidUsages(hid_usages_of(RemoteButton::Ok)))
            .unwrap();
        sender
            .send(EngineMessage::HidUsages(BTreeSet::new()))
            .unwrap();
        std::thread::sleep(Duration::from_millis(150));
        assert_eq!(
            fired.lock().unwrap().len(),
            1,
            "暂挂期间手势仍应被观察（回调照发）"
        );
        assert_eq!(tap_count(), 0, "暂挂期间不得注入任何按键");

        runtime.set_suspended(false);
        sender
            .send(EngineMessage::HidUsages(hid_usages_of(RemoteButton::Ok)))
            .unwrap();
        sender
            .send(EngineMessage::HidUsages(BTreeSet::new()))
            .unwrap();
        std::thread::sleep(Duration::from_millis(200));
        assert_eq!(fired.lock().unwrap().len(), 2, "解除暂挂后手势继续被观察");
        assert_eq!(tap_count(), 1, "解除暂挂后应恢复注入");

        drop(runtime);
        drop(gate);
    }

    /// 应用切换器集成：启动键打开会话；方向键在会话内被认领并改为 Tab；
    /// OK 确认后释放 Alt、清门控遮蔽，且不触发原映射。
    #[test]
    fn app_switcher_session_owns_controls_and_releases_alt_on_confirm() {
        use std::sync::atomic::Ordering;
        let _gate_lock = crate::key_gate::lock_gate_tests();
        let gate = crate::key_gate::KeyGate::start();
        let injector = Arc::new(RecordingInjector::default());
        injector.app_switcher_present.store(true, Ordering::SeqCst);
        let runtime = ButtonMappingRuntime::new(
            Arc::clone(&injector) as Arc<dyn MappingInjector>,
            Arc::new(UsageCounters::default()),
            Arc::new(StdMutex::new(RawInputSnapshot::default())),
            Arc::new(crate::macro_runtime::MacroRuntime::new(Arc::new(
                crate::macro_runtime::UnsupportedStepperSource,
            ))),
        );
        runtime.set_input_context(crate::RemoteModel::Rc001, true);
        let mut mappings = ButtonMappings::default();
        mappings.actions.insert(
            RemoteButton::Tv,
            ButtonActions {
                single: ButtonAction::AppSwitcher,
                ..ButtonActions::default()
            },
        );
        // 左键原本是退格：会话期间必须改为上一项（Shift+Tab），不得漏出原动作。
        mappings.actions.insert(
            RemoteButton::Left,
            ButtonActions {
                single: ButtonAction::Shortcut {
                    chord: KeyChord {
                        keys: vec![KeyCode::Backspace],
                    },
                },
                ..ButtonActions::default()
            },
        );
        runtime.set_mappings(mappings);
        let sender = runtime.sender();
        let commands = || injector.app_switcher_commands.lock().unwrap().clone();
        let press = |button| {
            sender
                .send(EngineMessage::HidUsages(hid_usages_of(button)))
                .unwrap()
        };
        let release = || {
            sender
                .send(EngineMessage::HidUsages(BTreeSet::new()))
                .unwrap()
        };

        press(RemoteButton::Tv);
        release();
        assert!(
            wait_until(
                || commands() == vec![AppSwitcherCommand::AltDown, AppSwitcherCommand::Tab],
                Duration::from_millis(500)
            ),
            "启动应只按一次 Alt 并发送第一次 Tab：{:?}",
            commands()
        );
        // 命令入队与门控遮蔽分属两个线程，满载时可能先后到达；等待遮蔽
        // 落位，而不是在命令列表刚变化时立刻读原子量（CI/本机满载 flake）。
        assert!(
            wait_until(
                || crate::key_gate::app_switcher_mask() != 0,
                Duration::from_millis(500)
            ),
            "会话期间控制键必须由门控直接遮蔽"
        );

        press(RemoteButton::Left);
        release();
        assert!(
            wait_until(
                || commands().last() == Some(&AppSwitcherCommand::ReverseTab),
                Duration::from_millis(500)
            ),
            "左键应改为 Shift+Tab：{:?}",
            commands()
        );
        assert!(
            injector.taps.lock().unwrap().is_empty(),
            "被会话认领的左键不得执行原退格映射"
        );

        press(RemoteButton::Ok);
        release();
        assert!(
            wait_until(
                || commands().last() == Some(&AppSwitcherCommand::AltUp),
                Duration::from_millis(500)
            ),
            "OK 应释放 Alt：{:?}",
            commands()
        );
        assert_eq!(
            crate::key_gate::app_switcher_mask(),
            0,
            "确认收尾后遮蔽必须清零（fail-open）"
        );
        assert!(injector.taps.lock().unwrap().is_empty());
        assert!(runtime.snapshot().last_error.is_none());

        // 重新启动会话后按语音键：引擎必须先取消切换器并释放 Alt，再让 BLE
        // 线程注入按住说话快捷键。
        injector.app_switcher_commands.lock().unwrap().clear();
        press(RemoteButton::Tv);
        release();
        assert!(
            wait_until(
                || commands() == vec![AppSwitcherCommand::AltDown, AppSwitcherCommand::Tab],
                Duration::from_millis(500)
            ),
            "第二次启动应重新按 Alt 并发送 Tab：{:?}",
            commands()
        );
        release_switcher_before_voice(&sender, Duration::from_millis(60)).unwrap();
        assert!(
            wait_until(
                || {
                    let current = commands();
                    current.last() == Some(&AppSwitcherCommand::AltUp)
                        && current.contains(&AppSwitcherCommand::Escape)
                },
                Duration::from_millis(500)
            ),
            "语音开始应 Esc 取消并释放 Alt：{:?}",
            commands()
        );
        assert!(
            wait_until(
                || crate::key_gate::app_switcher_mask() == 0,
                Duration::from_millis(500)
            ),
            "语音开始取消后遮蔽必须清零：{}",
            crate::key_gate::app_switcher_mask()
        );

        drop(runtime);
        drop(gate);
    }

    /// 组合动作里的 Alt+Tab：宏步骤只负责请求引擎启动会话；会话仍由
    /// 方向/OK/返回控制，确认后释放 Alt 并清门控遮蔽。
    #[test]
    fn app_switcher_macro_start_opens_the_session() {
        use std::sync::atomic::Ordering;
        let _gate_lock = crate::key_gate::lock_gate_tests();
        let gate = crate::key_gate::KeyGate::start();
        let injector = Arc::new(RecordingInjector::default());
        injector.app_switcher_present.store(true, Ordering::SeqCst);
        let runtime = ButtonMappingRuntime::new(
            Arc::clone(&injector) as Arc<dyn MappingInjector>,
            Arc::new(UsageCounters::default()),
            Arc::new(StdMutex::new(RawInputSnapshot::default())),
            Arc::new(crate::macro_runtime::MacroRuntime::new(Arc::new(
                crate::macro_runtime::UnsupportedStepperSource,
            ))),
        );
        runtime.set_input_context(crate::RemoteModel::Rc001, true);
        let sender = runtime.sender();
        let commands = || injector.app_switcher_commands.lock().unwrap().clone();

        sender.send(EngineMessage::StartAppSwitcher).unwrap();
        assert!(
            wait_until(
                || commands() == vec![AppSwitcherCommand::AltDown, AppSwitcherCommand::Tab],
                Duration::from_millis(500)
            ),
            "宏启动应让引擎按 Alt 并发送第一次 Tab：{:?}",
            commands()
        );
        assert_ne!(
            crate::key_gate::app_switcher_mask(),
            0,
            "宏启动的会话也必须遮蔽控制键"
        );

        sender
            .send(EngineMessage::HidUsages(hid_usages_of(RemoteButton::Ok)))
            .unwrap();
        sender
            .send(EngineMessage::HidUsages(BTreeSet::new()))
            .unwrap();
        assert!(
            wait_until(
                || commands().last() == Some(&AppSwitcherCommand::AltUp),
                Duration::from_millis(500)
            ),
            "OK 应确认并释放 Alt：{:?}",
            commands()
        );
        assert_eq!(
            crate::key_gate::app_switcher_mask(),
            0,
            "确认收尾后遮蔽必须清零"
        );

        drop(runtime);
        drop(gate);
    }

    /// 键位方案覆盖层里的 Alt+Tab：即使私有页仍写成一次性快捷键，覆盖层
    /// 加载时也应升级为 app_switcher 会话；基础映射不受影响。
    #[test]
    fn profile_overlay_alt_tab_shortcut_opens_the_app_switcher_session() {
        use std::sync::atomic::Ordering;
        let _gate_lock = crate::key_gate::lock_gate_tests();
        let gate = crate::key_gate::KeyGate::start();
        let injector = Arc::new(RecordingInjector::default());
        injector.app_switcher_present.store(true, Ordering::SeqCst);
        let runtime = ButtonMappingRuntime::new(
            Arc::clone(&injector) as Arc<dyn MappingInjector>,
            Arc::new(UsageCounters::default()),
            Arc::new(StdMutex::new(RawInputSnapshot::default())),
            Arc::new(crate::macro_runtime::MacroRuntime::new(Arc::new(
                crate::macro_runtime::UnsupportedStepperSource,
            ))),
        );
        runtime.set_input_context(crate::RemoteModel::Rc001, true);
        let mut overlay = ButtonMappings::default();
        overlay.actions.insert(
            RemoteButton::Tv,
            ButtonActions {
                single: ButtonAction::Shortcut {
                    chord: KeyChord {
                        keys: vec![KeyCode::Alt, KeyCode::Tab],
                    },
                },
                ..ButtonActions::default()
            },
        );
        runtime.set_profile_overlay(Some(overlay));
        let sender = runtime.sender();
        let commands = || injector.app_switcher_commands.lock().unwrap().clone();

        sender
            .send(EngineMessage::HidUsages(hid_usages_of(RemoteButton::Tv)))
            .unwrap();
        sender
            .send(EngineMessage::HidUsages(BTreeSet::new()))
            .unwrap();
        assert!(
            wait_until(
                || commands() == vec![AppSwitcherCommand::AltDown, AppSwitcherCommand::Tab],
                Duration::from_millis(500)
            ),
            "键位方案的 Alt+Tab 应启动会话：{:?}",
            commands()
        );
        assert_ne!(
            crate::key_gate::app_switcher_mask(),
            0,
            "会话期间控制键必须遮蔽"
        );

        sender
            .send(EngineMessage::HidUsages(hid_usages_of(RemoteButton::Ok)))
            .unwrap();
        sender
            .send(EngineMessage::HidUsages(BTreeSet::new()))
            .unwrap();
        assert!(
            wait_until(
                || commands().last() == Some(&AppSwitcherCommand::AltUp),
                Duration::from_millis(500)
            ),
            "OK 应确认并释放 Alt：{:?}",
            commands()
        );
        assert_eq!(crate::key_gate::app_switcher_mask(), 0);

        drop(runtime);
        drop(gate);
    }

    /// 打开应用动作：门控运行时，手势触发应调用 launch_app 而非 tap。
    #[test]
    fn open_app_action_launches_instead_of_tap() {
        let _gate_test = GATE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let gate = crate::key_gate::KeyGate::start();
        let injector = Arc::new(RecordingInjector::default());
        let snapshot = Arc::new(StdMutex::new(RawInputSnapshot::default()));
        let runtime = ButtonMappingRuntime::new(
            Arc::clone(&injector) as Arc<dyn MappingInjector>,
            Arc::new(UsageCounters::default()),
            snapshot,
            Arc::new(crate::macro_runtime::MacroRuntime::new(Arc::new(
                crate::macro_runtime::UnsupportedStepperSource,
            ))),
        );
        runtime.set_input_context(crate::RemoteModel::Rc001, true);
        let mut mappings = ButtonMappings::default();
        mappings.actions.insert(
            RemoteButton::Ok,
            ButtonActions {
                single: ButtonAction::OpenApp {
                    target: "notepad".to_owned(),
                },
                ..ButtonActions::default()
            },
        );
        runtime.set_mappings(mappings);

        let sender = runtime.sender();
        sender
            .send(EngineMessage::HidUsages(hid_usages_of(RemoteButton::Ok)))
            .unwrap();
        sender
            .send(EngineMessage::HidUsages(BTreeSet::new()))
            .unwrap();
        assert!(wait_until(
            || !injector.launches.lock().unwrap().is_empty(),
            Duration::from_secs(2)
        ));

        assert_eq!(
            injector.launches.lock().unwrap().as_slice(),
            &["notepad".to_owned()],
            "打开应用动作应调用 launch_app"
        );
        assert!(
            injector.taps.lock().unwrap().is_empty(),
            "打开应用动作不得注入按键"
        );
        drop(gate);
    }

    #[test]
    fn focus_input_action_requests_frontmost_focus_without_tapping() {
        let _gate_lock = GATE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let gate = crate::key_gate::KeyGate::start();
        let injector = Arc::new(RecordingInjector::default());
        let snapshot = Arc::new(StdMutex::new(RawInputSnapshot::default()));
        let runtime = ButtonMappingRuntime::new(
            Arc::clone(&injector) as Arc<dyn MappingInjector>,
            Arc::new(UsageCounters::default()),
            snapshot,
            Arc::new(crate::macro_runtime::MacroRuntime::new(Arc::new(
                crate::macro_runtime::UnsupportedStepperSource,
            ))),
        );
        runtime.set_input_context(crate::RemoteModel::Rc001, true);
        let mut mappings = ButtonMappings::default();
        mappings.actions.insert(
            RemoteButton::Ok,
            ButtonActions {
                single: ButtonAction::FocusInput,
                ..ButtonActions::default()
            },
        );
        runtime.set_mappings(mappings);

        let sender = runtime.sender();
        sender
            .send(EngineMessage::HidUsages(hid_usages_of(RemoteButton::Ok)))
            .unwrap();
        sender
            .send(EngineMessage::HidUsages(BTreeSet::new()))
            .unwrap();
        std::thread::sleep(Duration::from_millis(150));

        assert_eq!(
            injector.focus_requests.lock().unwrap().as_slice(),
            &[FocusCall::Frontmost],
            "聚焦输入框动作应请求聚焦当前前台应用"
        );
        assert!(injector.taps.lock().unwrap().is_empty());
        assert!(injector.launches.lock().unwrap().is_empty());
        drop(gate);
    }

    /// 自定义应用带聚焦档案：分发侧必须把档案一起交给注入器（策略在档案里）。
    #[test]
    fn open_app_with_focus_profile_forwards_strategy() {
        let _gate_lock = GATE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let gate = crate::key_gate::KeyGate::start();
        let injector = Arc::new(RecordingInjector::default());
        let snapshot = Arc::new(StdMutex::new(RawInputSnapshot::default()));
        let runtime = ButtonMappingRuntime::new(
            Arc::clone(&injector) as Arc<dyn MappingInjector>,
            Arc::new(UsageCounters::default()),
            snapshot,
            Arc::new(crate::macro_runtime::MacroRuntime::new(Arc::new(
                crate::macro_runtime::UnsupportedStepperSource,
            ))),
        );
        runtime.set_input_context(crate::RemoteModel::Rc001, true);
        let target = r"C:\Tools\notepad.exe";
        let mut mappings = ButtonMappings::default();
        mappings.actions.insert(
            RemoteButton::Ok,
            ButtonActions {
                single: ButtonAction::OpenApp {
                    target: target.to_owned(),
                },
                ..ButtonActions::default()
            },
        );
        mappings.focus_profiles.insert(
            target.to_owned(),
            crate::focus::AppFocusProfile {
                strategy: crate::focus::FocusStrategy::RecordedElement,
                shortcut: None,
                recorded: Some(crate::focus::RecordedFocusTarget {
                    control_type: "Edit".to_owned(),
                    automation_id: "chat-input".to_owned(),
                    ..Default::default()
                }),
                point: None,
            },
        );
        runtime.set_mappings(mappings);

        let sender = runtime.sender();
        sender
            .send(EngineMessage::HidUsages(hid_usages_of(RemoteButton::Ok)))
            .unwrap();
        sender
            .send(EngineMessage::HidUsages(BTreeSet::new()))
            .unwrap();
        std::thread::sleep(Duration::from_millis(150));

        assert_eq!(
            injector.focus_requests.lock().unwrap().as_slice(),
            &[FocusCall::LaunchThenFocus {
                target: target.to_owned(),
                strategy: "recorded_element",
            }],
            "自定义应用的聚焦档案必须原样交给注入器"
        );
        assert_eq!(
            injector.launches.lock().unwrap().as_slice(),
            &[target.to_owned()]
        );
        drop(gate);
    }

    /// Chrome 聚焦地址栏、网易云音乐聚焦搜索框：使用公开快捷键，
    /// 不依赖无障碍树（2026-10-23 现场实测 Ctrl+L / Ctrl+F 均生效）。
    #[test]
    fn preset_focus_profiles_use_known_shortcuts() {
        let chrome = preset_focus_profile("chrome");
        assert_eq!(chrome.strategy, crate::focus::FocusStrategy::AppShortcut);
        assert_eq!(
            chrome.shortcut.expect("Chrome 快捷键").keys,
            vec![KeyCode::LeftControl, KeyCode::L]
        );

        let netease = preset_focus_profile("netease_music");
        assert_eq!(netease.strategy, crate::focus::FocusStrategy::AppShortcut);
        assert_eq!(
            netease.shortcut.expect("网易云音乐快捷键").keys,
            vec![KeyCode::LeftControl, KeyCode::F]
        );

        // 其余预设继续走通用输入框选择。
        assert_eq!(
            preset_focus_profile("notepad").strategy,
            crate::focus::FocusStrategy::AutoComposer
        );
    }

    #[test]
    fn focus_profile_for_target_prefers_presets_and_keeps_custom_profiles() {
        let mut profiles = std::collections::BTreeMap::new();
        profiles.insert(
            "chrome".to_owned(),
            crate::focus::AppFocusProfile {
                strategy: crate::focus::FocusStrategy::OpenOnly,
                shortcut: None,
                recorded: None,
                point: None,
            },
        );
        // 旧显式档案不能覆盖预设能力，否则组合动作会与 UI 摘要不一致。
        assert_eq!(
            focus_profile_for_target(&profiles, "chrome")
                .expect("Chrome 预设档案")
                .strategy,
            crate::focus::FocusStrategy::AppShortcut
        );

        let custom = r"C:\Tools\Editor.exe";
        profiles.insert(
            custom.to_owned(),
            crate::focus::AppFocusProfile {
                strategy: crate::focus::FocusStrategy::OpenOnly,
                shortcut: None,
                recorded: None,
                point: None,
            },
        );
        assert_eq!(
            focus_profile_for_target(&profiles, custom)
                .expect("自定义档案")
                .strategy,
            crate::focus::FocusStrategy::OpenOnly
        );
        assert!(focus_profile_for_target(&profiles, r"C:\Tools\Missing.exe").is_none());
    }

    #[test]
    fn open_app_effective_profile_keeps_explicit_profile_for_custom_paths() {
        let mut mappings = ButtonMappings::default();
        assert_eq!(
            open_app_effective_profile(&mappings, "notepad")
                .expect("预设默认档案")
                .strategy,
            crate::focus::FocusStrategy::AutoComposer
        );

        // 同名但不在预设探测路径里的自定义程序不能被误认成预设。
        let custom_path = r"C:\Tools\notepad.exe";
        mappings.focus_profiles.insert(
            custom_path.to_owned(),
            crate::focus::AppFocusProfile {
                strategy: crate::focus::FocusStrategy::RecordedElement,
                ..Default::default()
            },
        );
        assert_eq!(
            open_app_effective_profile(&mappings, custom_path)
                .expect("自定义路径保留显式档案")
                .strategy,
            crate::focus::FocusStrategy::RecordedElement
        );
    }

    /// 预设应用没有显式档案时使用产品默认自动聚焦。
    #[test]
    fn open_app_preset_requests_auto_composer() {
        let _gate_lock = crate::key_gate::lock_gate_tests();
        let gate = crate::key_gate::KeyGate::start();
        let injector = Arc::new(RecordingInjector::default());
        let snapshot = Arc::new(StdMutex::new(RawInputSnapshot::default()));
        let runtime = ButtonMappingRuntime::new(
            Arc::clone(&injector) as Arc<dyn MappingInjector>,
            Arc::new(UsageCounters::default()),
            snapshot,
            Arc::new(crate::macro_runtime::MacroRuntime::new(Arc::new(
                crate::macro_runtime::UnsupportedStepperSource,
            ))),
        );
        runtime.set_input_context(crate::RemoteModel::Rc001, true);
        let mut mappings = ButtonMappings::default();
        mappings.actions.insert(
            RemoteButton::Ok,
            ButtonActions {
                single: ButtonAction::OpenApp {
                    target: "notepad".to_owned(),
                },
                ..ButtonActions::default()
            },
        );
        runtime.set_mappings(mappings);

        let sender = runtime.sender();
        sender
            .send(EngineMessage::HidUsages(hid_usages_of(RemoteButton::Ok)))
            .unwrap();
        sender
            .send(EngineMessage::HidUsages(BTreeSet::new()))
            .unwrap();
        std::thread::sleep(Duration::from_millis(150));

        assert_eq!(
            injector.focus_requests.lock().unwrap().as_slice(),
            &[FocusCall::LaunchThenFocus {
                target: "notepad".to_owned(),
                strategy: "auto_composer",
            }],
            "预设应用无显式档案时必须自动聚焦"
        );
        assert_eq!(
            injector.launches.lock().unwrap().as_slice(),
            &["notepad".to_owned()]
        );
        drop(gate);
    }

    /// 「只打开」的冷启动前台保障不得依赖私有键位方案宿主：公开构建里宿主
    /// 不可用，仍必须先等、再按应用入口重试一次（2026-11-12 飞书现场）。
    #[test]
    fn open_only_foreground_retries_without_private_button_profile_host() {
        use std::cell::Cell;
        let waits = Cell::new(0);
        let activations = Cell::new(0);
        let observed = ensure_open_only_foreground(
            "target",
            2,
            || false,
            |_, _| {
                waits.set(waits.get() + 1);
                waits.get() >= 2
            },
            |_| {
                activations.set(activations.get() + 1);
                true
            },
        );
        assert!(observed);
        assert_eq!(waits.get(), 2);
        assert_eq!(activations.get(), 1);
        assert!(
            !crate::button_profiles::is_button_profile_host_available(),
            "公开构建默认没有私有宿主，测试必须覆盖这个前提"
        );
    }

    /// 正常托盘恢复/已在前台时不得先做可见窗口激活或等待，直接返回。
    #[test]
    fn open_only_foreground_returns_immediately_when_already_foreground() {
        use std::cell::Cell;
        let waits = Cell::new(0);
        let activations = Cell::new(0);
        let observed = ensure_open_only_foreground(
            "target",
            8,
            || false,
            |_, _| {
                waits.set(waits.get() + 1);
                true
            },
            |_| {
                activations.set(activations.get() + 1);
                true
            },
        );
        assert!(observed);
        assert_eq!(waits.get(), 1);
        assert_eq!(activations.get(), 0);
    }

    /// 用户把刚打开的窗口关掉后，补偿路径只能尝试激活“已存在的可见窗口”；
    /// 没有可见窗口时必须保持关闭，不能再次启动应用。
    #[test]
    fn open_only_foreground_never_relaunches_a_closed_target() {
        use std::cell::Cell;
        let activations = Cell::new(0);
        let observed = ensure_open_only_foreground(
            "target",
            2,
            || false,
            |_, _| false,
            |_| {
                activations.set(activations.get() + 1);
                false
            },
        );
        assert!(!observed);
        // 每轮只尝试激活一次，绝不启动。
        assert_eq!(activations.get(), 2);
    }

    /// 新的打开请求产生后，旧的前台保障任务必须在下一轮检查时退出：
    /// 否则后台线程会在用户已经切到别的应用后继续抢焦点。
    #[test]
    fn open_only_foreground_stops_when_superseded_by_a_newer_open() {
        use std::cell::Cell;
        let waits = Cell::new(0);
        let activations = Cell::new(0);
        let observed = ensure_open_only_foreground(
            "target",
            8,
            || waits.get() >= 2,
            |_, _| {
                waits.set(waits.get() + 1);
                false
            },
            |_| {
                activations.set(activations.get() + 1);
                false
            },
        );
        assert!(!observed);
        // 零等待检查 + 一轮 500ms 等待后，第二轮开始前发现已被取代。
        assert_eq!(waits.get(), 2);
        assert_eq!(activations.get(), 1);
    }

    /// 自定义应用没有聚焦档案时保持「只打开」，不擅自抢焦点。
    #[test]
    fn open_app_custom_without_profile_stays_open_only() {
        let _gate_lock = crate::key_gate::lock_gate_tests();
        let gate = crate::key_gate::KeyGate::start();
        let injector = Arc::new(RecordingInjector::default());
        let snapshot = Arc::new(StdMutex::new(RawInputSnapshot::default()));
        let runtime = ButtonMappingRuntime::new(
            Arc::clone(&injector) as Arc<dyn MappingInjector>,
            Arc::new(UsageCounters::default()),
            snapshot,
            Arc::new(crate::macro_runtime::MacroRuntime::new(Arc::new(
                crate::macro_runtime::UnsupportedStepperSource,
            ))),
        );
        runtime.set_input_context(crate::RemoteModel::Rc001, true);
        let target = r"C:\Tools\custom.exe";
        let mut mappings = ButtonMappings::default();
        mappings.actions.insert(
            RemoteButton::Ok,
            ButtonActions {
                single: ButtonAction::OpenApp {
                    target: target.to_owned(),
                },
                ..ButtonActions::default()
            },
        );
        runtime.set_mappings(mappings);

        let sender = runtime.sender();
        sender
            .send(EngineMessage::HidUsages(hid_usages_of(RemoteButton::Ok)))
            .unwrap();
        sender
            .send(EngineMessage::HidUsages(BTreeSet::new()))
            .unwrap();
        std::thread::sleep(Duration::from_millis(150));

        assert_eq!(
            injector.focus_requests.lock().unwrap().as_slice(),
            &[FocusCall::LaunchThenFocus {
                target: target.to_owned(),
                strategy: "none",
            }],
            "自定义应用没有档案时只打开"
        );
        drop(gate);
    }

    #[test]
    fn gate_edge_and_hid_report_merge_into_one_press() {
        let _gate_test = GATE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let injector = Arc::new(RecordingInjector::default());
        let snapshot = Arc::new(StdMutex::new(RawInputSnapshot::default()));
        let runtime = ButtonMappingRuntime::new(
            Arc::clone(&injector) as Arc<dyn MappingInjector>,
            Arc::new(UsageCounters::default()),
            snapshot,
            Arc::new(crate::macro_runtime::MacroRuntime::new(Arc::new(
                crate::macro_runtime::UnsupportedStepperSource,
            ))),
        );
        runtime.set_input_context(crate::RemoteModel::Rc001, true);
        runtime.set_mappings(mappings_with_single(RemoteButton::Ok, KeyCode::Enter));

        let edges = Arc::new(StdMutex::new(Vec::new()));
        let edge_sink = Arc::clone(&edges);
        runtime.subscribe_button_edges(Arc::new(move |edge| {
            edge_sink.lock().unwrap().push(edge);
        }));

        let sender = runtime.sender();
        // 同一次物理按下：门控吞下的键盘边沿 + HID 报文（双源）。
        sender
            .send(EngineMessage::GateEdge(ButtonEdge {
                button: RemoteButton::Ok,
                is_pressed: true,
            }))
            .unwrap();
        sender
            .send(EngineMessage::HidUsages(hid_usages_of(RemoteButton::Ok)))
            .unwrap();
        std::thread::sleep(Duration::from_millis(100));
        // 双源并集去重：只产出一次按下边沿。
        assert_eq!(
            edges.lock().unwrap().as_slice(),
            &[ButtonEdge {
                button: RemoteButton::Ok,
                is_pressed: true
            }]
        );

        // 双源释放：门控 UP + 空 HID 报文 → 一次释放边沿。
        edges.lock().unwrap().clear();
        sender
            .send(EngineMessage::GateEdge(ButtonEdge {
                button: RemoteButton::Ok,
                is_pressed: false,
            }))
            .unwrap();
        sender
            .send(EngineMessage::HidUsages(BTreeSet::new()))
            .unwrap();
        std::thread::sleep(Duration::from_millis(100));
        assert_eq!(
            edges.lock().unwrap().as_slice(),
            &[ButtonEdge {
                button: RemoteButton::Ok,
                is_pressed: false
            }]
        );
    }

    #[test]
    fn listener_stop_releases_held_buttons_without_firing() {
        let _gate_test = GATE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let injector = Arc::new(RecordingInjector::default());
        let snapshot = Arc::new(StdMutex::new(RawInputSnapshot::default()));
        let runtime = ButtonMappingRuntime::new(
            Arc::clone(&injector) as Arc<dyn MappingInjector>,
            Arc::new(UsageCounters::default()),
            Arc::clone(&snapshot),
            Arc::new(crate::macro_runtime::MacroRuntime::new(Arc::new(
                crate::macro_runtime::UnsupportedStepperSource,
            ))),
        );
        runtime.set_input_context(crate::RemoteModel::Rc001, true);
        // 双击配置：释放后进入双击窗口（悬而未决），监听器停止必须取消它。
        let mut mappings = ButtonMappings::default();
        mappings.actions.insert(
            RemoteButton::Ok,
            ButtonActions {
                single: ButtonAction::Shortcut {
                    chord: KeyChord {
                        keys: vec![KeyCode::Enter],
                    },
                },
                double: ButtonAction::Shortcut {
                    chord: KeyChord {
                        keys: vec![KeyCode::Space],
                    },
                },
                long: ButtonAction::Disabled,
                hold_repeat: None,
                ok_context_click: false,
                extra: BTreeMap::new(),
                ..ButtonActions::default()
            },
        );
        runtime.set_mappings(mappings);

        let fired = Arc::new(StdMutex::new(Vec::new()));
        let fired_sink = Arc::clone(&fired);
        runtime.subscribe_button_gestures(Arc::new(move |gesture| {
            fired_sink.lock().unwrap().push(gesture);
        }));

        let sender = runtime.sender();
        sender
            .send(EngineMessage::HidUsages(hid_usages_of(RemoteButton::Ok)))
            .unwrap();
        sender
            .send(EngineMessage::HidUsages(BTreeSet::new()))
            .unwrap();
        std::thread::sleep(Duration::from_millis(50));
        sender.send(EngineMessage::ListenerStopped).unwrap();
        // 双击窗口（300ms）过后不应补发单击。
        std::thread::sleep(Duration::from_millis(450));
        assert!(
            fired.lock().unwrap().is_empty(),
            "监听器停止后挂起的双击窗口不得触发单击"
        );
        assert!(snapshot.lock().unwrap().active_buttons.is_empty());
    }

    #[cfg(windows)]
    #[test]
    fn focus_request_waiting_for_lock_cannot_restart_after_cancel() {
        let injector = Arc::new(SendInputInjector::new(Arc::new(
            crate::send_input_windows::SendInputRuntime::new(),
        )));
        let queued_generation = injector.focus_generation.load(Ordering::Acquire);
        let slot = injector.focus.lock().unwrap();
        let request = Arc::clone(&injector);
        let queued = std::thread::spawn(move || {
            request.with_focus_runner_generation(queued_generation, |_| true)
        });
        // 旧请求已取得代次，取消不等待其持有者正在等待的 focus 锁。
        injector.cancel_focus();
        drop(slot);
        assert_eq!(
            queued.join().unwrap(),
            None,
            "取消前的排队请求不得重开聚焦服务"
        );
        assert!(injector.focus.lock().unwrap().is_none());
        assert_eq!(
            injector.with_focus_runner(|_| true),
            Some(true),
            "取消后的新明确请求仍可启动服务"
        );
        injector.cancel_focus();
    }

    #[cfg(windows)]
    #[test]
    fn focus_shutdown_rejects_new_manual_requests() {
        let injector =
            SendInputInjector::new(Arc::new(crate::send_input_windows::SendInputRuntime::new()));
        injector.close_focus();
        assert_eq!(injector.with_focus_runner(|_| true), None);
        assert!(injector.focus_frontmost().is_err());
        assert!(injector.learn_focus_target().is_err());
        assert_eq!(
            injector.launch_app_and_focus("never-launch-during-exit", None, None),
            Err("应用正在退出".to_owned())
        );
        assert!(injector.focus.lock().unwrap().is_none());
        injector.close_focus();
        assert_eq!(injector.with_focus_runner(|_| true), None);
    }

    #[test]
    fn lifecycle_boundaries_cancel_pending_focus() {
        let _gate_test = GATE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let injector = Arc::new(RecordingInjector::default());
        let runtime = ButtonMappingRuntime::new(
            Arc::clone(&injector) as Arc<dyn MappingInjector>,
            Arc::new(UsageCounters::default()),
            Arc::new(StdMutex::new(RawInputSnapshot::default())),
            Arc::new(crate::macro_runtime::MacroRuntime::new(Arc::new(
                crate::macro_runtime::UnsupportedStepperSource,
            ))),
        );
        flush(&runtime);
        let initial = *injector.focus_cancellations.lock().unwrap();
        let sender = runtime.sender();
        sender.send(EngineMessage::ListenerStopped).unwrap();
        sender.send(EngineMessage::DeviceRemoved).unwrap();
        sender.send(EngineMessage::SetSuspended(true)).unwrap();
        sender.send(EngineMessage::SetSuspended(false)).unwrap();
        let mut disabled = ButtonMappings::default();
        disabled.enabled = false;
        sender
            .send(EngineMessage::MappingsChanged {
                execution: disabled.clone(),
                recognition: disabled,
                enhanced: false,
            })
            .unwrap();
        flush(&runtime);
        assert_eq!(*injector.focus_cancellations.lock().unwrap(), initial + 4);
        drop(runtime);
        assert_eq!(*injector.focus_cancellations.lock().unwrap(), initial + 5);
    }

    #[test]
    fn usage_counters_record_deduped_presses() {
        let _gate_test = GATE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let usage = Arc::new(UsageCounters::default());
        let snapshot = Arc::new(StdMutex::new(RawInputSnapshot::default()));
        let runtime = ButtonMappingRuntime::new(
            Arc::new(RecordingInjector::default()) as Arc<dyn MappingInjector>,
            Arc::clone(&usage),
            snapshot,
            Arc::new(crate::macro_runtime::MacroRuntime::new(Arc::new(
                crate::macro_runtime::UnsupportedStepperSource,
            ))),
        );
        runtime.set_input_context(crate::RemoteModel::Rc001, true);
        let sender = runtime.sender();
        // 双源同一次按下：语义按下只计一次。
        sender
            .send(EngineMessage::GateEdge(ButtonEdge {
                button: RemoteButton::Up,
                is_pressed: true,
            }))
            .unwrap();
        sender
            .send(EngineMessage::HidUsages(hid_usages_of(RemoteButton::Up)))
            .unwrap();
        std::thread::sleep(Duration::from_millis(100));
        assert_eq!(usage.snapshot().button_presses, 1);
    }

    #[test]
    fn set_mappings_preserves_unavailable_buttons() {
        let _gate_test = GATE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let runtime = ButtonMappingRuntime::new(
            Arc::new(RecordingInjector::default()) as Arc<dyn MappingInjector>,
            Arc::new(UsageCounters::default()),
            Arc::new(StdMutex::new(RawInputSnapshot::default())),
            Arc::new(crate::macro_runtime::MacroRuntime::new(Arc::new(
                crate::macro_runtime::UnsupportedStepperSource,
            ))),
        );
        runtime.set_input_context(crate::RemoteModel::Rc001, true);
        let mut mappings = ButtonMappings::default();
        mappings.actions.insert(
            RemoteButton::Left,
            ButtonActions {
                single: ButtonAction::Shortcut {
                    chord: KeyChord {
                        keys: vec![KeyCode::Backspace],
                    },
                },
                ..ButtonActions::default()
            },
        );
        let escape_action = ButtonActions {
            single: ButtonAction::Shortcut {
                chord: KeyChord {
                    keys: vec![KeyCode::Escape],
                },
            },
            ..ButtonActions::default()
        };
        for button in [
            RemoteButton::Back,
            RemoteButton::VolumeUp,
            RemoteButton::VolumeDown,
        ] {
            mappings.actions.insert(button, escape_action.clone());
        }
        mappings.actions.insert(
            RemoteButton::Tv,
            ButtonActions {
                single: ButtonAction::Shortcut {
                    chord: KeyChord {
                        keys: vec![KeyCode::LeftWindows],
                    },
                },
                ..ButtonActions::default()
            },
        );
        runtime.set_mappings(mappings);
        let effective = runtime.mappings();
        assert!(
            effective.actions.contains_key(&RemoteButton::Left),
            "左键自定义必须保留"
        );
        assert!(effective.actions.contains_key(&RemoteButton::Back));
        assert_eq!(
            effective
                .actions
                .get(&RemoteButton::Tv)
                .map(|a| a.single.clone()),
            Some(ButtonAction::Shortcut {
                chord: KeyChord {
                    keys: vec![KeyCode::LeftWindows],
                },
            }),
        );
        assert_eq!(
            effective.action_for(RemoteButton::Left, ButtonTrigger::Single),
            ButtonAction::Shortcut {
                chord: KeyChord {
                    keys: vec![KeyCode::Backspace],
                },
            },
        );
        assert_eq!(
            effective.action_for(RemoteButton::Back, ButtonTrigger::Single),
            ButtonAction::Shortcut {
                chord: KeyChord {
                    keys: vec![KeyCode::Escape],
                },
            },
        );
    }

    #[test]
    fn scene_recognition_placeholders_do_not_enable_default_repeat() {
        let scene = mappings_with_single(RemoteButton::Back, KeyCode::Escape);
        let recognition = merge_recognition_mappings(&ButtonMappings::default(), &scene);
        let config =
            crate::button_gestures::GestureConfig::for_button(&recognition, RemoteButton::Back)
                .unwrap();
        assert!(config.single_configured, "场景仍能识别单击");
        assert!(
            config.repeat_single.is_none(),
            "识别占位动作不得获得默认连发"
        );

        let execution = mappings_with_single(RemoteButton::Back, KeyCode::Backspace);
        let recognition = merge_recognition_mappings(&execution, &scene);
        let config =
            crate::button_gestures::GestureConfig::for_button(&recognition, RemoteButton::Back)
                .unwrap();
        assert_eq!(
            config.repeat_single,
            RemoteButton::Back.repeat_interval(),
            "真实映射的默认连发必须保留"
        );
    }

    /// 「按住连续触发」引擎路径（2026-10-05）：门控边沿驱动的连续拍注入、
    /// 释放即停；配置变化取消进行中的一趟、不得迟到拍。
    #[test]
    fn hold_repeat_long_injects_beats_until_release_or_reconfigure() {
        let _gate_lock = GATE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut gate: Option<crate::key_gate::KeyGate> = Some(crate::key_gate::KeyGate::start());
        let ensure_gate = |gate: &mut Option<crate::key_gate::KeyGate>| {
            if !crate::key_gate::is_gate_thread_alive() {
                *gate = None;
                std::thread::sleep(Duration::from_millis(50));
                *gate = Some(crate::key_gate::KeyGate::start());
                std::thread::sleep(Duration::from_millis(50));
            }
        };

        let injector = Arc::new(RecordingInjector::default());
        let runtime = ButtonMappingRuntime::new(
            Arc::clone(&injector) as Arc<dyn MappingInjector>,
            Arc::new(UsageCounters::default()),
            Arc::new(StdMutex::new(RawInputSnapshot::default())),
            Arc::new(crate::macro_runtime::MacroRuntime::new(Arc::new(
                crate::macro_runtime::UnsupportedStepperSource,
            ))),
        );
        runtime.set_input_context(crate::RemoteModel::Rc001, true);
        let mut mappings = ButtonMappings::default();
        // 长按=退格 + 开关=长按：550ms 触发后按 50ms 间隔连拍。
        mappings.actions.insert(
            RemoteButton::Back,
            ButtonActions {
                long: ButtonAction::Shortcut {
                    chord: KeyChord {
                        keys: vec![KeyCode::Backspace],
                    },
                },
                hold_repeat: Some(HoldRepeat::Long),
                ..ButtonActions::default()
            },
        );
        runtime.set_mappings(mappings);

        let sender = runtime.sender();
        let taps = || injector.taps.lock().unwrap().clone();

        ensure_gate(&mut gate);
        let base = taps().len();
        sender
            .send(EngineMessage::GateEdge(ButtonEdge {
                button: RemoteButton::Back,
                is_pressed: true,
            }))
            .unwrap();
        // 550ms 触发长按 + 50ms 间隔：三拍预算放宽到 2000ms（慢机每拍有延迟）。
        let got_three = wait_until(|| taps().len() >= base + 3, Duration::from_millis(2000));
        assert!(
            got_three,
            "长按连续触发应注入 ≥3 拍，实际 {}",
            taps().len() - base
        );
        sender
            .send(EngineMessage::GateEdge(ButtonEdge {
                button: RemoteButton::Back,
                is_pressed: false,
            }))
            .unwrap();
        std::thread::sleep(Duration::from_millis(150));
        let after_release = taps().len() - base;
        std::thread::sleep(Duration::from_millis(250));
        assert_eq!(taps().len() - base, after_release, "释放后不得再有连续拍");

        // 配置变化取消进行中的一趟：置空映射后不得再有拍。
        let before_hold = taps().len();
        sender
            .send(EngineMessage::GateEdge(ButtonEdge {
                button: RemoteButton::Back,
                is_pressed: true,
            }))
            .unwrap();
        assert!(
            wait_until(|| taps().len() > before_hold, Duration::from_millis(2000)),
            "第二次按住应先注入拍"
        );
        runtime.set_mappings(ButtonMappings::default());
        let after_reconfigure = taps().len();
        std::thread::sleep(Duration::from_millis(300));
        assert_eq!(taps().len(), after_reconfigure, "配置变化后不得再有连续拍");

        drop(runtime);
        drop(gate);
    }

    #[test]
    fn repeat_run_signals_ignore_one_shot_gestures() {
        use crate::button_gestures::GestureRecognizer;
        use crate::send_input::{ButtonTrigger, KeyChord};

        // 评审发现 1 回归：双击窗补发的单击与按住确认的首次单击都不得计成
        // map_repeat 拍，只有连续触发计时器产生的拍才进聚合。
        let mut mappings = ButtonMappings::default();
        mappings.actions.insert(
            RemoteButton::Back,
            ButtonActions {
                single: ButtonAction::Shortcut {
                    chord: KeyChord {
                        keys: vec![KeyCode::Delete],
                    },
                },
                double: ButtonAction::Shortcut {
                    chord: KeyChord {
                        keys: vec![KeyCode::Space],
                    },
                },
                long: ButtonAction::Disabled,
                hold_repeat: Some(HoldRepeat::Single),
                ok_context_click: false,
                extra: BTreeMap::new(),
                ..ButtonActions::default()
            },
        );
        let mut recognizer = GestureRecognizer::new();
        recognizer.configure(&mappings);
        let mut runs: RepeatRuns = BTreeMap::new();
        let t0 = Instant::now();

        // 快速点按 → 双击窗超时补发的单击：不是拍。
        recognizer.press(RemoteButton::Back, t0);
        assert!(recognizer
            .release(RemoteButton::Back, t0 + Duration::from_millis(60))
            .is_empty());
        let (_, ticks) = recognizer.advance_with_ticks(t0 + Duration::from_millis(360));
        note_advance_ticks(&mut runs, ticks);
        assert!(runs.is_empty(), "双击窗补发的单击不得计成连续拍");

        // 按住 → 300ms 的按住确认不是拍，350ms 起拍才是。
        let t1 = t0 + Duration::from_secs(2);
        recognizer.press(RemoteButton::Back, t1);
        let (_, ticks) = recognizer.advance_with_ticks(t1 + Duration::from_millis(300));
        note_advance_ticks(&mut runs, ticks);
        assert!(runs.is_empty(), "按住确认的首次单击不得计成连续拍");
        let (_, ticks) = recognizer.advance_with_ticks(t1 + Duration::from_millis(350));
        note_advance_ticks(&mut runs, ticks);
        assert_eq!(runs.get(&RemoteButton::Back).map(|run| run.1), Some(1));
        assert!(recognizer
            .release(RemoteButton::Back, t1 + Duration::from_millis(400))
            .is_empty());
        finish_repeat_run(&mut runs, RemoteButton::Back, "release");
        assert!(runs.is_empty(), "释放后该趟聚合必须收尾");
    }

    #[test]
    fn ok_context_click_window_boundaries() {
        let t0 = Instant::now();
        assert!(ok_context_click_window_hit(true, Some(t0), t0));
        assert!(ok_context_click_window_hit(
            true,
            Some(t0),
            t0 + OK_CONTEXT_CLICK_WINDOW
        ));
        assert!(
            !ok_context_click_window_hit(
                true,
                Some(t0),
                t0 + OK_CONTEXT_CLICK_WINDOW + Duration::from_millis(1)
            ),
            "超过 5 秒必须落空"
        );
        assert!(!ok_context_click_window_hit(false, Some(t0), t0));
        assert!(!ok_context_click_window_hit(true, None, t0));
    }

    #[test]
    fn ok_context_click_fires_after_a_remote_move_and_swallows_the_press() {
        let _gate_lock = GATE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut gate: Option<crate::key_gate::KeyGate> = Some(crate::key_gate::KeyGate::start());
        let ensure_gate = |gate: &mut Option<crate::key_gate::KeyGate>| {
            if !crate::key_gate::is_gate_thread_alive() {
                *gate = None;
                std::thread::sleep(Duration::from_millis(50));
                *gate = Some(crate::key_gate::KeyGate::start());
                std::thread::sleep(Duration::from_millis(50));
            }
        };

        let injector = Arc::new(RecordingInjector::default());
        let runtime = ButtonMappingRuntime::new(
            Arc::clone(&injector) as Arc<dyn MappingInjector>,
            Arc::new(UsageCounters::default()),
            Arc::new(StdMutex::new(RawInputSnapshot::default())),
            Arc::new(crate::macro_runtime::MacroRuntime::new(Arc::new(
                crate::macro_runtime::UnsupportedStepperSource,
            ))),
        );
        runtime.set_input_context(crate::RemoteModel::Rc001, true);
        let mut mappings = ButtonMappings::default();
        // 上键单击 = 移动光标（移动注入成功会刷新 OK 点击窗口）。
        mappings.actions.insert(
            RemoteButton::Up,
            ButtonActions {
                single: ButtonAction::MouseMove {
                    direction: MoveDirection::Right,
                    distance: 40,
                },
                ..ButtonActions::default()
            },
        );
        // OK：单击 = 回车（原动作）+ 开启上下文点击。
        mappings.actions.insert(
            RemoteButton::Ok,
            ButtonActions {
                single: ButtonAction::Shortcut {
                    chord: KeyChord {
                        keys: vec![KeyCode::Enter],
                    },
                },
                ok_context_click: true,
                ..ButtonActions::default()
            },
        );
        runtime.set_mappings(mappings);

        let sender = runtime.sender();
        let clicks = || injector.clicks.lock().unwrap().clone();
        let taps = || injector.taps.lock().unwrap().len();

        // 1) 遥控器移动光标：注入成功 → 窗口刷新。
        ensure_gate(&mut gate);
        sender
            .send(EngineMessage::GateEdge(ButtonEdge {
                button: RemoteButton::Up,
                is_pressed: true,
            }))
            .unwrap();
        assert!(
            wait_until(
                || !injector.moves.lock().unwrap().is_empty(),
                Duration::from_millis(1000)
            ),
            "上键应注入一次鼠标移动"
        );
        sender
            .send(EngineMessage::GateEdge(ButtonEdge {
                button: RemoteButton::Up,
                is_pressed: false,
            }))
            .unwrap();

        // 2) 窗口内按 OK：注入左键单击；不注入原动作（回车）。
        let taps_before = taps();
        sender
            .send(EngineMessage::GateEdge(ButtonEdge {
                button: RemoteButton::Ok,
                is_pressed: true,
            }))
            .unwrap();
        assert!(
            wait_until(|| !clicks().is_empty(), Duration::from_millis(1000)),
            "窗口内按 OK 应注入一次点击"
        );
        assert_eq!(clicks(), vec![MouseClickKind::Left]);
        sender
            .send(EngineMessage::GateEdge(ButtonEdge {
                button: RemoteButton::Ok,
                is_pressed: false,
            }))
            .unwrap();
        std::thread::sleep(Duration::from_millis(150));
        assert_eq!(taps(), taps_before, "上下文点击不得再执行 OK 的原动作");
        assert_eq!(clicks().len(), 1, "一次按压只点一次");

        // 3) 关闭开关后：按 OK 走原动作，不再点击。
        let mut off = ButtonMappings::default();
        off.actions.insert(
            RemoteButton::Ok,
            ButtonActions {
                single: ButtonAction::Shortcut {
                    chord: KeyChord {
                        keys: vec![KeyCode::Enter],
                    },
                },
                ok_context_click: false,
                ..ButtonActions::default()
            },
        );
        runtime.set_mappings(off);
        std::thread::sleep(Duration::from_millis(50));
        let taps_before_off = taps();
        let clicks_before_off = clicks().len();
        sender
            .send(EngineMessage::GateEdge(ButtonEdge {
                button: RemoteButton::Ok,
                is_pressed: true,
            }))
            .unwrap();
        assert!(
            wait_until(|| taps() > taps_before_off, Duration::from_millis(1000)),
            "开关关闭后按 OK 应执行原动作"
        );
        sender
            .send(EngineMessage::GateEdge(ButtonEdge {
                button: RemoteButton::Ok,
                is_pressed: false,
            }))
            .unwrap();
        std::thread::sleep(Duration::from_millis(100));
        assert_eq!(clicks().len(), clicks_before_off, "开关关闭后不得点击");

        drop(runtime);
        drop(gate);
    }

    /// 常用语面板桩（经 [`RecordingInjector`] 独立注入，不写进程级槽位，
    /// 避免用例并行时互相污染）：只认领被显式指派的边沿，记录全部到达的调用。
    #[derive(Debug, Default)]
    struct MockPhraseHost {
        /// 这些按键的按下沿返回「已消费」。
        claim_presses: StdMutex<BTreeSet<RemoteButton>>,
        /// 这些按键的释放沿返回「已消费」（模拟组件内部自持在按状态的实现形态）。
        claim_releases: StdMutex<BTreeSet<RemoteButton>>,
        /// 到达 `handle` 的全部边沿（顺序）。
        edges: StdMutex<Vec<(RemoteButton, bool)>>,
        /// `reset` 的原因记录。
        resets: StdMutex<Vec<String>>,
    }

    impl crate::common_phrases::CommonPhraseHost for MockPhraseHost {
        fn handle(&self, button: RemoteButton, is_pressed: bool, _source: &str) -> bool {
            self.edges.lock().unwrap().push((button, is_pressed));
            if is_pressed {
                self.claim_presses.lock().unwrap().contains(&button)
            } else {
                self.claim_releases.lock().unwrap().contains(&button)
            }
        }

        fn reset(&self, reason: &str) {
            self.resets.lock().unwrap().push(reason.to_owned());
        }
    }

    /// 双击共存的映射：释放沿才进入手势判定——认领漏出手势的回归会以
    /// 「幽灵单击/幽灵双击」暴露。
    fn mappings_with_double(
        button: RemoteButton,
        single: KeyCode,
        double: KeyCode,
    ) -> ButtonMappings {
        let mut mappings = ButtonMappings::default();
        mappings.actions.insert(
            button,
            ButtonActions {
                single: ButtonAction::Shortcut {
                    chord: KeyChord { keys: vec![single] },
                },
                double: ButtonAction::Shortcut {
                    chord: KeyChord { keys: vec![double] },
                },
                ..ButtonActions::default()
            },
        );
        mappings
    }

    /// 单击 + 长按共存的映射：按住超过阈值触发长按——残留的按压状态会以
    /// 「幽灵长按」暴露。
    fn mappings_with_long(button: RemoteButton, single: KeyCode, long: KeyCode) -> ButtonMappings {
        let mut mappings = ButtonMappings::default();
        mappings.actions.insert(
            button,
            ButtonActions {
                single: ButtonAction::Shortcut {
                    chord: KeyChord { keys: vec![single] },
                },
                long: ButtonAction::Shortcut {
                    chord: KeyChord { keys: vec![long] },
                },
                ..ButtonActions::default()
            },
        );
        mappings
    }

    fn spawn_phrase_engine(
        injector: &Arc<RecordingInjector>,
        mappings: ButtonMappings,
    ) -> (ButtonMappingRuntime, Arc<StdMutex<Vec<FiredGesture>>>) {
        let runtime = ButtonMappingRuntime::new(
            Arc::clone(injector) as Arc<dyn MappingInjector>,
            Arc::new(UsageCounters::default()),
            Arc::new(StdMutex::new(RawInputSnapshot::default())),
            Arc::new(crate::macro_runtime::MacroRuntime::new(Arc::new(
                crate::macro_runtime::UnsupportedStepperSource,
            ))),
        );
        runtime.set_input_context(crate::RemoteModel::Rc001, true);
        runtime.set_mappings(mappings);
        let fired = Arc::new(StdMutex::new(Vec::new()));
        let fired_sink = Arc::clone(&fired);
        runtime.subscribe_button_gestures(Arc::new(move |gesture| {
            fired_sink.lock().unwrap().push(gesture);
        }));
        (runtime, fired)
    }

    #[test]
    fn capture_handoff_cancels_pending_and_held_gestures_without_reconfiguring() {
        let _gate_lock = GATE_TEST_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let _gate = crate::key_gate::KeyGate::start();
        for case in ["double_pending", "long_pending", "repeat_running"] {
            let injector = Arc::new(RecordingInjector::default());
            let mut mappings = if case == "double_pending" {
                mappings_with_double(RemoteButton::Back, KeyCode::Backspace, KeyCode::Enter)
            } else {
                mappings_with_long(RemoteButton::Back, KeyCode::Backspace, KeyCode::Enter)
            };
            mappings
                .actions
                .get_mut(&RemoteButton::Back)
                .unwrap()
                .hold_repeat = Some(if case == "repeat_running" {
                HoldRepeat::Long
            } else {
                HoldRepeat::Off
            });
            let (runtime, fired) = spawn_phrase_engine(&injector, mappings);
            runtime.set_input_context(crate::RemoteModel::Rc003, true);
            let owned_mask = 1 << RemoteButton::Back.ordinal();
            runtime.set_capture_owned(owned_mask);
            let sender = runtime.sender();
            sender
                .send(EngineMessage::DriverEdge(ButtonEdge {
                    button: RemoteButton::Back,
                    is_pressed: true,
                }))
                .unwrap();
            if case == "double_pending" {
                sender
                    .send(EngineMessage::DriverEdge(ButtonEdge {
                        button: RemoteButton::Back,
                        is_pressed: false,
                    }))
                    .unwrap();
            }
            if case == "repeat_running" {
                assert!(wait_until(
                    || !injector.taps.lock().unwrap().is_empty(),
                    Duration::from_secs(2)
                ));
            }
            sender
                .send(EngineMessage::CaptureContinuityReset {
                    old_owned_mask: owned_mask,
                })
                .unwrap();
            assert!(runtime.wait_for_idle(Duration::from_secs(1)));
            let before_release = fired.lock().unwrap().len();
            let before_taps = injector.taps.lock().unwrap().len();
            if case != "double_pending" {
                sender
                    .send(EngineMessage::DriverEdge(ButtonEdge {
                        button: RemoteButton::Back,
                        is_pressed: false,
                    }))
                    .unwrap();
            }
            assert!(runtime.wait_for_idle(Duration::from_secs(1)));
            std::thread::sleep(Duration::from_millis(700));
            assert_eq!(
                fired.lock().unwrap().len(),
                before_release,
                "{case} fired after cancellation"
            );
            assert_eq!(injector.taps.lock().unwrap().len(), before_taps);
            assert_eq!(
                runtime.configuration.lock().unwrap().captured_mask,
                owned_mask
            );
            assert!(runtime.configuration.lock().unwrap().enhanced);
            sender
                .send(EngineMessage::DriverEdge(ButtonEdge {
                    button: RemoteButton::Back,
                    is_pressed: true,
                }))
                .unwrap();
            sender
                .send(EngineMessage::DriverEdge(ButtonEdge {
                    button: RemoteButton::Back,
                    is_pressed: false,
                }))
                .unwrap();
            assert!(wait_until(
                || fired.lock().unwrap().len() > before_release,
                Duration::from_secs(1)
            ));
            assert!(runtime.wait_for_idle(Duration::from_secs(1)));
            assert_eq!(fired.lock().unwrap().len(), before_release + 1);
            assert_eq!(injector.taps.lock().unwrap().len(), before_taps + 1);
        }
    }

    #[test]
    fn capture_handoff_preserves_claimed_release_and_other_button_gesture() {
        let _gate_lock = GATE_TEST_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let _gate = crate::key_gate::KeyGate::start();
        let injector = Arc::new(RecordingInjector::default());
        let host = Arc::new(MockPhraseHost::default());
        host.claim_presses
            .lock()
            .unwrap()
            .insert(RemoteButton::Back);
        injector.set_phrase_host(host.clone());
        let mut mappings =
            mappings_with_double(RemoteButton::Back, KeyCode::Backspace, KeyCode::Enter);
        mappings.actions.extend(
            mappings_with_double(RemoteButton::Left, KeyCode::Space, KeyCode::Enter).actions,
        );
        let (runtime, fired) = spawn_phrase_engine(&injector, mappings);
        runtime.set_input_context(crate::RemoteModel::Rc003, true);
        runtime.set_capture_owned(
            (1 << RemoteButton::Back.ordinal()) | (1 << RemoteButton::Left.ordinal()),
        );
        let sender = runtime.sender();
        for (button, is_pressed) in [
            (RemoteButton::Back, true),
            (RemoteButton::Left, true),
            (RemoteButton::Left, false),
        ] {
            sender
                .send(EngineMessage::DriverEdge(ButtonEdge { button, is_pressed }))
                .unwrap();
        }
        sender
            .send(EngineMessage::CaptureContinuityReset {
                old_owned_mask: 1 << RemoteButton::Back.ordinal(),
            })
            .unwrap();
        sender
            .send(EngineMessage::DriverEdge(ButtonEdge {
                button: RemoteButton::Back,
                is_pressed: false,
            }))
            .unwrap();
        assert!(wait_until(
            || !fired.lock().unwrap().is_empty(),
            Duration::from_secs(1)
        ));
        assert!(runtime.wait_for_idle(Duration::from_secs(1)));
        assert_eq!(
            host.edges
                .lock()
                .unwrap()
                .iter()
                .filter(|(button, _)| *button == RemoteButton::Back)
                .copied()
                .collect::<Vec<_>>(),
            vec![(RemoteButton::Back, true), (RemoteButton::Back, false)]
        );
        assert!(host.resets.lock().unwrap().is_empty());
        assert_eq!(
            fired
                .lock()
                .unwrap()
                .iter()
                .map(|gesture| gesture.button)
                .collect::<Vec<_>>(),
            vec![RemoteButton::Left]
        );
        assert_eq!(injector.taps.lock().unwrap().len(), 1);
    }

    /// 认领早于手势识别（评审 M2）：面板认领的按下沿不进入单击/双击/长按判定；
    /// 未认领时照常触发（对照）。
    #[test]
    fn phrase_claim_precedes_gesture_recognition() {
        let _gate_lock = crate::key_gate::lock_gate_tests();
        let injector = Arc::new(RecordingInjector::default());
        let host = Arc::new(MockPhraseHost::default());
        injector
            .set_phrase_host(Arc::clone(&host) as Arc<dyn crate::common_phrases::CommonPhraseHost>);
        let (runtime, fired) = spawn_phrase_engine(
            &injector,
            mappings_with_single(RemoteButton::Ok, KeyCode::Enter),
        );
        let sender = runtime.sender();

        // 对照：未认领的按下+释放照常进入手势识别。
        sender
            .send(EngineMessage::HidUsages(hid_usages_of(RemoteButton::Ok)))
            .unwrap();
        sender
            .send(EngineMessage::HidUsages(BTreeSet::new()))
            .unwrap();
        assert!(
            wait_until(
                || host.edges.lock().unwrap().len() >= 2,
                Duration::from_millis(1000)
            ),
            "未认领的边沿必须到达组件"
        );
        assert_eq!(fired.lock().unwrap().len(), 1, "未认领时手势照常触发");

        // 认领：按下沿与释放沿都不进入手势识别。
        host.claim_presses.lock().unwrap().insert(RemoteButton::Ok);
        sender
            .send(EngineMessage::HidUsages(hid_usages_of(RemoteButton::Ok)))
            .unwrap();
        sender
            .send(EngineMessage::HidUsages(BTreeSet::new()))
            .unwrap();
        assert!(
            wait_until(
                || host.edges.lock().unwrap().len() >= 4,
                Duration::from_millis(1000)
            ),
            "认领的按下与释放都必须到达组件"
        );
        std::thread::sleep(Duration::from_millis(150));
        assert_eq!(
            fired.lock().unwrap().len(),
            1,
            "被认领的按下不得触达手势识别"
        );
        drop(runtime);
    }

    /// 认领的 DOWN 不漏出手势（评审 M2）：释放沿即使不再被组件认领
    /// （面板已关/组件只认按下），也必须由公开侧配对消费——否则该释放会对一个
    /// 从未开始的按压做判定，污染下一轮为「幽灵双击」。
    #[test]
    fn phrase_claimed_press_release_is_consumed_without_ghost_gestures() {
        let _gate_lock = crate::key_gate::lock_gate_tests();
        let injector = Arc::new(RecordingInjector::default());
        let host = Arc::new(MockPhraseHost::default());
        injector
            .set_phrase_host(Arc::clone(&host) as Arc<dyn crate::common_phrases::CommonPhraseHost>);
        let (runtime, fired) = spawn_phrase_engine(
            &injector,
            mappings_with_double(RemoteButton::Ok, KeyCode::Enter, KeyCode::Space),
        );
        let sender = runtime.sender();

        // 第一轮：按下被认领；释放不被认领 → 必须由公开侧配对消费。
        host.claim_presses.lock().unwrap().insert(RemoteButton::Ok);
        send_ok_press_and_release(&sender);
        assert!(
            wait_until(
                || host.edges.lock().unwrap().len() >= 2,
                Duration::from_millis(1000)
            ),
            "认领的按下与配对消费前的释放都必须到达组件"
        );

        // 第二轮：正常按压 → 必须得到一次单击（Enter），不得是双击（Space）。
        host.claim_presses.lock().unwrap().clear();
        send_ok_press_and_release(&sender);
        assert!(
            wait_until(
                || !fired.lock().unwrap().is_empty(),
                Duration::from_millis(1500)
            ),
            "第二轮必须触发一次手势"
        );
        std::thread::sleep(Duration::from_millis(150));
        assert_eq!(
            fired.lock().unwrap().as_slice(),
            &[FiredGesture {
                button: RemoteButton::Ok,
                trigger: ButtonTrigger::Single
            }],
            "第一轮被认领的按压不得留下任何手势状态（否则第二轮会变成幽灵双击）"
        );
        drop(runtime);
    }

    /// 一次 OK 的 HID 按下 + 释放（顺序保证引擎按 FIFO 处理）。
    fn send_ok_press_and_release(sender: &std::sync::mpsc::Sender<EngineMessage>) {
        sender
            .send(EngineMessage::HidUsages(hid_usages_of(RemoteButton::Ok)))
            .unwrap();
        sender
            .send(EngineMessage::HidUsages(BTreeSet::new()))
            .unwrap();
    }

    /// handle 认领了「没有认领记录」的释放沿：先清该键残留手势状态再消费。
    /// 反例（修复前）：按下未被认领（长按计时已启动）→ 释放被认领 → 残留的
    /// 长按计时在 550ms 后引爆「幽灵长按」。
    #[test]
    fn panel_claimed_release_without_a_recorded_press_clears_stale_state() {
        let _gate_lock = crate::key_gate::lock_gate_tests();
        let injector = Arc::new(RecordingInjector::default());
        let host = Arc::new(MockPhraseHost::default());
        injector
            .set_phrase_host(Arc::clone(&host) as Arc<dyn crate::common_phrases::CommonPhraseHost>);
        let (runtime, fired) = spawn_phrase_engine(
            &injector,
            mappings_with_long(RemoteButton::Ok, KeyCode::Enter, KeyCode::Space),
        );
        let sender = runtime.sender();

        // 按下未被认领（进入长按计时），释放沿被认领（面板在这一刻打开）。
        sender
            .send(EngineMessage::HidUsages(hid_usages_of(RemoteButton::Ok)))
            .unwrap();
        assert!(
            wait_until(
                || host.edges.lock().unwrap().len() >= 1,
                Duration::from_millis(1000)
            ),
            "按下必须到达组件"
        );
        host.claim_releases.lock().unwrap().insert(RemoteButton::Ok);
        sender
            .send(EngineMessage::HidUsages(BTreeSet::new()))
            .unwrap();
        assert!(
            wait_until(
                || host.edges.lock().unwrap().len() >= 2,
                Duration::from_millis(1000)
            ),
            "释放沿必须到达组件"
        );

        // 越过长按阈值：被消费的释放沿不得留下任何可引爆的计时。
        std::thread::sleep(Duration::from_millis(700));
        assert!(
            fired.lock().unwrap().is_empty(),
            "被认领的释放沿必须先清掉残留按压状态（不得有幽灵长按）：{:?}",
            fired.lock().unwrap()
        );
        drop(runtime);
    }

    /// 来源失效（断连/监听停止/睡眠唤醒后的接口移除）必须通知组件收尾：
    /// 关闭面板并清认领状态（评审 B2 的公开侧调用点，2026-10-15）。
    #[test]
    fn device_removed_and_listener_stopped_reset_the_phrase_host() {
        let _gate_lock = crate::key_gate::lock_gate_tests();
        let injector = Arc::new(RecordingInjector::default());
        let host = Arc::new(MockPhraseHost::default());
        injector
            .set_phrase_host(Arc::clone(&host) as Arc<dyn crate::common_phrases::CommonPhraseHost>);
        let (runtime, _fired) = spawn_phrase_engine(
            &injector,
            mappings_with_single(RemoteButton::Ok, KeyCode::Enter),
        );
        let sender = runtime.sender();

        sender.send(EngineMessage::DeviceRemoved).unwrap();
        assert!(
            wait_until(
                || host
                    .resets
                    .lock()
                    .unwrap()
                    .iter()
                    .any(|reason| reason == "device_removed"),
                Duration::from_millis(1000)
            ),
            "设备移除必须 reset 常用语宿主：{:?}",
            host.resets.lock().unwrap()
        );
        sender.send(EngineMessage::ListenerStopped).unwrap();
        assert!(
            wait_until(
                || host
                    .resets
                    .lock()
                    .unwrap()
                    .iter()
                    .any(|reason| reason == "listener_stopped"),
                Duration::from_millis(1000)
            ),
            "监听停止必须 reset 常用语宿主：{:?}",
            host.resets.lock().unwrap()
        );
        drop(runtime);
    }

    /// 认领的按下遇到来源失效（DeviceRemoved）：release_all 的配对释放沿
    /// 由公开侧消费——不得对它做释放判定（否则双击窗被开启，300ms 后
    /// 补发一次幽灵单击）。
    #[test]
    fn phrase_claimed_press_survives_a_source_loss_without_ghost_gestures() {
        let _gate_lock = crate::key_gate::lock_gate_tests();
        let injector = Arc::new(RecordingInjector::default());
        let host = Arc::new(MockPhraseHost::default());
        injector
            .set_phrase_host(Arc::clone(&host) as Arc<dyn crate::common_phrases::CommonPhraseHost>);
        let (runtime, fired) = spawn_phrase_engine(
            &injector,
            mappings_with_double(RemoteButton::Ok, KeyCode::Enter, KeyCode::Space),
        );
        let sender = runtime.sender();

        host.claim_presses.lock().unwrap().insert(RemoteButton::Ok);
        sender
            .send(EngineMessage::HidUsages(hid_usages_of(RemoteButton::Ok)))
            .unwrap();
        assert!(
            wait_until(
                || host.edges.lock().unwrap().len() >= 1,
                Duration::from_millis(1000)
            ),
            "认领的按下必须到达组件"
        );

        sender.send(EngineMessage::DeviceRemoved).unwrap();
        assert!(
            wait_until(
                || host
                    .resets
                    .lock()
                    .unwrap()
                    .iter()
                    .any(|reason| reason == "device_removed"),
                Duration::from_millis(1000)
            ),
            "设备移除必须 reset 常用语宿主"
        );
        // 覆盖双击窗（300ms）之后仍不得出现任何补发手势。
        std::thread::sleep(Duration::from_millis(600));
        assert!(
            fired.lock().unwrap().is_empty(),
            "来源失效释放的按住不得触达手势识别（不得有幽灵单击/双击）：{:?}",
            fired.lock().unwrap()
        );
        drop(runtime);
    }

    fn single_actions(key: KeyCode) -> ButtonActions {
        ButtonActions {
            single: ButtonAction::Shortcut {
                chord: KeyChord { keys: vec![key] },
            },
            ..ButtonActions::default()
        }
    }

    fn overlay_with(button: RemoteButton, key: KeyCode) -> ButtonMappings {
        let mut overlay = ButtonMappings::default();
        overlay.actions.insert(button, single_actions(key));
        overlay
    }

    #[test]
    fn overlay_replaces_single_click_and_disables_secondary_gestures() {
        let base = mappings_with_single(RemoteButton::Ok, KeyCode::Enter);
        let overlay = overlay_with(RemoteButton::Ok, KeyCode::Space);

        assert_eq!(
            effective_action(
                &base,
                Some(&overlay),
                RemoteButton::Ok,
                ButtonTrigger::Single
            ),
            ButtonAction::Shortcut {
                chord: KeyChord {
                    keys: vec![KeyCode::Space]
                }
            }
        );
        assert_eq!(
            effective_action(
                &base,
                Some(&overlay),
                RemoteButton::Ok,
                ButtonTrigger::Double
            ),
            ButtonAction::Disabled,
            "方案只支持单击：双击必须被覆盖层显式关闭"
        );
        assert_eq!(
            effective_action(&base, Some(&overlay), RemoteButton::Ok, ButtonTrigger::Long),
            ButtonAction::Disabled,
            "方案只支持单击：长按必须被覆盖层显式关闭"
        );
    }

    #[test]
    fn overlay_disabled_still_swallows_the_key() {
        let base = ButtonMappings::default();
        let mut overlay = ButtonMappings::default();
        overlay
            .actions
            .insert(RemoteButton::Ok, ButtonActions::default());
        let bit = 1u64 << RemoteButton::Ok.ordinal();
        assert_eq!(
            effective_button_mask(&base, Some(&overlay)) & bit,
            bit,
            "方案里的不执行操作必须消费原始按键，而不是让原生键漏过去"
        );

        let mut disabled_base = base;
        disabled_base.enabled = false;
        assert_eq!(
            effective_button_mask(&disabled_base, Some(&overlay)) & bit,
            0,
            "基础总开关关闭时覆盖层不得单独吞键"
        );
    }

    #[test]
    fn overlay_keeps_buttons_not_listed_in_the_profile() {
        let mut base = mappings_with_single(RemoteButton::Ok, KeyCode::Enter);
        base.actions
            .insert(RemoteButton::Back, single_actions(KeyCode::Escape));
        let overlay = overlay_with(RemoteButton::Ok, KeyCode::Space);

        assert_eq!(
            effective_action(
                &base,
                Some(&overlay),
                RemoteButton::Back,
                ButtonTrigger::Single
            ),
            ButtonAction::Shortcut {
                chord: KeyChord {
                    keys: vec![KeyCode::Escape]
                }
            },
            "方案未列出的键继续走基础映射"
        );
        let merged = merged_mappings(&base, Some(&overlay));
        assert_eq!(merged.actions.len(), 2);
        assert_eq!(
            merged.action_for(RemoteButton::Ok, ButtonTrigger::Single),
            ButtonAction::Shortcut {
                chord: KeyChord {
                    keys: vec![KeyCode::Space]
                }
            }
        );
    }

    #[test]
    fn overlay_is_ignored_when_base_mappings_are_disabled() {
        let mut base = mappings_with_single(RemoteButton::Ok, KeyCode::Enter);
        base.enabled = false;
        let overlay = overlay_with(RemoteButton::Ok, KeyCode::Space);

        assert_eq!(
            effective_action(
                &base,
                Some(&overlay),
                RemoteButton::Ok,
                ButtonTrigger::Single
            ),
            ButtonAction::Shortcut {
                chord: KeyChord {
                    keys: vec![KeyCode::Enter]
                }
            },
            "基础映射总开关关闭时覆盖层不得单独生效"
        );
        let merged = merged_mappings(&base, Some(&overlay));
        assert!(!merged.enabled);
        assert_eq!(
            merged.action_for(RemoteButton::Ok, ButtonTrigger::Single),
            ButtonAction::Shortcut {
                chord: KeyChord {
                    keys: vec![KeyCode::Enter]
                }
            },
            "合并结果必须保留基础映射，供总开关恢复后继续使用"
        );
    }

    #[test]
    fn profile_overlay_replaces_runtime_shortcut() {
        let _gate_lock = crate::key_gate::lock_gate_tests();
        let gate = crate::key_gate::KeyGate::start();
        std::thread::sleep(Duration::from_millis(50));
        let injector = Arc::new(RecordingInjector::default());
        let (runtime, _fired) = spawn_phrase_engine(
            &injector,
            mappings_with_single(RemoteButton::Ok, KeyCode::Enter),
        );
        runtime.set_profile_overlay(Some(overlay_with(RemoteButton::Ok, KeyCode::Space)));
        let sender = runtime.sender();
        // 覆盖层消息与测试发送使用不同 Sender 克隆；等引擎先完成重配置，
        // 否则首个边沿可能仍按基础映射（Enter 被原生覆盖）执行。
        std::thread::sleep(Duration::from_millis(100));

        sender
            .send(EngineMessage::GateEdge(ButtonEdge {
                button: RemoteButton::Ok,
                is_pressed: true,
            }))
            .unwrap();
        sender
            .send(EngineMessage::GateEdge(ButtonEdge {
                button: RemoteButton::Ok,
                is_pressed: false,
            }))
            .unwrap();

        assert!(
            wait_until(
                || !injector.taps.lock().unwrap().is_empty(),
                Duration::from_millis(1000)
            ),
            "覆盖层单击必须在 1s 内注入"
        );
        assert_eq!(
            injector.taps.lock().unwrap().clone(),
            vec![KeyChord {
                keys: vec![KeyCode::Space]
            }],
            "运行时必须执行覆盖层动作，而不是基础映射"
        );
        drop(runtime);
        drop(gate);
    }
}
