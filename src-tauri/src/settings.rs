use sayall_core::{
    normalize_gain_db, AppSettings, ThemePreference, UsageStatistics, VoiceInputTool,
};
use sayall_windows::send_input::{ButtonAction, ButtonMappings, KeyChord, KeyCode};
use sayall_windows::templates::{
    new_template_id, ApplicationBinding, ImportedTemplatePreview, MappingConfiguration,
    MappingConfigurationImportPreview, MappingTemplate, TemplateImportPreview,
    TemplateImportRequest,
};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use crate::onboarding::{self, OnboardingState, OnboardingStep};

#[derive(Debug, Clone)]
pub struct SettingsStore {
    path: PathBuf,
    access: Arc<Mutex<()>>,
    revision: Arc<AtomicU64>,
    pending_imports: Arc<Mutex<HashMap<String, PendingImport>>>,
    import_sources: Arc<Mutex<HashMap<String, ImportSource>>>,
}

#[derive(Debug, Clone)]
struct PendingImport {
    revision: u64,
    configuration: MappingConfiguration,
    source_token: Option<String>,
}

#[derive(Debug, Clone)]
struct ImportSource {
    revision: u64,
    configuration: MappingConfiguration,
    new_template_ids: BTreeMap<String, String>,
    builtin_template_ids: HashSet<String>,
}

