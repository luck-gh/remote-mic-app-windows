<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import SettingsDialog from "./SettingsDialog.vue";
import { reportFrontendEvent, reasonToken } from "../lib/frontend-diagnostics";
import { useUiPreference } from "../lib/ui-preferences";
import {
  listRunningApps, pickCustomApp,
  removeApplicationBinding, updateApplicationBindings,
  reorderApplicationAssociations, upsertApplicationBinding,
  type ApplicationBinding, type MappingConfiguration, type TemplateCatalogEntry,
} from "../lib/bridge";

type TemplateChoice = { kind: "direct"; id: string; name: string };
interface AppChoice {
  id: string;
  name: string;
  source: "running" | "manual" | "saved";
  launchTarget?: string;
}
interface Association {
  applicationId: string;
  templateId: string;
  kind: "direct";
  binding: ApplicationBinding | null;
  menuOrder: number;
}

const props = defineProps<{ configuration: MappingConfiguration; catalog: TemplateCatalogEntry[]; disabled?: boolean }>();
const emit = defineEmits<{
  saved: [configuration: MappingConfiguration];
  status: [message: string];
  busyChange: [busy: boolean];
}>();

const dialogOpen = ref(false);
const expansion = useUiPreference("associationsExpanded");
const step = ref<1 | 2 | 3>(1);
const choices = ref<AppChoice[]>([]);
const selectedApp = ref<AppChoice | null>(null);
const selectedTemplate = ref<TemplateChoice | null>(null);
const search = ref("");
const loading = ref(false);
const saving = ref(false);
const busy = computed(() => saving.value || !!props.disabled);
watch(saving, (value) => emit("busyChange", value), { flush: "sync" });
watch(() => props.disabled, (value) => { if (value) finishDrag(); }, { flush: "sync" });
const loadError = ref<string | null>(null);
const error = ref<string | null>(null);
const draggingId = ref<string | null>(null);
const dropTargetId = ref<string | null>(null);
let interactiveDragOrigin = false;

const knownNames: Record<string, string> = {
  codex: "Codex", wechat: "微信", edge: "Edge 浏览器", chrome: "Chrome 浏览器",
};

function sameId(left: string, right: string): boolean {
  return left.toLocaleLowerCase() === right.toLocaleLowerCase();
}

function fallbackName(applicationId: string): string {
  const known = knownNames[applicationId.toLocaleLowerCase()];
  if (known) return known;
  const base = applicationId.split(/[\\/]/).pop() ?? applicationId;
  return base.replace(/\.exe$/i, "") || "自定义程序";
}

const associations = computed<Association[]>(() => props.configuration.applicationBindings.map((binding, index) => ({
  applicationId: binding.applicationId, templateId: binding.templateId, kind: "direct" as const, binding, menuOrder: binding.menuOrder ?? 0, stable: index,
})).sort((a,b) => a.menuOrder - b.menuOrder || a.stable - b.stable));

const filteredChoices = computed(() => {
  const query = search.value.trim().toLocaleLowerCase();
  return choices.value.filter((choice) => !query || choice.name.toLocaleLowerCase().includes(query));
});

const visibleAssociations = computed(() => associations.value);
const selectedIds = ref(new Set<string>());
const associationSection = ref<HTMLDetailsElement | null>(null);
const selectedAssociations = computed(() => visibleAssociations.value.filter(item => selectedIds.value.has(item.applicationId.toLocaleLowerCase())));
const allSelected = computed(() => visibleAssociations.value.length > 0 && selectedAssociations.value.length === visibleAssociations.value.length);
const partiallySelected = computed(() => selectedAssociations.value.length > 0 && !allSelected.value);
const bulkDialog = ref<{ action: "replace" | "remove"; targets: { id: string; name: string }[] } | null>(null);
const bulkTemplateId = ref<string | null>(null);
const bulkError = ref<string | null>(null);
let bulkTrigger: HTMLButtonElement | null = null;
watch(() => props.configuration.applicationBindings, bindings => {
  const existing = new Set(bindings.map(binding => binding.applicationId.toLocaleLowerCase()));
  selectedIds.value = new Set([...selectedIds.value].filter(id => existing.has(id)));
});

