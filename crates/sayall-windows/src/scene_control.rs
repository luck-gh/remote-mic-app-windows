//! Foreground-aware semantic scene runtime.
//!
//! The input thread only classifies a gesture and performs a bounded enqueue.
//! UI Automation and process activation run serially on the action worker.

use crate::application_control::{
    ActionOutcome, ActionResult, ApplicationControlBackend, ApplicationControlError,
    ApplicationController, CapabilitySnapshot, CapabilityState, FocusRegion, WindowToken,
};
use crate::button_mapping::{FiredGesture, GestureDisposition, RoutedGesture};
use crate::raw_input::{ButtonEdge, RemoteButton, ALL_BUTTONS};
use crate::send_input::{
    ButtonAction, ButtonActions, ButtonMappings, ButtonTrigger, KeyChord, KeyCode,
};
use crate::templates::{
    AdjustmentMode, ApplicationBinding, ControlRegion, MappingConfiguration, MappingTemplate,
    SemanticAction,
};
use serde::{Deserialize, Serialize};
#[cfg(test)]
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::sync::{Arc, Mutex, RwLock, Weak};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

const ACTION_QUEUE_CAPACITY: usize = 32;
const LAUNCH_CONFIRM_INTERVAL: Duration = Duration::from_millis(50);

fn launch_confirm_timeout() -> Duration {
    if cfg!(test) {
        Duration::from_millis(75)
    } else {
        Duration::from_secs(2)
    }
}

static VOICE_SCENE: std::sync::OnceLock<Mutex<Weak<SceneController>>> = std::sync::OnceLock::new();

pub(crate) fn register_voice_scene(scene: &Arc<SceneController>) {
    *lock(VOICE_SCENE.get_or_init(|| Mutex::new(Weak::new()))) = Arc::downgrade(scene);
}