const BUTTON_MAPPING_EXPORT_VERSION: u32 = 1;
const MAPPING_CONFIGURATION_EXPORT_VERSION: u32 = 3;
const MAX_BUTTON_MAPPING_IMPORT_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ButtonMappingConfiguration {
    format_version: u32,
    button_mappings: ButtonMappings,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct MappingConfigurationExport {
    format_version: u32,
    common_mappings: ButtonMappings,
    button_mapping_follow_enabled: bool,
    templates: Vec<MappingTemplate>,
    builtin_template_ids: Vec<String>,
    application_bindings: Vec<sayall_windows::templates::ExportApplicationBinding>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
enum ImportedMappingConfiguration {
    V3(MappingConfigurationExportImport),
    V1(ButtonMappingConfiguration),
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct MappingConfigurationExportImport {
    format_version: u32,
    common_mappings: ButtonMappings,
    #[serde(default)]
    button_mapping_follow_enabled: bool,
    #[serde(default)]
    templates: Vec<MappingTemplate>,
    #[serde(default)]
    builtin_template_ids: Vec<String>,
    #[serde(default)]
    application_bindings: Vec<sayall_windows::templates::ExportApplicationBinding>,
}

impl SettingsStore {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            access: Arc::new(Mutex::new(())),
            revision: Arc::new(AtomicU64::new(0)),
            pending_imports: Arc::new(Mutex::new(HashMap::new())),
            import_sources: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn load(&self) -> Result<AppSettings, String> {
        let _guard = lock(&self.access);
        self.load_unlocked()
    }

    fn load_unlocked(&self) -> Result<AppSettings, String> {
        let contents = match fs::read_to_string(&self.path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(AppSettings::default()),
            Err(error) => return Err(format!("读取应用设置失败：{error}")),
        };
        parse_settings(&contents)
    }

    pub fn capture_journal_path(&self) -> PathBuf {
        self.path.with_file_name("capture-input-session.json")
    }

    pub fn save_capture_audio(
        &self,
        value: sayall_core::CaptureInputSettings,
        audio: sayall_windows::AudioSnapshot,
    ) -> Result<(), String> {
        self.update("保存会话输入设备", |settings| {
            settings.capture_input = value;
            settings.audio_endpoint_id = audio.selected_endpoint_id;
            settings.audio_endpoint_name = audio.selected_endpoint_name;
        })
    }

    pub fn save_audio_endpoint(
        &self,
        endpoint_id: String,
        endpoint_name: String,
    ) -> Result<(), String> {
        self.update("保存音频端点设置", move |settings| {
            settings.audio_endpoint_id = Some(endpoint_id);
            settings.audio_endpoint_name = Some(endpoint_name);
        })
    }

    pub fn save_selected_remote_id(&self, device_id: String) -> Result<(), String> {
        self.update("保存小米语音遥控器设置", move |settings| {
            settings.selected_remote_id = Some(device_id);
        })
    }

    pub fn save_check_prerelease_updates(&self, enabled: bool) -> Result<(), String> {
        self.update("保存预览版更新设置", move |settings| {
            settings.check_prerelease_updates = enabled;
        })
    }

    pub fn save_launch_at_login(&self, enabled: bool) -> Result<(), String> {
        self.update("保存开机自启动设置", move |settings| {
            settings.launch_at_login = enabled;
        })
    }

    pub fn save_theme_preference(&self, preference: ThemePreference) -> Result<(), String> {
        self.update("保存外观设置", move |settings| {
            settings.theme_preference = preference;
        })
    }

    pub fn save_rc003_capture_enabled(&self, enabled: bool) -> Result<(), String> {
        self.update("保存全按键支持设置", |settings| {
            settings.rc003_capture_enabled = enabled;
        })
    }
    pub fn save_ui_preference(
        &self,
        field: sayall_core::UiPreference,
        enabled: bool,
    ) -> Result<(), String> {
        self.update("保存界面偏好", |settings| {
            settings.ui_preferences.set(field, enabled)
        })
    }

    /// 应用图标（设置页「应用图标」，2026-10-02）。

    /// 连接页显式选择的输入工具；None 保持未选择，不因初始化或读取失败写盘。
    pub fn save_voice_input_tool(&self, tool: Option<VoiceInputTool>) -> Result<(), String> {
        self.update("保存输入工具设置", move |settings| {
            settings.voice_input_tool = tool;
        })
    }

    /// 首次使用向导状态（`onboarding.json`，设计稿 §6）。
    ///
    /// 第一次调用执行一次性迁移判定并落盘，必须在任何其他设置写入之前调用
    /// （Tauri setup 最前段）。之后每次读取都返回已持久化的状态，不重写。
    pub fn ensure_onboarding_state(&self) -> Result<OnboardingState, String> {
        let _guard = lock(&self.access);
        self.ensure_onboarding_state_unlocked()
    }

    fn ensure_onboarding_state_unlocked(&self) -> Result<OnboardingState, String> {
        let path = onboarding::state_path(&self.path);
        match fs::read_to_string(&path) {
            Ok(contents) => match serde_json::from_str::<OnboardingState>(&contents) {
                Ok(state) => Ok(state.normalized()),
                Err(_) => {
                    // 损坏状态：保守修复（有旧安装证据按老用户、否则全新开始），
                    // 立即重写为有效文件，绝不静默丢弃成"没走过向导"。
                    let repaired = onboarding::resolve_initial_state(
                        onboarding::install_evidence_exists(&self.path),
                    );
                    self.write_onboarding_state(&repaired)?;
                    sayall_windows::gatt_note(format!(
                        "onboarding event=migration phase=completed result=passed source={} reason=state_repaired_from_corrupt completed_version={}",
                        if repaired.completed_version > 0 {
                            "existing_install"
                        } else {
                            "fresh_install"
                        },
                        repaired.completed_version
                    ));
                    Ok(repaired)
                }
            },
            Err(error) if error.kind() == ErrorKind::NotFound => {
                // 全新安装 vs 老用户升级：只看配置目录里有没有旧安装证据，
                // 一次性 migration_version 防止第二次启动误判（设计稿 §6.2）。
                let evidence = onboarding::install_evidence_exists(&self.path);
                let state = onboarding::resolve_initial_state(evidence);
                self.write_onboarding_state(&state)?;
                sayall_windows::gatt_note(format!(
                    "onboarding event=migration phase=completed result=passed source={} completed_version={} migration_version={}",
                    if evidence { "existing_install" } else { "fresh_install" },
                    state.completed_version,
                    state.migration_version
                ));
                Ok(state)
            }
            Err(error) => {
                sayall_windows::gatt_note(
                    "onboarding event=state_read phase=completed terminal_result=failed error_domain=settings error_code=read_failed reason=state_file_unreadable retryable=true".to_owned(),
                );
                Err(format!("读取向导状态失败：{error}"))
            }
        }
    }

    /// 持久化当前步骤。步骤未变化时不重写、不重复刷日志。
    pub fn save_onboarding_step(&self, step: OnboardingStep) -> Result<OnboardingState, String> {
        let _guard = lock(&self.access);
        let mut state = self.ensure_onboarding_state_unlocked()?;
        if state.step != step {
            state.step = step;
            state.flow_version = onboarding::CURRENT_FLOW_VERSION;
            self.write_onboarding_state(&state)?;
            sayall_windows::gatt_note(format!(
                "onboarding event=step_persisted step={}",
                state.step.as_str()
            ));
        }
        Ok(state)
    }

    /// 设置页「重新运行设置向导」：先回滚未提交的 staged 绑定，再重置向导进度；
    /// 不清除设备/映射/音频/其他设置。
    pub fn restart_onboarding(&self) -> Result<OnboardingState, String> {
        let _guard = lock(&self.access);
        self.restore_onboarding_staged_binding_unlocked("wizard_restart")?;
        let mut state = self.ensure_onboarding_state_unlocked()?;
        state.step = OnboardingStep::Welcome;
        state.completed_version = 0;
        state.flow_version = onboarding::CURRENT_FLOW_VERSION;
        self.write_onboarding_state(&state)?;
        sayall_windows::gatt_note(format!(
            "onboarding event=restarted flow_version={}",
            state.flow_version
        ));
        Ok(state)
    }

    /// 完成向导：提交 staged 事务（保留当前正式值、清除回滚快照）并标记完成。
    pub fn complete_onboarding(&self) -> Result<OnboardingState, String> {
        let _guard = lock(&self.access);
        let mut state = self.ensure_onboarding_state_unlocked()?;
        if let Some(staged) = state.staged.take() {
            sayall_windows::gatt_note(format!(
                "onboarding event=binding action=verified tool={} hotkey_count={}",
                crate::voice_input_tool_name(staged.tool),
                staged
                    .voice_hotkey
                    .as_ref()
                    .map(|chord| chord.keys.len())
                    .unwrap_or(0)
            ));
        }
        state.flow_version = onboarding::CURRENT_FLOW_VERSION;
        state.completed_version = state.flow_version;
        self.write_onboarding_state(&state)?;
        sayall_windows::gatt_note(format!(
            "onboarding event=completed flow_version={}",
            state.flow_version
        ));
        Ok(state)
    }

    /// 退出或跳过未完成验证：撤销临时绑定，再关闭自动展示。
    /// completed_version 表示本版向导已处理；只有 complete_onboarding 宣称验证通过。
    pub fn dismiss_onboarding(&self) -> Result<OnboardingState, String> {
        let _guard = lock(&self.access);
        self.restore_onboarding_staged_binding_unlocked("wizard_dismissed")?;
        let mut state = self.ensure_onboarding_state_unlocked()?;
        state.flow_version = onboarding::CURRENT_FLOW_VERSION;
        state.completed_version = state.flow_version;
        self.write_onboarding_state(&state)?;
        sayall_windows::gatt_note(format!(
            "onboarding event=dismissed terminal_result=cancelled flow_version={} step={} binding=restored",
            state.flow_version, state.step.as_str()
        ));
        Ok(state)
    }

    fn write_onboarding_state(&self, state: &OnboardingState) -> Result<(), String> {
        let path = onboarding::state_path(&self.path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                log_onboarding_write_failure("create_dir_failed", true);
                format!("创建应用设置目录失败：{error}")
            })?;
        }
        let contents = serde_json::to_vec_pretty(state).map_err(|error| {
            log_onboarding_write_failure("serialize_failed", false);
            format!("序列化向导状态失败：{error}")
        })?;
        fs::write(&path, contents).map_err(|error| {
            log_onboarding_write_failure("file_write_failed", true);
            format!("保存向导状态失败：{error}")
        })
    }

    /// 语音增益（dB，0–24，对齐 Mac `gainDB` 的持久化口径）。
    ///
    /// 越界与非有限值在这里钳制后落盘，并返回实际保存值供界面显示：
    /// 界面滑块与配置文件的取值从此只有一种语义（0 = 原始音量）。
    pub fn save_gain_db(&self, gain_db: f32) -> Result<f32, String> {
        let normalized = normalize_gain_db(gain_db);
        self.update("保存增益设置", move |settings| {
            settings.gain_db = normalized;
        })?;
        Ok(normalized)
    }

    pub fn usage_statistics(&self) -> Result<UsageStatistics, String> {
        self.load().map(|settings| settings.usage_statistics)
    }

    /// 暂存语音绑定（第④步选输入工具）：先落回滚快照，再应用正式值。
    ///
    /// 必须在向导进行中调用；快照只取**第一次**进入事务时的正式值（后续换工具
    /// 不覆盖回滚基线）。持久化顺序：快照先写、正式值后写——中途崩溃也不会
    /// 丢失回滚目标（设计稿 §5.4 / §6）。运行时应用（platform）由命令层负责。
    pub fn stage_onboarding_voice_binding(
        &self,
        tool: VoiceInputTool,
        hotkey: Option<KeyChord>,
    ) -> Result<OnboardingState, String> {
        let _guard = lock(&self.access);
        let mut state = self.ensure_onboarding_state_unlocked()?;
        if !state.is_active() {
            return Err("首次设置向导未在进行中".to_owned());
        }
        let staged = match state.staged.clone() {
            Some(existing) => onboarding::OnboardingStagedBinding {
                tool: Some(tool),
                voice_hotkey: hotkey.clone(),
                revert_tool: existing.revert_tool,
                revert_voice_hotkey: existing.revert_voice_hotkey,
            },
            None => onboarding::OnboardingStagedBinding {
                tool: Some(tool),
                voice_hotkey: hotkey.clone(),
                revert_tool: self.load_unlocked()?.voice_input_tool,
                revert_voice_hotkey: self.load_voice_hold_hotkey_unlocked()?,
            },
        };
        state.staged = Some(staged.clone());
        self.write_onboarding_state(&state)?;
        self.update_unlocked("保存输入工具设置", move |settings| {
            settings.voice_input_tool = Some(tool);
        })?;
        self.save_voice_hold_hotkey_unlocked(hotkey)?;
        sayall_windows::gatt_note(format!(
            "onboarding event=binding action=staged tool={} hotkey_count={} revert_tool={} revert_hotkey_count={}",
            crate::voice_input_tool_name(Some(tool)),
            staged
                .voice_hotkey
                .as_ref()
                .map(|chord| chord.keys.len())
                .unwrap_or(0),
            crate::voice_input_tool_name(staged.revert_tool),
            staged
                .revert_voice_hotkey
                .as_ref()
                .map(|chord| chord.keys.len())
                .unwrap_or(0)
        ));
        Ok(state)
    }

    /// 回滚并清除 staged 事务（启动恢复 / 重跑向导用）。返回是否处理了事务。
    ///
    /// 写序：先把回滚值写回正式配置，成功后才清除快照——中途失败时快照保留，
    /// 下次启动重试（幂等）。已完成状态下残留快照只清除、不动正式配置。
    pub fn restore_onboarding_staged_binding(&self, reason: &str) -> Result<bool, String> {
        let _guard = lock(&self.access);
        self.restore_onboarding_staged_binding_unlocked(reason)
    }

    fn restore_onboarding_staged_binding_unlocked(&self, reason: &str) -> Result<bool, String> {
        let mut state = self.ensure_onboarding_state_unlocked()?;
        let Some(staged) = state.staged.clone() else {
            return Ok(false);
        };
        if state.is_active() {
            self.update_unlocked("恢复输入工具设置", move |settings| {
                settings.voice_input_tool = staged.revert_tool;
            })?;
            self.save_voice_hold_hotkey_unlocked(staged.revert_voice_hotkey.clone())?;
            sayall_windows::gatt_note(format!(
                "onboarding event=binding action=restored reason={reason} tool={} hotkey_count={}",
                crate::voice_input_tool_name(staged.revert_tool),
                staged
                    .revert_voice_hotkey
                    .as_ref()
                    .map(|chord| chord.keys.len())
                    .unwrap_or(0)
            ));
        } else {
            // 异常组合（已完成却留有未提交快照）：只清快照，不动正式配置。
            sayall_windows::gatt_note(format!(
                "onboarding event=binding action=cleared reason={reason}_state_inconsistent"
            ));
        }
        state.staged = None;
        self.write_onboarding_state(&state)?;
        Ok(true)
    }

    pub fn record_usage(
        &self,
        local_date: String,
        button_presses: u64,
        voice_sessions: u64,
        voice_seconds: f64,
    ) -> Result<(), String> {
        if button_presses == 0 && voice_sessions == 0 && voice_seconds <= 0.0 {
            return Ok(());
        }
        self.update("保存本机使用统计", move |settings| {
            settings
                .usage_statistics
                .record_button_presses(&local_date, button_presses);
            settings.usage_statistics.record_voice_sessions(
                &local_date,
                voice_sessions,
                voice_seconds,
            );
        })
    }

    pub fn load_button_mappings(&self) -> Result<ButtonMappings, String> {
        let _guard = lock(&self.access);
        Ok(self.load_mapping_configuration_unlocked()?.common_mappings)
    }

    pub fn load_mapping_configuration(&self) -> Result<MappingConfiguration, String> {
        let _guard = lock(&self.access);
        self.load_mapping_configuration_unlocked()
    }

    pub fn apply_current_mapping_configuration(
        &self,
        apply: impl FnOnce(&MappingConfiguration),
    ) -> Result<(), String> {
        let _guard = lock(&self.access);
        let configuration = self.load_mapping_configuration_unlocked()?;
        apply(&configuration);
        Ok(())
    }

    fn load_mapping_configuration_unlocked(&self) -> Result<MappingConfiguration, String> {
        let path = self.button_mappings_path();
        let contents = match fs::read_to_string(&path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                return Ok(MappingConfiguration::default());
            }
            Err(error) => return Err(format!("读取按键映射失败：{error}")),
        };
        let value: serde_json::Value = serde_json::from_str(&contents)
            .map_err(|error| format!("解析按键映射失败：{error}"))?;
        let configuration = if value.get("commonMappings").is_some() {
            serde_json::from_value::<MappingConfiguration>(value)
        } else if value.get("enabled").is_some() && value.get("actions").is_some() {
            serde_json::from_value::<ButtonMappings>(value).map(|common_mappings| {
                MappingConfiguration {
                    common_mappings,
                    ..Default::default()
                }
            })
        } else {
            return Err("按键映射格式缺少通用映射字段".to_owned());
        }
        .map_err(|error| format!("解析按键映射失败：{error}"))?;
        configuration
            .normalized()
            .map_err(|error| format!("按键映射无效：{error}"))
    }

    pub fn save_button_mappings(&self, mappings: ButtonMappings) -> Result<ButtonMappings, String> {
        self.save_button_mappings_with(mappings, |_| {})
    }

    pub fn save_button_mappings_with(
        &self,
        mappings: ButtonMappings,
        apply: impl FnOnce(&MappingConfiguration),
    ) -> Result<ButtonMappings, String> {
        let (configuration, ()) = self.update_mapping_configuration(
            |configuration| {
                configuration.common_mappings = mappings;
                Ok(())
            },
            apply,
        )?;
        Ok(configuration.common_mappings)
    }

    pub fn save_mapping_configuration(
        &self,
        configuration: MappingConfiguration,
    ) -> Result<MappingConfiguration, String> {
        self.save_mapping_configuration_with(configuration, |_| {})
    }

    pub fn save_mapping_configuration_with(
        &self,
        mut configuration: MappingConfiguration,
        apply: impl FnOnce(&MappingConfiguration),
    ) -> Result<MappingConfiguration, String> {
        let _guard = lock(&self.access);
        // A mapping editor may have opened before the menu preference changed.
        // Only the dedicated preference transaction owns this setting.
        let latest = self.load_mapping_configuration_unlocked()?;
        configuration.menu_update_default = latest.menu_update_default;
        configuration.remembered_template_id = if configuration.button_mapping_follow_enabled
            != latest.button_mapping_follow_enabled
        {
            None
        } else {
            latest.remembered_template_id
        };
        let saved = self.save_mapping_configuration_unlocked(configuration)?;
        apply(&saved);
        Ok(saved)
    }

    /// Save the program captured when our menu opened, merging into the latest file.
    pub fn save_program_default(
        &self,
        application_id: &str,
        template_id: &str,
        apply: impl FnOnce(&MappingConfiguration),
    ) -> Result<MappingConfiguration, String> {
        self.update_mapping_configuration(
            |configuration| {
                let mut binding = configuration
                    .application_bindings
                    .iter()
                    .find(|b| b.application_id.eq_ignore_ascii_case(application_id))
                    .cloned()
                    .unwrap_or(ApplicationBinding {
                        application_id: application_id.into(),
                        template_id: template_id.into(),
                        menu_order: 0,
                        launch_target: None,
                    });
                binding.template_id = template_id.into();
                configuration.upsert_application_binding(binding)
            },
            apply,
        )
        .map(|(saved, ())| saved)
    }

    pub fn update_mapping_configuration<R>(
        &self,
        update: impl FnOnce(&mut MappingConfiguration) -> Result<R, String>,
        apply: impl FnOnce(&MappingConfiguration),
    ) -> Result<(MappingConfiguration, R), String> {
        let _guard = lock(&self.access);
        let mut configuration = self.load_mapping_configuration_unlocked()?;
        let following_before = configuration.button_mapping_follow_enabled;
        let result = update(&mut configuration)?;
        if configuration.button_mapping_follow_enabled != following_before {
            configuration.remembered_template_id = None;
        }
        let saved = self.save_mapping_configuration_unlocked(configuration)?;
        apply(&saved);
        Ok((saved, result))
    }

    /// The engine has already applied this choice. Do not reapply mappings while
    /// remembering it: a disk callback must not cancel a newer engine request.
    pub fn save_remembered_template(
        &self,
        request_id: u64,
        template_id: Option<&str>,
        is_current: impl FnOnce() -> bool,
    ) -> Result<bool, String> {
        let _guard = lock(&self.access);
        let mut configuration = self.load_mapping_configuration_unlocked()?;
        if configuration.button_mapping_follow_enabled || !is_current() {
            sayall_windows::gatt_note(format!("manual_template_memory phase=save_skipped request_id={request_id} reason=selection_changed"));
            return Ok(false);
        }
        if template_id.is_some_and(|id| configuration.template_mappings(id).is_none()) {
            return Err("已应用的模板已不存在，未能记住本次选择".into());
        }
        configuration.remembered_template_id = template_id.map(str::to_owned);
        self.save_mapping_configuration_unlocked(configuration)?;
        Ok(true)
    }

    fn save_mapping_configuration_unlocked(
        &self,
        configuration: MappingConfiguration,
    ) -> Result<MappingConfiguration, String> {
        let result = (|| {
            let configuration = configuration
                .normalized()
                .map_err(|error| format!("按键映射无效：{error}"))?;
            let path = self.button_mappings_path();
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|error| format!("创建应用设置目录失败：{error}"))?;
            }
            let contents = serde_json::to_vec_pretty(&configuration)
                .map_err(|error| format!("序列化按键映射失败：{error}"))?;
            atomic_write(&path, &contents).map_err(|error| format!("保存按键映射失败：{error}"))?;
            self.revision.fetch_add(1, Ordering::AcqRel);
            Ok(configuration)
        })();
        sayall_windows::gatt_note(format!(
            "template_configuration phase=persisted terminal_result={} error_domain={} error_code={}",
            if result.is_ok() { "passed" } else { "failed" },
            if result.is_ok() { "none" } else { "settings" },
            if result.is_ok() {
                "none"
            } else {
                "validation_or_write_failed"
            }
        ));
        result
    }

    pub fn export_mapping_configuration(
        &self,
        path: &Path,
        configuration: MappingConfiguration,
        template_ids: Option<&[String]>,
    ) -> Result<(), String> {
        let mut configuration = configuration
            .normalized()
            .map_err(|error| format!("按键映射无效：{error}"))?;
        let mut builtin_template_ids: Vec<String> = configuration
            .application_bindings
            .iter()
            .filter(|binding| {
                sayall_windows::templates::is_builtin_template_id(&binding.template_id)
            })
            .map(|binding| binding.template_id.clone())
            .collect();
        builtin_template_ids.sort();
        builtin_template_ids.dedup();
        if let Some(template_ids) = template_ids {
            if template_ids.is_empty() {
                return Err("至少选择一个模板".to_owned());
            }
            let selected: std::collections::HashSet<_> = template_ids.iter().collect();
            if selected.len() != template_ids.len()
                || template_ids.iter().any(|id| {
                    !configuration
                        .templates
                        .iter()
                        .any(|template| &template.id == id)
                        && !sayall_windows::templates::is_builtin_template_id(id)
                })
            {
                return Err("所选模板不存在或重复".to_owned());
            }
            configuration
                .templates
                .retain(|template| selected.contains(&template.id));
            configuration
                .application_bindings
                .retain(|binding| selected.contains(&binding.template_id));
            builtin_template_ids = template_ids
                .iter()
                .filter(|id| sayall_windows::templates::is_builtin_template_id(id))
                .cloned()
                .collect();
        }
        let exported = MappingConfigurationExport {
            format_version: MAPPING_CONFIGURATION_EXPORT_VERSION,
            common_mappings: portable_common_mappings(&configuration)?,
            button_mapping_follow_enabled: configuration.button_mapping_follow_enabled,
            templates: configuration.templates,
            builtin_template_ids,
            application_bindings: configuration
                .application_bindings
                .iter()
                .map(Into::into)
                .collect(),
        };
        let contents = serde_json::to_vec_pretty(&exported)
            .map_err(|error| format!("序列化模板配置失败：{error}"))?;
        atomic_write(path, &contents).map_err(|error| format!("写入模板配置失败：{error}"))
    }

    pub fn apply_mapping_configuration_import(
        &self,
        token: &str,
    ) -> Result<MappingConfiguration, String> {
        self.apply_mapping_configuration_import_with(token, |_| {})
    }

    pub fn apply_mapping_configuration_import_with(
        &self,
        token: &str,
        apply: impl FnOnce(&MappingConfiguration),
    ) -> Result<MappingConfiguration, String> {
        let _guard = lock(&self.access);
        let pending = lock(&self.pending_imports)
            .remove(token)
            .ok_or_else(|| "导入预览已失效，请重新预览".to_owned())?;
        if pending.revision != self.revision.load(Ordering::Acquire) {
            sayall_windows::gatt_note(
                "template_import phase=rejected reason=stale_preview".to_owned(),
            );
            return Err("导入预览已过期，请重新预览".to_owned());
        }
        let saved = self.save_mapping_configuration_unlocked(pending.configuration)?;
        apply(&saved);
        Ok(saved)
    }

    pub fn preview_mapping_configuration_import(
        &self,
        path: &Path,
    ) -> Result<MappingConfigurationImportPreview, String> {
        let parsed = read_mapping_import(path)?;
        let _guard = lock(&self.access);
        let current = self.load_mapping_configuration_unlocked()?;
        let (format_version, configuration, builtin_template_ids) =
            resolve_mapping_import(parsed, &current)?;
        let revision = self.revision.load(Ordering::Acquire);
        let source_token = (format_version == 3).then(|| format!("source-{}", new_template_id()));
        // Choosing another file ends the previous preview session; no arbitrary
        // template quota or long-lived collection of file snapshots is needed.
        lock(&self.pending_imports).clear();
        lock(&self.import_sources).clear();
        if let Some(source_token) = &source_token {
            lock(&self.import_sources).insert(
                source_token.clone(),
                ImportSource {
                    revision,
                    new_template_ids: configuration
                        .templates
                        .iter()
                        .map(|template| (template.id.clone(), new_template_id()))
                        .collect(),
                    builtin_template_ids: builtin_template_ids.iter().cloned().collect(),
                    configuration: configuration.clone(),
                },
            );
        }
        let token = format!("apply-{}", new_template_id());
        lock(&self.pending_imports).insert(
            token.clone(),
            PendingImport {
                revision,
                configuration: configuration.clone(),
                source_token: source_token.clone(),
            },
        );
        let template_name_conflicts = configuration
            .templates
            .iter()
            .filter(|template| {
                current
                    .templates
                    .iter()
                    .any(|local| local.name == template.name)
            })
            .map(|template| template.name.clone())
            .collect();
        let unresolved_application_ids = unresolved_configuration(&configuration);
        let mut preview_configuration = configuration;
        preview_configuration.common_mappings = portable_common_mappings(&preview_configuration)?;
        for binding in &mut preview_configuration.application_bindings {
            binding.launch_target = None;
        }
        sayall_windows::gatt_note(format!(
            "template_import phase=previewed format_version={format_version} template_count={}",
            preview_configuration.templates.len()
        ));
        Ok(MappingConfigurationImportPreview {
            token,
            source_token,
            format_version,
            configuration: preview_configuration,
            builtin_template_ids: builtin_template_ids.into_iter().collect(),
            template_name_conflicts,
            unresolved_application_ids,
        })
    }

    pub fn preview_template_import(
        &self,
        source_token: &str,
        request: TemplateImportRequest,
    ) -> Result<TemplateImportPreview, String> {
        let _guard = lock(&self.access);
        let source = lock(&self.import_sources)
            .get(source_token)
            .cloned()
            .ok_or_else(|| "模板来源预览已失效，请重新选择文件".to_owned())?;
        if source.revision != self.revision.load(Ordering::Acquire) {
            return Err("模板来源预览已过期，请重新选择文件".to_owned());
        }
        // A changed selection invalidates prior final previews, even if its new
        // name validation fails. The source token itself can never be applied.
        lock(&self.pending_imports)
            .retain(|_, pending| pending.source_token.as_deref() != Some(source_token));
        let selected: HashSet<_> = request.template_ids.iter().collect();
        if selected.is_empty()
            || selected.len() != request.template_ids.len()
            || selected.iter().any(|id| {
                !source.new_template_ids.contains_key(*id)
                    && !source.builtin_template_ids.contains(*id)
            })
        {
            return Err("所选模板不存在、重复或为空".to_owned());
        }
        if request
            .resolved_names
            .keys()
            .any(|id| !selected.contains(id) || source.builtin_template_ids.contains(id))
        {
            return Err("拟用名称包含未选择或只读内置模板".to_owned());
        }
        let current = self.load_mapping_configuration_unlocked()?;
        let mut next = current.clone();
        let mut templates = Vec::new();
        for original in &source.configuration.templates {
            if !selected.contains(&original.id) {
                continue;
            }
            let mut imported = original.clone();
            imported.id = source.new_template_ids[&original.id].clone();
            imported.name = request
                .resolved_names
                .get(&original.id)
                .unwrap_or(&original.name)
                .trim()
                .to_owned();
            templates.push(ImportedTemplatePreview {
                source_template_id: original.id.clone(),
                template: imported.clone(),
            });
            next.templates.push(imported);
        }
        let mut added_application_bindings = Vec::new();
        let mut replaced_application_ids = Vec::new();
        let mut skipped_application_ids = Vec::new();
        let mut changed_bindings = Vec::new();
        for imported in &source.configuration.application_bindings {
            if !selected.contains(&imported.template_id) {
                continue;
            }
            let existing = current
                .application_bindings
                .iter()
                .find(|binding| binding.application_id == imported.application_id);
            if existing.is_some() && !request.replace_application_bindings {
                skipped_application_ids.push(imported.application_id.clone());
                continue;
            }
            let binding = ApplicationBinding {
                application_id: imported.application_id.clone(),
                template_id: source
                    .new_template_ids
                    .get(&imported.template_id)
                    .cloned()
                    .unwrap_or_else(|| imported.template_id.clone()),
                menu_order: imported.menu_order,
                launch_target: local_launch_target(existing),
            };
            if existing.is_some() {
                replaced_application_ids.push(binding.application_id.clone());
                next.application_bindings
                    .retain(|old| old.application_id != binding.application_id);
            }
            added_application_bindings.push((&binding).into());
            changed_bindings.push(binding.clone());
            next.application_bindings.push(binding);
        }
        let next = next.normalized()?;
        let token = format!("apply-{}", new_template_id());
        lock(&self.pending_imports).insert(
            token.clone(),
            PendingImport {
                revision: source.revision,
                configuration: next,
                source_token: Some(source_token.to_owned()),
            },
        );
        sayall_windows::gatt_note(format!(
            "template_import phase=selected_previewed templates={} bindings_added={} bindings_replaced={} bindings_skipped={}",
            templates.len(),
            added_application_bindings.len(),
            replaced_application_ids.len(),
            skipped_application_ids.len()
        ));
        Ok(TemplateImportPreview {
            token,
            templates,
            added_application_bindings,
            replaced_application_ids,
            skipped_application_ids,
            unresolved_application_ids: unresolved_applications(&changed_bindings),
        })
    }
    pub fn load_voice_hold_hotkey(&self) -> Result<Option<KeyChord>, String> {
        let _guard = lock(&self.access);
        self.load_voice_hold_hotkey_unlocked()
    }

    /// v1 默认按住说话快捷键：左 Ctrl + 左 Win（适配微信输入法的默认语音热键）。
    pub fn default_voice_hold_hotkey() -> Option<KeyChord> {
        Some(KeyChord {
            keys: vec![
                sayall_windows::send_input::KeyCode::LeftControl,
                sayall_windows::send_input::KeyCode::LeftWindows,
            ],
        })
    }

    fn load_voice_hold_hotkey_unlocked(&self) -> Result<Option<KeyChord>, String> {
        let path = self.voice_hold_hotkey_path();
        let contents = match fs::read_to_string(&path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                return Ok(Self::default_voice_hold_hotkey());
            }
            Err(error) => return Err(format!("读取按住说话快捷键失败：{error}")),
        };
        serde_json::from_str::<Option<KeyChord>>(&contents)
            .map_err(|error| format!("解析按住说话快捷键失败：{error}"))
    }

    pub fn save_voice_hold_hotkey(
        &self,
        hotkey: Option<KeyChord>,
    ) -> Result<Option<KeyChord>, String> {
        let _guard = lock(&self.access);
        self.save_voice_hold_hotkey_unlocked(hotkey)
    }

    fn save_voice_hold_hotkey_unlocked(
        &self,
        hotkey: Option<KeyChord>,
    ) -> Result<Option<KeyChord>, String> {
        if let Some(chord) = &hotkey {
            chord
                .clone()
                .validated()
                .map_err(|error| format!("按住说话快捷键无效：{error}"))?;
        }
        let path = self.voice_hold_hotkey_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| format!("创建应用设置目录失败：{error}"))?;
        }
        let contents = serde_json::to_vec_pretty(&hotkey)
            .map_err(|error| format!("序列化按住说话快捷键失败：{error}"))?;
        fs::write(path, contents).map_err(|error| format!("保存按住说话快捷键失败：{error}"))?;
        Ok(hotkey)
    }

    /// 「其他工具」面板记住的按键（2026-10-01 Andy 反馈：选了「不按键 / 左 Alt」
    /// 之后切去豆包再切回来，会退回默认的右 Alt——因为选中别的工具会改写
    /// 按住说话快捷键，而「其他工具」没有自己的记忆）。
    ///
    /// `None` = 从未选过（保持现状）；`Some(vec![])` = 明确选了「不按键」。
    pub fn load_other_voice_hotkey(&self) -> Result<Option<Vec<KeyCode>>, String> {
        let _guard = lock(&self.access);
        let path = self.other_voice_hotkey_path();
        let contents = match fs::read_to_string(&path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(format!("读取「其他工具」按键记忆失败：{error}")),
        };
        serde_json::from_str::<Option<Vec<KeyCode>>>(&contents)
            .map_err(|error| format!("解析「其他工具」按键记忆失败：{error}"))
    }

    pub fn save_other_voice_hotkey(
        &self,
        keys: Option<Vec<KeyCode>>,
    ) -> Result<Option<Vec<KeyCode>>, String> {
        let _guard = lock(&self.access);
        if let Some(keys) = &keys {
            if !keys.is_empty() {
                KeyChord { keys: keys.clone() }
                    .validated()
                    .map_err(|error| format!("「其他工具」按键无效：{error}"))?;
            }
        }
        let path = self.other_voice_hotkey_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| format!("创建应用设置目录失败：{error}"))?;
        }
        let contents = serde_json::to_vec_pretty(&keys)
            .map_err(|error| format!("序列化「其他工具」按键记忆失败：{error}"))?;
        fs::write(path, contents)
            .map_err(|error| format!("保存「其他工具」按键记忆失败：{error}"))?;
        Ok(keys)
    }

    fn button_mappings_path(&self) -> PathBuf {
        self.path.with_file_name("button-mappings.json")
    }

    fn voice_hold_hotkey_path(&self) -> PathBuf {
        self.path.with_file_name("voice-hold-hotkey.json")
    }

    fn other_voice_hotkey_path(&self) -> PathBuf {
        self.path.with_file_name("other-voice-hotkey.json")
    }

    fn update(&self, operation: &str, update: impl FnOnce(&mut AppSettings)) -> Result<(), String> {
        let _guard = lock(&self.access);
        self.update_unlocked(operation, update)
    }

    fn update_unlocked(
        &self,
        operation: &str,
        update: impl FnOnce(&mut AppSettings),
    ) -> Result<(), String> {
        let mut settings = self.load_unlocked()?;
        settings.schema_version = AppSettings::default().schema_version;
        update(&mut settings);
        let contents = serialize_settings(&settings)?;
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|error| format!("创建应用设置目录失败：{error}"))?;
        }
        fs::write(&self.path, contents).map_err(|error| format!("{operation}失败：{error}"))
    }
}

