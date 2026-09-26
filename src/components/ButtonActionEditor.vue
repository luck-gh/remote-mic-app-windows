<script setup lang="ts">
import { computed, onMounted, onUnmounted, reactive, ref } from "vue";
import {
  actionSummary,
  chordLabel,
  pickCustomApp,
  type ButtonAction,
  type ButtonMappings,
  type ButtonTrigger,
  type KeyCode,
  type PresetAppInfo,
  type RemoteButton,
} from "../lib/bridge";

const props = defineProps<{
  mappings: ButtonMappings;
  button: RemoteButton;
  trigger: ButtonTrigger;
  presetApps: PresetAppInfo[];
  capabilityNote?: string | null;
  shortcutsOnly?: boolean;
}>();
const emit = defineEmits<{
  update: [action: ButtonAction];
  status: [message: string];
}>();

const capturing = ref(false);
const captureDisplay = ref<string[]>([]);
const heldModifiers = reactive(new Set<KeyCode>());
const action = computed<ButtonAction>(() => props.mappings.actions[props.button]?.[props.trigger] ?? { type: "disabled" });
const presetIds = computed(() => new Set(props.presetApps.map((app) => app.id)));
const customApps = computed(() => {
  const found = new Map<string, string>();
  for (const actions of Object.values(props.mappings.actions)) {
    for (const mapped of Object.values(actions)) {
      if (mapped.type === "open_app" && !presetIds.value.has(mapped.target)) {
        const base = mapped.target.split(/[\\/]/).pop() ?? mapped.target;
        found.set(mapped.target, base.replace(/\.(exe|lnk)$/i, "") || base);
      }
    }
  }
  return [...found].map(([path, name]) => ({ path, name }));
});

const groups: Array<{ label: string; items: Array<{ label: string; keys: KeyCode[] }> }> = [
  { label: "基础按键", items: [
    { label: "Enter", keys: ["enter"] }, { label: "Esc", keys: ["escape"] },
    { label: "空格", keys: ["space"] }, { label: "Tab", keys: ["tab"] },
    { label: "退格", keys: ["backspace"] }, { label: "删除", keys: ["delete"] },
    { label: "↑", keys: ["up"] }, { label: "↓", keys: ["down"] },
    { label: "←", keys: ["left"] }, { label: "→", keys: ["right"] },
    { label: "Home", keys: ["home"] }, { label: "复制", keys: ["control", "c"] },
    { label: "粘贴", keys: ["control", "v"] }, { label: "剪切", keys: ["control", "x"] },
    { label: "全选", keys: ["control", "a"] }, { label: "撤销", keys: ["control", "z"] },
    { label: "重做", keys: ["control", "y"] }, { label: "查找", keys: ["control", "f"] },
    { label: "保存", keys: ["control", "s"] }, { label: "发送", keys: ["control", "enter"] },
    { label: "换行", keys: ["shift", "enter"] }, { label: "右键菜单", keys: ["apps"] },
    { label: "刷新", keys: ["f5"] },
  ] },
  { label: "系统与媒体", items: [
    { label: "切换窗口", keys: ["alt", "tab"] }, { label: "显示桌面", keys: ["left_windows", "d"] },
    { label: "关闭窗口", keys: ["control", "w"] }, { label: "锁定", keys: ["left_windows", "l"] },
    { label: "搜索", keys: ["left_windows", "s"] }, { label: "截图", keys: ["left_windows", "shift", "s"] },
    { label: "静音", keys: ["volume_mute"] }, { label: "音量+", keys: ["volume_up"] },
    { label: "音量−", keys: ["volume_down"] }, { label: "播放/暂停", keys: ["media_play_pause"] },
    { label: "上一首", keys: ["media_prev"] }, { label: "下一首", keys: ["media_next"] },
  ] },
];

