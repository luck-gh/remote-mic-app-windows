<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { useCurrentTemplate } from "../lib/current-template";
import ButtonActionEditor from "../components/ButtonActionEditor.vue";
import BatteryIndicator from "../components/BatteryIndicator.vue";
import SettingsDialog from "../components/SettingsDialog.vue";
import { reportFrontendEvent } from "../lib/frontend-diagnostics";
import { useUiPreference } from "../lib/ui-preferences";
import {
  actionSummary,
  buttonLabel,
  buttonLabels,
  buttonTriggerLabel,
  chordLabel,
  exportMappingConfiguration,
  getButtonMappingSnapshot,
  getMappingConfiguration,
  getTemplateCatalog,
  listPresetApps,
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
  type ButtonEdge,
  type ButtonMappingSnapshot,
  type ButtonMappings,
  type MappingConfiguration,
  type TemplateCatalogEntry,
  type ButtonTrigger,
  type FiredGesture,
  type PresetAppInfo,
  type RawInputPhase,
  type RemoteButton,
  type RemoteModel,
  type RuntimeSnapshot,
} from "../lib/bridge";

const props = defineProps<{ runtime: RuntimeSnapshot | null }>();
const READONLY_ICON = "M7 10V7a5 5 0 0 1 10 0v3M5 10h14v11H5zM12 14v3";

/** 画布几何：对齐 Mac RemoteMappingCanvas——高度固定 640，宽度流式
 * （占满容器，ResizeObserver 观测）；卡宽 = clamp((宽-260)/2, 270, 300)，
 * 与 Mac cardWidth(for:) 同公式；容器不足最小画布 800px 时由 CSS
 * --map-scale 连续缩放兜底（小于最小窗口的恢复态窗口）。 */
const CANVAS_MIN_WIDTH = 800;
const CANVAS_HEIGHT = 640;
const REMOTE_WIDTH = 202;
const REMOTE_HEIGHT = 410;
const CARD_HEIGHT = 72;
const REMOTE_TOP = (CANVAS_HEIGHT - REMOTE_HEIGHT) / 2;
const VOICE_CARD_TOP = 8;
const CARD_ROW_GAP = 16;
const ORDINARY_ROWS_TOP = VOICE_CARD_TOP + CARD_HEIGHT + CARD_ROW_GAP;

/** 语音独占顶部一行；普通六行在其下等距排列，卡片均使用同一尺寸。 */
function rowTarget(row: number): number {
  const step = (CANVAS_HEIGHT - VOICE_CARD_TOP - ORDINARY_ROWS_TOP - CARD_HEIGHT) / 5;
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
  return connectionInfo.value?.remoteName ?? "未连接遥控器";
});

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

const mappings = ref<ButtonMappings>({ enabled: true, actions: {} });
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
/** 已安装的预设应用（打开应用动作可选列表）。 */
const presetApps = ref<PresetAppInfo[]>([]);
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

const activeTemplateEntry = computed(() => editingSource.value === "common" ? null : templateCatalog.value.find(t => t.id === editingSource.value.slice("template:".length)) ?? null);
const dirty = computed(() => JSON.stringify(mappings.value) !== JSON.stringify(savedSnapshot.value));

const enabled = computed({
  get: () => mappings.value.enabled,
  set: (value: boolean) => {
    mappings.value = { ...mappings.value, enabled: value };
  },
});

const editingSourceOptions = computed(() => [
  { value: "common" as EditingSource, label: "通用配置" },
  ...templateCatalog.value.map((entry) => ({
    value: `template:${entry.id}` as EditingSource,
    label: entry.name,
  })),
]);

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
  if (!selected) {
    editingSource.value = "common";
    const common = configuration.value?.commonMappings ?? { enabled: true, actions: {} };
    mappings.value = cloneMappings(common);
    savedSnapshot.value = cloneMappings(common);
    statusMessage.value = "所选模板已不存在，已返回通用配置";
    return false;
  }
  editingSource.value = source;
  mappings.value = cloneMappings(selected);
  savedSnapshot.value = cloneMappings(selected);
  editingTarget.value = null;
  return true;
}

const voiceActive = computed(
  () => props.runtime?.platform.connection.voiceState === "streaming",
);

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

function selectButton(button: RemoteButton): void {
  if (busy.value || menuReserved(button)) return;
  selectedButton.value = button;
}

function openEditor(button: RemoteButton, trigger: ButtonTrigger): void {
  if (busy.value || menuReserved(button)) return;
  selectedButton.value = button;
  editingTarget.value = { button, trigger };
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
  next.actions[target.button] = actions;
  mappings.value = next;
}