function toggleSelection(applicationId: string, checked: boolean): void {
  if (busy.value || bulkDialog.value) return;
  const next = new Set(selectedIds.value);
  const id = applicationId.toLocaleLowerCase();
  if (checked) next.add(id); else next.delete(id);
  selectedIds.value = next;
}

function toggleAll(event: Event): void {
  if (busy.value || bulkDialog.value) return;
  selectedIds.value = new Set((event.target as HTMLInputElement).checked
    ? visibleAssociations.value.map(item => item.applicationId.toLocaleLowerCase()) : []);
}

function openBulk(action: "replace" | "remove", event: Event): void {
  if (busy.value || bulkDialog.value || !selectedAssociations.value.length) return;
  bulkTrigger = event.currentTarget as HTMLButtonElement;
  bulkDialog.value = { action, targets: selectedAssociations.value.map(item => ({ id: item.applicationId, name: fallbackName(item.applicationId) })) };
  bulkTemplateId.value = null;
  bulkError.value = null;
  reportFrontendEvent({ event: "template_binding_batch", phase: "opened", result: "passed", reason: reasonToken(action, bulkDialog.value.targets.length) });
}

function closeBulk(): void {
  if (busy.value) return;
  reportFrontendEvent({ event: "template_binding_batch", phase: "completed", result: "cancelled", reason: "user_cancelled" });
  bulkDialog.value = null;
  void nextTick(restoreBulkFocus);
}

function restoreBulkFocus(): void {
  const section = associationSection.value;
  if (!section) return;
  if (bulkTrigger?.isConnected && !bulkTrigger.disabled) bulkTrigger.focus();
  else section.querySelector("summary")!.focus();
}

function trapBulkFocus(event: KeyboardEvent): void {
  const dialog = (event.currentTarget as HTMLElement).querySelector<HTMLElement>("[role='dialog']")!;
  const controls = Array.from(dialog.querySelectorAll<HTMLElement>("button:not([disabled]), [tabindex='0']"));
  const first = controls[0];
  const last = controls[controls.length - 1];
  if (!first) {
    event.preventDefault();
    dialog.focus();
  } else if (!controls.includes(document.activeElement as HTMLElement)) {
    event.preventDefault();
    (event.shiftKey ? last! : first).focus();
  } else if (event.shiftKey && document.activeElement === first) {
    event.preventDefault();
    last!.focus();
  } else if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault();
    first.focus();
  }
}

async function confirmBulk(event: MouseEvent): Promise<void> {
  const operation = bulkDialog.value;
  if (busy.value || !operation || (operation.action === "replace" && !bulkTemplateId.value)) return;
  const targetId = operation.action === "replace" ? bulkTemplateId.value : null;
  saving.value = true;
  if (event.currentTarget === document.activeElement) {
    (event.currentTarget as HTMLElement).closest<HTMLElement>("[role='dialog']")!.focus();
  }
  bulkError.value = null;
  const started = performance.now();
  const reasonCode = reasonToken(operation.action, operation.targets.length);
  reportFrontendEvent({ event: "template_binding_batch", phase: "started", result: "passed", reason: reasonCode });
  try {
    const saved = await updateApplicationBindings(operation.targets.map(item => item.id), targetId);
    emit("saved", saved);
    emit("status", operation.action === "replace" ? `已更换 ${operation.targets.length} 个程序的模板关联` : `已移除 ${operation.targets.length} 个程序关联；模板仍保留`);
    bulkDialog.value = null;
    reportFrontendEvent({ event: "template_binding_batch", phase: "completed", result: "passed", reason: reasonCode, elapsedMs: Math.round(performance.now() - started) });
  } catch (reason) {
    bulkError.value = reason instanceof Error ? reason.message : String(reason);
    reportFrontendEvent({ event: "template_binding_batch", phase: "completed", result: "failed", reason: reasonCode, elapsedMs: Math.round(performance.now() - started) });
  } finally {
    saving.value = false;
    if (!bulkDialog.value) void nextTick(restoreBulkFocus);
  }
}

