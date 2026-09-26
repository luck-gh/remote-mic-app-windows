//! Program-default templates and our own interactive template menu.
//! Third-party controls are never queried; mapped keys run in ButtonMappingRuntime.

use crate::application_control::{
    ApplicationControlBackend, ApplicationControlError, ApplicationController, WindowToken,
};
use crate::button_mapping::{FiredGesture, GestureDisposition, RoutedGesture};
use crate::raw_input::{ButtonEdge, RemoteButton};
use crate::send_input::{ButtonAction, ButtonMappings, ButtonTrigger, KeyChord, KeyCode};
use crate::templates::MappingConfiguration;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::sync::{Arc, Mutex, RwLock, Weak};
use std::thread::JoinHandle;
use std::time::Duration;

const PROGRAM_QUEUE_CAPACITY: usize = 32;
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
    Template,
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
    pub mapping_notice_enabled: bool,
    pub mapping_notice: Option<MappingNotice>,
    pub mapping_notice_revision: u64,
    pub enabled: bool,
    pub generation: u64,
    pub foreground_generation: u64,
    pub application_id: Option<String>,
    pub template_id: Option<String>,
    pub panel: Option<ScenePanel>,
    pub update_default: bool,
    pub preference_pending: bool,
    pub preference_error: bool,
    pub selected_index: Option<usize>,
    pub menu_items: Vec<SceneMenuItem>,
    pub waiting_for_release: bool,
    pub voice_active: bool,
    pub status: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SceneEvent {
    MappingNoticeEnabled {
        enabled: bool,
    },
    MappingApplied {
        notice: MappingNotice,
        revision: u64,
    },
    Snapshot {
        snapshot: SceneSnapshot,
    },
    DefaultTemplatePersistenceRequested {
        request_id: u64,
        application_id: String,
        template_id: String,
    },
    MenuPreferencePersistenceRequested {
        request_id: u64,
        enabled: bool,
    },
}

pub type SceneEventCallback = Arc<dyn Fn(SceneEvent) + Send + Sync>;

/// Current foreground selection, confirmed by the mapping engine after applying it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MappingNotice {
    pub kind: String,
    pub template_id: Option<String>,
    pub name: Option<String>,
    pub actions_available: bool,
    pub default_save_status: Option<String>,
}

#[derive(Debug, Clone)]
struct PanelState {
    kind: ScenePanel,
    selected: usize,
}

struct State {
    manual_template_id: Option<String>,
    manual_application_id: Option<String>,
    mapping_notice: Option<MappingNotice>,
    mapping_notice_revision: u64,
    configuration: MappingConfiguration,
    generation: u64,
    foreground_generation: u64,
    token: Option<WindowToken>,
    panel: Option<PanelState>,
    template_menu_focused: bool,
    menu_application_id: Option<String>,
    update_default: bool,
    preference_request: Option<(u64, u64, bool)>,
    preference_error: bool,
    default_request_sequence: u64,
    default_request: Option<(u64, String, String)>,
    default_event: Option<SceneEvent>,
    default_save_status: Option<String>,
    menu_native_held: BTreeSet<RemoteButton>,
    menu_native_routed: BTreeSet<RemoteButton>,
    menu_close_after_release: Option<bool>,
    menu_press_generation: Option<u64>,
    held: BTreeSet<RemoteButton>,
    handled_in_cycle: BTreeSet<RemoteButton>,
    waiting_for_release: bool,
    voice_active: bool,
    status: Option<String>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            manual_template_id: None,
            manual_application_id: None,
            mapping_notice: None,
            mapping_notice_revision: 0,
            configuration: MappingConfiguration::default(),
            generation: 1,
            foreground_generation: 0,
            token: None,
            panel: None,
            template_menu_focused: false,
            menu_application_id: None,
            update_default: false,
            preference_request: None,
            preference_error: false,
            default_request_sequence: 0,
            default_request: None,
            default_event: None,
            default_save_status: None,
            menu_native_held: BTreeSet::new(),
            menu_native_routed: BTreeSet::new(),
            menu_close_after_release: None,
            menu_press_generation: None,
            held: BTreeSet::new(),
            handled_in_cycle: BTreeSet::new(),
            waiting_for_release: false,
            voice_active: false,
            status: None,
        }
    }
}

