use sayall_core::VoiceInputTool;
use sayall_windows::raw_input::RawInputSnapshot;
use sayall_windows::send_input::{ButtonAction, KeyChord, ScrollDirection, SendInputSnapshot};
use sayall_windows::{
    AudioEndpoint, AudioSnapshot, ConnectionSnapshot, PairedRemote, PlatformError,
    PlatformSnapshot, UsageCounters, WindowsPlatform,
};
use std::fmt::Debug;
use std::sync::Arc;

pub trait PlatformRuntime: Debug + Send + Sync {
    fn capture_config_gate(&self) -> Arc<std::sync::Mutex<()>> {
        Arc::new(std::sync::Mutex::new(()))
    }
    fn shutdown_capture_input(&self) -> Result<(), String> {
        Ok(())
    }
    fn capture_input_snapshot(&self) -> sayall_windows::capture_input::CaptureInputSnapshot {
        Default::default()
    }
    fn initialize_capture_input(
        &self,
        _journal: std::path::PathBuf,
        _settings: sayall_core::CaptureInputSettings,
    ) -> Result<(), String> {
        Ok(())
    }
    fn list_capture_inputs(&self) -> Result<Vec<AudioEndpoint>, String> {
        Err("capture_routing_unsupported".into())
    }
    fn configure_capture_input(
        &self,
        _settings: sayall_core::CaptureInputSettings,
    ) -> Result<sayall_windows::capture_input::CaptureInputSnapshot, String> {
        Err("capture_routing_unsupported".into())
    }
    fn resolve_capture_recovery(
        &self,
        _restore: bool,
    ) -> Result<sayall_windows::capture_input::CaptureInputSnapshot, String> {
        Err("capture_routing_unsupported".into())
    }
    fn usage_counters(&self) -> Arc<UsageCounters>;
    fn snapshot(&self) -> PlatformSnapshot;
    fn scan_paired_remotes(&self) -> Result<Vec<PairedRemote>, PlatformError>;
    fn connection_snapshot(&self) -> ConnectionSnapshot;
    fn connect_remote(&self, device_id: String) -> Result<ConnectionSnapshot, PlatformError>;
    fn disconnect_remote(&self) -> Result<ConnectionSnapshot, PlatformError>;
    #[cfg(windows)]
    fn restore_remote(&self, device_id: String) -> Result<ConnectionSnapshot, PlatformError>;
    fn list_audio_endpoints(&self) -> Result<Vec<AudioEndpoint>, PlatformError>;
    fn select_audio_endpoint(&self, endpoint_id: String) -> Result<AudioSnapshot, PlatformError>;
    #[cfg(windows)]
    fn restore_audio_endpoint(
        &self,
        endpoint_id: String,
        expected_name: String,
    ) -> Result<AudioSnapshot, PlatformError>;
    fn audio_snapshot(&self) -> AudioSnapshot;
    fn raw_input_snapshot(&self) -> RawInputSnapshot;
    fn start_raw_input(&self) -> Result<RawInputSnapshot, PlatformError>;
    fn stop_raw_input(&self) -> Result<RawInputSnapshot, PlatformError>;
    fn send_input_snapshot(&self) -> SendInputSnapshot;
    fn test_shortcut(&self, chord: KeyChord) -> Result<SendInputSnapshot, PlatformError>;
    fn test_scroll(
        &self,
        direction: ScrollDirection,
        steps: u16,
    ) -> Result<SendInputSnapshot, PlatformError>;
    fn test_mouse_action(&self, action: ButtonAction) -> Result<SendInputSnapshot, PlatformError>;
    /// 预设应用清单（含安装状态）。
    fn preset_apps(&self) -> Vec<sayall_windows::app_launcher::PresetAppInfo>;
    /// 打开/激活预设应用（测试按钮与引擎共用路径）。
    fn launch_app(&self, target: &str) -> Result<(), PlatformError>;
    /// 打开固定的 Windows 蓝牙设置页；不接受前端 URI。
    fn open_bluetooth_settings(&self) -> Result<(), PlatformError>;
    fn voice_hold_hotkey(&self) -> Option<KeyChord>;
    fn set_voice_hold_hotkey(&self, hotkey: Option<KeyChord>);
    /// 「你在用的输入工具」：BLE 工作线程在语音会话开始前按它决定把哪个
    /// 输入法切进当前会话（`ime::ensure_session_ime`）。
    fn set_voice_input_tool(&self, _tool: Option<VoiceInputTool>) {}
    fn button_mappings(&self) -> sayall_windows::send_input::ButtonMappings;
    fn set_button_mappings(&self, mappings: sayall_windows::send_input::ButtonMappings);
    fn set_mapping_configuration(
        &self,
        configuration: sayall_windows::templates::MappingConfiguration,
    );
    fn scene_snapshot(&self) -> Option<sayall_windows::scene_control::SceneSnapshot>;
    fn set_template_menu_focus(&self, _focused: bool) {}
    fn template_menu_key(
        &self,
        _generation: u64,
        _button: sayall_windows::raw_input::RemoteButton,
        _down: bool,
    ) -> bool {
        false
    }
    fn set_template_menu_update_default(&self, _generation: u64, _enabled: bool) -> bool {
        false
    }
    fn complete_template_default_save(&self, _request_id: u64, _saved: bool) {}
    fn complete_menu_preference_save(&self, _request_id: u64, _saved: bool) {}
    fn prepare_template_menu_exit(&self) -> bool {
        true
    }
    fn restore_template_menu_target(&self) -> bool {
        false
    }
    fn template_menu_restore_failed(&self) {}
    fn set_mapping_notice_enabled(&self, enabled: bool);
    fn subscribe_scene_events(&self, callback: sayall_windows::scene_control::SceneEventCallback);
    fn button_mapping_snapshot(&self) -> sayall_windows::button_mapping::ButtonMappingSnapshot;
    fn subscribe_button_edges(&self, callback: sayall_windows::button_mapping::ButtonEdgeCallback);
    fn subscribe_button_gestures(
        &self,
        callback: sayall_windows::button_mapping::ButtonGestureCallback,
    );
    fn quiesce_input(&self) -> Result<(), PlatformError>;