const allTemplates = computed<TemplateChoice[]>(() => props.catalog.map((template) => ({
  kind: template.kind, id: template.id, name: template.name,
})));

function templateName(association: Association): string {
  return props.catalog.find((template) => template.id === association.templateId)?.name ?? "模板已不存在";
}

function associationStatus(_association: Association): string {
  return props.configuration.buttonMappingFollowEnabled ? "程序默认模板 · 已启用" : "程序默认模板 · 已保存，自动加载未启用";
}

function existingAssociation(applicationId: string): Association | undefined {
  return associations.value.find((item) => sameId(item.applicationId, applicationId));
}

function addChoice(next: AppChoice): void {
  const index = choices.value.findIndex((item) => sameId(item.id, next.id));
  if (index < 0) choices.value.push(next);
  else if (choices.value[index]!.source !== "running") choices.value[index] = next;
}

async function loadChoices(): Promise<void> {
  const started = performance.now();
  loading.value = true;
  loadError.value = null;
  try {
    const running = await listRunningApps();
    const next: AppChoice[] = running.map((app) => ({
      id: app.applicationId, name: app.name, source: "running" as const,
    }));
    choices.value = next;
    reportFrontendEvent({ event: "template_app_discovery", phase: "completed", result: "passed", reason: reasonToken("choices", next.length), elapsedMs: Math.round(performance.now() - started) });
  } catch (reason) {
    loadError.value = reason instanceof Error ? reason.message : String(reason);
    reportFrontendEvent({ event: "template_app_discovery", phase: "completed", result: "failed", reason: "public_app_sources_unavailable", elapsedMs: Math.round(performance.now() - started) });
  } finally {
    loading.value = false;
  }
}

function openAdd(): void {
  if (busy.value) return;
  reportFrontendEvent({ event: "template_binding_wizard", phase: "opened", result: "passed", reason: "user_requested_add" });
  dialogOpen.value = true;
  step.value = 1;
  selectedApp.value = null;
  selectedTemplate.value = null;
  search.value = "";
  error.value = null;
  void loadChoices();
}

function openChange(association: Association): void {
  if (busy.value) return;
  dialogOpen.value = true;
  step.value = 2;
  selectedApp.value = {
    id: association.applicationId,
    name: fallbackName(association.applicationId),
    source: "saved",
    launchTarget: association.binding?.launchTarget ?? undefined,
  };
  selectedTemplate.value = {
    kind: association.kind, id: association.templateId, name: templateName(association),
  };
  error.value = null;
}

function close(): void {
  if (!busy.value) dialogOpen.value = false;
}

async function pickManual(): Promise<void> {
  if (busy.value) return;
  error.value = null;
  try {
    const app = await pickCustomApp();
    if (!app) return;
    const choice: AppChoice = {
      id: app.applicationId, name: app.name, source: "manual", launchTarget: app.path,
    };
    addChoice(choice);
    selectedApp.value = choice;
  } catch (reason) {
    error.value = reason instanceof Error ? reason.message : String(reason);
  }
}

function nextStep(): void {
  if (busy.value) return;
  error.value = null;
  if (step.value === 1) {
    if (!selectedApp.value) { error.value = "请选择一个程序"; return; }
    step.value = 2;
  } else if (step.value === 2) {
    if (!selectedTemplate.value) { error.value = "请选择一个模板"; return; }
    step.value = 3;
  }
}

function previousStep(): void {
  if (busy.value) return;
  error.value = null;
  if (step.value > 1) step.value = (step.value - 1) as 1 | 2;
}

