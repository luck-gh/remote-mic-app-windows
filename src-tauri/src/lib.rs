use sayall_windows::raw_input::{RawInputSnapshot, RemoteButton};
use sayall_windows::send_input::{
    ButtonAction, ButtonMappings, ButtonTrigger, KeyChord, SendInputSnapshot,
};
use sayall_windows::{
    AudioEndpoint, AudioSnapshot, ConnectionSnapshot, PairedRemote, PlatformSnapshot,
    WindowsPlatform,
};
use serde::{Deserialize, Serialize};
use settings::SettingsStore;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Condvar, Mutex, Weak};
use tauri::{Emitter, Manager};

mod diagnostics;
mod platform;
mod settings;
mod updater;

use diagnostics::DiagnosticReport;
use platform::PlatformRuntime;
use sayall_core::ThemePreference;
use updater::{
    check_app_update, get_app_update_preferences, install_app_update, set_app_update_preferences,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeSnapshot {
    /// 应用版本（package_info 同源；String 而非 &'static str——不再依赖编译期常量）。
    app_version: String,
    platform: PlatformSnapshot,
}

#[tauri::command]
fn get_capture_input(
    state: tauri::State<'_, AppState>,
) -> sayall_windows::capture_input::CaptureInputSnapshot {
    state.platform.capture_input_snapshot()
}
#[tauri::command]
async fn list_capture_inputs(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<AudioEndpoint>, String> {
    let platform = state.platform.clone();
    tauri::async_runtime::spawn_blocking(move || platform.list_capture_inputs())
        .await
        .map_err(|_| "输入设备枚举任务失败".to_owned())?
}

#[tauri::command]
async fn set_capture_input(
    state: tauri::State<'_, AppState>,
    config: sayall_core::CaptureInputSettings,
) -> Result<sayall_windows::capture_input::CaptureInputSnapshot, String> {
    let platform = state.platform.clone();
    let settings = state.settings.clone();
    let operation = state.capture_config_operation.clone();
    tauri::async_runtime::spawn_blocking(move || {
        capture_config_transaction(&operation, platform.as_ref(), config, |value| {
            settings.save_capture_input(value)
        })
    })
    .await
    .map_err(|_| "输入设备设置任务失败".to_owned())?
}
fn capture_config_transaction(
    operation: &Mutex<()>,
    platform: &dyn PlatformRuntime,
    config: sayall_core::CaptureInputSettings,
    persist: impl FnOnce(sayall_core::CaptureInputSettings) -> Result<(), String>,
) -> Result<sayall_windows::capture_input::CaptureInputSnapshot, String> {
    let _operation = operation.lock().unwrap_or_else(|e| e.into_inner());
    let previous = platform.capture_input_snapshot().settings;
    match platform.configure_capture_input(config.clone()) {
        Ok(_) => {}
        Err(error) => {
            let rollback = platform.configure_capture_input(previous);
            sayall_windows::gatt_note(format!(
                "capture_input action=config_apply result=failed rollback_ok={}",
                rollback.is_ok()
            ));
            return Err(error);
        }
    };
    if persist(config).is_err() {
        let rollback = platform.configure_capture_input(previous);
        sayall_windows::gatt_note(format!(
            "capture_input action=config_persist result=failed rollback_ok={}",
            rollback.is_ok()
        ));
        return Err("保存输入设备设置失败，请重新检查设置".to_owned());
    }
    Ok(platform.capture_input_snapshot())
}

#[tauri::command]
async fn resolve_capture_recovery(
    state: tauri::State<'_, AppState>,
    restore: bool,
) -> Result<sayall_windows::capture_input::CaptureInputSnapshot, String> {
    let platform = state.platform.clone();
    let operation = state.capture_config_operation.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _operation = operation.lock().unwrap_or_else(|e| e.into_inner());
        platform.resolve_capture_recovery(restore)
    })
    .await
    .map_err(|_| "输入设备恢复任务失败".to_owned())?
}

struct AppState {
    capture_config_operation: Arc<Mutex<()>>,
    platform: Arc<dyn PlatformRuntime>,
    exit_cleanup: ExitCleanup,
    settings: SettingsStore,
    /// check_app_update 暂存的待安装更新（install_app_update 取走）。
    /// tauri_plugin_updater::Update 未实现 Debug，用手写 impl 只呈现存在性。
    pending_update: std::sync::Mutex<Option<tauri_plugin_updater::Update>>,
}

#[derive(Debug, Clone, Copy)]
enum ExitCleanupPhase {
    Idle,
    Running,
    Finished { clean: bool },
}

struct RawInputSupervisorWorker {
    handle: std::thread::JoinHandle<()>,
    stopped: mpsc::Receiver<()>,
}

struct RawInputSupervisor {
    stop: Arc<(Mutex<bool>, Condvar)>,
    worker: Mutex<Option<RawInputSupervisorWorker>>,
    spawn_failed: bool,
}

impl RawInputSupervisor {
    fn stop(&self) -> Result<(), &'static str> {
        {
            let (lock, wake) = self.stop.as_ref();
            *lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = true;
            wake.notify_all();
        }
        let Some(worker) = self
            .worker
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
        else {
            return if self.spawn_failed {
                Err("worker_spawn_failed")
            } else {
                Ok(())
            };
        };
        match worker
            .stopped
            .recv_timeout(raw_input_supervisor_stop_bound())
        {
            Ok(()) => worker.handle.join().map_err(|_| "worker_panicked"),
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                worker.handle.join().map_err(|_| "worker_panicked")
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // Dropping a JoinHandle detaches it. The worker only owns a
                // Weak platform reference and every start attempt is
                // independently bounded, so a late worker cannot keep the
                // platform alive or begin another iteration after stop.
                drop(worker.handle);
                Err("worker_stop_timeout")
            }
        }
    }
}

struct ExitCleanupInner {
    platform: Arc<dyn PlatformRuntime>,
    supervisor: RawInputSupervisor,
    phase: Mutex<ExitCleanupPhase>,
    phase_changed: Condvar,
    exit_worker_started: AtomicBool,
}

#[derive(Clone)]
pub(crate) struct ExitCleanup(Arc<ExitCleanupInner>);

impl ExitCleanup {
    fn new(platform: Arc<dyn PlatformRuntime>, supervisor: RawInputSupervisor) -> Self {
        Self(Arc::new(ExitCleanupInner {
            platform,
            supervisor,
            phase: Mutex::new(ExitCleanupPhase::Idle),
            phase_changed: Condvar::new(),
            exit_worker_started: AtomicBool::new(false),
        }))
    }

    fn is_finished(&self) -> bool {
        matches!(
            *self
                .0
                .phase
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
            ExitCleanupPhase::Finished { .. }
        )
    }

    fn begin_exit_worker(&self) -> bool {
        !self.is_finished()
            && self
                .0
                .exit_worker_started
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
    }

    fn reset_exit_worker(&self) {
        self.0.exit_worker_started.store(false, Ordering::Release);
    }

    pub(crate) fn shutdown_blocking(&self) -> bool {
        let mut phase = self
            .0
            .phase
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        loop {
            match *phase {
                ExitCleanupPhase::Finished { clean } => return clean,
                ExitCleanupPhase::Idle => {
                    *phase = ExitCleanupPhase::Running;
                    break;
                }
                ExitCleanupPhase::Running => {
                    let waited = self
                        .0
                        .phase_changed
                        .wait_timeout(phase, std::time::Duration::from_secs(45))
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    phase = waited.0;
                    if waited.1.timed_out() {
                        sayall_windows::gatt_note(
                            "app_shutdown stage=wait_for_owner phase=completed terminal_result=failed error_domain=lifecycle error_code=cleanup_wait_timeout reason=owner_not_completed retryable=true took_ms=45000".to_owned(),
                        );
                        return false;
                    }
                }
            }
        }
        drop(phase);

        let overall_started = std::time::Instant::now();
        let mut failures = 0u8;
        sayall_windows::gatt_note(
            "app_shutdown stage=overall phase=started terminal_result=pending".to_owned(),
        );

        let started = std::time::Instant::now();
        sayall_windows::gatt_note(
            "app_shutdown stage=supervisor_stop phase=started terminal_result=pending".to_owned(),
        );
        let supervisor_result = self.0.supervisor.stop();
        failures += u8::from(supervisor_result.is_err());
        sayall_windows::gatt_note(format!(
            "app_shutdown stage=supervisor_stop phase=completed terminal_result={} error_code={} took_ms={}",
            if supervisor_result.is_ok() {
                "passed"
            } else {
                "failed"
            },
            supervisor_result.err().unwrap_or("none"),
            started.elapsed().as_millis()
        ));

        let started = std::time::Instant::now();
        sayall_windows::gatt_note(
            "app_shutdown stage=input_quiesce phase=started terminal_result=pending".to_owned(),
        );
        let input_result = self.0.platform.quiesce_input();
        failures += u8::from(input_result.is_err());
        sayall_windows::gatt_note(format!(
            "app_shutdown stage=input_quiesce phase=completed terminal_result={} error_code={} took_ms={}",
            if input_result.is_ok() {
                "passed"
            } else {
                "failed"
            },
            if input_result.is_ok() {
                "none"
            } else {
                "barrier_failed"
            },
            started.elapsed().as_millis()
        ));

        let started = std::time::Instant::now();
        sayall_windows::gatt_note(
            "app_shutdown stage=raw_input_stop phase=started terminal_result=pending".to_owned(),
        );
        let raw_input_result = self.0.platform.stop_raw_input();
        failures += u8::from(raw_input_result.is_err());
        sayall_windows::gatt_note(format!(
            "app_shutdown stage=raw_input_stop phase=completed terminal_result={} error_code={} took_ms={}",
            if raw_input_result.is_ok() {
                "passed"
            } else {
                "failed"
            },
            if raw_input_result.is_ok() {
                "none"
            } else {
                "stop_failed"
            },
            started.elapsed().as_millis()
        ));

        let started = std::time::Instant::now();
        sayall_windows::gatt_note(
            "app_shutdown stage=ble_disconnect phase=started terminal_result=pending".to_owned(),
        );
        let disconnect_result = self.0.platform.disconnect_remote();
        failures += u8::from(disconnect_result.is_err());
        sayall_windows::gatt_note(format!(
            "app_shutdown stage=ble_disconnect phase=completed terminal_result={} error_code={} took_ms={}",
            if disconnect_result.is_ok() {
                "passed"
            } else {
                "failed"
            },
            if disconnect_result.is_ok() {
                "none"
            } else {
                "disconnect_failed"
            },
            started.elapsed().as_millis()
        ));

        // BLE owns hotkey UP and audio interruption; never restore capture first.
        if disconnect_result.is_ok() {
            let route_result = self.0.platform.shutdown_capture_input();
            failures += u8::from(route_result.is_err());
            sayall_windows::gatt_note(format!(
                "app_shutdown stage=capture_route result={}",
                route_result
                    .as_ref()
                    .map(|_| "passed")
                    .unwrap_or_else(|e| e.as_str())
            ));
        }
        let clean = failures == 0;
        sayall_windows::gatt_note(format!(
            "app_shutdown stage=overall phase=completed terminal_result={} failed_stages={} took_ms={}",
            if clean { "passed" } else { "failed" },
            failures,
            overall_started.elapsed().as_millis()
        ));
        let mut phase = self
            .0
            .phase
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *phase = ExitCleanupPhase::Finished { clean };
        self.0.phase_changed.notify_all();
        clean
    }
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AppState")
            .field("platform", &self.platform)
            .field("settings", &self.settings)
            .field(
                "pending_update",
                &if self
                    .pending_update
                    .lock()
                    .map(|u| u.is_some())
                    .unwrap_or(false)
                {
                    "Some"
                } else {
                    "None"
                },
            )
            .finish()
    }
}

