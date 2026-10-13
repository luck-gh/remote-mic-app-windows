import { ref } from "vue";

// 显式离开与卸载的清理完成后才允许重跑，避免旧草稿回滚覆盖新一轮设置。
let cleanup = Promise.resolve();

// 用户显式设备/增强操作属于已有后台命令，退出不假称取消，也不等待系统授权。
export const onboardingExternalState = ref<"pending" | "passed" | "failed" | null>(null);

export function trackOnboardingExternalOperation<T extends { lastError: string | null }>(operation: Promise<T>): Promise<T> {
  onboardingExternalState.value = "pending";
  void operation.then(
    result => { onboardingExternalState.value = result.lastError ? "failed" : "passed"; },
    () => { onboardingExternalState.value = "failed"; },
  );
  return operation;
}

export function assertOnboardingCanRestart(): void {
  if (onboardingExternalState.value === "pending") {
    throw new Error("Windows 授权或增强设置仍在处理中；操作结束后可以重新运行向导。");
  }
}

export function registerOnboardingCleanup(operation: Promise<void>): void {
  const previous = cleanup;
  cleanup = Promise.allSettled([previous, operation]).then(() => undefined);
}

export function waitForOnboardingCleanup(): Promise<void> {
  return cleanup;
}