async function confirm(): Promise<void> {
  if (busy.value || !selectedApp.value || !selectedTemplate.value) return;
  const existing = existingAssociation(selectedApp.value.id);
  if (existing?.kind === selectedTemplate.value.kind && existing.templateId === selectedTemplate.value.id) {
    error.value = "该程序已经关联这个模板，无需重复添加";
    return;
  }
  saving.value = true;
  error.value = null;
  const started = performance.now();
  const reasonCode = `${selectedTemplate.value.kind}_${existing ? "replace" : "add"}`;
  reportFrontendEvent({ event: "template_binding_save", phase: "started", result: "passed", reason: reasonCode });
  try {
    const saved = await upsertApplicationBinding({ applicationId: selectedApp.value.id, templateId: selectedTemplate.value.id,
      menuOrder: existing?.menuOrder ?? associations.value.length,
      launchTarget: selectedApp.value.launchTarget ?? existing?.binding?.launchTarget ?? null,
    });
    emit("saved", saved);
    emit("status", existing
      ? `已更新“${selectedApp.value.name}”的模板关联`
      : `已添加“${selectedApp.value.name}”的模板关联`);
    dialogOpen.value = false;
    reportFrontendEvent({ event: "template_binding_save", phase: "completed", result: "passed", reason: reasonCode, elapsedMs: Math.round(performance.now() - started) });
  } catch (reason) {
    error.value = reason instanceof Error ? reason.message : String(reason);
    reportFrontendEvent({ event: "template_binding_save", phase: "completed", result: "failed", reason: reasonCode, elapsedMs: Math.round(performance.now() - started) });
  } finally {
    saving.value = false;
  }
}

async function remove(association: Association): Promise<void> {
  if (busy.value) return;
  saving.value = true;
  error.value = null;
  try {
    const saved = await removeApplicationBinding(association.applicationId);
    emit("saved", saved);
    emit("status", `已移除“${fallbackName(association.applicationId)}”的关联；模板仍保留`);
  } catch (reason) {
    error.value = reason instanceof Error ? reason.message : String(reason);
  } finally {
    saving.value = false;
  }
}

function isInteractive(target: EventTarget | null): boolean {
  return target instanceof Element
    && !!target.closest("button, input, select, textarea, a, [contenteditable]:not([contenteditable='false']), [role='button']");
}
function rememberDragOrigin(event: PointerEvent): void {
  interactiveDragOrigin = isInteractive(event.target);
}
function startDrag(event: DragEvent, association: Association): void {
  if (busy.value || interactiveDragOrigin || isInteractive(event.target)) {
    event.preventDefault();
    finishDrag();
    return;
  }
  draggingId.value = association.applicationId;
  event.dataTransfer?.setData("text/plain", association.applicationId);
  if (event.dataTransfer) event.dataTransfer.effectAllowed = "move";
}
function finishDrag(): void { draggingId.value = null; dropTargetId.value = null; interactiveDragOrigin = false; }
async function dropAssociation(target: Association): Promise<void> {
  const sourceId = draggingId.value;
  finishDrag();
  if (busy.value || !sourceId || sameId(sourceId, target.applicationId)) return;
  const ids = visibleAssociations.value.map((item) => item.applicationId);
  const from = ids.findIndex((id) => sameId(id, sourceId)); const to = ids.findIndex((id) => sameId(id, target.applicationId));
  if (from < 0 || to < 0) return;
  ids.splice(to, 0, ids.splice(from, 1)[0]!);
  saving.value = true; error.value = null;
  try { emit("saved", await reorderApplicationAssociations(ids)); emit("status", "已保存程序关联展示顺序"); }
  catch (reason) { error.value = reason instanceof Error ? reason.message : String(reason); }
  finally { saving.value = false; }
}
</script>

