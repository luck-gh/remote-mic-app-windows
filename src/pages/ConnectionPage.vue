<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import EnhancedCaptureConfirmDialog from "../components/EnhancedCaptureConfirmDialog.vue";
import BatteryIndicator from "../components/BatteryIndicator.vue";
import { VOICE_HOTKEY_CUSTOM_CAPTURE_ENABLED } from "../lib/feature-flags";
import { reasonToken, reportFrontendEvent } from "../lib/frontend-diagnostics";
import { t, tPage } from "../lib/i18n";
import {
  captureFailureText,
  captureInactiveText,
  captureRevokedText,
  captureSwitchState as captureSwitchStateFn,
  normalizeCaptureRevoked,
  normalizeCaptureUnavailable,
  shouldShowCaptureInactive,
  shouldShowCaptureRevoked,
} from "../lib/capture-switch";
import {
  normalizeVoiceAudioHealth,
  shouldShowVoiceAudioDegraded,
  voiceAudioDegradedText,
} from "../lib/voice-audio-health";
import type {
  AudioEndpoint,
  AudioSnapshot,
  AudioRouteSnapshot,
  CaptureInputSnapshot,
  CaptureInputSettings,
  DefaultCaptureInputs,
  CaptureSupport,
  Rc003TaskStatus,
  Rc003CaptureHealth,
  Rc003CaptureRevoked,
  Rc003CaptureUnavailable,
  ConnectionSnapshot,
  KeyChord,
  PairedRemote,
  RuntimeSnapshot,
  VoiceInputTool,
} from "../lib/bridge";
import {
  audioPhaseLabel,
  chordLabel,
  connectRemote,
  connectionPhaseLabel,
  disconnectRemote,
  getAudioSnapshot,
  getAudioRouteSnapshot,
  getCaptureInput,
  getDefaultCaptureInputs,
  setDefaultCaptureInput,
  listCaptureInputs,
  setCaptureInput,
  resolveCaptureRecovery,
  getCaptureSupport,
  getRc003BridgeSnapshot,
  getRc003TaskStatus,
  enableRc003Capture,
  disableRc003Capture,
  getConnectionSnapshot,
  getGainDb,
  getOtherVoiceHotkey,
  getVoiceHoldHotkey,
  getVoiceInputTool,
  getVokieInstallation,
  isRecommendedVoiceEndpoint,
  listAudioEndpoints,
  openVbCableDownloadPage,
  launchVokie,
  openVokieHomepage,
  remoteModelLabel,
  scanPairedRemotes,
  selectAudioEndpoint,
  setGainDb,
  setOtherVoiceHotkey,
  setVoiceHoldHotkey,
  setVoiceInputTool,
  startShortcutCapture,
  stopShortcutCapture,
  subscribeShortcutCaptureEdges,
  voiceHoldHotkeyLabel,
  type KeyCode,
  type ShortcutCaptureEdge,
} from "../lib/bridge";

const props = defineProps<{ runtime: RuntimeSnapshot | null }>();

const emptyConnection = (): ConnectionSnapshot => ({
  phase: "idle",
  remoteName: null,
  remoteModel: "unknown",
  capabilities: null,
  voiceState: "idle",
  decodedSamples: 0,
  voiceAudioHealth: "unknown",
  voiceAudioRateHz: null,
  generation: 0,
  reconnectAttempt: 0,
  powerNotificationsAvailable: false,
  lastError: null,
});

const emptyAudio = (): AudioSnapshot => ({
  phase: "unsupported",
  selectedEndpointId: null,
  selectedEndpointName: null,
  queuedSamples: 0,
  submittedSamples: 0,
  generation: 0,
  lastError: null,
});

/**
 * IPC / 后端错误 → 界面文案。原始错误只进控制台（排查用），界面显示与当前
 * 语言一致的一句短句：英文界面不夹带后端中文原文，用户也不需要读懂内部错误。
 */
function actionErrorText(scope: string, error: unknown): string {
  console.error(`[connection] ${scope}`, error);
  return t("操作没有完成，请稍后重试。");
}

const connection = ref<ConnectionSnapshot>(emptyConnection());
const audio = ref<AudioSnapshot>(emptyAudio());
const audioRoute = ref<AudioRouteSnapshot | null>(null);
const routeError = ref("");
const showAdvancedAudio = ref(false);
let audioRouteRequest = 0;
let reportedAudioRoute = "";
function reportAudioRoute(phase: string, reason: string | null) {
  const phases = ["paired", "manual", "unconfigured", "unavailable"];
  const reasons = ["capture_missing", "render_missing", "ambiguous", "unsupported", "identity_changed", "render_mismatch", "audio_unready", "metadata_unavailable", "enumeration_failed", "snapshot_read_failed"];
  const safePhase = phases.includes(phase) ? phase : "unknown";
  const safeReason = reason === null ? "none" : reasons.includes(reason) ? reason : "unknown";
  const signature = `${safePhase}_${safeReason}`;
  if (signature === reportedAudioRoute) return;
  reportedAudioRoute = signature;
  reportFrontendEvent({ event: "audio_route_status", phase: "completed", result: safePhase === "paired" ? "passed" : safePhase === "unavailable" ? "failed" : "unknown", reason: signature });
}
const captureInput = ref<CaptureInputSnapshot>({ settings: { enabled: false, keepSelected: false, endpointId: null, endpointName: null }, phase: "disabled", recoveryPending: false, lastError: null });
const captureEndpoints = ref<AudioEndpoint[]>([]);
const captureBusy = ref(false);
const pendingCapture = ref<CaptureInputSettings | null>(null);
let captureRevision = 0;
const captureMessage = ref("");
const defaultCaptureInputs = ref<DefaultCaptureInputs | null>(null);
const defaultCaptureError = ref(false);
const captureChoice = ref<string>();
const selectedCaptureId = computed(() => captureChoice.value ?? pendingCapture.value?.endpointId ?? captureInput.value.settings.endpointId ?? '');
let defaultCaptureRequest = 0;
let defaultCaptureReading = false;
async function refreshDefaultCaptureInputs() {
  if (defaultCaptureReading || captureBusy.value) return;
  const request = ++defaultCaptureRequest;
  defaultCaptureReading = true;
  try {
    const actual = await getDefaultCaptureInputs();
    if (request === defaultCaptureRequest && !unmounted) {
      defaultCaptureInputs.value = actual;
      defaultCaptureError.value = false;
    }
  } catch (error) {
    if (request === defaultCaptureRequest && !unmounted) {
      defaultCaptureError.value = true;
      reportFrontendEvent({event: 'default_capture_read', phase: 'completed', result: 'failed', reason: 'read_failed'});
    }
  } finally { defaultCaptureReading = false; }
}
function defaultCaptureName(role: keyof DefaultCaptureInputs): string {
  if (defaultCaptureError.value) return t('读取失败，请刷新设备列表');
  if (!defaultCaptureInputs.value) return t('读取中…');
  return defaultCaptureInputs.value[role]?.name ?? t('未设置');
}
async function selectCaptureTarget(event: Event) {
  const id = (event.target as HTMLSelectElement).value;
  captureChoice.value = id;
  captureMessage.value = '';
  if (captureInput.value.settings.enabled) await changeCaptureInput(true, id || null);
}
async function applyWindowsDefaultInput() {
  if (captureBusy.value || audioBusy.value || connection.value.voiceState !== 'idle' || captureInput.value.recoveryPending) return;
  const target = captureEndpoints.value.find(endpoint => endpoint.id === selectedCaptureId.value);
  if (!target) { captureMessage.value = t('所选麦克风已不可用，请刷新设备列表。'); return; }
  ++defaultCaptureRequest;
  captureBusy.value = true;
  captureMessage.value = '';
  reportFrontendEvent({event: 'default_capture_set', phase: 'requested', result: 'unknown', reason: 'explicit_user_request'});
  let succeeded = false;
  try {
    defaultCaptureInputs.value = await setDefaultCaptureInput(target.id, target.name);
    defaultCaptureError.value = false;
    succeeded = true;
    captureMessage.value = t('已设为 Windows 默认麦克风');
    reportFrontendEvent({event: 'default_capture_set', phase: 'completed', result: 'passed', reason: 'defaults_confirmed'});
  } catch (error) {
    captureMessage.value = actionErrorText('set_default_capture_input', error);
    reportFrontendEvent({event: 'default_capture_set', phase: 'completed', result: 'failed', reason: 'set_failed'});
  } finally { captureBusy.value = false; }
  if (!succeeded) await refreshDefaultCaptureInputs();
}
const capturePhase = computed(() => ({ active: "本次说话正在使用所选输入设备", relinquished: "检测到外部改选，本次已让出控制", recovery_required: "上次会话未完整恢复，请选择如何处理", failed: "输入设备服务不可用", unsupported: "仅 Windows 应用支持" }[captureInput.value.phase] ?? (captureInput.value.settings.enabled
  ? captureInput.value.settings.keepSelected
    ? "松开后保留为 Windows 默认麦克风；已是默认时可直接开始。"
    : "松开后恢复原麦克风；已是默认时可直接开始。"
  : "")));
async function refreshCaptureInput() {
  if (captureBusy.value) return;
  const revision = captureRevision;
  try { const snapshot = await getCaptureInput(); if (revision === captureRevision && !captureBusy.value) captureInput.value = snapshot; }
  catch (error) { if (revision === captureRevision) captureMessage.value = String(error); }
}
async function scanCaptureInputs() {
  captureBusy.value = true;
  try { captureEndpoints.value = await listCaptureInputs(); }
  catch (error) { captureMessage.value = String(error); }
  finally { captureBusy.value = false; }
}
async function changeCaptureInput(enabled: boolean, endpointId = pendingCapture.value?.endpointId ?? captureInput.value.settings.endpointId, keepSelected = captureInput.value.settings.keepSelected) {
  if (captureBusy.value || selectingEndpointId.value) return;
  const endpoint = captureEndpoints.value.find(e => e.id === endpointId);
  const config = { enabled, keepSelected: enabled && keepSelected, endpointId, endpointName: endpointId ? (endpoint?.name ?? captureInput.value.settings.endpointName) : null };
  const targetChanged = endpointId !== captureInput.value.settings.endpointId || config.endpointName !== captureInput.value.settings.endpointName;
  const revision = ++captureRevision;
  captureBusy.value = true; captureMessage.value = "";
  try {
    const result = await setCaptureInput(config);
    if (revision === captureRevision) {
      captureInput.value = result;
      pendingCapture.value = null;
      await Promise.all([refreshAudio(), refreshAudioRoute()]);
    }
  } catch (error) {
    captureMessage.value = String(error);
    // Keep the user's explicit target as an unsaved draft, separate from runtime.
    pendingCapture.value = targetChanged && endpoint ? config : null;
    if (pendingCapture.value) showAdvancedAudio.value = true;
    try { captureInput.value = await getCaptureInput(); } catch { /* keep last confirmed state */ }
  }
  finally { captureBusy.value = false; }
}
async function changeCaptureSwitch(event: Event) {
  const control = event.target as HTMLInputElement;
  const enabled = control.checked;
  control.checked = captureInput.value.settings.enabled;
  await changeCaptureInput(enabled, enabled ? selectedCaptureId.value || null : undefined, enabled);
}
async function changeCaptureRestore(event: Event) {
  const control = event.target as HTMLInputElement;
  const restore = control.checked;
  control.checked = captureInput.value.settings.enabled && !captureInput.value.settings.keepSelected;
  await changeCaptureInput(true, undefined, !restore);
}
async function recoverCaptureInput(restore: boolean) {
  const revision = ++captureRevision;
  captureBusy.value = true; captureMessage.value = "";
  try { const result = await resolveCaptureRecovery(restore); if (revision === captureRevision) captureInput.value = result; }
  catch (error) { captureMessage.value = String(error); }
  finally { captureBusy.value = false; }
}
const scanning = ref(false);
const connectingDeviceId = ref("");
const disconnecting = ref(false);
const devices = ref<PairedRemote[]>([]);
const scanMessage = ref("");
const operationMessage = ref("");
const audioEndpoints = ref<AudioEndpoint[]>([]);
const scanningAudio = ref(false);
const audioScanComplete = ref(false);
const selectingEndpointId = ref("");
const openingVbCablePage = ref(false);
const audioMessage = ref("");
/**
 * 语音增益（dB，0–24；对齐 Mac 设置页「增益」滑块）：0 = 原始音量。
 *
 * 拖动（input）只更新显示，松手（change）才走 IPC 落盘——避免拖一次写几十遍
 * 设置文件；保存返回的钳制值才回显，失败回到上一次生效值并给出原因。
 */