fn read_mapping_import(path: &Path) -> Result<ImportedMappingConfiguration, String> {
    use std::io::Read;
    let file = fs::File::open(path).map_err(|error| format!("读取模板配置失败：{error}"))?;
    let mut bytes = Vec::new();
    file.take(MAX_BUTTON_MAPPING_IMPORT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("读取模板配置失败：{error}"))?;
    if bytes.len() as u64 > MAX_BUTTON_MAPPING_IMPORT_BYTES {
        return Err("模板配置文件过大".to_owned());
    }
    serde_json::from_slice(&bytes).map_err(|error| format!("解析模板配置失败：{error}"))
}

fn local_launch_target(binding: Option<&ApplicationBinding>) -> Option<String> {
    binding
        .and_then(|binding| binding.launch_target.as_ref())
        .filter(|target| Path::new(target).is_absolute() && Path::new(target).is_file())
        .cloned()
}

fn unresolved_applications(bindings: &[ApplicationBinding]) -> Vec<String> {
    bindings
        .iter()
        .filter(|binding| {
            binding.launch_target.is_none()
                && sayall_windows::app_launcher::preset_app(&binding.application_id).is_none()
        })
        .map(|binding| binding.application_id.clone())
        .collect()
}

fn logical_application_id(value: &str) -> bool {
    !value.trim().is_empty()
        && !value
            .chars()
            .any(|ch| ch.is_control() || matches!(ch, ':' | '/' | '\\'))
}

