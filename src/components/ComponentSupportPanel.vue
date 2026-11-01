<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { reportFrontendEvent } from "../lib/frontend-diagnostics";
import EnhancedCaptureConfirmDialog from "./EnhancedCaptureConfirmDialog.vue";
import {
  getRc003BridgeSnapshot, getRc003TaskStatus,
  enableRc003Capture, disableRc003Capture, getCaptureSupport,
  type CaptureSupport, type Rc003BridgeSnapshot, type Rc003TaskStatus,
} from "../lib/bridge";

const loading = ref(false);
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
    : { state: "cleanup_blocked", text: "按键支持暂不可用", tone: "error", icon: "!", detail: "上一次关闭的完成状态尚未确认，当前无法继续开启。请刷新状态查看恢复结果。" };
  if (captureReadError.value) return { state: "read_failed", text: "状态读取失败", tone: "error", icon: "!", detail: "暂时无法确认当前状态，请刷新后再操作。" };
  if (captureSupport.value?.available === false) return { state: "unsupported", text: "暂不可用", tone: "warning", icon: "!", detail: "这台电脑暂不支持全按键支持，其他按键不受影响。" };
  const task = captureTask.value;
  if (!task) return { state: "unknown", text: "正在读取状态", tone: "pending", icon: "◷", detail: "正在检查全按键支持。" };
  if (captureError.value?.action === "disable") return { state: "failed", text: "关闭未完成", tone: "error", icon: "!", detail: task.lastError || "尚未确认关闭结果，请刷新状态。" };
  if (captureError.value || task.lastError) return { state: "failed", text: task.enabled ? "运行异常" : "未能开启", tone: "error", icon: "!", detail: task.lastError || (task.enabled ? "全按键支持尚未恢复，请刷新状态或关闭后重试。" : "全按键支持未启动，请确认授权情况后再操作。") };
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
    if (!disposed) captureError.value = { action: "enable", message: "未能开启全按键支持。请确认 Windows 授权已完成，再刷新状态。" };
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
    if (!disposed) captureError.value = { action: "disable", message: "关闭未完成，请刷新状态后重试关闭。" };
    await readCaptureState(true);
  } finally { captureBusy.value = false; captureAction.value = null; }
}
async function refresh() {
  loading.value = true;
  captureError.value = null;
  try { await readCaptureState(); }
  finally { loading.value = false; }
}
onMounted(() => {
  void refresh();
  statusTimer = setInterval(() => {
    if (!captureBusy.value) void readCaptureState();
  }, 1000);
});
onUnmounted(() => { disposed = true; if (statusTimer !== null) clearInterval(statusTimer); });
</script>

<template>
  <section class="component-support">
    <div class="support-heading">
      <div class="support-title">
        <h2>全按键支持</h2>
        <span class="support-status" :class="capturePresentation.tone" role="status" aria-live="polite"><span aria-hidden="true">{{ capturePresentation.icon }}</span> {{ capturePresentation.text }}</span>
      </div>
      <button type="button" class="secondary-button support-refresh" :disabled="loading || captureBusy" @click="() => refresh()">{{ loading ? "检测中…" : "刷新状态" }}</button>
    </div>
    <div class="capture-summary">
      <div class="capture-state-copy" tabindex="0">
        <p>{{ capturePresentation.detail }}</p>
        <p v-if="captureError && !cleanupPending && !captureTask?.lastError" role="alert" class="capture-error">{{ captureError.message }}</p>
      </div>
      <div class="button-row capture-controls">
        <template v-if="cleanupPending">
          <button v-if="captureTask?.canRetryCleanup" type="button" class="secondary-button" :disabled="captureBusy || captureReadError" @click="disableCapture">{{ captureBusy ? "正在恢复…" : "重试关闭" }}</button>
        </template>
        <template v-else>
          <button v-if="!captureTask?.enabled || captureNeedsRestart" type="button" class="primary-button" :disabled="!captureCanEnable" @click="requestCapture">{{ captureBusy ? "正在开启…" : captureTask?.enabled ? "重新授权并启动" : "开启全按键支持" }}</button>
          <button v-if="captureTask?.enabled" type="button" class="secondary-button" :disabled="captureBusy || captureReadError" @click="disableCapture">{{ captureBusy ? "处理中…" : "关闭全按键支持" }}</button>
        </template>
      </div>
    </div>
    <details class="capture-help">
      <summary>授权与自动恢复说明</summary>
      <p>每次开启需确认 Windows 管理员授权；保持开启时，重新打开无线麦会自动恢复。关闭后不再自动恢复，重新开启需要再次授权。</p>
    </details>
    <EnhancedCaptureConfirmDialog v-if="confirmCapture" @close="confirmCapture = false" @confirm="enableCapture" />
  </section>
</template>

<style scoped>
.component-support { min-width: 0; }
.support-heading, .support-title { display: flex; align-items: center; flex-wrap: wrap; gap: 10px; }
.support-heading { justify-content: space-between; }
.support-title h2 { margin: 0; }
.support-refresh { flex-shrink: 0; }
.support-status { display: inline-flex; align-items: center; gap: 5px; max-width: 100%; padding: 4px 9px; border: 1px solid currentColor; border-radius: 6px; font-size: 12px; font-weight: 600; line-height: 1.4; }
.support-status.pending { color: var(--text-control); background: var(--pending-surface); }
.support-status.warning { color: var(--warning-text); background: var(--warning-surface); }
.support-status.success { color: var(--success-text); background: var(--success-surface); }
.support-status.error { color: var(--error-text); background: var(--error-surface); }
.capture-summary { display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 12px 20px; margin-top: 12px; }
.capture-state-copy { flex: 1 1 260px; min-width: 0; height: 3.2em; overflow: auto; overflow-wrap: anywhere; font-size: 13px; line-height: 1.6; }
.capture-state-copy:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
.capture-state-copy p { margin: 0; font-size: 13px; line-height: 1.6; }
.capture-state-copy .capture-error { margin-top: 7px; color: var(--error-text); }
.capture-controls { flex-shrink: 0; justify-content: flex-start; }
.capture-help { margin-top: 12px; color: var(--text-secondary); font-size: 12px; }
.capture-help summary { cursor: pointer; width: fit-content; max-width: 100%; }
.capture-help p { margin: 8px 0 0; line-height: 1.6; }
@media (max-width: 640px) {
  .support-heading { align-items: flex-start; }
  .support-title { gap: 7px; }
  .capture-controls { width: 100%; }
}
</style>