    #[cfg(feature = "runtime-simulation")]
    fn run_simulated_voice_session(&self) -> Result<PlatformSnapshot, PlatformError> {
        Err(PlatformError::UnsupportedPlatform)
    }
}

impl PlatformRuntime for WindowsPlatform {
    fn capture_config_gate(&self) -> Arc<std::sync::Mutex<()>> {
        self.capture_config_gate()
    }
    fn shutdown_capture_input(&self) -> Result<(), String> {
        self.shutdown_capture_input()
    }
    fn capture_input_snapshot(&self) -> sayall_windows::capture_input::CaptureInputSnapshot {
        self.capture_input_snapshot()
    }
    fn initialize_capture_input(
        &self,
        journal: std::path::PathBuf,
        settings: sayall_core::CaptureInputSettings,
    ) -> Result<(), String> {
        self.initialize_capture_input(journal, settings)
    }
    fn list_capture_inputs(&self) -> Result<Vec<AudioEndpoint>, String> {
        self.list_capture_inputs()
    }
    fn configure_capture_input(
        &self,
        settings: sayall_core::CaptureInputSettings,
    ) -> Result<sayall_windows::capture_input::CaptureInputSnapshot, String> {
        self.configure_capture_input(settings)
    }
    fn resolve_capture_recovery(
        &self,
        restore: bool,
    ) -> Result<sayall_windows::capture_input::CaptureInputSnapshot, String> {
        self.resolve_capture_recovery(restore)
    }

    fn usage_counters(&self) -> Arc<UsageCounters> {
        self.usage_counters()
    }

    fn snapshot(&self) -> PlatformSnapshot {
        self.snapshot()
    }

    fn scan_paired_remotes(&self) -> Result<Vec<PairedRemote>, PlatformError> {
        self.scan_paired_remotes()
    }