enum Work {
    RefreshForeground,
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
        Self::with_backends(Arc::new(ApplicationController::new()), true)
    }

    fn with_backends(
        application: Arc<dyn ApplicationControlBackend>,
        watch_foreground: bool,
    ) -> Arc<Self> {
        let state = Arc::new(Mutex::new(State::default()));
        let callbacks = Arc::new(RwLock::new(Vec::new()));
        let (sender, receiver) = mpsc::sync_channel(PROGRAM_QUEUE_CAPACITY);
        let worker_state = Arc::clone(&state);
        let worker_callbacks = Arc::clone(&callbacks);
        let worker = std::thread::Builder::new()
            .name("sayall-program-selection".to_owned())
            .spawn(move || program_worker(receiver, worker_state, worker_callbacks, application))
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
        {
            let mut state = lock(&self.state);
            state.configuration = configuration;
            if !state.configuration.menu_template_switch_enabled
                || state
                    .manual_template_id
                    .as_deref()
                    .is_some_and(|id| !template_exists(&state.configuration, id))
            {
                state.manual_template_id = None;
                state.manual_application_id = None;
            }
            cancel_state(&mut state, "configuration_changed");
        }
        self.emit_snapshot();
        self.refresh_foreground();
        self.active_scene_mappings()
    }

    pub fn snapshot(&self) -> SceneSnapshot {
        snapshot_from(&lock(&self.state))
    }

    pub fn set_mapping_notice_enabled(&self, enabled: bool) {
        lock(&self.state).configuration.mapping_notice_enabled = enabled;
        // Presentation-only: do not cancel held keys or reapply input mappings.
        emit(
            &self.callbacks,
            SceneEvent::MappingNoticeEnabled { enabled },
        );
    }

    #[cfg(test)]
    pub(crate) fn mapping_notice_candidate(&self) -> (u64, u64, MappingNotice) {
        let state = lock(&self.state);
        (
            state.generation,
            state.foreground_generation,
            mapping_notice_from(&state),
        )
    }

    pub(crate) fn confirm_mapping_notice(
        &self,
        generation: u64,
        foreground: u64,
        mut notice: MappingNotice,
        available: bool,
    ) {
        let event = {
            let mut state = lock(&self.state);
            if state.generation != generation || state.foreground_generation != foreground {
                return;
            }
            notice.actions_available =
                available && !matches!(notice.kind.as_str(), "disabled" | "unconfigured");
            if state.mapping_notice.as_ref() == Some(&notice) {
                return;
            }
            state.mapping_notice_revision = state.mapping_notice_revision.saturating_add(1);
            state.mapping_notice = Some(notice.clone());
            SceneEvent::MappingApplied {
                notice,
                revision: state.mapping_notice_revision,
            }
        };
        crate::ble::gatt_note(format!("mapping_notice phase=applied generation={generation} foreground_generation={foreground} actions_available={available}"));
        emit(&self.callbacks, event);
    }

    /// Returns the ordinary-key profile selected by the current foreground
    /// identity. `None` means the common mappings remain active.
    pub fn active_button_mapping(&self) -> Option<ButtonMappings> {
        active_button_mapping_from(&lock(&self.state))
    }

    pub fn active_scene_mappings(&self) -> ButtonMappings {
        active_scene_mappings_from(&lock(&self.state))
    }

    pub(crate) fn application_mapping_update(
        &self,
    ) -> (
        Option<ButtonMappings>,
        ButtonMappings,
        u64,
        u64,
        MappingNotice,
    ) {
        let state = lock(&self.state);
        (
            active_button_mapping_from(&state),
            active_scene_mappings_from(&state),
            state.generation,
            state.foreground_generation,
            mapping_notice_from(&state),
        )
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

    /// Restore only after the explicitly focused template menu releases its keys.
    pub fn restore_target_foreground(&self) -> Result<(), ApplicationControlError> {
        let token = lock(&self.state)
            .token
            .clone()
            .ok_or(ApplicationControlError::NoForegroundWindow)?;
        let (reply, receiver) = mpsc::sync_channel(1);
        self.sender
            .try_send(Work::RestoreForeground { token, reply })
            .map_err(|_| ApplicationControlError::WindowOperationFailed)?;
        let result = receiver
            .recv_timeout(Duration::from_secs(1))
            .map_err(|_| ApplicationControlError::WindowOperationFailed)?
            .map_err(|_| ApplicationControlError::WindowOperationFailed);
        if result.is_ok() {
            self.refresh_foreground();
        }
        result
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
                if state.held.insert(edge.button) {
                    state.handled_in_cycle.remove(&edge.button);
                    if edge.button == RemoteButton::Menu {
                        // The press that opens the panel has no panel generation.
                        // Only a new press inside the verified interactive menu counts.
                        state.menu_press_generation =
                            default_intent_available(&state).then_some(state.generation);
                    }
                }
            } else {
                state.held.remove(&edge.button);
                if edge.button == RemoteButton::Menu {
                    if state.menu_press_generation.take().is_some() {
                        crate::gatt_note(format!(
                            "template_default_intent phase=release generation={} long_consumed={}",
                            state.generation,
                            state.handled_in_cycle.contains(&RemoteButton::Menu)
                        ));
                    }
                }
                state.menu_native_routed.remove(&edge.button);
                emit |= finish_template_menu(&mut state);
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

    /// Called only by the host after checking this menu's actual foreground HWND.
    pub fn set_template_menu_focus(&self, focused: bool) {
        {
            let mut state = lock(&self.state);
            if !is_template_menu(&state) {
                return;
            }
            state.template_menu_focused = focused;
            if !focused {
                state.menu_press_generation = None;
                state.menu_native_held.clear();
                state.menu_native_routed.clear();
                state.menu_close_after_release = None;
                state.panel = None;
                state.status = Some("template_menu_focus_lost".to_owned());
                state.waiting_for_release = !state.held.is_empty();
            }
        }
        crate::ble::gatt_note(format!("template_menu phase=focus verified={focused}"));
        self.emit_snapshot();
    }

    /// Native keys belong to our focused menu, not to an inferred remote device.
    pub fn template_menu_key(&self, generation: u64, button: RemoteButton, down: bool) -> bool {
        {
            let mut state = lock(&self.state);
            let draining_up = !down && state.menu_native_held.contains(&button);
            if !is_template_menu(&state)
                || !state.template_menu_focused
                || (!draining_up && (state.generation != generation || state.voice_active))
            {
                return false;
            }
            if down {
                state.menu_native_held.insert(button);
                if state.menu_close_after_release.is_none() {
                    match button {
                        RemoteButton::Up
                        | RemoteButton::Left
                        | RemoteButton::Down
                        | RemoteButton::Right => {
                            let panel = state.panel.clone().unwrap();
                            route_panel(
                                &mut state,
                                panel,
                                FiredGesture {
                                    button,
                                    trigger: ButtonTrigger::Single,
                                },
                            );
                        }
                        _ => {}
                    }
                }
            } else if state.menu_native_held.remove(&button) {
                if state.menu_close_after_release.is_none() {
                    match button {
                        RemoteButton::Ok if state.preference_request.is_none() => {
                            state.menu_close_after_release = Some(true)
                        }
                        RemoteButton::Back => state.menu_close_after_release = Some(false),
                        _ => {}
                    }
                }
                finish_template_menu(&mut state);
            }
        }
        crate::ble::gatt_note(format!(
            "template_menu phase=native_key button={button:?} down={down}"
        ));
        self.emit_snapshot();
        true
    }

    /// Keep the focused menu alive through a held native key's real release.
    pub fn prepare_template_menu_exit(&self) -> bool {
        let ready = {
            let mut state = lock(&self.state);
            let preference_pending = state.preference_request.is_some();
            if !is_template_menu(&state) {
                return !preference_pending;
            }
            state.menu_close_after_release = Some(false);
            state.menu_press_generation = None;
            finish_template_menu(&mut state);
            !is_template_menu(&state) && !preference_pending
        };
        self.emit_snapshot();
        ready
    }

    pub fn template_menu_restore_failed(&self) {
        {
            let mut state = lock(&self.state);
            open_panel(&mut state, ScenePanel::Template);
            state.template_menu_focused = true;
            state.status = Some("template_menu_restore_failed".to_owned());
        }
        self.emit_snapshot();
    }

    pub fn set_update_default(&self, generation: u64, enabled: bool) -> bool {
        let changed = {
            let mut state = lock(&self.state);
            if state.generation != generation || !default_intent_available(&state) {
                return false;
            }
            set_default_intent(&mut state, enabled, "control");
            true
        };
        self.emit_snapshot();
        changed
    }

    pub fn complete_default_save(&self, request_id: u64, saved: bool) {
        {
            let mut state = lock(&self.state);
            let Some((id, application, _)) = state.default_request.as_ref() else {
                return;
            };
            if *id != request_id {
                return;
            }
            let applies_here = state
                .token
                .as_ref()
                .is_some_and(|t| t.application_id().eq_ignore_ascii_case(application));
            state.default_request = None;
            if applies_here {
                state.default_save_status = Some(if saved { "saved" } else { "failed" }.into());
            }
        }
        crate::gatt_note(format!(
            "template_default phase=completed request_id={request_id} saved={saved}"
        ));
        self.emit_snapshot();
    }

    pub fn complete_menu_preference_save(&self, request_id: u64, saved: bool) {
        {
            let mut state = lock(&self.state);
            let Some((id, generation, enabled)) = state.preference_request else {
                return;
            };
            if id != request_id {
                return;
            }
            state.preference_request = None;
            if saved {
                state.configuration.menu_update_default = enabled;
            }
            // A closed/replaced menu is never reopened or rewritten by an old response.
            if state.generation == generation {
                state.update_default = state.configuration.menu_update_default;
            }
            state.preference_error = !saved;
        }
        crate::gatt_note(format!(
            "template_default_intent phase=persisted request_id={request_id} saved={saved}"
        ));
        self.emit_snapshot();
    }

    pub fn handle_gesture(&self, routed: RoutedGesture) -> GestureDisposition {
        let disposition = {
            let mut state = lock(&self.state);
            if state.token.is_none() {
                return GestureDisposition::PassThrough;
            }
            let menu_key = state.configuration.menu_template_switch_enabled
                && routed.gesture.button == RemoteButton::Menu;
            if state.panel.is_none() && !menu_key {
                return GestureDisposition::PassThrough;
            }
            if state.voice_active || state.waiting_for_release {
                return GestureDisposition::Blocked;
            }
            if routed.native_delivered {
                if is_template_menu(&state) {
                    state.menu_native_routed.insert(routed.gesture.button);
                }
                GestureDisposition::Blocked
            } else if is_template_menu(&state) {
                if !state.template_menu_focused
                    || state.menu_native_routed.contains(&routed.gesture.button)
                    || state.menu_native_held.contains(&routed.gesture.button)
                {
                    GestureDisposition::Blocked
                } else {
                    let panel = state.panel.clone().unwrap();
                    route_panel(&mut state, panel, routed.gesture)
                }
            } else if menu_key
                && routed.gesture.trigger == ButtonTrigger::Single
                && state.handled_in_cycle.insert(RemoteButton::Menu)
            {
                open_panel(&mut state, ScenePanel::Template);
                GestureDisposition::Handled
            } else {
                GestureDisposition::Blocked
            }
        };
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
                lock(&self.state).status = Some("program_monitor_stopped".to_owned());
                false
            }
        }
    }

    fn emit_snapshot(&self) {
        emit_snapshot_for(&self.state, &self.callbacks);
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

fn route_panel(
    state: &mut State,
    mut panel: PanelState,
    gesture: FiredGesture,
) -> GestureDisposition {
    if gesture.button == RemoteButton::Menu && gesture.trigger == ButtonTrigger::Long {
        let valid_press = state.menu_press_generation == Some(state.generation)
            && state.held.contains(&RemoteButton::Menu);
        if !valid_press
            || !default_intent_available(state)
            || !state.handled_in_cycle.insert(RemoteButton::Menu)
        {
            crate::gatt_note(format!(
                "template_default_intent phase=rejected generation={} valid_press={valid_press}",
                state.generation
            ));
            return GestureDisposition::Blocked;
        }
        set_default_intent(state, !state.update_default, "remote_long");
        return GestureDisposition::Handled;
    }
    if gesture.trigger != ButtonTrigger::Single || state.menu_close_after_release.is_some() {
        return GestureDisposition::Blocked;
    }
    match gesture.button {
        RemoteButton::Ok | RemoteButton::Back | RemoteButton::Power | RemoteButton::Menu => {
            if gesture.button == RemoteButton::Ok && state.preference_request.is_some() {
                return GestureDisposition::Blocked;
            }
            if !state.handled_in_cycle.insert(gesture.button) {
                return GestureDisposition::Blocked;
            }
            state.menu_close_after_release = Some(gesture.button == RemoteButton::Ok);
            finish_template_menu(state);
            return GestureDisposition::Handled;
        }
        _ => {}
    }
    let count = menu_items(state, ScenePanel::Template).len();
    if count == 0 {
        return GestureDisposition::Blocked;
    }
    match gesture.button {
        RemoteButton::Up | RemoteButton::Left => {
            panel.selected = if panel.selected == 0 {
                count - 1
            } else {
                panel.selected - 1
            }
        }
        RemoteButton::Down | RemoteButton::Right => panel.selected = (panel.selected + 1) % count,
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
    if kind == ScenePanel::Template {
        state.generation = state.generation.saturating_add(1);
        state.menu_press_generation = None;
        state.template_menu_focused = false;
        state.menu_native_held.clear();
        state.menu_native_routed.clear();
        state.menu_close_after_release = None;
    }
    state.menu_application_id = state.token.as_ref().map(|t| t.application_id().to_owned());
    state.update_default = state.configuration.menu_update_default;
    state.panel = Some(PanelState { kind, selected: 0 });
    state.status = None;
}

fn program_worker(
    receiver: Receiver<Work>,
    state: Arc<Mutex<State>>,
    callbacks: Arc<RwLock<Vec<SceneEventCallback>>>,
    application: Arc<dyn ApplicationControlBackend>,
) {
    while let Ok(work) = receiver.recv() {
        match work {
            Work::Shutdown => break,
            Work::RefreshForeground => {
                refresh_foreground_state(&state, &callbacks, application.as_ref())
            }
            Work::RestoreForeground { token, reply } => {
                let _ = reply.try_send(
                    application
                        .restore_foreground(&token)
                        .map_err(|e| e.to_string()),
                );
            }
        }
    }
}

fn refresh_foreground_state(
    state: &Arc<Mutex<State>>,
    callbacks: &Arc<RwLock<Vec<SceneEventCallback>>>,
    application: &dyn ApplicationControlBackend,
) {
    #[cfg(windows)]
    if own_process_is_foreground() {
        return;
    }
    let generation = lock(state).generation;
    let observed = application.identify_foreground();
    apply_foreground_observation(state, callbacks, generation, observed);
}

fn apply_foreground_observation(
    state: &Arc<Mutex<State>>,
    callbacks: &Arc<RwLock<Vec<SceneEventCallback>>>,
    generation: u64,
    observed: Result<WindowToken, ApplicationControlError>,
) {
    let mut changed = false;
    {
        let mut state = lock(state);
        if state.generation != generation {
            return;
        }
        match observed {
            Ok(token) => {
                if token.process_id() == std::process::id() {
                    return;
                }
                let program_changed = state.token.as_ref().is_none_or(|current| {
                    !current
                        .application_id()
                        .eq_ignore_ascii_case(token.application_id())
                });
                let window_changed = state
                    .token
                    .as_ref()
                    .is_none_or(|current| current.generation() != token.generation());
                if program_changed {
                    state.manual_template_id = None;
                    state.manual_application_id = None;
                    state.default_save_status = None;
                    cancel_state(&mut state, "program_changed");
                    changed = true;
                } else if window_changed && state.panel.is_some() {
                    // External focus loss cancels our menu; no foreground restoration.
                    state.panel = None;
                    state.template_menu_focused = false;
                    cancel_state(&mut state, "menu_focus_lost");
                    changed = true;
                }
                state.foreground_generation = token.generation();
                state.token = Some(token);
            }
            Err(_) => {
                if state.token.take().is_some() {
                    state.manual_template_id = None;
                    state.manual_application_id = None;
                    cancel_state(&mut state, "foreground_unavailable");
                    changed = true;
                }
            }
        }
    }
    if changed {
        crate::gatt_note(
            "template_program phase=changed identity=public_process ui_query=false".to_owned(),
        );
        emit_snapshot_for(state, callbacks);
    }
}

fn cancel_state(state: &mut State, reason: &str) {
    // A configuration/voice cancellation may not send a held native Enter back
    // to the target window. Drain the focused menu first; focus loss is separate.
    let drain_menu = is_template_menu(state)
        && state.template_menu_focused
        && (!state.held.is_empty() || !state.menu_native_held.is_empty());
    state.generation = state.generation.saturating_add(1).max(1);
    state.menu_press_generation = None;
    if drain_menu {
        state.menu_close_after_release = Some(false);
    } else {
        state.panel = None;
        state.template_menu_focused = false;
        state.menu_native_held.clear();
        state.menu_native_routed.clear();
        state.menu_close_after_release = None;
    }
    state.handled_in_cycle.clear();
    if !state.held.is_empty() {
        state.waiting_for_release = true;
    }
    state.status = Some(reason.to_owned());
}

fn is_template_menu(state: &State) -> bool {
    state
        .panel
        .as_ref()
        .is_some_and(|p| p.kind == ScenePanel::Template)
}

fn default_intent_available(state: &State) -> bool {
    is_template_menu(state)
        && state.configuration.menu_template_switch_enabled
        && state.template_menu_focused
        && !state.voice_active
        && !state.waiting_for_release
        && state.menu_close_after_release.is_none()
        && state.preference_request.is_none()
        && state.token.as_ref().is_some_and(|token| {
            state
                .menu_application_id
                .as_ref()
                .is_some_and(|application| token.application_id().eq_ignore_ascii_case(application))
        })
}

fn set_default_intent(state: &mut State, enabled: bool, source: &str) {
    state.update_default = enabled;
    state.preference_error = false;
    state.default_request_sequence = state.default_request_sequence.saturating_add(1);
    let request_id = state.default_request_sequence;
    state.preference_request = Some((request_id, state.generation, enabled));
    state.default_event = Some(SceneEvent::MenuPreferencePersistenceRequested {
        request_id,
        enabled,
    });
    crate::gatt_note(format!(
        "template_default_intent phase=changed source={source} generation={} enabled={enabled} persisted=false",
        state.generation
    ));
}

fn finish_template_menu(state: &mut State) -> bool {
    if !is_template_menu(state) || !state.held.is_empty() || !state.menu_native_held.is_empty() {
        return false;
    }
    let Some(confirm) = state.menu_close_after_release.take() else {
        return false;
    };
    let selected = state.panel.as_ref().unwrap().selected;
    if confirm && state.template_menu_focused && state.configuration.menu_template_switch_enabled {
        if let Some(item) = menu_items(state, ScenePanel::Template).get(selected) {
            state.manual_template_id = Some(item.template_id.clone());
            state.manual_application_id = state.menu_application_id.clone();
            state.default_save_status = None;
            state.default_request = None;
            if state.update_default {
                if let Some(application_id) = state.menu_application_id.clone() {
                    state.default_request_sequence =
                        state.default_request_sequence.saturating_add(1);
                    let request_id = state.default_request_sequence;
                    state.default_request =
                        Some((request_id, application_id.clone(), item.template_id.clone()));
                    state.default_event = Some(SceneEvent::DefaultTemplatePersistenceRequested {
                        request_id,
                        application_id,
                        template_id: item.template_id.clone(),
                    });
                    state.default_save_status = Some("saving".into());
                }
            }
            crate::ble::gatt_note(
                "template_selection source=menu phase=requested selected=true released=true"
                    .to_owned(),
            );
        }
    }
    state.panel = None;
    state.template_menu_focused = false;
    state.menu_press_generation = None;
    state.handled_in_cycle.clear();
    state.menu_native_routed.clear();
    state.status = Some(
        if confirm {
            "manual_template_selected"
        } else {
            "template_menu_cancelled"
        }
        .to_owned(),
    );
    true
}

fn active_template_id(state: &State) -> Option<String> {
    selected_template_for_application(state, state.token.as_ref()?.application_id())
        .map(str::to_owned)
}

fn template_exists(configuration: &MappingConfiguration, id: &str) -> bool {
    configuration.template_mappings(id).is_some()
}

fn selected_template_for_application<'a>(state: &'a State, app: &str) -> Option<&'a str> {
    if state.configuration.menu_template_switch_enabled
        && state
            .manual_application_id
            .as_ref()
            .is_some_and(|id| id.eq_ignore_ascii_case(app))
    {
        if let Some(id) = state
            .manual_template_id
            .as_deref()
            .filter(|id| template_exists(&state.configuration, id))
        {
            return Some(id);
        }
    }
    if !state.configuration.button_mapping_follow_enabled {
        return None;
    }
    state
        .configuration
        .application_bindings
        .iter()
        .find(|b| b.application_id.eq_ignore_ascii_case(app))
        .map(|b| b.template_id.as_str())
}
fn menu_items(state: &State, _kind: ScenePanel) -> Vec<SceneMenuItem> {
    state
        .configuration
        .template_catalog()
        .into_iter()
        .map(|t| SceneMenuItem {
            application_id: None,
            running: active_template_id(state).as_deref() == Some(t.id.as_str()),
            template_id: t.id,
            label: t.name,
        })
        .collect()
}
fn active_button_mapping_from(state: &State) -> Option<ButtonMappings> {
    state
        .configuration
        .template_mappings(selected_template_for_application(
            state,
            state.token.as_ref()?.application_id(),
        )?)
}
fn recognition_marker() -> ButtonAction {
    ButtonAction::Shortcut {
        chord: KeyChord {
            keys: vec![KeyCode::Escape],
        },
    }
}
fn active_scene_mappings_from(state: &State) -> ButtonMappings {
    let mut mappings = ButtonMappings {
        enabled: false,
        actions: Default::default(),
    };
    if state.token.is_some() && state.configuration.menu_template_switch_enabled {
        mappings.enabled = true;
        mappings
            .actions
            .entry(RemoteButton::Menu)
            .or_default()
            .single = recognition_marker();
        if state.panel.is_some() {
            mappings.actions.entry(RemoteButton::Menu).or_default().long = recognition_marker();
            for button in [
                RemoteButton::Up,
                RemoteButton::Down,
                RemoteButton::Left,
                RemoteButton::Right,
                RemoteButton::Ok,
                RemoteButton::Back,
                RemoteButton::Power,
            ] {
                mappings.actions.entry(button).or_default().single = recognition_marker();
            }
        }
    }
    mappings
}

