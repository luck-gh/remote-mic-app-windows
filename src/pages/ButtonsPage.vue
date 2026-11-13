<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { useCurrentTemplate } from "../lib/current-template";
import ButtonActionEditor from "../components/ButtonActionEditor.vue";
import BatteryIndicator from "../components/BatteryIndicator.vue";
import ComponentSupportPanel from "../components/ComponentSupportPanel.vue";
import MembershipRequiredButton from "../components/MembershipRequiredButton.vue";
import SettingsDialog from "../components/SettingsDialog.vue";
import { reportFrontendEvent, reasonToken } from "../lib/frontend-diagnostics";
import { t, tPage } from "../lib/i18n";
import { effectiveLanguage } from "../lib/language-state";
import { useUiPreference } from "../lib/ui-preferences";
import { COMBINATION_ACTIONS_ENABLED } from "../lib/feature-flags";
import {
  FOCUS_STRATEGY_OPTIONS,
  effectiveFocusProfile,
  presetFocusDefault,
} from "../lib/focus-defaults";
import {
  actionSummary,
  activateAppForLearning,
  buttonLabel,
  buttonLabels,
  buttonTriggerLabel,
  connectionPhaseLabel,
  disconnectRemote,
  chordLabel,
  exportMappingConfiguration,
  getButtonMappingSnapshot,
  getMappingConfiguration,
  getTemplateCatalog,
  getCombinedActions,
  getCommonPhrasePanelSnapshot,
  focusLearnErrorLabel,
  focusPointErrorLabel,
  learnFocusPoint,
  learnFocusTarget,
  listPresetApps,
  mouseClickLabels,
  mouseMoveLabels,
  openAppLabel,
  openCommonPhraseExample,
  pickCustomApp,
  registerCommonPhraseAvailability,
  registerPresetAppNames,
  remoteModelLabel,
  saveButtonMappings,
  saveButtonMappingTemplate,
  saveMappingConfiguration,
  setMenuTemplateSwitchEnabled,
  startRawInput,
  stopRawInput,
  subscribeButtonEdges,
  subscribeButtonGestures,
  updateButtonMappingTemplate,
  resetBuiltinTemplate,
  type ButtonAction,
  type ButtonActions,
  type AppFocusProfile,
  type ButtonEdge,
  type ButtonMappingSnapshot,
  type ButtonMappings,
  type MappingConfiguration,
  type TemplateCatalogEntry,
  type ButtonTrigger,
  type CommonPhraseAccess,
  type CustomAppPick,
  type FiredGesture,
  type KeyCode,
  type PresetAppInfo,
  type RawInputPhase,
  type MacroDefinition,
  type PairedRemote,
  type RemoteButton,
  type RemoteModel,
  type RuntimeSnapshot,
  type ShortcutCaptureEdge,
} from "../lib/bridge";

import type { NavigationRequest } from "../navigation";
const props = defineProps<{ runtime: RuntimeSnapshot | null; initialEditor?: { button: RemoteButton; trigger: ButtonTrigger; editingSource?: "common" } | null }>();
const READONLY_ICON = "M7 10V7a5 5 0 0 1 10 0v3M5 10h14v11H5zM12 14v3";
const emit = defineEmits<{ navigate: [page: string | NavigationRequest] }>();

/** 画布几何：紧凑高度固定 580，宽度流式
 * （占满容器，ResizeObserver 观测）；卡宽 = clamp((宽-260)/2, 270, 300)，
 * 与 Mac cardWidth(for:) 同公式；容器不足最小画布 800px 时由 CSS
 * --map-scale 连续缩放兜底（小于最小窗口的恢复态窗口）。 */
const CANVAS_MIN_WIDTH = 800;
const CANVAS_HEIGHT = 580;
const REMOTE_WIDTH = 202;
const REMOTE_HEIGHT = 410;
const CARD_HEIGHT = 72;
const REMOTE_TOP = (CANVAS_HEIGHT - REMOTE_HEIGHT) / 2;
const VOICE_CARD_TOP = 0;
const CARD_ROW_GAP = 12;
const ORDINARY_ROWS_TOP = VOICE_CARD_TOP + CARD_HEIGHT + CARD_ROW_GAP;

/** 语音独占顶部一行；普通六行在其下等距排列，卡片均使用同一尺寸。 */
function rowTarget(row: number): number {
  const step = CARD_HEIGHT + CARD_ROW_GAP;
  return (ORDINARY_ROWS_TOP + row * step + CARD_HEIGHT / 2) / CANVAS_HEIGHT;
}

const canvasEl = ref<HTMLElement | null>(null);
const remotePhotoEl = ref<HTMLElement | null>(null);
const remoteImageEl = ref<HTMLImageElement | null>(null);
const canvasWidth = ref(CANVAS_MIN_WIDTH);
const cardWidth = computed(() =>
  Math.min(300, Math.max(270, (canvasWidth.value - 260) / 2)),
);
const remoteLeft = computed(() => (canvasWidth.value - REMOTE_WIDTH) / 2);

interface Placement {
  button: RemoteButton;
  side: "left" | "right" | "center";
  anchor: [number, number];
  targetY: number;
}

/** 按键卡片布局表：对齐 Mac RemoteMappingLayout.buttonPlacements。 */
const PLACEMENTS: Placement[] = [
  { button: "power", side: "left", anchor: [0.386, 0.099], targetY: rowTarget(0) },
  { button: "up", side: "left", anchor: [0.502, 0.179], targetY: rowTarget(1) },
  { button: "left", side: "left", anchor: [0.362, 0.246], targetY: rowTarget(2) },
  { button: "back", side: "left", anchor: [0.406, 0.389], targetY: rowTarget(3) },
  { button: "home", side: "left", anchor: [0.406, 0.479], targetY: rowTarget(4) },
  { button: "menu", side: "left", anchor: [0.406, 0.569], targetY: rowTarget(5) },
  { button: "right", side: "right", anchor: [0.638, 0.246], targetY: rowTarget(0) },
  { button: "ok", side: "right", anchor: [0.502, 0.246], targetY: rowTarget(1) },
  { button: "down", side: "right", anchor: [0.502, 0.317], targetY: rowTarget(2) },
  { button: "volume_up", side: "right", anchor: [0.604, 0.39], targetY: rowTarget(3) },
  { button: "volume_down", side: "right", anchor: [0.604, 0.48], targetY: rowTarget(4) },
  { button: "tv", side: "right", anchor: [0.604, 0.569], targetY: rowTarget(5) },
];
const VOICE_PLACEMENT: Placement = {
  button: "ok", // 语音卡不对应 RemoteButton；占位仅用于定位。
  side: "center",
  anchor: [0.63, 0.099],
  targetY: (VOICE_CARD_TOP + CARD_HEIGHT / 2) / CANVAS_HEIGHT,
};
const TRIGGERS: ButtonTrigger[] = ["single", "double", "long"];

/** 当前连接的遥控器型号（未连接时 unknown，按 RC003 保守处理）。 */
const remoteModel = computed<RemoteModel>(
  () => props.runtime?.platform.connection.remoteModel ?? "unknown",
);

const deviceLabel = computed(() => {
  if (remoteModel.value !== "unknown") return remoteModelLabel(remoteModel.value);
  return connectionInfo.value?.remoteName ?? t("未连接遥控器");
});

/**
 * 操作失败文案：中文界面保留既有原始错误（既有行为与测试口径）；英文界面只给
 * 一句短的用户语言，原始错误写进控制台/诊断，不把中文原文摊到英文 UI 上。
 */
function operationErrorText(
  context: string,
  error: unknown,
  englishFallback: string,
): string {
  console.warn(context, error);
  if (effectiveLanguage.value === "zh-Hans") {
    return error instanceof Error ? error.message : String(error);
  }
  return t(englishFallback);
}

interface ImageFrame { left: number; top: number; width: number; height: number }
const remoteImageFrame = ref<ImageFrame>({
  left: remoteLeft.value,
  top: REMOTE_TOP,
  width: REMOTE_WIDTH,
  height: REMOTE_HEIGHT,
});

/**
 * 以图片实际渲染内容为唯一热点坐标系。照片、热点与 SVG 连线都读取这一个 frame；
 * 即使容器尺寸或 object-fit 发生变化，也不会再各自使用独立 top/scale。
 */
function measureRemoteImageFrame(): void {
  const photo = remotePhotoEl.value;
  const image = remoteImageEl.value;
  if (!photo || !image) return;
  const hasLayout = photo.clientWidth > 0 && photo.clientHeight > 0;
  const boxWidth = hasLayout ? photo.clientWidth : REMOTE_WIDTH;
  const boxHeight = hasLayout ? photo.clientHeight : REMOTE_HEIGHT;
  const naturalWidth = image.naturalWidth || REMOTE_WIDTH;
  const naturalHeight = image.naturalHeight || REMOTE_HEIGHT;
  const scale = Math.max(boxWidth / naturalWidth, boxHeight / naturalHeight);
  const width = naturalWidth * scale;
  const height = naturalHeight * scale;
  remoteImageFrame.value = {
    left: (hasLayout ? photo.offsetLeft : remoteLeft.value) + (boxWidth - width) / 2,
    top: (hasLayout ? photo.offsetTop : REMOTE_TOP) + (boxHeight - height) / 2,
    width,
    height,
  };
}

function anchorPoint(placement: Placement): { x: number; y: number } {
  const frame = remoteImageFrame.value;
  return {
    x: frame.left + frame.width * placement.anchor[0],
    y: frame.top + frame.height * placement.anchor[1],
  };
}

function cardTop(placement: Placement): number {
  return placement.targetY * CANVAS_HEIGHT - CARD_HEIGHT / 2;
}

/** 卡片朝向遥控器一侧的边缘中点（箭头/连线的落点基准）。 */
function cardEdgePoint(placement: Placement): { x: number; y: number } {
  if (placement.side === "center") {
    return { x: canvasWidth.value / 2, y: VOICE_CARD_TOP + CARD_HEIGHT };
  }
  return {
    x: placement.side === "left" ? cardWidth.value : canvasWidth.value - cardWidth.value,
    y: placement.targetY * CANVAS_HEIGHT,
  };
}

/** 连线与箭头一体化：线画到箭头底部（距卡边 13px），箭头补足到距卡边 7px，
 * 整体读作一条带箭头的连线（线不再穿过箭头延伸到卡片边缘）。 */
function lineEndPoint(placement: Placement): { x: number; y: number } {
  const edge = cardEdgePoint(placement);
  if (placement.side === "center") return { x: edge.x, y: edge.y + 13 };
  const direction = placement.side === "left" ? -1 : 1;
  return { x: edge.x - direction * 13, y: edge.y };
}