pub(crate) fn notify_voice_activity(active: bool) {
    let scene = VOICE_SCENE.get().and_then(|slot| lock(slot).upgrade());
    if let Some(scene) = scene {
        scene.notify_voice_active(active);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScenePanel {
    Application,
    Adjustment,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneMenuItem {
    pub application_id: Option<String>,
    pub template_id: String,
    pub label: String,
    pub running: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneSnapshot {
    pub enabled: bool,
    pub generation: u64,
    pub foreground_generation: u64,
    pub application_id: Option<String>,
    pub template_id: Option<String>,
    pub focus_region: FocusRegion,
    pub control_region: Option<ControlRegion>,
    pub adjustment_mode: Option<AdjustmentMode>,
    pub panel: Option<ScenePanel>,
    pub selected_index: Option<usize>,
    pub menu_items: Vec<SceneMenuItem>,
    pub waiting_for_release: bool,
    pub voice_active: bool,
    pub last_action: Option<SemanticAction>,
    pub last_result: Option<ActionResult>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SceneEvent {
    Snapshot {
        snapshot: SceneSnapshot,
    },
    ActionCompleted {
        outcome: ActionOutcome,
    },
    LaunchFailed {
        application_id: String,
        generation: u64,
        reason: String,
    },
    AdjustmentModePersistenceRequested {
        template_id: String,
        mode: AdjustmentMode,
        generation: u64,
    },
}

pub type SceneEventCallback = Arc<dyn Fn(SceneEvent) + Send + Sync>;

pub trait ApplicationLauncherBackend: Send + Sync {
    fn activate_or_launch(&self, target: &str) -> Result<(), String>;
}

#[derive(Debug)]
struct SystemApplicationLauncher;

impl ApplicationLauncherBackend for SystemApplicationLauncher {
    fn activate_or_launch(&self, target: &str) -> Result<(), String> {
        crate::app_launcher::activate_or_launch(target)
    }
}

#[derive(Debug, Clone)]
struct PanelState {
    kind: ScenePanel,
    selected: usize,
}

struct State {
    configuration: MappingConfiguration,
    generation: u64,
    foreground_generation: u64,
    token: Option<WindowToken>,
    focus: FocusRegion,
    capabilities: Option<CapabilitySnapshot>,
    panel: Option<PanelState>,
    held: BTreeSet<RemoteButton>,
    handled_in_cycle: BTreeSet<RemoteButton>,
    waiting_for_release: bool,
    voice_active: bool,
    defer_foreground_refresh: bool,
    last_action: Option<SemanticAction>,
    last_result: Option<ActionResult>,
    status: Option<String>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            configuration: MappingConfiguration {
                common_mappings: ButtonMappings::default(),
                template_control_enabled: false,
                templates: Vec::new(),
                application_bindings: Vec::new(),
            },
            generation: 1,
            foreground_generation: 0,
            token: None,
            focus: FocusRegion::Unknown,
            capabilities: None,
            panel: None,
            held: BTreeSet::new(),
            handled_in_cycle: BTreeSet::new(),
            waiting_for_release: false,
            voice_active: false,
            defer_foreground_refresh: false,
            last_action: None,
            last_result: None,
            status: None,
        }
    }
}

enum Work {
    RefreshForeground,
    Perform {
        generation: u64,
        token: WindowToken,
        action: SemanticAction,
    },
    Launch {
        generation: u64,
        binding: ApplicationBinding,
    },
    RestoreForeground {
        token: WindowToken,
        reply: SyncSender<Result<(), String>>,
    },
    Shutdown,
}

pub struct SceneController {
    state: Arc<Mutex<State>>,
    sender: SyncSender<Work>,
    callbacks: Arc<RwLock<Vec<SceneEventCallback>>>,
    worker: Mutex<Option<JoinHandle<()>>>,
    #[cfg(windows)]
    foreground_watcher: Mutex<Option<ForegroundWatcher>>,
}

impl SceneController {
    pub fn new() -> Arc<Self> {
        Self::with_backends(
            Arc::new(ApplicationController::new()),
            Arc::new(SystemApplicationLauncher),
            true,
        )
    }

    fn with_backends(
        application: Arc<dyn ApplicationControlBackend>,
        launcher: Arc<dyn ApplicationLauncherBackend>,
        watch_foreground: bool,
    ) -> Arc<Self> {
        let state = Arc::new(Mutex::new(State::default()));
        let callbacks = Arc::new(RwLock::new(Vec::new()));
        let (sender, receiver) = mpsc::sync_channel(ACTION_QUEUE_CAPACITY);
        let worker_state = Arc::clone(&state);
        let worker_callbacks = Arc::clone(&callbacks);
        let worker = std::thread::Builder::new()
            .name("sayall-scene-actions".to_owned())
            .spawn(move || {
                action_worker(
                    receiver,
                    worker_state,
                    worker_callbacks,
                    application,
                    launcher,
                )
            })
            .ok();
        let controller = Arc::new(Self {
            state,
            sender,
            callbacks,
            worker: Mutex::new(worker),
            #[cfg(windows)]
            foreground_watcher: Mutex::new(None),
        });
        #[cfg(windows)]
        if watch_foreground {
            *lock(&controller.foreground_watcher) =
                ForegroundWatcher::start(controller.sender.clone());
        }
        #[cfg(not(windows))]
        let _ = watch_foreground;
        controller.refresh_foreground();
        controller
    }

    pub fn set_configuration(&self, configuration: MappingConfiguration) -> ButtonMappings {
        let recognition = scene_recognition_mappings(&configuration);
        {
            let mut state = lock(&self.state);
            state.configuration = configuration;
            cancel_state(&mut state, "configuration_changed");
        }
        self.emit_snapshot();
        self.refresh_foreground();
        recognition
    }

    pub fn snapshot(&self) -> SceneSnapshot {
        snapshot_from(&lock(&self.state))
    }

    pub fn subscribe(&self, callback: SceneEventCallback) {
        self.callbacks
            .write()
            .unwrap_or_else(|p| p.into_inner())
            .push(callback);
    }

    pub fn refresh_foreground(&self) {
        #[cfg(windows)]
        if own_process_is_foreground() {
            return;
        }
        self.try_enqueue(Work::RefreshForeground, "foreground_queue_full");
    }

    /// Restore the captured target before an optional mouse-driven overlay
    /// command. The overlay itself should use a non-activating window style.
    pub fn restore_target_foreground(&self) -> Result<(), ApplicationControlError> {
        let token = lock(&self.state)
            .token
            .clone()
            .ok_or(ApplicationControlError::NoForegroundWindow)?;
        let (reply, receiver) = mpsc::sync_channel(1);
        self.sender
            .try_send(Work::RestoreForeground { token, reply })
            .map_err(|_| ApplicationControlError::AutomationUnavailable)?;
        receiver
            .recv_timeout(Duration::from_secs(1))
            .map_err(|_| ApplicationControlError::AutomationUnavailable)?
            .map_err(|_| ApplicationControlError::AutomationUnavailable)
    }

    pub fn notify_voice_active(&self, active: bool) {
        let changed = {
            let mut state = lock(&self.state);
            if state.voice_active == active {
                false
            } else {
                state.voice_active = active;
                if active {
                    cancel_state(&mut state, "voice_active");
                }
                true
            }
        };
        if changed {
            crate::ble::gatt_note(format!("scene_voice active={active} panel_closed={active}"));
            self.emit_snapshot();
            if !active {
                self.refresh_foreground();
            }
        }
    }

    pub fn handle_edge(&self, edge: ButtonEdge) {
        let mut emit = false;
        {
            let mut state = lock(&self.state);
            if edge.is_pressed {
                state.held.insert(edge.button);
                state.handled_in_cycle.remove(&edge.button);
            } else {
                state.held.remove(&edge.button);
                if state.held.is_empty() && state.waiting_for_release {
                    state.waiting_for_release = false;
                    state.status = None;
                    emit = true;
                }
            }
        }
        if emit {
            self.emit_snapshot();
        }
    }

    pub fn handle_gesture(&self, routed: RoutedGesture) -> GestureDisposition {
        let mut work = None;
        let mut persistence = None;
        let disposition;
        {
            let mut state = lock(&self.state);
            if !state.configuration.template_control_enabled || state.token.is_none() {
                return GestureDisposition::PassThrough;
            }
            if state.voice_active || state.waiting_for_release {
                return GestureDisposition::Blocked;
            }
            let app_id = state
                .token
                .as_ref()
                .map(|token| token.application_id().to_owned());
            let Some(binding) = app_id
                .as_deref()
                .and_then(|id| binding_for(&state.configuration, id).cloned())
            else {
                return GestureDisposition::PassThrough;
            };
            if routed.native_delivered {
                state.status = Some("input_not_exclusively_captured".to_owned());
                disposition = GestureDisposition::Blocked;
            } else if let Some(panel) = state.panel.clone() {
                disposition = route_panel(
                    &mut state,
                    panel,
                    routed.gesture,
                    &mut work,
                    &mut persistence,
                );
            } else {
                disposition = route_template(&mut state, &binding, routed.gesture, &mut work);
            }
        }
        if let Some(work) = work {
            if !self.try_enqueue(work, "action_queue_full") {
                let mut state = lock(&self.state);
                state.waiting_for_release = true;
                state.panel = None;
                state.generation = state.generation.saturating_add(1);
            }
        }
        if let Some(event) = persistence {
            emit(&self.callbacks, event);
        }
        self.emit_snapshot();
        disposition
    }

    fn try_enqueue(&self, work: Work, reason: &str) -> bool {
        match self.sender.try_send(work) {
            Ok(()) => true,
            Err(TrySendError::Full(_)) => {
                let mut state = lock(&self.state);
                state.status = Some(reason.to_owned());
                crate::ble::gatt_note(format!("scene_enqueue result=blocked reason={reason}"));
                false
            }
            Err(TrySendError::Disconnected(_)) => {
                lock(&self.state).status = Some("action_worker_stopped".to_owned());
                false
            }
        }
    }

    fn emit_snapshot(&self) {
        emit(
            &self.callbacks,
            SceneEvent::Snapshot {
                snapshot: self.snapshot(),
            },
        );
    }
}

impl Drop for SceneController {
    fn drop(&mut self) {
        #[cfg(windows)]
        lock(&self.foreground_watcher).take();
        let _ = self.sender.send(Work::Shutdown);
        if let Some(worker) = lock(&self.worker).take() {
            let _ = worker.join();
        }
    }
}

fn route_template(
    state: &mut State,
    binding: &ApplicationBinding,
    gesture: FiredGesture,
    work: &mut Option<Work>,
) -> GestureDisposition {
    if gesture.button == RemoteButton::Menu {
        if !state.handled_in_cycle.insert(gesture.button) {
            state.status = Some("duplicate_gesture_blocked".to_owned());
            return GestureDisposition::Blocked;
        }
        match gesture.trigger {
            ButtonTrigger::Single => open_panel(state, ScenePanel::Application),
            ButtonTrigger::Long => open_panel(state, ScenePanel::Adjustment),
            ButtonTrigger::Double => return GestureDisposition::Blocked,
        }
        return GestureDisposition::Handled;
    }
    let Some(template) = template_for(&state.configuration, &binding.template_id) else {
        state.status = Some("template_missing".to_owned());
        return GestureDisposition::Blocked;
    };
    let action = adjustment_action(template.adjustment_mode, gesture.button).unwrap_or_else(|| {
        if state.focus == FocusRegion::Modal
            && gesture.button == RemoteButton::Back
            && gesture.trigger == ButtonTrigger::Single
        {
            SemanticAction::Escape
        } else {
            action_for(template, state.focus, gesture)
        }
    });
    if one_shot_action(&action) && !state.handled_in_cycle.insert(gesture.button) {
        state.status = Some("duplicate_gesture_blocked".to_owned());
        return GestureDisposition::Blocked;
    }
    match action {
        SemanticAction::OpenApplicationMenu => {
            open_panel(state, ScenePanel::Application);
            return GestureDisposition::Handled;
        }
        SemanticAction::OpenAdjustmentMenu => {
            open_panel(state, ScenePanel::Adjustment);
            return GestureDisposition::Handled;
        }
        _ => {}
    }
    if action == SemanticAction::Disabled {
        state.status = Some("semantic_action_disabled".to_owned());
        return GestureDisposition::Blocked;
    }
    let Some(token) = state.token.clone() else {
        return GestureDisposition::Blocked;
    };
    if !capability_available(state.capabilities.as_ref(), &action) {
        state.last_action = Some(action);
        state.last_result = Some(ActionResult::Unavailable);
        state.status = Some("semantic_action_unavailable".to_owned());
        return GestureDisposition::Blocked;
    }
    state.last_action = Some(action.clone());
    *work = Some(Work::Perform {
        generation: state.generation,
        token,
        action,
    });
    GestureDisposition::Handled
}

fn route_panel(
    state: &mut State,
    mut panel: PanelState,
    gesture: FiredGesture,
    work: &mut Option<Work>,
    persistence: &mut Option<SceneEvent>,
) -> GestureDisposition {
    if gesture.trigger != ButtonTrigger::Single {
        return GestureDisposition::Blocked;
    }
    let count = menu_items(state, panel.kind).len();
    match gesture.button {
        RemoteButton::Up | RemoteButton::Left => panel.selected = panel.selected.saturating_sub(1),
        RemoteButton::Down | RemoteButton::Right => {
            panel.selected = (panel.selected + 1).min(count.saturating_sub(1))
        }
        RemoteButton::Back | RemoteButton::Power | RemoteButton::Menu => {
            if !state.handled_in_cycle.insert(gesture.button) {
                return GestureDisposition::Blocked;
            }
            state.panel = None;
            if state.defer_foreground_refresh {
                state.defer_foreground_refresh = false;
                state.generation = state.generation.saturating_add(1);
                if !state.held.is_empty() {
                    state.waiting_for_release = true;
                }
                *work = Some(Work::RefreshForeground);
            }
            state.status = Some("menu_cancelled".to_owned());
            return GestureDisposition::Handled;
        }
        RemoteButton::Ok => match panel.kind {
            ScenePanel::Application => {
                if !state.handled_in_cycle.insert(gesture.button) {
                    return GestureDisposition::Blocked;
                }
                let bindings = sorted_bindings(&state.configuration);
                if let Some(mut binding) =
                    bindings.get(panel.selected).map(|value| (**value).clone())
                {
                    if binding.launch_target.is_none() {
                        binding.launch_target = Some(binding.application_id.clone());
                    }
                    *work = Some(Work::Launch {
                        generation: state.generation,
                        binding,
                    });
                    state.status = Some("launch_confirming_foreground".to_owned());
                } else {
                    state.status = Some("menu_selection_unavailable".to_owned());
                }
                return GestureDisposition::Handled;
            }
            ScenePanel::Adjustment => {
                if !state.handled_in_cycle.insert(gesture.button) {
                    return GestureDisposition::Blocked;
                }
                let modes = [
                    AdjustmentMode::Volume,
                    AdjustmentMode::Page,
                    AdjustmentMode::Zoom,
                ];
                if let Some(mode) = modes.get(panel.selected).copied() {
                    if let Some(template_id) = active_template_id(state) {
                        state.panel = None;
                        state.status = Some("configuration_persistence_requested".to_owned());
                        *persistence = Some(SceneEvent::AdjustmentModePersistenceRequested {
                            template_id,
                            mode,
                            generation: state.generation,
                        });
                    }
                }
                return GestureDisposition::Handled;
            }
        },
        _ => return GestureDisposition::Blocked,
    }
    state.panel = Some(panel);
    GestureDisposition::Handled
}

fn open_panel(state: &mut State, kind: ScenePanel) {
    let count = menu_items(state, kind).len();
    if count == 0 {
        state.status = Some("menu_empty".to_owned());
        return;
    }
    state.panel = Some(PanelState { kind, selected: 0 });
    state.status = None;
}

fn action_worker(
    receiver: Receiver<Work>,
    state: Arc<Mutex<State>>,
    callbacks: Arc<RwLock<Vec<SceneEventCallback>>>,
    application: Arc<dyn ApplicationControlBackend>,
    launcher: Arc<dyn ApplicationLauncherBackend>,
) {
    while let Ok(work) = receiver.recv() {
        match work {
            Work::Shutdown => break,
            Work::RefreshForeground => {
                refresh_foreground_state(&state, &callbacks, application.as_ref())
            }
            Work::Perform {
                generation,
                token,
                action,
            } => {
                if lock(&state).generation != generation {
                    continue;
                }
                let outcome = application.perform(&token, action);
                {
                    let mut state = lock(&state);
                    if state.generation != generation {
                        continue;
                    }
                    state.last_action = Some(outcome.action.clone());
                    state.last_result = Some(outcome.result);
                    state.status = outcome.reason.map(|reason| format!("{reason:?}"));
                }
                crate::ble::gatt_note(format!(
                    "scene_action generation={generation} result={:?}",
                    outcome.result
                ));
                emit(&callbacks, SceneEvent::ActionCompleted { outcome });
                emit_snapshot_for(&state, &callbacks);
            }
            Work::Launch {
                generation,
                binding,
            } => {
                if lock(&state).generation != generation {
                    continue;
                }
                let target = binding
                    .launch_target
                    .as_deref()
                    .unwrap_or(&binding.application_id);
                let result = launcher.activate_or_launch(target);
                let deadline = Instant::now() + launch_confirm_timeout();
                let mut confirmed = None;
                if result.is_ok() {
                    while Instant::now() < deadline {
                        if lock(&state).generation != generation {
                            break;
                        }
                        if let Ok(token) = application.identify_foreground() {
                            if token.application_id() == binding.application_id {
                                confirmed = Some(token);
                                break;
                            }
                        }
                        std::thread::sleep(LAUNCH_CONFIRM_INTERVAL);
                    }
                }
                let mut failure = None;
                {
                    let mut state = lock(&state);
                    if state.generation != generation {
                        continue;
                    }
                    if let Some(token) = confirmed {
                        state.token = Some(token.clone());
                        state.foreground_generation = token.generation();
                        state.panel = None;
                        state.defer_foreground_refresh = false;
                        state.status = Some("launch_confirmed".to_owned());
                    } else {
                        let reason = if result.is_err() {
                            "launch_failed"
                        } else {
                            "launch_foreground_timeout"
                        };
                        state.defer_foreground_refresh = true;
                        state.status = Some(reason.to_owned());
                        failure = Some(SceneEvent::LaunchFailed {
                            application_id: binding.application_id.clone(),
                            generation,
                            reason: reason.to_owned(),
                        });
                    }
                }
                if let Some(event) = failure {
                    emit(&callbacks, event);
                    emit_snapshot_for(&state, &callbacks);
                } else {
                    refresh_foreground_state(&state, &callbacks, application.as_ref());
                }
            }
            Work::RestoreForeground { token, reply } => {
                let result = application
                    .restore_foreground(&token)
                    .map_err(|error| error.to_string());
                let _ = reply.try_send(result);
            }
        }
    }
}

fn refresh_foreground_state(
    state: &Arc<Mutex<State>>,
    callbacks: &Arc<RwLock<Vec<SceneEventCallback>>>,
    application: &dyn ApplicationControlBackend,
) {
    if {
        let state = lock(state);
        state.defer_foreground_refresh
            && state
                .panel
                .as_ref()
                .is_some_and(|panel| panel.kind == ScenePanel::Application)
    } {
        crate::ble::gatt_note(
            "scene_foreground result=deferred reason=unconfirmed_application_switch".to_owned(),
        );
        return;
    }
    let observed = application.identify_foreground().and_then(|token| {
        let focus = application.classify_focus(&token)?;
        let capabilities = application.capabilities(&token)?;
        Ok((token, focus.region, capabilities))
    });
    {
        let mut state = lock(state);
        match observed {
            Ok((token, focus, capabilities)) => {
                let changed = state
                    .token
                    .as_ref()
                    .is_none_or(|current| current.generation() != token.generation());
                if changed {
                    cancel_state(&mut state, "foreground_changed");
                }
                state.foreground_generation = token.generation();
                state.token = Some(token);
                state.focus = focus;
                state.capabilities = Some(capabilities);
            }
            Err(error) => {
                if state.token.take().is_some() {
                    cancel_state(&mut state, "foreground_unavailable");
                }
                state.focus = FocusRegion::Unknown;
                state.capabilities = None;
                state.status = Some(
                    match error {
                        ApplicationControlError::UnsupportedPlatform => "platform_unsupported",
                        _ => "foreground_unavailable",
                    }
                    .to_owned(),
                );
            }
        }
    }
    emit_snapshot_for(state, callbacks);
}

fn cancel_state(state: &mut State, reason: &str) {
    state.generation = state.generation.saturating_add(1).max(1);
    state.panel = None;
    state.defer_foreground_refresh = false;
    state.capabilities = None;
    state.handled_in_cycle.clear();
    if !state.held.is_empty() {
        state.waiting_for_release = true;
    }
    state.status = Some(reason.to_owned());
}

fn capability_available(
    capabilities: Option<&CapabilitySnapshot>,
    action: &SemanticAction,
) -> bool {
    capabilities.is_some_and(|snapshot| {
        snapshot.actions.iter().any(|capability| {
            capability.action == *action && capability.state == CapabilityState::Available
        })
    })
}

fn action_for(
    template: &MappingTemplate,
    focus: FocusRegion,
    gesture: FiredGesture,
) -> SemanticAction {
    let region = match focus {
        FocusRegion::ApplicationList => ControlRegion::ApplicationList,
        FocusRegion::Content => ControlRegion::Content,
        FocusRegion::Input | FocusRegion::ImeCandidate => ControlRegion::Input,
        FocusRegion::Modal | FocusRegion::Unknown => return SemanticAction::Disabled,
    };
    let Some(actions) = template
        .region_actions
        .get(&region)
        .and_then(|buttons| buttons.get(&gesture.button))
    else {
        return SemanticAction::Disabled;
    };
    match gesture.trigger {
        ButtonTrigger::Single => actions.single.clone(),
        ButtonTrigger::Double => actions.double.clone(),
        ButtonTrigger::Long => actions.long.clone(),
    }
}

fn adjustment_action(mode: AdjustmentMode, button: RemoteButton) -> Option<SemanticAction> {
    match (mode, button) {
        (AdjustmentMode::Volume, RemoteButton::VolumeUp) => Some(SemanticAction::VolumeUp),
        (AdjustmentMode::Volume, RemoteButton::VolumeDown) => Some(SemanticAction::VolumeDown),
        (AdjustmentMode::Page, RemoteButton::VolumeUp) => Some(SemanticAction::PageUp),
        (AdjustmentMode::Page, RemoteButton::VolumeDown) => Some(SemanticAction::PageDown),
        (AdjustmentMode::Zoom, RemoteButton::VolumeUp) => Some(SemanticAction::ZoomIn),
        (AdjustmentMode::Zoom, RemoteButton::VolumeDown) => Some(SemanticAction::ZoomOut),
        _ => None,
    }
}

fn one_shot_action(action: &SemanticAction) -> bool {
    matches!(
        action,
        SemanticAction::Send
            | SemanticAction::ActivateSelection
            | SemanticAction::PreviousTab
            | SemanticAction::NextTab
            | SemanticAction::OpenApplicationMenu
            | SemanticAction::OpenAdjustmentMenu
    )
}

fn scene_recognition_mappings(configuration: &MappingConfiguration) -> ButtonMappings {
    let mut mappings = ButtonMappings::default();
    mappings.enabled = configuration.template_control_enabled;
    if !mappings.enabled {
        return mappings;
    }
    for button in ALL_BUTTONS {
        let mut actions = ButtonActions::default();
        if button == RemoteButton::Menu {
            actions.single = recognition_marker();
            actions.long = recognition_marker();
        }
        for template in &configuration.templates {
            for region in template.region_actions.values() {
                if let Some(semantic) = region.get(&button) {
                    if semantic.single != SemanticAction::Disabled {
                        actions.single = recognition_marker();
                    }
                    if semantic.double != SemanticAction::Disabled {
                        actions.double = recognition_marker();
                    }
                    if semantic.long != SemanticAction::Disabled {
                        actions.long = recognition_marker();
                    }
                }
            }
        }
        if matches!(button, RemoteButton::VolumeUp | RemoteButton::VolumeDown) {
            actions.single = recognition_marker();
        }
        if actions != ButtonActions::default() {
            mappings.actions.insert(button, actions);
        }
    }
    mappings
}

fn recognition_marker() -> ButtonAction {
    ButtonAction::Shortcut {
        chord: KeyChord {
            keys: vec![KeyCode::Escape],
        },
    }
}

fn binding_for<'a>(
    configuration: &'a MappingConfiguration,
    id: &str,
) -> Option<&'a ApplicationBinding> {
    configuration
        .application_bindings
        .iter()
        .find(|binding| binding.application_id == id)
}

fn template_for<'a>(
    configuration: &'a MappingConfiguration,
    id: &str,
) -> Option<&'a MappingTemplate> {
    configuration
        .templates
        .iter()
        .find(|template| template.id == id)
}

