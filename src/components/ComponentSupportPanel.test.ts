import { flushPromises, mount } from "@vue/test-utils";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import ComponentSupportPanel from "./ComponentSupportPanel.vue";

const mocks = vi.hoisted(() => ({
  getComponentStatus: vi.fn(), performComponentAction: vi.fn(),
  getRc003BridgeSnapshot: vi.fn(), getRc003TaskStatus: vi.fn(),
  getCaptureSupport: vi.fn(),
  enableRc003Capture: vi.fn(), disableRc003Capture: vi.fn(),
}));
vi.mock("../lib/bridge", async (importOriginal) => ({ ...(await importOriginal<typeof import("../lib/bridge")>()), ...mocks }));
const status = { component: "vb_cable", installation: "available", package: "download_available", installedVersion: null, serviceInstalled: true, loaded: true, bound: null, audioEndpointsReady: true, restartRequired: false, reason: "ready", blockers: ["official_wizard_required"], allowedActions: ["open_vendor_wizard"] } as const;
const disabled = { installed: true, authorizationRequired: true, enabled: false, cleanupPending: false, canRetryCleanup: false, helperPath: "helper.exe", lastError: null };
const listening = { phase: "listening", port: 0, helperPid: 0, acceptedTotal: 0, deniedTotal: 0, replacedTotal: 0, edgesApplied: 0, usagesDropped: 0, malformedTotal: 0, watchdogReleaseTotal: 0, pressedUsages: [], lastRxAgeMs: null, targetGeneration: 0, targetUsages: [], ownedUsages: [] };
const wrappers: ReturnType<typeof mount>[] = [];
async function panel() { const wrapper = mount(ComponentSupportPanel); wrappers.push(wrapper); await flushPromises(); return wrapper; }
function captureToggle(wrapper: ReturnType<typeof mount>) { return wrapper.get('input[aria-label="全按键支持"]'); }
function button(wrapper: ReturnType<typeof mount>, label: string) { return wrapper.findAll("button").find(item => item.text() === label)!; }
describe("unified button support and audio component", () => {
  it('uses the upstream-style controlled toggle without a manual refresh or authorization disclosure', async () => {
    const wrapper = await panel();
    const toggle = wrapper.get('input[aria-label="全按键支持"]');
    expect((toggle.element as HTMLInputElement).checked).toBe(false);
    expect(wrapper.text()).not.toContain('刷新状态');
    expect(wrapper.find('.capture-help').exists()).toBe(false);
    await toggle.setValue(true);
    expect(wrapper.find('dialog').exists()).toBe(true);
    expect((toggle.element as HTMLInputElement).checked).toBe(false);
    expect(mocks.enableRc003Capture).not.toHaveBeenCalled();
  });
  beforeEach(() => {
    vi.resetAllMocks();
    mocks.getComponentStatus.mockResolvedValue(status);
    mocks.getCaptureSupport.mockResolvedValue({ available: true, reason: null, nativeArch: "x64", helperExpected: "sayall-helper.exe" });
    mocks.getRc003TaskStatus.mockResolvedValue(disabled);
    mocks.getRc003BridgeSnapshot.mockResolvedValue(listening);
    mocks.enableRc003Capture.mockResolvedValue({ ...disabled, enabled: true });
    mocks.disableRc003Capture.mockResolvedValue(disabled);
  });
  afterEach(() => { for (const wrapper of wrappers.splice(0)) wrapper.unmount(); vi.useRealTimers(); });
  it("disables the local support entry on an unsupported host without requesting authorization", async () => {
    mocks.getCaptureSupport.mockResolvedValue({ available: false, reason: "arch_unsupported", nativeArch: "arm64", helperExpected: "sayall-helper.exe" });
    const wrapper = await panel();
    expect(wrapper.get('[role="status"]').text()).toContain("暂不可用");
    expect(captureToggle(wrapper).attributes("disabled")).toBeDefined();
    await captureToggle(wrapper).setValue(true);
    expect(wrapper.find("dialog").exists()).toBe(false);
    expect(mocks.enableRc003Capture).not.toHaveBeenCalled();
  });
  it("requires explicit confirmation on every enable even with an existing task", async () => {
    const wrapper = await panel();
    expect(wrapper.text()).toContain("全按键支持");
    expect(wrapper.text()).not.toMatch(/三键|五键|随应用自动启动/);
    expect(wrapper.get('[role="status"]').text()).toContain("已关闭");
    expect(mocks.enableRc003Capture).not.toHaveBeenCalled();
    await captureToggle(wrapper).setValue(true);
    expect(wrapper.find("dialog").exists()).toBe(true);
    expect(mocks.enableRc003Capture).not.toHaveBeenCalled();
    await button(wrapper, "取消").trigger("click");
    expect(wrapper.find("dialog").exists()).toBe(false);
    await captureToggle(wrapper).setValue(true);
    await button(wrapper, "开启").trigger("click"); await flushPromises();
    expect(mocks.enableRc003Capture).toHaveBeenCalledTimes(1);
    expect(wrapper.get('[role="status"]').text()).toContain("等待启动完成");
  });
  it("does not mark capture ready after authorization alone and exposes a confirmed retry", async () => {
    mocks.getRc003TaskStatus.mockResolvedValue({ ...disabled, enabled: true });
    mocks.getRc003BridgeSnapshot.mockResolvedValue({ ...listening, phase: "failed" });
    const wrapper = await panel();
    expect(wrapper.get('[role="status"]').text()).toContain("运行异常");
    expect(wrapper.get('[role="status"]').text()).not.toContain("已接管");
    await button(wrapper, "重新授权并启动").trigger("click");
    expect(wrapper.find("dialog").exists()).toBe(true);
    await button(wrapper, "开启").trigger("click"); await flushPromises();
    expect(mocks.enableRc003Capture).toHaveBeenCalledTimes(1);
  });
  it("preserves disabled state and a visible error when Windows authorization is cancelled", async () => {
    mocks.enableRc003Capture.mockRejectedValue(new Error("授权未完成（UAC 被取消）"));
    const wrapper = await panel();
    await captureToggle(wrapper).setValue(true);
    await button(wrapper, "开启").trigger("click"); await flushPromises();
    expect(wrapper.get('[role="alert"]').text()).toContain("授权");
    expect(wrapper.get('[role="status"]').text()).toContain("未能开启");
    expect(mocks.disableRc003Capture).not.toHaveBeenCalled();
  });
  it.each([false, true])("shows the backend failure reason without repeating a generic authorization error when enabled=%s", async (enabled) => {
    const reason = "旧版按键组件仍被 Windows 占用。请在下次正常重启 Windows 后重新开启全按键支持。";
    const wrapper = await panel();
    mocks.getRc003TaskStatus.mockResolvedValue({ ...disabled, enabled, lastError: reason });
    mocks.enableRc003Capture.mockRejectedValue(new Error("启动未完成"));
    await captureToggle(wrapper).setValue(true);
    await button(wrapper, "开启").trigger("click"); await flushPromises();
    expect(wrapper.get(".capture-state-copy").text()).toContain(reason);
    expect(wrapper.find('[role="alert"]').exists()).toBe(false);
    expect(wrapper.get('[role="status"]').text()).toContain(enabled ? "运行异常" : "未能开启");
  });
  it("recovers from a bridge read failure after a successful enable without reporting an authorization failure", async () => {
    vi.useFakeTimers();
    const wrapper = await panel();
    mocks.getRc003TaskStatus.mockResolvedValue({ ...disabled, enabled: true });
    mocks.getRc003BridgeSnapshot.mockRejectedValueOnce(new Error("bridge read unavailable")).mockResolvedValue({ ...listening, phase: "connected", targetUsages: [40], ownedUsages: [40] });
    await captureToggle(wrapper).setValue(true);
    await button(wrapper, "开启").trigger("click"); await flushPromises();
    expect(mocks.enableRc003Capture).toHaveBeenCalledOnce();
    expect(wrapper.get('[role="status"]').text()).toContain("状态读取失败");
    expect(wrapper.find('[role="alert"]').exists()).toBe(false);
    vi.advanceTimersByTime(1000); await flushPromises();
    expect(wrapper.get('[role="status"]').text()).toContain("已开启");
    expect(wrapper.find('[role="alert"]').exists()).toBe(false);
    expect(captureToggle(wrapper).attributes("disabled")).toBeUndefined();
  });
  it("serializes repeated clicks while authorization is pending", async () => {
    let finish!: (value: typeof disabled) => void;
    mocks.enableRc003Capture.mockImplementation(() => new Promise(done => { finish = done; }));
    const wrapper = await panel();
    await captureToggle(wrapper).setValue(true);
    await button(wrapper, "开启").trigger("click");
    expect(wrapper.find("dialog").exists()).toBe(false);
    expect(captureToggle(wrapper).attributes("disabled")).toBeDefined();
    expect(mocks.enableRc003Capture).toHaveBeenCalledTimes(1);
    finish({ ...disabled, enabled: true }); await flushPromises();
  });
  it("disables without authorization and refreshes actual state after a stop failure", async () => {
    mocks.getRc003TaskStatus.mockResolvedValueOnce({ ...disabled, enabled: true }).mockResolvedValue({ ...disabled, cleanupPending: true, canRetryCleanup: true });
    mocks.disableRc003Capture.mockRejectedValue(new Error("助手停止尚未确认"));
    const wrapper = await panel();
    await captureToggle(wrapper).setValue(false); await flushPromises();
    expect(mocks.disableRc003Capture).toHaveBeenCalledTimes(1);
    expect(wrapper.find("dialog").exists()).toBe(false);
    expect(wrapper.get('[role="status"]').text()).toContain("关闭待完成");
    expect(captureToggle(wrapper).attributes("disabled")).toBeDefined();
    expect(button(wrapper, "重试关闭")).toBeDefined();
  });
  it("reports ownership acknowledgement separately from merely connecting the helper", async () => {
    vi.useFakeTimers();
    mocks.getRc003TaskStatus.mockResolvedValue({ ...disabled, enabled: true });
    mocks.getRc003BridgeSnapshot.mockResolvedValue({ ...listening, phase: "connected", helperPid: 7, targetUsages: [40], ownedUsages: [] });
    const wrapper = await panel();
    expect(wrapper.get('[role="status"]').text()).toContain("正在应用按键配置");
    mocks.getRc003BridgeSnapshot.mockResolvedValue({ ...listening, phase: "connected", helperPid: 7, targetUsages: [40], ownedUsages: [40] });
    vi.advanceTimersByTime(1000); await flushPromises();
    expect(wrapper.get('[role="status"]').text()).toContain("已开启");
    mocks.getRc003BridgeSnapshot.mockResolvedValue(listening);
    vi.advanceTimersByTime(1000); await flushPromises();
    expect(wrapper.get('[role="status"]').text()).toContain("等待启动完成");
  });
  it("clears a timed-out close after a later poll confirms that cleanup completed", async () => {
    vi.useFakeTimers();
    mocks.getRc003TaskStatus.mockResolvedValueOnce({ ...disabled, enabled: true }).mockResolvedValueOnce({ ...disabled, cleanupPending: true, canRetryCleanup: true }).mockResolvedValue(disabled);
    mocks.disableRc003Capture.mockRejectedValue(new Error("cleanup timeout"));
    const wrapper = await panel();
    await captureToggle(wrapper).setValue(false); await flushPromises();
    expect(wrapper.get('[role="status"]').text()).toContain("关闭待完成");
    vi.advanceTimersByTime(1000); await flushPromises();
    expect(wrapper.get('[role="status"]').text()).toContain("已关闭");
    expect(wrapper.find('[role="alert"]').exists()).toBe(false);
    expect(captureToggle(wrapper).attributes("disabled")).toBeUndefined();
  });
  it("keeps a cancelled enable visible when polling merely confirms the disabled preference", async () => {
    vi.useFakeTimers();
    mocks.enableRc003Capture.mockRejectedValue(new Error("authorization cancelled"));
    const wrapper = await panel();
    await captureToggle(wrapper).setValue(true);
    await button(wrapper, "开启").trigger("click"); await flushPromises();
    vi.advanceTimersByTime(1000); await flushPromises();
    expect(wrapper.get('[role="status"]').text()).toContain("未能开启");
    expect(wrapper.get('[role="alert"]').text()).toContain("授权");
  });
  it('clears an ambiguous enable failure only after the same session confirms complete ownership', async () => {
    vi.useFakeTimers();
    mocks.enableRc003Capture.mockRejectedValue(new Error('response lost'));
    const wrapper = await panel();
    await captureToggle(wrapper).setValue(true);
    await button(wrapper, '开启').trigger('click'); await flushPromises();
    expect(wrapper.get('[role="status"]').text()).toContain('未能开启');
    mocks.getRc003TaskStatus.mockResolvedValue({...disabled, enabled: true});
    mocks.getRc003BridgeSnapshot.mockResolvedValue({...listening, phase: 'connected', targetUsages: [], ownedUsages: []});
    vi.advanceTimersByTime(1000); await flushPromises();
    expect(wrapper.find('[role="alert"]').exists()).toBe(true);
    mocks.getRc003BridgeSnapshot.mockResolvedValue({...listening, phase: 'connected', targetUsages: [40, 41], ownedUsages: [40]});
    vi.advanceTimersByTime(1000); await flushPromises();
    expect(wrapper.find('[role="alert"]').exists()).toBe(true);
    mocks.getRc003BridgeSnapshot.mockResolvedValue({...listening, phase: 'connected', targetUsages: [40, 41], ownedUsages: [40, 41]});
    vi.advanceTimersByTime(1000); await flushPromises();
    expect(wrapper.get('[role="status"]').text()).toBe('已开启');
    expect(wrapper.find('[role="alert"]').exists()).toBe(false);
    expect((captureToggle(wrapper).element as HTMLInputElement).checked).toBe(true);
  });
  it.each(['cleanup', 'task_error', 'disconnected'] as const)('does not erase the enable error when complete usages coexist with %s', async (reason) => {
    vi.useFakeTimers();
    mocks.enableRc003Capture.mockRejectedValue(new Error('response lost'));
    const wrapper = await panel();
    await captureToggle(wrapper).setValue(true);
    await button(wrapper, '开启').trigger('click'); await flushPromises();
    mocks.getRc003TaskStatus.mockResolvedValue({...disabled, enabled: true, cleanupPending: reason === 'cleanup', lastError: reason === 'task_error' ? '启动失败' : null});
    mocks.getRc003BridgeSnapshot.mockResolvedValue({...listening, phase: reason === 'disconnected' ? 'listening' : 'connected', targetUsages: [40], ownedUsages: [40]});
    vi.advanceTimersByTime(1000); await flushPromises();
    // Once blockers disappear, a disabled readback must still expose the original error.
    mocks.getRc003TaskStatus.mockResolvedValue(disabled);
    vi.advanceTimersByTime(1000); await flushPromises();
    expect(wrapper.get('[role="status"]').text()).toBe('未能开启');
    expect(wrapper.get('[role="alert"]').text()).toContain('授权');
  });
  it("ignores a stale poll completed after a successful authorization", async () => {
    vi.useFakeTimers();
    const wrapper = await panel();
    let finish!: (value: typeof disabled) => void;
    mocks.getRc003TaskStatus.mockImplementationOnce(() => new Promise(done => { finish = done; }));
    vi.advanceTimersByTime(1000); await flushPromises();
    await captureToggle(wrapper).setValue(true);
    await button(wrapper, "开启").trigger("click"); await flushPromises();
    finish(disabled); await flushPromises();
    expect(wrapper.get('[role="status"]').text()).toContain("等待启动完成");
  });
  it("keeps cleanup failure above the saved disabled preference and offers only cleanup retry", async () => {
    mocks.getRc003TaskStatus.mockResolvedValue({ ...disabled, cleanupPending: true, canRetryCleanup: true, lastError: "instance=4 cleanup_ack_missing" });
    const wrapper = await panel();
    expect(wrapper.get('[role="status"]').text()).toContain("关闭待完成");
    expect(wrapper.text()).not.toMatch(/instance|ACK|cleanup_ack/);
    expect(captureToggle(wrapper).attributes("disabled")).toBeDefined();
    await button(wrapper, "重试关闭").trigger("click"); await flushPromises();
    expect(mocks.disableRc003Capture).toHaveBeenCalledOnce();
    expect(mocks.enableRc003Capture).not.toHaveBeenCalled();
    expect(wrapper.find("dialog").exists()).toBe(false);
    expect(wrapper.get('[role="status"]').text()).toContain("已关闭");
  });
  it("allows recovery after the old helper exited and waits for confirmed cleanup before enabling", async () => {
    mocks.getRc003TaskStatus.mockResolvedValue({ ...disabled, cleanupPending: true, canRetryCleanup: true });
    mocks.getRc003BridgeSnapshot.mockResolvedValue({ ...listening, phase: "stopped", helperPid: 0 });
    let finishRecovery!: (value: typeof disabled) => void;
    mocks.disableRc003Capture.mockImplementation(() => new Promise(done => { finishRecovery = done; }));
    const wrapper = await panel();
    expect(wrapper.get(".capture-state-copy").text()).toContain("自动恢复残留的按键状态");
    expect(button(wrapper, "重试关闭").attributes("disabled")).toBeUndefined();
    await button(wrapper, "重试关闭").trigger("click");
    expect(wrapper.get('[role="status"]').text()).toContain("正在恢复按键状态");
    expect(captureToggle(wrapper).attributes("disabled")).toBeDefined();
    expect(captureToggle(wrapper).attributes("disabled")).toBeDefined();
    expect(mocks.disableRc003Capture).toHaveBeenCalledOnce();
    finishRecovery(disabled); await flushPromises();
    expect(wrapper.get('[role="status"]').text()).toContain("已关闭");
    expect(captureToggle(wrapper).attributes("disabled")).toBeUndefined();
  });
  it("immediately reads back a failed enable even while an older poll is unfinished", async () => {
    vi.useFakeTimers();
    const wrapper = await panel();
    let finishOldPoll!: (value: typeof disabled) => void;
    mocks.getRc003TaskStatus.mockImplementationOnce(() => new Promise(done => { finishOldPoll = done; }));
    vi.advanceTimersByTime(1000); await flushPromises();
    mocks.getRc003TaskStatus.mockResolvedValue({ ...disabled, cleanupPending: true, canRetryCleanup: false });
    mocks.enableRc003Capture.mockRejectedValue(new Error("cleanup_ack_missing"));
    await captureToggle(wrapper).setValue(true);
    await button(wrapper, "开启").trigger("click"); await flushPromises();
    expect(mocks.getRc003TaskStatus).toHaveBeenCalledTimes(3);
    expect(wrapper.get('[role="status"]').text()).toContain("按键支持暂不可用");
    expect(wrapper.findAll(".capture-controls button")).toHaveLength(0);
    finishOldPoll(disabled); await flushPromises();
    expect(wrapper.get('[role="status"]').text()).toContain("按键支持暂不可用");
  });
  it("does not offer a cleanup retry when the previous session can no longer complete it", async () => {
    vi.useFakeTimers();
    mocks.getRc003TaskStatus.mockResolvedValue({ ...disabled, cleanupPending: true, canRetryCleanup: false, lastError: "instance=4 cleanup_ack_missing" });
    const wrapper = await panel();
    expect(wrapper.get('[role="status"]').text()).toContain("按键支持暂不可用");
    expect(wrapper.text()).not.toMatch(/instance|ACK|cleanup_ack/);
    expect(wrapper.findAll(".capture-controls button")).toHaveLength(0);
    expect(captureToggle(wrapper).attributes("disabled")).toBeDefined();
    vi.advanceTimersByTime(1000); await flushPromises();
    expect(wrapper.get('[role="status"]').text()).toContain("按键支持暂不可用");
    expect(mocks.disableRc003Capture).not.toHaveBeenCalled();
    expect(mocks.enableRc003Capture).not.toHaveBeenCalled();
  });
  it("clears read failure only after a successful read and prevents enabling from stale state", async () => {
    vi.useFakeTimers();
    const wrapper = await panel();
    mocks.getRc003TaskStatus.mockRejectedValueOnce(new Error("snapshot unavailable"));
    vi.advanceTimersByTime(1000); await flushPromises();
    expect(wrapper.get('[role="status"]').text()).toContain("状态读取失败");
    expect(captureToggle(wrapper).attributes("disabled")).toBeDefined();
    vi.advanceTimersByTime(1000); await flushPromises();
    expect(wrapper.get('[role="status"]').text()).toContain("已关闭");
    expect(wrapper.find('[role="alert"]').exists()).toBe(false);
    expect(captureToggle(wrapper).attributes("disabled")).toBeUndefined();
  });
  it("waits without offering repeated enable and does not duplicate the authorization dialog help", async () => {
    mocks.getRc003TaskStatus.mockResolvedValue({ ...disabled, enabled: true });
    const wrapper = await panel();
    expect(wrapper.get('[role="status"]').text()).toContain("等待启动完成");
    expect(button(wrapper, "重新授权并启动")).toBeUndefined();
    expect((captureToggle(wrapper).element as HTMLInputElement).checked).toBe(true);
    expect(wrapper.find(".capture-help").exists()).toBe(false);
  });
  it("owns capture support without audio component maintenance", async () => {
    const wrapper = await panel();
    expect(wrapper.text()).not.toContain("签名驱动组件");
    expect(wrapper.text()).not.toContain("VB-CABLE");
    expect(mocks.getComponentStatus).not.toHaveBeenCalled();
    expect(mocks.performComponentAction).not.toHaveBeenCalled();
  });
  it("keeps the current template slot immediately after capture controls without authorizing capture", async () => {
    const wrapper = mount(ComponentSupportPanel, {slots: {default: '<label>当前模板<select aria-label="当前模板"><option>模板 A</option></select></label>'}});
    wrappers.push(wrapper); await flushPromises();
    expect(wrapper.get('.support-heading select').attributes('aria-label')).toBe('当前模板');
    expect(wrapper.get('.support-heading .capture-controls').text()).toContain('全按键支持');
    expect(mocks.enableRc003Capture).not.toHaveBeenCalled();
  });
});