function connectionPath(placement: Placement): string {
  const start = anchorPoint(placement);
  const end = lineEndPoint(placement);
  if (placement.side === "center") {
    const distance = Math.min(48, Math.max(24, (start.y - end.y) * 0.48));
    return `M ${start.x.toFixed(1)} ${start.y.toFixed(1)} C ${start.x.toFixed(1)} ${(start.y - distance).toFixed(1)}, ${end.x.toFixed(1)} ${(end.y + distance * 0.55).toFixed(1)}, ${end.x.toFixed(1)} ${end.y.toFixed(1)}`;
  }
  const direction = placement.side === "left" ? -1 : 1;
  const distance = Math.min(70, Math.max(34, Math.abs(end.x - start.x) * 0.58));
  const endpointDistance = Math.min(42, Math.max(24, distance * 0.6));
  const control1 = { x: start.x + direction * distance, y: start.y };
  const control2 = { x: end.x - direction * endpointDistance, y: end.y };
  return `M ${start.x.toFixed(1)} ${start.y.toFixed(1)} C ${control1.x.toFixed(1)} ${control1.y.toFixed(1)}, ${control2.x.toFixed(1)} ${control2.y.toFixed(1)}, ${end.x.toFixed(1)} ${end.y.toFixed(1)}`;
}

/** 箭头（与连线同色同类）：尖端距卡片边 7px、底宽 12px/半高 4px，
 * 底部与连线终点重合（整体一条连线）。 */
function arrowPolygon(placement: Placement): string {
  const edge = cardEdgePoint(placement);
  if (placement.side === "center") {
    const tipY = edge.y + 7;
    const baseY = tipY + 6;
    return `${edge.x.toFixed(1)},${tipY.toFixed(1)} ${(edge.x - 4).toFixed(1)},${baseY.toFixed(1)} ${(edge.x + 4).toFixed(1)},${baseY.toFixed(1)}`;
  }
  const direction = placement.side === "left" ? -1 : 1;
  const tip = { x: edge.x - direction * 7, y: edge.y };
  const baseX = tip.x - direction * 6;
  return `${tip.x.toFixed(1)},${tip.y.toFixed(1)} ${baseX.toFixed(1)},${(tip.y - 4).toFixed(1)} ${baseX.toFixed(1)},${(tip.y + 4).toFixed(1)}`;
}

/** 按键图标：SVG path 组（24x24 视窗），形状对齐 Mac RemoteMappingCanvas
 * 的 SF Symbols（power/chevron/circle.circle/uturn/speaker/house/line.3/tv）。 */
const buttonIcons: Record<RemoteButton, string[]> = {
  power: ["M12 3v8", "M7.2 6.4a7 7 0 1 0 9.6 0"],
  up: ["M6 14.5l6-6 6 6"],
  down: ["M6 9.5l6 6 6-6"],
  left: ["M14.5 6l-6 6 6 6"],
  right: ["M9.5 6l6 6-6 6"],
  ok: ["M12 3.5a8.5 8.5 0 1 1 0 17 8.5 8.5 0 0 1 0-17z", "M12 10a2.4 2.4 0 1 1 0 4.8 2.4 2.4 0 0 1 0-4.8z"],
  back: ["M9 13.5L4 8.5l5-5", "M4 8.5h10.2a5.5 5.5 0 0 1 0 11H11"],
  home: ["M3.5 11.2L12 3.5l8.5 7.7", "M6 9.5V20.5h12V9.5", "M10 20.5v-5.5h4v5.5"],
  menu: ["M4 6h16", "M4 12h16", "M4 18h16"],
  tv: ["M3 7.5h18a1 1 0 0 1 1 1V18a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1V8.5a1 1 0 0 1 1-1z", "M17 3l-5 4-5-4"],
  volume_up: ["M11 5.5L6.5 9H3v6h3.5L11 18.5z", "M15.5 9.5l5 5", "M20.5 9.5l-5 5"],
  volume_down: ["M11 5.5L6.5 9H3v6h3.5L11 18.5z", "M15 12h5.5"],
  volume_mute: ["M11 5.5L6.5 9H3v6h3.5L11 18.5z", "M15.5 9.5l5 5", "M20.5 9.5l-5 5"],
};
/** 语音键图标（Mac mic.fill：实心话筒）。 */
const VOICE_ICON_FILLED = "M12 2.8a3.4 3.4 0 0 1 3.4 3.4v5.6a3.4 3.4 0 0 1-6.8 0V6.2A3.4 3.4 0 0 1 12 2.8z";
const VOICE_ICON_STROKES = ["M6.3 11.5a5.7 5.7 0 0 0 11.4 0", "M12 17.2v3.8"];

/**
 * 两个总开关的上次已知值（2026-10-28）。
 *
 * 为什么要单独缓存：这两只开关的值来自 IPC，而 IPC 必然异步——首帧只能先渲染占位，
 * 等数据回来再挂开关本体，用户看到的就是"先空白、再跳出来"（2026-10-28 反馈）。
 * 把上次的值同步写进 localStorage，首帧就能直接渲染成正确状态；IPC 返回的权威值
 * 通常与缓存一致，于是没有可见跳变。首次安装没有缓存时开关仍会从默认值改成真实值，
 * 那一次跳变无法避免。
 *
 * 只存两个布尔值，不含任何用户数据。
 */
const SWITCH_CACHE_KEY = "sayall.buttons.switches.v1";