<template>
  <details ref="associationSection" class="card collapsible-card application-associations" :open="expansion.value.value" :aria-busy="expansion.pending.value" :inert="!!bulkDialog" @toggle="expansion.toggleDetails">
    <summary><h2>程序关联</h2><span class="section-count">{{ configuration.applicationBindings.length }} 个程序</span></summary>
    <div class="collapsible-body">
    <div class="card-title-row">
      <div>
        <p class="muted">这里只显示你已添加的关联。添加或更换关联不会开启任何运行开关。</p>
        <p class="muted">以下状态确认已保存的关联和开关；当前前台是否实际命中模板，仍以实际按键验证为准。</p>
      </div>
      <button type="button" class="primary-button" :disabled="busy" @click="openAdd">添加程序</button>
    </div>
    <p v-if="error && !dialogOpen" class="error-text">{{ error }}</p>
    <div v-if="visibleAssociations.length" class="association-table" role="table" aria-label="已添加的程序关联">
      <div class="association-table-row association-table-header" role="row">
        <span role="columnheader"><label class="association-selection"><input type="checkbox" aria-label="全选程序关联" :checked="allSelected" :indeterminate="partiallySelected" :disabled="busy || !!bulkDialog" @change="toggleAll" />软件名称</label></span>
        <span role="columnheader" class="association-template-header">选择的模板</span>
        <span role="columnheader" class="association-bulk-actions" aria-label="批量操作">
          <button type="button" class="secondary-button" :disabled="busy || !!bulkDialog || !selectedAssociations.length" @click="openBulk('replace', $event)">更换模板</button>
          <button type="button" class="secondary-button" :disabled="busy || !!bulkDialog || !selectedAssociations.length" @click="openBulk('remove', $event)">移除</button>
        </span>
      </div>
      <div v-for="association in visibleAssociations" :key="association.applicationId" class="association-table-row association-row" :class="{ dragging: sameId(draggingId ?? '', association.applicationId), 'drop-target': sameId(dropTargetId ?? '', association.applicationId) }" role="row" :draggable="!busy" @pointerdown.capture="rememberDragOrigin" @dragstart="startDrag($event, association)" @dragend="finishDrag" @dragover.prevent="!busy && (dropTargetId = association.applicationId)" @dragleave="dropTargetId = null" @drop.prevent="dropAssociation(association)">
        <span role="cell"><label class="association-selection"><input type="checkbox" :aria-label="`选择 ${fallbackName(association.applicationId)}`" :checked="selectedIds.has(association.applicationId.toLocaleLowerCase())" :disabled="busy || !!bulkDialog" @change="toggleSelection(association.applicationId, ($event.target as HTMLInputElement).checked)" /><strong>{{ fallbackName(association.applicationId) }}</strong></label></span>
        <span role="cell" class="association-template-cell">
          <strong>{{ templateName(association) }}</strong>
          <small>{{ associationStatus(association) }}</small>
        </span>
        <span role="cell" class="button-row association-actions">
          <button type="button" class="secondary-button" :disabled="busy" @click="openChange(association)">更换模板</button>
          <button type="button" class="secondary-button" :disabled="busy" @click="remove(association)">移除</button>
        </span>
      </div>
    </div>
    <p v-else class="empty-association muted">尚未添加程序关联。当前继续使用通用配置。</p>
    </div>
  </details>
  <p v-if="expansion.error.value" role="alert" class="error-text">{{ expansion.error.value }}</p>

  <SettingsDialog v-if="bulkDialog" :title="bulkDialog.action === 'replace' ? '批量更换模板' : '批量移除关联'" :description="`已选择 ${bulkDialog.targets.length} 个程序`" :busy="busy" @close="closeBulk" @keydown.tab="trapBulkFocus">
    <div class="bulk-targets" aria-label="本次选择的程序"><span v-for="target in bulkDialog.targets" :key="target.id">{{ target.name }}</span></div>
    <template v-if="bulkDialog.action === 'replace'">
      <p class="selection-context">为这些程序选择同一个模板，保持当前运行开关。</p>
      <div v-if="allTemplates.length" class="choice-list bulk-template-list">
        <button v-for="choice in allTemplates" :key="choice.id" type="button" class="choice-card bulk-template-choice" :class="{ selected: bulkTemplateId === choice.id }" :aria-pressed="bulkTemplateId === choice.id" :disabled="busy" @click="bulkTemplateId = choice.id">{{ choice.name }}</button>
      </div>
      <p v-else class="muted">还没有可关联的模板。请先在按键页保存按键模板。</p>
    </template>
    <p v-else class="selection-context">确认后只移除上述程序关联，模板仍保留，运行开关保持不变。</p>
    <div class="bulk-feedback" :tabindex="bulkError ? 0 : undefined" :aria-label="bulkError ? '批量操作错误详情' : undefined"><p v-if="bulkError" role="alert" class="error-text">{{ bulkError }}</p></div>
    <template #actions>
      <button type="button" class="primary-button" :disabled="busy || (bulkDialog.action === 'replace' && !bulkTemplateId)" @click="confirmBulk">{{ busy ? '正在保存…' : bulkDialog.action === 'replace' ? '确认更换' : '确认移除' }}</button>
      <button type="button" class="secondary-button" :disabled="busy" @click="closeBulk">取消</button>
    </template>
  </SettingsDialog>

  <SettingsDialog
    v-if="dialogOpen"
    :title="step === 1 ? '选择程序' : step === 2 ? '选择模板' : '确认关联'"
    :description="`第 ${step} 步，共 3 步`"
    :busy="busy"
    @close="close"
  >
    <template v-if="step === 1">
      <div class="program-toolbar">
        <input v-model="search" type="search" placeholder="搜索程序" aria-label="搜索程序" />
        <button type="button" class="secondary-button" :disabled="busy" @click="pickManual">选择 .exe / .lnk…</button>
      </div>
      <p v-if="loading" class="muted">正在查找运行中的程序…</p>
      <div v-else-if="loadError" class="dialog-feedback">
        <p class="error-text">程序列表加载失败：{{ loadError }}</p>
        <button type="button" class="secondary-button" @click="loadChoices">重试</button>
      </div>
      <div v-else-if="filteredChoices.length" class="choice-list">
        <button
          v-for="choice in filteredChoices"
          :key="choice.id"
          type="button"
          class="choice-card"
          :class="{ selected: selectedApp && sameId(selectedApp.id, choice.id) }"
          @click="selectedApp = choice"
        >
          <span><strong>{{ choice.name }}</strong><small>{{ choice.source === 'running' ? '正在运行' : choice.source === 'manual' ? '本机文件' : '已有关联' }}</small></span>
          <span v-if="existingAssociation(choice.id)" class="choice-badge">已关联</span>
        </button>
      </div>
      <p v-else class="muted">没有匹配的程序。可清除搜索词或选择本机 .exe / .lnk。</p>
    </template>

    <template v-else-if="step === 2">
      <p class="selection-context">为 <strong>{{ selectedApp?.name }}</strong> 选择一个模板。</p>
      <div v-if="allTemplates.length" class="choice-list">
        <button
          v-for="choice in allTemplates"
          :key="`${choice.kind}:${choice.id}`"
          type="button"
          class="choice-card template-choice"
          :class="{ selected: selectedTemplate?.kind === choice.kind && selectedTemplate?.id === choice.id }"
          @click="selectedTemplate = choice"
        >
          <span>
            <strong>{{ choice.name }}</strong>
            <small v-if="choice.kind === 'direct'">完整按键模板：保存每个普通按键的单击、双击和长按动作</small>
            <small v-else>按键模板：按程序列表、内容区和输入框执行公开 UI 能力</small>
          </span>
          <span class="choice-badge">{{ choice.kind === "direct" ? "完整按键" : "区域语义" }}</span>
        </button>
      </div>
      <p v-else class="muted">还没有可关联的模板。请先在按键页保存按键模板。</p>
    </template>

    <template v-else>
      <dl class="confirmation-list">
        <div><dt>程序</dt><dd>{{ selectedApp?.name }}</dd></div>
        <div><dt>模板</dt><dd>{{ selectedTemplate?.name }}</dd></div>
        <div><dt>能力</dt><dd>{{ selectedTemplate?.kind === "direct" ? "完整按键模板" : "按键模板" }}</dd></div>
        <div><dt>运行状态</dt><dd>保持当前开关设置，不会自动开启</dd></div>
      </dl>
      <p v-if="selectedApp && existingAssociation(selectedApp.id)" class="info-callout warning">
        此程序已有一个关联。确认后会更换关联，原模板对象不会删除。
      </p>
    </template>

    <p v-if="error" class="error-text">{{ error }}</p>
    <template #actions>
      <button v-if="step < 3" type="button" class="primary-button" :disabled="busy || (step === 2 && !allTemplates.length)" @click="nextStep">下一步</button>
      <button v-else type="button" class="primary-button" :disabled="busy" @click="confirm">{{ busy ? "正在保存…" : existingAssociation(selectedApp?.id ?? '') ? "确认更换" : "确认添加" }}</button>
      <button v-if="step > 1" type="button" class="secondary-button" :disabled="busy" @click="previousStep">上一步</button>
      <button type="button" class="secondary-button" :disabled="busy" @click="close">取消</button>
    </template>
  </SettingsDialog>