fn active_template_id(state: &State) -> Option<String> {
    let app = state.token.as_ref()?.application_id();
    Some(binding_for(&state.configuration, app)?.template_id.clone())
}

fn sorted_bindings(configuration: &MappingConfiguration) -> Vec<&ApplicationBinding> {
    let mut values: Vec<_> = configuration.application_bindings.iter().collect();
    values.sort_by_key(|binding| (binding.menu_order, binding.application_id.as_str()));
    values
}

fn menu_items(state: &State, kind: ScenePanel) -> Vec<SceneMenuItem> {
    match kind {
        ScenePanel::Application => sorted_bindings(&state.configuration)
            .into_iter()
            .map(|binding| SceneMenuItem {
                application_id: Some(binding.application_id.clone()),
                template_id: binding.template_id.clone(),
                label: binding.application_id.clone(),
                running: state
                    .token
                    .as_ref()
                    .is_some_and(|token| token.application_id() == binding.application_id),
            })
            .collect(),
        ScenePanel::Adjustment => [
            AdjustmentMode::Volume,
            AdjustmentMode::Page,
            AdjustmentMode::Zoom,
        ]
        .into_iter()
        .map(|mode| SceneMenuItem {
            application_id: None,
            template_id: format!("{mode:?}").to_lowercase(),
            label: format!("{mode:?}"),
            running: active_template_id(state)
                .and_then(|id| template_for(&state.configuration, &id))
                .is_some_and(|template| template.adjustment_mode == mode),
        })
        .collect(),
    }
}

