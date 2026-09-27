<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import {
  getComponentStatus, performComponentAction, startHidHostEnhancement, getHidHostStatus,
  getHidHostAutoRestore, setHidHostAutoRestore,
  type ComponentAction, type ComponentOperation, type ComponentStatus,
} from "../lib/bridge";

const statuses = ref<ComponentStatus[]>([]);
const loading = ref(false);
const busy = ref(false);
const message = ref("");
const hostStatus = ref("未启动");
const autoRestore = ref(false);
let statusTimer: ReturnType<typeof setInterval> | null = null;
async function toggleAutoRestore() {
  busy.value = true;
  try { autoRestore.value = await setHidHostAutoRestore(!autoRestore.value); }
  catch (error) { message.value = String(error); }
  finally { busy.value = false; }
}
async function startHost() {
  busy.value = true;
  try { hostStatus.value = await startHidHostEnhancement(); }
  catch (error) { hostStatus.value = String(error); }
  finally { busy.value = false; }
}
const hid = computed(() => statuses.value.find((item) => item.component === "hid_enhancement"));
const vb = computed(() => statuses.value.find((item) => item.component === "vb_cable"));
const vbReady = computed(() => vb.value?.installation === "available");
const vendorWizardAvailable = computed(() => vb.value?.allowedActions.includes("open_vendor_wizard") ?? false);
const hidActions: { action: ComponentAction; label: string }[] = [{action:"install",label:"安装按键增强"},{action:"repair",label:"修复按键增强"},{action:"remove",label:"卸载按键增强"}];
function hidStatusText() {
  if (!hid.value) return "正在读取增强驱动状态。";
  if (hid.value.package !== "trusted") return "当前版本尚无 Microsoft 签名的增强驱动安装包，暂不可安装。";
  if (hid.value.installation === "available") return "驱动包与设备通道已通过检测，实体三键效果须按型号验收。";
  return "增强驱动包校验已通过，可执行下方允许的维护操作。";
}
async function maintainHid(action: ComponentAction) {
  busy.value=true;message.value="";
  try {message.value=resultText(await performComponentAction("hid_enhancement",action));await refresh(false);}
  catch {message.value="增强驱动维护未完成，请刷新检测。";}
  finally {busy.value=false;}
}

function vbStatusText() {
  if (!vb.value) return "正在检测 VB-CABLE。";
  if (vbReady.value) return "已检测到 VB-CABLE 服务和音频端点。此结果仅说明检测通过。";
  if (vb.value.installation === "restart_required") return "安装状态需要在重启电脑后重新检测。";
  return "尚未检测到可用的 VB-CABLE。可打开官方安装/卸载向导进行维护。";
}
function resultText(result: ComponentOperation) {
  if (result.outcome === "failed" && result.status.restartRequired) return "维护未完成，Windows 报告需要重启。请保存工作后按需重启，再刷新检测；应用不会自行重启。";
  if (result.component === "hid_enhancement" && result.reason === "operation_in_progress") return "增强通道尚未确认可以安全维护。请先释放所有按键；刚连接时可短按确认并释放，再重试。其他维护操作须先完成。";
  if (result.outcome === "wizard_closed") return "安装/卸载向导已关闭。请刷新检测确认实际状态。";
  if (result.outcome === "cancelled") return "已取消管理员授权，组件状态未改变。";
  if (result.outcome === "timed_out") return "向导可能仍在运行，请先在官方向导完成或取消，再刷新检测。";
  if (result.outcome === "denied") return "管理员授权被拒绝，组件状态未改变。";
  if (result.outcome === "restart_required") return "向导提示需要重启电脑，请重启后刷新检测。";
  if (result.outcome === "completed") return "操作完成，已重新读取组件状态。";
  if (result.reason === "download_failed") return "官网下载失败，未启动安装向导。";
  if (["hash_mismatch", "signature_invalid", "publisher_mismatch", "version_mismatch"].includes(result.reason)) return "组件包校验未通过，已阻止启动维护操作。";
  if (result.reason === "operation_in_progress") return "已有向导操作正在进行中，请先完成或取消后再试。";
  return "操作未完成，请刷新检测后再试。";
}
async function refresh(clearMessage = true) {
  loading.value = true;
  if (clearMessage) message.value = "";
  try {
    statuses.value = await getComponentStatus();
    hostStatus.value = await getHidHostStatus();
    autoRestore.value = await getHidHostAutoRestore();
  }
  catch { message.value = "暂时无法读取组件状态，请稍后重新检测。"; }
  finally { loading.value = false; }
}
async function openVendorWizard() {
  busy.value = true; message.value = "";
  try { message.value = resultText(await performComponentAction("vb_cable", "open_vendor_wizard")); await refresh(false); }
  catch { message.value = "无法打开官方安装/卸载向导，请稍后重试。"; }
  finally { busy.value = false; }
}
onMounted(() => {
  void refresh();
  statusTimer = setInterval(() => {
    void getHidHostStatus().then(value => { hostStatus.value = value; }).catch(() => {});
  }, 1000);
});
onUnmounted(() => { if (statusTimer !== null) clearInterval(statusTimer); });
</script>