</template>

<style scoped>
.application-associations { margin-top: 0; }
.association-table { margin-top: 14px; border: 1px solid var(--border); border-radius: 10px; overflow: hidden; }
.association-table-row { display: grid; grid-template-columns: minmax(130px, 1fr) minmax(180px, 1.35fr) 224px; gap: 16px; align-items: center; padding: 12px 14px; }
.association-table-header { padding-top: 9px; padding-bottom: 9px; color: var(--muted); background: var(--surface-subtle); font-size: 12px; font-weight: 600; }
.association-row + .association-row { border-top: 1px solid var(--border); }
.association-template-cell strong, .association-template-cell small { display: block; }
.association-template-cell small { margin-top: 3px; color: var(--muted); font-size: 12.5px; }
.association-actions, .association-bulk-actions { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 8px; }
.association-actions button, .association-bulk-actions button { min-width: 0; white-space: nowrap; display: inline-flex; align-items: center; justify-content: center; }
.association-row { cursor: grab; }
.association-row.dragging { opacity: .55; }
.association-row.drop-target { box-shadow: inset 0 3px 0 var(--accent); }
.association-row button { cursor: pointer; }
.association-selection { display: flex; align-items: center; gap: 10px; cursor: pointer; overflow-wrap: anywhere; }
.association-selection input { flex: 0 0 auto; width: 16px; height: 16px; margin: 0; accent-color: var(--accent); }
.association-table-row > *, .association-template-cell strong { min-width: 0; overflow-wrap: anywhere; }
.bulk-targets { display: flex; flex-wrap: wrap; gap: 6px 12px; max-height: 90px; overflow: auto; margin-bottom: 12px; overflow-wrap: anywhere; }
.bulk-template-list { max-height: 260px; overflow: auto; }
.bulk-feedback { height: 64px; overflow: auto; margin-top: 12px; }
.bulk-feedback:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
.bulk-feedback .error-text { margin: 0; white-space: pre-wrap; overflow-wrap: anywhere; }
.empty-association { margin: 14px 0 2px; }
.program-toolbar { display: grid; grid-template-columns: minmax(0, 1fr) auto; gap: 10px; }
.choice-list { display: grid; gap: 8px; margin-top: 12px; }
.choice-card {
  width: 100%; min-height: 58px; display: flex; align-items: center; justify-content: space-between;
  gap: 14px; padding: 10px 12px; border: 1px solid var(--border); border-radius: 10px;
  color: var(--text-primary); background: var(--surface-subtle); text-align: left;
}
.choice-card:hover { border-color: var(--accent); }
.choice-card.selected { border-color: var(--accent); box-shadow: inset 0 0 0 1px var(--accent); }
.choice-card strong, .choice-card small { display: block; }
.choice-card small { margin-top: 3px; color: var(--muted); font-size: 12.5px; }
.choice-badge { flex: 0 0 auto; color: var(--accent-text); font-size: 12px; font-weight: 600; }
.selection-context { margin: 0 0 10px; }
.confirmation-list { display: grid; gap: 10px; margin: 0; }
.confirmation-list div { display: grid; grid-template-columns: 88px minmax(0, 1fr); gap: 12px; }
.confirmation-list dt { color: var(--muted); }
.confirmation-list dd { margin: 0; font-weight: 600; }
.dialog-feedback { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
@media (max-width: 680px) {
  .program-toolbar { grid-template-columns: 1fr; }
  .association-table-row { grid-template-columns: minmax(0, 1fr) auto; gap: 8px 12px; }
  .association-table-header { grid-template-columns: minmax(0, 1fr) auto; }
  .association-template-header { display: none; }
  .association-bulk-actions { grid-column: 2; }
  .association-bulk-actions button { padding-inline: 10px; }
  .association-row .association-actions { grid-column: 1 / -1; padding-top: 4px; }
  .association-row .association-template-cell { grid-column: 1 / -1; }
}
</style>