const gainDb = ref(0);
/** 最后一次确认保存的增益：保存失败时回退到它，不留在"看起来生效"的位置。 */
const savedGainDb = ref(0);
const savingGain = ref(false);
const gainError = ref("");
const voiceHotkey = ref<KeyChord | null>(null);
const savingVoiceHotkey = ref(false);
const voiceHotkeyMessage = ref("");
const capturingVoiceHotkey = ref(false);
const captureStartingVoiceHotkey = ref(false);
/** 录入开始时仍有 preheld 键按住：后端吞键但不投递边沿，直到全部松开。 */
const waitingPreheldRelease = ref(false);
const voiceCaptureDisplay = ref<KeyCode[]>([]);
/** 当前选择的输入工具；null = 尚未读取（或从未选择过，正在推断）。 */
const voiceInputTool = ref<VoiceInputTool | null>(null);
const savingVoiceInputTool = ref(false);
/**
 * 正在为哪个工具落"按住说话快捷键"（含两段 IPC 全程）。见 toolChordPill：
 * 这期间状态胶囊按乐观态显示，避免切换工具的瞬间闪一帧"未同步"黄标。
 */
const applyingTool = ref<VoiceInputTool | null>(null);
/**
 * Vokie 检测状态（2026-10-01 Andy 需求 + 快捷键冲突处理）：
 * - `installed === false`：Vokie 面板显示官网入口；
 * - 已安装但没运行：提示“没有运行”——没运行就不会响应右 Alt；
 * - `running === true`：豆包面板给出冲突提示（两者都用右 Alt）。
 * 检测在 Rust 侧只读完成（卸载表 / 开始菜单 / App Paths + 进程名），只回布尔值、不回路径。
 */
const vokieInstalled = ref<boolean | null>(null);
const vokieRunning = ref<boolean | null>(null);
const checkingVokie = ref(false);
const openingVokiePage = ref(false);
const launchingVokie = ref(false);
const vokieCheckMessage = ref("");
let pollTimer: ReturnType<typeof setInterval> | undefined;
let unlistenVoiceCapture: (() => void) | null = null;
let voiceCaptureTimeout: number | null = null;
/**
 * 落盘稳定窗口：外部钩子（微信输入法等）会吞掉完成键的物理边沿、随后把整个
 * 组合以注入副本重放（见 Bugs/2026-09-27-ime-chord-hook-eats-active-hotkey-capture.md）。
 * 副本可能晚于用户物理松开到达，因此"全部松开"后不立即落盘，先等一个短窗口；
 * 窗口内又出现按下沿则取消并重新等待。
 */
let voiceCaptureSettleTimeout: number | null = null;
let voiceCaptureRequestId = 0;
let unmounted = false;
const voiceCapturePressed = new Set<KeyCode>();
/** 本次录入会话按过的全部按键（按首次按下顺序，去重）。 */
const voiceCaptureEverPressed: KeyCode[] = [];
let voiceCapturedKeys: KeyCode[] | null = null;

/** 按住说话快捷键默认值（v1 固定，适配微信输入法的默认语音热键）。 */
const DEFAULT_VOICE_HOTKEY_KEYS: KeyCode[] = ["left_control", "left_windows"];

/**
 * 输入工具 → "按住说话快捷键"（2026-09-30 连接页改版的设计契约）：
 * 选工具即自动落这个组合，用户不需要理解快捷键本身。
 * `other` 不预设——由用户在"其他工具"面板里自己选（右 Alt / 左 Alt / 不按键）。
 */
const VOICE_TOOL_CHORDS: Record<VoiceInputTool, KeyCode[] | null> = {
  doubao: ["right_alt"],
  wechat: [...DEFAULT_VOICE_HOTKEY_KEYS],
  vokie: ["right_alt"],
  other: null,
};

/** 工具卡片顺序：豆包第一（2026-09-30 Andy 要求），Vokie 在“其他工具”之前（2026-10-01 新增）。 */
const TOOL_CARDS: Array<{ id: VoiceInputTool; name: string; note: string }> = [
  { id: "doubao", name: "豆包输入法", note: "使用右 Alt 语音键" },
  { id: "wechat", name: "微信输入法", note: "用默认语音键，最省事" },
  { id: "vokie", name: "Vokie", note: "流式显示，智能整理，不占用输入法" },
  { id: "other", name: "其他工具", note: "自己指定按键" },
];

/** "其他工具"提供的快捷键；不按键时只接收语音。 */
const OTHER_CHORD_OPTIONS: Array<{ id: string; keys: KeyCode[]; label: string }> = [
  { id: "right_alt", keys: ["right_alt"], label: "右 Alt" },
  { id: "left_alt", keys: ["left_alt"], label: "左 Alt" },
  { id: "none", keys: [], label: "不按键" },
];

const CAPTURE_MODIFIER_KEYS: ReadonlySet<KeyCode> = new Set<KeyCode>([
  "left_control",
  "right_control",
  "left_shift",
  "right_shift",
  "left_alt",
  "right_alt",
  "left_windows",
  "right_windows",
]);

const activeVoiceHotkeyKeys = computed(() =>
  voiceHotkey.value ? [...voiceHotkey.value.keys].sort().join("+") : "",
);

function chordKeysActive(keys: string[]): boolean {
  return [...keys].sort().join("+") === activeVoiceHotkeyKeys.value;
}

function otherChordActive(option: { keys: KeyCode[] }): boolean {
  return option.keys.length ? chordKeysActive(option.keys) : activeVoiceHotkeyKeys.value === "";
}

/** 工具卡片/豆包面板显示的"已自动设置"判据：当前快捷键 == 该工具要求的组合。 */
function toolChordActive(tool: VoiceInputTool): boolean {
  const keys = VOICE_TOOL_CHORDS[tool];
  return keys !== null && chordKeysActive(keys);
}

/**
 * 状态胶囊（"已自动设置 / 未同步，点左侧卡片重设"）的判据。
 *
 * 切换工具是两段异步 IPC（落工具选择 → 写按住说话快捷键），期间 `voiceHotkey`
 * 还是上一代的组合——纯按 `toolChordActive` 判定会闪一帧"未同步"黄标
 * （2026-10-01 Andy 实测：快速在微信/豆包之间切换时黄标闪现）。正在为这个工具
 * 落快捷键时按乐观态显示"已自动设置"；失败时 applyingTool 清空，回到真实判定
 * （写失败的真实状态由 voiceHotkeyMessage + 重新读取的 voiceHotkey 呈现）。
 */
function toolChordPill(tool: VoiceInputTool): { ok: boolean; label: string } {
  if (applyingTool.value === tool) {
    return { ok: true, label: t("已自动设置") };
  }
  return toolChordActive(tool)
    ? { ok: true, label: t("已自动设置") }
    : { ok: false, label: t("未同步，点左侧卡片重设") };
}

/**
 * 老配置（从未选过工具）按当前快捷键推断一次：左 Ctrl + 左 Win = 微信、
 * 右 Alt = 豆包（Vokie 与豆包用同一个组合，只能靠已保存的工具区分）、
 * 其余（左 Alt / 不按键 / 自定义组合）= 其他工具。
 */
function deriveToolFromChord(chord: KeyChord | null): VoiceInputTool {
  const normalized = chord ? [...chord.keys].sort().join("+") : "";
  if (normalized === [...DEFAULT_VOICE_HOTKEY_KEYS].sort().join("+")) return "wechat";
  if (normalized === "right_alt") return "doubao";
  return "other";
}

/**
 * 选择输入工具：立即把该工具要求的"按住说话快捷键"落盘（用户不需要理解快捷键），
 * 并把工具选择持久化（决定下次进页展示哪套引导）。
 */
async function selectVoiceInputTool(tool: VoiceInputTool): Promise<void> {
  if (savingVoiceInputTool.value) return;
  if (capturingVoiceHotkey.value) {
    await finishVoiceHotkeyCapture(t("已切换输入工具，已取消录入"));
  }
  const previous = voiceInputTool.value;
  savingVoiceInputTool.value = true;
  // 乐观态窗口：工具已切、快捷键还在写盘路上，期间状态胶囊不许判成"未同步"。
  applyingTool.value = tool;
  voiceHotkeyMessage.value = "";
  voiceInputTool.value = tool;
  try {
    await setVoiceInputTool(tool);
  } catch (error) {
    voiceInputTool.value = previous;
    voiceHotkeyMessage.value = actionErrorText("set_voice_input_tool", error);
    savingVoiceInputTool.value = false;
    applyingTool.value = null;
    return;
  }
  savingVoiceInputTool.value = false;
  const keys = VOICE_TOOL_CHORDS[tool];
  try {
    if (keys) {
      await applyVoiceHotkey([...keys]);
    } else if (tool === "other") {
      // 「其他工具」有自己的记忆：切去豆包/微信会改写"按住说话快捷键"，
      // 切回来时按用户上次选的恢复（never = 从未选过 → 保持现状）。
      const remembered = await getOtherVoiceHotkey();
      if (remembered) {
        await applyVoiceHotkey([...remembered]);
      }
    }
  } finally {
    applyingTool.value = null;
  }
  // Vokie：每次选中都重查一次安装状态（用户可能刚装好就回来选）。
  if (tool === "vokie") {
    void refreshVokieInstallation();
  }
}

/** 「其他工具」记住用户选的按键：写失败不影响本次生效（只是下次不恢复）。 */
async function rememberOtherVoiceHotkey(keys: string[]): Promise<void> {
  try {
    await setOtherVoiceHotkey([...keys]);
  } catch (error) {
    voiceHotkeyMessage.value = actionErrorText("set_other_voice_hotkey", error);
  }
}

/** 「其他工具」面板里选按键：先生效，再记住（切去豆包/微信再切回来时恢复）。 */
async function applyOtherChord(keys: string[]): Promise<void> {
  await applyVoiceHotkey([...keys]);
  await rememberOtherVoiceHotkey(keys);
}

/** Vokie 检测；失败保持 null，不把"检测失败"当成"没装/没运行"。 */
async function refreshVokieInstallation(): Promise<void> {
  if (checkingVokie.value) return;
  checkingVokie.value = true;
  try {
    const status = await getVokieInstallation();
    vokieInstalled.value = status.installed;
    vokieRunning.value = status.running;
    vokieCheckMessage.value = "";
  } catch (error) {
    vokieInstalled.value = null;
    vokieRunning.value = null;
    vokieCheckMessage.value = actionErrorText("vokie_installation", error);
  } finally {
    checkingVokie.value = false;
  }
}

