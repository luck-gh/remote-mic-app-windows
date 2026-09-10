<script setup lang="ts">
import { onMounted, ref } from "vue";
import {
  createMappingTemplate, deleteMappingTemplate, duplicateMappingTemplate,
  getMappingConfiguration, getMappingTemplatePresets, applyMappingTemplatePreset, renameMappingTemplate, saveMappingConfiguration,
  type MappingConfiguration, type MappingTemplate,
} from "../lib/bridge";
import TemplateRegionEditor from "./TemplateRegionEditor.vue";
import ApplicationTemplateBindings from "./ApplicationTemplateBindings.vue";
import MappingConfigurationTransfer from "./MappingConfigurationTransfer.vue";
import SceneStatus from "./SceneStatus.vue";

const configuration = ref<MappingConfiguration | null>(null);
const name = ref("");
const error = ref<string | null>(null);
const busy = ref(false);
const deleting = ref<MappingTemplate | null>(null);
const replacementTemplateId = ref("");
const editing = ref<MappingTemplate | null>(null);
const presets = ref<MappingTemplate[]>([]);

async function refresh() { configuration.value = await getMappingConfiguration(); }
async function run(action: () => Promise<unknown>) {
  error.value = null; busy.value = true;
  try { await action(); await refresh(); } catch (reason) { error.value = String(reason); }
  finally { busy.value = false; }
}
function valid(value: string) { return value.trim().length > 0 && !configuration.value?.templates.some((item) => item.name === value.trim()); }
function add() { if (!valid(name.value)) { error.value = "请输入不重复的模板名称"; return; } void run(async () => { await createMappingTemplate(name.value.trim()); name.value = ""; }); }
function rename(template: MappingTemplate) { const next = window.prompt("模板名称", template.name); if (next && valid(next)) void run(() => renameMappingTemplate(template.id, next.trim())); }
function remove(template: MappingTemplate) {
  const bound = configuration.value?.applicationBindings.filter((item) => item.templateId === template.id) ?? [];
  if (bound.length) { deleting.value = template; replacementTemplateId.value = ""; return; }
  void run(() => deleteMappingTemplate(template.id, null));
}
function confirmDelete() { if (!deleting.value || !replacementTemplateId.value) { error.value = "请选择替换模板"; return; } const target = deleting.value; void run(async () => { await deleteMappingTemplate(target.id, replacementTemplateId.value); deleting.value = null; }); }
function toggle() { if (!configuration.value) return; void run(() => saveMappingConfiguration({ ...configuration.value!, templateControlEnabled: !configuration.value!.templateControlEnabled })); }
async function saveTemplate(next: MappingTemplate) {
  if (!configuration.value) return;
  error.value = null; busy.value = true;
  try {
    const saved = await saveMappingConfiguration({ ...configuration.value, templates: configuration.value.templates.map((item) => item.id === next.id ? next : item) });
    configuration.value = saved; editing.value = null;
  } catch (reason) { error.value = String(reason); }
  finally { busy.value = false; }
}
async function applyPreset(preset: MappingTemplate) { const chosen=window.prompt("新模板名称",preset.name); if(!chosen)return; await run(()=>applyMappingTemplatePreset(preset.id,chosen.trim())); }
onMounted(() => { void refresh().catch((reason) => error.value = String(reason)); void getMappingTemplatePresets().then((items)=>presets.value=items).catch(()=>{}); });
</script>

<template>
  <section class="card">
    <div class="card-title-row"><div><h2>场景模板</h2><p class="muted">模板只会在应用绑定后生效。</p></div><button type="button" class="secondary-button" :disabled="busy || !configuration" @click="toggle">{{ configuration?.templateControlEnabled ? "关闭模板控制" : "启用模板控制" }}</button></div>
    <div class="button-row"><input v-model="name" :disabled="busy" placeholder="新模板名称" @keyup.enter="add" /><button type="button" :disabled="busy" @click="add">新建模板</button></div>
    <div v-if="presets.length" class="button-row"><span class="muted">推荐预设：</span><button v-for="preset in presets" :key="preset.id" type="button" class="secondary-button" :disabled="busy" @click="applyPreset(preset)">应用 {{ preset.name }}</button></div>
    <p v-if="error" class="error-text">{{ error }}</p>
    <p v-if="!configuration" class="muted">正在读取模板…</p>
    <ul v-else class="setting-list"><li v-for="template in configuration.templates" :key="template.id"><span><strong>{{ template.name }}</strong><small>已绑定 {{ configuration.applicationBindings.filter((item) => item.templateId === template.id).length }} 个应用</small></span><span class="button-row"><button type="button" class="secondary-button" :disabled="busy" @click="editing = template">编辑</button><button type="button" class="secondary-button" :disabled="busy" @click="run(() => duplicateMappingTemplate(template.id, `${template.name} 副本`))">复制</button><button type="button" class="secondary-button" :disabled="busy" @click="rename(template)">重命名</button><button type="button" class="secondary-button" :disabled="busy" @click="remove(template)">删除</button></span></li></ul>
    <div v-if="deleting" class="status-panel"><div><strong>替换已绑定应用</strong><small>删除“{{ deleting.name }}”前，请选择接替它的模板。</small><select v-model="replacementTemplateId"><option value="">选择模板</option><option v-for="item in configuration?.templates.filter((item) => item.id !== deleting?.id)" :key="item.id" :value="item.id">{{ item.name }}</option></select><span class="button-row"><button type="button" :disabled="busy" @click="confirmDelete">确认删除</button><button type="button" class="secondary-button" @click="deleting = null">取消</button></span></div></div>
    <TemplateRegionEditor v-if="editing" :template="editing" @save="saveTemplate" @cancel="editing = null" />
    <ApplicationTemplateBindings v-if="configuration" :configuration="configuration" @saved="configuration = $event" />
    <MappingConfigurationTransfer v-if="configuration" :configuration="configuration" @applied="configuration = $event" />
    <SceneStatus v-if="configuration" :configuration="configuration" />
  </section>
</template>