    fn connection_snapshot(&self) -> ConnectionSnapshot {
        self.connection_snapshot()
    }

    fn connect_remote(&self, device_id: String) -> Result<ConnectionSnapshot, PlatformError> {
        self.connect_remote(device_id)
    }

    fn disconnect_remote(&self) -> Result<ConnectionSnapshot, PlatformError> {
        self.disconnect_remote()
    }

    #[cfg(windows)]
    fn restore_remote(&self, device_id: String) -> Result<ConnectionSnapshot, PlatformError> {
        self.restore_remote(device_id)
    }

    fn list_audio_endpoints(&self) -> Result<Vec<AudioEndpoint>, PlatformError> {
        self.list_audio_endpoints()
    }

    fn select_audio_endpoint(&self, endpoint_id: String) -> Result<AudioSnapshot, PlatformError> {
        self.select_audio_endpoint(endpoint_id)
    }

    #[cfg(windows)]
    fn restore_audio_endpoint(
        &self,
        endpoint_id: String,
        expected_name: String,
    ) -> Result<AudioSnapshot, PlatformError> {
        self.restore_audio_endpoint(endpoint_id, expected_name)
    }

    fn audio_snapshot(&self) -> AudioSnapshot {
        self.audio_snapshot()
    }

    fn raw_input_snapshot(&self) -> RawInputSnapshot {
        self.raw_input_snapshot()
    }

    fn start_raw_input(&self) -> Result<RawInputSnapshot, PlatformError> {
        self.start_raw_input()
    }

    fn stop_raw_input(&self) -> Result<RawInputSnapshot, PlatformError> {
        self.stop_raw_input()
    }

    fn send_input_snapshot(&self) -> SendInputSnapshot {
        self.send_input_snapshot()
    }

    fn test_shortcut(&self, chord: KeyChord) -> Result<SendInputSnapshot, PlatformError> {
        self.test_shortcut(chord)
    }

    fn test_scroll(
        &self,
        direction: ScrollDirection,
        steps: u16,
    ) -> Result<SendInputSnapshot, PlatformError> {
        self.test_scroll(direction, steps)
    }

    fn test_mouse_action(&self, action: ButtonAction) -> Result<SendInputSnapshot, PlatformError> {
        self.test_mouse_action(action)
    }

    fn preset_apps(&self) -> Vec<sayall_windows::app_launcher::PresetAppInfo> {
        sayall_windows::app_launcher::probe_preset_apps()
    }

    fn launch_app(&self, target: &str) -> Result<(), PlatformError> {
        sayall_windows::app_launcher::activate_or_launch(target)
            .map_err(|error| PlatformError::SendInput(error.to_string()))
    }

    fn open_bluetooth_settings(&self) -> Result<(), PlatformError> {
        sayall_windows::app_launcher::open_bluetooth_settings()
            .map_err(|error| PlatformError::WindowsApi(error.to_string()))
    }

    fn voice_hold_hotkey(&self) -> Option<KeyChord> {
        WindowsPlatform::voice_hold_hotkey(self)
    }

    fn set_voice_hold_hotkey(&self, hotkey: Option<KeyChord>) {
        WindowsPlatform::set_voice_hold_hotkey(self, hotkey)
    }

    fn set_voice_input_tool(&self, tool: Option<VoiceInputTool>) {
        WindowsPlatform::set_voice_input_tool(self, tool)
    }

    fn button_mappings(&self) -> sayall_windows::send_input::ButtonMappings {
        WindowsPlatform::button_mappings(self)
    }

    fn set_button_mappings(&self, mappings: sayall_windows::send_input::ButtonMappings) {
        WindowsPlatform::set_button_mappings(self, mappings)
    }

    fn set_mapping_configuration(
        &self,
        configuration: sayall_windows::templates::MappingConfiguration,
    ) {
        WindowsPlatform::set_mapping_configuration(self, configuration)
    }

    fn scene_snapshot(&self) -> Option<sayall_windows::scene_control::SceneSnapshot> {
        Some(WindowsPlatform::scene_snapshot(self))
    }