function readSwitchCache(): { enabled: boolean; lockSelection: boolean } | null {
  try {
    const raw = localStorage.getItem(SWITCH_CACHE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as { enabled?: unknown; lockSelection?: unknown };
    if (typeof parsed.enabled !== "boolean" || typeof parsed.lockSelection !== "boolean") {
      return null;
    }
    return { enabled: parsed.enabled, lockSelection: parsed.lockSelection };
  } catch {
    return null;
  }
}

function writeSwitchCache(enabled: boolean, lockSelection: boolean): void {
  try {
    localStorage.setItem(SWITCH_CACHE_KEY, JSON.stringify({ enabled, lockSelection }));
  } catch {
    // 存储不可用时只是少一次首帧加速，不影响功能与保存。
  }
}

const switchCache = readSwitchCache();
const mappings = ref<ButtonMappings>({ enabled: switchCache?.enabled ?? true, actions: {} });
/** 映射是否已从设置读回：「启用自定义按键功能」开关就绪前用占位符顶位（防滑入）。 */
const savedSnapshot = ref<ButtonMappings>({ enabled: true, actions: {} });
type EditingSource = "common" | `template:${string}`;
const currentTemplate = useCurrentTemplate();
const configuration = ref<MappingConfiguration | null>(null);
// This is the saved Menu opt-in, not program-default following or mapping enablement.
const menuTemplateSwitchEnabled = ref<boolean | null>(null);
const menuStateReadFailed = ref(false);
const menuModePending = ref(false);
const menuModeError = ref<string | null>(null);
let menuConfigurationRevision = 0;
const templateCatalog = ref<TemplateCatalogEntry[]>([]);
const editingSource = ref<EditingSource>("common");
// 导航只选择编辑目标, 不改变实际应用中的模板。
const navigationEditingSource = ref<EditingSource | null>(props.initialEditor?.editingSource ?? null);
const retainedDrafts = ref(new Map<EditingSource, { mappings: ButtonMappings; saved: ButtonMappings }>());
let discardedSource: EditingSource | null = null;
/** 已安装的预设应用（打开应用动作可选列表）。 */
const presetApps = ref<PresetAppInfo[]>([]);
/** 组合动作库（「组合动作」动作分组的候选；2026-10-15）。 */
/** 常用语组件是否可用（页面加载快照后更新；不可用时不显示动作入口）。 */
const selectedButton = ref<RemoteButton | null>(null);
const editingTarget = ref<{ button: RemoteButton; trigger: ButtonTrigger } | null>(null);
const editorPanel = ref<HTMLElement | null>(null);
const selectionPreference = useUiPreference("lockButtonSelection");
const lockSelection = selectionPreference.value;
const activeButtons = ref<Set<RemoteButton>>(new Set());
const lastFired = ref<FiredGesture | null>(null);
const firedFlash = ref<{ button: RemoteButton; trigger: ButtonTrigger } | null>(null);
const mappingSnapshot = ref<ButtonMappingSnapshot | null>(null);
const busy = ref(false);
const statusMessage = ref<string | null>(null);
const saveTemplateDialogOpen = ref(false);
const footerFeedbackOpen = ref(false);
const displayedFooterFeedback = ref<string[]>([]);
const selectionToggle = ref<HTMLInputElement | null>(null);
let footerFeedbackTrigger: HTMLButtonElement;
const templateNameDraft = ref("");
const templateNameError = ref<string | null>(null);
let unlistenEdges: (() => void) | null = null;
let unlistenGestures: (() => void) | null = null;
let snapshotTimer: number | null = null;
let flashTimer: number | null = null;
let resizeObserver: ResizeObserver | null = null;
let unmounted = false;
let resourcesReady = false;
let saveSequence = 0;
let snapshotPending = false;

function menuReserved(button: RemoteButton): boolean {
  return button === "menu" && menuTemplateSwitchEnabled.value !== false;
}

const menuStateLabel = computed(() => menuTemplateSwitchEnabled.value === true
  ? "已启用模板切换" : menuStateReadFailed.value ? "菜单功能读取失败" : "正在读取菜单功能");
const menuBehavior: Record<ButtonTrigger, string> = {
  single: "打开 / 取消",
  double: "按单击处理",
  long: "切换保存选项",
};
const menuBehaviorDetail: Record<ButtonTrigger, string> = {
  single: "面板外单击打开模板选择；面板内单击取消选择。",
  double: "没有独立双击动作，连续短按按两次单击处理。",
  long: "面板外与单击相同，仅打开；面板内再次长按切换“同时更新此程序的默认模板”，松开不取消。",
};

function cellSummary(button: RemoteButton, trigger: ButtonTrigger): string {
  if (!menuReserved(button)) return actionSummary(actionOf(button, trigger));
  return menuTemplateSwitchEnabled.value === true ? menuBehavior[trigger] : "暂不可编辑";
}

async function readMenuConfiguration(): Promise<MappingConfiguration | null> {
  if (menuModePending.value) return null;
  const revision = menuConfigurationRevision;
  try {
    const saved = await getMappingConfiguration();
    if (!unmounted && revision === menuConfigurationRevision && !menuModePending.value) applySavedMenuMode(saved);
    return saved;
  } catch {
    if (!unmounted && revision === menuConfigurationRevision && !menuModePending.value) {
      if (!menuStateReadFailed.value) reportFrontendEvent({event:"buttons_menu_ownership", phase:"completed", result:"failed", reason:"configuration_read_failed"});
      menuTemplateSwitchEnabled.value = null;
      menuStateReadFailed.value = true;
    }
    return null;
  }
}

function applySavedMenuMode(saved: MappingConfiguration): void {
  menuTemplateSwitchEnabled.value = saved.menuTemplateSwitchEnabled;
  menuStateReadFailed.value = false;
  if (configuration.value) configuration.value = { ...configuration.value, menuTemplateSwitchEnabled: saved.menuTemplateSwitchEnabled };
}

async function toggleMenuMode(event: Event): Promise<void> {
  const previous = menuTemplateSwitchEnabled.value;
  // A native change toggles the DOM first; show only the confirmed setting while saving.
  (event.target as HTMLInputElement).checked = previous === true;
  if (unmounted || busy.value || menuModePending.value || previous === null || !configuration.value) return;
  const revision = ++menuConfigurationRevision;
  menuModePending.value = true;
  menuModeError.value = null;
  reportFrontendEvent({event:"buttons_menu_mode", phase:"requested", result:"passed", reason:previous ? "disable" : "enable"});
  try {
    const saved = await setMenuTemplateSwitchEnabled(!previous);
    if (unmounted || revision !== menuConfigurationRevision) return;
    applySavedMenuMode(saved);
    reportFrontendEvent({event:"buttons_menu_mode", phase:"completed", result:"passed", reason:"saved_setting_applied"});
  } catch {
    if (unmounted || revision !== menuConfigurationRevision) return;
    menuModeError.value = "保存未确认，请检查当前开关状态。";
    reportFrontendEvent({event:"buttons_menu_mode", phase:"completed", result:"failed", reason:"save_unconfirmed"});
    try {
      const saved = await getMappingConfiguration();
      if (!unmounted && revision === menuConfigurationRevision) applySavedMenuMode(saved);
    } catch {
      if (!unmounted && revision === menuConfigurationRevision) {
        menuTemplateSwitchEnabled.value = null;
        menuStateReadFailed.value = true;
        reportFrontendEvent({event:"buttons_menu_mode", phase:"completed", result:"failed", reason:"readback_failed"});
      }
    }
  } finally {
    if (!unmounted && revision === menuConfigurationRevision) menuModePending.value = false;
  }
}

watch(menuTemplateSwitchEnabled, (value) => {
  if (value !== false) {
    if (editingTarget.value?.button === "menu") editingTarget.value = null;
    if (selectedButton.value === "menu") selectedButton.value = null;
  }
  if (value !== null) reportFrontendEvent({event:"buttons_menu_ownership", phase:"completed", result:"passed", reason:value ? "template_menu_reserved" : "custom_mapping_available"});
}, {flush:"sync"});

function releasePageResources(): void {
  unlistenEdges?.();
  unlistenEdges = null;
  unlistenGestures?.();
  unlistenGestures = null;
  if (snapshotTimer !== null) window.clearInterval(snapshotTimer);
  snapshotTimer = null;
  if (flashTimer !== null) window.clearTimeout(flashTimer);
  flashTimer = null;
  resizeObserver?.disconnect();
  resizeObserver = null;
}

/** 连续执行分组的「关闭」选项：同一中文原文在编辑面板里是 Close，这里需要 Off。 */
const repeatOffLabel = computed(() =>
  effectiveLanguage.value === "zh-Hans" ? "关闭" : "Off",
);

const activeTemplateEntry = computed(() => editingSource.value === "common" ? null : templateCatalog.value.find(t => t.id === editingSource.value.slice("template:".length)) ?? null);
const dirty = computed(() => JSON.stringify(mappings.value) !== JSON.stringify(savedSnapshot.value));
const editingAvailable = computed(() => !!currentTemplate.applied.value && mappingsForSource(editingSource.value) !== null);
const editingBusy = computed(() => busy.value || currentTemplate.applying.value || !editingAvailable.value);
const footerFeedback = computed(() => {
  const messages: string[] = [];
  if (selectionPreference.error.value) messages.push(selectionPreference.error.value);
  if (statusMessage.value) messages.push(statusMessage.value);
  if (mappingSnapshot.value?.lastError) messages.push(mappingSnapshot.value.lastError);
  if (mappingSnapshot.value && !mappings.value.enabled) messages.push("总开关关闭（按键保持原样）");
  if (dirty.value) messages.push("当前编辑目标有未保存更改");
  return messages;
});

function openFooterFeedback(event: MouseEvent): void {
  footerFeedbackTrigger = event.currentTarget as HTMLButtonElement;
  displayedFooterFeedback.value = [...footerFeedback.value];
  footerFeedbackOpen.value = true;
  reportFrontendEvent({ event: "button_mapping_feedback", phase: "started", result: "passed", reason: "user_opened" });
}

async function closeFooterFeedback(): Promise<void> {
  footerFeedbackOpen.value = false;
  await nextTick();
  (footerFeedbackTrigger.isConnected ? footerFeedbackTrigger : selectionToggle.value!).focus();
  reportFrontendEvent({ event: "button_mapping_feedback", phase: "completed", result: "passed", reason: "user_dismissed" });
}

function trapFooterFeedbackFocus(event: KeyboardEvent): void {
  const controls = (event.currentTarget as HTMLElement).querySelectorAll<HTMLElement>('button, [tabindex="0"]');
  const first = controls[0]!;
  const last = controls[controls.length - 1]!;
  if (event.shiftKey && document.activeElement === first) {
    event.preventDefault();
    last.focus();
  } else if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault();
    first.focus();
  }
}

const enabled = computed({
  get: () => mappings.value.enabled,
  set: (value: boolean) => {
    mappings.value = { ...mappings.value, enabled: value };
  },
});

const activeTemplateOptions = computed(() => {
  const activeId = currentTemplate.templateId.value;
  const active = activeId ? templateCatalog.value.find(entry => entry.id === activeId) : null;
  return [
    { value: "common", label: "通用配置" },
    ...(activeId && !active ? [{ value: activeId, label: currentTemplate.applied.value?.name ?? "当前模板" }] : []),
    ...templateCatalog.value.map(entry => ({ value: entry.id, label: entry.name })),
  ];
});
const templateRefreshError = ref('');
let templateRefreshPending = false;
let templateRefreshRequest = 0;
async function refreshTemplateChoices(): Promise<void> {
  if (templateRefreshPending || unmounted) return;
  templateRefreshPending = true;
  const request = ++templateRefreshRequest;
  const catalogAtRequest = templateCatalog.value;
  reportFrontendEvent({event: 'button_template_choices', phase: 'requested', result: 'unknown', reason: 'selector_opened'});
  try {
    const catalog = await getTemplateCatalog();
    if (unmounted || request !== templateRefreshRequest) return;
    if (templateCatalog.value !== catalogAtRequest) {
      reportFrontendEvent({event: 'button_template_choices', phase: 'completed', result: 'unknown', reason: 'catalog_changed_while_reading'});
      return;
    }
    templateCatalog.value = catalog;
    templateRefreshError.value = '';
    reportFrontendEvent({event: 'button_template_choices', phase: 'completed', result: 'passed', reason: 'read_completed'});
  } catch {
    if (!unmounted && request === templateRefreshRequest) templateRefreshError.value = '未能刷新模板列表，请重新打开下拉菜单重试。';
    reportFrontendEvent({event: 'button_template_choices', phase: 'completed', result: 'failed', reason: 'read_failed'});
  } finally { if (request === templateRefreshRequest) templateRefreshPending = false; }
}

async function selectActiveTemplate(event: Event): Promise<void> {
  const select = event.currentTarget as HTMLSelectElement;
  const requested = select.value === "common" ? null : select.value;
  select.value = currentTemplate.applied.value?.templateId ?? "common";
  if (busy.value || currentTemplate.applying.value || requested === currentTemplate.templateId.value) return;
  reportFrontendEvent({ event: "button_mapping_template_switch", phase: "started", result: "passed", reason: dirty.value ? "unsaved_draft" : "saved_configuration" });
  const source = editingSource.value;
  if (dirty.value) {
    if (window.confirm("当前配置有未保存更改。确定将先保存，再切换当前模板。")) {
      if (!await persist("配置已保存")) {
        reportFrontendEvent({ event: "button_mapping_template_switch", phase: "completed", result: "failed", reason: "save_failed" });
        return;
      }
      if (editingSource.value !== source) {
        reportFrontendEvent({ event: "button_mapping_template_switch", phase: "completed", result: "cancelled", reason: "external_template_changed" });
        return;
      }
    } else if (!window.confirm("放弃当前未保存更改并切换当前模板？")) {
      reportFrontendEvent({ event: "button_mapping_template_switch", phase: "completed", result: "cancelled", reason: "draft_retained" });
      return;
    } else {
      discardedSource = source;
    }
  }
  await currentTemplate.activate(requested);
  if (!currentTemplate.error.value && currentTemplate.templateId.value === requested) navigationEditingSource.value = null;
  await nextTick();
  discardedSource = null;
  reportFrontendEvent({ event: "button_mapping_template_switch", phase: "completed", result: currentTemplate.error.value ? "failed" : currentTemplate.templateId.value === requested ? "passed" : "skipped", reason: currentTemplate.error.value ? "engine_rejected" : currentTemplate.templateId.value === requested ? "engine_confirmed" : "superseded" });
}

function cloneMappings(value: ButtonMappings): ButtonMappings {
  return JSON.parse(JSON.stringify(value)) as ButtonMappings;
}

function mappingsForSource(source: EditingSource): ButtonMappings | null {
  if (!configuration.value) return null;
  if (source === "common") return configuration.value.commonMappings;
  const templateId = source.slice("template:".length);
  return configuration.value.templates.find(template => template.id === templateId)?.mappings
    ?? templateCatalog.value.find(template => template.id === templateId)?.buttonMappings ?? null;
}

function loadEditingSource(source: EditingSource): boolean {
  const selected = mappingsForSource(source);
  editingSource.value = source;
  editingTarget.value = null;
  if (!selected) {
    mappings.value = { enabled: false, actions: {} };
    savedSnapshot.value = cloneMappings(mappings.value);
    statusMessage.value = "当前模板配置不可用；请选择其他模板后再编辑";
    return false;
  }
  mappings.value = cloneMappings(selected);
  savedSnapshot.value = cloneMappings(selected);
  statusMessage.value = null;
  return true;
}