async function openVokiePage(): Promise<void> {
  openingVokiePage.value = true;
  try {
    await openVokieHomepage();
    vokieCheckMessage.value = t("已打开 Vokie 官网；安装后点“重新检测”。");
  } catch (error) {
    vokieCheckMessage.value = actionErrorText("open_vokie_homepage", error);
  } finally {
    openingVokiePage.value = false;
  }
}

/** 打开 Vokie 之后的自动复查：间隔 1.5s、最多 8 次（约 12 秒）。 */
const VOKIE_LAUNCH_POLL_INTERVAL_MS = 1500;
const VOKIE_LAUNCH_POLL_ATTEMPTS = 8;

/**
 * 点「打开 Vokie」后替用户等它起来并自动复查。
 * 2026-10-02 Andy 反馈：点了按钮、Vokie 明明已经打开，但「Vokie 没有运行」的
 * 提示还挂着，得手动再点一次「重新检测」——因为启动是异步的，点完立刻检测
 * 多半还是 false。这里轮询到 running=true 就清掉提示（提示区变成「Vokie 已在
 * 运行。」）；超时仍没起来就明确告诉他可以再点「重新检测」。
 */
async function waitForVokieRunning(): Promise<void> {
  for (let attempt = 0; attempt < VOKIE_LAUNCH_POLL_ATTEMPTS; attempt += 1) {
    await new Promise<void>((resolve) =>
      window.setTimeout(resolve, VOKIE_LAUNCH_POLL_INTERVAL_MS),
    );
    if (unmounted) return;
    await refreshVokieInstallation();
    if (unmounted) return;
    if (vokieRunning.value === true) {
      vokieCheckMessage.value = t("Vokie 已在运行。");
      return;
    }
  }
  if (!unmounted) {
    vokieCheckMessage.value = t("还没检测到 Vokie 在运行；如果它已经打开，点“重新检测”。");
  }
}

/** 打开 Vokie（装了但没运行时）：启动后自动等待并复查，用户不用再点一次。 */
async function launchVokieApp(): Promise<void> {
  launchingVokie.value = true;
  try {
    await launchVokie();
    vokieCheckMessage.value = t("已打开 Vokie；正在等待它启动…");
    void waitForVokieRunning();
  } catch (error) {
    vokieCheckMessage.value = actionErrorText("launch_vokie", error);
  } finally {
    launchingVokie.value = false;
  }
}

async function applyVoiceHotkey(keys: string[]) {
  savingVoiceHotkey.value = true;
  voiceHotkeyMessage.value = "";
  try {
    voiceHotkey.value = await setVoiceHoldHotkey(
      keys.length ? { keys: [...keys] } : null,
    );
    voiceHotkeyMessage.value = voiceHotkey.value
      ? t("按住说话快捷键已设为 {chord}", {
          chord: voiceHoldHotkeyLabel(voiceHotkey.value),
        })
      : t("按住说话快捷键已关闭，语音键仅输出语音");
  } catch (error) {
    voiceHotkeyMessage.value = actionErrorText("set_voice_hold_hotkey", error);
    await refreshVoiceHotkey();
  } finally {
    savingVoiceHotkey.value = false;
  }
}

// ---------------------------------------------------------------------------
// 「支持更多输入工具」开关（2026-09-29 Andy 需求：连接页的第二入口）。
//
// **与按键页「全按键支持」是同一个设置项**（AppSettings.rc003_capture_enabled），
// 不是新状态：两处开关操作同一对 IPC（enable/disable_rc003_capture），状态
// 由每页挂载时的 getRc003TaskStatus 对账对齐——页面经 <component :is> 切换
// 时组件重建，每次进页都对账一次，两个入口不会漂移。
//
// 判据与文案与 ButtonsPage.toggleRc003Capture 同源：2026-10-03 起**每次
// 开启都先弹确认**（每次开启都会重新授权、都会弹 Windows 授权窗口），
// 不再依赖授权判据；关闭方向永远直接执行。开关只在需要它的工具下面板里
// 出现（豆包必须开、其他工具建议开、微信不需要——见 2026-09-30 设计稿 v3）。
// ---------------------------------------------------------------------------

const rc003CaptureEnabled = ref<boolean | null>(null);
const rc003CaptureBusy = ref(false);
const showCaptureConfirm = ref(false);
const captureSwitchEl = ref<HTMLInputElement | null>(null);
const rc003CaptureHint = ref("");

/**
 * 本机对「全按键支持」（本页叫「支持更多输入工具」）的支持情况（2026-10-07
 * issue #206）：`null` = 尚未读到，按可用处理，不因一次 IPC 失败把功能锁死。
 */
const captureSupport = ref<CaptureSupport | null>(null);
const captureUnsupported = computed(() => captureSupport.value?.available === false);

/**
 * 本机不可用时的文案（与按键页同一句，名称随本页入口）：
 * 用户可见文案的规范来源是 docs/product-copy.md（§2 名称表 / §7 决定索引）。
 */
const CAPTURE_UNSUPPORTED_TITLE =
  "“支持更多输入工具”在这台电脑上暂不可用；新版本支持后会恢复正常。";
const CAPTURE_UNSUPPORTED_NOTE = "这台电脑暂不支持“支持更多输入工具”，其他按键不受影响。";
const CAPTURE_FAILURE_NAME = "“支持更多输入工具”";

const captureSwitchState = computed(() =>
  captureSwitchStateFn({
    enabled: rc003CaptureEnabled.value,
    busy: rc003CaptureBusy.value,
    unsupported: captureUnsupported.value,
  }),
);

/** 失败时显示的一句用户语言；其余情况为 null（模板据此不渲染任何文字）。 */
const captureFailure = computed(() =>
  captureFailureText(rc003CaptureHint.value, t(CAPTURE_FAILURE_NAME)),
);

/**
 * 「全按键支持」实际生效状态（2026-10-13）：Rust 常驻对账发布，本页只读。
 * 开关开着 ≠ 生效——`inactive` 才显示提示（见 captureInactiveNote）。
 */
const rc003CaptureHealth = ref<Rc003CaptureHealth | "unknown">("unknown");

/** 开着没生效的原因（2026-10-27，issue #291）：决定提示说"关掉再打开"还是"别的程序占着"。 */
const rc003CaptureUnavailable = ref<Rc003CaptureUnavailable | "unknown">("unknown");

/** 启动对账把开关关掉的原因（2026-10-15，issue #264 附加观察甲）：`none` = 没发生。 */
const rc003CaptureRevoked = ref<Rc003CaptureRevoked | "unknown">("unknown");

/**
 * 「开着但没生效」的提示（2026-10-13 Andy 定稿，文案见 captureInactiveText）：
 * 后端说 `inactive` 才出现，且只在**遥控器链路正常**时出现——链路本身断掉时
 * 页面已有连接状态与既有文案说明，不把用户往遥控器侧引（这条链断在 PC 内部）。
 * 与失败句同一位：失败句优先（那是用户刚做的动作），一次只显示一句。
 */
const captureInactiveNote = computed(() =>
  rc003CaptureHealth.value !== "unknown" && shouldShowCaptureInactive({
    switchState: captureSwitchState.value,
    health: rc003CaptureHealth.value,
    linkReady: atvvReady.value,
  })
    ? captureInactiveText(t(CAPTURE_FAILURE_NAME), rc003CaptureUnavailable.value)
    : null,
);

/**
 * 「开关被应用自己关掉了」的解释（2026-10-15，issue #264 附加观察甲）：升级后对账会
 * 因系统授权需要重新确认而回落开关，本页必须说明原因，否则用户只会觉得"升级后变差"。
 * 与失败句、失效句同一位：操作失败优先，其次这句，再次「开着没生效」。
 */
const captureRevokedNote = computed(() =>
  shouldShowCaptureRevoked({
    switchState: captureSwitchState.value,
    revoked: rc003CaptureRevoked.value,
  })
    ? captureRevokedText(t(CAPTURE_FAILURE_NAME))
    : null,
);

/**
 * 把开关的 DOM 状态写回绑定值。原生复选框被点击的瞬间浏览器先翻了 checked，
 * 而操作失败（状态没变）时 Vue 判定 props 无变化、不会生成 DOM 补丁——
 * 会出现「状态是关、界面是开」（ButtonsPage 2026-09-25 真机实证，同法防御）。
 */
function syncCaptureSwitchDom(): void {
  const el = captureSwitchEl.value;
  if (el) {
    el.checked = rc003CaptureEnabled.value === true;
  }
}

async function reconcileRc003Capture(): Promise<void> {
  // 本机支持情况单独取、单独兜（2026-10-07 issue #206）：读不到就保持上一次的
  // 值，不因为一次 IPC 失败把开关锁成不可用。
  try {
    const support = await getCaptureSupport();
    captureSupport.value = support ?? null;
  } catch {
    // 读不到支持情况：保持当前值（按可用处理）。
  }
  try {
    const status = await getRc003TaskStatus();
    // 首次对账（null → 权威值）直接采纳；此后以本页用户操作为准，
    // 不被每秒变化无关的 installed 抖动带偏（与 ButtonsPage 同策略）。
    if (rc003CaptureEnabled.value === null) {
      rc003CaptureEnabled.value = status.enabled;
    }
  } catch {
    // 对账失败保持 null：开关显示占位符（不可点），不猜状态。
  }
  // 「全按键支持」实际生效状态（2026-10-13）：单独取、单独兜——任务状态读不到
  // 不该连带把生效状态也冻住（反之亦然）。内存读，随既有轮询每秒刷新。
  try {
    const bridge = await getRc003BridgeSnapshot();
    rc003CaptureHealth.value = bridge.captureHealth;
    rc003CaptureUnavailable.value = normalizeCaptureUnavailable(
      bridge.captureUnavailable,
    );
    rc003CaptureRevoked.value = normalizeCaptureRevoked(bridge.captureRevoked);
  } catch {
    // 读不到保持上一次的值，最多晚一秒反映。
  }
}

async function toggleRc003Capture() {
  if (rc003CaptureBusy.value) return;
  // 本机不可用（ARM64，issue #206）：开关已置灰，这里再守一道——合成事件与
  // 键盘操作都可能绕过原生 disabled。不弹确认、不动 IPC，把 DOM 写回绑定值。
  if (captureUnsupported.value) {
    syncCaptureSwitchDom();
    return;
  }
  // 与 ButtonsPage.toggleRc003Capture 同源：每次开启都先弹确认
  // （2026-10-03 定稿：每次开启都重新授权，都会弹 Windows 授权窗口）。
  // 关闭方向永远直接执行。
  if (rc003CaptureEnabled.value !== true) {
    showCaptureConfirm.value = true;
    // 开关 DOM 在点击瞬间已被浏览器翻转，先写回关闭，等确认后再真正执行。
    syncCaptureSwitchDom();
    return;
  }
  await applyCaptureToggle();
}

async function applyCaptureToggle() {
  if (rc003CaptureBusy.value) return;
  rc003CaptureBusy.value = true;
  const wasEnabled = rc003CaptureEnabled.value;
  try {
    const next: Rc003TaskStatus =
      wasEnabled === true ? await disableRc003Capture() : await enableRc003Capture();
    if (next.lastError) {
      rc003CaptureHint.value = next.lastError;
      reportFrontendEvent({
        event: "rc003_capture_toggle",
        phase: "completed",
        result: "failed",
        reason: reasonToken("page_connection", "was_enabled", wasEnabled === true, "switch_failed"),
      });
      return;
    }
    rc003CaptureEnabled.value = next.enabled;
    rc003CaptureHint.value = "";
    reportFrontendEvent({
      event: "rc003_capture_toggle",
      phase: "completed",
      result: "passed",
      reason: reasonToken("page_connection", "enabled", next.enabled),
    });
  } catch (error) {
    rc003CaptureHint.value = error instanceof Error ? error.message : String(error);
    reportFrontendEvent({
      event: "rc003_capture_toggle",
      phase: "completed",
      result: "failed",
      reason: reasonToken("page_connection", "was_enabled", wasEnabled === true, "switch_reverted"),
    });
    // 同步回退到点击前的状态，并用权威状态校正一次（与 ButtonsPage 同源）。
    rc003CaptureEnabled.value = wasEnabled === true;
    void getRc003TaskStatus()
      .then((status) => {
        rc003CaptureEnabled.value = status.enabled;
      })
      .catch(() => {});
  } finally {
    rc003CaptureBusy.value = false;
    syncCaptureSwitchDom();
  }
}

