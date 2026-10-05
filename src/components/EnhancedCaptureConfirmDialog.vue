<script setup lang="ts">
import { onMounted, ref } from "vue";

/** Every explicit enable requires confirmation; this component never changes capture state. */
const emit = defineEmits<{ confirm: []; close: [] }>();
const dialog = ref<HTMLDialogElement | null>(null);

onMounted(() => {
  // jsdom 可能没有 showModal，退回 open 属性（与 RegisteredAppsDialog 同一套写法）。
  if (dialog.value?.showModal) dialog.value.showModal();
  else dialog.value?.setAttribute("open", "");
});
</script>

<template>
  <dialog
    ref="dialog"
    class="capture-confirm-dialog"
    aria-labelledby="capture-confirm-title"
    @cancel.prevent="emit('close')"
  >
    <h3 id="capture-confirm-title">开启“全按键支持”？</h3>
    <p class="capture-confirm-intro">
      开启后，无线麦会直接读取遥控器上已配置的按键，用于统一执行已配置的按键动作。每次开启都会弹出 Windows 授权窗口，请选择“是”；关闭时会等待按键释放并停止接管。
    </p>
    <p class="capture-confirm-note">
      个别杀毒软件可能把这个功能当成风险，拦截或关闭它，导致按键失灵、开启失败。
    </p>
    <p class="capture-confirm-note">
      个别游戏带有防作弊保护，可能和这个功能合不来：表现为按键失灵、游戏打不开等。玩这类游戏前，建议先把全按键支持关掉。
    </p>
    <p class="capture-confirm-privacy">
      按键数据仅在本机处理，不会上传；可随时关闭。
    </p>
    <footer class="capture-confirm-footer">
      <button type="button" class="secondary-button" @click="emit('close')">取消</button>
      <button type="button" class="primary-button" @click="emit('confirm')">开启</button>
    </footer>
  </dialog>
</template>

<style scoped>
.capture-confirm-dialog {
  width: min(540px, calc(100vw - 40px));
  box-sizing: border-box;
  padding: 20px 22px;
  border: 1px solid var(--border-strong);
  border-radius: 10px;
  background: var(--surface-canvas);
  color: var(--text-primary);
  overflow: auto;
}
.capture-confirm-dialog::backdrop {
  background: #0008;
}
.capture-confirm-dialog h3 {
  margin: 0 0 12px;
  font-size: 15px;
  font-weight: 500;
}
.capture-confirm-intro {
  margin: 0 0 12px;
  font-size: 13px;
  line-height: 1.6;
}
.capture-confirm-note {
  margin: 0 0 8px;
  padding: 10px 12px;
  border-radius: 6px;
  background: var(--warning-surface);
  color: var(--warning-text);
  font-size: 13px;
  line-height: 1.6;
}
.capture-confirm-privacy {
  margin: 0 0 14px;
  font-size: 12px;
  line-height: 1.6;
  color: var(--text-secondary);
}
.capture-confirm-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
</style>
