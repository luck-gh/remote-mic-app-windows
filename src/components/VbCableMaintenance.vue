<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { getComponentStatus, performComponentAction, type ComponentOperation, type ComponentStatus } from '../lib/bridge';
import { reportFrontendEvent } from '../lib/frontend-diagnostics';
import { t } from '../lib/i18n';

const status=ref<ComponentStatus|null>(null);
const loading=ref(false);
const busy=ref(false);
const message=ref('');
const readError=ref('');
let disposed=false;
const ready=computed(()=>status.value?.installation==='available');
const wizardAvailable=computed(()=>!readError.value && (status.value?.allowedActions.includes('open_vendor_wizard')??false));
const feedback=computed(()=>[message.value,readError.value,status.value?.installation==='restart_required'?t('安装状态需要在重启电脑后重新检测。'):''].filter(Boolean).join(' · '));
function resultText(result: ComponentOperation) {
  if (result.outcome === "failed" && result.status.restartRequired) return "维护未完成，Windows 报告需要重启。请保存工作后按需重启，再刷新检测；应用不会自行重启。";
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

async function refresh(clearMessage=true) {
  loading.value=true;
  readError.value='';
  if(clearMessage)message.value='';
  try {
    const actual=await getComponentStatus('vb_cable');
    if(!disposed)status.value=actual;
    reportFrontendEvent({event:'vb_cable_status',phase:'completed',result:disposed?'unknown':'passed',reason:disposed?'late_read_ignored':actual.installation});
  }
  catch { if(!disposed)readError.value=t('暂时无法读取组件状态，请稍后重新检测。');reportFrontendEvent({event:'vb_cable_status',phase:'completed',result:'failed',reason:'read_failed'}); }
  finally { if(!disposed)loading.value=false; }
}
async function openVendorWizard() {
  busy.value=true;message.value='';
  reportFrontendEvent({event:'vb_cable_maintenance',phase:'requested',result:'unknown',reason:'explicit_user_request'});
  try {
    const operation=await performComponentAction('vb_cable','open_vendor_wizard');
    reportFrontendEvent({event:'vb_cable_maintenance',phase:'completed',result:['completed','wizard_closed'].includes(operation.outcome)?'passed':operation.outcome==='restart_required'?'unknown':'failed',reason:disposed?'late_'+operation.outcome:operation.outcome});
    if(!disposed){message.value=t(resultText(operation));await refresh(false);}
  } catch {
    reportFrontendEvent({event:'vb_cable_maintenance',phase:'completed',result:'failed',reason:'request_failed'});
    if(!disposed)message.value=t('无法打开官方安装/卸载向导，请稍后重试。');
  } finally { if(!disposed)busy.value=false; }
}
onMounted(()=>{void refresh();});
onUnmounted(()=>{disposed=true;});
</script>

<template>
  <section class="vb-cable-maintenance" :aria-label="t('VB-CABLE 音频组件')">
    <div class="maintenance-heading">
      <h3>{{ t('VB-CABLE 音频组件') }}</h3>
      <span class="muted">{{ ready && !readError ? t('已检测到') : t('需要检查') }}</span>
    </div>
    <div class="button-row">
      <button type="button" class="secondary-button" :disabled="loading || busy" @click="()=>refresh()">{{ loading ? t('检测中…') : t('重新检测') }}</button>
      <button type="button" class="secondary-button" :disabled="!wizardAvailable || busy || loading" @click="openVendorWizard">{{ t('打开官方安装/卸载向导') }}</button>
      <a href="https://vb-audio.com/Services/licensing.htm" target="_blank" rel="noreferrer">{{ t('许可与捐赠说明') }}</a>
    </div>
    <p class="component-message" role="status" aria-live="polite" tabindex="0" :title="feedback">{{ feedback }}</p>
  </section>
</template>

<style scoped>
.vb-cable-maintenance { margin: 0; }
.maintenance-heading { display: flex; align-items: baseline; flex-wrap: wrap; gap: 10px; margin-bottom: 10px; }
.maintenance-heading h3 { margin: 0; font-size: 14px; }
.maintenance-heading span { font-size: 12px; }
.button-row { justify-content: flex-start; align-items: center; }
.button-row button { white-space: normal; overflow-wrap: anywhere; }
.button-row a { font-size: 12px; }
.component-message { height: 1.4em; line-height: 1.4; font-size: 13px; margin: 4px 0 0; overflow: auto; overflow-wrap: anywhere; }
.component-message:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
</style>