function openRequestedEditor(request: NonNullable<typeof props.initialEditor>): void {
  if (!configuration.value) return;
  const source = request.editingSource ?? (currentTemplate.templateId.value === null ? "common" : `template:${currentTemplate.templateId.value}`);
  if (!loadEditingSource(source)) return;
  selectedButton.value = request.button;
  editingTarget.value = { button: request.button, trigger: request.trigger };
  reportFrontendEvent({ event: "button_mapping_editor_target", phase: "completed", result: "passed", reason: request.editingSource ? "navigation_common_binding" : "navigation_applied_template" });
}

watch(() => props.initialEditor, request => {
  navigationEditingSource.value = request?.editingSource ?? null;
  if (request) openRequestedEditor(request);
});

watch([currentTemplate.applied, configuration, navigationEditingSource], () => {
  if (!currentTemplate.applied.value || !configuration.value) return;
  const source: EditingSource = navigationEditingSource.value ?? (currentTemplate.templateId.value === null ? "common" : `template:${currentTemplate.templateId.value}`);
  if (source === editingSource.value) return;
  if (dirty.value) {
    if (discardedSource !== editingSource.value) {
      retainedDrafts.value.set(editingSource.value, { mappings: cloneMappings(mappings.value), saved: cloneMappings(savedSnapshot.value) });
    }
    reportFrontendEvent({ event: "button_mapping_editor_draft", phase: "completed", result: "passed", reason: discardedSource === editingSource.value ? "discarded" : "retained" });
  }
  const draft = retainedDrafts.value.get(source);
  const loaded = loadEditingSource(source);
  if (loaded && draft) {
    mappings.value = draft.mappings;
    savedSnapshot.value = draft.saved;
    retainedDrafts.value.delete(source);
    statusMessage.value = "已恢复此模板的未保存草稿；保存后才会生效";
  }
  reportFrontendEvent({ event: "button_mapping_editor_target", phase: "completed", result: loaded ? "passed" : "failed", reason: !loaded ? "configuration_unavailable" : draft ? "retained_draft" : "applied_template" });
});

const voiceActive = computed(
  () => props.runtime?.platform.connection.voiceState === "streaming",
);
const voiceButtonPressed = computed(() => currentTemplate.sceneSnapshot.value?.voiceButtonPressed === true);
const voiceReceived = ref(false);
const voiceStarted = ref(false);
// Observe each edge synchronously: a quick DOWN/UP may share a Vue render tick.
// Keep the receipt visible after UP without inventing a longer physical hold.
watch(voiceButtonPressed, (pressed) => {
  if (pressed) {
    voiceReceived.value = true;
    voiceStarted.value = voiceActive.value;
  }
  reportFrontendEvent({ event: "buttons_voice_observation", phase: "completed", result: "passed", reason: pressed ? "pressed" : "released" });
}, { flush: "sync" });
watch(voiceActive, (active) => {
  if (active && voiceReceived.value) voiceStarted.value = true;
}, { flush: "sync" });
const voiceFeedback = computed(() => {
  if (!voiceReceived.value) return "";
  if (voiceActive.value) return "已收到语音键，语音进行中。";
  const error = props.runtime?.platform.connection.lastError;
  if (error) {
    const detail = error === "select an output endpoint before starting voice"
      ? "声音通道尚未就绪，请到“连接与语音”选择可用的声音通道。"
      : error === "the selected audio endpoint is temporarily unavailable; its saved selection is retained"
        ? "所选声音通道暂不可用，请到“连接与语音”检查。"
        : error;
    return `已收到语音键。当前语音状态：${detail}`;
  }
  if (voiceStarted.value) return "已收到语音键，语音已结束。";
  return voiceButtonPressed.value ? "已收到语音键，正在等待语音开始。" : "已收到语音键，未观察到语音开始。";
});

function actionsOf(button: RemoteButton): ButtonActions {
  return (
    mappings.value.actions[button] ?? {
      single: { type: "disabled" },
      double: { type: "disabled" },
      long: { type: "disabled" },
    }
  );
}

function actionOf(button: RemoteButton, trigger: ButtonTrigger): ButtonAction {
  return actionsOf(button)[trigger];
}

/** 卡片格子的悬停标题：随界面语言实时生成（含全角冒号的原文作为字典 key）。 */
function actionCellTitle(button: RemoteButton, trigger: ButtonTrigger): string {
  return t("{button} · {trigger}：{summary}", {
    button: buttonLabels[button],
    trigger: buttonTriggerLabel(trigger),
    summary: actionSummary(actionOf(button, trigger)),
  });
}
function selectButton(button: RemoteButton): void {
  if (busy.value || menuReserved(button)) return;
  selectedButton.value = button;
}

function openEditor(button: RemoteButton, trigger: ButtonTrigger): void {
  if (busy.value || menuReserved(button)) return;
  selectedButton.value = button;
  editingTarget.value = { button, trigger };
}

const REPEAT_CAPABLE_BUTTONS: RemoteButton[] = [
  "back",
  "up",
  "down",
  "left",
  "right",
  "volume_up",
  "volume_down",
];
/** 单独修饰键：不可作为「可连续执行」的单键动作（与 Rust `KeyCode::is_modifier` 一致）。 */
const REPEAT_MODIFIER_KEYS = new Set<KeyCode>([
  "control",
  "left_control",
  "right_control",
  "shift",
  "left_shift",
  "right_shift",
  "alt",
  "left_alt",
  "right_alt",
  "left_windows",
  "right_windows",
]);

/** 动作是否可连续执行（与 Rust `ButtonAction::allows_repeat` 同源口径）。 */
function actionAllowsRepeat(action: ButtonAction): boolean {
  if (action.type === "scroll" || action.type === "mouse_move" || action.type === "app_switcher" || action.type === "page_up" || action.type === "page_down") return true;
  if (action.type !== "shortcut") return false;
  return action.chord.keys.length === 1 && !REPEAT_MODIFIER_KEYS.has(action.chord.keys[0]);
}

const repeatGroupVisible = computed(() => {
  const target = editingTarget.value;
  if (!target) return false;
  return target.trigger === "single" || target.trigger === "long";
});

type HoldRepeatChoice = "auto" | "off" | "single" | "long";

/** 选项不可选的原因（null = 可选）；radio 展示与一致性收口共用同一判据。 */
function holdRepeatBlockedReason(
  actions: ButtonActions,
  slot: "single" | "long",
): string | null {
  if (slot === "long") {
    if (actions.long.type === "disabled") return t("先给这个按键配置长按动作");
    if (!actionAllowsRepeat(actions.long)) {
      return t("“长按”的动作只执行一次（组合快捷键、打开应用等），不能连续执行；把它改成单个按键后可选");
    }
    return null;
  }
  if (actions.single.type === "disabled") return t("先给这个按键配置单击动作");
  if (!actionAllowsRepeat(actions.single)) {
    return t("“单击”的动作只执行一次（组合快捷键、打开应用等），不能连续执行；改成单个按键后可选");
  }
  if (actions.long.type !== "disabled") {
    // 指路前先确认「重复长按动作」真的可选：长按动作只执行一次时它同样是
    // 灰的，原句会指向一个点不到的选项（2026-10-05 用户反馈）。
    return holdRepeatBlockedReason(actions, "long") === null
      ? t("这个按键已有长按动作，按住会先执行长按；要连续执行，请选“重复长按动作”")
      : t("这个按键已有长按动作：按住会执行长按。要连续执行单击动作，请先清除长按动作");
  }
  return null;
}

/** 当前按键的「按住时连续执行」状态（按键级，两页同源）。 */
const repeatChoice = computed<HoldRepeatChoice>(() => {
  const target = editingTarget.value;
  const hold = target ? actionsOf(target.button).holdRepeat : undefined;
  return hold ?? "auto";
});

const repeatAutoHint = computed(() => {
  const target = editingTarget.value;
  if (!target) return "";
  const actions = actionsOf(target.button);
  if (actions.long.type !== "disabled") return "当前：按住后执行长按动作一次。";
  if (actions.single.type === "disabled") return "当前：未设置单击动作，按住不执行动作。";
  return actionAllowsRepeat(actions.single)
    ? "当前：按住会连续执行单击动作。"
    : "当前：单击动作只执行一次。";
});

const repeatSingleReason = computed<string | null>(() => {
  const target = editingTarget.value;
  return target ? holdRepeatBlockedReason(actionsOf(target.button), "single") : null;
});

const repeatLongReason = computed<string | null>(() => {
  const target = editingTarget.value;
  return target ? holdRepeatBlockedReason(actionsOf(target.button), "long") : null;
});

/** 常驻原因行：只解释当前页面对应的选项。 */
const repeatPageReason = computed<string | null>(() => {
  const target = editingTarget.value;
  if (!target || repeatChoice.value === "auto") return null;
  if (target.trigger === "single") return repeatSingleReason.value;
  if (target.trigger === "long") return repeatLongReason.value;
  return null;
});

/** 当前编辑按键是否配置了双击（「重复单击动作」悬停据此切换口径）。 */
const doubleConfiguredOnEditingButton = computed(() => {
  const target = editingTarget.value;
  return !!target && actionsOf(target.button).double.type !== "disabled";
});

/** 「重复单击动作」的悬停说明（不可选时为原因；配双击时按压住确认口径）。 */
const repeatSingleTitle = computed(() => {
  if (repeatSingleReason.value) return repeatSingleReason.value;
  return doubleConfiguredOnEditingButton.value
    ? t("按住超过约 0.3 秒开始，单击动作会不断重复，直到松手")
    : t("按住不放，单击动作会不断重复，直到松手");
});

/** 「重复长按动作」的悬停说明（不可选时为原因）。 */
const repeatLongTitle = computed(
  () => repeatLongReason.value ?? t("按住约 0.55 秒后，长按动作会不断重复，直到松手"),
);

/**
 * OK 键「移动光标后按 OK 点击」（2026-10-06 定稿，方案见
 * `docs/plan/2026-10-06-ok-context-click.md`、产品逻辑见
 * `docs/product/button-behavior.md` §2）：按键级开关、默认关、只对 OK 键提供，
 * 三个槽位页都在同一位置显示（与当前编辑哪个槽位无关）。
 */
const okClickVisible = computed(
  () => editingSource.value === "common" && editingTarget.value?.button === "ok" && editingTarget.value.trigger !== "double",
);

const okClickChecked = computed(() => actionsOf("ok").okContextClick === true);

function toggleOkContextClick(): void {
  if (busy.value || editingSource.value !== "common") return;
  const target = editingTarget.value;
  if (!target || target.button !== "ok") return;
  const actions = { ...actionsOf("ok") };
  if (actions.okContextClick) {
    delete actions.okContextClick;
  } else {
    actions.okContextClick = true;
  }
  mappings.value = {
    ...mappings.value,
    actions: { ...mappings.value.actions, [target.button]: actions },
  };
  reportFrontendEvent({ event: "button_mapping_ok_click", phase: "draft_changed", result: "passed", reason: actions.okContextClick ? "enabled" : "disabled" });
  statusMessage.value = (
    actions.okContextClick
      ? t("已开启：用遥控器移动光标后，按 OK 会点击光标位置")
      : t("已关闭：移动光标后按 OK 点击")
  );
}