#[tauri::command]
fn get_runtime_snapshot(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> RuntimeSnapshot {
    RuntimeSnapshot {
        // 版本统一取 package_info（tauri.conf.json 的 version，与安装包/更新器
        // 比较同源）。此前用编译期 CARGO_PKG_VERSION（Cargo.toml），两者在
        // "--config 覆盖版本"的本地构建/预发布场景会漂移（2026-09-06 实证：
        // 安装 0.2.0 构建而关于页显示 0.1.0）。
        app_version: app.package_info().version.to_string(),
        platform: state.platform.snapshot(),
    }
}

#[tauri::command]
fn get_diagnostic_report(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> DiagnosticReport {
    let platform = state.platform.snapshot();
    let send_input = state.platform.send_input_snapshot();
    DiagnosticReport::capture(
        &app.package_info().version.to_string(),
        &platform,
        &send_input,
    )
}

#[tauri::command]
async fn scan_paired_remotes(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<PairedRemote>, String> {
    let platform = Arc::clone(&state.platform);
    tauri::async_runtime::spawn_blocking(move || platform.scan_paired_remotes())
        .await
        .map_err(|error| format!("扫描任务失败：{error}"))?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn get_connection_snapshot(
    state: tauri::State<'_, AppState>,
) -> Result<ConnectionSnapshot, String> {
    let platform = Arc::clone(&state.platform);
    tauri::async_runtime::spawn_blocking(move || platform.connection_snapshot())
        .await
        .map_err(|error| format!("读取连接状态失败：{error}"))
}

#[tauri::command]
async fn connect_remote(
    device_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<ConnectionSnapshot, String> {
    let platform = Arc::clone(&state.platform);
    let settings = state.settings.clone();
    tauri::async_runtime::spawn_blocking(move || {
        settings.save_selected_remote_id(device_id.clone())?;
        platform
            .connect_remote(device_id)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("连接任务失败：{error}"))?
}

#[tauri::command]
async fn disconnect_remote(
    state: tauri::State<'_, AppState>,
) -> Result<ConnectionSnapshot, String> {
    let platform = Arc::clone(&state.platform);
    tauri::async_runtime::spawn_blocking(move || {
        platform
            .disconnect_remote()
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("断开任务失败：{error}"))?
}

#[tauri::command]
async fn open_bluetooth_settings(state: tauri::State<'_, AppState>) -> Result<(), String> {
    sayall_windows::gatt_note(
        "bluetooth_settings action=open phase=started target=fixed_public_uri".to_owned(),
    );
    let platform = Arc::clone(&state.platform);
    let result = tauri::async_runtime::spawn_blocking(move || platform.open_bluetooth_settings())
        .await
        .map_err(|error| format!("打开 Windows 蓝牙设置任务失败：{error}"))?
        .map_err(|error| error.to_string());
    sayall_windows::gatt_note(match &result {
        Ok(()) => {
            "bluetooth_settings action=open phase=completed terminal_result=passed target=fixed_public_uri"
                .to_owned()
        }
        Err(_) => "bluetooth_settings action=open phase=completed terminal_result=failed error_domain=platform error_code=shell_open_failed reason=windows_settings_unavailable retryable=true".to_owned(),
    });
    result
}

#[tauri::command]
async fn list_audio_endpoints(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<AudioEndpoint>, String> {
    let platform = Arc::clone(&state.platform);
    tauri::async_runtime::spawn_blocking(move || platform.list_audio_endpoints())
        .await
        .map_err(|error| format!("枚举音频端点任务失败：{error}"))?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn get_audio_snapshot(state: tauri::State<'_, AppState>) -> Result<AudioSnapshot, String> {
    let platform = Arc::clone(&state.platform);
    tauri::async_runtime::spawn_blocking(move || platform.audio_snapshot())
        .await
        .map_err(|error| format!("读取音频状态失败：{error}"))
}

#[tauri::command]
async fn select_audio_endpoint(
    endpoint_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<AudioSnapshot, String> {
    let platform = Arc::clone(&state.platform);
    let settings = state.settings.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let snapshot = platform
            .select_audio_endpoint(endpoint_id)
            .map_err(|error| error.to_string())?;
        let (Some(id), Some(name)) = (
            snapshot.selected_endpoint_id.clone(),
            snapshot.selected_endpoint_name.clone(),
        ) else {
            return Err("WASAPI 已初始化，但未返回所选端点身份".to_owned());
        };
        settings.save_audio_endpoint(id, name)?;
        Ok(snapshot)
    })
    .await
    .map_err(|error| format!("选择音频端点任务失败：{error}"))?
}

#[tauri::command]
async fn get_raw_input_snapshot(
    state: tauri::State<'_, AppState>,
) -> Result<RawInputSnapshot, String> {
    let platform = Arc::clone(&state.platform);
    tauri::async_runtime::spawn_blocking(move || platform.raw_input_snapshot())
        .await
        .map_err(|error| format!("读取 Raw Input 状态失败：{error}"))
}

#[tauri::command]
async fn start_raw_input(state: tauri::State<'_, AppState>) -> Result<RawInputSnapshot, String> {
    let platform = Arc::clone(&state.platform);
    tauri::async_runtime::spawn_blocking(move || platform.start_raw_input())
        .await
        .map_err(|error| format!("启动 Raw Input 任务失败：{error}"))?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn stop_raw_input(state: tauri::State<'_, AppState>) -> Result<RawInputSnapshot, String> {
    let platform = Arc::clone(&state.platform);
    tauri::async_runtime::spawn_blocking(move || platform.stop_raw_input())
        .await
        .map_err(|error| format!("停止 Raw Input 任务失败：{error}"))?
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn get_button_mappings(state: tauri::State<'_, AppState>) -> ButtonMappings {
    state.platform.button_mappings()
}

#[tauri::command]
fn get_mapping_configuration(
    state: tauri::State<'_, AppState>,
) -> Result<sayall_windows::templates::MappingConfiguration, String> {
    state.settings.load_mapping_configuration()
}

fn apply_mapping_configuration(
    platform: &dyn platform::PlatformRuntime,
    configuration: &sayall_windows::templates::MappingConfiguration,
) {
    platform.set_mapping_configuration(configuration.clone());
}

#[tauri::command]
async fn preview_mapping_configuration_import(
    state: tauri::State<'_, AppState>,
) -> Result<Option<sayall_windows::templates::MappingConfigurationImportPreview>, String> {
    let settings = state.settings.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let Some(path) = sayall_windows::file_dialog::pick_button_mapping_import_path()? else {
            return Ok(None);
        };
        settings
            .preview_mapping_configuration_import(&path)
            .map(Some)
    })
    .await
    .map_err(|error| format!("预览模板导入任务失败：{error}"))?
}

#[tauri::command]
async fn apply_mapping_configuration_import(
    token: String,
    state: tauri::State<'_, AppState>,
) -> Result<sayall_windows::templates::MappingConfiguration, String> {
    let settings = state.settings.clone();
    let platform = Arc::clone(&state.platform);
    tauri::async_runtime::spawn_blocking(move || {
        settings.apply_mapping_configuration_import_with(&token, |saved| {
            apply_mapping_configuration(platform.as_ref(), saved);
        })
    })
    .await
    .map_err(|error| format!("应用模板导入任务失败：{error}"))?
}

#[tauri::command]
async fn export_mapping_configuration(
    template_ids: Option<Vec<String>>,
    state: tauri::State<'_, AppState>,
) -> Result<bool, String> {
    let settings = state.settings.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let Some(path) = sayall_windows::file_dialog::pick_button_mapping_export_path()? else {
            return Ok(false);
        };
        let configuration = settings.load_mapping_configuration()?;
        settings.export_mapping_configuration(&path, configuration, template_ids.as_deref())?;
        Ok(true)
    })
    .await
    .map_err(|error| format!("导出模板配置任务失败：{error}"))?
}

#[tauri::command]
async fn save_mapping_configuration(
    configuration: sayall_windows::templates::MappingConfiguration,
    state: tauri::State<'_, AppState>,
) -> Result<sayall_windows::templates::MappingConfiguration, String> {
    let settings = state.settings.clone();
    let platform = Arc::clone(&state.platform);
    tauri::async_runtime::spawn_blocking(move || {
        settings.save_mapping_configuration_with(configuration, |saved| {
            apply_mapping_configuration(platform.as_ref(), saved);
        })
    })
    .await
    .map_err(|error| format!("保存模板配置任务失败：{error}"))?
}

#[tauri::command]
fn get_mapping_template_presets() -> Vec<sayall_windows::templates::MappingTemplate> {
    sayall_windows::templates::MappingConfiguration::recommended_templates()
}

#[tauri::command]
fn get_scene_snapshot(
    state: tauri::State<'_, AppState>,
) -> Option<sayall_windows::scene_control::SceneSnapshot> {
    state.platform.scene_snapshot()
}

#[tauri::command]
fn get_component_status() -> Vec<sayall_windows::component_support::ComponentStatus> {
    sayall_windows::component_support::inspect_components()
}

#[tauri::command]
async fn perform_component_action(
    component: sayall_windows::component_support::ComponentKind,
    action: sayall_windows::component_support::ComponentAction,
) -> Result<sayall_windows::component_support::ComponentOperation, String> {
    tauri::async_runtime::spawn_blocking(move || {
        sayall_windows::component_support::perform_component_action(component, action)
    })
    .await
    .map_err(|error| format!("组件操作工作线程失败：{error}"))
}

#[tauri::command]
async fn apply_mapping_template_preset(
    preset_id: String,
    name: String,
    state: tauri::State<'_, AppState>,
) -> Result<sayall_windows::templates::MappingTemplate, String> {
    let settings = state.settings.clone();
    let platform = Arc::clone(&state.platform);
    tauri::async_runtime::spawn_blocking(move || {
        let (_, template) = settings.update_mapping_configuration(
            |configuration| configuration.apply_template_preset(&preset_id, name),
            |saved| apply_mapping_configuration(platform.as_ref(), saved),
        )?;
        Ok(template)
    })
    .await
    .map_err(|error| format!("应用推荐模板任务失败：{error}"))?
}

#[tauri::command]
async fn preview_template_import(
    source_token: String,
    request: sayall_windows::templates::TemplateImportRequest,
    state: tauri::State<'_, AppState>,
) -> Result<sayall_windows::templates::TemplateImportPreview, String> {
    let settings = state.settings.clone();
    tauri::async_runtime::spawn_blocking(move || {
        settings.preview_template_import(&source_token, request)
    })
    .await
    .map_err(|error| format!("预览选中模板任务失败：{error}"))?
}

#[tauri::command]
async fn create_mapping_template(
    name: String,
    state: tauri::State<'_, AppState>,
) -> Result<sayall_windows::templates::MappingTemplate, String> {
    let settings = state.settings.clone();
    let platform = Arc::clone(&state.platform);
    tauri::async_runtime::spawn_blocking(move || {
        let (_, template) = settings.update_mapping_configuration(
            |configuration| configuration.create_template(name),
            |saved| apply_mapping_configuration(platform.as_ref(), saved),
        )?;
        Ok(template)
    })
    .await
    .map_err(|error| format!("新建模板任务失败：{error}"))?
}

#[tauri::command]
async fn duplicate_mapping_template(
    template_id: String,
    name: String,
    state: tauri::State<'_, AppState>,
) -> Result<sayall_windows::templates::MappingTemplate, String> {
    let settings = state.settings.clone();
    let platform = Arc::clone(&state.platform);
    tauri::async_runtime::spawn_blocking(move || {
        let (_, template) = settings.update_mapping_configuration(
            |configuration| configuration.duplicate_template(&template_id, name),
            |saved| apply_mapping_configuration(platform.as_ref(), saved),
        )?;
        Ok(template)
    })
    .await
    .map_err(|error| format!("复制模板任务失败：{error}"))?
}

#[tauri::command]
async fn rename_mapping_template(
    template_id: String,
    name: String,
    state: tauri::State<'_, AppState>,
) -> Result<sayall_windows::templates::MappingConfiguration, String> {
    let settings = state.settings.clone();
    let platform = Arc::clone(&state.platform);
    tauri::async_runtime::spawn_blocking(move || {
        settings
            .update_mapping_configuration(
                |configuration| {
                    let template = configuration
                        .templates
                        .iter_mut()
                        .find(|template| template.id == template_id)
                        .ok_or_else(|| "模板不存在".to_owned())?;
                    template.name = name;
                    Ok(())
                },
                |saved| apply_mapping_configuration(platform.as_ref(), saved),
            )
            .map(|(saved, ())| saved)
    })
    .await
    .map_err(|error| format!("重命名模板任务失败：{error}"))?
}

#[tauri::command]
async fn delete_mapping_template(
    template_id: String,
    replacement_template_id: Option<String>,
    unbind_applications: Option<bool>,
    state: tauri::State<'_, AppState>,
) -> Result<sayall_windows::templates::MappingConfiguration, String> {
    let settings = state.settings.clone();
    let platform = Arc::clone(&state.platform);
    tauri::async_runtime::spawn_blocking(move || {
        settings
            .update_mapping_configuration(
                |configuration| {
                    if !configuration
                        .templates
                        .iter()
                        .any(|template| template.id == template_id)
                    {
                        return Err("模板不存在".to_owned());
                    }
                    let bound = configuration
                        .application_bindings
                        .iter()
                        .any(|binding| binding.template_id == template_id);
                    if unbind_applications.unwrap_or(false) {
                        if replacement_template_id.is_some() {
                            return Err("重新绑定与解除绑定只能选择一项".to_owned());
                        }
                        configuration
                            .application_bindings
                            .retain(|binding| binding.template_id != template_id);
                    } else if bound {
                        let replacement = replacement_template_id.ok_or_else(|| {
                            "模板仍有应用绑定，须重新绑定或明确解除绑定".to_owned()
                        })?;
                        if replacement == template_id
                            || !configuration
                                .templates
                                .iter()
                                .any(|template| template.id == replacement)
                        {
                            return Err("替换模板不存在".to_owned());
                        }
                        for binding in &mut configuration.application_bindings {
                            if binding.template_id == template_id {
                                binding.template_id = replacement.clone();
                            }
                        }
                    }
                    configuration
                        .templates
                        .retain(|template| template.id != template_id);
                    Ok(())
                },
                |saved| apply_mapping_configuration(platform.as_ref(), saved),
            )
            .map(|(saved, ())| saved)
    })
    .await
    .map_err(|error| format!("删除模板任务失败：{error}"))?
}

#[tauri::command]
async fn upsert_application_binding(
    binding: sayall_windows::templates::ApplicationBinding,
    state: tauri::State<'_, AppState>,
) -> Result<sayall_windows::templates::MappingConfiguration, String> {
    let settings = state.settings.clone();
    let platform = Arc::clone(&state.platform);
    tauri::async_runtime::spawn_blocking(move || {
        settings
            .update_mapping_configuration(
                |configuration| {
                    configuration
                        .application_bindings
                        .retain(|item| item.application_id != binding.application_id);
                    configuration.application_bindings.push(binding);
                    Ok(())
                },
                |saved| apply_mapping_configuration(platform.as_ref(), saved),
            )
            .map(|(saved, ())| saved)
    })
    .await
    .map_err(|error| format!("保存应用绑定任务失败：{error}"))?
}

#[tauri::command]
async fn remove_application_binding(
    application_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<sayall_windows::templates::MappingConfiguration, String> {
    let settings = state.settings.clone();
    let platform = Arc::clone(&state.platform);
    tauri::async_runtime::spawn_blocking(move || {
        settings
            .update_mapping_configuration(
                |configuration| {
                    configuration
                        .application_bindings
                        .retain(|binding| binding.application_id != application_id);
                    Ok(())
                },
                |saved| apply_mapping_configuration(platform.as_ref(), saved),
            )
            .map(|(saved, ())| saved)
    })
    .await
    .map_err(|error| format!("解除应用绑定任务失败：{error}"))?
}
#[tauri::command]
async fn save_button_mappings(
    mappings: ButtonMappings,
    state: tauri::State<'_, AppState>,
) -> Result<ButtonMappings, String> {
    let started = std::time::Instant::now();
    let summary = button_mapping_log_summary(&mappings);
    sayall_windows::gatt_note(format!(
        "shortcut_settings feature=button_mapping action=save phase=requested {summary}"
    ));
    let settings = state.settings.clone();
    let platform = Arc::clone(&state.platform);
    let result =
        match tauri::async_runtime::spawn_blocking(move || -> Result<ButtonMappings, String> {
            settings.save_button_mappings_with(mappings, |saved| {
                apply_mapping_configuration(platform.as_ref(), saved);
            })
        })
        .await
        {
            Ok(result) => result,
            Err(error) => Err(format!("保存按键映射任务失败：{error}")),
        };
    sayall_windows::gatt_note(match &result {
        Ok(saved) => format!(
            "shortcut_settings feature=button_mapping action=save phase=completed terminal_result=passed {} elapsed_ms={}",
            button_mapping_log_summary(saved),
            started.elapsed().as_millis()
        ),
        Err(_) => format!(
            "shortcut_settings feature=button_mapping action=save phase=completed terminal_result=failed error_domain=settings error_code=save_failed reason=validation_or_persistence_failed retryable=true elapsed_ms={}",
            started.elapsed().as_millis()
        ),
    });
    result
}

#[tauri::command]
async fn reset_button_mappings(
    state: tauri::State<'_, AppState>,
) -> Result<ButtonMappings, String> {
    let started = std::time::Instant::now();
    sayall_windows::gatt_note(
        "shortcut_settings feature=button_mapping action=reset phase=requested".to_owned(),
    );
    let settings = state.settings.clone();
    let platform = Arc::clone(&state.platform);
    let result =
        match tauri::async_runtime::spawn_blocking(move || -> Result<ButtonMappings, String> {
            settings.save_button_mappings_with(ButtonMappings::default(), |saved| {
                apply_mapping_configuration(platform.as_ref(), saved);
            })
        })
        .await
        {
            Ok(result) => result,
            Err(error) => Err(format!("恢复默认按键映射任务失败：{error}")),
        };
    sayall_windows::gatt_note(match &result {
        Ok(saved) => format!(
            "shortcut_settings feature=button_mapping action=reset phase=completed terminal_result=passed {} elapsed_ms={}",
            button_mapping_log_summary(saved),
            started.elapsed().as_millis()
        ),
        Err(_) => format!(
            "shortcut_settings feature=button_mapping action=reset phase=completed terminal_result=failed error_domain=settings error_code=save_failed reason=defaults_persistence_failed retryable=true elapsed_ms={}",
            started.elapsed().as_millis()
        ),
    });
    result
}

fn button_mapping_log_summary(mappings: &ButtonMappings) -> String {
    let mut shortcut_count = 0_usize;
    let mut open_app_count = 0_usize;
    let mut disabled_count = 0_usize;
    for actions in mappings.actions.values() {
        for action in [&actions.single, &actions.double, &actions.long] {
            match action {
                ButtonAction::Shortcut { .. } => shortcut_count += 1,
                ButtonAction::OpenApp { .. } => open_app_count += 1,
                ButtonAction::Disabled => disabled_count += 1,
            }
        }
    }
    format!(
        "enabled={} button_count={} shortcut_count={shortcut_count} open_app_count={open_app_count} disabled_cell_count={disabled_count}",
        mappings.enabled,
        mappings.actions.len()
    )
}

#[tauri::command]
async fn test_button_mapping(
    button: RemoteButton,
    trigger: ButtonTrigger,
    state: tauri::State<'_, AppState>,
) -> Result<SendInputSnapshot, String> {
    let action = state.platform.button_mappings().action_for(button, trigger);
    let platform = Arc::clone(&state.platform);
    match action {
        ButtonAction::Shortcut { chord } => {
            tauri::async_runtime::spawn_blocking(move || platform.test_shortcut(chord))
                .await
                .map_err(|error| format!("测试快捷键任务失败：{error}"))?
                .map_err(|error| error.to_string())
        }
        ButtonAction::OpenApp { target } => tauri::async_runtime::spawn_blocking(move || {
            platform
                .launch_app(&target)
                .map(|_| SendInputSnapshot::default())
        })
        .await
        .map_err(|error| format!("测试打开应用任务失败：{error}"))?
        .map_err(|error| error.to_string()),
        ButtonAction::Disabled => Err("该触发方式当前未配置动作".to_owned()),
    }
}

#[tauri::command]
fn list_preset_apps(
    state: tauri::State<'_, AppState>,
) -> Vec<sayall_windows::app_launcher::PresetAppInfo> {
    state.platform.preset_apps()
}

/// 原生文件选择器：选择自定义应用（.exe/.lnk）。用户取消返回 null。
#[tauri::command]
fn pick_custom_app() -> Option<sayall_windows::app_launcher::CustomAppPick> {
    sayall_windows::app_launcher::pick_custom_app()
}

#[tauri::command]
fn get_button_mapping_snapshot(
    state: tauri::State<'_, AppState>,
) -> sayall_windows::button_mapping::ButtonMappingSnapshot {
    state.platform.button_mapping_snapshot()
}

#[tauri::command]
fn get_send_input_snapshot(state: tauri::State<'_, AppState>) -> SendInputSnapshot {
    state.platform.send_input_snapshot()
}

#[tauri::command]
fn get_voice_hold_hotkey(state: tauri::State<'_, AppState>) -> Option<KeyChord> {
    let hotkey = state.platform.voice_hold_hotkey();
    sayall_windows::gatt_note(format!(
        "shortcut_settings feature=voice_hold action=load phase=completed terminal_result=passed enabled={} key_count={}",
        hotkey.is_some(),
        hotkey.as_ref().map(|chord| chord.keys.len()).unwrap_or(0)
    ));
    hotkey
}

#[tauri::command]
async fn set_voice_hold_hotkey(
    hotkey: Option<KeyChord>,
    state: tauri::State<'_, AppState>,
) -> Result<Option<KeyChord>, String> {
    let started = std::time::Instant::now();
    let enabled = hotkey.is_some();
    let key_count = hotkey.as_ref().map(|chord| chord.keys.len()).unwrap_or(0);
    sayall_windows::gatt_note(format!(
        "shortcut_settings feature=voice_hold action=save phase=requested enabled={enabled} key_count={key_count}"
    ));
    let platform = Arc::clone(&state.platform);
    let settings = state.settings.clone();
    let result = match tauri::async_runtime::spawn_blocking(move || {
        let saved = settings.save_voice_hold_hotkey(hotkey)?;
        platform.set_voice_hold_hotkey(saved.clone());
        Ok(saved)
    })
    .await
    {
        Ok(result) => result,
        Err(error) => Err(format!("保存按住说话快捷键任务失败：{error}")),
    };
    sayall_windows::gatt_note(match &result {
        Ok(_) => format!(
            "shortcut_settings feature=voice_hold action=save phase=completed terminal_result=passed enabled={enabled} key_count={key_count} elapsed_ms={}",
            started.elapsed().as_millis()
        ),
        Err(_) => format!(
            "shortcut_settings feature=voice_hold action=save phase=completed terminal_result=failed enabled={enabled} key_count={key_count} error_domain=settings error_code=save_failed reason=validation_or_persistence_failed retryable=true elapsed_ms={}",
            started.elapsed().as_millis()
        ),
    });
    result
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FrontendDiagnosticEvent {
    event: String,
    phase: String,
    result: String,
    reason: String,
    elapsed_ms: u64,
}

#[tauri::command]
fn report_frontend_event(report: FrontendDiagnosticEvent) {
    sayall_windows::gatt_note(format!(
        "frontend event={} phase={} result={} reason={} elapsed_ms={}",
        diagnostic_token(&report.event),
        diagnostic_token(&report.phase),
        diagnostic_token(&report.result),
        diagnostic_token(&report.reason),
        report.elapsed_ms
    ));
}

fn diagnostic_token(value: &str) -> &str {
    if !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
    {
        value
    } else {
        "invalid"
    }
}

#[tauri::command]
async fn get_theme_preference(
    operation_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<ThemePreference, String> {
    let settings = state.settings.clone();
    let started = std::time::Instant::now();
    let operation_id = sanitized_theme_operation_id(&operation_id);
    sayall_windows::gatt_note(format!(
        "theme_preference operation_id={operation_id} action=load phase=requested"
    ));
    let result = match tauri::async_runtime::spawn_blocking(move || {
        settings.load().map(|settings| settings.theme_preference)
    })
    .await
    {
        Ok(result) => result,
        Err(error) => Err(format!("读取外观设置任务失败：{error}")),
    };
    match &result {
        Ok(preference) => sayall_windows::gatt_note(format!(
            "theme_preference operation_id={operation_id} action=load phase=persisted result=passed preference={} elapsed_ms={}",
            theme_preference_name(*preference),
            started.elapsed().as_millis()
        )),
        Err(_) => sayall_windows::gatt_note(format!(
            "theme_preference operation_id={operation_id} action=load phase=persisted result=failed error_domain=settings error_code=load_failed reason=settings_load_failed retryable=true elapsed_ms={}",
            started.elapsed().as_millis()
        )),
    }
    result
}

#[tauri::command]
async fn set_theme_preference(
    preference: ThemePreference,
    operation_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<ThemePreference, String> {
    let settings = state.settings.clone();
    let started = std::time::Instant::now();
    let operation_id = sanitized_theme_operation_id(&operation_id);
    sayall_windows::gatt_note(format!(
        "theme_preference operation_id={operation_id} action=save phase=requested preference={}",
        theme_preference_name(preference)
    ));
    let result = match tauri::async_runtime::spawn_blocking(move || {
        settings.save_theme_preference(preference)?;
        Ok(preference)
    })
    .await
    {
        Ok(result) => result,
        Err(error) => Err(format!("保存外观设置任务失败：{error}")),
    };
    match &result {
        Ok(saved) => sayall_windows::gatt_note(format!(
            "theme_preference operation_id={operation_id} action=save phase=persisted result=passed preference={} elapsed_ms={}",
            theme_preference_name(*saved),
            started.elapsed().as_millis()
        )),
        Err(_) => sayall_windows::gatt_note(format!(
            "theme_preference operation_id={operation_id} action=save phase=persisted result=failed preference={} error_domain=settings error_code=save_failed reason=settings_save_failed retryable=true elapsed_ms={}",
            theme_preference_name(preference),
            started.elapsed().as_millis()
        )),
    }
    result
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ThemeAction {
    Initialize,
    Change,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum EffectiveTheme {
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ThemeTerminalResult {
    Passed,
    Failed,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ThemeResultReason {
    Applied,
    PreferenceLoadFailed,
    NativeApplyFailed,
    ApplyOrSaveFailed,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ThemeResultReport {
    operation_id: String,
    action: ThemeAction,
    preference: ThemePreference,
    resolved_theme: EffectiveTheme,
    terminal_result: ThemeTerminalResult,
    reason: ThemeResultReason,
    elapsed_ms: u64,
}

#[tauri::command]
fn report_theme_result(report: ThemeResultReport) {
    sayall_windows::gatt_note(format!(
        "theme_preference operation_id={} action={} phase=completed preference={} resolved={} terminal_result={} reason={} elapsed_ms={}",
        sanitized_theme_operation_id(&report.operation_id),
        theme_action_name(report.action),
        theme_preference_name(report.preference),
        effective_theme_name(report.resolved_theme),
        theme_terminal_result_name(report.terminal_result),
        theme_result_reason_name(report.reason),
        report.elapsed_ms
    ));
}

fn sanitized_theme_operation_id(operation_id: &str) -> &str {
    if !operation_id.is_empty()
        && operation_id.len() <= 48
        && operation_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        operation_id
    } else {
        "invalid"
    }
}

fn theme_action_name(action: ThemeAction) -> &'static str {
    match action {
        ThemeAction::Initialize => "initialize",
        ThemeAction::Change => "change",
    }
}

fn effective_theme_name(theme: EffectiveTheme) -> &'static str {
    match theme {
        EffectiveTheme::Light => "light",
        EffectiveTheme::Dark => "dark",
    }
}

fn theme_terminal_result_name(result: ThemeTerminalResult) -> &'static str {
    match result {
        ThemeTerminalResult::Passed => "passed",
        ThemeTerminalResult::Failed => "failed",
    }
}

fn theme_result_reason_name(reason: ThemeResultReason) -> &'static str {
    match reason {
        ThemeResultReason::Applied => "applied",
        ThemeResultReason::PreferenceLoadFailed => "preference_load_failed",
        ThemeResultReason::NativeApplyFailed => "native_apply_failed",
        ThemeResultReason::ApplyOrSaveFailed => "apply_or_save_failed",
    }
}

fn theme_preference_name(preference: ThemePreference) -> &'static str {
    match preference {
        ThemePreference::System => "system",
        ThemePreference::Light => "light",
        ThemePreference::Dark => "dark",
    }
}

#[cfg(feature = "runtime-simulation")]
#[tauri::command]
fn run_runtime_simulation_voice_session(
    state: tauri::State<'_, AppState>,
) -> Result<PlatformSnapshot, String> {
    state
        .platform
        .run_simulated_voice_session()
        .map_err(|error| error.to_string())
}

#[cfg(feature = "runtime-simulation")]
#[tauri::command]
fn complete_runtime_simulation_smoke(
    result: serde_json::Value,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let report_path = std::env::var_os("SAYALL_RUNTIME_SIMULATION_REPORT")
        .ok_or_else(|| "缺少 Windows CI 仿真报告路径".to_owned())?;
    let contents = serde_json::to_vec_pretty(&result)
        .map_err(|error| format!("序列化 Windows CI 仿真报告失败：{error}"))?;
    std::fs::write(report_path, contents)
        .map_err(|error| format!("写入 Windows CI 仿真报告失败：{error}"))?;
    let passed = result
        .get("passed")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(200));
        app.exit(if passed { 0 } else { 1 });
    });
    Ok(())
}

fn create_platform() -> Arc<dyn PlatformRuntime> {
    #[cfg(feature = "runtime-simulation")]
    if runtime_simulation_requested() {
        return Arc::new(platform::SimulatedPlatform::default());
    }

    Arc::new(WindowsPlatform::default())
}

/// 语义按键边沿/手势 → Tauri 事件（button-edge / button-gesture）。
/// 引擎线程回调，Emitter::emit 线程安全。
fn register_button_events(platform: &Arc<dyn PlatformRuntime>, app: tauri::AppHandle) {
    let edge_app = app.clone();
    platform.subscribe_button_edges(Arc::new(move |edge| {
        let _ = edge_app.emit("button-edge", &edge);
    }));
    let gesture_app = app;
    platform.subscribe_button_gestures(Arc::new(move |gesture| {
        let _ = gesture_app.emit("button-gesture", &gesture);
    }));
}

const SCENE_OVERLAY_LABEL: &str = "scene-overlay";

fn create_scene_overlay(app: &tauri::App) -> tauri::Result<()> {
    if app.get_webview_window(SCENE_OVERLAY_LABEL).is_some() {
        return Ok(());
    }
    tauri::WebviewWindowBuilder::new(
        app,
        SCENE_OVERLAY_LABEL,
        tauri::WebviewUrl::App("index.html?scene-overlay=1".into()),
    )
    .title("无线麦场景菜单")
    .inner_size(380.0, 360.0)
    .resizable(false)
    .closable(false)
    .minimizable(false)
    .maximizable(false)
    .decorations(false)
    .transparent(true)
    .shadow(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .focused(false)
    .focusable(false)
    .visible(false)
    .build()?;
    sayall_windows::gatt_note(
        "scene_overlay action=create focusable=false visible=false terminal_result=passed"
            .to_owned(),
    );
    Ok(())
}

fn update_scene_overlay(app: &tauri::AppHandle, event: &sayall_windows::scene_control::SceneEvent) {
    let sayall_windows::scene_control::SceneEvent::Snapshot { snapshot } = event else {
        return;
    };
    let Some(window) = app.get_webview_window(SCENE_OVERLAY_LABEL) else {
        sayall_windows::gatt_note(
            "scene_overlay action=visibility terminal_result=failed reason=window_unavailable"
                .to_owned(),
        );
        return;
    };
    let result = if snapshot.panel.is_some() {
        position_scene_overlay(&window).and_then(|()| window.show())
    } else {
        window.hide()
    };
    sayall_windows::gatt_note(format!(
        "scene_overlay action={} terminal_result={} reason={}",
        if snapshot.panel.is_some() {
            "show"
        } else {
            "hide"
        },
        if result.is_ok() { "passed" } else { "failed" },
        if result.is_ok() {
            "snapshot_applied"
        } else {
            "window_operation_failed"
        },
    ));
}

fn position_scene_overlay(window: &tauri::WebviewWindow) -> tauri::Result<()> {
    let monitors = window.available_monitors()?;
    let target_center = foreground_window_center();
    let monitor = target_center
        .and_then(|(x, y)| {
            monitors.iter().find(|monitor| {
                let position = monitor.position();
                let size = monitor.size();
                x >= position.x
                    && y >= position.y
                    && x < position.x.saturating_add(size.width as i32)
                    && y < position.y.saturating_add(size.height as i32)
            })
        })
        .or_else(|| monitors.first());
    let Some(monitor) = monitor else {
        return Ok(());
    };
    let overlay = window.outer_size()?;
    let monitor_position = monitor.position();
    let monitor_size = monitor.size();
    let x = monitor_position
        .x
        .saturating_add((monitor_size.width.saturating_sub(overlay.width) / 2) as i32);
    let y = monitor_position.y.saturating_add(24);
    window.set_position(tauri::PhysicalPosition::new(x, y))
}

#[cfg(windows)]
fn foreground_window_center() -> Option<(i32, i32)> {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowRect};
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.0.is_null() {
        return None;
    }
    let mut bounds = RECT::default();
    unsafe { GetWindowRect(hwnd, &mut bounds) }.ok()?;
    Some((
        bounds.left.saturating_add(bounds.right) / 2,
        bounds.top.saturating_add(bounds.bottom) / 2,
    ))
}

#[cfg(not(windows))]
fn foreground_window_center() -> Option<(i32, i32)> {
    None
}

fn register_scene_events(
    platform: &Arc<dyn PlatformRuntime>,
    settings: SettingsStore,
    app: tauri::AppHandle,
) -> std::io::Result<()> {
    use sayall_windows::scene_control::SceneEvent;
    let (sender, receiver) = std::sync::mpsc::channel();
    let runtime = Arc::clone(platform);
    let event_app = app.clone();
    std::thread::Builder::new()
        .name("sayall-scene-settings".to_owned())
        .spawn(move || {
            while let Ok((template_id, mode, generation)) = receiver.recv() {
                let result = settings.update_mapping_configuration(
                    |configuration| {
                        let template = configuration.templates.iter_mut().find(|template| template.id == template_id)
                            .ok_or_else(|| "调节模式目标模板已删除".to_owned())?;
                        template.adjustment_mode = mode;
                        Ok(())
                    },
                    |configuration| apply_mapping_configuration(runtime.as_ref(), configuration),
                );
                if result.is_err() {
                    let restored = settings.apply_current_mapping_configuration(|configuration| {
                        apply_mapping_configuration(runtime.as_ref(), configuration);
                    });
                    sayall_windows::gatt_note(format!("scene_control action=restore_confirmed_configuration generation={generation} terminal_result={}", if restored.is_ok() { "passed" } else { "failed" }));
                }
                sayall_windows::gatt_note(format!(
                    "scene_control action=persist_adjustment_mode generation={generation} terminal_result={} reason={}",
                    if result.is_ok() { "passed" } else { "failed" },
                    if result.is_ok() { "latest_configuration_updated" } else { "validation_or_write_failed" },
                ));
                let _ = app.emit("scene-mode-persistence", serde_json::json!({ "generation": generation, "saved": result.is_ok() }));
            }
        })
        ?;
    platform.subscribe_scene_events(Arc::new(move |event| {
        update_scene_overlay(&event_app, &event);
        let _ = event_app.emit("scene-event", &event);
        if let SceneEvent::AdjustmentModePersistenceRequested { template_id, mode, generation } = &event {
            if sender.send((template_id.clone(), *mode, *generation)).is_err() {
                sayall_windows::gatt_note("scene_control action=persist_adjustment_mode terminal_result=failed reason=worker_unavailable".to_owned());
            }
        }
    }));
    Ok(())
}

/// Raw Input 监听自愈监督线程：启动尝试一次（遥控器休眠时可能失败）；
/// 此后每 10 秒巡检，phase=Failed（启动失败或监听线程意外退出）时自动重启。
/// Stopped（用户在按键页显式停止）不重启；成功后保持低频巡检自愈。
fn raw_input_supervisor_interval() -> std::time::Duration {
    #[cfg(test)]
    return std::time::Duration::from_millis(20);
    #[cfg(not(test))]
    std::time::Duration::from_secs(10)
}

fn raw_input_supervisor_stop_bound() -> std::time::Duration {
    #[cfg(test)]
    return std::time::Duration::from_secs(1);
    #[cfg(not(test))]
    // RawInputRuntime::start has a 5s ready wait followed by a bounded 2s
    // stop attempt. One extra second covers supervisor scheduling.
    std::time::Duration::from_secs(8)
}

fn spawn_raw_input_supervisor(platform: Weak<dyn PlatformRuntime>) -> RawInputSupervisor {
    let stop = Arc::new((Mutex::new(false), Condvar::new()));
    let worker_stop = Arc::clone(&stop);
    let (stopped_tx, stopped_rx) = mpsc::channel();
    let worker = std::thread::Builder::new()
        .name("sayall-raw-input-supervisor".to_owned())
        .spawn(move || {
            let mut initial_attempt_pending = true;
            loop {
                let (stop_lock, _) = worker_stop.as_ref();
                if *stop_lock
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                {
                    break;
                }
                let Some(platform) = platform.upgrade() else {
                    break;
                };
                let phase = platform.raw_input_snapshot().phase;
                let should_start = phase == sayall_windows::raw_input::RawInputPhase::Failed
                    || (initial_attempt_pending
                        && phase == sayall_windows::raw_input::RawInputPhase::Stopped);
                if should_start {
                    // stop can arrive after the snapshot; re-check at the only
                    // point that may re-enable the listener.
                    if *stop_lock
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                    {
                        break;
                    }
                    let _ = platform.start_raw_input();
                }
                initial_attempt_pending = false;
                drop(platform);
                let (stop_lock, wake) = worker_stop.as_ref();
                let stopped = stop_lock
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                if *stopped {
                    break;
                }
                let (stopped, _) = wake
                    .wait_timeout(stopped, raw_input_supervisor_interval())
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                if *stopped {
                    break;
                }
            }
            let _ = stopped_tx.send(());
        });
    let (worker, spawn_failed) = match worker {
        Ok(handle) => (
            Some(RawInputSupervisorWorker {
                handle,
                stopped: stopped_rx,
            }),
            false,
        ),
        Err(_) => {
            sayall_windows::gatt_note(
                "raw_input_supervisor stage=spawn phase=completed terminal_result=failed error_domain=thread error_code=spawn_failed reason=worker_unavailable retryable=true".to_owned(),
            );
            (None, true)
        }
    };
    RawInputSupervisor {
        stop,
        worker: Mutex::new(worker),
        spawn_failed,
    }
}

fn request_clean_exit(app: tauri::AppHandle, exit_code: i32) {
    let cleanup = app.state::<AppState>().exit_cleanup.clone();
    if cleanup.is_finished() {
        app.exit(exit_code);
        return;
    }
    if !cleanup.begin_exit_worker() {
        return;
    }
    let exit_app = app.clone();
    if std::thread::Builder::new()
        .name("sayall-exit-cleanup".to_owned())
        .spawn(move || {
            cleanup.shutdown_blocking();
            exit_app.exit(exit_code);
        })
        .is_err()
    {
        app.state::<AppState>().exit_cleanup.reset_exit_worker();
        sayall_windows::gatt_note(
            "app_shutdown stage=worker_spawn phase=completed terminal_result=failed error_domain=thread error_code=spawn_failed reason=cleanup_not_started retryable=true".to_owned(),
        );
    }
}

#[cfg(feature = "runtime-simulation")]
fn runtime_simulation_requested() -> bool {
    std::env::var_os("SAYALL_WINDOWS_RUNTIME_SIMULATION").as_deref()
        == Some(std::ffi::OsStr::new("1"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let log_path = std::env::var_os("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("SayAll")
        .join("Logs")
        .join("sayall-diagnostic.log");
    let log_ready = sayall_windows::initialize_diagnostic_log(
        log_path,
        sayall_windows::DiagnosticLogMetadata {
            app_version: env!("CARGO_PKG_VERSION").to_owned(),
            app_build: option_env!("SAYALL_APP_BUILD")
                .unwrap_or("unknown")
                .to_owned(),
            source_revision: env!("SAYALL_SOURCE_REVISION").to_owned(),
            build_channel: option_env!("SAYALL_BUILD_CHANNEL")
                .unwrap_or("unknown")
                .to_owned(),
            release_tag: option_env!("SAYALL_RELEASE_TAG")
                .unwrap_or("unknown")
                .to_owned(),
        },
    );
    sayall_windows::gatt_note(format!(
        "app_lifecycle event=process_start phase=started result={} diagnostic_schema=1 process_architecture={} windows_version=unknown windows_build=unknown",
        if log_ready { "passed" } else { "failed" },
        std::env::consts::ARCH
    ));
    #[cfg(windows)]
    if let Err(error) = sayall_windows::compatibility::check_current_windows() {
        sayall_windows::gatt_note(
            "app_lifecycle event=compatibility_check phase=completed terminal_result=failed error_domain=windows error_code=unsupported_version reason=os_requirement_not_met retryable=false".to_owned(),
        );
        sayall_windows::compatibility::show_unsupported_windows_message(error);
        eprintln!("{error}");
        return;
    }
    // 单实例守卫（2026-09-05 实证：双实例并存——开发构建与已部署版抢遥控器
    // 连接、抑制器互扰、抢不到连接的实例还会周期性无线电重启杀掉对方的
    // 连接）。命名互斥体跨进程互斥；已存在实例时本次启动直接退出。
    // 注意：互斥体名不得含反斜杠——对象管理器会把名字按路径解析，要求
    // 父对象目录存在（"SayAll\Windows\…" 直接 ERROR_PATH_NOT_FOUND，
    // 2026-09-05 探针实证）；创建失败按 fail-closed 处理（退出）——
    // 双实例的危害（互扰+互杀连接）远大于极端情况下的误拦。
    #[cfg(windows)]
    {
        use windows::core::w;
        use windows::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS};
        use windows::Win32::System::Threading::CreateMutexW;
        const SINGLE_INSTANCE_MUTEX: windows::core::PCWSTR = w!("SayAll.Windows.SingleInstance");
        match unsafe { CreateMutexW(None, false, SINGLE_INSTANCE_MUTEX) } {
            Ok(handle) => {
                // CreateMutexW 对"已存在"返回有效句柄 + GetLastError=
                // ERROR_ALREADY_EXISTS（不是失败）；其余残留错误值无意义。
                if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
                    sayall_windows::gatt_note(
                        "app_lifecycle event=single_instance phase=completed terminal_result=failed error_domain=process error_code=already_running reason=existing_instance retryable=false".to_owned(),
                    );
                    eprintln!("SayAll 已在运行：单实例守卫阻止了第二个实例启动");
                    unsafe {
                        let _ = CloseHandle(handle);
                    }
                    return;
                }
                // 故意持有互斥体句柄不关闭：进程存活期间保持占有，退出时由系统释放。
                std::mem::forget(handle);
                sayall_windows::gatt_note(
                    "app_lifecycle event=single_instance phase=completed terminal_result=passed"
                        .to_owned(),
                );
            }
            Err(error) => {
                sayall_windows::gatt_note(
                    "app_lifecycle event=single_instance phase=completed terminal_result=failed error_domain=windows error_code=mutex_create_failed reason=guard_unavailable retryable=true".to_owned(),
                );
                eprintln!("单实例互斥体创建失败：{error}（fail-closed 退出）");
                return;
            }
        }
    }

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        // 应用内更新（GitHub Releases 静态 latest.json + minisign 验签）。
        .plugin(tauri_plugin_updater::Builder::new().build())
        .on_page_load(|webview, payload| {
            let phase = match payload.event() {
                tauri::webview::PageLoadEvent::Started => "started",
                tauri::webview::PageLoadEvent::Finished => "finished",
            };
            sayall_windows::gatt_note(format!(
                "webview event=document_load phase={phase} result=passed main_window={}",
                webview.label() == "main"
            ));
        })
        .setup(|app| {
            sayall_windows::gatt_note(
                "app_lifecycle event=tauri_setup phase=started result=passed".to_owned(),
            );
            // 托盘图标：主窗口关闭后驻留；菜单 = 显示主界面 / 退出；
            // 左键点击托盘 = 显示并聚焦主窗口（Mac StatusIcon 同款行为）。
            #[cfg(all(windows, not(feature = "runtime-simulation")))]
            {
                use tauri::menu::{Menu, MenuItem};
                use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

                let show = MenuItem::with_id(app, "tray-show", "显示主界面", true, None::<&str>)?;
                let quit = MenuItem::with_id(app, "tray-quit", "退出", true, None::<&str>)?;
                let menu = Menu::with_items(app, &[&show, &quit])?;
                let icon = app.default_window_icon().cloned().ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::NotFound, "缺少应用图标，无法创建托盘")
                })?;
                TrayIconBuilder::with_id("sayall-tray")
                    .icon(icon)
                    .menu(&menu)
                    .show_menu_on_left_click(false)
                    .tooltip("无线麦 SayAll")
                    .on_menu_event(|app, event| match event.id.as_ref() {
                        "tray-show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "tray-quit" => request_clean_exit(app.clone(), 0),
                        _ => {}
                    })
                    .on_tray_icon_event(|tray, event| {
                        if let TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } = event
                        {
                            if let Some(window) = tray.app_handle().get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    })
                    .build(app)?;
            }

            #[cfg(feature = "runtime-simulation")]
            let settings_path = if runtime_simulation_requested() {
                let directory = std::env::var_os("SAYALL_RUNTIME_SIMULATION_STATE_DIR")
                    .ok_or_else(|| {
                        std::io::Error::new(
                            std::io::ErrorKind::NotFound,
                            "缺少 Windows CI 仿真设置目录",
                        )
                    })?;
                std::path::PathBuf::from(directory).join("settings.json")
            } else {
                app.path().app_config_dir()?.join("settings.json")
            };
            #[cfg(not(feature = "runtime-simulation"))]
            let settings_path = app.path().app_config_dir()?.join("settings.json");
            let settings = SettingsStore::new(settings_path);
            let saved_settings = match settings.load() {
                Ok(settings) => {
                    sayall_windows::gatt_note(
                        "settings feature=application action=load phase=completed terminal_result=passed".to_owned(),
                    );
                    settings
                }
                Err(error) => {
                    sayall_windows::gatt_note(
                        "settings feature=application action=load phase=completed terminal_result=failed error_domain=settings error_code=parse_or_read_failed reason=defaults_applied retryable=true".to_owned(),
                    );
                    eprintln!("{error}");
                    Default::default()
                }
            };
            let platform = create_platform();
            if let Err(error) = platform.initialize_capture_input(settings.capture_journal_path(), saved_settings.capture_input.clone()) {
                sayall_windows::gatt_note(format!("capture_input action=initialize result=failed error_code={error}"));
            }

            let mapping_configuration = match settings.load_mapping_configuration() {
                Ok(configuration) => {
                    sayall_windows::gatt_note(format!(
                        "shortcut_settings feature=button_mapping action=load phase=completed terminal_result=passed {}",
                        button_mapping_log_summary(&configuration.common_mappings)
                    ));
                    configuration
                }
                Err(error) => {
                    sayall_windows::gatt_note(
                        "shortcut_settings feature=button_mapping action=load phase=completed terminal_result=failed error_domain=settings error_code=parse_or_read_failed reason=defaults_applied retryable=true".to_owned(),
                    );
                    eprintln!("{error}");
                    sayall_windows::templates::MappingConfiguration::default()
                }
            };
            // 启动即热加载已保存映射（引擎与门控吞键配置同步就绪）。
            platform.set_mapping_configuration(mapping_configuration);

            #[cfg(windows)]
            if let (Some(endpoint_id), Some(endpoint_name)) = (
                saved_settings.audio_endpoint_id,
                saved_settings.audio_endpoint_name,
            ) {
                if let Err(error) = platform.restore_audio_endpoint(endpoint_id, endpoint_name) {
                    sayall_windows::gatt_note(
                        "audio_endpoint action=restore phase=ipc_completed terminal_result=failed error_domain=platform error_code=restore_request_failed reason=platform_rejected retryable=true".to_owned(),
                    );
                    eprintln!("恢复已保存的音频端点失败：{error}");
                }
            }

            #[cfg(windows)]
            if let Some(device_id) = saved_settings.selected_remote_id {
                match platform.restore_remote(device_id) {
                    Ok(_) => {}
                    Err(error) => {
                        eprintln!("恢复已保存的小米语音遥控器失败：{error}");
                    }
                }
            }

            match settings.load_voice_hold_hotkey() {
                Ok(hotkey) => {
                    sayall_windows::gatt_note(format!(
                        "shortcut_settings feature=voice_hold action=load phase=completed terminal_result=passed enabled={} key_count={}",
                        hotkey.is_some(),
                        hotkey.as_ref().map(|chord| chord.keys.len()).unwrap_or(0)
                    ));
                    platform.set_voice_hold_hotkey(hotkey)
                }
                Err(error) => {
                    sayall_windows::gatt_note(
                        "shortcut_settings feature=voice_hold action=load phase=completed terminal_result=failed error_domain=settings error_code=parse_or_read_failed reason=disabled_fallback retryable=true".to_owned(),
                    );
                    eprintln!("{error}");
                    platform.set_voice_hold_hotkey(None);
                }
            }

            #[cfg(not(windows))]
            let _ = saved_settings;

            create_scene_overlay(app)?;
            // 语义按键边沿与手势事件 → 前端（画布高亮与单击/双击/长按反馈）。
            register_button_events(&platform, app.handle().clone());
            register_scene_events(&platform, settings.clone(), app.handle().clone())?;

            // Raw Input 监听自愈：启动即尝试，失败（遥控器休眠/未连接）进入
            // 10 秒重试循环；用户在按键页显式停止（Stopped）时不重试。
            let supervisor = spawn_raw_input_supervisor(Arc::downgrade(&platform));
            let exit_cleanup = ExitCleanup::new(Arc::clone(&platform), supervisor);

            app.manage(AppState {
                capture_config_operation: platform.capture_config_gate(),
                platform,
                exit_cleanup,
                settings,
                pending_update: std::sync::Mutex::new(None),
            });
            sayall_windows::gatt_note(
                "app_lifecycle event=tauri_setup phase=completed terminal_result=passed window_created=true state_managed=true".to_owned(),
            );
            Ok(())
        });

    let builder = builder
        // 关闭主窗口 → 隐藏到托盘驻留（托盘菜单"退出"才真正退出；
        // 退出走 Tauri 正常事件循环结束，平台组件 Drop 清理照常执行）。
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        });

    #[cfg(feature = "runtime-simulation")]
    let builder = builder.invoke_handler(tauri::generate_handler![
        get_capture_input,
        list_capture_inputs,
        set_capture_input,
        resolve_capture_recovery,
        get_runtime_snapshot,
        get_diagnostic_report,
        scan_paired_remotes,
        get_connection_snapshot,
        connect_remote,
        disconnect_remote,
        open_bluetooth_settings,
        list_audio_endpoints,
        get_audio_snapshot,
        select_audio_endpoint,
        get_raw_input_snapshot,
        start_raw_input,
        stop_raw_input,
        get_button_mappings,
        get_mapping_configuration,
        get_mapping_template_presets,
        get_scene_snapshot,
        get_component_status,
        perform_component_action,
        apply_mapping_template_preset,
        preview_template_import,
        preview_mapping_configuration_import,
        apply_mapping_configuration_import,
        export_mapping_configuration,
        save_mapping_configuration,
        create_mapping_template,
        duplicate_mapping_template,
        rename_mapping_template,
        delete_mapping_template,
        upsert_application_binding,
        remove_application_binding,
        save_button_mappings,
        reset_button_mappings,
        test_button_mapping,
        list_preset_apps,
        pick_custom_app,
        get_button_mapping_snapshot,
        get_send_input_snapshot,
        get_voice_hold_hotkey,
        set_voice_hold_hotkey,
        get_theme_preference,
        set_theme_preference,
        report_theme_result,
        get_app_update_preferences,
        set_app_update_preferences,
        check_app_update,
        install_app_update,
        report_frontend_event,
        run_runtime_simulation_voice_session,
        complete_runtime_simulation_smoke
    ]);
    #[cfg(not(feature = "runtime-simulation"))]
    let builder = builder.invoke_handler(tauri::generate_handler![
        get_capture_input,
        list_capture_inputs,
        set_capture_input,
        resolve_capture_recovery,
        get_runtime_snapshot,
        get_diagnostic_report,
        scan_paired_remotes,
        get_connection_snapshot,
        connect_remote,
        disconnect_remote,
        open_bluetooth_settings,
        list_audio_endpoints,
        get_audio_snapshot,
        select_audio_endpoint,
        get_raw_input_snapshot,
        start_raw_input,
        stop_raw_input,
        get_button_mappings,
        get_mapping_configuration,
        get_mapping_template_presets,
        get_scene_snapshot,
        get_component_status,
        perform_component_action,
        apply_mapping_template_preset,
        preview_template_import,
        preview_mapping_configuration_import,
        apply_mapping_configuration_import,
        export_mapping_configuration,
        save_mapping_configuration,
        create_mapping_template,
        duplicate_mapping_template,
        rename_mapping_template,
        delete_mapping_template,
        upsert_application_binding,
        remove_application_binding,
        save_button_mappings,
        reset_button_mappings,
        test_button_mapping,
        list_preset_apps,
        pick_custom_app,
        get_button_mapping_snapshot,
        get_send_input_snapshot,
        get_voice_hold_hotkey,
        set_voice_hold_hotkey,
        get_theme_preference,
        set_theme_preference,
        report_theme_result,
        get_app_update_preferences,
        set_app_update_preferences,
        check_app_update,
        install_app_update,
        report_frontend_event
    ]);

    let app = builder.build(tauri::generate_context!()).unwrap_or_else(|_| {
        sayall_windows::gatt_note(
            "app_lifecycle event=event_loop phase=completed terminal_result=failed error_domain=tauri error_code=build_failed reason=application_build_failed retryable=false".to_owned(),
        );
        panic!("failed to build SayAll Windows app");
    });
    app.run(|app, event| {
        if let tauri::RunEvent::ExitRequested { code, api, .. } = event {
            let cleanup = app.state::<AppState>().exit_cleanup.clone();
            if !cleanup.is_finished() {
                api.prevent_exit();
                request_clean_exit(app.clone(), code.unwrap_or(0));
            }
        }
    });
    sayall_windows::gatt_note(
        "app_lifecycle event=process_exit phase=completed terminal_result=passed".to_owned(),
    );
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;
    use sayall_windows::raw_input::{RawInputPhase, RawInputSnapshot};
    use sayall_windows::send_input::{ButtonMappings, KeyChord, SendInputSnapshot};
    use sayall_windows::{
        AudioEndpoint, AudioSnapshot, ConnectionSnapshot, PairedRemote, PlatformError,
        PlatformSnapshot, UsageCounters,
    };
    use std::sync::atomic::{AtomicU64, AtomicUsize};

    #[derive(Debug)]
    struct TestPlatform {
        calls: Mutex<Vec<&'static str>>,
        raw_phase: Mutex<RawInputPhase>,
        starts: AtomicUsize,
        fail_disconnect: AtomicBool,
        quiesce_delay_ms: AtomicU64,
        capture: Mutex<sayall_core::CaptureInputSettings>,
    }

    impl Default for TestPlatform {
        fn default() -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
                raw_phase: Mutex::new(RawInputPhase::Stopped),
                starts: AtomicUsize::new(0),
                fail_disconnect: AtomicBool::new(false),
                quiesce_delay_ms: AtomicU64::new(0),
                capture: Mutex::new(Default::default()),
            }
        }
    }

    impl TestPlatform {
        fn record(&self, call: &'static str) {
            self.calls
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(call);
        }

        fn calls(&self) -> Vec<&'static str> {
            self.calls
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone()
        }

        fn set_raw_phase(&self, phase: RawInputPhase) {
            *self
                .raw_phase
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = phase;
        }
    }

    impl PlatformRuntime for TestPlatform {
        fn capture_input_snapshot(&self) -> sayall_windows::capture_input::CaptureInputSnapshot {
            sayall_windows::capture_input::CaptureInputSnapshot {
                settings: self.capture.lock().unwrap().clone(),
                ..Default::default()
            }
        }
        fn configure_capture_input(
            &self,
            settings: sayall_core::CaptureInputSettings,
        ) -> Result<sayall_windows::capture_input::CaptureInputSnapshot, String> {
            *self.capture.lock().unwrap() = settings;
            Ok(self.capture_input_snapshot())
        }
        fn shutdown_capture_input(&self) -> Result<(), String> {
            self.record("capture_shutdown");
            Ok(())
        }

        fn usage_counters(&self) -> Arc<UsageCounters> {
            Arc::new(UsageCounters::default())
        }

        fn snapshot(&self) -> PlatformSnapshot {
            panic!("not used by lifecycle tests")
        }

        fn scan_paired_remotes(&self) -> Result<Vec<PairedRemote>, PlatformError> {
            Ok(Vec::new())
        }

        fn connection_snapshot(&self) -> ConnectionSnapshot {
            ConnectionSnapshot::default()
        }

        fn connect_remote(&self, _device_id: String) -> Result<ConnectionSnapshot, PlatformError> {
            Err(PlatformError::UnsupportedPlatform)
        }

        fn disconnect_remote(&self) -> Result<ConnectionSnapshot, PlatformError> {
            self.record("disconnect");
            if self.fail_disconnect.load(Ordering::Acquire) {
                Err(PlatformError::WorkerUnavailable)
            } else {
                Ok(ConnectionSnapshot::default())
            }
        }

        #[cfg(windows)]
        fn restore_remote(&self, _device_id: String) -> Result<ConnectionSnapshot, PlatformError> {
            Err(PlatformError::UnsupportedPlatform)
        }

        fn list_audio_endpoints(&self) -> Result<Vec<AudioEndpoint>, PlatformError> {
            Ok(Vec::new())
        }

        fn select_audio_endpoint(
            &self,
            _endpoint_id: String,
        ) -> Result<AudioSnapshot, PlatformError> {
            Err(PlatformError::UnsupportedPlatform)
        }

        #[cfg(windows)]
        fn restore_audio_endpoint(
            &self,
            _endpoint_id: String,
            _expected_name: String,
        ) -> Result<AudioSnapshot, PlatformError> {
            Err(PlatformError::UnsupportedPlatform)
        }

        fn audio_snapshot(&self) -> AudioSnapshot {
            AudioSnapshot::default()
        }

        fn raw_input_snapshot(&self) -> RawInputSnapshot {
            RawInputSnapshot {
                phase: *self
                    .raw_phase
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner()),
                ..RawInputSnapshot::default()
            }
        }

        fn start_raw_input(&self) -> Result<RawInputSnapshot, PlatformError> {
            self.starts.fetch_add(1, Ordering::AcqRel);
            self.record("start");
            self.set_raw_phase(RawInputPhase::Ready);
            Ok(self.raw_input_snapshot())
        }

        fn stop_raw_input(&self) -> Result<RawInputSnapshot, PlatformError> {
            self.record("stop");
            self.set_raw_phase(RawInputPhase::Stopped);
            Ok(self.raw_input_snapshot())
        }

        fn send_input_snapshot(&self) -> SendInputSnapshot {
            SendInputSnapshot::default()
        }

        fn test_shortcut(&self, _chord: KeyChord) -> Result<SendInputSnapshot, PlatformError> {
            Err(PlatformError::UnsupportedPlatform)
        }

        fn preset_apps(&self) -> Vec<sayall_windows::app_launcher::PresetAppInfo> {
            Vec::new()
        }

        fn launch_app(&self, _target: &str) -> Result<(), PlatformError> {
            Err(PlatformError::UnsupportedPlatform)
        }

        fn open_bluetooth_settings(&self) -> Result<(), PlatformError> {
            Err(PlatformError::UnsupportedPlatform)
        }

        fn voice_hold_hotkey(&self) -> Option<KeyChord> {
            None
        }

        fn set_voice_hold_hotkey(&self, _hotkey: Option<KeyChord>) {}

        fn button_mappings(&self) -> ButtonMappings {
            ButtonMappings::default()
        }

        fn set_button_mappings(&self, _mappings: ButtonMappings) {}

        fn set_mapping_configuration(
            &self,
            _configuration: sayall_windows::templates::MappingConfiguration,
        ) {
        }

        fn scene_snapshot(&self) -> Option<sayall_windows::scene_control::SceneSnapshot> {
            None
        }

        fn subscribe_scene_events(
            &self,
            _callback: sayall_windows::scene_control::SceneEventCallback,
        ) {
        }

        fn button_mapping_snapshot(&self) -> sayall_windows::button_mapping::ButtonMappingSnapshot {
            Default::default()
        }

        fn subscribe_button_edges(
            &self,
            _callback: sayall_windows::button_mapping::ButtonEdgeCallback,
        ) {
        }

        fn subscribe_button_gestures(
            &self,
            _callback: sayall_windows::button_mapping::ButtonGestureCallback,
        ) {
        }

        fn quiesce_input(&self) -> Result<(), PlatformError> {
            self.record("quiesce");
            std::thread::sleep(std::time::Duration::from_millis(
                self.quiesce_delay_ms.load(Ordering::Acquire),
            ));
            Ok(())
        }
    }

    fn stopped_supervisor() -> RawInputSupervisor {
        RawInputSupervisor {
            stop: Arc::new((Mutex::new(false), Condvar::new())),
            worker: Mutex::new(None),
            spawn_failed: false,
        }
    }

    #[test]
    fn capture_config_save_failure_rolls_back_before_releasing_gate() {
        let platform = TestPlatform::default();
        let gate = Mutex::new(());
        let next = sayall_core::CaptureInputSettings {
            enabled: true,
            endpoint_id: Some("target".into()),
            endpoint_name: Some("target".into()),
        };
        let result = capture_config_transaction(&gate, &platform, next, |_| {
            assert!(gate.try_lock().is_err());
            Err("disk_failed".into())
        });
        assert!(result.is_err());
        assert_eq!(
            platform.capture_input_snapshot().settings,
            Default::default()
        );
        assert!(gate.try_lock().is_ok());
    }
    #[test]
    fn capture_config_concurrent_saves_keep_runtime_and_persisted_value_aligned() {
        let platform = Arc::new(TestPlatform::default());
        let gate = Arc::new(Mutex::new(()));
        let saved = Arc::new(Mutex::new(sayall_core::CaptureInputSettings::default()));
        let (entered_tx, entered_rx) = mpsc::channel();
        let (continue_tx, continue_rx) = mpsc::channel();
        let a = {
            let platform = platform.clone();
            let gate = gate.clone();
            let saved = saved.clone();
            std::thread::spawn(move || {
                capture_config_transaction(
                    &gate,
                    platform.as_ref(),
                    sayall_core::CaptureInputSettings {
                        endpoint_id: Some("a".into()),
                        ..Default::default()
                    },
                    |value| {
                        entered_tx.send(()).unwrap();
                        continue_rx.recv().unwrap();
                        *saved.lock().unwrap() = value;
                        Ok(())
                    },
                )
            })
        };
        entered_rx.recv().unwrap();
        assert!(gate.try_lock().is_err());
        let b = {
            let platform = platform.clone();
            let gate = gate.clone();
            let saved = saved.clone();
            std::thread::spawn(move || {
                capture_config_transaction(
                    &gate,
                    platform.as_ref(),
                    sayall_core::CaptureInputSettings {
                        endpoint_id: Some("b".into()),
                        ..Default::default()
                    },
                    |value| {
                        *saved.lock().unwrap() = value;
                        Ok(())
                    },
                )
            })
        };
        continue_tx.send(()).unwrap();
        a.join().unwrap().unwrap();
        b.join().unwrap().unwrap();
        assert_eq!(
            platform.capture_input_snapshot().settings,
            *saved.lock().unwrap()
        );
        assert_eq!(saved.lock().unwrap().endpoint_id.as_deref(), Some("b"));
    }
    #[test]
    fn capture_shutdown_follows_ble_owner_release_barrier() {
        let platform = Arc::new(TestPlatform::default());
        let cleanup = ExitCleanup::new(
            platform.clone(),
            RawInputSupervisor {
                stop: Arc::new((Mutex::new(true), Condvar::new())),
                worker: Mutex::new(None),
                spawn_failed: false,
            },
        );
        assert!(cleanup.shutdown_blocking());
        assert_eq!(
            platform.calls(),
            vec!["quiesce", "stop", "disconnect", "capture_shutdown"]
        );
    }

    #[test]
    fn supervisor_stop_prevents_restart_and_releases_platform_owner() {
        let platform = Arc::new(TestPlatform::default());
        let runtime: Arc<dyn PlatformRuntime> = platform.clone();
        let supervisor = spawn_raw_input_supervisor(Arc::downgrade(&runtime));
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
        while platform.starts.load(Ordering::Acquire) == 0 && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(platform.starts.load(Ordering::Acquire), 1);

        assert_eq!(supervisor.stop(), Ok(()));
        let starts_after_stop = platform.starts.load(Ordering::Acquire);
        platform.set_raw_phase(RawInputPhase::Failed);
        std::thread::sleep(std::time::Duration::from_millis(80));
        assert_eq!(platform.starts.load(Ordering::Acquire), starts_after_stop);

        let weak = Arc::downgrade(&runtime);
        drop(runtime);
        drop(platform);
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn exit_cleanup_is_ordered_idempotent_and_retains_failure_result() {
        let platform = Arc::new(TestPlatform::default());
        platform.fail_disconnect.store(true, Ordering::Release);
        platform.quiesce_delay_ms.store(50, Ordering::Release);
        let runtime: Arc<dyn PlatformRuntime> = platform.clone();
        let cleanup = ExitCleanup::new(runtime, stopped_supervisor());

        let owner_cleanup = cleanup.clone();
        let owner = std::thread::spawn(move || owner_cleanup.shutdown_blocking());
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
        while platform.calls().is_empty() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        let repeated_cleanup = cleanup.clone();
        let repeated = std::thread::spawn(move || repeated_cleanup.shutdown_blocking());
        assert!(!owner.join().unwrap());
        assert!(!repeated.join().unwrap());
        assert!(cleanup.is_finished());
        assert!(!cleanup.begin_exit_worker());
        assert_eq!(platform.calls(), vec!["quiesce", "stop", "disconnect"]);
    }
}
