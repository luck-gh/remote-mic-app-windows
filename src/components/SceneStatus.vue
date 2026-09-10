<script setup lang="ts">
import { onMounted,onUnmounted,ref } from "vue";
import { getSceneSnapshot,subscribeSceneEvents,type MappingConfiguration,type SceneSnapshot } from "../lib/bridge";
const props=defineProps<{configuration:MappingConfiguration}>();const snapshot=ref<SceneSnapshot|null>(null);const error=ref<string|null>(null);let stop:(()=>void)|null=null;
function templateName(){return props.configuration.templates.find(x=>x.id===snapshot.value?.templateId)?.name??"未匹配模板"}
function statusLabel(status:string){return ({launch_failed:"启动应用失败",launch_foreground_timeout:"应用启动后未确认前台",semantic_action_unavailable:"当前应用不支持此操作",input_not_exclusively_captured:"按键未被完整接管",action_queue_full:"操作队列繁忙",action_worker_stopped:"控制服务已停止",template_missing:"模板不可用",menu_cancelled:"菜单已取消"} as Record<string,string>)[status]??"当前操作暂不可用"}
onMounted(async()=>{try{snapshot.value=await getSceneSnapshot();stop=await subscribeSceneEvents(event=>{if(event.type==="snapshot")snapshot.value=event.snapshot;else if(event.type==="launch_failed")error.value="应用启动失败，请检查应用是否可用后重试"})}catch(e){error.value="状态更新暂不可用"}});onUnmounted(()=>stop?.());
</script>
<template><section class="card"><h2>当前控制场景</h2><p v-if="error" class="error-text">{{error}}</p><p v-else-if="!snapshot" class="muted">场景状态暂不可用。</p><div v-else><p>{{snapshot.applicationId??"未识别应用"}} · {{templateName()}}</p><p class="muted">区域：{{snapshot.controlRegion??"未确认"}}；调节：{{snapshot.adjustmentMode??"未确认"}}</p><p v-if="snapshot.panel">菜单已打开</p><p v-if="snapshot.waitingForRelease">请先松开当前按键</p><p v-if="snapshot.status" class="muted">{{statusLabel(snapshot.status)}}</p></div></section></template>
