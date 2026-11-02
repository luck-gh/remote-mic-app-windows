<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { reportFrontendEvent } from "../lib/frontend-diagnostics";
import EnhancedCaptureConfirmDialog from "./EnhancedCaptureConfirmDialog.vue";
import {
  getRc003BridgeSnapshot, getRc003TaskStatus,
  enableRc003Capture, disableRc003Capture, getCaptureSupport,
  type CaptureSupport, type Rc003BridgeSnapshot, type Rc003TaskStatus,
} from "../lib/bridge";

const captureTask = ref<Rc003TaskStatus | null>(null);
const captureBridge = ref<Rc003BridgeSnapshot | null>(null);
const captureBusy = ref(false);
const captureAction = ref<"enable" | "disable" | null>(null);
const captureError = ref<{ action: "enable" | "disable"; message: string } | null>(null);
const captureReadError = ref(false);
const captureSupport = ref<CaptureSupport | null>(null);
const confirmCapture = ref(false);
const cleanupPending = computed(() => captureTask.value?.cleanupPending === true);
const captureCanEnable = computed(() => Boolean(captureTask.value) && captureSupport.value?.available !== false && !captureBusy.value && !captureReadError.value && !cleanupPending.value);
const captureNeedsRestart = computed(() => captureTask.value?.enabled && (captureBridge.value?.phase === "failed" || Boolean(captureTask.value.lastError)));
const capturePresentation = computed(() => {
  if (captureBusy.value) {
    if (captureAction.value === "disable" && cleanupPending.value) return { state: "recovering", text: "正在恢复按键状态", tone: "warning", icon: "◷", detail: "正在清理上次会话并确认按键释放，请稍候。" };
    return captureAction.value === "disable"
      ? { state: "stopping", text: "正在关闭", tone: "warning", icon: "◷", detail: "正在释放按键并结束本次支持，请稍候。" }
      : { state: "starting", text: "正在开启", tone: "warning", icon: "◷", detail: "请完成 Windows 管理员授权，随后等待启动结果。" };
  }
  if (cleanupPending.value) return captureTask.value?.canRetryCleanup
    ? { state: "cleanup_pending", text: "关闭待完成", tone: "warning", icon: "◷", detail: "上一次关闭尚未完成。重试关闭会自动恢复残留的按键状态，完成后可再次开启。" }
    : { state: "cleanup_blocked", text: "按键支持暂不可用", tone: "error", icon: "!", detail: "上一次关闭的完成状态尚未确认，当前无法继续开启，正在自动检查恢复结果。" };
  if (captureReadError.value) return { state: "read_failed", text: "状态读取失败", tone: "error", icon: "!", detail: "暂时无法确认当前状态，正在自动重试。" };
  if (captureSupport.value?.available === false) return { state: "unsupported", text: "暂不可用", tone: "warning", icon: "!", detail: "这台电脑暂不支持全按键支持，其他按键不受影响。" };
  const task = captureTask.value;
  if (!task) return { state: "unknown", text: "正在读取状态", tone: "pending", icon: "◷", detail: "正在检查全按键支持。" };
  if (captureError.value?.action === "disable") return { state: "failed", text: "关闭未完成", tone: "error", icon: "!", detail: task.lastError || "尚未确认关闭结果，正在自动检查。" };
  if (captureError.value || task.lastError) return { state: "failed", text: task.enabled ? "运行异常" : "未能开启", tone: "error", icon: "!", detail: task.lastError || (task.enabled ? "全按键支持尚未恢复，可关闭后重试或重新授权并启动。" : "全按键支持未启动，请再次开启并确认 Windows 授权。") };
  if (!task.enabled) return { state: "disabled", text: "已关闭", tone: "pending", icon: "—", detail: "开启后可使用 RC003 上已配置的按键；RC001 保持原有输入方式。" };
  const snapshot = captureBridge.value;
  if (snapshot?.phase === "failed") return { state: "failed", text: "运行异常", tone: "error", icon: "!", detail: "全按键支持启动失败，可重新授权并启动。" };
  if (!snapshot || snapshot.phase !== "connected") return { state: "waiting", text: "等待启动完成", tone: "warning", icon: "◷", detail: "开启请求已提交，按键支持尚未就绪。" };
  if (!snapshot.targetUsages.length) return { state: "no_mapping", text: "等待按键配置", tone: "warning", icon: "◷", detail: "当前窗口没有需要接管的按键，请在“按键”或“模板”页检查配置。" };
  if (snapshot.targetUsages.some(usage => !snapshot.ownedUsages.includes(usage))) {
    return { state: "applying", text: "正在应用按键配置", tone: "warning", icon: "◷", detail: "连接已建立，正在等待当前按键配置生效。" };
  }
  return { state: "active", text: "已开启", tone: "success", icon: "✓", detail: "当前窗口已配置的按键已就绪。" };
});
watch(() => capturePresentation.value.state, (state) => {
  reportFrontendEvent({ event: "capture_support_state", phase: "completed", result: ["failed", "read_failed", "cleanup_pending", "cleanup_blocked"].includes(state) ? "failed" : state === "active" || state === "disabled" ? "passed" : "unknown", reason: state });
});
let statusTimer: ReturnType<typeof setInterval> | null = null;
let disposed = false;
let captureReadPending = false;
let captureGeneration = 0;
let captureReadGeneration = 0;
async function readCaptureState(force = false) {
  if ((captureReadPending && !force) || disposed) return;
  captureReadPending = true;
  const generation = captureGeneration;
  const readGeneration = ++captureReadGeneration;
  try {
    const [task, bridge, support] = await Promise.all([getRc003TaskStatus(), getRc003BridgeSnapshot(), getCaptureSupport().catch(() => null)]);
    if (!disposed && generation === captureGeneration && readGeneration === captureReadGeneration) {
      captureTask.value = task;
      captureBridge.value = bridge;
      if (support) captureSupport.value = support;
      captureReadError.value = false;
      if (captureError.value?.action === "disable" && !task.enabled && !task.cleanupPending && !task.lastError) captureError.value = null;
      if (captureError.value?.action === "enable" && task.enabled && !task.cleanupPending && !task.lastError
        && bridge.phase === "connected" && bridge.targetUsages.length > 0
        && bridge.targetUsages.every(usage => bridge.ownedUsages.includes(usage))) captureError.value = null;
    }
  } catch {
    if (!disposed && generation === captureGeneration && readGeneration === captureReadGeneration) captureReadError.value = true;
  } finally { if (readGeneration === captureReadGeneration) captureReadPending = false; }
}
function requestCapture() {
  if (captureCanEnable.value) confirmCapture.value = true;
}
async function enableCapture() {
  confirmCapture.value = false;
  if (!captureCanEnable.value) return;
  captureGeneration += 1;
  captureBusy.value = true;
  captureAction.value = "enable";
  captureError.value = null;
  try {
    const task = await enableRc003Capture();
    if (!disposed) captureTask.value = task;
    try {
      const bridge = await getRc003BridgeSnapshot();
      if (!disposed) { captureBridge.value = bridge; captureReadError.value = false; }
    } catch {
      if (!disposed) captureReadError.value = true;
    }
  } catch {
    if (!disposed) captureError.value = { action: "enable", message: "未能开启全按键支持。请再次开启并完成 Windows 授权。" };
    await readCaptureState(true);
  } finally { captureBusy.value = false; captureAction.value = null; }
}
async function disableCapture() {
  if (captureBusy.value) return;
  captureGeneration += 1;
  captureBusy.value = true;
  captureAction.value = "disable";
  captureError.value = null;
  try {
    const task = await disableRc003Capture();
    if (!disposed) { captureTask.value = task; captureReadError.value = false; }
  } catch {
    if (!disposed) captureError.value = { action: "disable", message: "关闭未完成，正在自动检查释放结果；确认后可重试关闭。" };
    await readCaptureState(true);
  } finally { captureBusy.value = false; captureAction.value = null; }
}
function toggleCapture(event: Event) {
  const input = event.currentTarget as HTMLInputElement;
  // The task readback owns the switch, including a cancelled authorization.
  input.checked = captureTask.value?.enabled === true;
  if (captureBusy.value || captureReadError.value || cleanupPending.value) return;
  if (captureTask.value?.enabled) void disableCapture();
  else requestCapture();
}
const showCaptureNotice = computed(() => !['active', 'disabled'].includes(capturePresentation.value.state));
const captureToggleDisabled = computed(() => captureBusy.value || captureReadError.value || cleanupPending.value || !captureTask.value || (!captureTask.value.enabled && !captureCanEnable.value));
onMounted(() => {
  void readCaptureState();
  statusTimer = setInterval(() => {
    if (!captureBusy.value) void readCaptureState();
  }, 1000);
});
onUnmounted(() => { disposed = true; if (statusTimer !== null) clearInterval(statusTimer); });
</script>

