import { invoke } from "@tauri-apps/api/core";
import { assertOnboardingCanRestart, onboardingExternalState, waitForOnboardingCleanup } from "../onboarding/lifecycle";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import { t } from "./i18n";
import { effectiveLanguage, type EffectiveLanguage } from "./language-state";

export const VB_CABLE_DOWNLOAD_URL = "https://vb-audio.com/Cable/";

/**
 * Vokie 官网（2026-10-01 Andy 提供）。
 *
 * 同样是 opener 白名单的键：新增 URL 必须同步
 * `src-tauri/capabilities/default.json` 与 `bridge.test.ts` 的白名单断言。
 */
export const VOKIE_HOMEPAGE_URL = "https://vokie.com/";


export type ConnectionPhase =
  | "idle"
  | "connecting"
  | "discovering"
  | "awaiting_capabilities"
  | "ready"
  | "streaming"
  | "draining"
  | "reconnecting"
  | "suspended"
  | "disconnected"
  | "failed";

export type VoiceSessionState = "idle" | "streaming" | "draining";

export type RemoteModel = "rc001" | "rc003" | "unknown";

export type AudioPhase =
  | "unconfigured"
  | "ready"
  | "streaming"
  | "draining"
  | "failed"
  | "unsupported";

export interface AudioEndpoint {
  id: string;
  name: string;
  isVirtualCableCandidate: boolean;
}

/**
 * 界面上的“推荐”判据：值得推荐给输入法当麦克风来源的 VB-CABLE 渲染端点。
 *
 * 渲染端点的名字跨 VB-CABLE 驱动版本变化（2026-10-02 现场）：
 *   经典 2 通道：`CABLE Input (VB-Audio Virtual Cable)`
 *   新版 16 通道：`CABLE In 16 Ch (2- VB-Audio Virtual Cable)`
 * 两者都能把声音送回录音端 `CABLE Output`（`examples/cable_loopback_probe.rs`
 * 实测两个端点回环 peak 均 ≈11k），因此都推荐；VB-CABLE A/B 的
 * `CABLE-A/B Input` 不带 "VB-Audio Virtual Cable"，不推荐（它们的录音端不是
 * 输入法默认监听的 CABLE Output）。
 *
 * 后端的 `isVirtualCableCandidate` 是更宽的候选判定（含 VB-CABLE A/B 的
 * CABLE-A/B Input 与 CI 仿真端点），只用于自动选择与安装检测，不足以
 * 决定推荐标记；两者刻意分开。
 */
export function isRecommendedVoiceEndpoint(endpoint: AudioEndpoint): boolean {
  const name = endpoint.name.trim().toLowerCase();
  if (!name.includes("vb-audio virtual cable")) {
    return false;
  }
  return name.includes("cable input") || name.includes("cable in");
}

export interface AudioSnapshot {
  phase: AudioPhase;
  selectedEndpointId: string | null;
  selectedEndpointName: string | null;
  queuedSamples: number;
  submittedSamples: number;
  generation: number;
  lastError: string | null;
}

export type RawInputPhase = "stopped" | "starting" | "awaiting" | "ready" | "failed" | "unsupported";

export type RemoteButton =
  | "back"
  | "ok"
  | "tv"
  | "home"
  | "right"
  | "left"
  | "down"
  | "up"
  | "menu"
  | "power"
  | "volume_mute"
  | "volume_up"
  | "volume_down";

export type ButtonTrigger = "single" | "double" | "long";

export interface ButtonEdge {
  button: RemoteButton;
  isPressed: boolean;
}

export interface ShortcutCaptureEdge {
  key: KeyCode;
  isPressed: boolean;
  /** 边沿来源：real = 物理事件；injected = 外部钩子（输入法）吞下后重放的副本。 */
  source?: "real" | "injected";
}

export interface RawInputSnapshot {
  phase: RawInputPhase;
  matchedDeviceCount: number;
  rawEventCount: number;
  semanticEdgeCount: number;
  lastButton: RemoteButton | null;
  lastIsPressed: boolean | null;
  activeButtons: RemoteButton[];
  lastError: string | null;
  /** 报文来自遥控器但路径与绑定不符而被丢弃的次数（绑定失效的直接证据）。 */
  staleRemoteEventCount: number;
}

export type KeyCode = string;

export interface KeyChord {
  keys: KeyCode[];
}

export type MouseClickKind = "left" | "right" | "double_left" | "middle";
export type MoveDirection = "up" | "down" | "left" | "right";

/** 读取时翻译：常量对象也要跟随语言切换，不能在校验/加载时冻结文案。 */
export const mouseClickLabels: Record<MouseClickKind, string> = {
  get left() {
    return t("左键单击");
  },
  get right() {
    return t("右键单击");
  },
  get double_left() {
    return t("左键双击");
  },
  get middle() {
    return t("中键单击");
  },
};
export const mouseMoveLabels: Record<MoveDirection, string> = {
  get up() {
    return t("鼠标向上");
  },
  get down() {
    return t("鼠标向下");
  },
  get left() {
    return t("鼠标向左");
  },
  get right() {
    return t("鼠标向右");
  },
};

export type ButtonAction =
  | { type: "disabled" }
  | { type: "shortcut"; chord: KeyChord }
  | { type: "task_switch"; view: "applications" | "desktops" }
  | { type: "scroll"; direction: "up" | "down"; steps?: number }
  | { type: "mouse_click"; kind: MouseClickKind }
  | { type: "mouse_move"; direction: MoveDirection; distance: number }
  | { type: "open_app"; target: string }
  | { type: "focus_input" }
  | { type: "run_macro"; macroId: string; macroVersion: string }
  | { type: "open_common_phrases" };

// ===== 组合动作（Rust `sayall_core::macros` 的 camelCase 镜像）=====

export type MacroAction =
  | "open_application"
  | "wait_for_application"
  | "send_shortcut"
  | "focus_input"
  | "focus_frontmost"
  | "system_action"
  | "open_url"
  | "run_macro";

export type MacroModifier = "control" | "alt" | "shift" | "win";

/** 修饰键固定显示顺序（对齐 Mac PRODUCT_SPEC：Control → Option → Shift → Command）。 */
export const macroModifierOrder: MacroModifier[] = ["control", "alt", "shift", "win"];
export const macroModifierLabels: Record<MacroModifier, string> = {
  control: "Ctrl",
  alt: "Alt",
  shift: "Shift",
  win: "Win",
};

export interface MacroChord {
  modifiers: MacroModifier[];
  /** 语义键名（`n`、`return`、`escape`、`space`、方向键名、`f1`…`f12`）。 */
  key: string;
}

export type ApplicationWaitState = "running" | "visible_window";

export interface MacroParameters {
  target?: string;
  state?: ApplicationWaitState;
  chord?: MacroChord;
  url?: string;
  nestedMacroId?: string;
  nestedMacroName?: string;
}

export interface MacroStep {
  stepId: string;
  action: MacroAction;
  timeoutMs?: number;
  parameters: MacroParameters;
}

export interface MacroDefinition {
  schemaVersion: string;
  macroId: string;
  version: string;
  name: string;
  summary?: string;
  steps: MacroStep[];
}

export interface MacroLibraryDocument {
  formatVersion: number;
  definitions: MacroDefinition[];
}

export type MacroTerminalResult =
  | "completed"
  | "completed_with_unknown_result"
  | "step_failed"
  | "missing_dependency"
  | "permission_denied"
  | "timed_out"
  | "cancelled"
  | "invalid_macro"
  | "nested_too_deep"
  | "cycle_detected";

export type MacroStepOutcome =
  | "done"
  | "unknown"
  | "missing_dependency"
  | "permission_denied"
  | "failed";

export interface MacroStepReport {
  index: number;
  stepId: string;
  action: MacroAction;
  outcome: MacroStepOutcome;
  elapsedMs: number;
}

export interface MacroRunReport {
  macroId: string;
  result: MacroTerminalResult;
  elapsedMs: number;
  steps: MacroStepReport[];
}

export interface MacroRunStatus {
  running: boolean;
  report: MacroRunReport | null;
}

/** 预设应用条目（list_preset_apps 返回；对齐 Mac PresetApplication）。 */
export interface PresetAppInfo {
  id: string;
  name: string;
  installed: boolean;
}

/** 打开应用后的聚焦方式（对齐 Rust `FocusStrategy`）。 */
export type FocusStrategy =
  | "open_only"
  | "auto_composer"
  | "app_shortcut"
  | "recorded_element"
  | "recorded_point";

/** 相对目标顶层窗口的归一化矩形（物理像素换算成比例）。 */
export interface NormalizedRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

/**
 * 用户记录的输入框语义特征（不含任何输入内容）。
 *
 * `name` / `windowTitle` 属本地配置数据（可能含文档名等个人化信息），只存本地
 * 配置文件，不进日志、不随诊断上传。
 */
export interface RecordedFocusTarget {
  controlType: string;
  automationId: string;
  className?: string;
  name?: string;
  windowTitle?: string;
  normalizedRect?: NormalizedRect;
  contextTokens?: string[];
}

/** 「记录点击位置」：相对目标窗口的归一化坐标（0..1）与记录时的窗口尺寸。 */
export interface RecordedPoint {
  xRatio: number;
  yRatio: number;
  windowWidth: number;
  windowHeight: number;
}

/** 「打开应用」目标的聚焦档案（键 = open_app 的 target）。 */
export interface AppFocusProfile {
  strategy: FocusStrategy;
  shortcut?: KeyChord;
  recorded?: RecordedFocusTarget;
  point?: RecordedPoint;
}

/** 每键三列（单击/双击/长按），对齐 Mac 原版 ButtonTrigger。 */
export interface ButtonActions {
  single: ButtonAction;
  double: ButtonAction;
  long: ButtonAction;
  /**
   * 「按住连续触发」策略；缺省 = 自动（无长按时连续执行可重复的单击）。
   * off 显式关闭，single / long 显式指定连续执行的槽位。
   * 每键至多一个槽位，互斥由界面与 Rust `normalized()` 强制。
   */
  holdRepeat?: "off" | "single" | "long";
  /**
   * OK 键：用遥控器移动过光标后的 5 秒内，按 OK 直接点击光标位置
   * （缺省 = 关闭；只对 OK 键生效）。
   */
  okContextClick?: boolean;
}

export interface ButtonMappings {
  /** 配置结构版本（当前为 2：缺省自动，off 显式关闭；保存时由 Rust 校验）。 */
  schemaVersion?: number;
  enabled: boolean;
  actions: Partial<Record<RemoteButton, ButtonActions>>;
  applications?: AppLibraryEntry[];
  focusProfiles?: Record<string, AppFocusProfile>;
}

export interface CaptureInputSettings {
  enabled: boolean;
  endpointId: string | null;
  endpointName: string | null;
}
export interface AudioRouteSnapshot {
  phase: "paired" | "manual" | "unconfigured" | "unavailable";
  reason: string | null;
  captureEndpointId: string | null;
  captureEndpointName: string | null;
  renderEndpointId: string | null;
  renderEndpointName: string | null;
}
/** Read-back identity proof: ready render alone cannot establish the selected capture route. */
export function isPairedAudioRoute(
  route: AudioRouteSnapshot | null,
  capture: CaptureInputSnapshot | null,
  audio: AudioSnapshot | null,
): boolean {
  return route?.phase === "paired"
    && !!capture?.settings.endpointId
    && !capture.recoveryPending
    && !capture.lastError
    && route.captureEndpointId === capture.settings.endpointId
    && !!route.renderEndpointId
    && route.renderEndpointId === audio?.selectedEndpointId;
}

export async function getAudioRouteSnapshot(): Promise<AudioRouteSnapshot> {
  if (!isTauriRuntime()) return { phase: "unavailable", reason: "unsupported", captureEndpointId: null, captureEndpointName: null, renderEndpointId: null, renderEndpointName: null };
  return invoke<AudioRouteSnapshot>("get_audio_route_snapshot");
}
export interface CaptureInputSnapshot {
  settings: CaptureInputSettings;
  phase: string;
  recoveryPending: boolean;
  lastError: string | null;
}
export async function getCaptureInput(): Promise<CaptureInputSnapshot> {
  if (!isTauriRuntime()) return { settings: { enabled: false, endpointId: null, endpointName: null }, phase: "unsupported", recoveryPending: false, lastError: null };
  return invoke<CaptureInputSnapshot>("get_capture_input");
}
export async function listCaptureInputs(): Promise<AudioEndpoint[]> {
  if (!isTauriRuntime()) return [];
  return invoke<AudioEndpoint[]>("list_capture_inputs");
}
export async function setCaptureInput(config: CaptureInputSettings, writerEndpointId?: string): Promise<CaptureInputSnapshot> {
  if (!isTauriRuntime()) throw new Error("请在 Windows 应用中设置会话输入设备");
  return invoke<CaptureInputSnapshot>("set_capture_input", { config, ...(writerEndpointId ? { writerEndpointId } : {}) });
}
export async function resolveCaptureRecovery(restore: boolean): Promise<CaptureInputSnapshot> {
  if (!isTauriRuntime()) throw new Error("请在 Windows 应用中恢复输入设备");
  return invoke<CaptureInputSnapshot>("resolve_capture_recovery", { restore });
}
export interface RunningAppInfo { applicationId: string; name: string; preset: boolean; }