function confirmCaptureDialog(): void {
  showCaptureConfirm.value = false;
  void applyCaptureToggle();
}

function closeCaptureDialog(): void {
  showCaptureConfirm.value = false;
  syncCaptureSwitchDom();
}

/**
 * 按住说话快捷键录入：录入门走 OS 级低级钩子（与按键映射页的自定义录入
 * 同一条链路），Win+L 之类的系统组合在到达 Shell 前就被成对吞下，不会
 * 真的锁屏。保存点放在"全部按键松开"之后，避免录入完成但物理键尚未松开
 * 时被系统补执行。Esc（未按修饰键）取消；15 秒未完成自动结束，此时已录到
 * 的组合不再丢弃。
 *
 * 入口在"其他工具"面板；微信 / 豆包
 * 两个工具的快捷键是固定组合，选择即自动设置。
 */
async function beginVoiceHotkeyCapture(): Promise<void> {
  if (capturingVoiceHotkey.value || captureStartingVoiceHotkey.value) return;
  const requestId = ++voiceCaptureRequestId;
  captureStartingVoiceHotkey.value = true;
  voiceHotkeyMessage.value = "";
  cancelVoiceCaptureSettle();
  try {
    const preheld = await startShortcutCapture();
    if (unmounted || requestId !== voiceCaptureRequestId) {
      await stopShortcutCapture().catch(() => undefined);
      return;
    }
    voiceCapturePressed.clear();
    voiceCaptureEverPressed.length = 0;
    voiceCapturedKeys = null;
    voiceCaptureDisplay.value = [];
    capturingVoiceHotkey.value = true;
    // preheld 键的边沿对录入不可见（其 DOWN 已进 OS，UP 必须放行），
    // 后端等它们全部松开后才开始投递边沿；提示用户先松手，避免把
    // "按住中打开录入"截断成半截组合。
    waitingPreheldRelease.value = preheld.length > 0;
    if (preheld.length > 0) {
      voiceHotkeyMessage.value = t(
        "检测到仍有按住的按键，请先松开所有按键；松开后即可按新组合，录入将自动开始",
      );
    }
    if (voiceCaptureTimeout !== null) window.clearTimeout(voiceCaptureTimeout);
    voiceCaptureTimeout = window.setTimeout(() => {
      void finishVoiceHotkeyCapture(t("录入已超时，请重新录入"));
    }, 15_000);
  } catch (error) {
    voiceHotkeyMessage.value = actionErrorText("start_shortcut_capture", error);
  } finally {
    if (requestId === voiceCaptureRequestId) captureStartingVoiceHotkey.value = false;
  }
}

/**
 * 结束录入：已录到组合则落盘生效；零/半截边沿时结合微信输入法语音观测推断
 * （见 applyVoiceHotkey 上方的推断说明）；否则只显示传入的取消原因。
 */
async function finishVoiceHotkeyCapture(cancelMessage?: string): Promise<void> {
  voiceCaptureRequestId += 1;
  captureStartingVoiceHotkey.value = false;
  capturingVoiceHotkey.value = false;
  waitingPreheldRelease.value = false;
  cancelVoiceCaptureSettle();
  if (voiceCaptureTimeout !== null) window.clearTimeout(voiceCaptureTimeout);
  voiceCaptureTimeout = null;
  voiceCapturePressed.clear();
  // 推断判定在清空前取样：会话内见到的边沿数（0 = 全吞，1 = 半截）。
  const seenKeyCount = voiceCaptureEverPressed.length;
  voiceCaptureEverPressed.length = 0;
  const keys = voiceCapturedKeys;
  voiceCapturedKeys = null;
  voiceCaptureDisplay.value = [];
  const stop = await stopShortcutCapture().catch(() => null);
  const wetypeVoice = stop?.wetypeVoice ?? "unknown";
  // 半截会话（scheduleVoiceCaptureFinish 在稳定窗口到期时已把唯一修饰键写进
  // keys）与零边沿会话同样走推断：微信输入法吞键发生在 RIT 层，唯一旁证是其
  // 语音被触发。单主键（如 D）不推断——按主键不会触发微信输入法语音。
  const shouldInferWetypeChord =
    wetypeVoice === "observed" &&
    (keys === null
      ? seenKeyCount <= 1
      : keys.length === 1 && CAPTURE_MODIFIER_KEYS.has(keys[0]));
  if (shouldInferWetypeChord) {
    await applyVoiceHotkey([...DEFAULT_VOICE_HOTKEY_KEYS]);
    // applyVoiceHotkey 成功会覆写消息，推断说明必须在其后写入；失败时保留错误信息。
    if (voiceHotkey.value) {
      voiceHotkeyMessage.value = t(
        "你按下的组合触发了微信输入法的语音（按键被其拦截，内容无法读取），已按微信输入法语音键默认值 左 Ctrl + 左 Win 生效。此快捷键需与微信输入法语音键一致；若你修改过微信输入法的语音键，请在微信输入法设置中查看后重新录入对应组合",
      );
    }
    return;
  }
  if (keys && keys.length > 0) {
    await applyVoiceHotkey([...keys]);
    if (voiceInputTool.value === "other") {
      await rememberOtherVoiceHotkey([...keys]);
    }
    appendWetypeChordNotice([...keys]);
    return;
  }
  if (cancelMessage) {
    voiceHotkeyMessage.value = cancelMessage;
    return;
  }
  voiceHotkeyMessage.value = t(
    "本次未捕获到任何按键。微信输入法会拦截它自己的语音键（默认 左 Ctrl + 左 Win），本次也未观测到语音被触发；请重试，或直接选择上方工具卡片使用固定组合",
  );
}

/**
 * 落盘组合不是微信输入法语音键（默认 左 Ctrl + 左 Win）时提醒：按住说话会把
 * 该组合注入系统来唤起微信输入法语音，组合不一致则按住说话无法生效——这正是
 * "能录上 Win+右 Ctrl 反而没用"的原因。默认组合与推断落盘不需要这句提醒。
 */
function appendWetypeChordNotice(keys: KeyCode[]): void {
  const appliedPrefix = t("按住说话快捷键已设为 {chord}", { chord: "" }).trim();
  if (!voiceHotkeyMessage.value.startsWith(appliedPrefix)) return;
  const normalized = [...keys].sort().join("+");
  if ([...DEFAULT_VOICE_HOTKEY_KEYS].sort().join("+") === normalized) return;
  voiceHotkeyMessage.value += t(
    "。注意：按住说话会把该组合注入系统来唤起微信输入法语音，若与微信输入法语音键不一致将无法生效",
  );
}

/** 落盘稳定窗口时长：外部钩子重放的注入副本通常在物理边沿的同一输入批次内到达。 */
const VOICE_CAPTURE_SETTLE_MS = 200;

/** 取消待执行的落盘稳定窗口（新按下沿到来时需要重新计时）。 */
function cancelVoiceCaptureSettle(): void {
  if (voiceCaptureSettleTimeout !== null) window.clearTimeout(voiceCaptureSettleTimeout);
  voiceCaptureSettleTimeout = null;
}

/**
 * 全部按键松开后延迟定稿：外部钩子吞掉完成键的物理边沿后会以注入副本重放
 * 整个组合，副本可能晚于物理松开到达；立即落盘会把组合截断成"只剩第一个键"
 * （Bugs/2026-09-27）。窗口内若再出现按下沿，cancelVoiceCaptureSettle 会取消
 * 本次计时并重新等待。
 */
function scheduleVoiceCaptureFinish(): void {
  cancelVoiceCaptureSettle();
  voiceCaptureSettleTimeout = window.setTimeout(() => {
    voiceCaptureSettleTimeout = null;
    if (!capturingVoiceHotkey.value || voiceCapturePressed.size > 0) return;
    if (!voiceCapturedKeys && voiceCaptureEverPressed.length > 0) {
      voiceCapturedKeys = [...voiceCaptureEverPressed];
    }
    void finishVoiceHotkeyCapture();
  }, VOICE_CAPTURE_SETTLE_MS);
}

async function acceptVoiceCaptureEdge(edge: ShortcutCaptureEdge): Promise<void> {
  if (!capturingVoiceHotkey.value) return;
  // 后端只在 preheld 键全部松开后才开始投递边沿：第一条边沿即已武装。
  if (waitingPreheldRelease.value) {
    waitingPreheldRelease.value = false;
    voiceHotkeyMessage.value = "";
  }
  const { key, isPressed } = edge;
  if (!isPressed) {
    voiceCapturePressed.delete(key);
    if (voiceCapturedKeys) {
      voiceCaptureDisplay.value = voiceCapturedKeys;
      if (voiceCapturePressed.size === 0) scheduleVoiceCaptureFinish();
      return;
    }
    // 组合里没有主键时（默认的 左 Ctrl + 左 Win、豆包的"长按右 Alt"都是这种），
    // 在最后一个按键松开时按本次会话按过的全部修饰键落盘。不能只看最后松开
    // 的那个键：Ctrl+Win 先松 Win 会把组合截断成只剩 Ctrl（Bugs/2026-09-27）。
    // 能走到这里说明会话里没有主键（主键按下时即成组），故全部是修饰键。
    // 定稿延后到稳定窗口（见 scheduleVoiceCaptureFinish）：被吞掉的完成键只
    // 会以输入法重放的注入副本形式稍后到达，立即落盘会把它丢掉。
    if (voiceCapturePressed.size === 0 && voiceCaptureEverPressed.length > 0) {
      voiceCaptureDisplay.value = [...voiceCaptureEverPressed];
      scheduleVoiceCaptureFinish();
    }
    return;
  }
  // 已经拿到终止键后继续保持原生拦截，直到本次组合的所有 DOWN 都收到配对 UP。
  if (voiceCapturedKeys) return;
  cancelVoiceCaptureSettle();
  if (key === "escape" && voiceCapturePressed.size === 0) {
    await finishVoiceHotkeyCapture(t("已取消录入"));
    return;
  }
  voiceCapturePressed.add(key);
  if (!voiceCaptureEverPressed.includes(key)) voiceCaptureEverPressed.push(key);
  if (CAPTURE_MODIFIER_KEYS.has(key)) {
    voiceCaptureDisplay.value = [...voiceCapturePressed];
    return;
  }
  voiceCapturedKeys = [...voiceCapturePressed];
  voiceCaptureDisplay.value = voiceCapturedKeys;
}

function handleVoiceCaptureBlur(): void {
  if (capturingVoiceHotkey.value || captureStartingVoiceHotkey.value) {
    void finishVoiceHotkeyCapture(t("窗口失去焦点，已取消录入"));
  }
}

async function refreshVoiceHotkey() {
  try {
    voiceHotkey.value = await getVoiceHoldHotkey();
  } catch (error) {
    voiceHotkeyMessage.value = actionErrorText("get_voice_hold_hotkey", error);
  }
}