<template>
  <section class="component-support">
    <div class="support-heading">
      <slot name="prefix" />
      <label class="toggle-row capture-controls" :title="capturePresentation.text">
        <span>全按键支持</span>
        <input class="toggle-input" type="checkbox" aria-label="全按键支持" :checked="captureTask?.enabled === true" :disabled="captureToggleDisabled" :aria-busy="captureBusy" @change="toggleCapture" />
      </label>
      <slot />
      <slot name="device" />
    </div>
    <div class="capture-summary" tabindex="0">
      <div v-show="showCaptureNotice" class="capture-state-copy" :class="capturePresentation.tone">
        <span class="support-status" role="status" aria-live="polite">{{ capturePresentation.text }}</span>
        <span> · {{ capturePresentation.detail }}</span>
        <p v-if="captureError && !cleanupPending && !captureTask?.lastError" role="alert" class="capture-error">{{ captureError.message }}</p>
        <button v-if="cleanupPending && captureTask?.canRetryCleanup" type="button" class="capture-recovery" :disabled="captureBusy || captureReadError" @click="disableCapture">{{ captureBusy ? "正在恢复…" : "重试关闭" }}</button>
        <button v-if="!cleanupPending && captureNeedsRestart" type="button" class="capture-recovery" :disabled="!captureCanEnable" @click="requestCapture">重新授权并启动</button>
      </div>
      <slot name="feedback" />
    </div>
    <EnhancedCaptureConfirmDialog v-if="confirmCapture" @close="confirmCapture = false" @confirm="enableCapture" />
  </section>
</template>

<style scoped>
.component-support { min-width: 0; }
.support-heading { display: flex; align-items: center; flex-wrap: wrap; gap: 8px; min-width: 0; }
.capture-controls { flex: 0 0 auto; gap: 6px; font-size: 12px; white-space: nowrap; }
.capture-controls .toggle-input { width: 36px; height: 22px; }
.capture-controls .toggle-input::after { width: 16px; height: 16px; }
.capture-controls .toggle-input::after { top: 3px; }
.capture-summary { height: 20px; overflow: auto; overflow-wrap: anywhere; margin-top: 0; font-size: 12px; line-height: 20px; }
.capture-summary:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
.support-status { font-weight: 600; }
.capture-state-copy { display: inline; }
.capture-state-copy.warning { color: var(--warning-text); }
.capture-state-copy.error, .capture-error { color: var(--error-text); }
.capture-state-copy p { margin: 0; }
.capture-recovery { display: inline; margin-left: 8px; padding: 0; border: 0; background: none; color: var(--accent); font-size: inherit; text-decoration: underline; }
</style>