export interface MappingTemplate { id: string; name: string; mappings: ButtonMappings; }
export type ButtonMappingTemplate = MappingTemplate;
export interface ApplicationBinding { applicationId: string; templateId: string; menuOrder: number; launchTarget?: string | null; }
export interface TemplateCatalogEntry { id: string; name: string; kind: "direct"; builtIn: boolean; buttonMappings: ButtonMappings | null; }
export interface MappingConfiguration {
  menuUpdateDefault?: boolean;
  menuTemplateSwitchEnabled: boolean;
  mappingNoticeEnabled: boolean;
  commonMappings: ButtonMappings;
  templates: MappingTemplate[];
  applicationBindings: ApplicationBinding[];
  buttonMappingFollowEnabled: boolean;
  rememberedTemplateId?: string | null;
}
export interface MappingConfigurationImportPreview { token: string; sourceToken: string | null; formatVersion: number; configuration: MappingConfiguration; builtinTemplateIds: string[]; templateNameConflicts: string[]; unresolvedApplicationIds: string[]; }
export interface SceneMenuItem { applicationId:string|null; templateId:string; label:string; running:boolean; }
export interface MappingNotice { kind:string; templateId:string|null; name:string|null; actionsAvailable:boolean; defaultSaveStatus?:string|null; manualSaveStatus?:"saving"|"saved"|"failed"|null; }
export interface SceneSnapshot { mappingNoticeEnabled:boolean; mappingNotice:MappingNotice|null; mappingNoticeRevision:number; enabled:boolean; generation:number; menuGeneration:number|null; foregroundGeneration:number; applicationId:string|null; templateId:string|null; panel:"template"|null; updateDefault:boolean; preferencePending?:boolean; preferenceError?:boolean; selectedIndex:number|null; menuItems:SceneMenuItem[]; waitingForRelease:boolean; voiceActive:boolean; voiceButtonPressed:boolean; status:string|null; }
export type SceneEvent =
  | { type:"mapping_notice_enabled"; enabled:boolean }
  | { type:"snapshot"; snapshot:SceneSnapshot }
  | { type:"mapping_applied"; notice:MappingNotice; revision:number }
  | { type:"manual_template_persistence_requested"; request_id:number; template_id:string|null }
  | { type:"default_template_persistence_requested"; requestId:number; applicationId:string; templateId:string };
export interface TemplateImportRequest { templateIds: string[]; resolvedNames: Record<string,string>; replaceApplicationBindings: boolean; }
export interface TemplateImportPreview { token:string; templates:Array<{sourceTemplateId:string;template:MappingTemplate}>; addedApplicationBindings:ApplicationBinding[]; replacedApplicationIds:string[]; skippedApplicationIds:string[]; unresolvedApplicationIds:string[]; }
export type ComponentKind = "vb_cable";
export type ComponentAction = "install" | "repair" | "remove" | "open_vendor_wizard";
export type InstallationState = "unknown" | "not_installed" | "installed_not_loaded" | "available" | "restart_required" | "incompatible" | "failed" | "not_implemented";
export type PackageState = "missing" | "trusted" | "signature_missing" | "authorization_missing" | "incompatible" | "failed" | "download_available";
export type ComponentReason =
  | "ready" | "not_installed" | "service_not_loaded" | "audio_endpoints_missing" | "identity_unavailable"
  | "package_missing" | "signing_policy_missing" | "authorization_missing" | "uninstall_package_missing"
  | "unsupported_platform" | "unsupported_architecture" | "detection_failed" | "access_denied"
  | "restart_required" | "path_rejected" | "hash_mismatch" | "signature_invalid" | "publisher_mismatch"
  | "version_mismatch" | "invalid_package" | "uac_cancelled" | "uac_denied" | "helper_unavailable"
  | "helper_timed_out" | "helper_failed" | "verification_failed" | "operation_unsupported" | "operation_in_progress"
  | "not_implemented" | "official_wizard_required" | "download_failed" | "wizard_closed";
export type ComponentOperationOutcome = "blocked" | "cancelled" | "denied" | "timed_out" | "restart_required" | "failed" | "completed" | "wizard_closed";
export interface ComponentStatus {
  component: ComponentKind; installation: InstallationState; package: PackageState;
  installedVersion: string | null; serviceInstalled: boolean | null; loaded: boolean | null;
  bound: boolean | null; audioEndpointsReady: boolean | null; restartRequired: boolean;
  reason: ComponentReason; blockers: ComponentReason[]; allowedActions: ComponentAction[];
}
export interface ComponentOperation {
  component: ComponentKind; action: ComponentAction; outcome: ComponentOperationOutcome;
  reason: ComponentReason; status: ComponentStatus;
}

export interface FiredGesture {
  button: RemoteButton;
  trigger: ButtonTrigger;
}

/** 「聚焦输入框」最近一次结果（focus_service::FocusReport；reason 见 FocusFailure 标识）。 */
export interface FocusReport {
  requestId: number;
  kind: string;
  focused: boolean;
  reason?: string;
  attempts: number;
  elapsedMs: number;
}

export interface ButtonMappingSnapshot {
  observedButtons: RemoteButton[];
  enabled: boolean;
  gateActive: boolean;
  listenerActive: boolean;
  swallowedEdges: number;
  leakedDowns: number;
  firedGestures: number;
  lastFired: FiredGesture | null;
  lastError: string | null;
  lastFocus?: FocusReport;
}

export interface SendInputSnapshot {
  available: boolean;
  submittedBatches: number;
  submittedEvents: number;
  lastError: string | null;
}

export interface AtvvCapabilities {
  version: number;
  codecs: number;
  interaction: number;
  frameSize: number;
  selectedCodec: number;
  sampleRate: number;
}

export interface ConnectionSnapshot {
  phase: ConnectionPhase;
  batteryLevel?: number | null;
  remoteName: string | null;
  remoteModel: RemoteModel;
  /** 当前连接设备的稳定令牌（Rust `device_token`，与扫描列表同一套算法）；老快照没有此字段。 */
  remoteToken?: string | null;
  capabilities: AtvvCapabilities | null;
  voiceState: VoiceSessionState;
  decodedSamples: number;
  /**
   * 最近一次足够长的语音会话的音频完整度（2026-10-24 现场）。
   *
   * `degraded` = 遥控器音频只到了满速的约 1/3（常见于蓝牙适配器同时连接了
   * 另一支 BLE 键盘/鼠标/耳机），输入法可能转写不出文字。老快照没有此字段，
   * 一律按 `unknown` 处理（不提示）。
   */
  voiceAudioHealth?: VoiceAudioHealth;
  /** 最近一次判定的实际样本率（Hz）；`null`/缺失 = 尚未判定。 */
  voiceAudioRateHz?: number | null;
  generation: number;
  reconnectAttempt: number;
  powerNotificationsAvailable: boolean;
  lastError: string | null;
}

/** 与 Rust `VoiceAudioHealth` 逐字对应。 */
export type VoiceAudioHealth = "unknown" | "full" | "degraded";

export interface PlatformSnapshot {
  platform: string;
  windowsApiAvailable: boolean;
  bleScanAvailable: boolean;
  bleVoiceReady: boolean;
  wasapiReady: boolean;
  rawInputReady: boolean;
  sendInputReady: boolean;
  verificationStatus: string;
  connection: ConnectionSnapshot;
  audio: AudioSnapshot;
  rawInput: RawInputSnapshot;
  buttonMapping: ButtonMappingSnapshot;
}

export interface RuntimeSnapshot {
  appVersion: string;
  platform: PlatformSnapshot;
}

export interface DiagnosticReport {
  schemaVersion: number;
  appVersion: string;
  /** 构建号（CI / 本地构建注入；取不到为 "unknown"）。 */
  appBuild: string;
  /** 源码修订（40 位 git SHA；取不到为 "unknown"）。 */
  sourceRevision: string;
  /** 发布通道（取不到为 "unknown"）。 */
  buildChannel: string;
  /** 运行机器的 Windows 版本（major.minor.build，如 "10.0.26100"）。 */
  windowsVersion: string;
  /** 进程架构（x86_64 等）。 */
  processArchitecture: string;
  /**
   * 本次开机的引导类型（2026-10-20）：`full` / `hiberboot` / `hibernate_resume` /
   * `unknown`。快速启动（hiberboot）开机后部分蓝牙适配器会带旧固件状态、
   * 造成链路反复断开——报障判读的第一个分流问题就是"这次开机是不是 hiberboot"。
   */
  bootType: string;
  /**
   * 「快速启动」功能是否开启；**读不到为 `null`**（不是 false——读不到不等于没开）。
   * 与 `bootType` 并列：开关开着但用户点的是「重启」时，`bootType` 仍是 `full`。
   */
  fastStartupEnabled: boolean | null;
  platform: string;
  verificationStatus: string;
  capabilities: {
    windowsApiAvailable: boolean;
    bleScanAvailable: boolean;
    bleVoiceReady: boolean;
    wasapiReady: boolean;
    rawInputReady: boolean;
    sendInputReady: boolean;
  };
  connection: {
    phase: ConnectionPhase;
    capabilitiesConfirmed: boolean;
    sampleRate: number | null;
    frameSize: number | null;
    decodedSamples: number;
    generation: number;
    reconnectAttempt: number;
    powerNotificationsAvailable: boolean;
    errorPresent: boolean;
  };
  audio: {
    phase: AudioPhase;
    endpointConfigured: boolean;
    queuedSamples: number;
    submittedSamples: number;
    generation: number;
    errorPresent: boolean;
  };
  rawInput: {
    phase: RawInputPhase;
    matchedDeviceCount: number;
    rawEventCount: number;
    semanticEdgeCount: number;
    lastButton: RemoteButton | null;
    lastIsPressed: boolean | null;
    errorPresent: boolean;
  };
  sendInput: {
    available: boolean;
    submittedBatches: number;
    submittedEvents: number;
    errorPresent: boolean;
  };
  buttonMapping: {
    enabled: boolean;
    gateActive: boolean;
    listenerActive: boolean;
    swallowedEdges: number;
    leakedDowns: number;
    firedGestures: number;
    errorPresent: boolean;
  };
  /** 输入法（2026-10-13 现场，schemaVersion 3）：诊断"语音没出字是不是没切输入法"。 */
  ime: {
    detected: boolean;
    activeName: string | null;
    installedNames: string[];
    expectedName: string | null;
    matchesExpected: boolean | null;
  };
}

export interface PairedRemote {
  id: string;
  name: string;
  model: RemoteModel;
  isSupportedCandidate: boolean;
  /** 设备稳定令牌（Rust `device_token`）：把扫描行与"当前连着的设备"对上；老快照没有此字段。 */
  token?: string;
}

/** 应用内更新（Rust updater command 契约，camelCase 对齐 src-tauri/src/updater.rs）。 */
export interface AppUpdateInfo {
  currentVersion: string;
  available: boolean;
  version: string | null;
  notes: string | null;
  date: string | null;
}

export interface AppUpdatePreferences {
  includePrereleases: boolean;
}

export type ThemePreference = "system" | "light" | "dark";

/** 应用语言偏好；与 Rust `AppLanguage` / Mac `AppLanguage` 的持久化值一致。 */
export type AppLanguagePreference = "system" | "zh-Hans" | "en";
/** 推荐页实际使用的语言；`system` 已在语言模块里解析成这两个之一。 */
export type { EffectiveLanguage } from "./language-state";

/** Windows 系统强调色（设置 > 个性化 > 颜色），Rust accent 命令契约。 */
export interface AccentRgb {
  r: number;
  g: number;
  b: number;
}

export async function getSystemAccentColor(): Promise<AccentRgb | null> {
  if (!isTauriRuntime()) return null;
  return invoke<AccentRgb | null>("get_system_accent_color");
}

export async function subscribeAccentChanges(
  handler: (color: AccentRgb) => void,
): Promise<() => void> {
  if (!isTauriRuntime()) {
    return () => {};
  }
  const { listen } = await import("@tauri-apps/api/event");
  const unlisten = await listen<AccentRgb>("system-accent-changed", (event) =>
    handler(event.payload),
  );
  return () => {
    void unlisten();
  };
}

export async function getLaunchAtLogin(): Promise<boolean> {
  if (!isTauriRuntime()) return false;
  return invoke<boolean>("get_launch_at_login");
}

export async function setLaunchAtLogin(enabled: boolean): Promise<boolean> {
  if (!isTauriRuntime()) throw new Error("当前是浏览器预览，无法设置开机自启动");
  return invoke<boolean>("set_launch_at_login", { enabled });
}

/**
 * 「启动时自动打开主面板」（设置页「启动行为」，对齐 Mac `openMainWindowAtLaunch`）。
 *
 * 浏览器预览按新装默认返回 `true`，与 Rust `AppSettings::default()` 同源。
 */
export async function getOpenWindowAtLaunch(): Promise<boolean> {
  if (!isTauriRuntime()) return true;
  return invoke<boolean>("get_open_window_at_launch");
}

export async function setOpenWindowAtLaunch(enabled: boolean): Promise<boolean> {
  if (!isTauriRuntime()) throw new Error("当前是浏览器预览，无法设置启动时打开主面板");
  return invoke<boolean>("set_open_window_at_launch", { enabled });
}

/**
 * 本次启动前刚完成更新时的版本号（对齐 Mac `update.completed.*`）。
 *
 * Rust 侧取走即清空：同一次运行里只提示一次，重新挂载页面不会重复弹。
 * 浏览器预览没有更新机制，返回 `null`。
 */
export async function takeUpdateCompletedVersion(): Promise<string | null> {
  if (!isTauriRuntime()) return null;
  return invoke<string | null>("take_update_completed_version");
}