fn snapshot_from(state: &State) -> SceneSnapshot {
    let active_template = active_template_id(state);
    let mode = active_template
        .as_deref()
        .and_then(|id| template_for(&state.configuration, id))
        .map(|template| template.adjustment_mode);
    SceneSnapshot {
        enabled: state.configuration.template_control_enabled,
        generation: state.generation,
        foreground_generation: state.foreground_generation,
        application_id: state
            .token
            .as_ref()
            .map(|token| token.application_id().to_owned()),
        template_id: active_template,
        focus_region: state.focus,
        control_region: state.focus.control_region(),
        adjustment_mode: mode,
        panel: state.panel.as_ref().map(|panel| panel.kind),
        selected_index: state.panel.as_ref().map(|panel| panel.selected),
        menu_items: state
            .panel
            .as_ref()
            .map_or_else(Vec::new, |panel| menu_items(state, panel.kind)),
        waiting_for_release: state.waiting_for_release,
        voice_active: state.voice_active,
        last_action: state.last_action.clone(),
        last_result: state.last_result,
        status: state.status.clone(),
    }
}

fn emit(callbacks: &Arc<RwLock<Vec<SceneEventCallback>>>, event: SceneEvent) {
    let callbacks = callbacks.read().unwrap_or_else(|p| p.into_inner()).clone();
    for callback in callbacks {
        callback(event.clone());
    }
}

