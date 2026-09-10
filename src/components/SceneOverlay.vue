<script setup lang="ts">
import { onMounted,onUnmounted,ref } from "vue";
import { getSceneSnapshot,subscribeSceneEvents,type SceneSnapshot } from "../lib/bridge";
const snapshot=ref<SceneSnapshot|null>(null);let stop:(()=>void)|null=null;
onMounted(async()=>{snapshot.value=await getSceneSnapshot();stop=await subscribeSceneEvents(event=>{if(event.type==="snapshot")snapshot.value=event.snapshot})});onUnmounted(()=>stop?.());
function title(){return snapshot.value?.panel==="application"?"程序列表":"调节功能"}
</script>
<template><main v-if="snapshot?.panel" class="scene-overlay"><h1>{{title()}}</h1><ul><li v-for="(item,index) in snapshot.menuItems" :key="`${item.label}-${index}`" :class="{selected:index===snapshot.selectedIndex}"><span>{{item.label}}</span><small>{{item.running?'运行中':'可选择'}}</small></li></ul><p>方向键选择 · 确定应用 · 返回取消</p></main><main v-else class="scene-overlay-empty"></main></template>
