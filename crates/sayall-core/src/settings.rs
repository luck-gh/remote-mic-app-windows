use crate::UsageStatistics;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum VoiceTriggerMode {
    #[default]
    Hold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThemePreference {
    #[default]
    System,
    Light,
    Dark,
}

/// A capture endpoint is distinct from the render endpoint receiving decoded PCM.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CaptureInputSettings {
    pub enabled: bool,
    pub endpoint_id: Option<String>,
    pub endpoint_name: Option<String>,
}

/// 用户在连接页选择的输入工具（决定"按住说话快捷键"的默认组合与引导步骤）。
///
/// `None` 表示用户尚未选择；页面可按当前快捷键显示初始选项，
/// 只有用户显式选择才持久化。既有默认快捷键的运行行为由平台层保持。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceInputTool {
    Wechat,
    Doubao,
    Vokie,
    Other,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub ui_preferences: UiPreferences,
    pub schema_version: u32,
    pub selected_remote_id: Option<String>,
    pub audio_endpoint_id: Option<String>,
    pub audio_endpoint_name: Option<String>,
    pub capture_input: CaptureInputSettings,
    pub gain_db: f32,
    pub voice_trigger_mode: VoiceTriggerMode,
    /// 连接页选择的输入工具（微信输入法 / 豆包输入法 / 其他工具）。
    pub voice_input_tool: Option<VoiceInputTool>,
    pub launch_at_login: bool,
    pub open_window_at_launch: bool,
    pub rc003_capture_enabled: bool,
    pub check_prerelease_updates: bool,
    pub theme_preference: ThemePreference,
    pub usage_statistics: UsageStatistics,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            ui_preferences: UiPreferences::default(),
            schema_version: 3,
            selected_remote_id: None,
            audio_endpoint_id: None,
            audio_endpoint_name: None,
            capture_input: CaptureInputSettings::default(),
            gain_db: 0.0,
            voice_trigger_mode: VoiceTriggerMode::Hold,
            voice_input_tool: None,
            launch_at_login: false,
            open_window_at_launch: true,
            rc003_capture_enabled: false,
            check_prerelease_updates: false,
            theme_preference: ThemePreference::System,
            usage_statistics: UsageStatistics::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct UiPreferences {
    pub lock_button_selection: bool,
    pub templates_expanded: bool,
    pub associations_expanded: bool,
}
impl Default for UiPreferences {
    fn default() -> Self {
        Self {
            lock_button_selection: true,
            templates_expanded: true,
            associations_expanded: true,
        }
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UiPreference {
    LockButtonSelection,
    TemplatesExpanded,
    AssociationsExpanded,
}
impl UiPreferences {
    pub fn set(&mut self, field: UiPreference, enabled: bool) {
        match field {
            UiPreference::LockButtonSelection => self.lock_button_selection = enabled,
            UiPreference::TemplatesExpanded => self.templates_expanded = enabled,
            UiPreference::AssociationsExpanded => self.associations_expanded = enabled,
        }
    }
}

impl AppSettings {
    pub fn normalized(mut self) -> Self {
        self.schema_version = Self::default().schema_version;
        self.gain_db = crate::normalize_gain_db(self.gain_db);
        self.usage_statistics = self.usage_statistics.normalized();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_gain_and_keeps_hold_as_only_voice_mode() {
        let settings = AppSettings {
            gain_db: 30.0,
            ..AppSettings::default()
        }
        .normalized();
        assert_eq!(settings.gain_db, 24.0);
        assert_eq!(settings.voice_trigger_mode, VoiceTriggerMode::Hold);
    }

    #[test]
    fn older_settings_without_endpoint_name_remain_compatible() {
        let settings: AppSettings = serde_json::from_str(
            r#"{"schema_version":1,"audio_endpoint_id":"endpoint-1","gain_db":0.0,"voice_trigger_mode":"hold","launch_at_login":false,"open_window_at_launch":true}"#,
        )
        .unwrap();
        let settings = settings.normalized();

        assert_eq!(settings.audio_endpoint_id.as_deref(), Some("endpoint-1"));
        assert_eq!(settings.audio_endpoint_name, None);
        assert_eq!(settings.schema_version, 3);
        assert!(!settings.check_prerelease_updates);
        assert_eq!(settings.theme_preference, ThemePreference::System);
        assert_eq!(settings.usage_statistics, UsageStatistics::default());
    }

    #[test]
    fn theme_preferences_round_trip() {
        for preference in [
            ThemePreference::System,
            ThemePreference::Light,
            ThemePreference::Dark,
        ] {
            let settings = AppSettings {
                theme_preference: preference,
                ..AppSettings::default()
            };
            let encoded = serde_json::to_string(&settings).unwrap();
            let decoded: AppSettings = serde_json::from_str(&encoded).unwrap();
            assert_eq!(decoded.normalized().theme_preference, preference);
        }
    }

    #[test]
    fn voice_input_tool_defaults_to_none_and_round_trips() {
        // 老配置没有这个字段：必须落成 None（由界面按当前快捷键推断一次），
        // 不能在 Rust 侧替用户猜成某个工具。
        let settings: AppSettings = serde_json::from_str(
            r#"{"schema_version":3,"gain_db":0.0,"voice_trigger_mode":"hold"}"#,
        )
        .unwrap();
        assert_eq!(settings.voice_input_tool, None);

        for tool in [
            VoiceInputTool::Wechat,
            VoiceInputTool::Doubao,
            VoiceInputTool::Vokie,
            VoiceInputTool::Other,
        ] {
            let settings = AppSettings {
                voice_input_tool: Some(tool),
                ..AppSettings::default()
            };
            let encoded = serde_json::to_string(&settings).unwrap();
            assert!(encoded.contains("\"voice_input_tool\""));
            let decoded: AppSettings = serde_json::from_str(&encoded).unwrap();
            assert_eq!(decoded.voice_input_tool, Some(tool));
        }
    }
}

#[cfg(test)]
mod enhancement_default_tests {
    use super::AppSettings;
    #[test]
    fn retired_app_icon_is_ignored_without_resetting_preferences() {
        for icon in [
            serde_json::json!("faceted-duck"),
            serde_json::json!("standard"),
            serde_json::json!({"unknown": true}),
        ] {
            let loaded: AppSettings = serde_json::from_value(serde_json::json!({
                "app_icon": icon,
                "gain_db": 12.0,
                "launch_at_login": true,
                "theme_preference": "dark",
                "capture_input": {"enabled": true, "endpointId": "capture-test"}
            }))
            .unwrap();
            let encoded = serde_json::to_value(loaded).unwrap();
            assert!(encoded.get("app_icon").is_none());
            assert_eq!(encoded["gain_db"], 12.0);
            assert_eq!(encoded["launch_at_login"], true);
            assert_eq!(encoded["theme_preference"], "dark");
            assert_eq!(encoded["capture_input"]["endpointId"], "capture-test");
        }
    }
    #[test]
    fn legacy_helper_opt_in_does_not_authorize_unified_capture() {
        let loaded: AppSettings = serde_json::from_str(
            r#"{"restore_hid_enhancement":true,"capture_input":{"enabled":true,"endpointId":"capture-test"},"ui_preferences":{"lockButtonSelection":false}}"#,
        ).unwrap();
        let encoded = serde_json::to_value(loaded).unwrap();
        assert_eq!(encoded["rc003_capture_enabled"], false);
        assert!(encoded.get("restore_hid_enhancement").is_none());
        assert_eq!(encoded["capture_input"]["endpointId"], "capture-test");
        assert_eq!(encoded["ui_preferences"]["lockButtonSelection"], false);
        assert!(encoded["ui_preferences"]
            .get("hidEnhancementExpanded")
            .is_none());
    }
    #[test]
    fn unified_capture_requires_explicit_opt_in_and_reopens_saved_intent() {
        assert!(!AppSettings::default().rc003_capture_enabled);
        assert!(
            !serde_json::from_str::<AppSettings>("{}")
                .unwrap()
                .rc003_capture_enabled
        );
        let saved: AppSettings = serde_json::from_str(r#"{"rc003_capture_enabled":true}"#).unwrap();
        assert!(saved.rc003_capture_enabled);
        let reopened: AppSettings =
            serde_json::from_str(&serde_json::to_string(&saved).unwrap()).unwrap();
        assert!(reopened.rc003_capture_enabled);
    }
}