/**
 * 初始化只读快捷键与输入工具。工具缺失时按快捷键展示建议引导，不写配置：
 * 右 Alt 无法区分豆包、Vokie 与其他工具，持久化只来自用户显式选择。
 */
async function initializeShortcutSettings(): Promise<void> {
  const [chord, storedTool] = await Promise.all([
    getVoiceHoldHotkey().catch((error) => {
      voiceHotkeyMessage.value = actionErrorText("get_voice_hold_hotkey", error);
      return null;
    }),
    getVoiceInputTool().catch((error) => {
      voiceHotkeyMessage.value = error instanceof Error ? error.message : String(error);
      reportFrontendEvent({
        event: "voice_input_tool_load",
        phase: "completed",
        result: "failed",
        reason: "read_failed_configuration_preserved",
      });
      return null;
    }),
  ]);
  voiceHotkey.value = chord;
  if (storedTool) {
    voiceInputTool.value = storedTool;
    return;
  }
  const derived = deriveToolFromChord(chord);
  voiceInputTool.value = derived;
}

watch(
  () => props.runtime?.platform.connection,
  (snapshot) => {
    if (snapshot) connection.value = snapshot;
  },
  { immediate: true },
);

watch(
  () => props.runtime?.platform.audio,
  (snapshot) => {
    if (snapshot) audio.value = snapshot;
  },
  { immediate: true },
);

const connectionActive = computed(() =>
  [
    "connecting",
    "discovering",
    "awaiting_capabilities",
    "ready",
    "streaming",
    "draining",
    "reconnecting",
    "suspended",
  ].includes(connection.value.phase),
);

const atvvReady = computed(() =>
  ["ready", "streaming", "draining"].includes(connection.value.phase),
);

const audioBusy = computed(() => ["streaming", "draining"].includes(audio.value.phase));

const virtualCableEndpoints = computed(() =>
  audioEndpoints.value.filter((endpoint) => endpoint.isVirtualCableCandidate),
);

const virtualCableInstalled = computed(() => virtualCableEndpoints.value.length > 0);

/**
 * 蓝牙音频不完整提示（2026-10-24 现场）：只在**后端判定为 degraded** 时出现。
 * 满速、未判定、老快照都不提示（`normalizeVoiceAudioHealth` 回落 unknown）。
 */
const voiceAudioDegradedNote = computed(() =>
  shouldShowVoiceAudioDegraded(
    normalizeVoiceAudioHealth(connection.value.voiceAudioHealth),
  )
    ? voiceAudioDegradedText()
    : null,
);

const phaseTone = computed(() => {
  if (connection.value.phase === "failed") return "error";
  if (connection.value.phase === "streaming") return "active";
  if (connection.value.phase === "ready") return "success";
  if (connectionActive.value) return "warning";
  return "pending";
});

/** 已写入控制台的连接/语音错误原文：同一条错误只记一次，避免每秒轮询刷屏。 */
let loggedConnectionError: string | null = null;
let loggedAudioError: string | null = null;

const phaseDetail = computed(() => {
  if (connection.value.lastError) {
    // 原始错误只进控制台（排查用），界面显示与当前语言一致的一句短句。
    if (loggedConnectionError !== connection.value.lastError) {
      loggedConnectionError = connection.value.lastError;
      console.error("[connection] connection_snapshot", connection.value.lastError);
    }
    return t("连接出现问题，请稍后重试。");
  }
  loggedConnectionError = null;
  if (connection.value.capabilities) {
    return t("语音功能已就绪，按住遥控器语音键就能说话");
  }
  return t("连接后即可使用遥控器语音键");
});

/**
 * 状态条标题：已识别型号时显示型号（2026-10-01 Andy 要求——RC001 = 小米蓝牙语音遥控器 2、
 * RC003 = 小米蓝牙语音遥控器 2 Pro）。型号未识别（GATT 2A24 还没读回）或未连接时退回
 * 蓝牙广播名 / 阶段文案，避免把"连接后显示"这类占位当标题。
 */
const connectionTitle = computed(() => {
  const model = connection.value.remoteModel;
  if (connectionActive.value && model !== "unknown") return remoteModelLabel(model);
  return connection.value.remoteName ?? connectionPhaseLabel(connection.value.phase);
});

const audioTone = computed(() => {
  if (routeError.value || audioRoute.value?.phase === "unavailable") return "error";
  if (audio.value.phase === "streaming") return "active";
  if (!captureInput.value.settings.enabled && audioRoute.value?.phase === "manual" && ["ready", "draining"].includes(audio.value.phase)) return "success";
  if (audioRoute.value?.phase !== "paired") return "pending";
  return "success";
});

const audioRouteTitle = computed(() => {
  if (routeError.value) return "声音通道状态读取失败";
  if (!audioRoute.value) return "正在确认声音通道…";
  return { paired: "声音传输已就绪", manual: captureInput.value.settings.enabled ? "使用手动声音通道" : "声音写入已就绪", unconfigured: "请选择声音写入设备", unavailable: "声音通道暂不可用" }[audioRoute.value.phase];
});

const audioDetail = computed(() => {
  if (audio.value.lastError) {
    // 原始错误只进控制台（排查用），界面显示与当前语言一致的一句短句。
    if (loggedAudioError !== audio.value.lastError) {
      loggedAudioError = audio.value.lastError;
      console.error("[connection] audio_snapshot", audio.value.lastError);
    }
    return t("语音设备出现问题，请稍后重试。");
  }
  loggedAudioError = null;
  if (audio.value.selectedEndpointName) return t("语音会写入选中的设备");
  return t("不会自动改动系统默认设备，需要在这里明确选择");
});
const audioRouteDetail = computed(() => {
  if (routeError.value) return captureInput.value.settings.enabled ? "暂时无法确认声音是否进入目标麦克风，请刷新后重试。" : "暂时无法确认声音写入设备状态，请刷新后重试。";
  const route = audioRoute.value;
  if (!route) return captureInput.value.settings.enabled ? "正在检查所选麦克风对应的声音通道。" : "正在检查声音写入设备。";
  if (route.phase === "unconfigured" && !captureInput.value.settings.enabled) return "请选择声音写入设备";
  if (route.phase === "paired") return "遥控器声音会进入所选麦克风，内部声音通道已自动配对。";
  if (route.phase === "manual" && !captureInput.value.settings.enabled) return "声音将写入所选设备；输入工具的麦克风需单独选择。";
  if (route.phase === "manual") return "已保留手动写入端；尚未确认它是否通向所选麦克风，可在高级设置中检查。";
  const reasons: Record<string, string> = {
    capture_missing: "所选麦克风已不可用，请确认设备连接后刷新。",
    render_missing: "未找到所选麦克风对应的声音写入端，请检查虚拟音频设备。",
    ambiguous: "存在多个候选声音通道，无法明确配对，尚未自动选择。",
    unsupported: "此设备无法自动配对声音通道，可在高级设置中检查手动写入端。",
    identity_changed: "声音设备身份发生变化，尚未确认对应通道，请重新选择目标麦克风。",
    render_mismatch: "当前写入端与目标麦克风不属于已确认的同一通道，请重新选择目标麦克风。",
    audio_unready: "声音通道已识别，但写入端尚未就绪，请刷新状态。",
    metadata_unavailable: "无法读取音频设备的关联信息，尚未自动连接声音通道。",
    enumeration_failed: "无法读取音频设备列表，请刷新后重试。",
  };
  return reasons[route.reason ?? ""] ?? (route.phase === "unconfigured" ? "选择目标麦克风后，程序会自动连接对应的声音通道。" : "尚未确认声音通道，请刷新状态或检查高级设置。");
});

async function refreshAudioRoute() {
  const request = ++audioRouteRequest;
  try {
    const snapshot = await getAudioRouteSnapshot();
    if (unmounted || request !== audioRouteRequest) return;
    audioRoute.value = snapshot; routeError.value = "";
    reportAudioRoute(snapshot.phase, snapshot.reason);
  } catch {
    if (unmounted || request !== audioRouteRequest) return;
    audioRoute.value = null; routeError.value = "read_failed";
    reportAudioRoute("unavailable", "snapshot_read_failed");
  }
}

async function refreshConnection() {
  try {
    connection.value = await getConnectionSnapshot();
  } catch (error) {
    operationMessage.value = actionErrorText("get_connection_snapshot", error);
  }
}

async function refreshAudio() {
  try {
    audio.value = await getAudioSnapshot();
    return true;
  } catch (error) {
    audioMessage.value = actionErrorText("get_audio_snapshot", error);
    return false;
  }
}

async function scan() {
  scanning.value = true;
  operationMessage.value = "";
  scanMessage.value = t("正在寻找小米遥控器…");
  try {
    devices.value = await scanPairedRemotes();
    scanMessage.value = devices.value.length
      ? t("找到 {count} 个已配对的小米遥控器", { count: devices.value.length })
      : t("没有找到已配对的小米遥控器");
  } catch (error) {
    devices.value = [];
    scanMessage.value = actionErrorText("scan_paired_remotes", error);
  } finally {
    scanning.value = false;
  }
}

async function connect(device: PairedRemote) {
  connectingDeviceId.value = device.id;
  operationMessage.value = "";
  try {
    connection.value = await connectRemote(device.id);
    operationMessage.value = t("已连接，正在确认语音功能");
  } catch (error) {
    operationMessage.value = actionErrorText("connect_remote", error);
    await refreshConnection();
  } finally {
    connectingDeviceId.value = "";
  }
}

async function disconnect() {
  disconnecting.value = true;
  operationMessage.value = "";
  try {
    connection.value = await disconnectRemote();
    operationMessage.value = t("遥控器连接已释放，本次运行已停止自动重连");
  } catch (error) {
    operationMessage.value = actionErrorText("disconnect_remote", error);
  } finally {
    disconnecting.value = false;
  }
}

async function detectAudioEndpoints() {
  scanningAudio.value = true;
  audioMessage.value = t("正在读取语音设备…");
  try {
    audioEndpoints.value = await listAudioEndpoints();
    audioScanComplete.value = true;
    const virtualCables = audioEndpoints.value.filter(
      (endpoint) => endpoint.isVirtualCableCandidate,
    );
    audioMessage.value = virtualCables.length
      ? t("已检测到 {count} 个 VB-CABLE 语音设备", { count: virtualCables.length })
      : t("未检测到 VB-CABLE；安装完成后需要重启电脑，再重新检测");
  } catch (error) {
    audioEndpoints.value = [];
    audioScanComplete.value = false;
    audioMessage.value = actionErrorText("list_audio_endpoints", error);
  } finally {
    scanningAudio.value = false;
  }
}

async function scanAudio() {
  await Promise.all([detectAudioEndpoints(), scanCaptureInputs(), refreshCaptureInput(), refreshAudio(), refreshAudioRoute()]);
  await refreshDefaultCaptureInputs();
}

async function changeAudioOutput(event: Event) {
  const control = event.target as HTMLSelectElement;
  const requested = control.value;
  control.value = audio.value.selectedEndpointId ?? "";
  const endpoint = audioEndpoints.value.find(item => item.id === requested);
  if (!endpoint) {
    audioMessage.value = t("所选声音写入设备当前不可用");
    reportFrontendEvent({ event: "audio_output_select", phase: "completed", result: "failed", reason: "endpoint_unavailable" });
    return;
  }
  await chooseAudioEndpoint(endpoint);
}

