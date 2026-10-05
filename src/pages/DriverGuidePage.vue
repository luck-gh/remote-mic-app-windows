<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import ComponentSupportPanel from "../components/ComponentSupportPanel.vue";
import {
  connectRemote, connectionPhaseLabel, getConnectionSnapshot, openBluetoothSettings, remoteModelLabel, scanPairedRemotes,
  type ConnectionSnapshot, type PairedRemote, type RuntimeSnapshot,
} from "../lib/bridge";

const props = defineProps<{ runtime: RuntimeSnapshot | null }>();
const connection = ref<ConnectionSnapshot | null>(null);
const devices = ref<PairedRemote[]>([]);
const scanning = ref(false);
const connectingDeviceId = ref("");
const message = ref("");
const ready = computed(() => connection.value?.capabilities != null && ["ready", "streaming", "draining"].includes(connection.value.phase));
const connectionPresentation = computed(() => {
  const phase = connection.value?.phase;
  if (phase === "failed") return { tone: "error", icon: "!" };
  if (phase && ["ready", "streaming", "draining"].includes(phase)) return { tone: "success", icon: "✓" };
  if (phase && ["connecting", "discovering", "awaiting_capabilities", "reconnecting"].includes(phase)) return { tone: "warning", icon: "◷" };
  return { tone: "pending", icon: "—" };
});
watch(() => props.runtime?.platform.connection, (snapshot) => { if (snapshot) connection.value = snapshot; }, { immediate: true });
async function refreshConnection() { try { connection.value = await getConnectionSnapshot(); } catch { message.value = "暂时无法读取遥控器状态。"; } }
async function scan() { scanning.value = true; message.value = ""; try { devices.value = await scanPairedRemotes(); message.value = devices.value.length ? "已找到 Windows 中已配对的遥控器。" : "未找到已配对的遥控器，请先在 Windows 蓝牙设置中完成配对。"; } catch { message.value = "扫描未完成，请稍后重试。"; } finally { scanning.value = false; } }
async function connect(device: PairedRemote) { connectingDeviceId.value = device.id; message.value = ""; try { connection.value = await connectRemote(device.id); message.value = "已发起连接，正在确认语音能力。"; } catch { message.value = "连接未完成，请确认遥控器仍在附近并重试。"; } finally { connectingDeviceId.value = ""; } }
async function openSettings() { try { await openBluetoothSettings(); } catch { message.value = "无法打开 Windows 蓝牙设置。"; } }
onMounted(() => { void refreshConnection(); });
</script>

<template>
  <section class="driver-guide">
    <header class="page-header"><div><h1>驱动与配对</h1><p class="muted">管理遥控器的全按键支持、语音组件和蓝牙连接。</p></div></header>
    <div class="guide-grid">
      <article class="card"><ComponentSupportPanel /></article>
      <article class="card connection-card">
        <div class="connection-heading">
          <h2>遥控器蓝牙连接</h2>
          <div class="connection-status" role="status" aria-live="polite"><span class="badge driver-status" :class="connectionPresentation.tone"><span aria-hidden="true">{{ connectionPresentation.icon }}</span> {{ ready ? "蓝牙已连接" : connection ? connectionPhaseLabel(connection.phase) : "正在读取状态" }}</span></div>
        </div>
        <p class="connection-description"><template v-if="ready">{{ connection?.remoteName ?? remoteModelLabel(connection?.remoteModel ?? 'unknown') }}已连接，语音按下开始、松开结束。</template><template v-else>蓝牙连接用于遥控器通信和语音输入。</template>全按键支持的状态单独显示在上方。</p>
        <details class="pairing-guide">
          <summary>配对与重新连接</summary>
          <div class="pairing-content">
            <strong>重新配对方法</strong>
            <p class="muted">RC003：同时长按菜单和主页，直到遥控器进入配对模式。RC001：请按随附说明书进入配对模式。</p>
            <div class="button-row"><button type="button" class="secondary-button" :disabled="scanning" @click="openSettings">打开 Windows 蓝牙设置</button><button type="button" class="secondary-button" :disabled="scanning" @click="scan">{{ scanning ? "扫描中…" : "扫描已配对遥控器" }}</button></div>
          </div>
        </details>
        <ul v-if="devices.length" class="device-list"><li v-for="device in devices" :key="device.id"><div><strong>{{ device.name }}</strong><small>{{ device.isSupportedCandidate ? `${remoteModelLabel(device.model)} · Windows 已配对并被 SayAll 发现` : "不支持的设备" }}</small></div><button type="button" :disabled="Boolean(connectingDeviceId) || !device.isSupportedCandidate" @click="connect(device)">{{ connectingDeviceId === device.id ? "连接中…" : "连接" }}</button></li></ul>
        <p v-if="message" class="operation-message" aria-live="polite">{{ message }}</p>
      </article>
    </div>
  </section>
</template>

<style scoped>
.guide-grid { display: grid; gap: 14px; grid-template-columns: 1fr; align-items: start; }
.guide-grid > .card { min-width: 0; }
.connection-heading { display: flex; align-items: center; flex-wrap: wrap; gap: 10px; }
.connection-heading h2 { margin: 0; }
.connection-description { margin: 12px 0 0; font-size: 13px; }
.pairing-guide { margin-top: 12px; }
.pairing-guide summary { cursor: pointer; width: fit-content; font-size: 12px; color: var(--text-secondary); }
.pairing-content { margin-top: 12px; font-size: 13px; }
.pairing-guide p { margin: 5px 0 10px; }
.pairing-content .button-row { justify-content: flex-start; }
.driver-guide :deep(:is(.primary-button, .secondary-button, .device-list button)) {
  min-height: 34px;
  border: 1px solid var(--text-secondary);
  font-weight: 600;
  line-height: 1.4;
  white-space: normal;
  overflow-wrap: anywhere;
  text-decoration: none;
}
.driver-guide :deep(.primary-button) { color: var(--text-on-accent); background: var(--accent); border-color: var(--accent); }
.driver-guide :deep(.primary-button:not(:disabled):hover) { background: var(--accent-dark); border-color: var(--accent-dark); }
.driver-guide :deep(:is(.secondary-button, .device-list button)) { color: var(--text-control); background: var(--surface-control); }
.driver-guide :deep(:is(.secondary-button, .device-list button):not(:disabled):hover) { background: var(--pending-surface); border-color: var(--text-control); }
.driver-guide :deep(:is(.primary-button, .secondary-button, .device-list button):not(:disabled):active) { box-shadow: inset 0 2px 0 var(--shadow-control); transform: translateY(1px); }
.driver-guide :deep(:is(a, summary):focus-visible) { outline: 2px solid var(--accent); outline-offset: 3px; border-radius: 4px; }
.driver-guide :deep(button:disabled) { opacity: 1; color: var(--text-secondary); background: var(--surface-subtle); border-color: var(--text-secondary); border-style: dashed; box-shadow: none; }
.driver-status { gap: 5px; padding: 4px 9px; border: 1px solid currentColor; border-radius: 6px; white-space: normal; overflow-wrap: anywhere; font-size: 12px; line-height: 1.4; }
.driver-status.pending { color: var(--text-control); background: var(--pending-surface); }
.driver-status.warning { color: var(--warning-text); background: var(--warning-surface); }
.driver-status.success { color: var(--success-text); background: var(--success-surface); }
.driver-status.error { color: var(--error-text); background: var(--error-surface); }
.connection-status { display: flex; align-items: center; }
</style>