/**
 * 动作变更后的「按住时连续执行」一致性收口：已选目标不再可用时自动回到
 * 「关闭」并返回提示（不偷改选择到另一个目标，保留用户的显式选择）。
 * 提示经 `persist(message)` 展示，避免被 persist 的清理逻辑覆盖。
 */
function reconcileHoldRepeat(
  actions: ButtonActions,
): { actions: ButtonActions; notice: string | null } {
  const hold = actions.holdRepeat;
  if (!hold || hold === "off") return { actions, notice: null };
  const slot = hold === "long" ? "long" : "single";
  if (holdRepeatBlockedReason(actions, slot) === null) return { actions, notice: null };
  const next = { ...actions };
  next.holdRepeat = "off";
  // 自动关闭的提示同样不得指向灰掉的选项（长按动作只执行一次时它不可选）。
  const notice =
    slot === "single" && actions.long.type !== "disabled"
      ? holdRepeatBlockedReason(actions, "long") === null
        ? t("这个按键已配置长按动作：按住会先执行长按，“按住时连续执行”已关闭（要连续执行，请选“重复长按动作”）")
        : t("这个按键已配置长按动作：按住会执行长按，而它只执行一次，“按住时连续执行”已关闭")
      : t("动作已变更，“按住时连续执行”已关闭");
  return { actions: next, notice };
}

function selectHoldRepeat(choice: HoldRepeatChoice): void {
  if (busy.value) return;
  const target = editingTarget.value;
  if (!target) return;
  const reason =
    choice === "single"
      ? repeatSingleReason.value
      : choice === "long"
        ? repeatLongReason.value
        : null;
  if (reason) {
    statusMessage.value = reason;
    return;
  }
  if (repeatChoice.value === choice) return;
  const actions = { ...actionsOf(target.button) };
  if (choice === "auto") {
    delete actions.holdRepeat;
  } else {
    actions.holdRepeat = choice;
  }
  mappings.value = {
    ...mappings.value,
    actions: { ...mappings.value.actions, [target.button]: actions },
  };
  reportFrontendEvent({ event: "button_mapping_repeat", phase: "draft_changed", result: "passed", reason: choice });
  statusMessage.value = (
    choice === "auto"
      ? "已选择自动：未设置长按时连续执行可重复的单击动作；设置长按后只执行一次"
      : choice === "off"
        ? "已关闭“按住时连续执行”"
        : choice === "long"
          ? "已开启：长按后不松手会继续连续执行"
          : "已开启：按住不放会连续执行"
  );
}


function applyFocusProfile(target: string, profile: AppFocusProfile): void {
  const editing = editingTarget.value;
  if (busy.value || editingSource.value !== "common" || !editing || menuReserved(editing.button)) return;
  const action = actionOf(editing.button, editing.trigger);
  if (action.type !== "open_app" || action.target !== target) return;
  mappings.value = { ...mappings.value, focusProfiles: { ...mappings.value.focusProfiles, [target]: profile } };
}

function applyAction(action: ButtonAction): void {
  if (busy.value) return;
  const target = editingTarget.value;
  if (!target || menuReserved(target.button)) return;
  const next: ButtonMappings = {
    ...mappings.value,
    actions: { ...mappings.value.actions },
  };
  const actions = { ...actionsOf(target.button) };
  actions[target.trigger] = action;
  const reconciled = reconcileHoldRepeat(actions);
  next.actions[target.button] = reconciled.actions;
  if (reconciled.notice) statusMessage.value = reconciled.notice;
  mappings.value = next;
}

async function persist(message?: string): Promise<boolean> {
  const request = ++saveSequence;
  const source = editingSource.value;
  const payload = cloneMappings(mappings.value);
  payload.schemaVersion ??= 5;
  busy.value = true;
  statusMessage.value = null;
  reportFrontendEvent({ event: "button_mapping_editor_save", phase: "started", result: "passed", reason: source === "common" ? "common" : "template" });
  try {
    const saved = source === "common"
      ? await saveButtonMappings(payload)
      : (await updateButtonMappingTemplate(source.slice("template:".length), payload)).mappings;
    if (request !== saveSequence) return true;
    if (configuration.value) {
      if (source === "common") {
        configuration.value = { ...configuration.value, commonMappings: saved };
      } else {
        const templateId = source.slice("template:".length);
        configuration.value = {
          ...configuration.value,
          templates: configuration.value.templates.map((template) =>
            template.id === templateId ? { ...template, mappings: saved } : template,
          ),
        };
      }
    }
    if (source !== "common") {
      const id = source.slice("template:".length);
      templateCatalog.value = templateCatalog.value.map(item => item.id === id ? {...item, buttonMappings: cloneMappings(saved)} : item);
      if (configuration.value && !configuration.value.templates.some(item => item.id === id)) {
        configuration.value.templates.push({id, name: templateCatalog.value.find(item => item.id === id)!.name, mappings: cloneMappings(saved)});
      }
    }
    retainedDrafts.value.delete(source);
    reportFrontendEvent({ event: "button_mapping_editor_save", phase: "completed", result: "passed", reason: source === "common" ? "common" : "template" });
    if (editingSource.value !== source) return true;
    mappings.value = cloneMappings(saved);
    savedSnapshot.value = cloneMappings(saved);
    if (message) {
      statusMessage.value = message;
    }
    return true;
  } catch (error) {
    statusMessage.value = error instanceof Error ? error.message : String(error);
    reportFrontendEvent({ event: "button_mapping_editor_save", phase: "completed", result: "failed", reason: source === "common" ? "common" : "template" });
    return false;
  } finally {
    if (request === saveSequence) busy.value = false;
  }
}

// 页面底部的快照错误只显示固定一句用户语言；原始错误留在控制台/后端日志。
watch(
  () => mappingSnapshot.value?.lastError,
  (raw) => {
    if (raw) console.warn("button mapping snapshot error", raw);
  },
);

async function restoreDefaults(): Promise<void> {
  if (busy.value) return;
  const source = editingSource.value;
  const template = activeTemplateEntry.value;
  if (template?.builtIn) {
    if (!window.confirm(`只复位“${template.name}”为当前版本默认按键？该模板修改和当前草稿将丢失，其它模板及关联保持不变。`)) return;
    busy.value = true;
    reportFrontendEvent({ event: "button_mapping_template_reset", phase: "started", result: "passed", reason: "builtin" });
    try {
      const reset = await resetBuiltinTemplate(template.id);
      if (unmounted) return;
      const catalog = await getTemplateCatalog();
      if (unmounted) return;
      const restored = catalog.find(item => item.id === template.id);
      if (!restored?.buttonMappings) throw new Error("复位后未能读回当前模板配置");
      configuration.value = {
        ...configuration.value!,
        templates: [
          ...configuration.value!.templates.filter(item => item.id !== template.id),
          ...reset.templates.filter(item => item.id === template.id),
        ],
      };
      templateCatalog.value = templateCatalog.value.map(item => item.id === template.id ? restored : item);
      retainedDrafts.value.delete(source);
      reportFrontendEvent({ event: "button_mapping_template_reset", phase: "completed", result: "passed", reason: editingSource.value === source ? "current_template" : "previous_template" });
      if (editingSource.value !== source) return;
      mappings.value = cloneMappings(restored.buttonMappings);
      savedSnapshot.value = cloneMappings(restored.buttonMappings);
      statusMessage.value = `已复位“${template.name}”`;
    } catch (error) {
      if (!unmounted && editingSource.value === source) statusMessage.value = String(error);
      reportFrontendEvent({ event: "button_mapping_template_reset", phase: "completed", result: "failed", reason: "reset_failed" });
    } finally {
      if (!unmounted) busy.value = false;
    }
    return;
  }
  mappings.value = { enabled: true, actions: {} };
  statusMessage.value = "已载入默认草稿；点击“保存配置”后写入当前编辑目标";
}

async function saveConfiguration(): Promise<void> {
  await persist(t("配置已保存并生效"));
}

function openSaveTemplateDialog(): void {
  if (busy.value) return;
  templateNameDraft.value = "";
  templateNameError.value = null;
  saveTemplateDialogOpen.value = true;
}

async function saveAsTemplate(): Promise<void> {
  const name = templateNameDraft.value.trim();
  if (!name) {
    templateNameError.value = "请输入模板名称";
    return;
  }
  busy.value = true;
  statusMessage.value = null;
  templateNameError.value = null;
  const started = performance.now();
  reportFrontendEvent({ event: "button_template_create", phase: "started", result: "passed", reason: editingSource.value === "common" ? "common_draft" : "template_draft" });
  try {
    const template = await saveButtonMappingTemplate(name, { ...cloneMappings(mappings.value), schemaVersion: mappings.value.schemaVersion ?? 5 });
    if (configuration.value) {
      configuration.value = {
        ...configuration.value,
        templates: [...configuration.value.templates, template],
      };
    }
    statusMessage.value = `已保存为按键模板“${name}”；可在“模板”页关联程序`;
    saveTemplateDialogOpen.value = false;
    reportFrontendEvent({ event: "button_template_create", phase: "completed", result: "passed", reason: editingSource.value === "common" ? "common_draft" : "template_draft", elapsedMs: Math.round(performance.now() - started) });
  } catch (error) {
    templateNameError.value = error instanceof Error ? error.message : String(error);
    reportFrontendEvent({ event: "button_template_create", phase: "completed", result: "failed", reason: editingSource.value === "common" ? "common_draft" : "template_draft", elapsedMs: Math.round(performance.now() - started) });
  } finally {
    busy.value = false;
  }
}

async function exportConfiguration(): Promise<void> {
  if (busy.value) return;
  busy.value = true;
  try {
    const ids = editingSource.value === "common" ? null : [editingSource.value.slice("template:".length)];
    if (await exportMappingConfiguration(ids)) statusMessage.value = "配置已导出";
  } catch (error) { statusMessage.value = error instanceof Error ? error.message : String(error); }
  finally { busy.value = false; }
}



function phaseLabel(phase: RawInputPhase | undefined): string {
  switch (phase) {
    case "ready":
      return t("按键监听已就绪");
    case "starting":
      return t("正在启动监听");
    case "failed":
      return t("监听启动失败（自动重试中）");
    case "stopped":
      return t("监听已停止");
    case "awaiting":
      return t("等待遥控器连接");
    case "unsupported":
      return t("当前环境暂不支持");
    default:
      return t("正在读取状态");
  }
}

