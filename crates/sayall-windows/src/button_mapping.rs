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

use crate::button_gestures::GestureRecognizer;
use crate::key_gate;
use crate::raw_input::{
    ButtonEdge, ButtonStateMerger, RawInputSnapshot, RawKeyboardEvent, RemoteButton,
};
use crate::send_input::{
    native_key, ButtonAction, ButtonMappings, ButtonTrigger, HoldRepeat, KeyChord, KeyCode,
    MouseClickKind, MoveDirection, ScrollDirection,
};
use crate::UsageCounters;

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
    SetSuspended(bool),
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
        }
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
    ) -> Result<(), String> {
        let generation = self.focus_generation.load(Ordering::Acquire);
        if self.focus_closed.load(Ordering::Acquire) {
            return Err("应用正在退出".to_owned());
        }
        self.launch_app(target)?;
        if self.focus_closed.load(Ordering::Acquire)
            || self.focus_generation.load(Ordering::Acquire) != generation
        {
            return Err("聚焦请求已取消".to_owned());
        }
        let strategy = profile
            .map(|profile| profile.strategy)
            .unwrap_or(crate::focus::FocusStrategy::OpenOnly);
        match strategy {
            crate::focus::FocusStrategy::OpenOnly => Ok(()),
            crate::focus::FocusStrategy::AppShortcut => {
                let chord = profile
                    .and_then(|profile| profile.shortcut.clone())
                    .ok_or_else(|| "该应用未录入聚焦快捷键".to_owned())?;
                self.tap(&chord)
            }
            crate::focus::FocusStrategy::RecordedElement => {
                let recorded = profile
                    .and_then(|profile| profile.recorded.clone())
                    .ok_or_else(|| "该应用还没有学习过输入框".to_owned())?;
                self.with_focus_runner_generation(generation, |runner| {
                    runner.submit(crate::focus_service::FocusTask::LaunchThenFocus(
                        crate::focus::FocusChoice::Recorded(recorded),
                    ))
                })
                .flatten()
                .map(|_| ())
                .ok_or_else(|| "聚焦服务不可用".to_owned())
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

/// 按键映射引擎运行时。持有句柄即运行；线程在 `Shutdown` 或通道关闭时退出。
pub struct ButtonMappingRuntime {
    observation: Arc<ButtonObservation>,
    configuration: Mutex<InputConfiguration>,
    capture_bridge: Mutex<std::sync::Weak<crate::rc003_bridge::Rc003Bridge>>,
    capture_enabled: std::sync::atomic::AtomicBool,
    sender: Sender<EngineMessage>,
    receiver: Mutex<Option<Receiver<EngineMessage>>>,
    state: Arc<Mutex<EngineState>>,
    edge_callbacks: Arc<RwLock<Vec<ButtonEdgeCallback>>>,
    gesture_callbacks: Arc<RwLock<Vec<ButtonGestureCallback>>>,
    gesture_handler: Arc<RwLock<Option<GestureHandler>>>,
    /// 与引擎工作线程共享的注入器句柄（用于读取最近一次聚焦结果、触发测试聚焦）。
    injector: Arc<dyn MappingInjector>,
    worker: Option<JoinHandle<()>>,
}

#[derive(Default)]
struct InputConfiguration {
    mappings: ButtonMappings,
    profile_mappings: Option<ButtonMappings>,
    scene_mappings: ButtonMappings,
    model: crate::RemoteModel,
    connected: bool,
    enhanced: bool,
    captured_mask: u64,
}

impl InputConfiguration {
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
        let execution = self.apply_input_capabilities(
            self.profile_mappings
                .as_ref()
                .unwrap_or(&self.mappings)
                .clone(),
        );
        let scene = self.apply_input_capabilities(self.scene_mappings.clone());
        let recognition = merge_recognition_mappings(&execution, &scene);
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

        let runtime = Self {
            observation: Arc::clone(&observation),
            configuration: Mutex::new(InputConfiguration::default()),
            capture_bridge: Mutex::new(std::sync::Weak::new()),
            capture_enabled: std::sync::atomic::AtomicBool::new(false),
            sender,
            receiver: Mutex::new(Some(receiver)),
            state: Arc::clone(&state),
            edge_callbacks: Arc::clone(&edge_callbacks),
            gesture_callbacks: Arc::clone(&gesture_callbacks),
            gesture_handler: Arc::clone(&gesture_handler),
            injector: Arc::clone(&injector),
            worker: None,
        };

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
    pub(crate) fn set_capture_owned(&self, mask: u64) {
        let mut configuration = self.configuration.lock().unwrap_or_else(|p| p.into_inner());
        let mask = if configuration.connected && configuration.model == crate::RemoteModel::Rc003 {
            mask & (configuration
                .profile_mappings
                .as_ref()
                .unwrap_or(&configuration.mappings)
                .mapped_mask()
                | configuration.scene_mappings.mapped_mask())
        } else {
            0
        };
        if configuration.captured_mask == mask {
            return;
        }
        configuration.captured_mask = mask;
        configuration.enhanced = mask != 0;
        let (execution, recognition) = configuration.effective_mappings();
        let _ = self.sender.send(EngineMessage::MappingsChanged {
            execution,
            recognition,
            enhanced: mask != 0,
        });
        crate::ble::gatt_note(format!(
            "enhanced_capture event=engine_ownership mask={mask} available={}",
            mask != 0
        ));
    }

    #[cfg(test)]
    fn set_input_enhancement(&self, available: bool) {
        self.set_capture_owned(if available { (1 << 13) - 1 } else { 0 });
    }

    pub(crate) fn requested_capture_mask(&self) -> u64 {
        let c = self.configuration.lock().unwrap_or_else(|p| p.into_inner());
        if !c.connected || c.model != crate::RemoteModel::Rc003 {
            return 0;
        }
        let profile = c.profile_mappings.as_ref().unwrap_or(&c.mappings);
        profile.mapped_mask() | c.scene_mappings.mapped_mask()
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
        let profile = self.mappings().focus_profiles.get(target).cloned();
        self.injector.launch_app_and_focus(target, profile.as_ref())
    }

    /// UI「开始学习输入框」：阻塞至多 3 秒，返回捕获到的输入框特征。
    pub fn learn_focus_target(&self) -> Result<crate::focus::RecordedFocusTarget, String> {
        self.injector.learn_focus_target()
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
        (
            mappings.enabled,
            mappings.actions(RemoteButton::Ok).ok_context_click,
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
    let mut mapping_notice: Option<MappingNoticeCallback> = None;
    // 「按住连续触发」统计：起拍建条目，释放/复位时聚合落一行。
    let mut repeat_runs: RepeatRuns = BTreeMap::new();
    // OK 键上下文点击：最近一次由遥控器注入成功的鼠标移动时刻（5 秒窗口）。
    let mut last_remote_move_at: Option<Instant> = None;

    loop {
        let timeout = recognizer
            .next_deadline()
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
                            &mappings,
                            &recognition_mappings,
                            &state,
                            &gesture_callbacks,
                            &gesture_handler,
                            &injector,
                            &mut native_pending,
                            &mut last_remote_move_at,
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
                    continue;
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    // 引擎线程收尾：把仍在进行中的连续触发聚合日志补齐
                    // （2026-10-05 评审发现 2）。
                    finish_all_repeat_runs(&mut repeat_runs, "shutdown");
                    break;
                }
            },
            None => match receiver.recv() {
                Ok(message) => message,
                Err(_) => {
                    finish_all_repeat_runs(&mut repeat_runs, "shutdown");
                    break;
                }
            },
        };

        match message {
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
                    &mut last_remote_move_at,
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
                    &mut last_remote_move_at,
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
                    &mut last_remote_move_at,
                );
            }
            EngineMessage::ListenerStopped | EngineMessage::DeviceRemoved => {
                injector.cancel_focus();
                observation.clear();
                crate::ble::gatt_note(format!(
                    "map_reset source={}",
                    match message {
                        EngineMessage::ListenerStopped => "listener_stopped",
                        _ => "device_removed",
                    }
                ));
                // 释放全部按住状态：取消所有手势计时，不触发动作。
                recognizer.release_all();
                cancel_gesture_input(&merger, &mut gesture_input, &snapshot, &edge_callbacks);
                native_pending.clear();
                key_gate::cancel_pending_holds();
                finish_all_repeat_runs(&mut repeat_runs, "reset");
            }
            EngineMessage::MappingsChanged {
                execution,
                recognition,
                enhanced,
            } => {
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
                    mappings.enabled, configured
                ));
                if let Some(callback) = &mapping_notice {
                    (callback.0)(mappings.mapped_mask() != 0);
                }
            }
            EngineMessage::Shutdown => {
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
                }
                lock_state(&state).suspended = suspended;
                crate::ble::gatt_note(format!("map_suspension suspended={suspended}"));
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
    last_remote_move_at: &mut Option<Instant>,
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
    let press_count = edges.iter().filter(|edge| edge.is_pressed).count() as u64;
    usage.record_button_presses(press_count);
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
            native_pending.remove(&edge.button);
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
            // OK 上下文点击：命中窗口时立即点击光标位置，并吞掉本次按压
            // （单击/双击/长按都不执行）；注入失败则回落到正常路径。
            if !lock_state(state).suspended
                && !native_pending.contains(&edge.button)
                && read_lock(recognition_mappings)
                    .actions(edge.button)
                    .ok_context_click
                && ok_context_click_applies(&edge, &mappings, *last_remote_move_at, now, injector)
            {
                recognizer.swallow_press(edge.button);
                Vec::new()
            } else {
                recognizer.press(edge.button, now)
            }
        } else {
            recognizer.release(edge.button, now)
        };
        if !edge.is_pressed {
            // 含「按住连续触发」的一趟结束：落聚合日志（拍数/时长）。
            finish_repeat_run(repeat_runs, edge.button, "release");
        }
        for trigger in fired {
            if fire_gesture(
                edge.button,
                trigger,
                mappings,
                recognition_mappings,
                state,
                gesture_callbacks,
                gesture_handler,
                injector,
                native_pending,
                last_remote_move_at,
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

#[allow(clippy::too_many_arguments)]
fn fire_gesture(
    button: RemoteButton,
    trigger: ButtonTrigger,
    mappings: &Arc<RwLock<ButtonMappings>>,
    recognition_mappings: &Arc<RwLock<ButtonMappings>>,
    state: &Arc<Mutex<EngineState>>,
    gesture_callbacks: &Arc<RwLock<Vec<ButtonGestureCallback>>>,
    gesture_handler: &Arc<RwLock<Option<GestureHandler>>>,
    injector: &Arc<dyn MappingInjector>,
    native_pending: &mut BTreeSet<RemoteButton>,
    last_remote_move_at: &mut Option<Instant>,
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
            crate::ble::gatt_note(format!(
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
        crate::ble::gatt_note(format!(
            "map_route button={button:?} trigger={trigger:?} disposition={disposition:?} native_delivered={native_delivered}"
        ));
        if disposition != GestureDisposition::PassThrough {
            return false;
        }
    }
    let mappings = read_lock(mappings).clone();
    if !mappings.enabled {
        return false;
    }
    let action = mappings.action_for(button, trigger);
    if action == ButtonAction::Disabled {
        crate::ble::gatt_note(format!(
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
        ButtonAction::Scroll { direction, steps } => {
            crate::ble::gatt_note(format!(
                "map_fire button={button:?} trigger={trigger:?} action=scroll direction={direction:?} steps={steps}"
            ));
            if let Err(error) = injector.scroll(direction, steps) {
                lock_state(state).last_error = Some(format!("滚轮事件发送失败：{error}"));
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
            crate::ble::gatt_note(format!(
                "map_fire button={:?} trigger={:?} action=shortcut chord={}",
                button,
                trigger,
                chord
                    .keys
                    .iter()
                    .map(|key| format!("{key:?}"))
                    .collect::<Vec<_>>()
                    .join("+")
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
                        crate::ble::gatt_note(LOCK_WORKSTATION_INJECT_NOTE.to_owned());
                    } else {
                        let echo = observe_injection_echo(echo_before);
                        crate::ble::gatt_note(shortcut_injection_note(echo));
                    }
                    return terminal_action;
                }
                Err(error) => {
                    crate::ble::gatt_note("map_inject result=err error_domain=send_input error_code=injection_failed reason=backend_rejected retryable=true".to_owned());
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
            let profile = mappings.focus_profiles.get(&target).cloned();
            let strategy = profile
                .as_ref()
                .map(|profile| profile.strategy.as_str())
                .unwrap_or("none");
            crate::ble::gatt_note(format!(
                "map_fire button={:?} trigger={:?} action=open_app target_kind={target_kind} focus_strategy={strategy}",
                button, trigger,
            ));
            match injector.launch_app_and_focus(&target, profile.as_ref()) {
                Ok(()) => crate::ble::gatt_note(format!(
                    "map_launch result=ok target_kind={} focus_strategy={}",
                    target_kind, strategy
                )),
                Err(error) => {
                    crate::ble::gatt_note(format!(
                        "map_launch result=err target_kind={} focus_strategy={} error_domain=shell error_code=launch_failed reason=target_unavailable retryable=true",
                        target_kind, strategy
                    ));
                    lock_state(state).last_error = Some(format!("打开应用失败：{error}"));
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
    fn unified_capture_targets_include_profile_and_menu_without_common_actions() {
        let _gate_test = GATE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let runtime = ButtonMappingRuntime::new(
            Arc::new(RecordingInjector::default()),
            Arc::new(UsageCounters::default()),
            Arc::new(StdMutex::new(RawInputSnapshot::default())),
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
        runtime.set_capture_owned(1 << RemoteButton::Menu.ordinal());
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
        runtime.set_capture_owned(1 << RemoteButton::Ok.ordinal()); // stale target ACK
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
        runtime.set_capture_owned((1 << 13) - 1); // late callback after disconnect
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
            runtime.set_capture_owned(1 << RemoteButton::Ok.ordinal());
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
        fail: bool,
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
            RemoteButton::Up => 0x0052,
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
        // 给引擎线程一点时间处理消息。
        std::thread::sleep(Duration::from_millis(100));

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

    /// 打开应用带聚焦档案：分发侧必须把档案一起交给注入器（策略在档案里）。
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
        mappings.focus_profiles.insert(
            "notepad".to_owned(),
            crate::focus::AppFocusProfile {
                strategy: crate::focus::FocusStrategy::RecordedElement,
                shortcut: None,
                recorded: Some(crate::focus::RecordedFocusTarget {
                    control_type: "Edit".to_owned(),
                    automation_id: "chat-input".to_owned(),
                    ..Default::default()
                }),
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
                strategy: "recorded_element",
            }],
            "带档案的打开应用必须把策略交给注入器"
        );
        assert_eq!(
            injector.launches.lock().unwrap().as_slice(),
            &["notepad".to_owned()]
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
            injector.launch_app_and_focus("never-launch-during-exit", None),
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
}
