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
watch(() => props.runtime?.platform.connection, (snapshot) => { if (snapshot) connection.value = snapshot; }, { immediate: true });
async function refreshConnection() { try { connection.value = await getConnectionSnapshot(); } catch { message.value = "暂时无法读取遥控器状态。"; } }
async function scan() { scanning.value = true; message.value = ""; try { devices.value = await scanPairedRemotes(); message.value = devices.value.length ? "已找到 Windows 中已配对的遥控器。" : "未找到已配对的遥控器，请先在 Windows 蓝牙设置中完成配对。"; } catch { message.value = "扫描未完成，请稍后重试。"; } finally { scanning.value = false; } }
async function connect(device: PairedRemote) { connectingDeviceId.value = device.id; message.value = ""; try { connection.value = await connectRemote(device.id); message.value = "已发起连接，正在确认语音能力。"; } catch { message.value = "连接未完成，请确认遥控器仍在附近并重试。"; } finally { connectingDeviceId.value = ""; } }
async function openSettings() { try { await openBluetoothSettings(); } catch { message.value = "无法打开 Windows 蓝牙设置。"; } }
onMounted(() => { void refreshConnection(); });
</script>

<template>
  <section>
    <header class="page-header"><div><h1>驱动与配对</h1><p class="muted">可先检查增强支持，再连接已配对的遥控器。基础语音和模板不依赖增强驱动。</p></div></header>
    <div class="guide-grid">
      <article class="card"><h2>1. 可选增强支持</h2><p class="muted">按键增强驱动目前只影响额外按键能力，不影响基础语音和模板。</p><ComponentSupportPanel /></article>
      <article class="card"><h2>2. 遥控器配对与连接</h2><div v-if="ready" class="info-callout"><strong>已完成连接</strong><p>{{ connection?.remoteName ?? remoteModelLabel(connection?.remoteModel ?? 'unknown') }} 已就绪，可直接使用语音和按键设置。实际语音仍需现场按住语音键确认。</p></div><div class="pairing-guide"><strong>重新配对方法</strong><p class="muted">RC003：同时长按菜单和主页，直到遥控器进入配对模式。RC001：请按随附说明书进入配对模式。</p><div class="button-row"><button type="button" class="secondary-button" :disabled="scanning" @click="openSettings">打开 Windows 蓝牙设置</button><button type="button" :disabled="scanning" @click="scan">{{ scanning ? "扫描中…" : "扫描已配对遥控器" }}</button></div></div><p v-if="connection" class="muted">当前状态：{{ connectionPhaseLabel(connection.phase) }}</p><ul v-if="devices.length" class="device-list"><li v-for="device in devices" :key="device.id"><div><strong>{{ device.name }}</strong><small>{{ device.isSupportedCandidate ? `${remoteModelLabel(device.model)} · Windows 已配对并被 SayAll 发现` : "不支持的设备" }}</small></div><button type="button" :disabled="Boolean(connectingDeviceId) || !device.isSupportedCandidate" @click="connect(device)">{{ connectingDeviceId === device.id ? "连接中…" : "连接" }}</button></li></ul><p v-if="message" class="operation-message" aria-live="polite">{{ message }}</p></article>
    </div>
  </section>
</template>

<style scoped>
.guide-grid { display: grid; gap: 14px; grid-template-columns: 1fr; align-items: start; }
.guide-grid > .card :deep(.component-support) { margin: 12px 0 0; padding: 0; border: 0; box-shadow: none; background: transparent; }
.pairing-guide { margin-top: 12px; }
.pairing-guide p { margin: 5px 0 10px; }
</style>