    fn set_template_menu_focus(&self, focused: bool) {
        WindowsPlatform::set_template_menu_focus(self, focused);
    }
    fn template_menu_key(
        &self,
        generation: u64,
        button: sayall_windows::raw_input::RemoteButton,
        down: bool,
    ) -> bool {
        WindowsPlatform::template_menu_key(self, generation, button, down)
    }
    fn set_template_menu_update_default(&self, generation: u64, enabled: bool) -> bool {
        self.set_template_menu_update_default(generation, enabled)
    }
    fn complete_template_default_save(&self, request_id: u64, saved: bool) {
        self.complete_template_default_save(request_id, saved);
    }
    fn complete_menu_preference_save(&self, request_id: u64, saved: bool) {
        self.complete_menu_preference_save(request_id, saved);
    }
    fn prepare_template_menu_exit(&self) -> bool {
        WindowsPlatform::prepare_template_menu_exit(self)
    }
    fn restore_template_menu_target(&self) -> bool {
        WindowsPlatform::restore_template_menu_target(self)
    }
    fn template_menu_restore_failed(&self) {
        WindowsPlatform::template_menu_restore_failed(self);
    }

    fn set_mapping_notice_enabled(&self, enabled: bool) {
        WindowsPlatform::set_mapping_notice_enabled(self, enabled)
    }

    fn subscribe_scene_events(&self, callback: sayall_windows::scene_control::SceneEventCallback) {
        WindowsPlatform::subscribe_scene_events(self, callback)
    }

    fn button_mapping_snapshot(&self) -> sayall_windows::button_mapping::ButtonMappingSnapshot {
        WindowsPlatform::button_mapping_snapshot(self)
    }

    fn subscribe_button_edges(&self, callback: sayall_windows::button_mapping::ButtonEdgeCallback) {
        WindowsPlatform::subscribe_button_edges(self, callback)
    }

    fn subscribe_button_gestures(
        &self,
        callback: sayall_windows::button_mapping::ButtonGestureCallback,
    ) {
        WindowsPlatform::subscribe_button_gestures(self, callback)
    }

    fn quiesce_input(&self) -> Result<(), PlatformError> {
        WindowsPlatform::quiesce_input(self)
    }
}

#[cfg(feature = "runtime-simulation")]
mod simulation {
    use super::*;
    use sayall_core::{AtvvCapabilities, AtvvVoicePipeline, PipelineOutput, VoiceSessionState};
    use sayall_windows::raw_input::{RawInputPhase, RemoteButton};
    use sayall_windows::send_input::{plan_key_tap, KeyChord};
    use sayall_windows::{AudioPhase, ConnectionPhase, RemoteModel};
    use std::sync::{Mutex, MutexGuard};

    const RC001_ID: &str = "ci-simulation-rc001";
    const RC003_ID: &str = "ci-simulation-rc003";
    const CABLE_ENDPOINT_ID: &str = "ci-simulation-cable-input";
    const CABLE_ENDPOINT_NAME: &str = "CABLE Input (CI Simulation)";

    #[derive(Debug)]
    struct SimulationState {
        connection: ConnectionSnapshot,
        audio: AudioSnapshot,
        raw_input: RawInputSnapshot,
        send_input: SendInputSnapshot,
    }

    impl Default for SimulationState {
        fn default() -> Self {
            Self {
                connection: ConnectionSnapshot::default(),
                audio: AudioSnapshot::default(),
                raw_input: RawInputSnapshot::default(),
                send_input: SendInputSnapshot {
                    available: true,
                    ..SendInputSnapshot::default()
                },
            }
        }
    }

    #[derive(Debug, Default)]
    pub struct SimulatedPlatform {
        usage: Arc<UsageCounters>,
        state: Mutex<SimulationState>,
        voice_hold_hotkey: Mutex<Option<KeyChord>>,
        voice_input_tool: Mutex<Option<VoiceInputTool>>,
        button_mappings: Mutex<sayall_windows::send_input::ButtonMappings>,
    }