async function chooseAudioEndpoint(endpoint: AudioEndpoint) {
  if (captureBusy.value || selectingEndpointId.value || audioBusy.value || captureInput.value.recoveryPending) return;
  selectingEndpointId.value = endpoint.id;
  audioMessage.value = t("正在打开语音设备…");
  try {
    if (pendingCapture.value) {
      captureBusy.value = true;
      captureInput.value = await setCaptureInput(pendingCapture.value, endpoint.id);
      pendingCapture.value = null;
      captureMessage.value = "";
      await refreshAudio();
      audioMessage.value = t("手动写入端已保存；请按目标麦克风实际收到的声音验证。");
    } else {
      const snapshot = await selectAudioEndpoint(endpoint.id);
      audio.value = snapshot;
      const actualName = snapshot.selectedEndpointName ?? endpoint.name;
      audioMessage.value = t("已选择 {name}", { name: actualName });
    }
    await refreshAudioRoute();
  } catch (error) {
    audioMessage.value = actionErrorText("select_audio_endpoint", error);
    await refreshAudio();
    try { captureInput.value = await getCaptureInput(); } catch { /* keep last confirmed state */ }
  } finally {
    selectingEndpointId.value = "";
    captureBusy.value = false;
  }
}

async function openVbCablePage() {
  openingVbCablePage.value = true;
  try {
    await openVbCableDownloadPage();
    audioMessage.value = t(
      "已打开 VB-CABLE 官方下载页面；安装时需要管理员权限，完成后请重启电脑",
    );
  } catch (error) {
    audioMessage.value = actionErrorText("open_vb_cable_page", error);
  } finally {
    openingVbCablePage.value = false;
  }
}

async function initializeAudio() {
  await refreshAudio();
  await Promise.all([detectAudioEndpoints(), scanCaptureInputs(), refreshAudioRoute()]);
}

function onGainInput(event: Event): void {
  gainDb.value = Number((event.target as HTMLInputElement).value);
}

async function onGainCommit(event: Event): Promise<void> {
  const desired = Number((event.target as HTMLInputElement).value);
  savingGain.value = true;
  gainError.value = "";
  try {
    // 以保存返回值为准回显：越界值会被后端钳制（0–24 dB），界面不假装存了别的数。
    savedGainDb.value = await setGainDb(desired);
    gainDb.value = savedGainDb.value;
  } catch (error) {
    console.error("[connection] set_gain_db", error);
    gainError.value = t("增益没有保存，已恢复到上一次的值。");
    gainDb.value = savedGainDb.value;
  } finally {
    savingGain.value = false;
  }
}

onMounted(async () => {
  void reconcileRc003Capture();
  void refreshCaptureInput();
  void refreshDefaultCaptureInputs();
  window.addEventListener("blur", handleVoiceCaptureBlur);
  void refreshConnection();
  void initializeAudio();
  void initializeShortcutSettings();
  void refreshVokieInstallation();
  void getGainDb()
    .then((value) => {
      gainDb.value = value;
      savedGainDb.value = value;
    })
    .catch((error) => {
      gainError.value = actionErrorText("get_gain_db", error);
    });
  let defaultPoll = 0;
  pollTimer = setInterval(() => {
    void reconcileRc003Capture();
    void refreshCaptureInput();
    void refreshConnection();
    void refreshAudio();
    void refreshAudioRoute();
    if (++defaultPoll % 5 === 0 && !audioBusy.value) void refreshDefaultCaptureInputs();
  }, 1_000);
  const stopCaptureEdges = await subscribeShortcutCaptureEdges((edge) => {
    void acceptVoiceCaptureEdge(edge);
  });
  if (unmounted) {
    stopCaptureEdges();
    return;
  }
  unlistenVoiceCapture = stopCaptureEdges;
});

onUnmounted(() => {
  unmounted = true;
  window.removeEventListener("blur", handleVoiceCaptureBlur);
  if (pollTimer) clearInterval(pollTimer);
  cancelVoiceCaptureSettle();
  if (voiceCaptureTimeout !== null) window.clearTimeout(voiceCaptureTimeout);
  voiceCaptureTimeout = null;
  unlistenVoiceCapture?.();
  unlistenVoiceCapture = null;
  void stopShortcutCapture().catch(() => undefined);
});
</script>