/** 手动启停监听：自动启动之外保留显式控制（停止后自动重试不生效）。 */
async function toggleListener(): Promise<void> {
  busy.value = true;
  statusMessage.value = null;
  try {
    if (rawInput.value?.phase === "ready") {
      await stopRawInput();
      statusMessage.value = t("监听已停止；映射与高亮暂停（按住说话不受影响）");
    } else {
      await startRawInput();
      statusMessage.value = t("监听已启动");
    }
  } catch (error) {
    statusMessage.value = operationErrorText(
      "toggle raw input listener failed",
      error,
      "没能切换按键监听状态，请重试。",
    );
  } finally {
    busy.value = false;
  }
}

const rawInput = computed(() => props.runtime?.platform.rawInput);
const connectionInfo = computed(() => props.runtime?.platform.connection);
const remoteConnectionActive = computed(() => connectionInfo.value !== undefined && [
  'connecting', 'discovering', 'awaiting_capabilities', 'ready',
  'streaming', 'draining', 'reconnecting', 'suspended',
].includes(connectionInfo.value.phase));
const disconnectBusy = ref(false);
const connectionFeedback = ref('');
async function changeRemoteConnection() {
  if (disconnectBusy.value) return;
  if (!remoteConnectionActive.value) { emit('navigate', 'connection'); return; }
  disconnectBusy.value = true;
  connectionFeedback.value = '';
  reportFrontendEvent({event: 'buttons_remote_disconnect', phase: 'requested', result: 'unknown', reason: 'explicit_user_request'});
  try {
    const actual = await disconnectRemote();
    const released = actual.phase === 'disconnected';
    connectionFeedback.value = released ? '遥控器连接已释放，本次运行已停止自动重连' : '断开请求已提交，正在等待连接释放';
    reportFrontendEvent({event: 'buttons_remote_disconnect', phase: 'completed', result: released ? 'passed' : 'unknown', reason: released ? 'released' : 'awaiting_release'});
  } catch {
    connectionFeedback.value = '未能断开遥控器，请到连接与语音页查看状态后重试。';
    reportFrontendEvent({event: 'buttons_remote_disconnect', phase: 'completed', result: 'failed', reason: 'request_failed'});
  } finally { disconnectBusy.value = false; }
}

function refreshCanvasGeometry(width?: number): void {
  if (width !== undefined) canvasWidth.value = Math.max(CANVAS_MIN_WIDTH, Math.round(width));
  void nextTick(measureRemoteImageFrame);
}

onMounted(async () => {
  const setupStarted = performance.now();
  // Measure before the initial IPC resolves so the first frame uses the real canvas width.
  if (canvasEl.value) refreshCanvasGeometry(canvasEl.value.clientWidth);
  const [loaded, snapshot, apps, catalog] = await Promise.all([
    readMenuConfiguration(),
    getButtonMappingSnapshot(),
    listPresetApps().catch(() => [] as PresetAppInfo[]),
    getTemplateCatalog(),
  ]);
  if (unmounted) {
    return;
  }
  presetApps.value = apps.filter((app) => app.installed);
  registerPresetAppNames(presetApps.value);
  configuration.value = loaded;
  templateCatalog.value = catalog;
  loadEditingSource(navigationEditingSource.value ?? (currentTemplate.templateId.value === null ? "common" : `template:${currentTemplate.templateId.value}`));
  mappingSnapshot.value = snapshot;
  activeButtons.value = new Set(snapshot.observedButtons);
  if (props.initialEditor) openRequestedEditor(props.initialEditor);

  const stopEdges = await subscribeButtonEdges((edge: ButtonEdge) => {
    const next = new Set(activeButtons.value);
    if (edge.isPressed) {
      next.add(edge.button);
    } else {
      next.delete(edge.button);
    }
    activeButtons.value = next;
    if (!lockSelection.value && edge.isPressed && !menuReserved(edge.button)) {
      selectedButton.value = edge.button;
    }
  });
  if (unmounted) {
    stopEdges();
    return;
  }
  unlistenEdges = stopEdges;

  const stopGestures = await subscribeButtonGestures((gesture: FiredGesture) => {
    lastFired.value = gesture;
    firedFlash.value = { button: gesture.button, trigger: gesture.trigger };
    if (flashTimer !== null) window.clearTimeout(flashTimer);
    flashTimer = window.setTimeout(() => {
      firedFlash.value = null;
    }, 600);
  });
  if (unmounted) {
    stopGestures();
    return;
  }
  unlistenGestures = stopGestures;

  snapshotTimer = window.setInterval(async () => {
    if (snapshotPending || unmounted) return;
    snapshotPending = true;
    try {
      const [snapshot] = await Promise.all([getButtonMappingSnapshot(), readMenuConfiguration()]);
      if (unmounted) return;
      mappingSnapshot.value = snapshot;
      // Update only live state, never replace the user's editing draft.
      activeButtons.value = new Set(snapshot.observedButtons);
    } finally { snapshotPending = false; }
  }, 1_000);

  // 流式画布：观测容器宽（不足最小画布 800px 时保持 800 由 CSS 缩放兜底）。
  // 首次宽度已在 onMounted 同步段测过（见函数开头），此处只订阅后续变化。
  // jsdom 测试环境无 ResizeObserver，跳过观测。
  if (canvasEl.value && typeof ResizeObserver !== "undefined") {
    refreshCanvasGeometry(canvasEl.value.clientWidth);
    resizeObserver = new ResizeObserver((entries) => {
      for (const entry of entries) {
        if (entry.target === canvasEl.value) refreshCanvasGeometry(entry.contentRect.width);
      }
      void nextTick(measureRemoteImageFrame);
    });
    resizeObserver.observe(canvasEl.value);
    if (remotePhotoEl.value) resizeObserver.observe(remotePhotoEl.value);
  } else {
    measureRemoteImageFrame();
  }
  resourcesReady = true;
  reportFrontendEvent({
    event: "buttons_page_resource_setup",
    phase: "completed",
    result: "passed",
    reason: "listeners_and_polling_ready",
    elapsedMs: Math.max(0, Math.round(performance.now() - setupStarted)),
  });
});

onUnmounted(() => {
  unmounted = true;
  releasePageResources();
  reportFrontendEvent({
    event: "buttons_page_resource_cleanup",
    phase: "completed",
    result: "passed",
    reason: resourcesReady ? "unmounted_after_cleanup" : "unmounted_before_setup_completed",
  });
});
</script>

