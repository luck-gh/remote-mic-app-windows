<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import {
  getComponentStatus, performComponentAction,
  type ComponentOperation, type ComponentStatus,
} from "../lib/bridge";

const statuses = ref<ComponentStatus[]>([]);
const loading = ref(false);
const busy = ref(false);
const message = ref("");
const confirmingVendorWizard = ref(false);
const hid = computed(() => statuses.value.find((item) => item.component === "hid_enhancement"));
const vb = computed(() => statuses.value.find((item) => item.component === "vb_cable"));
const vbReady = computed(() => vb.value?.installation === "available");
const vendorWizardAvailable = computed(() => vb.value?.allowedActions.includes("open_vendor_wizard") ?? false);

function vbStatusText() {
  if (!vb.value) return "正在检测 VB-CABLE。";
  if (vbReady.value) return "已检测到 VB-CABLE 服务和音频端点。此结果仅说明检测通过。";
  if (vb.value.installation === "restart_required") return "安装状态需要在重启电脑后重新检测。";
  return "尚未检测到可用的 VB-CABLE。可打开官方安装/卸载向导进行维护。";
}
function resultText(result: ComponentOperation) {
  if (result.outcome === "wizard_closed") return "安装/卸载向导已关闭。请刷新检测确认实际状态。";
  if (result.outcome === "cancelled") return "已取消管理员授权，组件状态未改变。";
  if (result.outcome === "timed_out") return "向导可能仍在运行，请先在官方向导完成或取消，再刷新检测。";
  if (result.outcome === "denied") return "管理员授权被拒绝，组件状态未改变。";
  if (result.outcome === "restart_required") return "向导提示需要重启电脑，请重启后刷新检测。";
  if (result.outcome === "completed") return "操作完成，已重新读取组件状态。";
  if (result.reason === "download_failed") return "官网下载失败，未启动安装向导。";
  if (["hash_mismatch", "signature_invalid", "publisher_mismatch", "version_mismatch"].includes(result.reason)) return "下载包校验未通过，已阻止启动安装向导。";
  if (result.reason === "operation_in_progress") return "已有向导操作正在进行中，请先完成或取消后再试。";
  return "操作未完成，请刷新检测后再试。";
}
async function refresh(clearMessage = true) {
  loading.value = true;
  if (clearMessage) message.value = "";
  try { statuses.value = await getComponentStatus(); }
  catch { message.value = "暂时无法读取组件状态，请稍后重新检测。"; }
  finally { loading.value = false; }
}
async function openVendorWizard() {
  confirmingVendorWizard.value = false; busy.value = true; message.value = "";
  try { message.value = resultText(await performComponentAction("vb_cable", "open_vendor_wizard")); await refresh(false); }
  catch { message.value = "无法打开官方安装/卸载向导，请稍后重试。"; }
  finally { busy.value = false; }
}
onMounted(() => { void refresh(); });
</script>

<template>
  <section class="component-support">
    <div class="card-title-row"><div><h2>可选增强支持</h2><p class="muted">不会阻塞基础语音、连接或模板设置。</p></div><button type="button" class="secondary-button" :disabled="loading || busy" @click="() => refresh()">{{ loading ? "检测中…" : "刷新检测" }}</button></div>
    <p v-if="message" class="operation-message" aria-live="polite">{{ message }}</p>
    <div class="component-grid">
      <article class="status-panel component-card"><div><strong>按键增强驱动</strong><small>{{ hid?.installation === "not_implemented" ? "当前测试版暂未提供可安装的增强驱动。" : "正在读取增强驱动状态。" }}</small><p class="muted">基础语音与模板不受影响；增强能力将在可安装组件提供后另行开放。</p></div></article>
      <article class="status-panel component-card"><div><strong>VB-CABLE</strong><small>{{ vbStatusText() }}</small><p class="muted">来源：VB-Audio 官方 Pack45。安装/卸载由官方向导完成，可能请求管理员授权；关闭向导不代表操作成功。</p></div><button v-if="vendorWizardAvailable" type="button" class="secondary-button" :disabled="busy" @click="confirmingVendorWizard = true">打开官方安装/卸载向导</button></article>
    </div>
    <div v-if="confirmingVendorWizard" class="info-callout warning"><strong>打开 VB-CABLE 官方向导</strong><p>将从 VB-Audio 官方来源获取 Pack45，并在校验后启动独立的管理员安装/卸载向导。VB-CABLE 为 donationware；请先阅读许可与捐赠说明。不会静默安装、修复或卸载。</p><div class="button-row"><a class="secondary-button" href="https://vb-audio.com/Services/licensing.htm" target="_blank" rel="noreferrer">查看许可与捐赠说明</a><button type="button" :disabled="busy" @click="openVendorWizard">继续打开向导</button><button type="button" class="secondary-button" :disabled="busy" @click="confirmingVendorWizard = false">取消</button></div></div>
  </section>
</template>

<style scoped>
.component-grid { display: grid; gap: 12px; }
.component-card { align-items: flex-start; gap: 12px; }
.component-card small { display: block; margin-top: 4px; }
.component-card p { margin: 7px 0 0; font-size: 0.86rem; }
</style>