export interface AppUpdateProgress {
  downloaded: number;
  contentLength: number | null;
  finished: boolean;
}

const browserSnapshot: RuntimeSnapshot = {
  // 浏览器预览没有安装包可读，这里跟随当前应用版本：它是预览里"设置页版本号"
  // 与侧栏底部的唯一来源，写死旧值会让预览显示一个不存在的版本。
  appVersion: "0.8.2",
  platform: {
    platform: "browser-preview",
    windowsApiAvailable: false,
    bleScanAvailable: false,
    bleVoiceReady: false,
    wasapiReady: false,
    rawInputReady: false,
    sendInputReady: false,
    verificationStatus: "浏览器预览仅展示界面，不代表真机已通过",
    connection: {
      phase: "idle",
      remoteName: null,
      remoteModel: "unknown",
      capabilities: null,
      voiceState: "idle",
      decodedSamples: 0,
      generation: 0,
      reconnectAttempt: 0,
      powerNotificationsAvailable: false,
      lastError: null,
    },
    audio: {
      phase: "unsupported",
      selectedEndpointId: null,
      selectedEndpointName: null,
      queuedSamples: 0,
      submittedSamples: 0,
      generation: 0,
      lastError: null,
    },
    rawInput: {
      phase: "unsupported",
      matchedDeviceCount: 0,
      rawEventCount: 0,
      semanticEdgeCount: 0,
      lastButton: null,
      lastIsPressed: null,
      activeButtons: [],
      lastError: null,
      staleRemoteEventCount: 0,
    },
    buttonMapping: {
      enabled: true,
      gateActive: false,
      observedButtons: [],
      listenerActive: false,
      swallowedEdges: 0,
      leakedDowns: 0,
      firedGestures: 0,
      lastFired: null,
      lastError: null,
    },
  },
};

export function isTauriRuntime(): boolean {
  return "__TAURI_INTERNALS__" in window;
}

export async function getRuntimeSnapshot(): Promise<RuntimeSnapshot> {
  if (!isTauriRuntime()) {
    return browserSnapshot;
  }
  return invoke<RuntimeSnapshot>("get_runtime_snapshot");
}

/** 设置页账号区只需要的会员摘要；不携带令牌、租约或设备标识。 */
export interface MembershipAccountSummary {
  availability: "available" | "unavailable";
  accountDisplayName: string | null;
  plusActive: boolean;
  accountSignedIn: boolean;
  syncState: "current" | "offline" | "unavailable";
}

export async function getMembershipAccountSummary(): Promise<MembershipAccountSummary> {
  if (!isTauriRuntime()) {
    return {
      availability: "unavailable",
      accountDisplayName: null,
      plusActive: false,
      accountSignedIn: false,
      syncState: "unavailable",
    };
  }
  return invoke<MembershipAccountSummary>("get_membership_snapshot");
}

/** 个人中心日历的一天（本地日期 `YYYY-MM-DD`）。 */
export interface UsageProfileDay {
  date: string;
  buttonPresses: number;
  voiceSeconds: number;
}

/** 单次语音排行的一条；应用名只来自本机保存的显示名。 */
export interface UsageVoiceSession {
  endedAtMs: number;
  durationSeconds: number;
  applicationName: string | null;
}

/** 个人中心读模型（`get_usage_profile`）。 */
export interface UsageProfile {
  totalButtonPresses: number;
  totalVoiceSeconds: number;
  weekButtonPresses: number;
  longestVoiceSeconds: number;
  buttonPressesByControl: Record<string, number>;
  voiceSessions: UsageVoiceSession[];
  recentDays: UsageProfileDay[];
}