<template>
  <section class="buttons-page">
    <div class="buttons-scroll">
    <header class="page-header mapping-header">
      <ComponentSupportPanel>
      <template #prefix>
        <h1>按键映射</h1>
        <label class="toggle-row mapping-toggle-slot" title="开启后，遥控器按键按本页配置执行动作；关闭时，遥控器保持原始按键行为。">
          <span>启用自定义按键功能</span>
          <input v-model="enabled" type="checkbox" class="toggle-input" :disabled="editingBusy" />
        </label>
      </template>
      <template #device>
        <div class="device-chip" :class="{ connected: connectionInfo?.phase === 'ready' || connectionInfo?.phase === 'streaming' }">
          <strong class="remote-name">{{ deviceLabel }}</strong>
          <div class="remote-status">
          <span class="status-dot" :class="connectionInfo?.phase === 'streaming' ? 'active' : connectionInfo?.phase === 'ready' ? 'success' : 'pending'"></span>
          <span>{{ connectionInfo ? connectionPhaseLabel(connectionInfo.phase) : '未连接' }}</span>
          <BatteryIndicator :connection="connectionInfo" />
          </div>
          <button type="button" class="secondary-button remote-connection-action" :disabled="disconnectBusy" @click="changeRemoteConnection">{{ disconnectBusy ? '正在断开…' : remoteConnectionActive ? '断开' : '重新连接' }}</button>
        </div>
      </template>
        <div class="current-template-display">
          <span>当前模板</span>
          <select aria-label="当前使用的按键模板" :value="!currentTemplate.applied.value ? '' : currentTemplate.templateId.value ?? 'common'" :disabled="busy || currentTemplate.applying.value || !currentTemplate.applied.value" :aria-busy="currentTemplate.applying.value" @pointerdown="refreshTemplateChoices" @focus="refreshTemplateChoices" @change="selectActiveTemplate">
            <option value="" disabled>正在确认当前模板…</option>
            <option v-for="option in activeTemplateOptions" :key="option.value" :value="option.value">{{ option.label }}</option>
          </select>
        </div>


      <template #feedback>
      <p v-if="templateRefreshError" class="error-text" role="status">{{ templateRefreshError }}</p>
      <p class="remote-connection-feedback" role="status" aria-live="polite">{{ connectionFeedback }}</p>
      <p v-if="currentTemplate.error.value" class="error-text" role="status">{{ currentTemplate.error.value }}</p>
      <p v-if="currentTemplate.persistenceMessage.value" class="mapping-header-status muted" role="status" aria-live="polite">{{ currentTemplate.persistenceMessage.value }}</p>
      <p v-if="retainedDrafts.size" class="muted" role="status">已在本页保留其他模板的未保存草稿；切回可继续编辑，保存后才会生效。</p>
      <p v-if="voiceFeedback" class="voice-feedback" role="status">{{ voiceFeedback }}</p>
      </template>
      </ComponentSupportPanel>
    </header>

    <div id="global-mapping-panel">
    <div ref="canvasEl" class="mapping-canvas" :style="{ height: `${CANVAS_HEIGHT}px`, '--mapping-canvas-height': `${CANVAS_HEIGHT}px`, '--mapping-card-height': `${CARD_HEIGHT}px` }">
      <svg
        class="mapping-connections"
        :width="canvasWidth"
        :height="CANVAS_HEIGHT"
        aria-hidden="true"
      >
        <template v-for="placement in PLACEMENTS" :key="placement.button">
          <path
            :d="connectionPath(placement)"
            :class="{ selected: selectedButton === placement.button, active: activeButtons.has(placement.button) }"
            fill="none"
          />
          <polygon
            :points="arrowPolygon(placement)"
            :class="{ selected: selectedButton === placement.button, active: activeButtons.has(placement.button) }"
          />
        </template>
        <path
          :d="connectionPath(VOICE_PLACEMENT)"
          :class="{ active: voiceButtonPressed }"
          fill="none"
        />
        <polygon :points="arrowPolygon(VOICE_PLACEMENT)" :class="{ active: voiceButtonPressed }" />
      </svg>

      <figure ref="remotePhotoEl" class="remote-photo" :style="{ left: `${remoteLeft}px`, top: `${REMOTE_TOP}px` }">
        <img ref="remoteImageEl" src="/RC003-remote-photo@2x.webp" alt="小米蓝牙语音遥控器 2 Pro（RC003）示意图" draggable="false" @load="measureRemoteImageFrame" />
      </figure>
      <span
        v-for="placement in PLACEMENTS"
        :key="`anchor-${placement.button}`"
        class="anchor-dot"
        :class="{ visible: activeButtons.has(placement.button) }"
        :style="{
          left: `${anchorPoint(placement).x - 4}px`,
          top: `${anchorPoint(placement).y - 4}px`,
        }"
      ></span>
      <span
        class="anchor-dot voice"
        :class="{ visible: voiceButtonPressed }"
        :style="{
          left: `${anchorPoint(VOICE_PLACEMENT).x - 4}px`,
          top: `${anchorPoint(VOICE_PLACEMENT).y - 4}px`,
        }"
      ></span>

      <article
        v-for="placement in PLACEMENTS"
        :key="placement.button"
        class="mapping-card"
        :class="{
          left: placement.side === 'left',
          right: placement.side === 'right',
          selected: selectedButton === placement.button,
          active: activeButtons.has(placement.button),
          flashed: firedFlash?.button === placement.button,
          'menu-reserved': menuReserved(placement.button),
          'mapping-readonly': menuReserved(placement.button),
        }"
        :style="{ top: `${cardTop(placement)}px`, width: `${cardWidth}px` }"
        @click="selectButton(placement.button)"
      >
        <div class="mapping-card-title">
          <svg class="mapping-icon" viewBox="0 0 24 24" fill="none" aria-hidden="true">
            <path
              v-for="(path, index) in buttonIcons[placement.button]"
              :key="index"
              :d="path"
              stroke="currentColor"
              stroke-width="1.9"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
          </svg>
          <strong>{{ buttonLabels[placement.button] }}</strong>
          <label v-if="placement.button === 'menu'" class="menu-mode-control" @click.stop @keydown.stop @keyup.stop>
            <input type="checkbox" aria-label="菜单键切换模板" aria-describedby="menu-mode-tooltip menu-mode-feedback" :checked="menuTemplateSwitchEnabled === true" :disabled="busy || menuModePending || menuTemplateSwitchEnabled === null || !configuration" :aria-busy="menuModePending" @change.stop="toggleMenuMode" />
            <span class="menu-mode-lock">
              <svg v-if="menuTemplateSwitchEnabled === true" class="readonly-icon" viewBox="0 0 24 24" fill="none" aria-label="固定功能，不可自定义">
                <path :d="READONLY_ICON" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" />
              </svg>
            </span>
            <span id="menu-mode-tooltip" class="menu-mode-tooltip" role="tooltip">启用后，菜单键用于切换模板；关闭后可自定义。</span>
          </label>
          <span v-if="menuReserved(placement.button)" class="menu-reserved-label">{{ menuStateLabel }}</span>
        </div>
        <div class="mapping-cells" :aria-disabled="menuReserved(placement.button) ? true : undefined">
          <button
            v-for="trigger in TRIGGERS"
            :key="trigger"
            type="button"
            class="mapping-cell"
            :class="{
              set: !menuReserved(placement.button) && actionOf(placement.button, trigger).type !== 'disabled',
              editing:
                editingTarget?.button === placement.button && editingTarget?.trigger === trigger,
              flashed: firedFlash?.button === placement.button && firedFlash?.trigger === trigger,
            }"
            :disabled="editingBusy || menuReserved(placement.button)"
            :title="
              menuReserved(placement.button)
                ? menuTemplateSwitchEnabled === true ? menuBehaviorDetail[trigger] : menuStateLabel
                : `${buttonLabels[placement.button]} · ${buttonTriggerLabel(trigger)}：${actionSummary(actionOf(placement.button, trigger))}`
            "
            @click.stop="openEditor(placement.button, trigger)"
          >
            <small>{{ buttonTriggerLabel(trigger) }}</small>
            <span>{{ cellSummary(placement.button, trigger) }}</span>
          </button>
        </div>
      </article>

      <article
        class="mapping-card mapping-readonly voice-card center"
        aria-disabled="true"
        :class="{ active: voiceButtonPressed }"
        :style="{ top: `${VOICE_CARD_TOP}px`, width: `${cardWidth}px` }"
      >
        <div class="mapping-card-title">
          <svg class="mapping-icon" viewBox="0 0 24 24" fill="none" aria-hidden="true">
            <path :d="VOICE_ICON_FILLED" fill="currentColor" />
            <path
              v-for="(path, index) in VOICE_ICON_STROKES"
              :key="index"
              :d="path"
              stroke="currentColor"
              stroke-width="1.9"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
          </svg>
          <strong>语音键</strong>
          <svg class="readonly-icon" viewBox="0 0 24 24" fill="none" aria-label="固定功能，不可自定义">
            <path :d="READONLY_ICON" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" />
          </svg>
          <span class="badge pending voice-badge" :class="{ active: voiceButtonPressed }">{{ voiceActive ? '语音进行中' : voiceButtonPressed ? '已收到按键' : '按住说话' }}</span>
        </div>
        <p class="voice-note">{{ t("按下开始、松开结束；不参与自定义映射，不加双击/长按延迟。") }}</p>
      </article>
    </div>

    <div id="menu-mode-feedback" class="menu-mode-feedback" aria-live="polite">
      <span v-if="menuModePending" class="muted">正在保存…</span>
      <span v-else-if="menuModeError" class="error-text" role="alert">{{ menuModeError }}</span>
    </div>
    <p v-if="menuStateReadFailed" class="menu-reserved-note" role="alert">未能读取菜单功能，暂不可编辑菜单键；正在重新读取，已有映射未更改。</p>

    <article v-if="editingTarget" ref="editorPanel" class="card mapping-editor">
      <div class="card-title-row">
        <div>
          <h2>{{ buttonLabel(editingTarget.button) }} · {{ buttonTriggerLabel(editingTarget.trigger) }}</h2>
          <p class="muted">{{ t("当前：{summary}", { summary: actionSummary(actionOf(editingTarget.button, editingTarget.trigger)) }) }}</p>
        </div>
        <div class="button-row">
          <button
            class="secondary-button editor-disable-btn"
            :class="{ 'is-active': actionOf(editingTarget.button, editingTarget.trigger).type === 'disabled' }"
            type="button"
            :disabled="editingBusy"
            :title="t('只禁用当前格子的映射，此按键恢复原始行为')"
            @click="applyAction({ type: 'disabled' })"
          >
            {{ t("禁用按键") }}
          </button>
          <button class="secondary-button" type="button" :disabled="editingBusy" @click="editingTarget = null">关闭</button>
        </div>
      </div>
      <ButtonActionEditor
        @navigate="emit('navigate', $event)"
        :key="`${editingSource}:${editingTarget.button}:${editingTarget.trigger}`"
        :shortcuts-only="editingSource !== 'common'"
        :mappings="mappings"
        :saved-mappings="savedSnapshot"
        :busy="editingBusy"
        :button="editingTarget.button"
        :trigger="editingTarget.trigger"
        :preset-apps="presetApps"
        @update="applyAction"
        @focus-profile="applyFocusProfile"
        @applications="mappings = { ...mappings, applications: $event }"
        @status="statusMessage = $event"
      />
      <div v-if="repeatGroupVisible" class="repeat-group">
        <span class="repeat-label">按住时连续执行</span>
        <div class="repeat-options">
          <button
            class="chip"
            :class="{ selected: repeatChoice === 'auto' }"
            type="button"
            title="未设置长按动作时，连续执行可重复的单击动作；设置长按后只执行一次"
            :disabled="editingBusy"
            @click="selectHoldRepeat('auto')"
          >
            自动（默认）
          </button>
          <button
            class="chip"
            :class="{ selected: repeatChoice === 'off' }"
            type="button"
            title="按住不放只执行一次"
            :disabled="editingBusy"
            @click="selectHoldRepeat('off')"
          >
            关闭
          </button>
          <button
            class="chip"
            :class="{ selected: repeatChoice === 'single' }"
            type="button"
            :title="repeatSingleTitle"
            :disabled="editingBusy || (!!repeatSingleReason && repeatChoice !== 'single')"
            @click="selectHoldRepeat('single')"
          >
            重复单击动作
          </button>
          <button
            class="chip"
            :class="{ selected: repeatChoice === 'long' }"
            type="button"
            :title="repeatLongTitle"
            :disabled="editingBusy || (!!repeatLongReason && repeatChoice !== 'long')"
            @click="selectHoldRepeat('long')"
          >
            重复长按动作
          </button>
        </div>
        <p v-if="repeatChoice === 'auto'" class="muted repeat-hint repeat-auto-hint">{{ repeatAutoHint }}</p>
        <p v-if="repeatPageReason" class="muted repeat-hint">{{ repeatPageReason }}</p>
      </div>
      <label v-if="okClickVisible" class="toggle-row ok-click-toggle" :title="t('刚用遥控器移动过光标时，按 OK 会点击光标位置')">
        <span>{{ t("移动光标后按 OK 点击") }}</span>
        <input
          class="toggle-input"
          type="checkbox"
          :checked="okClickChecked"
          :disabled="editingBusy"
          @change="toggleOkContextClick"
        />
      </label>
      <p v-if="okClickVisible" class="muted ok-click-hint">
        刚用遥控器移动过光标（5 秒内），按 OK 会点击光标位置。这时 OK 的单击、双击、长按都不会执行。
      </p>
      <p v-if="editingTarget.trigger === 'single'" class="muted editor-note">
        {{ doubleConfiguredOnEditingButton ? "配置了双击：单击会稍等片刻（约 0.3 秒）以区分双击。" : "单击在按下瞬间执行。" }}
      </p>
      <p v-else class="muted editor-note">
        {{ editingTarget.trigger === "double" ? "双击判定窗口约 0.3 秒：配置后单击会稍等片刻以区分双击。" : "长按约 0.55 秒后执行。" }}
      </p>
    </article>

    <p class="muted mapping-observation-hint">按下高亮只表示已收到按键；动作按当前程序的模板执行，未配置则不执行。</p>
    </div>
    </div>

    <footer class="mapping-actions" aria-label="按键配置操作">
      <div class="mapping-listener">
        <div class="mapping-listener-controls">
          <label class="toggle-row" title="开启后，操作实体遥控器不会切换正在编辑的按键。">
            <span>锁定当前按键</span>
            <input ref="selectionToggle" :checked="lockSelection" :aria-busy="selectionPreference.pending.value" :aria-disabled="!selectionPreference.loaded.value || selectionPreference.pending.value" @change="selectionPreference.toggleCheckbox" type="checkbox" class="toggle-input" />
          </label>
          <div class="mapping-listener-button-slot">
            <button
              v-if="rawInput?.phase === 'ready' || rawInput?.phase === 'stopped' || rawInput?.phase === 'failed'"
              class="secondary-button footer-listener-toggle"
              type="button"
              :disabled="busy"
              @click="toggleListener"
            >
              {{ rawInput?.phase === "ready" ? t("停止监听") : t("启动监听") }}
            </button>
          </div>
        </div>
        <div class="mapping-footer-status" :role="selectionPreference.error.value || mappingSnapshot?.lastError ? 'alert' : 'status'">
          <button v-if="footerFeedback.length" class="mapping-feedback-summary" type="button" aria-haspopup="dialog" @click="openFooterFeedback">
            <span>{{ footerFeedback.join(" · ") }}</span><span class="mapping-feedback-link">详情</span>
          </button>
          <template v-else>
            <span class="status-dot" :class="rawInput?.phase === 'ready' ? 'success' : 'pending'"></span>
            <span class="mapping-phase-label" :title="phaseLabel(rawInput?.phase)">{{ phaseLabel(rawInput?.phase) }}</span>
          </template>
        </div>
      </div>
      <div class="mapping-save-actions">
        <div class="button-row">
          <button class="secondary-button" type="button" :disabled="editingBusy" @click="saveConfiguration">
            保存当前配置
          </button>
          <button class="secondary-button" type="button" :disabled="editingBusy" @click="openSaveTemplateDialog">
            保存为模板
          </button>
          <button class="secondary-button" type="button" :disabled="editingBusy" @click="exportConfiguration">
            {{ t("导出配置…") }}
          </button>
          <button class="secondary-button" type="button" :disabled="editingBusy" @click="restoreDefaults">
            {{ activeTemplateEntry?.builtIn ? "复位模板" : "恢复默认" }}
          </button>
        </div>
      </div>
    </footer>
  </section>

  <SettingsDialog v-if="footerFeedbackOpen" title="按键操作反馈" @close="closeFooterFeedback" @keydown.tab="trapFooterFeedbackFocus">
    <div class="mapping-feedback-details" tabindex="0" aria-label="完整操作反馈">
      <p v-for="(message, index) in displayedFooterFeedback" :key="index">{{ message }}</p>
    </div>
  </SettingsDialog>

  <SettingsDialog
    v-if="saveTemplateDialogOpen"
    title="保存为完整按键模板"
    description="保存当前编辑目标的草稿副本；不会开启自动切换或创建程序关联。"
    :busy="busy"
    @close="saveTemplateDialogOpen = false"
  >
    <label class="save-template-field">模板名称<input v-model="templateNameDraft" :disabled="editingBusy" @keyup.enter="saveAsTemplate" /></label>
    <p v-if="templateNameError" class="error-text">{{ templateNameError }}</p>
    <template #actions>
      <button type="button" :disabled="editingBusy" @click="saveAsTemplate">{{ busy ? "正在保存…" : "保存模板" }}</button>
      <button type="button" class="secondary-button" :disabled="editingBusy" @click="saveTemplateDialogOpen = false">取消</button>
    </template>
  </SettingsDialog>