fn mapping_notice_from(state: &State) -> MappingNotice {
    let active = active_template_id(state);
    let template = active.as_ref().and_then(|id| {
        state
            .configuration
            .template_catalog()
            .into_iter()
            .find(|t| t.id == *id)
    });
    let mappings = active_button_mapping_from(state)
        .unwrap_or_else(|| state.configuration.common_mappings.clone());
    MappingNotice {
        kind: if !mappings.enabled {
            "disabled"
        } else if mappings.mapped_mask() == 0 {
            "unconfigured"
        } else if template.is_some() {
            "direct"
        } else {
            "common"
        }
        .into(),
        template_id: active,
        name: template.map(|t| t.name),
        actions_available: false,
        default_save_status: state.default_save_status.clone(),
    }
}
fn snapshot_from(state: &State) -> SceneSnapshot {
    SceneSnapshot {
        mapping_notice_enabled: state.configuration.mapping_notice_enabled,
        mapping_notice: state.mapping_notice.clone(),
        mapping_notice_revision: state.mapping_notice_revision,
        enabled: state.configuration.button_mapping_follow_enabled,
        generation: state.generation,
        foreground_generation: state.foreground_generation,
        application_id: state.token.as_ref().map(|t| t.application_id().to_owned()),
        template_id: active_template_id(state),
        panel: state.panel.as_ref().map(|p| p.kind),
        update_default: state.update_default,
        preference_pending: state.preference_request.is_some(),
        preference_error: state.preference_error,
        selected_index: state.panel.as_ref().map(|p| p.selected),
        menu_items: state
            .panel
            .as_ref()
            .map_or_else(Vec::new, |p| menu_items(state, p.kind)),
        waiting_for_release: state.waiting_for_release,
        voice_active: state.voice_active,
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
    // Build the owned snapshot before invoking subscribers. Keeping the guard
    // alive through `emit` deadlocks subscribers that legitimately query the
    // current application-specific button profile from this controller.
    let (snapshot, event) = {
        let mut state = lock(state);
        (snapshot_from(&state), state.default_event.take())
    };
    emit(callbacks, SceneEvent::Snapshot { snapshot });
    if let Some(event) = event {
        emit(callbacks, event);
    }
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
    use crate::application_control::ApplicationAdapterKind;
    use crate::button_gestures::{GestureRecognizer, LONG_PRESS_THRESHOLD};
    use crate::templates::{
        ApplicationBinding, BUILTIN_AGENT_TEMPLATE_ID, BUILTIN_CHAT_TEMPLATE_ID,
    };
    fn token(app: &str, window: u64) -> WindowToken {
        WindowToken::from_identity(
            app.into(),
            ApplicationAdapterKind::Generic,
            u32::MAX,
            window,
            window,
        )
    }
    fn state() -> Arc<Mutex<State>> {
        let mut state = State::default();
        state.configuration.menu_template_switch_enabled = true;
        state.configuration.button_mapping_follow_enabled = true;
        state
            .configuration
            .application_bindings
            .push(ApplicationBinding {
                application_id: "a".into(),
                template_id: BUILTIN_AGENT_TEMPLATE_ID.into(),
                menu_order: 0,
                launch_target: None,
            });
        state.token = Some(token("a", 1));
        state.foreground_generation = 1;
        Arc::new(Mutex::new(state))
    }
    fn observe(state: &Arc<Mutex<State>>, value: WindowToken) {
        let generation = lock(state).generation;
        apply_foreground_observation(
            state,
            &Arc::new(RwLock::new(Vec::new())),
            generation,
            Ok(value),
        );
    }
    fn choose(state: &mut State, save: bool) {
        open_panel(state, ScenePanel::Template);
        state.template_menu_focused = true;
        assert!(!state.update_default);
        state.update_default = save;
        state.panel.as_mut().unwrap().selected = 1;
        state.menu_close_after_release = Some(true);
        assert!(finish_template_menu(state));
    }
    fn controller() -> SceneController {
        let (sender, _receiver) = mpsc::sync_channel(PROGRAM_QUEUE_CAPACITY);
        SceneController {
            state: state(),
            sender,
            callbacks: Arc::new(RwLock::new(Vec::new())),
            worker: Mutex::new(None),
            #[cfg(windows)]
            foreground_watcher: Mutex::new(None),
        }
    }
    fn commit_preference(controller: &SceneController, saved: bool) {
        let id = lock(&controller.state).preference_request.unwrap().0;
        controller.complete_menu_preference_save(id, saved);
    }
    fn menu_gesture(controller: &SceneController, trigger: ButtonTrigger) -> GestureDisposition {
        controller.handle_gesture(RoutedGesture {
            gesture: FiredGesture {
                button: RemoteButton::Menu,
                trigger,
            },
            native_delivered: false,
        })
    }
    // Match the production engine's edge-callback -> recognizer -> route order.
    fn menu_edge(
        controller: &SceneController,
        recognizer: &mut GestureRecognizer,
        down: bool,
        now: std::time::Instant,
    ) -> Vec<GestureDisposition> {
        controller.handle_edge(ButtonEdge {
            button: RemoteButton::Menu,
            is_pressed: down,
        });
        let gestures = if down {
            recognizer.press(RemoteButton::Menu, now)
        } else {
            recognizer.release(RemoteButton::Menu, now)
        };
        gestures
            .into_iter()
            .map(|trigger| menu_gesture(controller, trigger))
            .collect()
    }
    fn open_for_menu_hold(controller: &SceneController) -> GestureRecognizer {
        let mut s = lock(&controller.state);
        open_panel(&mut s, ScenePanel::Template);
        s.template_menu_focused = true;
        let mut recognizer = GestureRecognizer::new();
        recognizer.configure(&active_scene_mappings_from(&s));
        recognizer
    }
    #[test]
    fn opening_press_cannot_toggle_and_only_panel_recognition_adds_long() {
        let controller = controller();
        let config = lock(&controller.state).configuration.clone();
        let outside = active_scene_mappings_from(&lock(&controller.state));
        assert_eq!(
            outside.action_for(RemoteButton::Menu, ButtonTrigger::Long),
            ButtonAction::Disabled
        );
        let mut recognizer = GestureRecognizer::new();
        recognizer.configure(&outside);
        let now = std::time::Instant::now();
        assert_eq!(
            menu_edge(&controller, &mut recognizer, true, now),
            vec![GestureDisposition::Handled]
        );
        controller.set_template_menu_focus(true);
        assert_eq!(
            menu_gesture(&controller, ButtonTrigger::Long),
            GestureDisposition::Blocked
        );
        assert!(!controller.snapshot().update_default);
        assert!(menu_edge(
            &controller,
            &mut recognizer,
            false,
            now + LONG_PRESS_THRESHOLD
        )
        .is_empty());
        assert!(controller.snapshot().panel.is_some());
        assert_ne!(
            active_scene_mappings_from(&lock(&controller.state))
                .action_for(RemoteButton::Menu, ButtonTrigger::Long),
            ButtonAction::Disabled
        );
        assert_eq!(lock(&controller.state).configuration, config);
    }
    #[test]
    fn two_menu_holds_toggle_once_each_release_is_silent_and_intent_does_not_reconfigure() {
        let controller = controller();
        let mut recognizer = open_for_menu_hold(&controller);
        let mappings = active_scene_mappings_from(&lock(&controller.state));
        let now = std::time::Instant::now();
        for (index, expected) in [true, false].into_iter().enumerate() {
            let start = now + std::time::Duration::from_secs(index as u64 * 3);
            assert!(menu_edge(&controller, &mut recognizer, true, start).is_empty());
            let fired = recognizer.advance(start + LONG_PRESS_THRESHOLD);
            assert_eq!(fired, vec![(RemoteButton::Menu, ButtonTrigger::Long)]);
            assert_eq!(
                menu_gesture(&controller, fired[0].1),
                GestureDisposition::Handled
            );
            assert_eq!(controller.snapshot().update_default, expected);
            commit_preference(&controller, true);
            controller.handle_edge(ButtonEdge {
                button: RemoteButton::Menu,
                is_pressed: true,
            });
            assert_eq!(
                menu_gesture(&controller, ButtonTrigger::Long),
                GestureDisposition::Blocked
            );
            assert!(recognizer
                .advance(start + std::time::Duration::from_secs(2))
                .is_empty());
            assert!(menu_edge(
                &controller,
                &mut recognizer,
                false,
                start + std::time::Duration::from_secs(2)
            )
            .is_empty());
            let s = lock(&controller.state);
            assert!(s.panel.is_some());
            assert!(s.held.is_empty());
            assert!(s.default_event.is_none());
            assert!(s.default_request.is_none());
            assert_eq!(active_scene_mappings_from(&s), mappings);
        }
    }
    #[test]
    fn panel_short_menu_still_cancels_without_changing_or_saving_intent() {
        let controller = controller();
        let mut recognizer = open_for_menu_hold(&controller);
        let now = std::time::Instant::now();
        assert!(menu_edge(&controller, &mut recognizer, true, now).is_empty());
        assert_eq!(
            menu_edge(
                &controller,
                &mut recognizer,
                false,
                now + LONG_PRESS_THRESHOLD / 2
            ),
            vec![GestureDisposition::Handled]
        );
        let s = lock(&controller.state);
        assert!(s.panel.is_none());
        assert!(!s.update_default);
        assert!(s.default_event.is_none());
        assert!(s.manual_template_id.is_none());
    }
    #[test]
    fn stale_menu_hold_cannot_change_reopened_cancelled_or_unfocused_panel() {
        for reason in ["focus", "configuration", "disconnect", "exit", "target"] {
            let controller = controller();
            let mut recognizer = open_for_menu_hold(&controller);
            let now = std::time::Instant::now();
            menu_edge(&controller, &mut recognizer, true, now);
            let old_generation = controller.snapshot().generation;
            match reason {
                "focus" => controller.set_template_menu_focus(false),
                "exit" => {
                    assert!(!controller.prepare_template_menu_exit());
                }
                "target" => {
                    lock(&controller.state).token = Some(token("b", 2));
                }
                _ => cancel_state(&mut lock(&controller.state), reason),
            }
            assert_eq!(
                menu_gesture(&controller, ButtonTrigger::Long),
                GestureDisposition::Blocked,
                "{reason}"
            );
            assert!(!controller.set_update_default(old_generation, true));
            {
                let mut s = lock(&controller.state);
                s.held.clear();
                s.waiting_for_release = false;
                s.token = Some(token("a", 1));
                open_panel(&mut s, ScenePanel::Template);
                s.template_menu_focused = true;
            }
            assert_eq!(
                menu_gesture(&controller, ButtonTrigger::Long),
                GestureDisposition::Blocked
            );
            assert!(!controller.snapshot().update_default);
            assert!(!controller.set_update_default(old_generation, true));
        }
    }
    #[test]
    fn control_and_remote_share_intent_but_only_released_confirmation_requests_save() {
        let controller = controller();
        let mut recognizer = open_for_menu_hold(&controller);
        let generation = controller.snapshot().generation;
        assert!(controller.set_update_default(generation, true));
        commit_preference(&controller, true);
        let now = std::time::Instant::now();
        menu_edge(&controller, &mut recognizer, true, now);
        for (_, trigger) in recognizer.advance(now + LONG_PRESS_THRESHOLD) {
            assert_eq!(
                menu_gesture(&controller, trigger),
                GestureDisposition::Handled
            );
        }
        assert!(!controller.snapshot().update_default);
        commit_preference(&controller, true);
        menu_edge(
            &controller,
            &mut recognizer,
            false,
            now + LONG_PRESS_THRESHOLD,
        );
        assert!(controller.set_update_default(generation, true));
        commit_preference(&controller, true);
        assert!(lock(&controller.state).default_event.is_none());
        controller.template_menu_key(generation, RemoteButton::Ok, true);
        assert!(lock(&controller.state).default_request.is_none());
        controller.template_menu_key(generation, RemoteButton::Ok, false);
        assert!(lock(&controller.state).default_request.is_some());
    }
    #[test]
    fn menu_preference_survives_cancel_and_new_controller_without_applying_template() {
        let controller = controller();
        open_for_menu_hold(&controller);
        assert!(controller.set_update_default(controller.snapshot().generation, true));
        assert!(controller.snapshot().preference_pending);
        assert!(!controller.set_update_default(controller.snapshot().generation, false));
        assert_eq!(
            menu_gesture(&controller, ButtonTrigger::Single),
            GestureDisposition::Handled
        );
        assert!(controller.snapshot().panel.is_none());
        commit_preference(&controller, true);
        assert!(controller.snapshot().panel.is_none());
        assert!(lock(&controller.state).manual_template_id.is_none());
        assert!(lock(&controller.state).default_request.is_none());
        let saved = lock(&controller.state).configuration.clone();
        let reopened = super::tests::controller();
        reopened.set_configuration(saved);
        open_for_menu_hold(&reopened);
        assert!(reopened.snapshot().update_default);
    }
    #[test]
    fn normal_exit_drains_open_menu_and_pending_preference_before_becoming_ready() {
        let controller = controller();
        open_for_menu_hold(&controller);
        assert!(controller.set_update_default(controller.snapshot().generation, true));
        controller.handle_edge(ButtonEdge {
            button: RemoteButton::Menu,
            is_pressed: true,
        });
        assert!(!controller.prepare_template_menu_exit());
        assert!(controller.snapshot().panel.is_some());
        controller.handle_edge(ButtonEdge {
            button: RemoteButton::Menu,
            is_pressed: false,
        });
        assert!(controller.snapshot().panel.is_none());
        assert!(!controller.prepare_template_menu_exit());
        commit_preference(&controller, true);
        assert!(controller.prepare_template_menu_exit());
        assert!(lock(&controller.state).configuration.menu_update_default);
        assert!(lock(&controller.state).manual_template_id.is_none());
    }
    #[test]
    fn failed_preference_restores_saved_value_and_late_response_cannot_change_new_panel() {
        let controller = controller();
        open_for_menu_hold(&controller);
        let generation = controller.snapshot().generation;
        assert!(controller.set_update_default(generation, true));
        let stale_id = lock(&controller.state).preference_request.unwrap().0;
        commit_preference(&controller, false);
        assert!(controller.snapshot().preference_error);
        assert!(!controller.snapshot().update_default);
        assert!(!lock(&controller.state).configuration.menu_update_default);
        assert!(controller.set_update_default(generation, true));
        controller.complete_menu_preference_save(stale_id, true);
        assert!(controller.snapshot().preference_pending);
        controller.set_template_menu_focus(false);
        open_for_menu_hold(&controller);
        let new_generation = controller.snapshot().generation;
        assert!(!controller.snapshot().update_default);
        commit_preference(&controller, true);
        assert_eq!(controller.snapshot().generation, new_generation);
        assert!(!controller.snapshot().update_default);
        assert!(lock(&controller.state).configuration.menu_update_default);
        open_for_menu_hold(&controller);
        assert!(controller.snapshot().update_default);
    }
    #[test]
    fn repeated_direct_gestures_pass_through_and_same_template_notice_is_deduplicated() {
        let controller = controller();
        for _ in 0..7 {
            for button in [
                RemoteButton::Left,
                RemoteButton::Back,
                RemoteButton::VolumeUp,
            ] {
                assert_eq!(
                    controller.handle_gesture(RoutedGesture {
                        gesture: FiredGesture {
                            button,
                            trigger: ButtonTrigger::Single
                        },
                        native_delivered: false
                    }),
                    GestureDisposition::PassThrough
                );
            }
        }
        let (g, f, n) = controller.mapping_notice_candidate();
        controller.confirm_mapping_notice(g, f, n, true);
        let revision = controller.snapshot().mapping_notice_revision;
        observe(&controller.state, token("a", 2));
        let (g, f, n) = controller.mapping_notice_candidate();
        controller.confirm_mapping_notice(g, f, n, true);
        assert_eq!(controller.snapshot().mapping_notice_revision, revision);
    }
    #[test]
    fn failed_default_save_keeps_temporary_selection_and_ignores_stale_result() {
        let controller = controller();
        choose(&mut lock(&controller.state), true);
        let request = lock(&controller.state).default_request.as_ref().unwrap().0;
        controller.complete_default_save(request, false);
        assert_eq!(
            active_template_id(&lock(&controller.state)).as_deref(),
            Some(BUILTIN_CHAT_TEMPLATE_ID)
        );
        assert_eq!(
            lock(&controller.state).default_save_status.as_deref(),
            Some("failed")
        );
        choose(&mut lock(&controller.state), false);
        controller.complete_default_save(request, true);
        assert!(lock(&controller.state).default_save_status.is_none());
    }
    #[test]
    fn temporary_selection_survives_same_program_windows_and_own_windows_only() {
        let state = state();
        choose(&mut lock(&state), false);
        let generation = lock(&state).generation;
        observe(&state, token("a", 2));
        assert_eq!(
            active_template_id(&lock(&state)).as_deref(),
            Some(BUILTIN_CHAT_TEMPLATE_ID)
        );
        assert_eq!(lock(&state).generation, generation);
        let own = WindowToken::from_identity(
            "sayall".into(),
            ApplicationAdapterKind::Generic,
            std::process::id(),
            3,
            3,
        );
        observe(&state, own);
        assert_eq!(lock(&state).token.as_ref().unwrap().application_id(), "a");
        observe(&state, token("b", 4));
        observe(&state, token("a", 5));
        assert_eq!(
            active_template_id(&lock(&state)).as_deref(),
            Some(BUILTIN_AGENT_TEMPLATE_ID)
        );
    }
    #[test]
    fn temporary_selection_still_expires_when_program_defaults_disabled() {
        let state = state();
        lock(&state).configuration.button_mapping_follow_enabled = false;
        choose(&mut lock(&state), false);
        observe(&state, token("a", 2));
        assert!(active_template_id(&lock(&state)).is_some());
        observe(&state, token("b", 3));
        observe(&state, token("a", 4));
        assert!(active_template_id(&lock(&state)).is_none());
    }
    #[test]
    fn menu_captures_original_program_and_waits_for_both_release_channels() {
        let state = state();
        let mut state = lock(&state);
        open_panel(&mut state, ScenePanel::Template);
        state.template_menu_focused = true;
        state.update_default = true;
        state.panel.as_mut().unwrap().selected = 1;
        state.held.insert(RemoteButton::Ok);
        state.menu_native_held.insert(RemoteButton::Ok);
        state.menu_close_after_release = Some(true);
        assert!(!finish_template_menu(&mut state));
        state.held.clear();
        assert!(!finish_template_menu(&mut state));
        state.menu_native_held.clear();
        assert!(finish_template_menu(&mut state));
        assert!(
            matches!(&state.default_event,Some(SceneEvent::DefaultTemplatePersistenceRequested{application_id,template_id,..}) if application_id=="a" && template_id==BUILTIN_CHAT_TEMPLATE_ID)
        );
        open_panel(&mut state, ScenePanel::Template);
        assert!(!state.update_default);
    }
    #[test]
    fn stale_observation_cannot_replace_config_generation_and_focus_loss_never_selects() {
        let state = state();
        let stale = lock(&state).generation;
        choose(&mut lock(&state), false);
        apply_foreground_observation(
            &state,
            &Arc::new(RwLock::new(Vec::new())),
            stale,
            Ok(token("b", 9)),
        );
        assert_eq!(lock(&state).token.as_ref().unwrap().application_id(), "a");
        {
            let mut s = lock(&state);
            open_panel(&mut s, ScenePanel::Template);
            s.template_menu_focused = true;
            s.panel.as_mut().unwrap().selected = 2;
        }
        observe(&state, token("a", 2));
        assert!(lock(&state).panel.is_none());
        assert_eq!(
            active_template_id(&lock(&state)).as_deref(),
            Some(BUILTIN_CHAT_TEMPLATE_ID)
        );
    }
    #[test]
    fn ordinary_keys_have_no_scene_route_or_worker_work_even_with_menu_enabled() {
        let state = state();
        let markers = active_scene_mappings_from(&lock(&state));
        assert_eq!(markers.actions.len(), 1);
        assert!(markers.actions.contains_key(&RemoteButton::Menu));
        let profile = active_button_mapping_from(&lock(&state)).unwrap();
        for button in [
            RemoteButton::Left,
            RemoteButton::Right,
            RemoteButton::Back,
            RemoteButton::VolumeUp,
            RemoteButton::VolumeDown,
        ] {
            assert!(matches!(
                profile.action_for(button, ButtonTrigger::Single),
                ButtonAction::Shortcut { .. }
            ));
            assert_eq!(
                profile.action_for(button, ButtonTrigger::Double),
                ButtonAction::Disabled
            );
            assert_eq!(
                profile.action_for(button, ButtonTrigger::Long),
                ButtonAction::Disabled
            );
        }
    }
    #[test]
    fn menu_wraps_and_cancel_retains_native_release_barrier() {
        let state = state();
        let mut s = lock(&state);
        open_panel(&mut s, ScenePanel::Template);
        s.template_menu_focused = true;
        let len = menu_items(&s, ScenePanel::Template).len();
        let p = s.panel.clone().unwrap();
        route_panel(
            &mut s,
            p,
            FiredGesture {
                button: RemoteButton::Up,
                trigger: ButtonTrigger::Single,
            },
        );
        assert_eq!(s.panel.as_ref().unwrap().selected, len - 1);
        let p = s.panel.clone().unwrap();
        route_panel(
            &mut s,
            p,
            FiredGesture {
                button: RemoteButton::Down,
                trigger: ButtonTrigger::Single,
            },
        );
        assert_eq!(s.panel.as_ref().unwrap().selected, 0);
        s.menu_native_held.insert(RemoteButton::Ok);
        cancel_state(&mut s, "exit");
        assert!(s.panel.is_some());
        assert_eq!(s.menu_close_after_release, Some(false));
        s.menu_native_held.clear();
        assert!(finish_template_menu(&mut s));
        assert!(s.manual_template_id.is_none());
    }
}