function browserDateKey(offsetFromToday: number): string {
  const date = new Date();
  date.setHours(12, 0, 0, 0);
  date.setDate(date.getDate() + offsetFromToday);
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

/** 浏览器预览用的固定样例（不进入 Tauri 路径，只用于布局走查）。 */
function browserUsageProfile(): UsageProfile {
  const recentDays: UsageProfileDay[] = Array.from({ length: 26 * 7 }, (_, index) => {
    const offset = index - 26 * 7 + 1;
    const active = (index * 7 + 3) % 5 !== 0;
    return {
      date: browserDateKey(offset),
      buttonPresses: active ? (index * 13) % 48 : 0,
      voiceSeconds: active && index % 4 === 0 ? (index * 17) % 301 : 0,
    };
  });
  return {
    totalButtonPresses: 561,
    totalVoiceSeconds: 33 * 60 + 12,
    weekButtonPresses: 9,
    longestVoiceSeconds: 85,
    buttonPressesByControl: {
      "button.back": 47,
      "button.ok": 42,
      "button.home": 45,
      "button.down": 45,
      "button.up": 41,
      "button.left": 40,
      "button.right": 45,
      "button.menu": 23,
      "button.tv": 12,
      "button.power": 8,
      "button.volume_up": 22,
      "button.volume_down": 19,
      "button.volume_mute": 11,
      voice: 132,
    },
    voiceSessions: [
      {
        endedAtMs: Date.now() - 3 * 60 * 60 * 1000,
        durationSeconds: 85,
        applicationName: "微信",
      },
      {
        endedAtMs: Date.now() - 26 * 60 * 60 * 1000,
        durationSeconds: 82,
        applicationName: "Chrome 浏览器",
      },
      {
        endedAtMs: Date.now() - 3 * 24 * 60 * 60 * 1000,
        durationSeconds: 61,
        applicationName: null,
      },
    ],
    recentDays,
  };
}

export async function getUsageProfile(): Promise<UsageProfile> {
  if (!isTauriRuntime()) {
    return browserUsageProfile();
  }
  return invoke<UsageProfile>("get_usage_profile");
}

export async function getDiagnosticReport(): Promise<DiagnosticReport> {
  if (!isTauriRuntime()) {
    return {
      schemaVersion: 4,
      appVersion: browserSnapshot.appVersion,
      appBuild: "unknown",
      sourceRevision: "unknown",
      buildChannel: "unknown",
      windowsVersion: "unknown",
      processArchitecture: "browser-preview",
      // 浏览器预览没有真实的开机上下文：一律 unknown / null（不是 false）。
      bootType: "unknown",
      fastStartupEnabled: null,
      platform: browserSnapshot.platform.platform,
      verificationStatus: browserSnapshot.platform.verificationStatus,
      capabilities: {
        windowsApiAvailable: false,
        bleScanAvailable: false,
        bleVoiceReady: false,
        wasapiReady: false,
        rawInputReady: false,
        sendInputReady: false,
      },
      connection: {
        phase: browserSnapshot.platform.connection.phase,
        capabilitiesConfirmed: false,
        sampleRate: null,
        frameSize: null,
        decodedSamples: 0,
        generation: 0,
        reconnectAttempt: 0,
        powerNotificationsAvailable: false,
        errorPresent: false,
      },
      audio: {
        phase: browserSnapshot.platform.audio.phase,
        endpointConfigured: false,
        queuedSamples: 0,
        submittedSamples: 0,
        generation: 0,
        errorPresent: false,
      },
      rawInput: {
        phase: browserSnapshot.platform.rawInput.phase,
        matchedDeviceCount: 0,
        rawEventCount: 0,
        semanticEdgeCount: 0,
        lastButton: null,
        lastIsPressed: null,
        errorPresent: false,
      },
      sendInput: {
        available: false,
        submittedBatches: 0,
        submittedEvents: 0,
        errorPresent: false,
      },
      buttonMapping: {
        enabled: true,
        gateActive: false,
        listenerActive: false,
        swallowedEdges: 0,
        leakedDowns: 0,
        firedGestures: 0,
        errorPresent: false,
      },
      ime: {
        detected: false,
        activeName: null,
        installedNames: [],
        expectedName: null,
        matchesExpected: null,
      },
    };
  }
  return invoke<DiagnosticReport>("get_diagnostic_report");
}

export function formatDiagnosticReport(
  report: DiagnosticReport,
  generatedAt = new Date().toISOString(),
): string {
  return JSON.stringify({ generatedAt, ...report }, null, 2);
}

/**
 * 打开诊断日志目录（关于页入口）。返回实际打开的目录供界面显示。
 *
 * 目录由 Rust 侧从日志初始化的落盘路径推导，前端不拼接、也不传路径。
 */
export async function openLogDirectory(): Promise<string> {
  if (!isTauriRuntime()) throw new Error("当前是浏览器预览，无法打开日志目录");
  return invoke<string>("open_log_directory");
}

/**
 * Ctrl+W：关闭主窗口——隐藏到托盘驻留，语义与点标题栏“X”完全一致。
 *
 * 有意**不**调用 `@tauri-apps/api` 的 `getCurrentWindow().close()`：那条路径在
 * Windows 上究竟是触发 `CloseRequested`（→ Rust 侧 `prevent_close` + hide，
 * 即隐藏到托盘）还是直接销毁窗口，取决于 tao 的平台实现细节，跨版本可能静默
 * 改变语义；这里显式调 Rust 命令，动作与“X”的收尾是同一行代码。
 */
export async function hideMainWindow(): Promise<void> {
  if (!isTauriRuntime()) throw new Error("当前是浏览器预览，无法关闭窗口");
  await invoke("hide_main_window");
}

export async function scanPairedRemotes(): Promise<PairedRemote[]> {
  if (!isTauriRuntime()) {
    throw new Error("当前是浏览器预览，无法读取已配对设备");
  }
  return invoke<PairedRemote[]>("scan_paired_remotes");
}

export async function getConnectionSnapshot(): Promise<ConnectionSnapshot> {
  if (!isTauriRuntime()) {
    return browserSnapshot.platform.connection;
  }
  return invoke<ConnectionSnapshot>("get_connection_snapshot");
}

export async function connectRemote(deviceId: string): Promise<ConnectionSnapshot> {
  if (!isTauriRuntime()) {
    throw new Error("当前是浏览器预览，无法连接遥控器");
  }
  return invoke<ConnectionSnapshot>("connect_remote", { deviceId });
}

export async function disconnectRemote(): Promise<ConnectionSnapshot> {
  if (!isTauriRuntime()) {
    throw new Error("当前是浏览器预览，无法断开遥控器");
  }
  return invoke<ConnectionSnapshot>("disconnect_remote");
}

export async function listAudioEndpoints(): Promise<AudioEndpoint[]> {
  if (!isTauriRuntime()) {
    throw new Error("当前是浏览器预览，无法读取音频设备");
  }
  return invoke<AudioEndpoint[]>("list_audio_endpoints");
}

export async function getAudioSnapshot(): Promise<AudioSnapshot> {
  if (!isTauriRuntime()) {
    return browserSnapshot.platform.audio;
  }
  return invoke<AudioSnapshot>("get_audio_snapshot");
}

export async function selectAudioEndpoint(endpointId: string): Promise<AudioSnapshot> {
  if (!isTauriRuntime()) {
    throw new Error("当前是浏览器预览，无法选择音频设备");
  }
  return invoke<AudioSnapshot>("select_audio_endpoint", { endpointId });
}

/** 向导入口用到的固定 Windows 设置页（不接收任意 URI；Rust 侧另有 ms-settings 前缀校验）。 */
export type WindowsSettingsSection = "bluetooth" | "sound" | "microphone";

export async function openWindowsSettings(section: WindowsSettingsSection): Promise<void> {
  if (!isTauriRuntime()) {
    return;
  }
  return invoke<void>("open_windows_settings", { section });
}

/**
 * 语音增益（dB，0–24；对齐 Mac 设置页「增益」滑块，0 = 原始音量）。
 *
 * 读取走持久化值；写入返回实际保存值（越界会被钳制），界面用它回显——
 * 不假设"写进去什么就存什么"。
 */
export async function getGainDb(): Promise<number> {
  if (!isTauriRuntime()) {
    return 0;
  }
  return invoke<number>("get_gain_db");
}

export async function setGainDb(gainDb: number): Promise<number> {
  if (!isTauriRuntime()) {
    return gainDb;
  }
  return invoke<number>("set_gain_db", { gainDb });
}

export async function openVbCableDownloadPage(): Promise<void> {
  if (!isTauriRuntime()) {
    window.open(VB_CABLE_DOWNLOAD_URL, "_blank", "noopener,noreferrer");
    return;
  }
  await openUrl(VB_CABLE_DOWNLOAD_URL);
}

/**
 * 外部链接的统一出口：浏览器预览开新标签，Tauri 运行时交给 opener 插件
 * （插件只放行 capabilities 白名单内的 URL，前端不做过滤，也不拼接参数）。
 */
async function openExternalUrl(url: string): Promise<void> {
  if (!isTauriRuntime()) {
    window.open(url, "_blank", "noopener,noreferrer");
    return;
  }
  await openUrl(url);
}

/** 连接页“Vokie 未安装”提示里的官网入口（2026-10-01）。 */
export async function openVokieHomepage(): Promise<void> {
  await openExternalUrl(VOKIE_HOMEPAGE_URL);
}


/** Vokie 检测结果：未安装时连接页显示官网入口；已安装但没运行时提示先启动它。 */
export interface VokieInstallation {
  installed: boolean;
  running: boolean;
}

export async function getVokieInstallation(): Promise<VokieInstallation> {
  if (!isTauriRuntime()) {
    return { installed: false, running: false };
  }
  return invoke<VokieInstallation>("get_vokie_installation");
}

/**
 * 打开 Vokie（连接页第 ② 步「打开 Vokie」按钮）：装了但没运行时一键叫起来。
 * 失败时抛错（调用方把原因显示给用户）。
 */
export async function launchVokie(): Promise<void> {
  if (!isTauriRuntime()) {
    return;
  }
  return invoke<void>("launch_vokie");
}

/**
 * 「其他工具」面板记住的按键：`null` = 从未选过（保持现状），`[]` = 明确选了
 * 「不按键」。选中「其他工具」时恢复它，用户改选时写回。
 */
export async function getOtherVoiceHotkey(): Promise<KeyCode[] | null> {
  if (!isTauriRuntime()) {
    return null;
  }
  return invoke<KeyCode[] | null>("get_other_voice_hotkey");
}

export async function setOtherVoiceHotkey(keys: KeyCode[]): Promise<KeyCode[] | null> {
  if (!isTauriRuntime()) {
    return keys;
  }
  return invoke<KeyCode[] | null>("set_other_voice_hotkey", { keys });
}

/**
 * 首次使用向导状态（Rust `onboarding.json`，设计稿 §6）。
 *
 * `isActive` = 当前流程版本尚未处理（验证完成或用户主动稍后设置都会关闭自动展示）。步骤 token 由
 * `src/onboarding/flow.ts` 的 `normalizeStep` 归一化（未知值 → welcome）。
 */
export interface OnboardingState {
  flowVersion: number;
  completedVersion: number;
  step: string;
  isActive: boolean;
}

/**
 * 浏览器预览（`pnpm dev`）兜底：默认「已完成」，预览环境用于页面开发，
 * 不把开发者锁进向导；真机/仿真环境始终走 Rust 状态文件。
 */
const BROWSER_ONBOARDING_STATE: OnboardingState = {
  flowVersion: 1,
  completedVersion: 1,
  step: "complete",
  isActive: false,
};

export async function getOnboardingState(): Promise<OnboardingState> {
  if (!isTauriRuntime()) {
    return { ...BROWSER_ONBOARDING_STATE };
  }
  return invoke<OnboardingState>("get_onboarding_state");
}

export async function saveOnboardingStep(step: string): Promise<OnboardingState> {
  if (!isTauriRuntime()) {
    return { ...BROWSER_ONBOARDING_STATE };
  }
  return invoke<OnboardingState>("save_onboarding_step", { step });
}

/** 设置页「重新运行设置向导」：只重置向导进度，不清除设备/映射/音频/其他设置。 */
export async function restartOnboarding(): Promise<OnboardingState> {
  assertOnboardingCanRestart();
  await waitForOnboardingCleanup();
  assertOnboardingCanRestart();
  if (!isTauriRuntime()) {
    return { ...BROWSER_ONBOARDING_STATE };
  }
  const state = await invoke<OnboardingState>("restart_onboarding");
  onboardingExternalState.value = null;
  return state;
}

export async function completeOnboarding(): Promise<OnboardingState> {
  if (!isTauriRuntime()) {
    return { ...BROWSER_ONBOARDING_STATE };
  }
  return invoke<OnboardingState>("complete_onboarding");
}

/** 退出/跳过结束不提交未验证绑定；保留进入向导前的有效工具与快捷键。 */
export async function dismissOnboarding(): Promise<OnboardingState> {
  if (!isTauriRuntime()) return { ...BROWSER_ONBOARDING_STATE };
  return invoke<OnboardingState>("dismiss_onboarding");
}

/** 停止本轮实际语音测试，释放注入按键与临时音频会话。 */
export async function cancelVoiceTest(): Promise<void> {
  if (!isTauriRuntime()) return;
  return invoke<void>("cancel_voice_test");
}

/**
 * 第④步暂存语音绑定：落回滚快照 + 应用正式配置与运行时（设计稿 §5.4）。
 * 只有向导进行中可调用；退出未完成流程/重跑向导会回滚到进入向导前的配置。
 */
export async function stageOnboardingVoiceBinding(
  tool: VoiceInputTool,
  hotkey: KeyChord | null,
): Promise<OnboardingState> {
  if (!isTauriRuntime()) {
    return { ...BROWSER_ONBOARDING_STATE };
  }
  return invoke<OnboardingState>("stage_onboarding_voice_binding", { tool, hotkey });
}

export async function getRawInputSnapshot(): Promise<RawInputSnapshot> {
  if (!isTauriRuntime()) {
    return browserSnapshot.platform.rawInput;
  }
  return invoke<RawInputSnapshot>("get_raw_input_snapshot");
}

export async function startRawInput(): Promise<RawInputSnapshot> {
  if (!isTauriRuntime()) {
    throw new Error("当前是浏览器预览，无法启动按键监听");
  }
  return invoke<RawInputSnapshot>("start_raw_input");
}

export async function stopRawInput(): Promise<RawInputSnapshot> {
  if (!isTauriRuntime()) {
    throw new Error("当前是浏览器预览，无法停止按键监听");
  }
  return invoke<RawInputSnapshot>("stop_raw_input");
}

export async function getButtonMappings(): Promise<ButtonMappings> {
  if (!isTauriRuntime()) {
    return { enabled: true, actions: {} };
  }
  return invoke<ButtonMappings>("get_button_mappings");
}

export async function openBluetoothSettings(): Promise<void> {
  if (!isTauriRuntime()) throw new Error("当前是浏览器预览，无法打开 Windows 蓝牙设置");
  await invoke("open_bluetooth_settings");
}

export async function getMappingConfiguration(): Promise<MappingConfiguration> {
  if (!isTauriRuntime()) return { menuTemplateSwitchEnabled: false, mappingNoticeEnabled: true, commonMappings: { enabled: true, actions: {} }, templates: [], applicationBindings: [], buttonMappingFollowEnabled: false };
  return invoke<MappingConfiguration>("get_mapping_configuration");
}
export async function saveMappingConfiguration(configuration: MappingConfiguration): Promise<MappingConfiguration> {
  return invoke<MappingConfiguration>("save_mapping_configuration", { configuration });
}
export async function setMappingNoticeEnabled(enabled: boolean): Promise<MappingConfiguration> {
  return invoke("set_mapping_notice_enabled", { enabled });
}
export async function setButtonMappingFollowEnabled(enabled: boolean): Promise<MappingConfiguration> {
  return invoke<MappingConfiguration>("set_button_mapping_follow_enabled", { enabled });
}
export async function createMappingTemplate(name: string): Promise<MappingTemplate> { return invoke("create_mapping_template", { name }); }
export async function saveButtonMappingTemplate(name: string, mappings: ButtonMappings): Promise<ButtonMappingTemplate> { return invoke("save_button_mapping_template", { name, mappings }); }
export async function duplicateButtonMappingTemplate(templateId: string, name: string): Promise<ButtonMappingTemplate> { return invoke("duplicate_button_mapping_template", { templateId, name }); }
export async function resetBuiltinTemplate(templateId: string): Promise<MappingConfiguration> { return invoke("reset_builtin_template", { templateId }); }
export async function updateButtonMappingTemplate(templateId: string, mappings: ButtonMappings): Promise<ButtonMappingTemplate> { return invoke("update_button_mapping_template", { templateId, mappings }); }
export async function reorderApplicationAssociations(applicationIds: string[]): Promise<MappingConfiguration> { return invoke("reorder_application_associations", { applicationIds }); }
export async function duplicateMappingTemplate(templateId: string, name: string): Promise<MappingTemplate> { return invoke("duplicate_mapping_template", { templateId, name }); }
export async function renameMappingTemplate(templateId: string, name: string): Promise<MappingConfiguration> { return invoke("rename_mapping_template", { templateId, name }); }
export async function deleteMappingTemplate(templateId: string, replacementTemplateId: string | null, unbindApplications = false): Promise<MappingConfiguration> { return invoke("delete_mapping_template", { templateId, replacementTemplateId, unbindApplications }); }
export async function upsertApplicationBinding(binding: ApplicationBinding): Promise<MappingConfiguration> { return invoke("upsert_application_binding", { binding }); }
export async function removeApplicationBinding(applicationId: string): Promise<MappingConfiguration> { return invoke("remove_application_binding", { applicationId }); }
export async function exportMappingConfiguration(templateIds: string[] | null = null): Promise<boolean> { return invoke("export_mapping_configuration", { templateIds }); }
export async function previewMappingConfigurationImport(): Promise<MappingConfigurationImportPreview | null> { return invoke("preview_mapping_configuration_import"); }
export async function applyMappingConfigurationImport(token: string): Promise<MappingConfiguration> { return invoke("apply_mapping_configuration_import", { token }); }
export async function previewTemplateImport(sourceToken:string, request:TemplateImportRequest):Promise<TemplateImportPreview>{return invoke("preview_template_import",{sourceToken,request});}
export async function getTemplateCatalog():Promise<TemplateCatalogEntry[]>{return invoke("get_template_catalog");}
export async function copyTemplateCatalogEntry(templateId:string,name:string):Promise<TemplateCatalogEntry>{return invoke("copy_template_catalog_entry",{templateId,name});}
export async function getSceneSnapshot():Promise<SceneSnapshot|null>{return invoke("get_scene_snapshot");}
export async function selectCurrentTemplate(templateId:string|null):Promise<SceneSnapshot>{return invoke("select_current_template", { templateId });}
export async function subscribeSceneEvents(callback:(event:SceneEvent)=>void):Promise<()=>void>{const unlisten=await listen<SceneEvent>("scene-event",event=>callback(event.payload));return unlisten;}
export async function getComponentStatus(component: ComponentKind): Promise<ComponentStatus> {
  if (!isTauriRuntime()) throw new Error("当前是浏览器预览，无法检测组件状态");
  return invoke<ComponentStatus>("get_component_status", { component });
}
export async function performComponentAction(component: ComponentKind, action: ComponentAction): Promise<ComponentOperation> {
  if (!isTauriRuntime()) throw new Error("当前是浏览器预览，无法执行组件操作");
  return invoke<ComponentOperation>("perform_component_action", { component, action });
}

export async function setMenuTemplateSwitchEnabled(enabled: boolean): Promise<MappingConfiguration> {
  return invoke("set_menu_template_switch_enabled", { enabled });
}

export type Rc003CaptureHealth = "off" | "starting" | "active" | "inactive";
export type Rc003CaptureUnavailable = "none" | "startup_failing";
export type Rc003CaptureRevoked = "none" | "task_missing" | "reauth_marker_present" | "task_target_mismatch" | "task_power_policy_mismatch" | "unsupported";

export interface Rc003BridgeSnapshot {
  captureHealth: Rc003CaptureHealth;
  captureUnavailable: Rc003CaptureUnavailable;
  captureRevoked: Rc003CaptureRevoked;
  phase: Rc003BridgePhase;
  port: number;
  helperPid: number;
  acceptedTotal: number;
  deniedTotal: number;
  replacedTotal: number;
  edgesApplied: number;
  usagesDropped: number;
  malformedTotal: number;
  watchdogReleaseTotal: number;
  pressedUsages: number[];
  lastRxAgeMs: number | null;
  targetGeneration: number;
  targetUsages: number[];
  ownedUsages: number[];
}

/** `stopped` = 该平台没有这个机制（非 Windows），或桥接未启用。 */
export type Rc003BridgePhase = "stopped" | "listening" | "connected" | "failed";

/**
 * 向导第⑥步：按键映射临时暂挂（只观察、不注入；内存态、不改用户配置）。
 * 进入「普通按键体验」时暂挂，离开时恢复；进程退出后自然复位。
 */
export async function setMappingSuspension(suspended: boolean): Promise<boolean> {
  if (!isTauriRuntime()) {
    return suspended;
  }
  return invoke<boolean>("set_mapping_suspension", { suspended });
}

/**
 * 向导第⑤步前置（探针④）：打开物理键观察窗口。
 * 返回窗口 id；0 = 当前环境不可用（仿真/钩子未运行），调用方按未知处理。
 * excludeVks：不计入的虚拟键码——「按住说话」和弦由报告层以非注入形态
 * 送进 OS（必须到达输入法），不属于手动输入。
 */
export async function beginKeyObservation(excludeVks: number[]): Promise<number> {
  if (!isTauriRuntime()) {
    return 0;
  }
  return invoke<number>("begin_key_observation", { excludeVks });
}

/**
 * 向导第⑤步前置（探针④）：关闭物理键观察窗口并取回计数。
 * 计数 = 窗口内「可能进入 OS 的物理按下沿」（非注入、未被门控吞下、不在
 * 排除集内）。返回 null = 计量不可靠（窗口过期/钩子停止）——fail-open，
 * 不得据此判定手动输入。
 */
export async function endKeyObservation(windowId: number): Promise<number | null> {
  if (!isTauriRuntime()) {
    return null;
  }
  return invoke<number | null>("end_key_observation", { windowId });
}

/** 浏览器 / 仿真环境没有这个机制：恒为 "不存在"，前端据此不显示这一条目。 */
const BROWSER_RC003_BRIDGE: Rc003BridgeSnapshot = {
  captureHealth: "off",
  captureUnavailable: "none",
  captureRevoked: "none",
  phase: "stopped",
  port: 0,
  helperPid: 0,
  acceptedTotal: 0,
  deniedTotal: 0,
  replacedTotal: 0,
  edgesApplied: 0,
  usagesDropped: 0,
  malformedTotal: 0,
  watchdogReleaseTotal: 0,
  pressedUsages: [],
  lastRxAgeMs: null,
  targetGeneration: 0,
  targetUsages: [],
  ownedUsages: [],
};

export async function getRc003BridgeSnapshot(): Promise<Rc003BridgeSnapshot> {
  // `typeof window` 这一层不能省：组件卸载后轮询仍可能再触发一次，
  // 而测试环境在 teardown 之后 window 已不可用 —— 直接调 `isTauriRuntime()`
  // 会抛 ReferenceError，表现为一个与被测功能无关的 unhandled rejection。
  if (typeof window === "undefined" || !isTauriRuntime()) {
    return BROWSER_RC003_BRIDGE;
  }
  return invoke<Rc003BridgeSnapshot>("get_rc003_bridge_snapshot");
}

/** Persisted intent is separate from the installed task and live bridge. */
export interface Rc003TaskStatus {
  installed: boolean;
  /** The previous capture session has not yet confirmed clean shutdown. */
  cleanupPending: boolean;
  canRetryCleanup: boolean;
  /**
   * 这次打开开关会触发系统授权（UAC）。2026-10-03 起恒为 true：每次开启都会
   * 重新注册任务并弹一次 Windows 授权窗口。字段保留给诊断与类型兼容，
   * 前端弹窗判据已不依赖它（每次开启都弹确认，只有关闭方向直接执行）。
   */
  authorizationRequired: boolean;
  /** 用户意图（持久化，默认关闭）。开关显示读它，而不是读 installed。 */
  enabled: boolean;
  helperPath: string | null;
  lastError: string | null;
}

/**
 * 开关打开：**每次都重新授权**（弹一次 UAC 重新注册任务，主程序会等它完成），
 * 然后触发助手；开关关闭：结束助手，任务留在系统里但授权视为作废
 * （提权任务普通权限删不掉），下次开启必弹 UAC（2026-10-03 Andy 定稿）。
 */
export async function getRc003TaskStatus(): Promise<Rc003TaskStatus> {
  if (typeof window === "undefined" || !isTauriRuntime()) {
    return { installed: false, authorizationRequired: true, enabled: false, cleanupPending: false, canRetryCleanup: false, helperPath: null, lastError: null };
  }
  return invoke<Rc003TaskStatus>("get_rc003_task_status");
}

export async function enableRc003Capture(): Promise<Rc003TaskStatus> {
  if (typeof window === "undefined" || !isTauriRuntime()) {
    return { installed: false, authorizationRequired: true, enabled: false, cleanupPending: false, canRetryCleanup: false, helperPath: null, lastError: null };
  }
  return invoke<Rc003TaskStatus>("enable_rc003_capture");
}

export async function disableRc003Capture(): Promise<Rc003TaskStatus> {
  if (typeof window === "undefined" || !isTauriRuntime()) {
    return { installed: false, authorizationRequired: true, enabled: false, cleanupPending: false, canRetryCleanup: false, helperPath: null, lastError: null };
  }
  return invoke<Rc003TaskStatus>("disable_rc003_capture");
}

/** 本机无法使用「全按键支持」的原因（`available` 为 false 时一定有值）。 */
export type CaptureUnsupportedReason = "arch_unsupported" | "helper_missing";

/**
 * 「全按键支持」在这台电脑上是否可用（2026-10-07 issue #206）。
 *
 * 背景：Windows 11 ARM64 上增强捕获起不来（产品只提供 x64 载荷，ARM64 上加载
 * 不了），后端因此在启动时把已持久化的开启意图回落为关闭、并停止重试空转；
 * 界面据此把开关与三键卡片置灰、说明原因与恢复方式，不再让页面永久停在
 * 「正在启动」。`available=false` 时桥接相位是 `stopped`（状态点不显示）。
 */
export interface CaptureSupport {
  /** 本机原生架构。 */
  nativeArch: "x64" | "arm64" | "unknown";
  /** 本机架构对应的载荷文件名（诊断信息，不进用户可见文案）。 */
  helperExpected: string;
  /** false = 本机无法使用「全按键支持」：界面必须置灰入口并说明原因。 */
  available: boolean;
  reason: CaptureUnsupportedReason | null;
}

/** 浏览器预览没有真实平台：按「可用」渲染，保证界面能完整走查。 */
const BROWSER_CAPTURE_SUPPORT: CaptureSupport = {
  nativeArch: "unknown",
  helperExpected: "sayall-helper.exe",
  available: true,
  reason: null,
};

/**
 * 读取本机的支持情况（挂载时一次 + 随页面既有轮询刷新）。
 *
 * 与 `getRc003TaskStatus` 同口径：非 Tauri 环境返回浏览器预览值，Tauri 环境
 * IPC 失败时抛错，由调用方 `.catch` 后保持上一次的值（读不到就不置灰，
 * 不因为一次 IPC 失败把功能锁死）。
 */
export async function getCaptureSupport(): Promise<CaptureSupport> {
  if (typeof window === "undefined" || !isTauriRuntime()) {
    return BROWSER_CAPTURE_SUPPORT;
  }
  return invoke<CaptureSupport>("get_capture_support");
}



export async function saveButtonMappings(mappings: ButtonMappings): Promise<ButtonMappings> {
  if (!isTauriRuntime()) {
    throw new Error("当前是浏览器预览，无法保存按键映射");
  }
  return invoke<ButtonMappings>("save_button_mappings", { mappings });
}

export async function resetButtonMappings(): Promise<ButtonMappings> {
  if (!isTauriRuntime()) {
    return { enabled: true, actions: {} };
  }
  return invoke<ButtonMappings>("reset_button_mappings");
}


/**
 * 键位方案访问状态（私有组件给出；公开构建恒为 unavailable）。
 *
 * 公开桥只保留稳定形状，不解析会员 token、不读取方案文件。
 */
export type ButtonProfilesAccess =
  | { status: "unavailable" }
  | { status: "requiresPlus" }
  | { status: "allowed"; validUntil?: string | null };

export async function getButtonProfilesAccess(): Promise<ButtonProfilesAccess> {
  if (!isTauriRuntime()) {
    return { status: "unavailable" };
  }
  return invoke<ButtonProfilesAccess>("button_profiles_access");
}

/** 键位方案页面快照（私有 JSON）；公开构建返回 null。 */
export async function getButtonProfilesSnapshot(): Promise<string | null> {
  if (!isTauriRuntime()) {
    return null;
  }
  return invoke<string | null>("button_profiles_snapshot");
}

/** 选择 `.sayall` 导入文件；取消返回 null。 */
export async function pickButtonProfilesImportFile(): Promise<string | null> {
  if (!isTauriRuntime()) {
    return null;
  }
  return invoke<string | null>("button_profiles_pick_import_file");
}

/** 选择键位方案备份的导出位置；取消返回 null。 */
export async function pickButtonProfilesExportFile(): Promise<string | null> {
  if (!isTauriRuntime()) {
    return null;
  }
  return invoke<string | null>("button_profiles_pick_export_file");
}

/** 键位方案页面命令（命令名与 payload 由私有组件定义）。 */
export async function dispatchButtonProfilesCommand(
  command: string,
  payload?: string,
): Promise<string> {
  if (!isTauriRuntime()) {
    throw new Error("当前是浏览器预览，无法执行键位方案命令");
  }
  return invoke<string>("button_profiles_dispatch", { command, payload });
}

export async function testButtonMapping(
  button: RemoteButton,
  trigger: ButtonTrigger,
): Promise<SendInputSnapshot> {
  if (!isTauriRuntime()) {
    throw new Error("当前是浏览器预览，无法执行按键测试");
  }
  return invoke<SendInputSnapshot>("test_button_mapping", { button, trigger });
}

export async function listPresetApps(): Promise<PresetAppInfo[]> {
  if (!isTauriRuntime()) {
    // 浏览器预览：展示完整预设表（仅渲染验证）。
    return [
      { id: "sayall", name: "无线麦", installed: true },
      { id: "codex", name: "Codex", installed: true },
      { id: "wechat", name: "微信", installed: true },
      { id: "netease_music", name: "网易云音乐", installed: true },
      { id: "chrome", name: "Chrome 浏览器", installed: true },
      { id: "chatgpt", name: "ChatGPT", installed: true },
      { id: "workbuddy", name: "WorkBuddy", installed: true },
      { id: "dimagent", name: "DimAgent", installed: true },
      { id: "vokie", name: "Vokie", installed: true },
      { id: "notepad", name: "记事本", installed: true },
      { id: "qq", name: "QQ", installed: true },
    ];
  }
  return invoke<PresetAppInfo[]>("list_preset_apps");
}

export async function listRunningApps(): Promise<RunningAppInfo[]> {
  if (!isTauriRuntime()) return [
    { applicationId: "edge", name: "Edge 浏览器", preset: true },
    { applicationId: "c:\\tools\\reader.exe", name: "Reader", preset: false },
  ];
  return invoke<RunningAppInfo[]>("list_running_apps");
}

/**
 * 「学习输入框」第一步：打开或切换到目标应用，不改变它的输入焦点。
 *
 * 对齐 Mac `recordCustomApplicationInput`：先打开目标 App，再进入学习窗口。
 */
export async function activateAppForLearning(target: string): Promise<void> {
  if (!isTauriRuntime()) {
    throw new Error("当前是浏览器预览，无法打开应用");
  }
  return invoke<void>("activate_app_for_learning", { target });
}

/**
 * 「学习输入框」：最多 6 秒内轮询系统焦点，返回捕获到的可编辑目标特征。
 *
 * 调用方应先通过 [`activateAppForLearning`] 打开目标应用；本命令不会再切换窗口。
 */
export async function learnFocusTarget(): Promise<RecordedFocusTarget> {
  if (!isTauriRuntime()) {
    throw new Error("当前是浏览器预览，无法学习输入框");
  }
  return invoke<RecordedFocusTarget>("learn_focus_target");
}

/**
 * 「记录点击位置」：把当前鼠标所在的相对位置记到目标应用上。
 *
 * 只读取光标坐标与目标窗口矩形，不读任何内容；光标不在目标窗口内时抛
 * `cursor_outside_target`（见 `focusPointErrorLabel`）。
 */
export async function learnFocusPoint(target: string): Promise<RecordedPoint> {
  if (!isTauriRuntime()) {
    throw new Error("当前是浏览器预览，无法记录点击位置");
  }
  return invoke<RecordedPoint>("learn_focus_point", { target });
}

/** 「测试打开与聚焦」：按目标与聚焦档案走一次生产路径（异步受理）。 */
export async function testAppFocus(target: string): Promise<void> {
  if (!isTauriRuntime()) {
    throw new Error("当前是浏览器预览，无法测试打开与聚焦");
  }
  return invoke<void>("test_app_focus", { target });
}

export async function getButtonMappingSnapshot(): Promise<ButtonMappingSnapshot> {
  if (!isTauriRuntime()) {
    return {
      enabled: true,
      gateActive: false,
      observedButtons: [],
      listenerActive: false,
      swallowedEdges: 0,
      leakedDowns: 0,
      firedGestures: 0,
      lastFired: null,
      lastError: null,
    };
  }
  return invoke<ButtonMappingSnapshot>("get_button_mapping_snapshot");
}

/** 订阅语义按键边沿（画布高亮数据源）；浏览器预览下为空订阅。 */
export async function subscribeButtonEdges(
  handler: (edge: ButtonEdge) => void,
): Promise<() => void> {
  if (!isTauriRuntime()) {
    return () => {};
  }
  const { listen } = await import("@tauri-apps/api/event");
  const unlisten = await listen<ButtonEdge>("button-edge", (event) => handler(event.payload));
  return () => {
    void unlisten();
  };
}

/** 订阅已触发手势（单击/双击/长按反馈）；浏览器预览下为空订阅。 */
export async function subscribeButtonGestures(
  handler: (gesture: FiredGesture) => void,
): Promise<() => void> {
  if (!isTauriRuntime()) {
    return () => {};
  }
  const { listen } = await import("@tauri-apps/api/event");
  const unlisten = await listen<FiredGesture>("button-gesture", (event) => handler(event.payload));
  return () => {
    void unlisten();
  };
}

/** 开始 OS 级快捷键录入；返回录入开始时仍被按住的键（preheld）。
 *  preheld 键的边沿对录入不可见（防粘键：其 DOWN 已进 OS，UP 必须放行），
 *  后端会等它们全部松开后才开始投递边沿——前端据此提示用户先松手，
 *  避免"按住中打开录入"被静默截断成半截组合。 */
export async function startShortcutCapture(): Promise<KeyCode[]> {
  if (!isTauriRuntime()) return [];
  const preheld = await invoke<KeyCode[]>("start_shortcut_capture");
  return preheld ?? [];
}

/** 微信输入法语音是否在录入会话期间被触发（观测其麦克风 ConsentStore）。 */
export type WetypeVoiceVerdict = "observed" | "not_observed" | "unknown";

export interface ShortcutCaptureStopResult {
  /** "unknown" 表示观测不可用，调用方不得据此推断用户按了什么。 */
  wetypeVoice: WetypeVoiceVerdict;
}

export async function stopShortcutCapture(): Promise<ShortcutCaptureStopResult | null> {
  if (!isTauriRuntime()) return null;
  const result = await invoke<ShortcutCaptureStopResult | null>("stop_shortcut_capture");
  return result ?? null;
}

/** 原生低级钩子录入边沿；Win+L 等系统组合在到达 Shell 前已成对吞下。 */
export async function subscribeShortcutCaptureEdges(
  handler: (edge: ShortcutCaptureEdge) => void,
): Promise<() => void> {
  if (!isTauriRuntime()) return () => {};
  const { listen } = await import("@tauri-apps/api/event");
  const unlisten = await listen<ShortcutCaptureEdge>("shortcut-capture-edge", (event) =>
    handler(event.payload),
  );
  return () => {
    void unlisten();
  };
}

export async function getSendInputSnapshot(): Promise<SendInputSnapshot> {
  if (!isTauriRuntime()) {
    return { available: false, submittedBatches: 0, submittedEvents: 0, lastError: null };
  }
  return invoke<SendInputSnapshot>("get_send_input_snapshot");
}

export async function getVoiceHoldHotkey(): Promise<KeyChord | null> {
  if (!isTauriRuntime()) {
    return null;
  }
  return invoke<KeyChord | null>("get_voice_hold_hotkey");
}

export async function setVoiceHoldHotkey(hotkey: KeyChord | null): Promise<KeyChord | null> {
  if (!isTauriRuntime()) {
    throw new Error("当前是浏览器预览，无法保存按住说话快捷键");
  }
  return invoke<KeyChord | null>("set_voice_hold_hotkey", { hotkey });
}

/**
 * 连接页选择的输入工具（2026-09-30 设计稿 v3）。
 *
 * 它决定连接页展示的快捷键建议与准备清单；基础语音仍使用成对的
 * 按住说话快捷键，不依赖可选按键增强 Helper。
 * `null` = 用户从未选择过：界面仅按当前快捷键展示引导，显式选择后才落存。
 */
export type VoiceInputTool = "wechat" | "doubao" | "vokie" | "other";

export async function getVoiceInputTool(): Promise<VoiceInputTool | null> {
  if (!isTauriRuntime()) {
    return null;
  }
  return invoke<VoiceInputTool | null>("get_voice_input_tool");
}

export async function setVoiceInputTool(tool: VoiceInputTool): Promise<VoiceInputTool> {
  if (!isTauriRuntime()) {
    throw new Error("当前是浏览器预览，无法保存输入工具设置");
  }
  const saved = await invoke<VoiceInputTool | null>("set_voice_input_tool", { tool });
  return saved ?? tool;
}


/**
 * 输入法状态（2026-10-13 现场）：诊断"语音没出字是不是因为没切输入法"。
 *
 * - `detected=false`：读不到输入法状态——界面**不提示**（fail-open，宁可不说也不能瞎说）；
 * - `expected=null`：当前的输入工具不需要切输入法（如 Vokie / 其他工具）；
 * - `matchesExpected`：`null` = 判定不出来，同样是"不提示"。
 */
export interface ImeStatus {
  detected: boolean;
  active: string | null;
  installed: string[];
  expected: string | null;
  matchesExpected: boolean | null;
}


/**
 * **只读**查询输入法状态（不切输入法）。复制诊断信息时用它，避免"复制一下顺手改了
 * 用户输入法"。浏览器预览下返回"读不到"。
 */
export async function getImeStatus(): Promise<ImeStatus> {
  if (!isTauriRuntime()) {
    return { detected: false, active: null, installed: [], expected: null, matchesExpected: null };
  }
  return invoke<ImeStatus>("get_ime_status");
}

/** 检查应用更新；浏览器预览下返回"无更新"占位（不发起网络请求）。 */
export async function checkAppUpdate(): Promise<AppUpdateInfo> {
  if (!isTauriRuntime()) {
    return {
      currentVersion: browserSnapshot.appVersion,
      available: false,
      version: null,
      notes: null,
      date: null,
    };
  }
  return invoke<AppUpdateInfo>("check_app_update");
}

export async function getAppUpdatePreferences(): Promise<AppUpdatePreferences> {
  if (!isTauriRuntime()) {
    return { includePrereleases: false };
  }
  return invoke<AppUpdatePreferences>("get_app_update_preferences");
}

export async function setAppUpdatePreferences(
  includePrereleases: boolean,
): Promise<AppUpdatePreferences> {
  if (!isTauriRuntime()) {
    return { includePrereleases };
  }
  return invoke<AppUpdatePreferences>("set_app_update_preferences", { includePrereleases });
}

export async function getThemePreference(operationId: string): Promise<ThemePreference> {
  if (!isTauriRuntime()) {
    return "system";
  }
  return invoke<ThemePreference>("get_theme_preference", { operationId });
}

export async function saveThemePreference(
  preference: ThemePreference,
  operationId: string,
): Promise<ThemePreference> {
  if (!isTauriRuntime()) {
    return preference;
  }
  return invoke<ThemePreference>("set_theme_preference", { preference, operationId });
}

export async function getApplicationLanguage(
  operationId: string,
): Promise<AppLanguagePreference> {
  if (!isTauriRuntime()) {
    return "system";
  }
  return invoke<AppLanguagePreference>("get_application_language", { operationId });
}

export async function saveApplicationLanguage(
  language: AppLanguagePreference,
  operationId: string,
): Promise<AppLanguagePreference> {
  if (!isTauriRuntime()) {
    return language;
  }
  return invoke<AppLanguagePreference>("set_application_language", {
    language,
    operationId,
  });
}

export interface ThemeResultReport {
  operationId: string;
  action: "initialize" | "change";
  preference: ThemePreference;
  resolvedTheme: "light" | "dark";
  terminalResult: "passed" | "failed";
  reason:
    | "applied"
    | "preference_load_failed"
    | "native_apply_failed"
    | "apply_or_save_failed";
  elapsedMs: number;
}

export async function reportThemeResult(report: ThemeResultReport): Promise<void> {
  if (!isTauriRuntime()) return;
  await invoke("report_theme_result", { report });
}

/** 下载并安装已检查到的更新（Windows 上安装成功时应用会退出并由安装器重启）。 */
export async function installAppUpdate(): Promise<void> {
  if (!isTauriRuntime()) {
    throw new Error("当前是浏览器预览，无法安装更新");
  }
  await invoke("install_app_update");
}

/** 订阅更新下载进度；浏览器预览下为空订阅。 */
export async function subscribeAppUpdateProgress(
  handler: (progress: AppUpdateProgress) => void,
): Promise<() => void> {
  if (!isTauriRuntime()) {
    return () => {};
  }
  const { listen } = await import("@tauri-apps/api/event");
  const unlisten = await listen<AppUpdateProgress>("app-update-progress", (event) =>
    handler(event.payload),
  );
  return () => {
    void unlisten();
  };
}

/** 纯英文/符号的键名不查字典，避免每次渲染都产生无意义缺失告警。 */
function localized(source: string): string {
  return /[\u4e00-\u9fff]/.test(source) ? t(source) : source;
}

const voiceHotkeyKeyLabels: Record<string, string> = {
  control: "Ctrl",
  left_control: "左 Ctrl",
  right_control: "右 Ctrl",
  shift: "Shift",
  left_shift: "左 Shift",
  right_shift: "右 Shift",
  alt: "Alt",
  left_alt: "左 Alt",
  right_alt: "右 Alt",
  left_windows: "左 Win",
  right_windows: "右 Win",
  enter: "Enter",
  escape: "Esc",
  space: "空格",
  tab: "Tab",
  apps: "右键菜单",
  oem1: ";",
  oem_plus: "=",
  oem_comma: ",",
  oem_minus: "-",
  oem_period: ".",
  oem2: "/",
  oem3: "`",
  oem4: "[",
  oem5: "\\",
  oem6: "]",
  oem7: "'",
  oem102: "\\",
  numpad0: "小键盘 0",
  numpad1: "小键盘 1",
  numpad2: "小键盘 2",
  numpad3: "小键盘 3",
  numpad4: "小键盘 4",
  numpad5: "小键盘 5",
  numpad6: "小键盘 6",
  numpad7: "小键盘 7",
  numpad8: "小键盘 8",
  numpad9: "小键盘 9",
  numpad_multiply: "小键盘 ×",
  numpad_add: "小键盘 +",
  numpad_subtract: "小键盘 −",
  numpad_decimal: "小键盘 .",
  numpad_divide: "小键盘 ÷",
  numlock: "Num Lock",
};

function voiceHotkeyKeyLabel(code: string): string {
  const known = voiceHotkeyKeyLabels[code];
  if (known) return localized(known);
  const digit = /^digit([0-9])$/.exec(code);
  if (digit) return digit[1];
  return code.toUpperCase();
}

export function voiceHoldHotkeyLabel(hotkey: KeyChord | null): string {
  // "关闭" 不能进总字典：同一个中文原文在弹窗按钮里是 "Close"，全局按原文
  // 查表会产生歧义；这里按调用时语言直接给出快捷键关闭态。
  if (!hotkey || hotkey.keys.length === 0) {
    return effectiveLanguage.value === "en" ? "Off" : "关闭";
  }
  return hotkey.keys.map(voiceHotkeyKeyLabel).join(" + ");
}

export function connectionPhaseLabel(phase: ConnectionPhase): string {
  return t(
    {
      idle: "尚未连接",
      connecting: "正在连接遥控器",
      discovering: "正在连接遥控器",
      awaiting_capabilities: "正在确认语音功能",
      ready: "已连接",
      streaming: "正在接收语音",
      draining: "正在结束本次语音",
      reconnecting: "正在等待遥控器重连",
      suspended: "电脑已进入睡眠",
      disconnected: "遥控器已断开",
      failed: "连接失败",
    }[phase],
  );
}

export function remoteModelLabel(model: RemoteModel): string {
  return t(
    {
      rc001: "小米蓝牙语音遥控器 2",
      rc003: "小米蓝牙语音遥控器 2 Pro",
      unknown: "连接后显示",
    }[model],
  );
}

export function audioPhaseLabel(phase: AudioPhase): string {
  return t(
    {
      unconfigured: "尚未选择设备",
      ready: "已就绪",
      streaming: "正在写入语音",
      draining: "正在结束",
      failed: "语音设备出错",
      unsupported: "当前环境不支持语音设备",
    }[phase],
  );
}

export const buttonLabels: Record<RemoteButton, string> = {
  get back() {
    return t("返回");
  },
  get ok() {
    return t("确定");
  },
  get tv() {
    return "TV";
  },
  get home() {
    return t("主页");
  },
  get right() {
    return t("右");
  },
  get left() {
    return t("左");
  },
  get down() {
    return t("下");
  },
  get up() {
    return t("上");
  },
  get menu() {
    return t("菜单");
  },
  get power() {
    return t("电源");
  },
  get volume_mute() {
    return t("静音");
  },
  get volume_up() {
    return t("音量+");
  },
  get volume_down() {
    return t("音量−");
  },
};

export function buttonLabel(button: RemoteButton): string {
  return buttonLabels[button];
}

export function buttonTriggerLabel(trigger: ButtonTrigger): string {
  return t(
    {
      single: "单击",
      double: "双击",
      long: "长按",
    }[trigger],
  );
}

const keyLabels: Record<string, string> = {
  ...voiceHotkeyKeyLabels,
  // 组合动作用 `return` 作为 Enter 的稳定存储名；界面统一显示键帽上的 "Enter"。
  return: "Enter",
  // 用厂商印在键帽上的英文名，避免“退格/删除”在中文里被混为一谈。
  backspace: "Backspace",
  home: "Home",
  page_up: "Page Up",
  page_down: "Page Down",
  end: "End",
  insert: "Insert",
  delete: "Delete",
  left: "←",
  up: "↑",
  right: "→",
  down: "↓",
  volume_mute: "静音",
  volume_down: "音量−",
  volume_up: "音量+",
  media_play_pause: "播放/暂停",
  media_prev: "上一首",
  media_next: "下一首",
  f1: "F1",
  f2: "F2",
  f3: "F3",
  f4: "F4",
  f5: "F5",
  f6: "F6",
  f7: "F7",
  f8: "F8",
  f9: "F9",
  f10: "F10",
  f11: "F11",
  f12: "F12",
};

export function keyLabel(code: KeyCode): string {
  const known = keyLabels[code];
  if (known) return localized(known);
  const digit = /^digit([0-9])$/.exec(code);
  if (digit) return digit[1];
  return code.toUpperCase();
}

export function chordLabel(chord: KeyChord): string {
  return chord.keys.map(keyLabel).join(" + ");
}

/** 预设应用显示名（页面加载 listPresetApps 后更新；测试可注入）。 */
const presetAppNames: Map<string, string> = new Map();

export function registerPresetAppNames(apps: Array<{ id: string; name: string }>): void {
  presetAppNames.clear();
  for (const app of apps) {
    presetAppNames.set(app.id, app.name);
  }
}

/** 组合动作名（页面加载库后更新；`actionSummary` 显示引用名）。 */
const macroNames: Map<string, string> = new Map();

export function registerMacroNames(definitions: Array<{ macroId: string; name: string }>): void {
  macroNames.clear();
  for (const definition of definitions) {
    macroNames.set(definition.macroId, definition.name);
  }
}

export function macroNameById(macroId: string): string | null {
  return macroNames.get(macroId) ?? null;
}

/**
 * 同一条组合动作只保留最新版本（面向用户的是「动作」而不是「版本」）。
 *
 * 库文件按不可变版本累积（编辑一次 = 新版本追加，绑定引用按最新版解析）；
 * 列表 / 选择器必须先用本函数去重，否则编辑一次就会多出一行
 * （2026-10-15 现场：1.0.0 与 1.0.1 同时显示成两个动作）。
 * 顺序 = 首次出现的顺序（列表稳定，编辑后不跳位）。
 */
export function latestMacroDefinitions(definitions: MacroDefinition[]): MacroDefinition[] {
  const parse = (version: string): [number, number, number] | null => {
    const match = /^(\d+)\.(\d+)\.(\d+)$/.exec(version.trim());
    return match ? [Number(match[1]), Number(match[2]), Number(match[3])] : null;
  };
  const greater = (candidate: string, current: string): boolean => {
    const left = parse(candidate);
    const right = parse(current);
    if (left && right) {
      for (let index = 0; index < 3; index += 1) {
        if (left[index] !== right[index]) return left[index] > right[index];
      }
      return false;
    }
    return candidate > current;
  };
  const latest: MacroDefinition[] = [];
  for (const definition of definitions) {
    const index = latest.findIndex((item) => item.macroId === definition.macroId);
    if (index === -1) {
      latest.push(definition);
    } else if (greater(definition.version, latest[index].version)) {
      latest[index] = definition;
    }
  }
  return latest;
}

/** 常用语组件是否可用（按键页加载快照后注册；测试可注入）。 */
let commonPhrasesAvailable = false;

/**
 * 注册常用语组件可用性（按键页加载 `getCommonPhrasePanelSnapshot()` 后调用）。
 *
 * 为什么用模块级缓存而不是每次问 IPC：`actionSummary` 是同步的渲染路径函数，
 * 不能等待异步快照——与预设应用名的处理方式一致。
 */
export function registerCommonPhraseAvailability(available: boolean): void {
  commonPhrasesAvailable = available;
}

/**
 * 「打开应用」的显示名：动作在前、应用名在后（对齐 Mac `action.open_*`）。
 *
 * 中文名直接相连（“打开微信”）；以拉丁字母或数字开头的名字按写法约定补一个
 * 空格（“打开 WorkBuddy”），避免中英文粘连。
 */
export function openAppLabel(name: string): string {
  const trimmed = name.trim();
  if (!trimmed) return t("打开应用");
  return /^[0-9A-Za-z]/.test(trimmed)
    ? t("打开 {name}", { name: trimmed })
    : t("打开{name}", { name: trimmed });
}

export function actionSummary(action: ButtonAction | undefined): string {
  if (!action || action.type === "disabled") return t("未设置");
  if (action.type === "task_switch") return action.view === "applications" ? "任务切换" : "任务视图";
  if (action.type === "focus_input") return t("聚焦输入框");
  if (action.type === "open_common_phrases") {
    // 缺私有组件（公开构建）时如实标注，不静默替换成含义不同的动作。
    return commonPhrasesAvailable ? t("打开常用语") : t("打开常用语（当前不可用）");
  }
  if (action.type === "scroll") {
    const label = action.direction === "up" ? t("滚轮向上") : t("滚轮向下");
    const steps = action.steps ?? 1;
    return steps === 1 ? label : t("{label} {steps} 格", { label, steps });
  }
  if (action.type === "mouse_click") return mouseClickLabels[action.kind];
  if (action.type === "mouse_move") return `${mouseMoveLabels[action.direction]} ${action.distance} px`;
  if (action.type === "open_app") {
    const known = presetAppNames.get(action.target);
    if (known) return openAppLabel(known);
    // 自定义应用：target 为路径，取文件名去扩展名作展示名。
    const base = action.target.split(/[\\/]/).pop() ?? action.target;
    const stem = base.replace(/\.(exe|lnk)$/i, "");
    return openAppLabel(stem || action.target);
  }
  if (action.type === "run_macro") {
    const known = macroNameById(action.macroId);
    return known ? t("执行{name}", { name: known }) : t("执行组合动作");
  }
  return chordLabel(action.chord);
}

/** 自定义应用选择结果（pick_custom_app 命令返回）。 */
export interface CustomAppPick {
  name: string;
  path: string;
  applicationId: string;
}

export type AppLibraryEntry = Pick<CustomAppPick, "name" | "path">;

export async function scanRegisteredApps(): Promise<AppLibraryEntry[]> {
  if (!isTauriRuntime()) throw new Error("应用扫描需要在 Windows 客户端中使用");
  return invoke<AppLibraryEntry[]>("scan_registered_apps");
}

/**
 * 打开原生文件选择器选择自定义应用（.exe/.lnk）。
 * 用户取消或浏览器预览环境返回 null。
 */
export async function pickCustomApp(): Promise<CustomAppPick | null> {
  if (!isTauriRuntime()) {
    return null;
  }
  try {
    return await invoke<CustomAppPick | null>("pick_custom_app");
  } catch {
    return null;
  }
}

// ===== 组合动作 IPC =====

/** 浏览器预览用的演示库（只服务界面预览与快照断言，不进入生产路径）。 */
const previewMacroLibrary: MacroLibraryDocument = {
  formatVersion: 1,
  definitions: [
    {
      schemaVersion: "0.1",
      macroId: "macro.preview01",
      version: "1.0.0",
      name: "打开记事本并聚焦",
      steps: [
        {
          stepId: "step.01",
          action: "open_application",
          parameters: { target: "notepad" },
        },
        {
          stepId: "step.02",
          action: "focus_input",
          parameters: { target: "notepad" },
        },
      ],
    },
  ],
};

export async function getCombinedActions(): Promise<MacroLibraryDocument> {
  if (!isTauriRuntime()) {
    return structuredClone(previewMacroLibrary);
  }
  return invoke<MacroLibraryDocument>("get_combined_actions");
}

export async function saveMacro(definition: MacroDefinition): Promise<MacroLibraryDocument> {
  if (!isTauriRuntime()) throw new Error("当前是浏览器预览，无法保存组合动作");
  return invoke<MacroLibraryDocument>("save_macro", { definition });
}

export interface MacroDeleteResult {
  library: MacroLibraryDocument;
  removedMacroIds: string[];
  rewrittenMacroIds: string[];
}

export async function deleteMacro(macroId: string): Promise<MacroDeleteResult> {
  if (!isTauriRuntime()) throw new Error("当前是浏览器预览，无法删除组合动作");
  return invoke<MacroDeleteResult>("delete_macro", { macroId });
}

/** 删除前的引用清单（按键 + 嵌套引用）。 */
export interface MacroButtonReference {
  button: string;
  trigger: string;
}

export interface MacroReferenceReport {
  buttons: MacroButtonReference[];
  nestedIn: string[];
}

/** 查询引用该组合动作的按键与组合动作（删除确认用）。 */
export async function macroReferences(macroId: string): Promise<MacroReferenceReport> {
  if (!isTauriRuntime()) {
    return { buttons: [], nestedIn: [] };
  }
  return invoke<MacroReferenceReport>("macro_references", { macroId });
}

/** 导出组合动作库；用户取消返回 false。 */
export async function exportCombinedActions(): Promise<boolean> {
  if (!isTauriRuntime()) throw new Error("当前是浏览器预览，无法导出组合动作");
  return invoke<boolean>("export_combined_actions");
}

/** 导入组合动作（合并入库）；用户取消返回 null。 */
export async function importCombinedActions(): Promise<MacroLibraryDocument | null> {
  if (!isTauriRuntime()) throw new Error("当前是浏览器预览，无法导入组合动作");
  return invoke<MacroLibraryDocument | null>("import_combined_actions");
}

/** 测试运行已保存的组合动作（按最新版本；异步受理）。 */
export async function testMacro(macroId: string): Promise<void> {
  if (!isTauriRuntime()) throw new Error("当前是浏览器预览，无法测试组合动作");
  return invoke<void>("test_macro", { macroId });
}

/** 测试运行未保存的草稿定义；返回本次运行 ID（用于轮询状态）。 */
export async function testMacroDefinition(definition: MacroDefinition): Promise<string> {
  if (!isTauriRuntime()) throw new Error("当前是浏览器预览，无法测试组合动作");
  return invoke<string>("test_macro_definition", { definition });
}

/** 测试运行草稿里的单个步骤；返回本次运行 ID。 */
export async function testMacroStep(
  definition: MacroDefinition,
  stepId: string,
): Promise<string> {
  if (!isTauriRuntime()) throw new Error("当前是浏览器预览，无法测试步骤");
  return invoke<string>("test_macro_step", { definition, stepId });
}

export async function cancelMacro(macroId: string): Promise<boolean> {
  if (!isTauriRuntime()) throw new Error("当前是浏览器预览，无法取消组合动作");
  return invoke<boolean>("cancel_macro", { macroId });
}

export async function getMacroRunStatus(macroId: string): Promise<MacroRunStatus> {
  if (!isTauriRuntime()) {
    return { running: false, report: null };
  }
  return invoke<MacroRunStatus>("get_macro_run_status", { macroId });
}

/** 组合动作步骤类型的中文名（对齐 Mac `macro.step.*` 文案）。 */
export const macroActionLabels: Record<MacroAction, string> = {
  get open_application() {
    return t("打开应用");
  },
  get wait_for_application() {
    return t("等待应用");
  },
  get send_shortcut() {
    return t("单键或快捷键");
  },
  get focus_input() {
    return t("聚焦学习过的输入框");
  },
  get focus_frontmost() {
    return t("聚焦输入框");
  },
  get system_action() {
    return t("系统与媒体动作");
  },
  get open_url() {
    return t("打开网址");
  },
  get run_macro() {
    return t("执行已有组合动作");
  },
};

export function macroActionLabel(action: MacroAction): string {
  return macroActionLabels[action] ?? action;
}

/** 组合键显示（修饰键固定顺序，`Ctrl + Alt + N`；与按键页文本惯例一致）。 */
export function macroChordLabel(chord: MacroChord): string {
  const modifiers = macroModifierOrder
    .filter((modifier) => chord.modifiers.includes(modifier))
    .map((modifier) => macroModifierLabels[modifier]);
  const key = chord.key.length === 1 ? chord.key.toUpperCase() : keyLabel(chord.key);
  return [...modifiers, key].join(" + ");
}

/** 步骤摘要（列表与详情共用一行说明）。 */
export function macroStepSummary(step: MacroStep): string {
  const parameters = step.parameters;
  switch (step.action) {
    case "open_application":
      return appTargetLabel(parameters.target);
    case "wait_for_application": {
      const app = appTargetLabel(parameters.target);
      return parameters.state === "running"
        ? t("{app} · App 已打开", { app })
        : t("{app} · App 窗口已出现", { app });
    }
    case "send_shortcut":
      return parameters.chord ? macroChordLabel(parameters.chord) : t("尚未录入按键");
    case "focus_input": {
      const app = appTargetLabel(parameters.target);
      return t("{app} · 聚焦学习过的输入框", { app });
    }
    case "focus_frontmost":
      return t("当前前台应用 · 聚焦输入框");
    case "system_action":
      return parameters.chord ? macroChordLabel(parameters.chord) : t("尚未选择系统动作");
    case "open_url":
      return parameters.url ?? t("尚未填写网址");
    case "run_macro":
      return parameters.nestedMacroName ?? t("尚未选择组合动作");
  }
}

function appTargetLabel(target: string | undefined): string {
  if (!target) return t("尚未选择 App");
  const known = presetAppNames.get(target);
  if (known) return known;
  if (!target.includes("\\") && !target.includes("/")) return target;
  const base = target.split(/[\\/]/).pop() ?? target;
  const stem = base.replace(/\.(exe|lnk)$/i, "");
  return stem || target;
}

/** 聚焦学习失败的机器码 → 用户文案（底层错误形如 `SendInput failed: no_candidate`）。 */
export function focusLearnErrorLabel(message: string): string {
  const code = message.replace(/^SendInput failed:\s*/i, "").trim();
  switch (code) {
    case "no_candidate":
      return t("没有学习成功，请确认目标应用已经打开，再点击它的输入框试一次");
    case "self_foreground":
      return t("学习期间无线麦在窗口最前面：请等目标应用打开后，再点击它的输入框试一次");
    case "not_foreground":
      return t("目标应用没有在最前面：请点击它的输入框后再试一次");
    case "target_exited":
      return t("目标应用已经退出：请重试，应用会重新打开");
    case "not_accessible":
      return t("这个输入框暂时读不到，请换一个输入框再试");
    case "timeout":
      return t("等待输入框超时，请重试");
    case "cancelled":
      return t("学习已取消");
    default:
      return message;
  }
}

/** 「记录点击位置」失败的机器码 → 用户文案（底层错误同样可能带 `SendInput failed:` 前缀）。 */
export function focusPointErrorLabel(message: string): string {
  const code = message.replace(/^SendInput failed:\s*/i, "").trim();
  switch (code) {
    case "cursor_outside_target":
      return t("鼠标不在目标应用的窗口里：把鼠标停在要点击的位置（例如输入框）上再试");
    case "target_window_missing":
      return t("目标应用没有可见窗口：先打开它再试");
    case "not_foreground":
      return t("目标应用没有在最前面：先切到它再试");
    case "window_size_changed":
      return t("目标窗口的大小和记录时相差太大：请重新记录一次位置");
    case "cursor_unavailable":
      return t("读不到鼠标位置：请重试");
    default:
      return message;
  }
}

/** 组合动作启动失败的机器码 → 用户文案（Rust 侧只给稳定 token）。 */
export function macroStartErrorLabel(code: string): string | null {
  if (code === "already_running") return t("这个组合动作正在执行，请稍后再试");
  if (code === "not_found") return t("找不到这个组合动作，请重新打开页面");
  if (code.startsWith("invalid:")) {
    const issues = code.slice("invalid:".length).split(",");
    if (issues.includes("name_empty")) return t("请先填写组合动作名称");
    if (issues.includes("name_too_long")) return t("组合动作名称太长，最多 20 个字符");
    if (issues.includes("too_few_steps") || issues.includes("too_many_steps")) {
      return t("步骤数量不对，请检查后重试");
    }
    if (issues.some((issue) => issue.startsWith("missing_parameter"))) {
      return t("有步骤还没填写完整，请检查每一步的参数");
    }
    return t("这条组合动作的设置不完整，请检查名称与步骤");
  }
  return null;
}

/** 整宏终态的用户文案（对齐 Mac `macro.status.execution.*`）。 */
export function macroTerminalLabel(result: MacroTerminalResult): string {
  switch (result) {
    case "completed":
      return t("组合动作执行成功");
    case "completed_with_unknown_result":
      return t("组合动作已执行，但无法确认第三方 App 的结果");
    case "cancelled":
      return t("组合动作已取消");
    case "timed_out":
      return t("组合动作执行超时");
    case "invalid_macro":
      return t("这条组合动作的设置有问题");
    case "missing_dependency":
      return t("需要使用的 App 或输入框记录不在这台电脑上");
    case "permission_denied":
      return t("组合动作缺少系统权限");
    case "nested_too_deep":
      return t("组合动作嵌套超过 8 层");
    case "cycle_detected":
      return t("组合动作不能直接或间接执行自己");
    case "step_failed":
      return t("组合动作中的步骤执行失败");
  }
}

/** 单步结果文案（对齐 Mac `macro.test.step.*`）。 */
export function macroStepOutcomeLabel(outcome: MacroStepOutcome): string {
  switch (outcome) {
    case "done":
      return t("步骤已完成");
    case "unknown":
      return t("请求已发送，请在目标 App 确认结果");
    case "missing_dependency":
      return t("所需动作或设置不可用");
    case "permission_denied":
      return t("权限未授权");
    case "failed":
      return t("执行失败，后续步骤已停止");
  }
}

/** 生成新的组合动作 ID（与库的稳定 ID 规则一致：`macro.` + 32 位十六进制）。 */
export function newMacroId(): string {
  const bytes = new Uint8Array(16);
  if (typeof crypto !== "undefined" && typeof crypto.getRandomValues === "function") {
    crypto.getRandomValues(bytes);
  } else {
    for (let index = 0; index < bytes.length; index += 1) {
      bytes[index] = Math.floor(Math.random() * 256);
    }
  }
  const hex = Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
  return `macro.${hex}`;
}

/** 生成新的步骤 ID（宏内唯一）。 */
export function newStepId(): string {
  const bytes = new Uint8Array(8);
  if (typeof crypto !== "undefined" && typeof crypto.getRandomValues === "function") {
    crypto.getRandomValues(bytes);
  } else {
    for (let index = 0; index < bytes.length; index += 1) {
      bytes[index] = Math.floor(Math.random() * 256);
    }
  }
  return `step.${Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("")}`;
}

/** 下一个补丁版本号（编辑保存总是新版本，对齐不可变版本语义）。 */
export function nextMacroVersion(version: string): string {
  const match = /^(\d+)\.(\d+)\.(\d+)$/.exec(version.trim());
  if (!match) return "1.0.0";
  return `${match[1]}.${match[2]}.${Number(match[3]) + 1}`;
}

/** 常用语门禁状态（与 Rust 侧 CommonPhraseAccess 一一对应）。 */
export type CommonPhraseAccess = "unavailable" | "denied" | "allowed";

/** 浮层面板一个位置的数据：只含短显示，不含完整句。 */
export interface CommonPhrasePanelEntry {
  button: string;
  label: string;
}

/** 浮层面板快照（渲染圆盘用）。 */
export interface CommonPhrasePanelSnapshot {
  /** 私有组件是否已注入（未注入时公开构建隐藏入口）。 */
  available: boolean;
  access: CommonPhraseAccess;
  canUse: boolean;
  /** 「常用语示例」只读预览模式（前端据此显示"登录会员后可使用"）。 */
  previewMode: boolean;
  /** 一次性失败提示的原因 key（**读取即消费**，Rust 侧读取后清空）；无提示为 null。 */
  notice: string | null;
  entries: CommonPhrasePanelEntry[];
}

/** 空快照：未注入私有组件或浏览器预览环境的回落值。 */
export function emptyCommonPhrasePanelSnapshot(): CommonPhrasePanelSnapshot {
  return {
    available: false,
    access: "unavailable",
    canUse: false,
    previewMode: false,
    notice: null,
    entries: [],
  };
}

/** 读取浮层面板数据。未注入私有组件时 available=false、entries 为空。 */
export async function getCommonPhrasePanelSnapshot(): Promise<CommonPhrasePanelSnapshot> {
  if (!isTauriRuntime()) {
    return emptyCommonPhrasePanelSnapshot();
  }
  try {
    return await invoke<CommonPhrasePanelSnapshot>("common_phrase_panel_snapshot");
  } catch {
    return emptyCommonPhrasePanelSnapshot();
  }
}

/** 关闭浮层面板（右上角关闭按钮；返回键由遥控器路径关闭）。 */
export async function closeCommonPhrasePanel(reason?: string): Promise<void> {
  if (!isTauriRuntime()) return;
  try {
    await invoke("close_common_phrase_panel", { reason: reason ?? null });
  } catch {
    // 关闭失败不阻塞界面：面板会在下一次按键或收尾路径里被关掉。
  }
}

/**
 * 订阅「面板显示」事件（宿主每次 show 面板都会广播）。
 *
 * 存在的理由：面板窗口是 NOACTIVATE，focus/visibility 变化不保证触发，
 * 仅靠前端事件会在"编辑常用语后再打开面板"时显示旧数据（2026-10-16 实测）。
 * 返回取消订阅函数。
 */
export async function onCommonPhrasePanelShown(handler: () => void): Promise<() => void> {
  if (!isTauriRuntime()) {
    return () => {};
  }
  const { listen } = await import("@tauri-apps/api/event");
  const unlisten = await listen("common-phrases-panel-shown", () => handler());
  return unlisten;
}

/** 鼠标点选圆盘某个位置（与遥控器按下等价）。返回是否已受理。 */
export async function selectCommonPhrase(button: string): Promise<boolean> {
  if (!isTauriRuntime()) return false;
  try {
    return await invoke<boolean>("select_common_phrase", { button });
  } catch {
    return false;
  }
}

/** 下行命令的返回体（`{"ok":true,"value":...}` / `{"ok":false,"error":"..."}`）。 */
interface CommonPhraseDispatchResult {
  ok: boolean;
  value?: string;
  error?: string;
}

/**
 * 下行命令通道（编辑器"调整常用语"、示例入口等）。未注入组件（公开构建）或
 * 未知命令会抛错——调用方必须按错误处理，不得静默当成功。
 */
async function dispatchCommonPhrase(command: string, payload?: unknown): Promise<string> {
  if (!isTauriRuntime()) {
    throw new Error("unavailable");
  }
  const raw = await invoke<string>("common_phrase_dispatch", {
    command,
    payload: payload === undefined ? null : JSON.stringify(payload),
  });
  const result = JSON.parse(raw) as CommonPhraseDispatchResult;
  if (!result.ok) {
    throw new Error(result.error ?? "edit_failed");
  }
  return result.value ?? "";
}

/** 编辑器（“调整常用语”）条目。 */
export interface CommonPhraseEditorEntry {
  id: string;
  chineseText: string;
  chineseLabel: string;
  englishText: string;
  englishLabel: string;
  /** 内置条目（编辑后自动落成用户副本）。 */
  builtIn: boolean;
}

/** 编辑器快照（“调整常用语”页数据源）。 */
export interface CommonPhraseEditorSnapshot {
  storeAvailable: boolean;
  loadFailed: boolean;
  /** 会员门禁：false 时界面显示会员提示、禁用编辑。 */
  allowed: boolean;
  entries: CommonPhraseEditorEntry[];
  /** 五个位置（ok/left/up/right/down）→ 条目 id；无分配的键位缺键。 */
  assignments: Record<string, string>;
}

/** 读取编辑器快照。未接通时抛错（调用方按会员提示处理）。 */
export async function getCommonPhraseEditorSnapshot(): Promise<CommonPhraseEditorSnapshot> {
  return JSON.parse(await dispatchCommonPhrase("editor_snapshot")) as CommonPhraseEditorSnapshot;
}

/** 编辑器保存输入：不给 id = 新建；给 id = 替换该条目。 */
export interface CommonPhraseEditInput {
  id?: string | null;
  chineseText: string;
  chineseLabel: string;
  englishText?: string;
  englishLabel?: string;
}

/** 保存一条常用语；返回条目 id（新建时为新用户副本 id）。 */
export async function saveCommonPhrase(input: CommonPhraseEditInput): Promise<string> {
  return dispatchCommonPhrase("editor_save", {
    id: input.id ?? null,
    chineseText: input.chineseText,
    chineseLabel: input.chineseLabel,
    englishText: input.englishText ?? "",
    englishLabel: input.englishLabel ?? "",
  });
}

/** 把某个面板位置分配给指定条目（立即保存）。 */
export async function assignCommonPhrase(button: string, id: string): Promise<void> {
  await dispatchCommonPhrase("editor_assign", { button, id });
}

/** 删除一条常用语（同时清掉相关分配）。 */
export async function deleteCommonPhrase(id: string): Promise<void> {
  await dispatchCommonPhrase("editor_delete", { id });
}

/** 按偏移量调整顺序（越界不留痕）。 */
export async function moveCommonPhrase(id: string, offset: number): Promise<void> {
  await dispatchCommonPhrase("editor_move", { id, offset });
}

/** 导入备份（整份替换；被改动的内置项转用户副本保留）。 */
export async function importCommonPhrases(json: string): Promise<void> {
  await dispatchCommonPhrase("editor_import", { json });
}

/** 导出备份（JSON 文本）。 */
export async function exportCommonPhrases(): Promise<string> {
  return dispatchCommonPhrase("editor_export");
}

/** 打开「常用语示例」（会员=普通面板、非会员=只读预览）。 */
export async function openCommonPhraseExample(): Promise<void> {
  await dispatchCommonPhrase("open_example");
}

/** 回眸：一个目标应用的汇总。 */
export interface TranscriptApplicationSummary {
  key: string;
  name: string;
  count: number;
  latestEndedAtMs: number;
  iconDataUrl: string | null;
}

/** 回眸：一条记录（不含输入框全文、窗口标题、路径等内部字段）。 */
export interface TranscriptRecordView {
  id: string;
  sessionId: string;
  endedAtMs: number;
  localDateKey: string;
  utcOffsetMinutes: number;
  applicationKey: string;
  applicationName: string;
  text: string;
}

/** 回眸：一条记录关联的原始录音摘要（不暴露文件路径）。 */
export interface TranscriptRecordingView {
  sessionId: string;
  endedAtMs: number;
  localDateKey: string;
  utcOffsetMinutes: number;
  applicationKey: string | null;
  applicationName: string | null;
  durationMs: number;
  byteCount: number;
}

/** 回眸页面快照。`recentCount` 只算最近 7 天，`totalCount` 是全部。 */
export interface TranscriptHistorySnapshot {
  enabled: boolean;
  audioEnabled: boolean;
  applications: TranscriptApplicationSummary[];
  records: TranscriptRecordView[];
  recordings: TranscriptRecordingView[];
  totalCount: number;
  recentCount: number;
  archiveError: string | null;
}

export async function getTranscriptHistory(): Promise<TranscriptHistorySnapshot> {
  return invoke<TranscriptHistorySnapshot>("get_transcript_history");
}

export async function setTranscriptHistoryEnabled(
  enabled: boolean,
): Promise<TranscriptHistorySnapshot> {
  return invoke<TranscriptHistorySnapshot>("set_transcript_history_enabled", { enabled });
}

export async function setTranscriptAudioRecordingEnabled(
  enabled: boolean,
): Promise<TranscriptHistorySnapshot> {
  return invoke<TranscriptHistorySnapshot>("set_transcript_audio_recording_enabled", { enabled });
}

export async function getTranscriptAudio(sessionId: string): Promise<string> {
  return invoke<string>("get_transcript_audio", { sessionId });
}

export async function exportTranscriptAudio(sessionId: string): Promise<boolean> {
  return invoke<boolean>("export_transcript_audio", { sessionId });
}

export async function deleteTranscriptRecording(
  sessionId: string,
): Promise<TranscriptHistorySnapshot> {
  return invoke<TranscriptHistorySnapshot>("delete_transcript_recording", { sessionId });
}

export async function deleteTranscript(
  recordId: string,
): Promise<TranscriptHistorySnapshot> {
  return invoke<TranscriptHistorySnapshot>("delete_transcript", { recordId });
}

export async function deleteTranscriptApplication(
  applicationKey: string,
): Promise<TranscriptHistorySnapshot> {
  return invoke<TranscriptHistorySnapshot>("delete_transcript_application", { applicationKey });
}

export async function deleteAllTranscripts(): Promise<TranscriptHistorySnapshot> {
  return invoke<TranscriptHistorySnapshot>("delete_all_transcripts");
}

/** 捕获成功后只广播一个轻量刷新信号，不带文字。 */
export async function subscribeTranscriptCaptured(
  handler: () => void,
): Promise<() => void> {
  const { listen } = await import("@tauri-apps/api/event");
  const unlisten = await listen("transcript-captured", () => handler());
  return () => {
    void unlisten();
  };
}

/** 录音落盘后广播刷新信号；没有文字记录时也能让「仅录音」条目及时出现。 */
export async function subscribeTranscriptRecordingFinished(
  handler: () => void,
): Promise<() => void> {
  const { listen } = await import("@tauri-apps/api/event");
  const unlisten = await listen("transcript-recording-finished", () => handler());
  return () => {
    void unlisten();
  };
}