<template>
  <section>
    <header class="page-header">
      <div>
        <h1>{{ tPage("连接") }}</h1>
      </div>
      <span class="badge" :class="phaseTone">{{ connectionPhaseLabel(connection.phase) }}</span>
    </header>

    <!-- ============ 设备：遥控器 + 语音设备 ============ -->
      <article class="card remote-connection-card">
        <div class="card-title-row">
          <div>
            <h2>{{ t("遥控器连接") }}</h2>
          </div>
          <button
            class="primary-button"
            type="button"
            :disabled="scanning || connectionActive || !runtime?.platform.bleScanAvailable"
            @click="scan"
          >
            {{ scanning ? t("扫描中…") : t("扫描已配对设备") }}
          </button>
        </div>

        <div class="status-panel" aria-live="polite">
          <div class="status-copy">
            <div class="status-heading">
              <span class="status-dot" :class="phaseTone"></span>
              <strong class="connection-title">{{ connectionTitle }}</strong>
            </div>
            <small>{{ phaseDetail }}</small>
          </div>
          <button
            v-if="connectionActive"
            class="secondary-button status-action"
            type="button"
            :disabled="disconnecting"
            @click="disconnect"
          >
            {{ disconnecting ? t("断开中…") : t("断开") }}
          </button>
        </div>

        <div v-if="connectionActive" class="chip-row">
          <span class="device-chip">
            <BatteryIndicator :connection="connection" />
          </span>
          <span class="device-chip">
            <span class="status-dot" :class="atvvReady ? 'success' : 'pending'"></span>
            {{ atvvReady ? t("语音按键已就绪") : t("正在确认语音功能") }}
          </span>
          <span v-if="connection.powerNotificationsAvailable" class="device-chip">
            <span class="status-dot success"></span>
            {{ t("睡眠唤醒后自动重连") }}
          </span>
        </div>

        <p v-if="scanMessage" class="muted scan-summary">{{ scanMessage }}</p>
        <p v-if="operationMessage" class="operation-message">{{ operationMessage }}</p>

        <ul v-if="devices.length" class="device-list">
          <li v-for="device in devices" :key="device.id">
            <div><strong>{{ device.name }}</strong><small>{{ remoteModelLabel(device.model) }}</small></div>
            <button
              type="button"
              :disabled="connectionActive || Boolean(connectingDeviceId)"
              @click="connect(device)"
            >
              {{ connectingDeviceId === device.id ? t("连接中…") : t("连接") }}
            </button>
          </li>
        </ul>
      <section class="voice-device-section" aria-labelledby="voice-devices-title">
        <div class="card-title-row">
          <div>
            <h3 id="voice-devices-title">{{ t("声音传输与麦克风") }}</h3>
          </div>
          <button
            class="secondary-button"
            type="button"
            :disabled="scanningAudio || audioBusy || !runtime?.platform.windowsApiAvailable"
            @click="scanAudio()"
          >
            {{ scanningAudio ? t("读取中…") : t("刷新设备列表") }}
          </button>
        </div>

        <section class="audio-output-settings" :aria-label="t('声音写入设备')">
        <div class="audio-output-heading">
          <label for="audio-output-target">{{ t("声音写入设备") }}</label>
        <div class="status-panel audio-route-status" aria-live="polite">
          <div class="status-copy">
            <div class="status-heading">
              <span class="status-dot" :class="audioTone"></span>
              <strong>{{ t(audioRouteTitle) }}</strong>
            </div>
          </div>
        </div>
        </div>
        <select id="audio-output-target" :value="audio.selectedEndpointId ?? ''" :disabled="scanningAudio || captureBusy || Boolean(selectingEndpointId) || audioBusy || !runtime?.platform.windowsApiAvailable || captureInput.recoveryPending" @change="changeAudioOutput">
          <option value="" disabled>{{ t("请选择声音写入设备") }}</option>
          <option v-if="audio.selectedEndpointId && !audioEndpoints.some(endpoint => endpoint.id === audio.selectedEndpointId)" :value="audio.selectedEndpointId" disabled>{{ audio.selectedEndpointName }}</option>
          <option v-for="endpoint in audioEndpoints" :key="endpoint.id" :value="endpoint.id">{{ endpoint.name }}</option>
        </select>
        <p class="muted audio-output-help">{{ t("上游方式：选择声音写入端，不改变 Windows 默认麦克风。VB-CABLE 选 CABLE Input，输入工具选 CABLE Output。") }}</p>
        <p class="audio-output-feedback scan-summary muted" role="status" aria-live="polite" :tabindex="audioMessage || audio.lastError || routeError || audioRoute?.phase === 'unavailable' ? 0 : undefined">{{ audio.lastError ? audioDetail : routeError || audioRoute?.phase === 'unavailable' ? t(audioRouteDetail) : audioMessage }}</p>
        </section>

        <section class="capture-input-settings" :aria-label="t('Windows 默认麦克风（可选）')">
          <h4>{{ t("Windows 默认麦克风（可选）") }}</h4>
          <dl class="windows-default-inputs" aria-live="polite" tabindex="0" :aria-label="t('当前 Windows 默认麦克风')">
            <div><dt>{{ t('普通输入') }}</dt><dd>{{ defaultCaptureName('console') }}</dd></div>
            <div><dt>{{ t('媒体输入') }}</dt><dd>{{ defaultCaptureName('multimedia') }}</dd></div>
            <div><dt>{{ t('通信输入') }}</dt><dd>{{ defaultCaptureName('communications') }}</dd></div>
          </dl>
          <div class="capture-input-target">
          <label for="capture-input-target">{{ t("目标麦克风输入") }}</label>
          <div class="button-row">
            <select id="capture-input-target" :value="selectedCaptureId" :disabled="captureBusy || Boolean(selectingEndpointId) || audioBusy || !runtime?.platform.windowsApiAvailable || captureInput.recoveryPending"
              @change="selectCaptureTarget">
              <option value="">{{ t("请选择输入设备") }}</option>
              <option v-if="captureInput.settings.endpointId && !captureEndpoints.some(e => e.id === captureInput.settings.endpointId)" :value="captureInput.settings.endpointId">{{ captureInput.settings.endpointName }}</option>
              <option v-for="endpoint in captureEndpoints" :key="endpoint.id" :value="endpoint.id">{{ endpoint.name }}</option>
            </select>
            <button id="set-windows-default-input" type="button" class="secondary-button" :disabled="captureBusy || Boolean(selectingEndpointId) || audioBusy || connection.voiceState !== 'idle' || !runtime?.platform.windowsApiAvailable || captureInput.recoveryPending || !captureEndpoints.some(endpoint => endpoint.id === selectedCaptureId)" @click="applyWindowsDefaultInput">{{ t('设为 Windows 默认') }}</button>
          </div>
          <p class="muted">{{ t("手动设置立即生效，不开启自动切换。启用自动切换时会匹配声音写入设备。") }}</p>
          </div>
          <div class="capture-input-behavior">
          <label class="setting-row" for="capture-input-switch">
            <span>{{ t("说话前自动检查并切换") }}</span>
            <input id="capture-input-switch" type="checkbox" :checked="captureInput.settings.enabled"
              :disabled="captureBusy || audioBusy || !runtime?.platform.windowsApiAvailable || !selectedCaptureId || captureInput.recoveryPending"
              @change="changeCaptureSwitch" />
          </label>
          <label class="setting-row capture-input-restore" for="capture-input-restore">
            <span>{{ t("松开后恢复原麦克风") }}</span>
            <input id="capture-input-restore" type="checkbox" :checked="captureInput.settings.enabled && !captureInput.settings.keepSelected"
              :disabled="captureBusy || audioBusy || !runtime?.platform.windowsApiAvailable || !captureInput.settings.endpointId || !captureInput.settings.enabled || captureInput.recoveryPending"
              @change="changeCaptureRestore" />
          </label>
          <p class="muted">{{ t("输入法已选择目标麦克风时，无需开启。") }}{{ t("切换可能增加启动耗时。") }}</p>
          </div>
          <div class="capture-input-feedback capture-input-status" aria-live="polite" tabindex="0">
          <button v-if="captureInput.settings.enabled && audioRoute?.phase === 'unavailable' && captureInput.settings.endpointId" type="button" class="secondary-button" :disabled="captureBusy || audioBusy || !runtime?.platform.windowsApiAvailable || captureInput.recoveryPending" @click="changeCaptureInput(captureInput.settings.enabled)">{{ t("重新连接声音通道") }}</button>
          <p v-if="captureMessage || captureInput.lastError" role="status">{{ (captureMessage || captureInput.lastError) === "normal_roles_split" ? t("普通输入的两个系统角色指向不同设备，无法保证原样恢复，本次未切换。请先在 Windows 声音设置中统一默认输入设备。") : (captureMessage || captureInput.lastError) }}</p>
          <p v-else>{{ capturePhase ? t(capturePhase) : "" }}</p>
          <p v-if="pendingCapture" role="status">{{ t("所选麦克风尚未保存。请在高级诊断中明确选择同一条音频线的声音写入端，两端将一起保存。") }}</p>
          <div v-if="captureInput.recoveryPending" class="info-callout warning">
            <p>{{ t("上次退出未能确认输入设备恢复。如果此后改过麦克风，请保留当前选择。恢复只处理仍符合原会话记录的设备。") }}</p>
            <div class="button-row">
              <button type="button" :disabled="captureBusy" @click="recoverCaptureInput(false)">{{ t("保留当前选择") }}</button>
              <button type="button" class="secondary-button" :disabled="captureBusy" @click="recoverCaptureInput(true)">{{ t("尝试恢复会话前设备") }}</button>
            </div>
          </div>
          </div>
        </section>
        <details class="audio-advanced" :open="showAdvancedAudio" @toggle="showAdvancedAudio = ($event.target as HTMLDetailsElement).open">
          <summary>{{ t("高级：声音传输诊断") }}</summary>
          <template v-if="showAdvancedAudio">
          <p class="muted">{{ t("手动写入端只用于诊断或已有特殊配置。它决定遥控器声音写到哪里，不改变 Windows 默认扬声器；选择扬声器可能直接播放声音，不能代表目标麦克风已收到声音。") }}</p>
          <p class="muted">{{ t("当前写入端") }}：{{ audio.selectedEndpointName ?? audioPhaseLabel(audio.phase) }}</p>
          <p class="muted">{{ t(audioRouteDetail) }}</p>
          <button type="button" class="secondary-button" :disabled="captureBusy || !runtime?.platform.windowsApiAvailable" @click="scanCaptureInputs">{{ t("读取输入设备") }}</button>
          <p v-if="audioMessage || audio.lastError" role="status">{{ audioMessage || audio.lastError }}</p>
        <ul v-if="audioEndpoints.length" class="device-list endpoint-list">
          <li v-for="endpoint in audioEndpoints" :key="endpoint.id">
            <div>
              <strong>{{ endpoint.name }}</strong>
              <strong
                v-if="isRecommendedVoiceEndpoint(endpoint)"
                class="endpoint-recommend"
              >
                {{ t("推荐") }}
              </strong>
              <small v-else>{{ t("仅用于手动诊断") }}</small>
            </div>
            <button
              type="button"
              :disabled="captureBusy || captureInput.recoveryPending || audioBusy || Boolean(selectingEndpointId) || (!pendingCapture && audio.selectedEndpointId === endpoint.id && audioRoute?.phase !== 'unavailable')"
              @click="chooseAudioEndpoint(endpoint)"
            >
              {{
                selectingEndpointId === endpoint.id
                  ? t("正在启用…")
                  : pendingCapture
                    ? t("选择并保存两端")
                  : audio.selectedEndpointId === endpoint.id && audioRoute?.phase !== 'unavailable'
                    ? t("当前设备")
                    : t("选择")
              }}
            </button>
          </li>
        </ul>
          </template>
        </details>

        <div class="gain-row">
          <label class="gain-label" for="audio-gain">{{ t("增益") }}</label>
          <input
            id="audio-gain"
            name="audio-gain"
            class="gain-slider"
            type="range"
            min="0"
            max="24"
            step="1"
            :value="gainDb"
            :disabled="savingGain"
            @input="onGainInput"
            @change="onGainCommit"
          />
          <span class="gain-value">{{ gainDb }} dB</span>
        </div>
        <p class="muted gain-help">
          {{ t("0 dB 保持原始音量；数值越大声音越响，也会放大环境噪声。建议先从 6–12 dB 开始。") }}
        </p>
        <p v-if="gainError" class="error-text" role="alert">{{ gainError }}</p>

        <div v-if="audioScanComplete && !virtualCableInstalled" class="info-callout warning vb-cable-callout">
          <div>
            <strong>{{ t("需要安装 VB-CABLE") }}</strong>
            <p>{{ t("由 VB-Audio 提供的免费虚拟声卡。安装需要管理员权限，完成后需重启电脑。") }}</p>
          </div>
          <div class="button-row">
            <button class="primary-button" type="button" :disabled="openingVbCablePage" @click="openVbCablePage">
              {{ openingVbCablePage ? t("正在打开…") : t("打开官方下载页") }}
            </button>
            <button class="secondary-button" type="button" :disabled="scanningAudio" @click="scanAudio()">
              {{ t("重新检测") }}
            </button>
          </div>
        </div>
      </section>
      </article>

    <!-- ============ 语音输入设置（三步） ============ -->
    <article class="card setup-card">
      <div class="card-title-row">
        <div>
          <h2>{{ t("语音输入设置") }}</h2>
          <p class="muted">{{ t("设置一次，之后按住遥控器语音键说话，松开就出字。") }}</p>
        </div>
      </div>

      <div
        v-if="voiceAudioDegradedNote"
        class="info-callout warning callout-small voice-audio-callout"
        role="status"
      >
        <strong>{{ t("蓝牙音频不完整") }}</strong>
        <p>{{ voiceAudioDegradedNote }}</p>
      </div>

      <div class="setup-columns">
        <!-- ① 输入工具 -->
        <section class="setup-col">
          <div class="col-head">
            <span class="step-index">1</span>
            <strong>{{ t("选择你在用的输入工具") }}</strong>
          </div>
          <div class="tool-list">
            <button
              v-for="card in TOOL_CARDS"
              :key="card.id"
              class="tool-card"
              :class="{ selected: voiceInputTool === card.id }"
              type="button"
              :disabled="savingVoiceInputTool"
              @click="selectVoiceInputTool(card.id)"
            >
              <span class="radio" aria-hidden="true"></span>
              <span>
                <strong>{{ t(card.name) }}</strong>
                <small>{{ t(card.note) }}</small>
              </span>
            </button>
          </div>
        </section>

        <!-- ② 遥控器语音键替你按哪个键 -->
        <section class="setup-col">
          <div class="col-head">
            <span class="step-index">2</span>
            <strong>{{ t("遥控器语音键替你按哪个键") }}</strong>
          </div>

          <div v-if="voiceInputTool === 'doubao'" class="tool-panel">
            <div class="chord-line">
              {{ t("按住遥控器语音键") }} <span class="muted">=</span>
              <span class="key">{{ t("右 Alt") }}</span>
              <span class="pill" :class="toolChordPill('doubao').ok ? 'ok' : 'warn'">
                {{ toolChordPill("doubao").label }}
              </span>
            </div>
            <div class="switch-line">
              <label
                class="toggle-row"
                for="capture-switch-doubao"
                :title="captureUnsupported ? t(CAPTURE_UNSUPPORTED_TITLE) : undefined"
              >
                <span>{{ t("支持更多输入工具") }}</span>
                <span class="toggle-switch-slot">
                  <span
                  v-if="rc003CaptureEnabled === null"
                  class="toggle-placeholder"
                  aria-hidden="true"
                  ></span>
                  <input
                  v-else
                  id="capture-switch-doubao"
                  ref="captureSwitchEl"
                  type="checkbox"
                  class="toggle-input capture-switch"
                  :checked="rc003CaptureEnabled === true"
                  :disabled="rc003CaptureBusy || captureUnsupported"
                  @change="toggleRc003Capture"
                  />
                  <span
                    v-if="captureSwitchState === 'busy'"
                    class="toggle-spinner"
                    aria-hidden="true"
                  ></span>
                </span>
              </label>
            </div>
            <!-- 本机不可用（ARM64，issue #206）：先说清这台电脑用不了，再说别的。
                 与按键页「全按键支持」同一句口径，只有名称随本页入口变化。 -->
            <div v-if="captureUnsupported" class="info-callout warning callout-small">
              {{ t(CAPTURE_UNSUPPORTED_NOTE) }}
            </div>
            <div
              v-if="vokieRunning === true" class="info-callout warning callout-small">
              {{ t("检测到 Vokie 正在运行：它和豆包用的是同一个快捷键（右 Alt），按住遥控器语音键可能唤起 Vokie。要用豆包，请先退出 Vokie。") }}
            </div>
            <p
              v-if="captureFailure || captureRevokedNote || captureInactiveNote"
              class="tiny capture-failure"
            >
              {{ captureFailure || captureRevokedNote || captureInactiveNote }}
            </p>
          </div>

          <div v-else-if="voiceInputTool === 'wechat'" class="tool-panel">
            <div class="chord-line">
              {{ t("按住遥控器语音键") }} <span class="muted">=</span>
              <span class="key">{{ t("左 Ctrl") }}</span>
              <span class="muted">+</span>
              <span class="key">{{ t("左 Win") }}</span>
              <span class="pill" :class="toolChordPill('wechat').ok ? 'ok' : 'warn'">
                {{ toolChordPill("wechat").label }}
              </span>
            </div>
            <p class="tiny muted">{{ t("微信输入法不需要“支持更多输入工具”开关。") }}</p>
          </div>

          <div v-else-if="voiceInputTool === 'vokie'" class="tool-panel">
            <div v-if="vokieInstalled === false" class="info-callout warning callout-small">
              {{ t("没有检测到 Vokie。先安装，再回来设置。") }}
              <div class="button-row">
                <button
                  class="primary-button"
                  type="button"
                  :disabled="openingVokiePage"
                  @click="openVokiePage"
                >
                  {{ openingVokiePage ? t("正在打开…") : t("打开官网 vokie.com") }}
                </button>
                <button
                  class="secondary-button"
                  type="button"
                  :disabled="checkingVokie"
                  @click="refreshVokieInstallation"
                >
                  {{ checkingVokie ? t("检测中…") : t("重新检测") }}
                </button>
              </div>
            </div>
            <div
              v-else-if="vokieRunning === false"
              class="info-callout warning callout-small"
            >
              {{ t("Vokie 没有运行：按住遥控器语音键不会唤起它（如果豆包输入法正在使用，出现的是豆包语音条）。打开 Vokie 后再试。") }}
              <div class="button-row">
                <button
                  class="primary-button"
                  type="button"
                  :disabled="launchingVokie"
                  @click="launchVokieApp"
                >
                  {{ launchingVokie ? t("正在打开…") : t("打开 Vokie") }}
                </button>
                <button
                  class="secondary-button"
                  type="button"
                  :disabled="checkingVokie"
                  @click="refreshVokieInstallation"
                >
                  {{ checkingVokie ? t("检测中…") : t("重新检测") }}
                </button>
              </div>
            </div>
            <div class="chord-line">
              {{ t("按住遥控器语音键") }} <span class="muted">=</span>
              <span class="key">{{ t("右 Alt") }}</span>
              <span class="pill" :class="toolChordPill('vokie').ok ? 'ok' : 'warn'">
                {{ toolChordPill("vokie").label }}
              </span>
            </div>
            <p v-if="vokieCheckMessage" class="tiny muted">{{ vokieCheckMessage }}</p>
          </div>

          <div v-else-if="voiceInputTool === 'other'" class="tool-panel">
            <p class="tiny muted" style="margin-top: 0">{{ t("选与你输入工具里一致的语音键：") }}</p>
            <div class="chip-select">
              <button
                v-for="option in OTHER_CHORD_OPTIONS"
                :key="option.id"
                class="chip"
                type="button"
                :aria-pressed="otherChordActive(option)"
                :disabled="savingVoiceHotkey || capturingVoiceHotkey"
                @click="applyOtherChord(option.keys)"
              >
                {{ t(option.label) }}
              </button>
              <button
                v-if="VOICE_HOTKEY_CUSTOM_CAPTURE_ENABLED"
                class="chip"
                type="button"
                :aria-pressed="!OTHER_CHORD_OPTIONS.some(otherChordActive)"
                :disabled="savingVoiceHotkey || captureStartingVoiceHotkey"
                @click="
                  capturingVoiceHotkey
                    ? finishVoiceHotkeyCapture(t('已取消录入'))
                    : beginVoiceHotkeyCapture()
                "
              >
                {{ capturingVoiceHotkey ? t("录入中…（按 Esc 取消）") : t("自定义组合键") }}
              </button>
              <button v-else class="chip" type="button" disabled>{{ t("自定义组合键（暂未开放）") }}</button>
            </div>
            <div class="switch-line">
              <label
                class="toggle-row"
                for="capture-switch-other"
                :title="captureUnsupported ? t(CAPTURE_UNSUPPORTED_TITLE) : undefined"
              >
                <span>{{ t("支持更多输入工具") }}</span>
                <span class="toggle-switch-slot">
                  <span
                  v-if="rc003CaptureEnabled === null"
                  class="toggle-placeholder"
                  aria-hidden="true"
                  ></span>
                  <input
                  v-else
                  id="capture-switch-other"
                  ref="captureSwitchEl"
                  type="checkbox"
                  class="toggle-input capture-switch"
                  :checked="rc003CaptureEnabled === true"
                  :disabled="rc003CaptureBusy || captureUnsupported"
                  @change="toggleRc003Capture"
                  />
                  <span
                    v-if="captureSwitchState === 'busy'"
                    class="toggle-spinner"
                    aria-hidden="true"
                  ></span>
                </span>
              </label>
            </div>
            <!-- 本机不可用（ARM64，issue #206）：说明句与豆包面板逐字一致。 -->
            <p v-if="captureUnsupported" class="tiny muted">{{ t(CAPTURE_UNSUPPORTED_NOTE) }}</p>
            <p
              v-if="captureFailure || captureRevokedNote || captureInactiveNote"
              class="tiny capture-failure"
            >
              {{ captureFailure || captureRevokedNote || captureInactiveNote }}
            </p>
            <p v-if="capturingVoiceHotkey" class="capture-display voice-hotkey-capture">
              {{
                voiceCaptureDisplay.length
                  ? chordLabel({ keys: voiceCaptureDisplay })
                  : waitingPreheldRelease
                    ? t("检测到仍有按住的按键，请先松开所有按键；松开后即可按新组合，录入将自动开始")
                    : t("请按下你输入工具当前设置的语音键（也可单独按一个修饰键）；按 Esc 取消")
              }}
            </p>
            <p v-if="capturingVoiceHotkey && waitingPreheldRelease" class="tiny muted">
              {{ t("按“自定义组合键”时仍按着键的组合不会完整录入，先松手即可。") }}
            </p>
          </div>

          <p v-if="voiceHotkeyMessage" class="tiny muted voice-hotkey-message">
            {{ voiceHotkeyMessage }}
          </p>
          <div class="info-callout callout-small">
            {{ t("避免其他 App 使用同一个快捷键（Vokie、Chatterfly 这类语音工具会抢在输入法前面）。按住语音键时，如果当前输入法还不是你选的工具，可能需要再按住一次才能正常使用。") }}
          </div>
        </section>

        <!-- ③ 照着做 -->
        <section class="setup-col">
          <div class="col-head">
            <span class="step-index">3</span>
            <strong>{{ t("照着做（只做一次）") }}</strong>
          </div>

          <ol v-if="voiceInputTool === 'doubao'" class="checklist">
            <li><span class="mark">1</span><span>{{ t("豆包麦克风选择与上方相同的目标；使用系统默认时开启麦克风切换") }}</span></li>
            <li><span class="mark">2</span><span>{{ t("豆包长按语音键选 右 Alt") }}</span></li>
            <li><span class="mark">3</span><span>{{ t("切到豆包后，按住遥控器语音键说话") }}</span></li>
          </ol>
          <ol v-else-if="voiceInputTool === 'wechat'" class="checklist">
            <li><span class="mark">1</span><span>{{ t("微信输入法麦克风选择与上方相同的目标；使用系统默认时开启麦克风切换") }}</span></li>
            <li><span class="mark">2</span><span>{{ t("切到微信输入法后，按住遥控器语音键说话") }}</span></li>
          </ol>
          <ol v-else-if="voiceInputTool === 'vokie'" class="checklist">
            <li><span class="mark">1</span><span>{{ t("Vokie 麦克风选择与上方相同的目标；使用系统默认时开启麦克风切换") }}</span></li>
            <li><span class="mark">2</span><span>{{ t("Vokie 快捷键保持默认的 右 Alt") }}</span></li>
            <li><span class="mark">3</span><span>{{ t("在要写字的地方按住遥控器语音键说话") }}</span></li>
          </ol>
          <ol v-else-if="voiceInputTool === 'other'" class="checklist">
            <li><span class="mark">1</span><span>{{ t("输入工具麦克风选择与上方相同的目标；使用系统默认时开启麦克风切换") }}</span></li>
            <li><span class="mark">2</span><span>{{ t("语音键与第 2 步选的键一致") }}</span></li>
            <li><span class="mark">3</span><span>{{ t("切到该输入法后，按住遥控器语音键说话") }}</span></li>
          </ol>
        </section>
      </div>
    </article>

    <details class="usage-hint-details faq">
      <summary>{{ t("常见问题（点开查看）") }}</summary>
      <ul>
        <li>{{ t("为什么语音键要“替你按一个键”？遥控器语音键不是键盘按键，输入法只认键盘按键，所以应用替你按住它。") }}</li>
        <li>{{ t("“支持更多输入工具”和“按键”页的“全按键支持”是同一个开关，两处随时同步；每次开启都会弹出一次系统授权。") }}</li>
        <li>{{ t("“替你按下的键”目前提供 左 Ctrl + 左 Win、右 Alt、左 Alt 和不按键四种；自定义组合键暂未开放。") }}</li>
        <li>{{ t("微信输入法要求按住约半秒以上（需要联网），快速点按不出字是它自己的要求，不是故障。") }}</li>
        <li>{{ t("豆包要是当前输入法，否则按住遥控器语音键只会弹出 Windows 的 Alt 菜单（记事本里会出现“文件(F)、编辑(E)”这类字母）。应用会在你**按住语音键时**把输入法切到所选工具；刚切换过输入工具后的第一次按住如果没反应，松开再按一次即可（第一次那下用于切换输入法）。") }}</li>
        <li>{{ t("如果输入工具使用系统默认麦克风，可开启“说话时切换到所选麦克风”；需要自动恢复时再勾选“松开后恢复原麦克风”。") }}</li>
        <li>{{ t("如果输入工具没有单独的麦克风选项，请选择 CABLE Output；否则让它使用与上方目标相同的麦克风。") }}</li>
        <li>{{ t("Vokie 和豆包用的是同一个快捷键（右 Alt），同一时间只会有一个在响应：Vokie 要在运行中，豆包要是当前输入法；两个同时开着时 Vokie 会抢先。") }}</li>
      </ul>
    </details>
  </section>
  <!-- 授权确认弹窗：与 ButtonsPage 共用同一组件（同一设置项、同一授权流程）；
       每次开启都先出现（toggleRc003Capture 决定，2026-10-03 起无授权判据）。 -->
  <EnhancedCaptureConfirmDialog
    v-if="showCaptureConfirm"
    @confirm="confirmCaptureDialog"
    @close="closeCaptureDialog"
  />