fn emit_snapshot_for(state: &Arc<Mutex<State>>, callbacks: &Arc<RwLock<Vec<SceneEventCallback>>>) {
    emit(
        callbacks,
        SceneEvent::Snapshot {
            snapshot: snapshot_from(&lock(state)),
        },
    );
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|p| p.into_inner())
}

#[cfg(windows)]
static FOREGROUND_SENDER: std::sync::OnceLock<Mutex<Option<SyncSender<Work>>>> =
    std::sync::OnceLock::new();

#[cfg(windows)]
struct ForegroundWatcher {
    thread_id: u32,
    worker: Option<JoinHandle<()>>,
}

#[cfg(windows)]
impl ForegroundWatcher {
    fn start(sender: SyncSender<Work>) -> Option<Self> {
        use std::sync::mpsc::channel;
        let slot = FOREGROUND_SENDER.get_or_init(|| Mutex::new(None));
        if lock(slot).is_some() {
            return None;
        }
        *lock(slot) = Some(sender);
        let (ready_tx, ready_rx) = channel();
        let worker = std::thread::Builder::new()
            .name("sayall-foreground-hook".to_owned())
            .spawn(move || {
                use windows::Win32::Foundation::HMODULE;
                use windows::Win32::System::Threading::GetCurrentThreadId;
                use windows::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent};
                use windows::Win32::UI::WindowsAndMessaging::{
                    DispatchMessageW, GetMessageW, TranslateMessage, EVENT_SYSTEM_FOREGROUND, MSG,
                    WINEVENT_OUTOFCONTEXT,
                };
                let thread_id = unsafe { GetCurrentThreadId() };
                let hook = unsafe {
                    SetWinEventHook(
                        EVENT_SYSTEM_FOREGROUND,
                        EVENT_SYSTEM_FOREGROUND,
                        Some(HMODULE::default()),
                        Some(foreground_callback),
                        0,
                        0,
                        WINEVENT_OUTOFCONTEXT,
                    )
                };
                let _ = ready_tx.send((thread_id, !hook.is_invalid()));
                if hook.is_invalid() {
                    return;
                }
                let mut message = MSG::default();
                while unsafe { GetMessageW(&mut message, None, 0, 0) }.as_bool() {
                    unsafe {
                        let _ = TranslateMessage(&message);
                        DispatchMessageW(&message);
                    }
                }
                unsafe {
                    let _ = UnhookWinEvent(hook);
                }
            })
            .ok()?;
        let (thread_id, ready) = ready_rx.recv_timeout(Duration::from_secs(1)).ok()?;
        if !ready {
            *lock(slot) = None;
            return None;
        }
        Some(Self {
            thread_id,
            worker: Some(worker),
        })
    }
}