fn portable_common_mappings(
    configuration: &MappingConfiguration,
) -> Result<ButtonMappings, String> {
    if configuration
        .application_bindings
        .iter()
        .any(|binding| !logical_application_id(&binding.application_id))
    {
        return Err("应用 ID 必须为逻辑引用，不能包含路径或协议".to_owned());
    }
    let mut mappings = configuration.common_mappings.clone();
    // Installed application targets belong to this machine, not a portable template.
    mappings.applications.clear();
    let mut references = HashMap::<String, String>::new();
    let mut reserved: HashSet<String> = configuration
        .application_bindings
        .iter()
        .map(|binding| binding.application_id.clone())
        .collect();
    for actions in mappings.actions.values() {
        for action in [&actions.single, &actions.double, &actions.long] {
            if let ButtonAction::OpenApp { target } = action {
                if logical_application_id(target) {
                    reserved.insert(target.clone());
                }
            }
        }
    }
    let mut sequence = 0;
    for actions in mappings.actions.values_mut() {
        for action in [&mut actions.single, &mut actions.double, &mut actions.long] {
            if let ButtonAction::OpenApp { target } = action {
                if logical_application_id(target) {
                    continue;
                }
                let reference = references.entry(target.clone()).or_insert_with(|| {
                    if let Some(binding) = configuration
                        .application_bindings
                        .iter()
                        .find(|binding| binding.launch_target.as_deref() == Some(target.as_str()))
                    {
                        return binding.application_id.clone();
                    }
                    loop {
                        sequence += 1;
                        let candidate = format!("custom-app-{sequence}");
                        if reserved.insert(candidate.clone()) {
                            return candidate;
                        }
                    }
                });
                *target = reference.clone();
            }
        }
    }
    Ok(mappings)
}

fn unresolved_configuration(configuration: &MappingConfiguration) -> Vec<String> {
    let mut unresolved: std::collections::BTreeSet<_> =
        unresolved_applications(&configuration.application_bindings)
            .into_iter()
            .collect();
    for actions in configuration.common_mappings.actions.values() {
        for action in [&actions.single, &actions.double, &actions.long] {
            if let ButtonAction::OpenApp { target } = action {
                if logical_application_id(target)
                    && sayall_windows::app_launcher::preset_app(target).is_none()
                {
                    unresolved.insert(target.clone());
                }
            }
        }
    }
    unresolved.into_iter().collect()
}