</template>

<style scoped>
.remote-connection-card { margin-bottom: 14px; }
.voice-device-section { margin-top: 18px; padding-top: 16px; border-top: 1px solid var(--border); }
.voice-device-section h3 { margin: 0; font-size: 16px; }
.audio-output-settings { margin-top: 12px; min-width: 0; }
.audio-output-heading { display: flex; align-items: center; justify-content: space-between; gap: 12px; margin-bottom: 6px; }
.audio-output-heading > label { font-size: 13px; font-weight: 600; }
.audio-output-settings > select { width: 100%; min-width: 0; min-height: 34px; }
.audio-output-help { margin: 6px 0 0; font-size: 12.5px; }
.audio-output-feedback { height: 2.8em; margin: 4px 0 0; font-size: 12.5px; line-height: 1.4; overflow: auto; overflow-wrap: anywhere; }
.audio-output-feedback:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
.audio-route-status { margin: 0; padding: 0; border: 0; background: transparent; min-width: 0; font-size: 12px; }
.capture-input-settings { display: grid; grid-template-columns: minmax(0, 1.1fr) minmax(0, 1fr); gap: 12px; margin-top: 10px; padding-top: 12px; border-top: 1px solid var(--border); }
.capture-input-settings > h4 { grid-column: 1 / -1; margin: 0; font-size: 14px; }
.windows-default-inputs { grid-column: 1 / -1; display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 12px; height: 4.4em; margin: 0; font-size: 13px; overflow: auto; overflow-wrap: anywhere; }
.windows-default-inputs dt { color: var(--muted); font-size: 12px; }
.windows-default-inputs dd { margin: 3px 0 0; }
.windows-default-inputs:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
.capture-input-target, .capture-input-behavior { min-width: 0; }
.capture-input-target > label { display: block; margin-bottom: 8px; font-weight: 600; }
.capture-input-target select { min-height: 36px; }
.capture-input-target .button-row { display: grid; grid-template-columns: minmax(0, 1fr) auto; gap: 8px; }
.capture-input-target select { min-width: 0; width: 100%; }
.capture-input-behavior .setting-row { align-items: flex-start; padding: 0; border: 0; gap: 12px; }
.capture-input-behavior .setting-row span { text-align: left; font-weight: 600; color: var(--text-primary); }
.capture-input-behavior input { flex-shrink: 0; }
.capture-input-behavior .capture-input-restore { margin-top: 10px; padding-left: 12px; }
.capture-input-settings p { margin: 10px 0 0; font-size: 13px; }
.capture-input-status { height: 2.8em; line-height: 1.4; overflow-y: auto; overflow-wrap: anywhere; }
.capture-input-status > p:first-child { margin-top: 0; }
.capture-input-status:focus-visible { outline: 2px solid var(--accent); outline-offset: 3px; }
.capture-input-feedback { grid-column: 1 / -1; min-width: 0; }
.audio-advanced { margin-top: 14px; font-size: 13px; }
.audio-advanced > summary { color: var(--muted); cursor: pointer; }
.audio-advanced > summary:focus-visible { outline: 2px solid var(--accent); outline-offset: 3px; }
@media (max-width: 1020px) {
  .capture-input-settings { grid-template-columns: minmax(0, 1fr); }
}
</style>