function selected(keys: KeyCode[]): boolean {
  return action.value.type === "shortcut" && action.value.chord.keys.join("+") === keys.join("+");
}
function apply(next: ButtonAction): void { emit("update", next); }
async function addCustomApp(): Promise<void> {
  const pick = await pickCustomApp();
  if (pick) apply({ type: "open_app", target: pick.path });
}
function codeToKeyCode(code: string): KeyCode | null {
  const named: Record<string, KeyCode> = {
    ControlLeft: "left_control", ControlRight: "right_control", ShiftLeft: "left_shift", ShiftRight: "right_shift",
    AltLeft: "left_alt", AltRight: "right_alt", MetaLeft: "left_windows", MetaRight: "right_windows",
    Enter: "enter", Space: "space", Tab: "tab", Backspace: "backspace", Escape: "escape",
    ArrowLeft: "left", ArrowUp: "up", ArrowRight: "right", ArrowDown: "down", Home: "home", End: "end",
    PageUp: "page_up", PageDown: "page_down", Insert: "insert", Delete: "delete", ContextMenu: "apps",
    VolumeMute: "volume_mute", VolumeUp: "volume_up", VolumeDown: "volume_down",
  };
  if (named[code]) return named[code];
  const letter = /^Key([A-Z])$/.exec(code); if (letter) return letter[1].toLowerCase();
  const digit = /^Digit([0-9])$/.exec(code); if (digit) return `digit${digit[1]}`;
  const fn = /^F([1-9]|1[0-2])$/.exec(code); return fn ? `f${fn[1]}` : null;
}
function keydown(event: KeyboardEvent): void {
  if (!capturing.value) return;
  event.preventDefault(); event.stopPropagation();
  const code = codeToKeyCode(event.code); if (!code) return;
  const modifier = /control|shift|alt|windows/.test(code);
  if (modifier) { if (!event.repeat) heldModifiers.add(code); captureDisplay.value = [...heldModifiers]; return; }
  if (code === "escape" && !heldModifiers.size) { capturing.value = false; emit("status", "已取消录入"); return; }
  const keys = [...heldModifiers, code];
  apply({ type: "shortcut", chord: { keys } });
  capturing.value = false; heldModifiers.clear(); captureDisplay.value = [];
  emit("status", `快捷键已录入：${chordLabel({ keys })}`);
}
function keyup(event: KeyboardEvent): void {
  if (!capturing.value) return;
  const code = codeToKeyCode(event.code); if (code) heldModifiers.delete(code);
  captureDisplay.value = [...heldModifiers];
}
onMounted(() => { window.addEventListener("keydown", keydown, true); window.addEventListener("keyup", keyup, true); });
onUnmounted(() => { window.removeEventListener("keydown", keydown, true); window.removeEventListener("keyup", keyup, true); });
</script>

<template>
  <div class="button-action-editor">
    <p class="muted">当前：{{ actionSummary(action) }}</p>
    <p v-if="capabilityNote" class="muted capability-note">{{ capabilityNote }}</p>
    <button class="secondary-button" :class="{ 'is-active': action.type === 'disabled' }" type="button" @click="apply({ type: 'disabled' })">禁用按键</button>
    <section v-for="group in groups" :key="group.label" class="action-section">
      <h4>{{ group.label }}</h4>
      <div class="preset-grid"><button v-for="preset in group.items" :key="preset.label" class="chip" :class="{ selected: selected(preset.keys) }" type="button" @click="apply({ type: 'shortcut', chord: { keys: [...preset.keys] } })">{{ preset.label }}</button></div>
    </section>
    <section v-if="!shortcutsOnly" class="action-section"><h4>打开应用</h4><div class="preset-grid">
      <button v-for="app in presetApps" :key="app.id" class="chip" :class="{ selected: action.type === 'open_app' && action.target === app.id }" type="button" @click="apply({ type: 'open_app', target: app.id })">{{ app.name }}</button>
      <button v-for="app in customApps" :key="app.path" class="chip" :class="{ selected: action.type === 'open_app' && action.target === app.path }" type="button" @click="apply({ type: 'open_app', target: app.path })">{{ app.name }}</button>
      <button class="chip" type="button" @click="addCustomApp">＋ 添加应用</button>
    </div></section>
    <section class="action-section"><h4>自定义</h4><button class="chip" :class="{ selected: capturing }" type="button" @click="capturing = !capturing">{{ capturing ? "录入中…（按 Esc 取消）" : "录入自定义快捷键" }}</button><span v-if="capturing" class="capture-display">{{ captureDisplay.length ? captureDisplay.join(" + ") : "请按下快捷键组合" }}</span></section>
  </div>
</template>

<style scoped>
.button-action-editor { display: grid; gap: 14px; }
.action-section { display: grid; gap: 8px; }
.action-section h4 { margin: 0; }
.preset-grid { display: flex; flex-wrap: wrap; gap: 8px; }
.capture-display { margin-left: 10px; color: var(--text-secondary); }
</style>