    impl SimulatedPlatform {
        fn paired_remotes() -> Vec<PairedRemote> {
            vec![
                PairedRemote {
                    id: RC001_ID.to_owned(),
                    name: "Xiaomi Bluetooth Remote 2".to_owned(),
                    model: RemoteModel::Rc001,
                    is_supported_candidate: true,
                },
                PairedRemote {
                    id: RC003_ID.to_owned(),
                    name: "Xiaomi Bluetooth Remote 2 Pro".to_owned(),
                    model: RemoteModel::Rc003,
                    is_supported_candidate: true,
                },
            ]
        }

        fn audio_endpoints() -> Vec<AudioEndpoint> {
            vec![AudioEndpoint {
                id: CABLE_ENDPOINT_ID.to_owned(),
                name: CABLE_ENDPOINT_NAME.to_owned(),
                is_virtual_cable_candidate: true,
            }]
        }

        fn capabilities() -> AtvvCapabilities {
            AtvvCapabilities::parse(&[0x0B, 0x01, 0x00, 0x02, 0x03, 0, 120])
                .expect("CI simulation capabilities are valid")
        }

        fn connect(&self, device_id: String) -> Result<ConnectionSnapshot, PlatformError> {
            let remote = Self::paired_remotes()
                .into_iter()
                .find(|remote| remote.id == device_id)
                .ok_or_else(|| PlatformError::Gatt("CI simulation remote is unknown".to_owned()))?;
            let mut state = lock(&self.state);
            state.connection = ConnectionSnapshot {
                phase: ConnectionPhase::Ready,
                remote_name: Some(remote.name),
                remote_model: remote.model,
                capabilities: Some(Self::capabilities()),
                voice_state: VoiceSessionState::Idle,
                generation: state.connection.generation,
                power_notifications_available: true,
                ..ConnectionSnapshot::default()
            };
            Ok(state.connection.clone())
        }
    }

    impl PlatformRuntime for SimulatedPlatform {
        fn usage_counters(&self) -> Arc<UsageCounters> {
            Arc::clone(&self.usage)
        }

        fn snapshot(&self) -> PlatformSnapshot {
            let state = lock(&self.state);
            PlatformSnapshot {
                platform: "windows-ci-simulation".to_owned(),
                windows_api_available: true,
                ble_scan_available: true,
                ble_voice_ready: matches!(
                    state.connection.phase,
                    ConnectionPhase::Ready | ConnectionPhase::Streaming | ConnectionPhase::Draining
                ),
                wasapi_ready: matches!(
                    state.audio.phase,
                    AudioPhase::Ready | AudioPhase::Streaming | AudioPhase::Draining
                ),
                raw_input_ready: state.raw_input.phase == RawInputPhase::Ready,
                send_input_ready: state.send_input.available,
                verification_status:
                    "Windows CI 仿真只验证 Tauri/WebView/IPC 状态闭环，不代表真实 RC001/RC003、BLE、WASAPI 或 Raw Input 已通过"
                        .to_owned(),
                connection: state.connection.clone(),
                audio: state.audio.clone(),
                raw_input: state.raw_input.clone(),
                button_mapping: self.button_mapping_snapshot(),
            }
        }

        fn scan_paired_remotes(&self) -> Result<Vec<PairedRemote>, PlatformError> {
            Ok(Self::paired_remotes())
        }

        fn connection_snapshot(&self) -> ConnectionSnapshot {
            lock(&self.state).connection.clone()
        }

        fn connect_remote(&self, device_id: String) -> Result<ConnectionSnapshot, PlatformError> {
            self.connect(device_id)
        }

        fn disconnect_remote(&self) -> Result<ConnectionSnapshot, PlatformError> {
            let mut state = lock(&self.state);
            state.connection = ConnectionSnapshot {
                phase: ConnectionPhase::Disconnected,
                generation: state.connection.generation,
                power_notifications_available: true,
                ..ConnectionSnapshot::default()
            };
            Ok(state.connection.clone())
        }

        #[cfg(windows)]
        fn restore_remote(&self, device_id: String) -> Result<ConnectionSnapshot, PlatformError> {
            self.connect(device_id)
        }