async function persist(message?: string): Promise<boolean> {
  const request = ++saveSequence;
  const source = editingSource.value;
  const payload = cloneMappings(mappings.value);
  busy.value = true;
  statusMessage.value = null;
  reportFrontendEvent({ event: "button_mapping_editor_save", phase: "started", result: "passed", reason: source === "common" ? "common" : "template" });
  try {
    const saved = source === "common"
      ? await saveButtonMappings(payload)
      : (await updateButtonMappingTemplate(source.slice("template:".length), payload)).mappings;
    if (request !== saveSequence || editingSource.value !== source) return true;
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
    mappings.value = cloneMappings(saved);
    savedSnapshot.value = cloneMappings(saved);
    if (message) {
      statusMessage.value = message;
    }
    reportFrontendEvent({ event: "button_mapping_editor_save", phase: "completed", result: "passed", reason: source === "common" ? "common" : "template" });
    return true;
  } catch (error) {
    statusMessage.value = error instanceof Error ? error.message : String(error);
    reportFrontendEvent({ event: "button_mapping_editor_save", phase: "completed", result: "failed", reason: source === "common" ? "common" : "template" });
    return false;
  } finally {
    if (request === saveSequence) busy.value = false;
  }
}

async function restoreDefaults(): Promise<void> {
  if (busy.value) return;
  const template = activeTemplateEntry.value;
  if (template?.builtIn) {
    if (!window.confirm(`只复位“${template.name}”为当前版本默认按键？该模板修改和当前草稿将丢失，其它模板及关联保持不变。`)) return;
    busy.value = true;
    try {
      configuration.value = await resetBuiltinTemplate(template.id);
      templateCatalog.value = await getTemplateCatalog();
      const saved = templateCatalog.value.find(item => item.id === template.id)?.buttonMappings;
      if (saved) { mappings.value = cloneMappings(saved); savedSnapshot.value = cloneMappings(saved); }
      statusMessage.value = `已复位“${template.name}”`;
    } catch (error) { statusMessage.value = String(error); } finally { busy.value = false; }
    return;
  }
  mappings.value = { enabled: true, actions: {} };
  statusMessage.value = "已载入默认草稿；点击“保存配置”后写入当前编辑目标";
}