<template>
  <section class="component-support">
    <div class="card-title-row"><div><h2>可选增强支持</h2><p class="muted">不会阻塞基础语音、连接或模板设置。</p></div><button type="button" class="secondary-button" :disabled="loading || busy" @click="() => refresh()">{{ loading ? "检测中…" : "刷新检测" }}</button></div>
    <p v-if="message" class="operation-message" aria-live="polite">{{ message }}</p>
    <div class="component-grid">
      <article class="status-panel component-card"><div><strong>RC003 三键增强</strong><small aria-live="polite">{{ hostStatus }}</small><p class="muted">使用已有返回、音量与 TV/Home 配置。增强由独立 Helper 请求管理员授权，主程序与基础语音保持普通权限。</p><label class="restore-choice"><input type="checkbox" :checked="autoRestore" :disabled="busy || loading" @change="toggleAutoRestore">启动时恢复三键增强</label><p class="muted">开启时立即请求一次，以后每次打开无线麦也会请求；每次仍可能出现 UAC。取消授权不会自动重试。关闭后不再自动请求，当前增强在正常退出无线麦时停止。</p></div><div class="button-row"><button type="button" class="secondary-button" :disabled="busy" @click="startHost">启动三键增强</button></div></article>
      <article class="status-panel component-card"><div><strong>1. 按键增强驱动</strong><small>{{ hidStatusText() }}</small><p class="muted">可继续使用模板配置及当前已支持的按键；返回和音量增强仍须在 RC001、RC003 上分别完成真机验收。</p><p v-if="hid?.allowedActions.length" class="muted">点击维护按钮后将请求管理员授权。请先释放遥控器按键；系统若要求重启会明确提示，不会自动重启。</p></div><div class="button-row"><template v-for="item in hidActions" :key="item.action"><button v-if="hid?.allowedActions.includes(item.action)" type="button" class="secondary-button" :disabled="busy" @click="maintainHid(item.action)">{{ item.label }}</button></template></div></article>
      <article class="status-panel component-card"><div><strong>2. VB-CABLE</strong><small>{{ vbStatusText() }}</small><p class="muted">来源：VB-Audio 官方 Pack45，采用 donationware 许可。点击下方按钮即表示由您主动开始：应用会下载并校验官方包，随后由系统请求管理员授权并打开官方安装/卸载向导；不会静默安装、修复或卸载，关闭向导不代表操作成功。</p></div><div class="button-row"><a class="secondary-button" href="https://vb-audio.com/Services/licensing.htm" target="_blank" rel="noreferrer">查看许可与捐赠说明</a><button v-if="vendorWizardAvailable" type="button" class="secondary-button" :disabled="busy" @click="openVendorWizard">打开官方安装/卸载向导</button></div></article>
    </div>
  </section>
</template>

<style scoped>
.component-grid { display: grid; gap: 12px; }
.component-card { flex-direction: column; align-items: flex-start; gap: 12px; }
.component-card small { display: block; margin-top: 4px; }
.component-card p { margin: 7px 0 0; font-size: 0.86rem; }
.restore-choice { display: flex; align-items: center; gap: 8px; margin-top: 12px; }
.restore-choice input { width: auto; }
</style>