fn resolve_mapping_import(
    parsed: ImportedMappingConfiguration,
    current: &MappingConfiguration,
) -> Result<(u32, MappingConfiguration, Vec<String>), String> {
    let (version, mut configuration, builtin_template_ids) = match parsed {
        ImportedMappingConfiguration::V1(v) if v.format_version == 1 => (
            1,
            MappingConfiguration {
                common_mappings: v.button_mappings,
                ..current.clone()
            },
            Vec::new(),
        ),
        ImportedMappingConfiguration::V3(v) if v.format_version == 3 => {
            let builtin_template_ids = v.builtin_template_ids;
            (
                3,
                MappingConfiguration {
                    common_mappings: v.common_mappings,
                    mapping_notice_enabled: current.mapping_notice_enabled,
                    menu_template_switch_enabled: current.menu_template_switch_enabled,
                    menu_update_default: current.menu_update_default,
                    button_mapping_follow_enabled: v.button_mapping_follow_enabled,
                    remembered_template_id: current.remembered_template_id.clone(),
                    templates: v.templates,
                    application_bindings: v
                        .application_bindings
                        .into_iter()
                        .map(|binding| {
                            let local = current
                                .application_bindings
                                .iter()
                                .find(|local| local.application_id == binding.application_id);
                            ApplicationBinding {
                                launch_target: local_launch_target(local),
                                application_id: binding.application_id,
                                template_id: binding.template_id,
                                menu_order: binding.menu_order,
                            }
                        })
                        .collect(),
                },
                builtin_template_ids,
            )
        }
        ImportedMappingConfiguration::V1(v) => {
            return Err(format!("不支持的按键映射配置版本：{}", v.format_version));
        }
        ImportedMappingConfiguration::V3(v) => {
            return Err(format!("不支持的模板配置版本：{}", v.format_version));
        }
    };
    if version == 3 {
        let builtin_ids: HashSet<_> = builtin_template_ids.iter().collect();
        if builtin_ids.len() != builtin_template_ids.len()
            || builtin_template_ids
                .iter()
                .any(|id| !sayall_windows::templates::is_builtin_template_id(id))
            || configuration.application_bindings.iter().any(|binding| {
                sayall_windows::templates::is_builtin_template_id(&binding.template_id)
                    && !builtin_ids.contains(&binding.template_id)
            })
        {
            return Err("导入配置包含无效或未声明的内置模板引用".to_owned());
        }
        if configuration
            .application_bindings
            .iter()
            .any(|binding| !logical_application_id(&binding.application_id))
        {
            return Err("导入应用 ID 必须为逻辑引用".to_owned());
        }
        for actions in configuration.common_mappings.actions.values_mut() {
            for action in [&mut actions.single, &mut actions.double, &mut actions.long] {
                if let ButtonAction::OpenApp { target } = action {
                    if !logical_application_id(target) {
                        return Err("v3 导入不接受启动路径或协议，请使用逻辑应用引用".to_owned());
                    }
                    if sayall_windows::app_launcher::preset_app(target).is_none() {
                        if let Some(local) = local_launch_target(
                            current
                                .application_bindings
                                .iter()
                                .find(|binding| binding.application_id == *target),
                        ) {
                            *target = local;
                        }
                    }
                }
            }
        }
    }
    let declared: HashSet<_> = builtin_template_ids.into_iter().collect();
    let builtin_template_ids = MappingConfiguration::recommended_templates()
        .into_iter()
        .map(|template| template.id)
        .filter(|id| declared.contains(id) && !configuration.templates.iter().any(|t| &t.id == id))
        .collect();
    Ok((version, configuration.normalized()?, builtin_template_ids))
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn log_onboarding_write_failure(reason: &str, retryable: bool) {
    sayall_windows::gatt_note(format!(
        "onboarding event=state_write phase=completed terminal_result=failed error_domain=settings error_code=write_failed reason={reason} retryable={retryable}"
    ));
}

fn parse_settings(contents: &str) -> Result<AppSettings, String> {
    serde_json::from_str(contents)
        .map(AppSettings::normalized)
        .map_err(|error| format!("解析应用设置失败：{error}"))
}

fn serialize_settings(settings: &AppSettings) -> Result<Vec<u8>, String> {
    serde_json::to_vec_pretty(settings).map_err(|error| format!("序列化应用设置失败：{error}"))
}

fn atomic_write(path: &Path, contents: &[u8]) -> Result<(), std::io::Error> {
    use std::io::Write;

    let parent = path.parent().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "配置文件缺少父目录")
    })?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "配置文件名无效"))?;
    let temp = parent.join(format!(
        ".{file_name}.{}.{}.tmp",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let write_result = (|| {
        let mut file = fs::File::create(&temp)?;
        file.write_all(contents)?;
        file.sync_all()?;
        drop(file);
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            use windows::core::PCWSTR;
            use windows::Win32::Storage::FileSystem::{
                MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
            };
            let source: Vec<u16> = temp.as_os_str().encode_wide().chain(Some(0)).collect();
            let destination: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
            unsafe {
                MoveFileExW(
                    PCWSTR(source.as_ptr()),
                    PCWSTR(destination.as_ptr()),
                    MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
                )
                .map_err(std::io::Error::other)?;
            }
            Ok(())
        }
        #[cfg(not(windows))]
        {
            fs::rename(&temp, path)
        }
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    write_result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn template_test_store() -> SettingsStore {
        let directory =
            std::env::temp_dir().join(format!("sayall-template-test-{}", new_template_id()));
        fs::create_dir_all(&directory).unwrap();
        SettingsStore::new(directory.join("settings.json"))
    }

    #[test]
    fn remembered_template_transaction_preserves_latest_choice_through_editor_save_and_reopen() {
        let store = template_test_store();
        let mut stale_editor = store.load_mapping_configuration().unwrap();
        assert!(store
            .save_remembered_template(
                4,
                Some(sayall_windows::templates::BUILTIN_AGENT_TEMPLATE_ID),
                || true
            )
            .unwrap());
        stale_editor.mapping_notice_enabled = false;
        let saved = store.save_mapping_configuration(stale_editor).unwrap();
        assert_eq!(
            saved.remembered_template_id.as_deref(),
            Some(sayall_windows::templates::BUILTIN_AGENT_TEMPLATE_ID)
        );
        let reopened = SettingsStore::new(store.path.clone())
            .load_mapping_configuration()
            .unwrap();
        assert_eq!(reopened, saved);
        assert!(store.save_remembered_template(5, None, || true).unwrap());
        assert!(SettingsStore::new(store.path.clone())
            .load_mapping_configuration()
            .unwrap()
            .remembered_template_id
            .is_none());
    }

    #[test]
    fn remembered_template_transaction_rejects_stale_missing_and_auto_choices() {
        let store = template_test_store();
        let original = store.load_mapping_configuration().unwrap();
        assert!(!store
            .save_remembered_template(
                1,
                Some(sayall_windows::templates::BUILTIN_AGENT_TEMPLATE_ID),
                || false
            )
            .unwrap());
        assert!(store
            .save_remembered_template(2, Some("missing-template"), || true)
            .is_err());
        assert_eq!(store.load_mapping_configuration().unwrap(), original);
        store
            .update_mapping_configuration(
                |c| {
                    c.button_mapping_follow_enabled = true;
                    Ok(())
                },
                |_| {},
            )
            .unwrap();
        assert!(!store
            .save_remembered_template(
                3,
                Some(sayall_windows::templates::BUILTIN_AGENT_TEMPLATE_ID),
                || true
            )
            .unwrap());
        assert!(store
            .load_mapping_configuration()
            .unwrap()
            .remembered_template_id
            .is_none());
    }

    #[test]
    fn remembered_template_mode_switch_delete_and_rename_have_distinct_memory_semantics() {
        let store = template_test_store();
        let (_, template) = store
            .update_mapping_configuration(|c| c.create_template("local test".into()), |_| {})
            .unwrap();
        store
            .save_remembered_template(1, Some(&template.id), || true)
            .unwrap();
        let renamed = store
            .update_mapping_configuration(
                |c| {
                    c.templates[0].name = "renamed test".into();
                    Ok(())
                },
                |_| {},
            )
            .unwrap()
            .0;
        assert_eq!(
            renamed.remembered_template_id.as_deref(),
            Some(template.id.as_str())
        );
        let deleted = store
            .update_mapping_configuration(
                |c| {
                    c.templates.clear();
                    Ok(())
                },
                |_| {},
            )
            .unwrap()
            .0;
        assert!(deleted.remembered_template_id.is_none());
        store
            .save_remembered_template(
                2,
                Some(sayall_windows::templates::BUILTIN_AGENT_TEMPLATE_ID),
                || true,
            )
            .unwrap();
        for following in [true, false] {
            let changed = store
                .update_mapping_configuration(
                    |c| {
                        c.button_mapping_follow_enabled = following;
                        Ok(())
                    },
                    |_| {},
                )
                .unwrap()
                .0;
            assert!(changed.remembered_template_id.is_none());
        }
    }

    #[cfg(windows)]
    #[test]
    fn remembered_template_write_failure_leaves_previous_file_and_returns_failure() {
        use std::os::windows::fs::OpenOptionsExt;
        let store = template_test_store();
        let path = store.button_mappings_path();
        store
            .save_mapping_configuration(MappingConfiguration::default())
            .unwrap();
        let before = fs::read(&path).unwrap();
        let guard = fs::OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(&path)
            .unwrap();
        assert!(store
            .save_remembered_template(
                1,
                Some(sayall_windows::templates::BUILTIN_AGENT_TEMPLATE_ID),
                || true
            )
            .is_err());
        assert_eq!(fs::read(&path).unwrap(), before);
        assert!(store
            .load_mapping_configuration()
            .unwrap()
            .remembered_template_id
            .is_none());
        drop(guard);
    }

    #[test]
    fn capture_and_writer_save_together_preserves_unrelated_settings() {
        let store = template_test_store();
        store.save_rc003_capture_enabled(true).unwrap();
        let mut expected = store.load().unwrap();
        let capture = sayall_core::CaptureInputSettings {
            enabled: true,
            endpoint_id: Some("capture-test".into()),
            endpoint_name: Some("capture-test".into()),
        };
        let audio = sayall_windows::AudioSnapshot {
            selected_endpoint_id: Some("render-test".into()),
            selected_endpoint_name: Some("render-test".into()),
            ..Default::default()
        };
        expected.capture_input = capture.clone();
        expected.audio_endpoint_id = audio.selected_endpoint_id.clone();
        expected.audio_endpoint_name = audio.selected_endpoint_name.clone();
        store.save_capture_audio(capture, audio).unwrap();
        assert_eq!(store.load().unwrap(), expected);
    }

    #[test]
    fn program_default_merges_latest_config_and_only_changes_captured_binding() {
        let store = template_test_store();
        let mut configuration = source_configuration();
        configuration.application_bindings.push(ApplicationBinding {
            application_id: "original".into(),
            template_id: configuration.templates[0].id.clone(),
            menu_order: 7,
            launch_target: Some("retained-local-target".into()),
        });
        store.save_mapping_configuration(configuration).unwrap();
        // An unrelated edit after opening the menu must survive the later selection.
        store
            .update_mapping_configuration(
                |c| {
                    c.mapping_notice_enabled = false;
                    c.templates[0].name = "Newer edit".into();
                    Ok(())
                },
                |_| {},
            )
            .unwrap();
        let mut expected = store.load_mapping_configuration().unwrap();
        let index = expected
            .application_bindings
            .iter()
            .position(|b| b.application_id == "original")
            .unwrap();
        expected.application_bindings[index].template_id =
            sayall_windows::templates::BUILTIN_CHAT_TEMPLATE_ID.into();
        let saved = store
            .save_program_default(
                "original",
                sayall_windows::templates::BUILTIN_CHAT_TEMPLATE_ID,
                |_| {},
            )
            .unwrap();
        assert_eq!(saved, expected);
        assert_eq!(
            SettingsStore::new(store.path.clone())
                .load_mapping_configuration()
                .unwrap(),
            expected
        );
        let applied = std::cell::Cell::new(false);
        assert!(store
            .save_program_default("original", "missing-template", |_| applied.set(true))
            .is_err());
        assert!(!applied.get());
        assert_eq!(store.load_mapping_configuration().unwrap(), expected);
    }

    #[test]
    fn menu_and_ui_preferences_merge_latest_fields_and_survive_store_reopen() {
        let store = template_test_store();
        let original = source_configuration();
        store.save_mapping_configuration(original.clone()).unwrap();
        store.save_rc003_capture_enabled(true).unwrap();
        std::thread::scope(|scope| {
            scope.spawn(|| {
                store
                    .save_ui_preference(sayall_core::UiPreference::LockButtonSelection, false)
                    .unwrap()
            });
            scope.spawn(|| {
                store
                    .save_ui_preference(sayall_core::UiPreference::TemplatesExpanded, false)
                    .unwrap()
            });
            scope.spawn(|| {
                store
                    .save_ui_preference(sayall_core::UiPreference::AssociationsExpanded, false)
                    .unwrap()
            });
            scope.spawn(|| {
                store
                    .update_mapping_configuration(
                        |c| {
                            c.menu_update_default = true;
                            Ok(())
                        },
                        |_| {},
                    )
                    .unwrap()
            });
            scope.spawn(|| {
                store
                    .update_mapping_configuration(
                        |c| {
                            c.templates[0].name = "new user edit".into();
                            Ok(())
                        },
                        |_| {},
                    )
                    .unwrap()
            });
        });
        let reopened = SettingsStore::new(store.path.clone());
        let app = reopened.load().unwrap();
        assert!(app.rc003_capture_enabled);
        assert_eq!(
            app.ui_preferences,
            sayall_core::UiPreferences {
                lock_button_selection: false,
                templates_expanded: false,
                associations_expanded: false,
            }
        );
        let mut expected = original;
        expected.menu_update_default = true;
        expected.templates[0].name = "new user edit".into();
        assert_eq!(reopened.load_mapping_configuration().unwrap(), expected);
        let mut stale_editor = expected.clone();
        stale_editor.menu_update_default = false;
        assert!(
            store
                .save_mapping_configuration(stale_editor)
                .unwrap()
                .menu_update_default
        );
        let before = fs::read(store.button_mappings_path()).unwrap();
        let applied = std::cell::Cell::new(false);
        assert!(store
            .update_mapping_configuration(
                |c| {
                    c.templates[0].id.clear();
                    c.menu_update_default = false;
                    Ok(())
                },
                |_| applied.set(true)
            )
            .is_err());
        assert!(!applied.get());
        assert_eq!(fs::read(store.button_mappings_path()).unwrap(), before);
        fs::write(&store.path, "invalid settings").unwrap();
        assert!(store
            .save_ui_preference(sayall_core::UiPreference::TemplatesExpanded, true)
            .is_err());
        assert_eq!(fs::read_to_string(&store.path).unwrap(), "invalid settings");
    }

    #[test]
    fn mapping_notice_preference_persists_without_changing_mapping_configuration() {
        let store = template_test_store();
        let original = source_configuration();
        store.save_mapping_configuration(original.clone()).unwrap();
        store
            .update_mapping_configuration(
                |value| {
                    value.mapping_notice_enabled = false;
                    Ok(())
                },
                |_| {},
            )
            .unwrap();
        let reopened = SettingsStore::new(store.path.clone());
        let mut loaded = reopened.load_mapping_configuration().unwrap();
        assert!(!loaded.mapping_notice_enabled);
        loaded.mapping_notice_enabled = true;
        assert_eq!(loaded, original);
        reopened
            .update_mapping_configuration(
                |value| {
                    value.mapping_notice_enabled = true;
                    Ok(())
                },
                |_| {},
            )
            .unwrap();
        assert_eq!(store.load_mapping_configuration().unwrap(), original);
        let absent: MappingConfiguration = serde_json::from_str("{}").unwrap();
        assert!(absent.mapping_notice_enabled);
    }

    #[test]
    fn enhancement_and_menu_opt_ins_reopen_without_replacing_other_preferences() {
        let store = template_test_store();
        let mut settings = store.load().unwrap();
        assert!(!settings.rc003_capture_enabled);
        store.save_theme_preference(ThemePreference::Dark).unwrap();
        settings = store.load().unwrap();
        store.save_rc003_capture_enabled(true).unwrap();
        settings.rc003_capture_enabled = true;
        assert_eq!(
            SettingsStore::new(store.path.clone()).load().unwrap(),
            settings
        );
        let mut configuration = source_configuration();
        assert!(!configuration.menu_template_switch_enabled);
        configuration.menu_template_switch_enabled = true;
        store
            .save_mapping_configuration(configuration.clone())
            .unwrap();
        assert_eq!(
            SettingsStore::new(store.path.clone())
                .load_mapping_configuration()
                .unwrap(),
            configuration
        );
        store.save_rc003_capture_enabled(false).unwrap();
        settings.rc003_capture_enabled = false;
        assert_eq!(store.load().unwrap(), settings);
        assert_eq!(store.load_mapping_configuration().unwrap(), configuration);
    }

    fn source_configuration() -> MappingConfiguration {
        let mut configuration = MappingConfiguration::default();
        let first = configuration.create_template("阅读".to_owned()).unwrap();
        let second = configuration.create_template("演示".to_owned()).unwrap();
        for (application_id, template_id) in [("edge", first.id), ("custom-reader", second.id)] {
            configuration.application_bindings.push(ApplicationBinding {
                application_id: application_id.to_owned(),
                template_id,
                menu_order: 0,
                launch_target: None,
            });
        }
        configuration
    }

    fn source_file(store: &SettingsStore, configuration: MappingConfiguration) -> PathBuf {
        let path = store
            .path
            .with_file_name(format!("source-{}.json", new_template_id()));
        store
            .export_mapping_configuration(&path, configuration, None)
            .unwrap();
        path
    }

    fn selected_request(ids: Vec<String>) -> TemplateImportRequest {
        TemplateImportRequest {
            template_ids: ids,
            resolved_names: BTreeMap::new(),
            replace_application_bindings: false,
        }
    }

    #[test]
    fn template_legacy_enabled_actions_is_not_silently_read_as_defaults() {
        use sayall_windows::raw_input::RemoteButton;
        use sayall_windows::send_input::{ButtonActions, KeyCode};
        let store = template_test_store();
        let mut mappings = ButtonMappings::default();
        mappings.enabled = false;
        mappings.actions.insert(
            RemoteButton::Back,
            ButtonActions {
                single: ButtonAction::Shortcut {
                    chord: KeyChord {
                        keys: vec![KeyCode::Escape],
                    },
                },
                ..Default::default()
            },
        );
        fs::write(
            store.button_mappings_path(),
            serde_json::to_vec(&mappings).unwrap(),
        )
        .unwrap();
        let loaded = store.load_mapping_configuration().unwrap();
        assert_eq!(loaded.common_mappings, mappings);
        assert!(!loaded.button_mapping_follow_enabled);
        assert!(!loaded.button_mapping_follow_enabled);
        assert!(loaded.templates.is_empty());
        store.save_mapping_configuration(loaded.clone()).unwrap();
        assert_eq!(store.load_mapping_configuration().unwrap(), loaded);
        fs::write(store.button_mappings_path(), b"{}").unwrap();
        assert!(store.load_mapping_configuration().is_err());
    }

    #[test]
    fn template_v1_import_changes_only_common_and_applies_once() {
        let store = template_test_store();
        let mut original = source_configuration();
        original.button_mapping_follow_enabled = true;
        store.save_mapping_configuration(original.clone()).unwrap();
        store.save_theme_preference(ThemePreference::Dark).unwrap();
        let unrelated_settings = fs::read(&store.path).unwrap();
        let mut mappings = ButtonMappings::default();
        mappings.enabled = false;
        let path = store.path.with_file_name("v1.json");
        fs::write(
            &path,
            serde_json::to_vec(&ButtonMappingConfiguration {
                format_version: 1,
                button_mappings: mappings.clone(),
            })
            .unwrap(),
        )
        .unwrap();
        let preview = store.preview_mapping_configuration_import(&path).unwrap();
        assert_eq!(preview.source_token, None);
        let mut runtime = original.clone();
        let saved = store
            .apply_mapping_configuration_import_with(&preview.token, |configuration| {
                runtime = configuration.clone()
            })
            .unwrap();
        assert_eq!(saved.common_mappings, mappings);
        assert_eq!(saved.templates, original.templates);
        assert_eq!(saved.application_bindings, original.application_bindings);
        assert!(saved.button_mapping_follow_enabled);
        assert_eq!(runtime, saved);
        assert_eq!(fs::read(&store.path).unwrap(), unrelated_settings);
        assert!(store
            .apply_mapping_configuration_import(&preview.token)
            .is_err());
    }

    #[test]
    fn template_selected_import_resolves_names_preserves_global_and_skips_bindings() {
        let store = template_test_store();
        let mut original = source_configuration();
        original.button_mapping_follow_enabled = true;
        store.save_mapping_configuration(original.clone()).unwrap();
        let source = source_configuration();
        let path = source_file(&store, source.clone());
        let initial = store.preview_mapping_configuration_import(&path).unwrap();
        let source_token = initial.source_token.as_ref().unwrap();
        assert!(store
            .apply_mapping_configuration_import(source_token)
            .is_err());
        let first_id = source.templates[0].id.clone();
        let mut request = selected_request(vec![first_id.clone()]);
        let before = fs::read(store.button_mappings_path()).unwrap();
        assert!(store
            .preview_template_import(source_token, request.clone())
            .is_err());
        assert_eq!(fs::read(store.button_mappings_path()).unwrap(), before);
        request
            .resolved_names
            .insert(first_id.clone(), "阅读副本".to_owned());
        let first = store
            .preview_template_import(source_token, request.clone())
            .unwrap();
        let second = store
            .preview_template_import(source_token, request)
            .unwrap();
        assert_eq!(first.templates, second.templates);
        assert_ne!(first.token, second.token);
        assert_ne!(second.templates[0].template.id, first_id);
        assert_eq!(second.skipped_application_ids, vec!["edge"]);
        assert!(second.added_application_bindings.is_empty());
        assert!(store
            .apply_mapping_configuration_import(&initial.token)
            .is_err());
        assert!(store
            .apply_mapping_configuration_import(&first.token)
            .is_err());
        let saved = store
            .apply_mapping_configuration_import(&second.token)
            .unwrap();
        assert_eq!(saved.common_mappings, original.common_mappings);
        assert_eq!(saved.application_bindings, original.application_bindings);
        assert!(saved.button_mapping_follow_enabled);
        assert_eq!(
            &saved.templates[..original.templates.len()],
            original.templates.as_slice()
        );
        assert_eq!(saved.templates.len(), original.templates.len() + 1);
    }

    #[test]
    fn template_selected_binding_replacement_is_explicit_and_keeps_valid_local_target() {
        let store = template_test_store();
        let mut current = source_configuration();
        let local_target = store.path.with_file_name("local-reader.exe");
        fs::write(&local_target, b"test fixture, never executed").unwrap();
        current.application_bindings[0].launch_target =
            Some(local_target.to_string_lossy().into_owned());
        store.save_mapping_configuration(current.clone()).unwrap();
        let source = source_configuration();
        let path = source_file(&store, source.clone());
        let initial = store.preview_mapping_configuration_import(&path).unwrap();
        assert!(initial
            .configuration
            .application_bindings
            .iter()
            .all(|binding| binding.launch_target.is_none()));
        let mut request = selected_request(
            source
                .templates
                .iter()
                .map(|template| template.id.clone())
                .collect(),
        );
        request.replace_application_bindings = true;
        for template in &source.templates {
            request
                .resolved_names
                .insert(template.id.clone(), format!("{}副本", template.name));
        }
        let preview = store
            .preview_template_import(initial.source_token.as_ref().unwrap(), request)
            .unwrap();
        assert_eq!(preview.replaced_application_ids.len(), 2);
        let saved = store
            .apply_mapping_configuration_import(&preview.token)
            .unwrap();
        assert_eq!(
            saved.application_bindings[0].launch_target,
            current.application_bindings[0].launch_target
        );
        assert_ne!(
            saved.application_bindings[0].template_id,
            current.application_bindings[0].template_id
        );
    }

    #[test]
    fn template_tokens_are_unique_one_use_and_reject_stale_revision() {
        let store = template_test_store();
        let source = source_configuration();
        let path = source_file(&store, source);
        let first = store.preview_mapping_configuration_import(&path).unwrap();
        let second = store.preview_mapping_configuration_import(&path).unwrap();
        assert_ne!(first.token, second.token);
        assert!(store
            .apply_mapping_configuration_import(&first.token)
            .is_err());
        let current = store
            .save_mapping_configuration(MappingConfiguration::default())
            .unwrap();
        assert!(store
            .apply_mapping_configuration_import(&second.token)
            .unwrap_err()
            .contains("过期"));
        assert_eq!(store.load_mapping_configuration().unwrap(), current);
        assert!(store
            .apply_mapping_configuration_import(&second.token)
            .is_err());
    }

    #[cfg(windows)]
    #[test]
    fn template_atomic_replace_failure_keeps_file_runtime_and_revision() {
        use std::os::windows::fs::OpenOptionsExt;
        let store = template_test_store();
        let original = source_configuration();
        store.save_mapping_configuration(original.clone()).unwrap();
        let before = fs::read(store.button_mappings_path()).unwrap();
        let revision = store.revision.load(Ordering::Acquire);
        let imported_path = source_file(&store, MappingConfiguration::default());
        let preview = store
            .preview_mapping_configuration_import(&imported_path)
            .unwrap();
        let guard = fs::OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(store.button_mappings_path())
            .unwrap();
        let mut runtime = original.clone();
        let mut candidate = original.clone();
        candidate.button_mapping_follow_enabled = true;
        assert!(store
            .save_mapping_configuration_with(candidate, |saved| runtime = saved.clone())
            .is_err());
        assert_eq!(runtime, original);
        assert_eq!(fs::read(store.button_mappings_path()).unwrap(), before);
        assert_eq!(store.revision.load(Ordering::Acquire), revision);
        assert!(store
            .apply_mapping_configuration_import_with(&preview.token, |saved| runtime =
                saved.clone())
            .is_err());
        assert_eq!(runtime, original);
        assert!(store
            .apply_mapping_configuration_import(&preview.token)
            .is_err());
        drop(guard);
        let mut invalid = original.clone();
        invalid.templates[1].name = invalid.templates[0].name.clone();
        assert!(store
            .save_mapping_configuration_with(invalid, |saved| runtime = saved.clone())
            .is_err());
        assert_eq!(runtime, original);
        assert_eq!(fs::read(store.button_mappings_path()).unwrap(), before);
    }

    #[test]
    fn template_v3_exports_logical_targets_and_rejects_incoming_paths_and_schemes() {
        use sayall_windows::raw_input::RemoteButton;
        use sayall_windows::send_input::ButtonActions;
        let store = template_test_store();
        for target in [
            r"C:\private\reader.exe",
            r"\\host\share\reader.exe",
            "https://host/reader.exe",
            "shell:AppsFolder",
        ] {
            let mut configuration = MappingConfiguration::default();
            configuration.common_mappings.actions.insert(
                RemoteButton::Home,
                ButtonActions {
                    single: ButtonAction::OpenApp {
                        target: target.to_owned(),
                    },
                    ..Default::default()
                },
            );
            let path = source_file(&store, configuration);
            let text = fs::read_to_string(&path).unwrap();
            assert!(!text.contains(target));
            assert!(text.contains("custom-app-1"));
            let preview = store.preview_mapping_configuration_import(&path).unwrap();
            assert_eq!(preview.unresolved_application_ids, vec!["custom-app-1"]);
            let mut value: serde_json::Value = serde_json::from_str(&text).unwrap();
            value["commonMappings"]["actions"]["home"]["single"]["target"] = target.into();
            fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
            assert!(store.preview_mapping_configuration_import(&path).is_err());
        }
    }

    #[test]
    fn template_selected_import_validates_selection_and_new_binding_preview() {
        let store = template_test_store();
        let source = source_configuration();
        let path = source_file(&store, source.clone());
        let initial = store.preview_mapping_configuration_import(&path).unwrap();
        let token = initial.source_token.unwrap();
        for ids in [
            vec![],
            vec!["missing".to_owned()],
            vec![source.templates[0].id.clone(); 2],
        ] {
            assert!(store
                .preview_template_import(&token, selected_request(ids))
                .is_err());
        }
        let preview = store
            .preview_template_import(
                &token,
                selected_request(vec![source.templates[1].id.clone()]),
            )
            .unwrap();
        assert_eq!(preview.added_application_bindings.len(), 1);
        assert_eq!(preview.unresolved_application_ids, vec!["custom-reader"]);
        assert!(preview.skipped_application_ids.is_empty());
        let saved = store
            .apply_mapping_configuration_import(&preview.token)
            .unwrap();
        assert_eq!(saved.templates.len(), 1);
        assert_eq!(
            saved.application_bindings[0].template_id,
            saved.templates[0].id
        );
    }

    #[test]
    fn template_transactions_serialize_crud_and_runtime_application() {
        let store = template_test_store();
        let observed = Arc::new(Mutex::new(Vec::new()));
        let workers: Vec<_> = ["A", "B"]
            .into_iter()
            .map(|name| {
                let store = store.clone();
                let observed = Arc::clone(&observed);
                std::thread::spawn(move || {
                    store
                        .update_mapping_configuration(
                            |configuration| configuration.create_template(name.to_owned()),
                            |configuration| lock(&observed).push(configuration.clone()),
                        )
                        .unwrap()
                })
            })
            .collect();
        for worker in workers {
            worker.join().unwrap();
        }
        let observed = lock(&observed);
        assert_eq!(
            observed
                .iter()
                .map(|configuration| configuration.templates.len())
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
        assert_eq!(store.load_mapping_configuration().unwrap(), observed[1]);
    }

    #[test]
    fn other_voice_hotkey_memory_distinguishes_never_chosen_from_no_key() {
        // 「其他工具」的记忆必须能区分「从未选过」（None）与「明确选了不按键」（Some(vec![])）。
        let store = SettingsStore::new(std::env::temp_dir().join("sayall-other-hotkey-test.json"));
        assert_eq!(store.load_other_voice_hotkey().unwrap(), None);
        assert_eq!(
            store.save_other_voice_hotkey(Some(Vec::new())).unwrap(),
            Some(Vec::new())
        );
        assert_eq!(store.load_other_voice_hotkey().unwrap(), Some(Vec::new()));
        let keys = vec![KeyCode::LeftAlt];
        store.save_other_voice_hotkey(Some(keys.clone())).unwrap();
        assert_eq!(store.load_other_voice_hotkey().unwrap(), Some(keys));
        let _ = std::fs::remove_file(store.path.with_file_name("other-voice-hotkey.json"));
    }

    #[test]
    fn settings_round_trip_preserves_stable_endpoint_identity() {
        let mut usage_statistics = UsageStatistics::default();
        usage_statistics.record_button_presses("2026-09-01", 3);
        usage_statistics.record_voice_sessions("2026-09-01", 2, 4.5);
        let settings = AppSettings {
            selected_remote_id: Some("test-remote-id".to_owned()),
            audio_endpoint_id: Some("test-endpoint-id".to_owned()),
            audio_endpoint_name: Some("CABLE Input (Test)".to_owned()),
            gain_db: 6.0,
            usage_statistics,
            ..AppSettings::default()
        };

        let encoded = serialize_settings(&settings).unwrap();
        let decoded = parse_settings(std::str::from_utf8(&encoded).unwrap()).unwrap();

        assert_eq!(decoded, settings);
    }

    #[test]
    fn settings_load_preserves_non_audio_preferences() {
        let decoded = parse_settings(
            r#"{"schema_version":1,"audio_endpoint_id":"endpoint","audio_endpoint_name":"Endpoint Name","gain_db":12.0,"voice_trigger_mode":"hold","launch_at_login":true,"open_window_at_launch":false}"#,
        )
        .unwrap();

        assert_eq!(decoded.audio_endpoint_id.as_deref(), Some("endpoint"));
        assert_eq!(
            decoded.audio_endpoint_name.as_deref(),
            Some("Endpoint Name")
        );
        assert_eq!(decoded.gain_db, 12.0);
        assert!(decoded.launch_at_login);
        assert!(!decoded.open_window_at_launch);
        assert!(!decoded.check_prerelease_updates);
        assert_eq!(decoded.theme_preference, ThemePreference::System);
    }

    #[test]
    fn retired_icon_choice_does_not_reset_saved_settings() {
        let path = std::env::temp_dir().join(format!(
            "sayall-test-retired-icon-{}.json",
            std::process::id()
        ));
        std::fs::write(&path,
            r#"{"schema_version":3,"gain_db":12.0,"voice_trigger_mode":"hold","app_icon":"standard","theme_preference":"dark"}"#,
        ).unwrap();
        let store = SettingsStore::new(path.clone());
        let settings = store.load().unwrap();
        assert_eq!(settings.gain_db, 12.0);
        assert_eq!(settings.theme_preference, ThemePreference::Dark);
        store.save_gain_db(settings.gain_db).unwrap();
        let persisted: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(persisted.get("app_icon").is_none());
        assert_eq!(store.load().unwrap().gain_db, 12.0);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn theme_preference_defaults_to_system_and_persists() {
        let path = std::env::temp_dir().join(format!(
            "sayall-test-theme-preference-{}.json",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let store = SettingsStore::new(path.clone());

        assert_eq!(
            store.load().unwrap().theme_preference,
            ThemePreference::System
        );
        store.save_theme_preference(ThemePreference::Dark).unwrap();
        assert_eq!(
            store.load().unwrap().theme_preference,
            ThemePreference::Dark
        );

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn prerelease_update_preference_defaults_off_and_persists() {
        let path = std::env::temp_dir().join(format!(
            "sayall-test-update-preference-{}.json",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let store = SettingsStore::new(path.clone());

        assert!(!store.load().unwrap().check_prerelease_updates);
        store.save_check_prerelease_updates(true).unwrap();
        assert!(store.load().unwrap().check_prerelease_updates);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn voice_input_tool_defaults_to_unset_and_persists() {
        let path = std::env::temp_dir().join(format!(
            "sayall-test-voice-input-tool-{}.json",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let store = SettingsStore::new(path.clone());

        // 老配置 / 新装：未选择过 → None（界面据此推断一次，不替用户猜）。
        assert_eq!(store.load().unwrap().voice_input_tool, None);
        store
            .save_voice_input_tool(Some(VoiceInputTool::Doubao))
            .unwrap();
        assert_eq!(
            store.load().unwrap().voice_input_tool,
            Some(VoiceInputTool::Doubao)
        );
        store
            .save_voice_input_tool(Some(VoiceInputTool::Other))
            .unwrap();
        assert_eq!(
            store.load().unwrap().voice_input_tool,
            Some(VoiceInputTool::Other)
        );

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn audio_gain_defaults_to_zero_and_persists_clamped() {
        let path = std::env::temp_dir().join(format!(
            "sayall-test-audio-gain-{}.json",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let store = SettingsStore::new(path.clone());

        // 新装 / 老配置：默认 0 dB（原始音量）。
        assert_eq!(store.load().unwrap().gain_db, 0.0);

        assert_eq!(store.save_gain_db(12.0).unwrap(), 12.0);
        assert_eq!(store.load().unwrap().gain_db, 12.0);

        // 越界与非法值：按 0–24 dB 钳制后落盘，并返回实际保存值（界面显示同源）。
        assert_eq!(store.save_gain_db(30.0).unwrap(), 24.0);
        assert_eq!(store.load().unwrap().gain_db, 24.0);
        assert_eq!(store.save_gain_db(f32::NAN).unwrap(), 0.0);
        assert_eq!(store.load().unwrap().gain_db, 0.0);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn launch_at_login_defaults_off_and_persists() {
        let path = std::env::temp_dir().join(format!(
            "sayall-test-launch-at-login-{}.json",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let store = SettingsStore::new(path.clone());

        assert!(!store.load().unwrap().launch_at_login);
        store.save_launch_at_login(true).unwrap();
        assert!(store.load().unwrap().launch_at_login);
        store.save_launch_at_login(false).unwrap();
        assert!(!store.load().unwrap().launch_at_login);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn button_mapping_json_round_trip_preserves_typed_shortcut() {
        use sayall_windows::raw_input::RemoteButton;
        use sayall_windows::send_input::{
            ButtonAction, ButtonActions, ButtonTrigger, KeyChord, KeyCode,
        };

        let mut mappings = ButtonMappings::default();
        mappings.actions.insert(
            RemoteButton::Ok,
            ButtonActions {
                single: ButtonAction::Shortcut {
                    chord: KeyChord {
                        keys: vec![KeyCode::Control, KeyCode::Enter],
                    },
                },
                double: ButtonAction::Disabled,
                long: ButtonAction::Shortcut {
                    chord: KeyChord {
                        keys: vec![KeyCode::Escape],
                    },
                },
                hold_repeat: None,
                ok_context_click: false,
            },
        );
        let encoded = serde_json::to_string(&mappings).unwrap();
        let decoded: ButtonMappings = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, mappings);
        assert_eq!(
            decoded.action_for(RemoteButton::Ok, ButtonTrigger::Single),
            ButtonAction::Shortcut {
                chord: KeyChord {
                    keys: vec![KeyCode::Control, KeyCode::Enter],
                }
            }
        );
    }

    #[test]
    fn exported_button_mapping_configuration_is_stable_versioned_and_rejects_before_mutation() {
        use sayall_windows::raw_input::RemoteButton;
        use sayall_windows::send_input::{ButtonAction, ButtonActions, KeyChord, KeyCode};

        let base = std::env::temp_dir().join(format!(
            "sayall-test-button-mapping-config-{}",
            std::process::id()
        ));
        let settings_path = base.join("settings.json");
        let export_path = base.join("mapping.json");
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        let store = SettingsStore::new(settings_path);
        let mut mappings = ButtonMappings::default();
        mappings
            .applications
            .push(sayall_windows::registered_apps::AppLibraryEntry {
                name: "Example".into(),
                path: "shell:AppsFolder\\Example!App".into(),
            });
        mappings.actions.insert(
            RemoteButton::Power,
            ButtonActions {
                single: ButtonAction::Shortcut {
                    chord: KeyChord {
                        keys: vec![KeyCode::Escape],
                    },
                },
                double: ButtonAction::Disabled,
                long: ButtonAction::Disabled,
                hold_repeat: None,
                ok_context_click: false,
            },
        );

        store.save_button_mappings(mappings.clone()).unwrap();
        assert_eq!(store.load_button_mappings().unwrap(), mappings);
        let configuration = store.load_mapping_configuration().unwrap();
        store
            .export_mapping_configuration(&export_path, configuration.clone(), None)
            .unwrap();
        let exported: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&export_path).unwrap()).unwrap();
        assert_eq!(exported["formatVersion"], 3);
        assert!(exported.get("commonMappings").is_some());
        assert!(!std::fs::read_to_string(&export_path)
            .unwrap()
            .contains("AppsFolder"));
        let first_export = std::fs::read(&export_path).unwrap();
        store
            .export_mapping_configuration(&export_path, configuration, None)
            .unwrap();
        assert_eq!(std::fs::read(&export_path).unwrap(), first_export);
        let preview = store
            .preview_mapping_configuration_import(&export_path)
            .unwrap();
        assert_eq!(
            preview.configuration.common_mappings.actions,
            mappings.actions
        );
        assert!(preview
            .configuration
            .common_mappings
            .applications
            .is_empty());
        std::fs::write(
            &export_path,
            br#"{"formatVersion":99,"buttonMappings":{"enabled":false,"actions":{}}}"#,
        )
        .unwrap();
        assert!(store
            .preview_mapping_configuration_import(&export_path)
            .is_err());
        assert_eq!(store.load_button_mappings().unwrap(), mappings);
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn voice_hold_hotkey_round_trips_and_validates_chord() {
        let store = SettingsStore::new(std::env::temp_dir().join(format!(
            "sayall-test-voice-hold-{}.json",
            std::process::id()
        )));
        let _ = std::fs::remove_file(store.voice_hold_hotkey_path());

        // 缺省文件 = v1 默认（左 Ctrl + 左 Win）
        let default = store.load_voice_hold_hotkey().unwrap();
        assert_eq!(default, SettingsStore::default_voice_hold_hotkey());
        assert_eq!(
            default.unwrap().keys,
            vec![
                sayall_windows::send_input::KeyCode::LeftControl,
                sayall_windows::send_input::KeyCode::LeftWindows,
            ]
        );

        let right_alt = KeyChord {
            keys: vec![sayall_windows::send_input::KeyCode::RightAlt],
        };
        let saved = store
            .save_voice_hold_hotkey(Some(right_alt.clone()))
            .unwrap();
        assert_eq!(saved, Some(right_alt.clone()));
        assert_eq!(store.load_voice_hold_hotkey().unwrap(), Some(right_alt));

        let disabled = store.save_voice_hold_hotkey(None).unwrap();
        assert_eq!(disabled, None);
        assert_eq!(store.load_voice_hold_hotkey().unwrap(), None);

        let invalid = KeyChord { keys: vec![] };
        assert!(store.save_voice_hold_hotkey(Some(invalid)).is_err());

        let _ = std::fs::remove_file(store.voice_hold_hotkey_path());
    }

    #[test]
    fn selected_template_export_excludes_unselected_templates_and_launch_targets() {
        let base =
            std::env::temp_dir().join(format!("sayall-template-export-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        let store = SettingsStore::new(base.join("settings.json"));
        let mut configuration = MappingConfiguration::default();
        let first = configuration.create_template("阅读".to_owned()).unwrap();
        let second = configuration.create_template("演示".to_owned()).unwrap();
        configuration.application_bindings.push(ApplicationBinding {
            application_id: "edge".to_owned(),
            template_id: first.id.clone(),
            menu_order: 0,
            launch_target: Some("C:\\private\\edge.exe".to_owned()),
        });
        let output = base.join("selected.json");
        store
            .export_mapping_configuration(&output, configuration, Some(&[first.id.clone()]))
            .unwrap();
        let text = std::fs::read_to_string(output).unwrap();
        assert!(text.contains(&first.id));
        assert!(!text.contains(&second.id));
        assert!(!text.contains("private"));
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn builtin_reference_round_trips_without_exporting_or_overwriting_its_body() {
        let base = std::env::temp_dir().join(format!(
            "sayall-builtin-template-export-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        let store = SettingsStore::new(base.join("settings.json"));
        let mut configuration = MappingConfiguration::default();
        configuration.application_bindings.push(ApplicationBinding {
            application_id: "edge".to_owned(),
            template_id: sayall_windows::templates::BUILTIN_BROWSER_TEMPLATE_ID.to_owned(),
            menu_order: 0,
            launch_target: None,
        });
        let output = base.join("builtin.json");
        store
            .export_mapping_configuration(
                &output,
                configuration,
                Some(&[sayall_windows::templates::BUILTIN_BROWSER_TEMPLATE_ID.to_owned()]),
            )
            .unwrap();
        let exported: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&output).unwrap()).unwrap();
        assert_eq!(exported["templates"].as_array().unwrap().len(), 0);
        assert_eq!(
            exported["builtinTemplateIds"][0],
            sayall_windows::templates::BUILTIN_BROWSER_TEMPLATE_ID
        );

        let preview = store.preview_mapping_configuration_import(&output).unwrap();
        assert_eq!(
            preview.builtin_template_ids,
            vec![sayall_windows::templates::BUILTIN_BROWSER_TEMPLATE_ID.to_owned()]
        );
        let selected = store
            .preview_template_import(
                preview.source_token.as_deref().unwrap(),
                TemplateImportRequest {
                    template_ids: preview.builtin_template_ids,
                    resolved_names: BTreeMap::new(),
                    replace_application_bindings: false,
                },
            )
            .unwrap();
        assert!(selected.templates.is_empty());
        assert_eq!(selected.added_application_bindings.len(), 1);
        let saved = store
            .apply_mapping_configuration_import(&selected.token)
            .unwrap();
        assert_eq!(
            saved.application_bindings[0].template_id,
            sayall_windows::templates::BUILTIN_BROWSER_TEMPLATE_ID
        );
        assert!(saved.templates.is_empty());
        let _ = std::fs::remove_dir_all(base);
    }

    fn onboarding_test_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "sayall-test-onboarding-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn onboarding_fresh_install_starts_at_welcome_and_is_idempotent() {
        let base = onboarding_test_dir("fresh");
        let store = SettingsStore::new(base.join("settings.json"));

        let state = store.ensure_onboarding_state().unwrap();
        assert!(state.is_active());
        assert_eq!(state.step, OnboardingStep::Welcome);
        assert_eq!(state.flow_version, onboarding::CURRENT_FLOW_VERSION);
        assert_eq!(
            state.migration_version,
            onboarding::CURRENT_MIGRATION_VERSION
        );
        assert!(onboarding::state_path(&store.path).exists());

        // 二次调用幂等：不重写、不重置进度。
        let again = store.ensure_onboarding_state().unwrap();
        assert_eq!(state, again);
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn imported_builtin_edit_becomes_a_copy_without_overwriting_local_default() {
        let base =
            std::env::temp_dir().join(format!("sayall-builtin-import-{}", std::process::id()));
        std::fs::create_dir_all(&base).unwrap();
        let store = SettingsStore::new(base.join("settings.json"));
        let path = base.join("edited-default.json");
        let mut builtin = MappingConfiguration::recommended_templates().remove(0);
        builtin.mappings.actions.clear();
        let document = serde_json::json!({
            "formatVersion": 3,
            "commonMappings": ButtonMappings::default(),
            "buttonMappingFollowEnabled": false,
            "templates": [builtin],
            "builtinTemplateIds": [sayall_windows::templates::BUILTIN_AGENT_TEMPLATE_ID],
            "applicationBindings": []
        });
        std::fs::write(&path, serde_json::to_vec_pretty(&document).unwrap()).unwrap();
        let preview = store.preview_mapping_configuration_import(&path).unwrap();
        assert!(
            preview.builtin_template_ids.is_empty(),
            "a customized body is imported as a copy, not a canonical reference"
        );
        assert_eq!(preview.configuration.templates.len(), 1);
        assert_eq!(
            store.load_mapping_configuration().unwrap().templates.len(),
            0
        );
        let selected = store
            .preview_template_import(
                preview.source_token.as_deref().unwrap(),
                TemplateImportRequest {
                    template_ids: vec![
                        sayall_windows::templates::BUILTIN_AGENT_TEMPLATE_ID.to_owned()
                    ],
                    resolved_names: BTreeMap::new(),
                    replace_application_bindings: false,
                },
            )
            .unwrap();
        let saved = store
            .apply_mapping_configuration_import(&selected.token)
            .unwrap();
        assert_eq!(saved.templates.len(), 1);
        assert_ne!(
            saved.templates[0].id,
            sayall_windows::templates::BUILTIN_AGENT_TEMPLATE_ID
        );
        assert!(saved.templates[0].mappings.actions.is_empty());
        assert!(!saved
            .template_mappings(sayall_windows::templates::BUILTIN_AGENT_TEMPLATE_ID)
            .unwrap()
            .actions
            .is_empty());
    }

    #[test]
    fn onboarding_existing_install_is_migrated_to_completed() {
        let base = onboarding_test_dir("existing");
        let store = SettingsStore::new(base.join("settings.json"));
        std::fs::write(
            base.join("settings.json"),
            r#"{"schema_version":3,"gain_db":0.0,"voice_trigger_mode":"hold"}"#,
        )
        .unwrap();

        let state = store.ensure_onboarding_state().unwrap();
        assert!(!state.is_active());
        assert_eq!(state.completed_version, onboarding::CURRENT_FLOW_VERSION);

        // 第二次启动仍是完成态（一次性迁移位防止误判）。
        let again = store.ensure_onboarding_state().unwrap();
        assert!(!again.is_active());
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn onboarding_corrupt_state_is_repaired_conservatively() {
        // 有旧安装证据：按老用户修复，重写为有效文件。
        let base = onboarding_test_dir("repair-existing");
        let store = SettingsStore::new(base.join("settings.json"));
        std::fs::write(base.join("settings.json"), "{}").unwrap();
        std::fs::write(onboarding::state_path(&store.path), "not json").unwrap();

        let state = store.ensure_onboarding_state().unwrap();
        assert!(!state.is_active());
        let repaired: OnboardingState = serde_json::from_str(
            &std::fs::read_to_string(onboarding::state_path(&store.path)).unwrap(),
        )
        .unwrap();
        assert!(!repaired.is_active());

        // 无任何证据：按全新修复，从欢迎开始。
        let base2 = onboarding_test_dir("repair-fresh");
        let store2 = SettingsStore::new(base2.join("settings.json"));
        std::fs::write(onboarding::state_path(&store2.path), "not json").unwrap();
        let state2 = store2.ensure_onboarding_state().unwrap();
        assert!(state2.is_active());
        assert_eq!(state2.step, OnboardingStep::Welcome);

        let _ = std::fs::remove_dir_all(base);
        let _ = std::fs::remove_dir_all(base2);
    }

    #[test]
    fn onboarding_unknown_step_in_file_normalizes_to_welcome_without_losing_completion() {
        let base = onboarding_test_dir("unknown-step");
        let store = SettingsStore::new(base.join("settings.json"));
        std::fs::write(
            onboarding::state_path(&store.path),
            r#"{"flow_version":1,"completed_version":1,"step":"legacy_step","migration_version":1}"#,
        )
        .unwrap();

        let state = store.ensure_onboarding_state().unwrap();
        assert_eq!(state.step, OnboardingStep::Welcome);
        assert!(!state.is_active());
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn onboarding_step_save_restart_and_complete_lifecycle() {
        let base = onboarding_test_dir("lifecycle");
        let store = SettingsStore::new(base.join("settings.json"));

        store
            .save_onboarding_step(OnboardingStep::VoiceTest)
            .unwrap();
        assert_eq!(
            store.ensure_onboarding_state().unwrap().step,
            OnboardingStep::VoiceTest
        );

        let restarted = store.restart_onboarding().unwrap();
        assert!(restarted.is_active());
        assert_eq!(restarted.step, OnboardingStep::Welcome);

        let completed = store.complete_onboarding().unwrap();
        assert!(!completed.is_active());
        assert_eq!(
            completed.completed_version,
            onboarding::CURRENT_FLOW_VERSION
        );
        // 完成后再次读取不被重置，重启也不会重新激活。
        assert!(!store.ensure_onboarding_state().unwrap().is_active());

        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn onboarding_staged_binding_snapshots_once_applies_formal_and_restores() {
        let base = onboarding_test_dir("staged-binding");
        let store = SettingsStore::new(base.join("settings.json"));
        let right_alt = KeyChord {
            keys: vec![KeyCode::RightAlt],
        };

        // 迁移判定先行（全新 → 进行中）；再模拟"老用户完成过、又主动重跑向导"：
        // 正式配置是他之前的选择，重跑后进入 active，staged 应以此为回滚基线。
        assert!(store.ensure_onboarding_state().unwrap().is_active());
        store.complete_onboarding().unwrap();
        store
            .save_voice_input_tool(Some(VoiceInputTool::Wechat))
            .unwrap();
        store
            .save_voice_hold_hotkey(SettingsStore::default_voice_hold_hotkey())
            .unwrap();
        assert!(store.restart_onboarding().unwrap().is_active());

        // 第一次暂存：应用正式值，并记录回滚基线。
        let state = store
            .stage_onboarding_voice_binding(VoiceInputTool::Doubao, Some(right_alt.clone()))
            .unwrap();
        assert_eq!(
            store.load().unwrap().voice_input_tool,
            Some(VoiceInputTool::Doubao)
        );
        assert_eq!(
            store.load_voice_hold_hotkey().unwrap(),
            Some(right_alt.clone())
        );
        let staged = state.staged.clone().unwrap();
        assert_eq!(staged.revert_tool, Some(VoiceInputTool::Wechat));
        assert_eq!(
            staged.revert_voice_hotkey,
            SettingsStore::default_voice_hold_hotkey()
        );

        // 二次暂存（换工具）：回滚基线保持第一次的值，不被覆盖。
        store
            .stage_onboarding_voice_binding(VoiceInputTool::Vokie, None)
            .unwrap();
        let staged = store.ensure_onboarding_state().unwrap().staged.unwrap();
        assert_eq!(staged.revert_tool, Some(VoiceInputTool::Wechat));
        assert_eq!(
            staged.revert_voice_hotkey,
            SettingsStore::default_voice_hold_hotkey()
        );
        assert_eq!(store.load_voice_hold_hotkey().unwrap(), None);

        // 回滚：正式配置恢复、快照清除；再次回滚是幂等空操作。
        assert!(store
            .restore_onboarding_staged_binding("test_restore")
            .unwrap());
        assert_eq!(
            store.load().unwrap().voice_input_tool,
            Some(VoiceInputTool::Wechat)
        );
        assert_eq!(
            store.load_voice_hold_hotkey().unwrap(),
            SettingsStore::default_voice_hold_hotkey()
        );
        assert!(store.ensure_onboarding_state().unwrap().staged.is_none());
        assert!(!store
            .restore_onboarding_staged_binding("test_restore_again")
            .unwrap());

        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn onboarding_dismiss_rolls_back_binding_without_claiming_verification() {
        let base = onboarding_test_dir("dismiss-rollback");
        let store = SettingsStore::new(base.join("settings.json"));
        store.ensure_onboarding_state().unwrap();
        store
            .save_voice_input_tool(Some(VoiceInputTool::Wechat))
            .unwrap();
        let original = store.load_voice_hold_hotkey().unwrap();
        store
            .stage_onboarding_voice_binding(VoiceInputTool::Doubao, None)
            .unwrap();
        store
            .save_onboarding_step(OnboardingStep::VoiceTest)
            .unwrap();
        let state = store.dismiss_onboarding().unwrap();
        assert!(!state.is_active());
        assert_eq!(state.step, OnboardingStep::VoiceTest);
        assert!(state.staged.is_none());
        assert_eq!(
            store.load().unwrap().voice_input_tool,
            Some(VoiceInputTool::Wechat)
        );
        assert_eq!(store.load_voice_hold_hotkey().unwrap(), original);
        assert!(!store.dismiss_onboarding().unwrap().is_active());
        assert!(store
            .stage_onboarding_voice_binding(VoiceInputTool::Doubao, None)
            .is_err());
        assert!(store.restart_onboarding().unwrap().is_active());
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn onboarding_stage_requires_active_flow() {
        let base = onboarding_test_dir("stage-inactive");
        let store = SettingsStore::new(base.join("settings.json"));
        store.complete_onboarding().unwrap();

        assert!(store
            .stage_onboarding_voice_binding(VoiceInputTool::Doubao, None)
            .is_err());
        assert!(!store
            .restore_onboarding_staged_binding("test_inactive")
            .unwrap());

        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn onboarding_restart_rolls_back_staged_binding_before_reset() {
        let base = onboarding_test_dir("restart-rollback");
        let store = SettingsStore::new(base.join("settings.json"));
        assert!(store.ensure_onboarding_state().unwrap().is_active());
        store
            .save_voice_input_tool(Some(VoiceInputTool::Wechat))
            .unwrap();
        store
            .stage_onboarding_voice_binding(
                VoiceInputTool::Doubao,
                Some(KeyChord {
                    keys: vec![KeyCode::RightAlt],
                }),
            )
            .unwrap();

        let state = store.restart_onboarding().unwrap();
        assert!(state.is_active());
        assert_eq!(state.step, OnboardingStep::Welcome);
        assert!(state.staged.is_none());
        assert_eq!(
            store.load().unwrap().voice_input_tool,
            Some(VoiceInputTool::Wechat)
        );

        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn onboarding_complete_commits_staged_binding_values() {
        let base = onboarding_test_dir("complete-commit");
        let store = SettingsStore::new(base.join("settings.json"));
        let right_alt = KeyChord {
            keys: vec![KeyCode::RightAlt],
        };
        assert!(store.ensure_onboarding_state().unwrap().is_active());
        store
            .save_voice_input_tool(Some(VoiceInputTool::Wechat))
            .unwrap();
        store
            .stage_onboarding_voice_binding(VoiceInputTool::Doubao, Some(right_alt.clone()))
            .unwrap();

        let state = store.complete_onboarding().unwrap();
        assert!(!state.is_active());
        assert!(state.staged.is_none());
        // 提交：保留暂存时应用的正式值（验证通过后的最终状态）。
        assert_eq!(
            store.load().unwrap().voice_input_tool,
            Some(VoiceInputTool::Doubao)
        );
        assert_eq!(
            store.load_voice_hold_hotkey().unwrap(),
            Some(right_alt.clone())
        );

        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn existing_mapping_load_save_and_export_preserve_automatic_repeat_configuration() {
        use sayall_windows::raw_input::RemoteButton;
        let store = template_test_store();
        let path = store.button_mappings_path();
        let legacy = r#"{"enabled":true,"actions":{"back":{"single":{"type":"shortcut","chord":{"keys":["backspace"]}},"double":{"type":"disabled"},"long":{"type":"disabled"}}}}"#;
        std::fs::write(&path, legacy).unwrap();
        let loaded = store.load_button_mappings().unwrap();
        assert_eq!(loaded.schema_version, 2);
        assert_eq!(loaded.actions(RemoteButton::Back).hold_repeat, None);
        assert_eq!(
            loaded
                .actions(RemoteButton::Back)
                .effective_hold_repeat(RemoteButton::Back),
            Some(sayall_windows::send_input::ButtonTrigger::Single)
        );
        store.save_button_mappings(loaded).unwrap();
        let configuration = store.load_mapping_configuration().unwrap();
        assert_eq!(
            configuration
                .common_mappings
                .actions(RemoteButton::Back)
                .hold_repeat,
            None
        );
        let export = path.with_file_name("export.json");
        store
            .export_mapping_configuration(&export, configuration, None)
            .unwrap();
        let value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(export).unwrap()).unwrap();
        assert!(value["commonMappings"]["actions"]["back"]["holdRepeat"].is_null());
    }

    #[test]
    fn newer_mapping_schema_is_rejected_without_overwriting_original() {
        let store = template_test_store();
        let path = store.button_mappings_path();
        let newer = r#"{"schemaVersion":9,"enabled":true,"actions":{}}"#;
        std::fs::write(&path, newer).unwrap();
        assert!(store.load_button_mappings().is_err());
        assert!(store
            .save_button_mappings(ButtonMappings::default())
            .is_err());
        assert_eq!(std::fs::read_to_string(path).unwrap(), newer);
    }

    #[test]
    fn hold_repeat_off_and_auto_round_trip_common_and_custom_template() {
        use sayall_windows::raw_input::RemoteButton;
        use sayall_windows::send_input::{ButtonActions, HoldRepeat, KeyCode};
        let store = template_test_store();
        let mut configuration = MappingConfiguration::default();
        for (button, key, hold_repeat) in [
            (
                RemoteButton::Back,
                KeyCode::Backspace,
                Some(HoldRepeat::Off),
            ),
            (RemoteButton::Right, KeyCode::Right, None),
        ] {
            configuration.common_mappings.actions.insert(
                button,
                ButtonActions {
                    single: ButtonAction::Shortcut {
                        chord: KeyChord { keys: vec![key] },
                    },
                    hold_repeat,
                    ..Default::default()
                },
            );
        }
        let custom = configuration.create_template("自动与关闭".into()).unwrap();
        let mut custom_mappings = configuration.common_mappings.clone();
        custom_mappings
            .actions
            .get_mut(&RemoteButton::Back)
            .unwrap()
            .hold_repeat = None;
        custom_mappings
            .actions
            .get_mut(&RemoteButton::Right)
            .unwrap()
            .hold_repeat = Some(HoldRepeat::Off);
        configuration
            .templates
            .iter_mut()
            .find(|template| template.id == custom.id)
            .unwrap()
            .mappings = custom_mappings.clone();
        let saved = store.save_mapping_configuration(configuration).unwrap();
        assert_eq!(store.load_mapping_configuration().unwrap(), saved);
        let exported_path = store.path.with_file_name("repeat-round-trip.json");
        store
            .export_mapping_configuration(&exported_path, saved.clone(), None)
            .unwrap();
        let exported: serde_json::Value =
            serde_json::from_slice(&fs::read(&exported_path).unwrap()).unwrap();
        assert_eq!(exported["formatVersion"], 3);
        assert_eq!(exported["commonMappings"]["schemaVersion"], 2);
        assert_eq!(exported["templates"][0]["mappings"]["schemaVersion"], 2);
        assert_eq!(
            exported["commonMappings"]["actions"]["back"]["holdRepeat"],
            "off"
        );
        assert!(exported["commonMappings"]["actions"]["right"]
            .get("holdRepeat")
            .is_none());

        let destination = template_test_store();
        let preview = destination
            .preview_mapping_configuration_import(&exported_path)
            .unwrap();
        assert_eq!(
            preview.configuration.common_mappings.actions,
            saved.common_mappings.actions
        );
        assert_eq!(preview.configuration.templates[0].mappings, custom_mappings);
        let imported = destination
            .apply_mapping_configuration_import(&preview.token)
            .unwrap();
        assert_eq!(
            imported.common_mappings.actions,
            saved.common_mappings.actions
        );
        assert_eq!(imported.templates[0].mappings, custom_mappings);
        assert_eq!(destination.load_mapping_configuration().unwrap(), imported);
    }

    #[test]
    fn nested_future_mapping_schema_rejects_save_export_and_import_without_writing() {
        let store = template_test_store();
        let mut configuration = MappingConfiguration::default();
        configuration.create_template("测试模板".into()).unwrap();
        let saved = store.save_mapping_configuration(configuration).unwrap();
        let original = fs::read(store.button_mappings_path()).unwrap();
        let export = store.path.with_file_name("future-export.json");
        store
            .export_mapping_configuration(&export, saved.clone(), None)
            .unwrap();
        let original_export = fs::read(&export).unwrap();
        let incoming = store.path.with_file_name("future-import.json");
        for common in [true, false] {
            let mut invalid = saved.clone();
            if common {
                invalid.common_mappings.schema_version = 3;
            } else {
                invalid.templates[0].mappings.schema_version = 3;
            }
            assert!(store.save_mapping_configuration(invalid.clone()).is_err());
            assert_eq!(fs::read(store.button_mappings_path()).unwrap(), original);
            assert!(store
                .export_mapping_configuration(&export, invalid, None)
                .is_err());
            assert_eq!(fs::read(&export).unwrap(), original_export);

            let mut document: serde_json::Value = serde_json::from_slice(&original_export).unwrap();
            if common {
                document["commonMappings"]["schemaVersion"] = 3.into();
            } else {
                document["templates"][0]["mappings"]["schemaVersion"] = 3.into();
            }
            fs::write(&incoming, serde_json::to_vec(&document).unwrap()).unwrap();
            assert!(store
                .preview_mapping_configuration_import(&incoming)
                .is_err());
            assert_eq!(fs::read(store.button_mappings_path()).unwrap(), original);
            assert_eq!(store.load_mapping_configuration().unwrap(), saved);
        }
    }
}
