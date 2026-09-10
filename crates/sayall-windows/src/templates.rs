//! Structured mapping-template configuration.
//!
//! This module deliberately only describes persisted user intent.  Resolving a
//! foreground application or injecting an action stays in the platform runtime.

use crate::send_input::ButtonMappings;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TEMPLATE_ID_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdjustmentMode {
    #[default]
    Volume,
    Page,
    Zoom,
}

/// A focused area that gives the same physical key an explicit meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlRegion {
    ApplicationList,
    Content,
    Input,
}

/// Intent resolved by a public application adapter.  It is never a fallback
/// shortcut: an unavailable intent is reported as unavailable by that adapter.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SemanticAction {
    #[default]
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
    BrowserBack,
    PreviousTab,
    NextTab,
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
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticButtonActions {
    #[serde(default)]
    pub single: SemanticAction,
    #[serde(default)]
    pub double: SemanticAction,
    #[serde(default)]
    pub long: SemanticAction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MappingTemplate {
    pub id: String,
    pub name: String,
    /// Region-scoped semantic intents.  Unbound applications use
    /// `MappingConfiguration.common_mappings` instead.
    #[serde(default)]
    pub region_actions:
        BTreeMap<ControlRegion, BTreeMap<crate::raw_input::RemoteButton, SemanticButtonActions>>,
    #[serde(default)]
    pub adjustment_mode: AdjustmentMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationBinding {
    pub application_id: String,
    pub template_id: String,
    #[serde(default)]
    pub menu_order: u32,
    /// Local-only launch target.  It is intentionally omitted from exports.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub launch_target: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportApplicationBinding {
    pub application_id: String,
    pub template_id: String,
    pub menu_order: u32,
}

impl From<&ApplicationBinding> for ExportApplicationBinding {
    fn from(binding: &ApplicationBinding) -> Self {
        Self {
            application_id: binding.application_id.clone(),
            template_id: binding.template_id.clone(),
            menu_order: binding.menu_order,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MappingConfiguration {
    #[serde(default)]
    pub common_mappings: ButtonMappings,
    #[serde(default)]
    pub template_control_enabled: bool,
    #[serde(default)]
    pub templates: Vec<MappingTemplate>,
    #[serde(default)]
    pub application_bindings: Vec<ApplicationBinding>,
}

/// A share-safe import preview. The token retains local launch resolutions;
/// the displayed configuration contains logical references and no local paths.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MappingConfigurationImportPreview {
    pub token: String,
    /// Only this source token can be used to prepare selected-template imports.
    /// It is never an apply token.
    pub source_token: Option<String>,
    pub format_version: u32,
    pub configuration: MappingConfiguration,
    pub template_name_conflicts: Vec<String>,
    pub unresolved_application_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateImportRequest {
    #[serde(default)]
    pub template_ids: Vec<String>,
    #[serde(default)]
    pub resolved_names: BTreeMap<String, String>,
    #[serde(default)]
    pub replace_application_bindings: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedTemplatePreview {
    pub source_template_id: String,
    pub template: MappingTemplate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateImportPreview {
    pub token: String,
    pub templates: Vec<ImportedTemplatePreview>,
    pub added_application_bindings: Vec<ExportApplicationBinding>,
    pub replaced_application_ids: Vec<String>,
    pub skipped_application_ids: Vec<String>,
    pub unresolved_application_ids: Vec<String>,
}

impl Default for MappingConfiguration {
    fn default() -> Self {
        Self {
            common_mappings: ButtonMappings::default(),
            template_control_enabled: false,
            templates: Vec::new(),
            application_bindings: Vec::new(),
        }
    }
}

impl MappingConfiguration {
    pub fn normalized(mut self) -> Result<Self, String> {
        self.common_mappings = self
            .common_mappings
            .normalized()
            .map_err(|error| format!("通用映射无效：{error}"))?;
        let mut ids = HashSet::new();
        let mut names = HashSet::new();
        for template in &mut self.templates {
            template.id = normalized_required(&template.id, "模板 ID")?;
            template.name = normalized_required(&template.name, "模板名称")?;
            if !ids.insert(template.id.clone()) {
                return Err("模板 ID 重复".to_owned());
            }
            if !names.insert(template.name.clone()) {
                return Err("模板名称重复".to_owned());
            }
        }
        let mut applications = HashSet::new();
        for binding in &mut self.application_bindings {
            binding.application_id = normalized_required(&binding.application_id, "应用 ID")?;
            binding.template_id = normalized_required(&binding.template_id, "模板 ID")?;
            if !applications.insert(binding.application_id.clone()) {
                return Err("应用绑定重复".to_owned());
            }
            if !ids.contains(&binding.template_id) {
                return Err("应用绑定引用了不存在的模板".to_owned());
            }
            binding.launch_target = binding
                .launch_target
                .take()
                .map(|target| normalized_required(&target, "启动目标"))
                .transpose()?;
        }
        Ok(self)
    }

    pub fn create_template(&mut self, name: String) -> Result<MappingTemplate, String> {
        let name = normalized_required(&name, "模板名称")?;
        if self.templates.iter().any(|template| template.name == name) {
            return Err("模板名称重复".to_owned());
        }
        let template = MappingTemplate {
            id: new_template_id(),
            name,
            region_actions: BTreeMap::new(),
            adjustment_mode: AdjustmentMode::Volume,
        };
        self.templates.push(template.clone());
        Ok(template)
    }

    pub fn duplicate_template(
        &mut self,
        template_id: &str,
        name: String,
    ) -> Result<MappingTemplate, String> {
        let name = normalized_required(&name, "模板名称")?;
        if self.templates.iter().any(|template| template.name == name) {
            return Err("模板名称重复".to_owned());
        }
        let source = self
            .templates
            .iter()
            .find(|template| template.id == template_id)
            .cloned()
            .ok_or_else(|| "模板不存在".to_owned())?;
        let template = MappingTemplate {
            id: new_template_id(),
            name,
            ..source
        };
        self.templates.push(template.clone());
        Ok(template)
    }

    pub fn recommended_templates() -> Vec<MappingTemplate> {
        [
            ("preset-agent", "Agent", agent_regions()),
            ("preset-chat", "聊天工具", agent_regions()),
            ("preset-browser", "浏览器", browser_regions()),
        ]
        .into_iter()
        .map(|(id, name, region_actions)| MappingTemplate {
            id: id.to_owned(),
            name: name.to_owned(),
            region_actions,
            adjustment_mode: AdjustmentMode::Volume,
        })
        .collect()
    }

    pub fn apply_template_preset(
        &mut self,
        preset_id: &str,
        name: String,
    ) -> Result<MappingTemplate, String> {
        let preset = Self::recommended_templates()
            .into_iter()
            .find(|preset| preset.id == preset_id)
            .ok_or_else(|| "推荐模板不存在".to_owned())?;
        let created = self.create_template(name)?;
        let template = self
            .templates
            .iter_mut()
            .find(|template| template.id == created.id)
            .expect("new template is present");
        template.region_actions = preset.region_actions;
        template.adjustment_mode = preset.adjustment_mode;
        Ok(template.clone())
    }
}

fn action(single: SemanticAction) -> SemanticButtonActions {
    let long = if single == SemanticAction::OpenApplicationMenu {
        SemanticAction::OpenAdjustmentMenu
    } else {
        SemanticAction::Disabled
    };
    SemanticButtonActions {
        single,
        long,
        ..Default::default()
    }
}

fn agent_regions(
) -> BTreeMap<ControlRegion, BTreeMap<crate::raw_input::RemoteButton, SemanticButtonActions>> {
    use crate::raw_input::RemoteButton::*;
    use SemanticAction::*;
    let mut list = BTreeMap::new();
    list.insert(Up, action(SelectPrevious));
    list.insert(Down, action(SelectNext));
    list.insert(Left, action(SelectParent));
    list.insert(Right, action(ExpandSelection));
    list.insert(Ok, action(ActivateSelection));
    list.insert(Back, action(CancelSelection));
    list.insert(Home, action(FocusApplicationList));
    list.insert(Tv, action(FocusInput));
    list.insert(Menu, action(OpenApplicationMenu));
    list.insert(Power, action(Escape));
    let mut content = BTreeMap::new();
    content.insert(Up, action(ScrollUp));
    content.insert(Down, action(ScrollDown));
    content.insert(Left, action(FocusApplicationList));
    content.insert(Right, action(FocusInput));
    content.insert(Ok, action(FocusInput));
    content.insert(Back, action(FocusApplicationList));
    content.insert(Menu, action(OpenApplicationMenu));
    content.insert(Power, action(Escape));
    let mut input = BTreeMap::new();
    input.insert(
        Ok,
        SemanticButtonActions {
            single: Newline,
            long: Send,
            ..Default::default()
        },
    );
    input.insert(Back, action(Backspace));
    input.insert(Menu, action(OpenApplicationMenu));
    input.insert(Power, action(Escape));
    BTreeMap::from([
        (ControlRegion::ApplicationList, list),
        (ControlRegion::Content, content),
        (ControlRegion::Input, input),
    ])
}

fn browser_regions(
) -> BTreeMap<ControlRegion, BTreeMap<crate::raw_input::RemoteButton, SemanticButtonActions>> {
    use crate::raw_input::RemoteButton::*;
    use SemanticAction::*;
    let mut page = BTreeMap::new();
    page.insert(Up, action(ScrollUp));
    page.insert(Down, action(ScrollDown));
    page.insert(Left, action(PreviousTab));
    page.insert(Right, action(NextTab));
    page.insert(Ok, action(ActivateSelection));
    page.insert(Home, action(FocusInput));
    page.insert(Back, action(BrowserBack));
    page.insert(Menu, action(OpenApplicationMenu));
    page.insert(Power, action(Escape));
    let mut input = BTreeMap::new();
    input.insert(Ok, action(NativeEnter));
    input.insert(Back, action(Backspace));
    input.insert(Menu, action(OpenApplicationMenu));
    input.insert(Power, action(Escape));
    BTreeMap::from([
        (ControlRegion::Content, page),
        (ControlRegion::Input, input),
    ])
}

fn normalized_required(value: &str, field: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        Err(format!("{field}不能为空"))
    } else {
        Ok(value.to_owned())
    }
}

pub fn new_template_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let sequence = TEMPLATE_ID_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("template-{nanos:x}-{sequence:x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_makes_an_independent_stable_id() {
        let mut configuration = MappingConfiguration::default();
        let first = configuration.create_template("阅读".to_owned()).unwrap();
        let copy = configuration
            .duplicate_template(&first.id, "阅读副本".to_owned())
            .unwrap();
        assert_ne!(first.id, copy.id);
        assert_eq!(copy.name, "阅读副本");
    }

    #[test]
    fn bindings_must_reference_unique_existing_templates() {
        let configuration = MappingConfiguration {
            application_bindings: vec![ApplicationBinding {
                application_id: "edge".to_owned(),
                template_id: "missing".to_owned(),
                menu_order: 0,
                launch_target: None,
            }],
            ..Default::default()
        };
        assert!(configuration.normalized().is_err());
    }

    #[test]
    fn preset_application_copies_canonical_regions_without_enabling_control() {
        let presets = MappingConfiguration::recommended_templates();
        assert_eq!(presets, MappingConfiguration::recommended_templates());
        let mut configuration = MappingConfiguration::default();
        let original_common = configuration.common_mappings.clone();
        let first = configuration
            .apply_template_preset("preset-agent", "我的Agent".to_owned())
            .unwrap();
        let second = configuration
            .apply_template_preset("preset-agent", "工作Agent".to_owned())
            .unwrap();
        assert_ne!(first.id, second.id);
        assert_ne!(first.id, "preset-agent");
        assert_eq!(first.region_actions, presets[0].region_actions);
        assert!(!configuration.template_control_enabled);
        assert_eq!(configuration.common_mappings, original_common);
        assert!(configuration
            .apply_template_preset("preset-agent", first.name)
            .is_err());
        assert!(configuration
            .apply_template_preset("missing", "未知".to_owned())
            .is_err());
        for preset in presets {
            for region in preset.region_actions.values() {
                let menu = &region[&crate::raw_input::RemoteButton::Menu];
                assert_eq!(menu.single, SemanticAction::OpenApplicationMenu);
                assert_eq!(menu.long, SemanticAction::OpenAdjustmentMenu);
                assert_eq!(menu.double, SemanticAction::Disabled);
            }
        }
    }
}