</template>

<style scoped>
.mapping-canvas {
  --map-scale: min(1, (100vw - 152px) / 800px, 100vh / var(--mapping-canvas-height));
  margin-bottom: calc(-1 * var(--mapping-canvas-height) * (1 - var(--map-scale)));
}
.mapping-header p { margin: 0; }
.remote-connection-feedback:empty { display: none; }
.mapping-card { border-width: 2px; padding: 5px 8px; }
.mapping-card.mapping-readonly { cursor: default; border-color: transparent; outline: none; box-shadow: none; }
.mapping-card.mapping-readonly:not(.active):not(.flashed) { background: var(--pending-surface); }
.mapping-card.mapping-readonly.flashed { background: var(--pressed-surface); }
.mapping-readonly .mapping-card-title strong,
.mapping-readonly .mapping-icon { color: var(--text-secondary); }
.readonly-icon { width: 12px; height: 12px; flex: 0 0 auto; color: var(--text-secondary); }
.menu-reserved-label { margin-left: auto; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--text-secondary); font-size: 11px; }
.menu-reserved .mapping-cell:disabled { opacity: 1; cursor: default; border-radius: 0; }
.menu-reserved .mapping-cell:disabled:not(.flashed) { background: transparent; }
.menu-reserved .mapping-cell span { color: var(--text-secondary); font-size: 11px; }
.menu-reserved .mapping-cell:last-child span { font-weight: 600; }
.menu-reserved .mapping-cell:disabled:hover { box-shadow: none; }
.menu-mode-control { position: relative; display: inline-flex; flex: 0 0 auto; align-items: center; gap: 4px; cursor: pointer; }
.menu-mode-control input { width: 18px; height: 18px; margin: 0; accent-color: var(--accent); }
.menu-mode-lock { display: inline-flex; align-items: center; width: 12px; height: 18px; }
.menu-mode-tooltip { position: absolute; z-index: 5; bottom: calc(100% + 6px); left: 0; width: 220px; max-width: calc(100vw - 80px); padding: 8px 10px; border-radius: 6px; background: var(--surface-control); color: var(--text-primary); border: 1px solid var(--border-strong); font-size: 12px; font-weight: 400; line-height: 1.5; visibility: hidden; pointer-events: none; }
.menu-mode-control:hover .menu-mode-tooltip,
.menu-mode-control:focus-within .menu-mode-tooltip { visibility: visible; }
.menu-mode-feedback { margin-top: 10px; min-height: 1.5em; font-size: 13px; line-height: 1.5; }
.menu-reserved-note { margin: 10px 0 12px; line-height: 1.5; font-size: 13px; }
.buttons-page {
  flex: 1 1 0;
  min-height: 0;
  min-width: 0;
  display: grid;
  grid-template-rows: minmax(0, 1fr) auto;
}
.buttons-scroll {
  min-height: 0;
  min-width: 0;
  overflow: auto;
  padding: 0 var(--content-inline-padding) 16px;
  scrollbar-gutter: stable;
}
.mapping-actions {
  display: flex;
  align-items: center;
  gap: 16px;
  min-width: 0;
  overflow: hidden;
  scrollbar-gutter: stable;
  padding: 8px var(--content-inline-padding) 12px;
  border-top: 1px solid var(--border);
  background: var(--surface-canvas);
}
.mapping-listener { display: flex; align-items: center; gap: 10px; flex: 1 1 0; min-width: 0; }
.mapping-listener-controls { display: flex; align-items: center; gap: 12px; min-height: 32px; }
.mapping-listener-controls .toggle-row { margin: 0; gap: 8px; white-space: nowrap; }
.mapping-listener-button-slot { width: 82px; min-height: 32px; flex: 0 0 auto; }
.mapping-listener-button-slot button { width: 100%; min-height: 32px; padding: 4px 8px; font-size: 12px; }
.mapping-footer-status { display: flex; align-items: center; gap: 7px; flex: 1 1 0; min-width: 0; height: 32px; font-size: 12px; line-height: 16px; }
.mapping-footer-status .status-dot { flex: 0 0 auto; }
.mapping-phase-label { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.mapping-feedback-summary { display: flex; align-items: center; gap: 6px; width: 100%; min-width: 0; min-height: 32px; padding: 0; border: 0; background: transparent; color: var(--accent-text); font-size: 12px; text-align: left; }
.mapping-feedback-summary > span:first-child { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.mapping-feedback-link { flex: 0 0 auto; text-decoration: underline; }
.mapping-feedback-details { max-height: min(440px, 55vh); overflow: auto; overflow-wrap: anywhere; }
.mapping-feedback-details p { margin: 0; }
.mapping-feedback-details p + p { margin-top: 8px; }
.mapping-feedback-details:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
.mapping-save-actions { flex: 0 0 auto; min-width: 0; }
.mapping-observation-hint { margin: 10px 0 0; font-size: 12.5px; }
.mapping-actions .button-row { justify-content: flex-end; flex-wrap: nowrap; }
.mapping-actions button { max-width: 100%; white-space: nowrap; }
@media (max-width: 800px) {
  .mapping-actions { flex-wrap: wrap; gap: 8px; }
  .mapping-listener, .mapping-save-actions { flex-basis: 100%; }
  .mapping-actions .button-row { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); }
  .mapping-actions .button-row button { min-height: 36px; }
}
.current-template-display {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
  max-width: 100%;
}
.current-template-display > span {
  font-size: 13px;
  white-space: nowrap;
}
.current-template-display select { min-height: 30px; min-width: 0; width: 140px; max-width: 100%; text-overflow: ellipsis; font-size: 12px; padding: 4px 6px; }
.save-template-field { display: grid; gap: 7px; font-size: 13px; font-weight: 600; }
.mapping-header {
  display: block;
  margin-bottom: 0;
}
.mapping-header h1 { white-space: nowrap; margin: 0; font-size: 25px; }
.mapping-toggle-slot { flex: 0 0 auto; gap: 6px; font-size: 12px; white-space: nowrap; }
.mapping-toggle-slot.unavailable { color: var(--text-muted); }
.mapping-header .device-chip {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  grid-template-rows: auto auto;
  align-items: center;
  gap: 3px 10px;
  margin-left: auto;
  width: 240px;
  max-width: 100%;
  min-width: 0;
  padding: 8px 10px;
  border-radius: 12px;
  white-space: normal;
}
.remote-name { grid-column: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 12px; }
.remote-status { grid-row: 2; display: flex; align-items: center; flex-wrap: wrap; gap: 5px; font-size: 11px; }
.remote-connection-action { grid-column: 2; grid-row: 1 / 3; padding: 5px 9px; min-height: 30px; font-size: 12px; }
.mapping-header-status, .voice-feedback { margin: 0; font-size: 12px; line-height: 1.6; }
@media (max-width: 900px) { .mapping-header .device-chip { width: 220px; } }
.repeat-group { display: flex; align-items: center; flex-wrap: wrap; gap: 8px; margin-top: 12px; }
.repeat-label { font-size: 13px; }
.repeat-options { display: flex; flex-wrap: wrap; gap: 8px; }
.repeat-hint { flex-basis: 100%; margin: 0; font-size: 12px; }
/* OK 键「移动光标后按 OK 点击」：按键级开关 + 常驻说明句（2026-10-06）。 */
.ok-click-toggle { margin-top: 12px; }
.ok-click-hint { margin: 6px 0 0; font-size: 12px; }
</style>