        fn list_audio_endpoints(&self) -> Result<Vec<AudioEndpoint>, PlatformError> {
            Ok(Self::audio_endpoints())
        }

        fn select_audio_endpoint(
            &self,
            endpoint_id: String,
        ) -> Result<AudioSnapshot, PlatformError> {
            let endpoint = Self::audio_endpoints()
                .into_iter()
                .find(|endpoint| endpoint.id == endpoint_id)
                .ok_or_else(|| {
                    PlatformError::Audio("CI simulation endpoint is unknown".to_owned())
                })?;
            let mut state = lock(&self.state);
            state.audio = AudioSnapshot {
                phase: AudioPhase::Ready,
                selected_endpoint_id: Some(endpoint.id),
                selected_endpoint_name: Some(endpoint.name),
                generation: state.audio.generation,
                ..AudioSnapshot::default()
            };
            Ok(state.audio.clone())
        }

        #[cfg(windows)]
        fn restore_audio_endpoint(
            &self,
            endpoint_id: String,
            expected_name: String,
        ) -> Result<AudioSnapshot, PlatformError> {
            if expected_name != CABLE_ENDPOINT_NAME {
                return Err(PlatformError::Audio(
                    "CI simulation endpoint name changed".to_owned(),
                ));
            }
            self.select_audio_endpoint(endpoint_id)
        }

        fn audio_snapshot(&self) -> AudioSnapshot {
            lock(&self.state).audio.clone()
        }

        fn raw_input_snapshot(&self) -> RawInputSnapshot {
            lock(&self.state).raw_input.clone()
        }

        fn start_raw_input(&self) -> Result<RawInputSnapshot, PlatformError> {
            let mut state = lock(&self.state);
            state.raw_input = RawInputSnapshot {
                phase: RawInputPhase::Ready,
                matched_device_count: 1,
                raw_event_count: 2,
                semantic_edge_count: 2,
                last_button: Some(RemoteButton::Ok),
                last_is_pressed: Some(false),
                active_buttons: Vec::new(),
                last_error: None,
                // 模拟一次全新绑定：尚未出现"报文到了、但路径与当前绑定不符"而被
                // 丢弃的事件，因此陈旧报文计数从 0 起（2026-09-18 新增字段）。
                stale_remote_event_count: 0,
            };
            Ok(state.raw_input.clone())
        }

        fn stop_raw_input(&self) -> Result<RawInputSnapshot, PlatformError> {
            let mut state = lock(&self.state);
            state.raw_input = RawInputSnapshot::default();
            Ok(state.raw_input.clone())
        }

        fn send_input_snapshot(&self) -> SendInputSnapshot {
            lock(&self.state).send_input.clone()
        }

        fn test_shortcut(&self, chord: KeyChord) -> Result<SendInputSnapshot, PlatformError> {
            let planned = plan_key_tap(&chord)
                .map_err(|error| PlatformError::SendInput(error.to_string()))?;
            let mut state = lock(&self.state);
            state.send_input.submitted_batches =
                state.send_input.submitted_batches.saturating_add(1);
            state.send_input.submitted_events = state
                .send_input
                .submitted_events
                .saturating_add(planned.len() as u64);
            state.send_input.last_error = None;
            Ok(state.send_input.clone())
        }

        fn test_scroll(
            &self,
            _direction: ScrollDirection,
            steps: u16,
        ) -> Result<SendInputSnapshot, PlatformError> {
            sayall_windows::send_input::validate_mouse_amount(steps, 100)
                .map_err(|error| PlatformError::SendInput(error.to_string()))?;
            let mut state = lock(&self.state);
            state.send_input.submitted_batches =
                state.send_input.submitted_batches.saturating_add(1);
            state.send_input.submitted_events = state.send_input.submitted_events.saturating_add(1);
            state.send_input.last_error = None;
            Ok(state.send_input.clone())
        }