#[cfg(windows)]
unsafe extern "system" fn foreground_callback(
    _hook: windows::Win32::UI::Accessibility::HWINEVENTHOOK,
    _event: u32,
    _hwnd: windows::Win32::Foundation::HWND,
    _object: i32,
    _child: i32,
    _thread: u32,
    _time: u32,
) {
    if _hwnd.0.is_null() {
        return;
    }
    let mut process_id = 0u32;
    unsafe {
        windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(
            _hwnd,
            Some(&mut process_id),
        );
    }
    if process_id == std::process::id() {
        return;
    }
    if let Some(slot) = FOREGROUND_SENDER.get() {
        if let Some(sender) = lock(slot).as_ref() {
            let _ = sender.try_send(Work::RefreshForeground);
        }
    }
}

#[cfg(windows)]
fn own_process_is_foreground() -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.0.is_null() {
        return false;
    }
    let mut process_id = 0u32;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut process_id));
    }
    process_id == std::process::id()
}

#[cfg(windows)]
impl Drop for ForegroundWatcher {
    fn drop(&mut self) {
        use windows::Win32::UI::WindowsAndMessaging::{PostThreadMessageW, WM_QUIT};
        if let Some(slot) = FOREGROUND_SENDER.get() {
            *lock(slot) = None;
        }
        unsafe {
            let _ = PostThreadMessageW(
                self.thread_id,
                WM_QUIT,
                Default::default(),
                Default::default(),
            );
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application_control::{
        ActionCapability, ApplicationAdapterKind, CapabilityReason, FocusSnapshot,
    };
    use crate::templates::SemanticButtonActions;

    struct FakeApplication {
        token: Mutex<Option<WindowToken>>,
        focus: Mutex<FocusRegion>,
        available: Mutex<Vec<SemanticAction>>,
        performed: Mutex<Vec<SemanticAction>>,
    }

    impl FakeApplication {
        fn new(application_id: &str, generation: u64) -> Self {
            Self {
                token: Mutex::new(Some(WindowToken::from_identity(
                    application_id.to_owned(),
                    ApplicationAdapterKind::Generic,
                    1,
                    generation,
                    generation,
                ))),
                focus: Mutex::new(FocusRegion::Input),
                available: Mutex::new(vec![SemanticAction::Send]),
                performed: Mutex::new(Vec::new()),
            }
        }
    }

    impl ApplicationControlBackend for FakeApplication {
        fn identify_foreground(&self) -> Result<WindowToken, ApplicationControlError> {
            lock(&self.token)
                .clone()
                .ok_or(ApplicationControlError::NoForegroundWindow)
        }

        fn classify_focus(
            &self,
            token: &WindowToken,
        ) -> Result<FocusSnapshot, ApplicationControlError> {
            let region = *lock(&self.focus);
            Ok(FocusSnapshot {
                region,
                control_region: region.control_region(),
                generation: token.generation(),
            })
        }

        fn capabilities(
            &self,
            token: &WindowToken,
        ) -> Result<CapabilitySnapshot, ApplicationControlError> {
            let focus = self.classify_focus(token)?;
            Ok(CapabilitySnapshot {
                application_id: token.application_id().to_owned(),
                adapter: token.adapter(),
                generation: token.generation(),
                focus,
                actions: lock(&self.available)
                    .iter()
                    .cloned()
                    .map(|action| ActionCapability {
                        action,
                        state: CapabilityState::Available,
                        reason: CapabilityReason::UiaPattern,
                    })
                    .collect(),
            })
        }

        fn perform(&self, token: &WindowToken, action: SemanticAction) -> ActionOutcome {
            lock(&self.performed).push(action.clone());
            ActionOutcome {
                action,
                result: ActionResult::Performed,
                reason: Some(CapabilityReason::UiaPattern),
                generation: token.generation(),
            }
        }

        fn restore_foreground(&self, _token: &WindowToken) -> Result<(), ApplicationControlError> {
            Ok(())
        }
    }

    #[derive(Default)]
    struct FakeLauncher {
        targets: Mutex<Vec<String>>,
    }

    impl ApplicationLauncherBackend for FakeLauncher {
        fn activate_or_launch(&self, target: &str) -> Result<(), String> {
            lock(&self.targets).push(target.to_owned());
            Ok(())
        }
    }

    fn configuration(application_id: &str) -> MappingConfiguration {
        let mut input = BTreeMap::new();
        input.insert(
            RemoteButton::Ok,
            SemanticButtonActions {
                single: SemanticAction::Send,
                ..SemanticButtonActions::default()
            },
        );
        MappingConfiguration {
            common_mappings: ButtonMappings::default(),
            template_control_enabled: true,
            templates: vec![MappingTemplate {
                id: "default".to_owned(),
                name: "Default".to_owned(),
                region_actions: BTreeMap::from([(ControlRegion::Input, input)]),
                adjustment_mode: AdjustmentMode::Volume,
            }],
            application_bindings: vec![ApplicationBinding {
                application_id: application_id.to_owned(),
                template_id: "default".to_owned(),
                menu_order: 0,
                launch_target: Some(application_id.to_owned()),
            }],
        }
    }

    fn controller(application: Arc<FakeApplication>) -> (Arc<SceneController>, Arc<FakeLauncher>) {
        let launcher = Arc::new(FakeLauncher::default());
        let controller = SceneController::with_backends(
            application as Arc<dyn ApplicationControlBackend>,
            Arc::clone(&launcher) as Arc<dyn ApplicationLauncherBackend>,
            false,
        );
        controller.set_configuration(configuration("codex"));
        wait_for_worker();
        (controller, launcher)
    }

    fn wait_for_worker() {
        std::thread::sleep(Duration::from_millis(30));
    }

    fn routed(button: RemoteButton, trigger: ButtonTrigger) -> RoutedGesture {
        RoutedGesture {
            gesture: FiredGesture { button, trigger },
            native_delivered: false,
        }
    }

    #[test]
    fn menu_short_and_long_open_distinct_panels() {
        let (scene, _) = controller(Arc::new(FakeApplication::new("codex", 1)));
        scene.handle_edge(ButtonEdge {
            button: RemoteButton::Menu,
            is_pressed: true,
        });
        assert_eq!(
            scene.handle_gesture(routed(RemoteButton::Menu, ButtonTrigger::Single)),
            GestureDisposition::Handled
        );
        assert_eq!(scene.snapshot().panel, Some(ScenePanel::Application));
        let (scene, _) = controller(Arc::new(FakeApplication::new("codex", 1)));
        scene.handle_edge(ButtonEdge {
            button: RemoteButton::Menu,
            is_pressed: true,
        });
        assert_eq!(
            scene.handle_gesture(routed(RemoteButton::Menu, ButtonTrigger::Long)),
            GestureDisposition::Handled
        );
        assert_eq!(scene.snapshot().panel, Some(ScenePanel::Adjustment));
    }

    #[test]
    fn foreground_change_requires_all_up_and_voice_closes_menu() {
        let app = Arc::new(FakeApplication::new("codex", 1));
        let (scene, _) = controller(Arc::clone(&app));
        scene.handle_edge(ButtonEdge {
            button: RemoteButton::Menu,
            is_pressed: true,
        });
        scene.handle_gesture(routed(RemoteButton::Menu, ButtonTrigger::Single));
        scene.notify_voice_active(true);
        assert_eq!(scene.snapshot().panel, None);
        assert!(scene.snapshot().waiting_for_release);
        scene.handle_edge(ButtonEdge {
            button: RemoteButton::Menu,
            is_pressed: false,
        });
        assert!(!scene.snapshot().waiting_for_release);
        scene.notify_voice_active(false);

        scene.handle_edge(ButtonEdge {
            button: RemoteButton::Ok,
            is_pressed: true,
        });
        *lock(&app.token) = Some(WindowToken::from_identity(
            "codex".to_owned(),
            ApplicationAdapterKind::Generic,
            2,
            2,
            2,
        ));
        scene.refresh_foreground();
        wait_for_worker();
        assert_eq!(scene.snapshot().foreground_generation, 2);
        assert!(scene.snapshot().waiting_for_release);
        scene.handle_edge(ButtonEdge {
            button: RemoteButton::Ok,
            is_pressed: false,
        });
        assert!(!scene.snapshot().waiting_for_release);
    }

    #[test]
    fn send_runs_once_and_unsupported_is_blocked() {
        let app = Arc::new(FakeApplication::new("codex", 1));
        let (scene, _) = controller(Arc::clone(&app));
        scene.handle_edge(ButtonEdge {
            button: RemoteButton::Ok,
            is_pressed: true,
        });
        assert_eq!(
            scene.handle_gesture(routed(RemoteButton::Ok, ButtonTrigger::Single)),
            GestureDisposition::Handled
        );
        assert_eq!(
            scene.handle_gesture(routed(RemoteButton::Ok, ButtonTrigger::Single)),
            GestureDisposition::Blocked
        );
        wait_for_worker();
        assert_eq!(lock(&app.performed).as_slice(), &[SemanticAction::Send]);
        scene.handle_edge(ButtonEdge {
            button: RemoteButton::Ok,
            is_pressed: false,
        });

        lock(&app.available).clear();
        scene.refresh_foreground();
        wait_for_worker();
        scene.handle_edge(ButtonEdge {
            button: RemoteButton::Ok,
            is_pressed: true,
        });
        assert_eq!(
            scene.handle_gesture(routed(RemoteButton::Ok, ButtonTrigger::Single)),
            GestureDisposition::Blocked
        );
        assert_eq!(lock(&app.performed).len(), 1);
    }

    #[test]
    fn leaked_or_unbound_gesture_never_adds_a_scene_action() {
        let app = Arc::new(FakeApplication::new("codex", 1));
        let (scene, _) = controller(Arc::clone(&app));
        scene.handle_edge(ButtonEdge {
            button: RemoteButton::Ok,
            is_pressed: true,
        });
        let mut leaked = routed(RemoteButton::Ok, ButtonTrigger::Single);
        leaked.native_delivered = true;
        assert_eq!(scene.handle_gesture(leaked), GestureDisposition::Blocked);
        assert!(lock(&app.performed).is_empty());

        scene.set_configuration(configuration("wechat"));
        wait_for_worker();
        scene.handle_edge(ButtonEdge {
            button: RemoteButton::Ok,
            is_pressed: false,
        });
        scene.handle_edge(ButtonEdge {
            button: RemoteButton::Ok,
            is_pressed: true,
        });
        assert_eq!(
            scene.handle_gesture(routed(RemoteButton::Ok, ButtonTrigger::Single)),
            GestureDisposition::PassThrough
        );
    }

    #[test]
    fn launch_timeout_defers_late_foreground_until_user_cancels() {
        let app = Arc::new(FakeApplication::new("codex", 1));
        let (scene, _) = controller(Arc::clone(&app));
        let events = Arc::new(Mutex::new(Vec::new()));
        scene.subscribe(Arc::new({
            let events = Arc::clone(&events);
            move |event| lock(&events).push(event)
        }));

        scene.handle_edge(ButtonEdge {
            button: RemoteButton::Menu,
            is_pressed: true,
        });
        scene.handle_gesture(routed(RemoteButton::Menu, ButtonTrigger::Single));
        scene.handle_edge(ButtonEdge {
            button: RemoteButton::Menu,
            is_pressed: false,
        });
        *lock(&app.token) = Some(WindowToken::from_identity(
            "executable:wrong.exe".to_owned(),
            ApplicationAdapterKind::Generic,
            2,
            2,
            2,
        ));
        scene.handle_edge(ButtonEdge {
            button: RemoteButton::Ok,
            is_pressed: true,
        });
        scene.handle_gesture(routed(RemoteButton::Ok, ButtonTrigger::Single));
        std::thread::sleep(Duration::from_millis(140));

        scene.refresh_foreground();
        wait_for_worker();
        let snapshot = scene.snapshot();
        assert_eq!(snapshot.panel, Some(ScenePanel::Application));
        assert_eq!(snapshot.application_id.as_deref(), Some("codex"));
        assert_eq!(
            snapshot.status.as_deref(),
            Some("launch_foreground_timeout")
        );
        assert!(lock(&events).iter().any(|event| matches!(
            event,
            SceneEvent::LaunchFailed { application_id, .. } if application_id == "codex"
        )));

        scene.handle_edge(ButtonEdge {
            button: RemoteButton::Ok,
            is_pressed: false,
        });
        scene.handle_edge(ButtonEdge {
            button: RemoteButton::Back,
            is_pressed: true,
        });
        assert_eq!(
            scene.handle_gesture(routed(RemoteButton::Back, ButtonTrigger::Single)),
            GestureDisposition::Handled
        );
        wait_for_worker();
        let snapshot = scene.snapshot();
        assert_eq!(snapshot.panel, None);
        assert_eq!(
            snapshot.application_id.as_deref(),
            Some("executable:wrong.exe")
        );
        assert!(snapshot.waiting_for_release);
        scene.handle_edge(ButtonEdge {
            button: RemoteButton::Back,
            is_pressed: false,
        });
        assert!(!scene.snapshot().waiting_for_release);
    }

    #[test]
    fn scene_recognition_is_independent_from_disabled_global_mappings() {
        let mut configuration = configuration("codex");
        configuration.common_mappings.enabled = false;
        configuration.templates[0]
            .region_actions
            .get_mut(&ControlRegion::Input)
            .unwrap()
            .get_mut(&RemoteButton::Ok)
            .unwrap()
            .long = SemanticAction::Send;
        let recognition = scene_recognition_mappings(&configuration);
        assert!(recognition.enabled);
        assert_ne!(
            recognition.action_for(RemoteButton::Ok, ButtonTrigger::Single),
            ButtonAction::Disabled
        );
        assert_ne!(
            recognition.action_for(RemoteButton::Ok, ButtonTrigger::Long),
            ButtonAction::Disabled
        );
        assert_ne!(
            recognition.action_for(RemoteButton::Menu, ButtonTrigger::Long),
            ButtonAction::Disabled
        );
        assert_ne!(
            recognition.action_for(RemoteButton::VolumeUp, ButtonTrigger::Single),
            ButtonAction::Disabled
        );

        configuration.template_control_enabled = false;
        assert!(!scene_recognition_mappings(&configuration).enabled);
    }

    #[test]
    fn adjustment_actions_repeat_while_the_button_is_held() {
        let app = Arc::new(FakeApplication::new("codex", 1));
        *lock(&app.available) = vec![SemanticAction::VolumeUp];
        let (scene, _) = controller(Arc::clone(&app));
        scene.handle_edge(ButtonEdge {
            button: RemoteButton::VolumeUp,
            is_pressed: true,
        });
        assert_eq!(
            scene.handle_gesture(routed(RemoteButton::VolumeUp, ButtonTrigger::Single)),
            GestureDisposition::Handled
        );
        assert_eq!(
            scene.handle_gesture(routed(RemoteButton::VolumeUp, ButtonTrigger::Single)),
            GestureDisposition::Handled
        );
        wait_for_worker();
        assert_eq!(
            lock(&app.performed).as_slice(),
            &[SemanticAction::VolumeUp, SemanticAction::VolumeUp]
        );
    }
}