async function saveConfiguration(): Promise<void> {
  await persist("配置已保存并生效");
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
    const template = await saveButtonMappingTemplate(name, cloneMappings(mappings.value));
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

async function selectEditingSource(event: Event): Promise<void> {
  const select = event.target as HTMLSelectElement;
  const requested = select.value as EditingSource;
  if (requested === editingSource.value) return;
  if (dirty.value) {
    if (window.confirm("当前配置有未保存更改。确定将先保存，再切换编辑目标。")) {
      const saved = await persist("配置已保存");
      if (!saved) {
        select.value = editingSource.value;
        return;
      }
    } else if (!window.confirm("放弃当前未保存更改并切换编辑目标？")) {
      select.value = editingSource.value;
      return;
    } else {
      mappings.value = cloneMappings(savedSnapshot.value);
    }
  }
  loadEditingSource(requested);
  select.value = editingSource.value;
}



function phaseLabel(phase: RawInputPhase | undefined): string {
  switch (phase) {
    case "ready":
      return "按键监听已就绪";
    case "starting":
      return "正在启动监听";
    case "failed":
      return "监听启动失败（自动重试中）";
    case "stopped":
      return "监听已停止";
    case "awaiting":
      return "等待遥控器连接";
    case "unsupported":
      return "当前环境暂不支持";
    default:
      return "正在读取状态";
  }
}

/** 手动启停监听：自动启动之外保留显式控制（停止后自动重试不生效）。 */
async function toggleListener(): Promise<void> {
  busy.value = true;
  statusMessage.value = null;
  try {
    if (rawInput.value?.phase === "ready") {
      await stopRawInput();
      statusMessage.value = "监听已停止；映射与高亮暂停（按住说话不受影响）";
    } else {
      await startRawInput();
      statusMessage.value = "监听已启动";
    }
  } catch (error) {
    statusMessage.value = error instanceof Error ? error.message : String(error);
  } finally {
    busy.value = false;
  }
}

const rawInput = computed(() => props.runtime?.platform.rawInput);
const connectionInfo = computed(() => props.runtime?.platform.connection);

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
  loadEditingSource("common");
  mappingSnapshot.value = snapshot;
  activeButtons.value = new Set(snapshot.observedButtons);

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
      <div class="mapping-heading-row">
        <h1>按键映射</h1>
        <label class="toggle-row mapping-toggle-slot" title="开启后，遥控器按键按本页配置执行动作；关闭时，遥控器保持原始按键行为。">
          <span>启用自定义按键功能</span>
          <input v-model="enabled" type="checkbox" class="toggle-input" :disabled="busy" />
        </label>
        <div class="device-chip" :class="{ connected: connectionInfo?.phase === 'ready' || connectionInfo?.phase === 'streaming' }">
          <span class="status-dot" :class="connectionInfo?.phase === 'streaming' ? 'active' : connectionInfo?.phase === 'ready' ? 'success' : 'pending'"></span>
          <span>{{ deviceLabel }}</span>
          <BatteryIndicator :connection="connectionInfo" />
        </div>
      </div>
      <div class="mapping-header-controls">
        <div class="current-template-display">
          <span>当前使用</span>
          <output aria-label="当前使用的按键模板">{{ !currentTemplate.applied.value ? '正在确认当前模板…' : currentTemplate.templateId.value ? currentTemplate.applied.value.name ?? '当前模板' : '通用配置' }}</output>
        </div>
        <label class="editing-source-picker">
          <span>编辑配置</span>
          <select :value="editingSource" :disabled="busy" @change="selectEditingSource">
            <option v-for="option in editingSourceOptions" :key="option.value" :value="option.value">{{ option.label }}</option>
          </select>
        </label>


      </div>
      <p v-if="currentTemplate.error.value" class="error-text" role="status">{{ currentTemplate.error.value }}</p>
      <p class="mapping-header-status muted">所有模板只发送按键或组合键；内置推荐可复制后编辑。</p>
    </header>

    <div id="global-mapping-panel">
    <div ref="canvasEl" class="mapping-canvas" :style="{ height: `${CANVAS_HEIGHT}px`, '--mapping-card-height': `${CARD_HEIGHT}px` }">
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
          :class="{ active: voiceActive }"
          fill="none"
        />
        <polygon :points="arrowPolygon(VOICE_PLACEMENT)" :class="{ active: voiceActive }" />
      </svg>

      <figure ref="remotePhotoEl" class="remote-photo" :style="{ left: `${remoteLeft}px`, top: `${REMOTE_TOP}px` }">
        <img ref="remoteImageEl" src="/RC003-remote-photo@2x.png" alt="小米蓝牙语音遥控器 2 Pro（RC003）示意图" draggable="false" @load="measureRemoteImageFrame" />
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
        :class="{ visible: voiceActive }"
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
            :disabled="busy || menuReserved(placement.button)"
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
        :class="{ active: voiceActive }"
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
          <span class="badge pending voice-badge" :class="{ active: voiceActive }">按住说话</span>
        </div>
        <p class="voice-note">按下开始、松开结束；不参与自定义映射，不加双击/长按延迟。</p>
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
          <p class="muted">当前：{{ actionSummary(actionOf(editingTarget.button, editingTarget.trigger)) }}</p>
        </div>
        <div class="button-row">
          <button
            class="secondary-button editor-disable-btn"
            :class="{ 'is-active': actionOf(editingTarget.button, editingTarget.trigger).type === 'disabled' }"
            type="button"
            :disabled="busy"
            title="只禁用当前格子的映射，此按键恢复原始行为"
            @click="applyAction({ type: 'disabled' })"
          >
            禁用按键
          </button>
          <button class="secondary-button" type="button" :disabled="busy" @click="editingTarget = null">关闭</button>
        </div>
      </div>
      <ButtonActionEditor
        :key="`${editingSource}:${editingTarget.button}:${editingTarget.trigger}`"
        :shortcuts-only="editingSource !== 'common'"
        :mappings="mappings"
        :button="editingTarget.button"
        :trigger="editingTarget.trigger"
        :preset-apps="presetApps"
        @update="applyAction"
        @applications="mappings = { ...mappings, applications: $event }"
        @status="statusMessage = $event"
      />

      <p v-if="editingTarget.trigger === 'single'" class="muted editor-note">
        未配置双击与长按时，单击在按下瞬间触发（零延迟）；返回/方向/音量键按住会连续触发。
      </p>
      <p v-else class="muted editor-note">
        {{ editingTarget.trigger === "double" ? "双击判定窗口约 0.3 秒：配置后单击会稍等片刻以区分双击。" : "长按约 0.55 秒触发；配置后按住连发停用。" }}
      </p>
    </article>

    <div class="card mapping-footer">
      <small class="muted">按下高亮只表示已收到按键；动作按当前程序的模板执行，未配置则不执行。</small>
      <div class="mapping-footer-status">
        <span class="status-dot" :class="rawInput?.phase === 'ready' ? 'success' : 'pending'"></span>
        <span>{{ phaseLabel(rawInput?.phase) }}</span>
        <button
          v-if="rawInput?.phase === 'ready' || rawInput?.phase === 'stopped' || rawInput?.phase === 'failed'"
          class="secondary-button footer-listener-toggle"
          type="button"
          :disabled="busy"
          @click="toggleListener"
        >
          {{ rawInput?.phase === "ready" ? "停止监听" : "启动监听" }}
        </button>
        <small v-if="mappingSnapshot && !mappings.enabled" class="muted"> · 总开关关闭（按键保持原样）</small>
        <small v-if="dirty" class="muted"> · 当前编辑目标有未保存更改</small>
      </div>
      <label class="toggle-row" title="开启后，操作实体遥控器不会切换正在编辑的按键。">
        <span>锁定当前按键</span>
        <input :checked="lockSelection" :aria-busy="selectionPreference.pending.value" :aria-disabled="!selectionPreference.loaded.value || selectionPreference.pending.value" @change="selectionPreference.toggleCheckbox" type="checkbox" class="toggle-input" />
      </label>
      <span class="muted lock-hint">按遥控器时保持当前编辑项</span>
      <span v-if="selectionPreference.error.value" role="alert" class="error-text">{{ selectionPreference.error.value }}</span>
    </div>
    </div>

    <p v-if="statusMessage" class="operation-message mapping-status">{{ statusMessage }}</p>
    <p v-if="mappingSnapshot?.lastError" class="error-text">{{ mappingSnapshot.lastError }}</p>
    </div>

    <footer class="mapping-actions" aria-label="按键配置操作">
      <div class="button-row">
        <button class="secondary-button" type="button" :disabled="busy" @click="saveConfiguration">
          保存当前配置
        </button>

        <button class="secondary-button" type="button" :disabled="busy" @click="openSaveTemplateDialog">
          保存为模板
        </button>
        <button class="secondary-button" type="button" :disabled="busy" @click="exportConfiguration">
          导出配置…
        </button>
        <button class="secondary-button" type="button" :disabled="busy" @click="restoreDefaults">
          {{ activeTemplateEntry?.builtIn ? "复位模板" : "恢复默认" }}
        </button>
      </div>
    </footer>
  </section>

  <SettingsDialog
    v-if="saveTemplateDialogOpen"
    title="保存为完整按键模板"
    description="保存当前编辑目标的草稿副本；不会开启自动切换或创建程序关联。"
    :busy="busy"
    @close="saveTemplateDialogOpen = false"
  >
    <label class="save-template-field">模板名称<input v-model="templateNameDraft" :disabled="busy" @keyup.enter="saveAsTemplate" /></label>
    <p v-if="templateNameError" class="error-text">{{ templateNameError }}</p>
    <template #actions>
      <button type="button" :disabled="busy" @click="saveAsTemplate">{{ busy ? "正在保存…" : "保存模板" }}</button>
      <button type="button" class="secondary-button" :disabled="busy" @click="saveTemplateDialogOpen = false">取消</button>
    </template>
  </SettingsDialog>

</template>

<style scoped>
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
  min-width: 0;
  /* 与独立正文的槽位对齐；hidden 只预留空间，不显示第二条滚动条。 */
  overflow: hidden;
  scrollbar-gutter: stable;
  padding: 12px var(--content-inline-padding) 16px;
  border-top: 1px solid var(--border);
  background: var(--surface-canvas);
}
.mapping-actions .button-row { justify-content: flex-end; }
.mapping-actions button { max-width: 100%; white-space: normal; }
.editing-source-picker, .current-template-display {
  display: grid;
  gap: 4px;
  min-width: 0;
}
.editing-source-picker > span, .current-template-display > span {
  color: var(--text-muted);
  font-size: 0.78rem;
}
.editing-source-picker select { min-height: 36px; min-width: 0; width: 100%; }
.current-template-display output {
  display: block;
  box-sizing: border-box;
  min-height: 36px;
  padding: 0 8px;
  line-height: 34px;
  border: 1px solid var(--border);
  border-radius: 4px;
  color: var(--text-secondary);
  background: var(--surface-control);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.save-template-field { display: grid; gap: 7px; font-size: 13px; font-weight: 600; }
.mapping-header {
  display: grid;
  gap: 10px;
  align-items: start;
}
.mapping-heading-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 250px minmax(0, 1fr);
  align-items: center;
  gap: 16px;
  min-width: 0;
}
.mapping-heading-row h1 { white-space: nowrap; }
.mapping-toggle-slot { justify-self: start; }
.mapping-toggle-slot.unavailable { color: var(--text-muted); }
.mapping-heading-row .device-chip { justify-self: end; }
.mapping-header-controls {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 240px));
  justify-content: start;
  gap: 12px;
  width: 100%;
}
.mapping-header-status { min-height: 20px; margin: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
@media (max-width: 620px) {
  .mapping-heading-row { grid-template-columns: 1fr; gap: 8px; }
  .mapping-heading-row .device-chip { justify-self: start; }
}
</style>