        fn test_mouse_action(
            &self,
            action: ButtonAction,
        ) -> Result<SendInputSnapshot, PlatformError> {
            let events = match action {
                ButtonAction::MouseClick { kind } => kind.event_count() as u64,
                ButtonAction::MouseMove {
                    direction,
                    distance,
                } => {
                    direction
                        .offset(distance)
                        .map_err(|error| PlatformError::SendInput(error.to_string()))?;
                    1
                }
                _ => return Err(PlatformError::SendInput("unsupported mouse action".into())),
            };
            let mut state = lock(&self.state);
            state.send_input.submitted_batches =
                state.send_input.submitted_batches.saturating_add(1);
            state.send_input.submitted_events =
                state.send_input.submitted_events.saturating_add(events);
            state.send_input.last_error = None;
            Ok(state.send_input.clone())
        }

        fn preset_apps(&self) -> Vec<sayall_windows::app_launcher::PresetAppInfo> {
            // CI 仿真环境：预设表全部标记为可用，验证 UI 渲染路径。
            sayall_windows::app_launcher::PRESET_APPS
                .iter()
                .map(|app| sayall_windows::app_launcher::PresetAppInfo {
                    id: app.id.to_owned(),
                    name: app.name.to_owned(),
                    installed: true,
                })
                .collect()
        }

        fn launch_app(&self, _target: &str) -> Result<(), PlatformError> {
            // 仿真环境不真实启动应用（CI 无桌面会话语义）。
            Ok(())
        }

        fn open_bluetooth_settings(&self) -> Result<(), PlatformError> {
            // CI 仿真只验证固定命令闭环，不打开真实系统设置。
            Ok(())
        }

        fn voice_hold_hotkey(&self) -> Option<KeyChord> {
            lock(&self.voice_hold_hotkey).clone()
        }

        fn set_voice_hold_hotkey(&self, hotkey: Option<KeyChord>) {
            *lock(&self.voice_hold_hotkey) = hotkey;
        }

        fn set_voice_input_tool(&self, tool: Option<VoiceInputTool>) {
            *lock(&self.voice_input_tool) = tool;
        }

        fn button_mappings(&self) -> sayall_windows::send_input::ButtonMappings {
            lock(&self.button_mappings).clone()
        }

        fn set_button_mappings(&self, mappings: sayall_windows::send_input::ButtonMappings) {
            *lock(&self.button_mappings) = mappings;
        }

        fn set_mapping_configuration(
            &self,
            configuration: sayall_windows::templates::MappingConfiguration,
        ) {
            *lock(&self.button_mappings) = configuration.common_mappings;
        }

        fn button_mapping_snapshot(&self) -> sayall_windows::button_mapping::ButtonMappingSnapshot {
            sayall_windows::button_mapping::ButtonMappingSnapshot {
                enabled: lock(&self.button_mappings).enabled,
                gate_active: false,
                observed_buttons: Vec::new(),
                listener_active: false,
                swallowed_edges: 0,
                leaked_downs: 0,
                fired_gestures: 0,
                last_fired: None,
                last_error: None,
            }
        }

        fn scene_snapshot(&self) -> Option<sayall_windows::scene_control::SceneSnapshot> {
            // The CI voice simulation does not simulate foreground accessibility.
            None
        }

        fn set_mapping_notice_enabled(&self, _enabled: bool) {}

        fn subscribe_scene_events(
            &self,
            _callback: sayall_windows::scene_control::SceneEventCallback,
        ) {
        }

        fn subscribe_button_edges(
            &self,
            _callback: sayall_windows::button_mapping::ButtonEdgeCallback,
        ) {
            // CI 仿真不产生真实按键边沿。
        }

        fn subscribe_button_gestures(
            &self,
            _callback: sayall_windows::button_mapping::ButtonGestureCallback,
        ) {
            // CI 仿真不产生真实手势。
        }

        fn quiesce_input(&self) -> Result<(), PlatformError> {
            Ok(())
        }

        fn run_simulated_voice_session(&self) -> Result<PlatformSnapshot, PlatformError> {
            {
                let state = lock(&self.state);
                if state.connection.phase != ConnectionPhase::Ready {
                    return Err(PlatformError::Protocol(
                        "CI simulation remote is not ready".to_owned(),
                    ));
                }
                if state.audio.phase != AudioPhase::Ready {
                    return Err(PlatformError::AudioEndpointNotSelected);
                }
            }

            let mut pipeline = AtvvVoicePipeline::default();
            pipeline
                .handle_control(&[0x0B, 0x01, 0x00, 0x02, 0x03, 0, 120])
                .map_err(|error| PlatformError::Protocol(error.to_string()))?;
            let started = pipeline
                .handle_control(&[0x04, 0x03, 0x02, 0x01])
                .map_err(|error| PlatformError::Protocol(error.to_string()))?;
            let PipelineOutput::StreamStarted { generation, .. } = started else {
                return Err(PlatformError::Protocol(
                    "CI simulation stream did not start".to_owned(),
                ));
            };
            let mut decoded_samples = 0u64;
            for audio in [&[0x11; 40][..], &[0x11; 80][..]] {
                let output = pipeline
                    .handle_audio(audio)
                    .map_err(|error| PlatformError::Protocol(error.to_string()))?;
                let PipelineOutput::Samples { samples, .. } = output else {
                    return Err(PlatformError::Protocol(
                        "CI simulation audio did not decode".to_owned(),
                    ));
                };
                decoded_samples = decoded_samples.saturating_add(samples.len() as u64);
            }
            pipeline
                .handle_control(&[0x00])
                .map_err(|error| PlatformError::Protocol(error.to_string()))?;
            pipeline
                .complete_drain(generation)
                .map_err(|error| PlatformError::Protocol(error.to_string()))?;

            {
                let mut state = lock(&self.state);
                state.connection.phase = ConnectionPhase::Ready;
                state.connection.voice_state = pipeline.state();
                state.connection.decoded_samples = state
                    .connection
                    .decoded_samples
                    .saturating_add(decoded_samples);
                state.connection.generation = generation;
                state.audio.phase = AudioPhase::Ready;
                state.audio.queued_samples = 0;
                state.audio.submitted_samples = state
                    .audio
                    .submitted_samples
                    .saturating_add(decoded_samples);
                state.audio.generation = generation;
            }
            Ok(self.snapshot())
        }
    }

    fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
        mutex
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use sayall_windows::send_input::{KeyChord, KeyCode};

        #[test]
        fn simulation_runs_connection_audio_raw_input_and_send_input_journey() {
            let platform = SimulatedPlatform::default();
            platform.open_bluetooth_settings().unwrap();
            assert_eq!(platform.scan_paired_remotes().unwrap().len(), 2);
            assert_eq!(
                platform
                    .connect_remote(RC001_ID.to_owned())
                    .unwrap()
                    .remote_model,
                RemoteModel::Rc001
            );
            assert_eq!(
                platform
                    .select_audio_endpoint(CABLE_ENDPOINT_ID.to_owned())
                    .unwrap()
                    .phase,
                AudioPhase::Ready
            );
            assert_eq!(
                platform.start_raw_input().unwrap().phase,
                RawInputPhase::Ready
            );
            let send_input = platform
                .test_shortcut(KeyChord {
                    keys: vec![KeyCode::LeftControl, KeyCode::C],
                })
                .unwrap();
            assert_eq!(send_input.submitted_batches, 1);
            assert_eq!(send_input.submitted_events, 4);

            let snapshot = platform.run_simulated_voice_session().unwrap();
            assert_eq!(snapshot.connection.decoded_samples, 240);
            assert_eq!(snapshot.connection.voice_state, VoiceSessionState::Idle);
            assert_eq!(snapshot.audio.submitted_samples, 240);
            assert!(snapshot.ble_voice_ready);
            assert!(snapshot.wasapi_ready);
            assert!(snapshot.raw_input_ready);
            assert!(snapshot.send_input_ready);
        }
    }
}

#[cfg(feature = "runtime-simulation")]
pub use simulation::SimulatedPlatform;
