import { flushPromises, mount } from "@vue/test-utils";
import { beforeEach, describe, expect, it, vi } from "vitest";
import ComponentSupportPanel from "./ComponentSupportPanel.vue";

const mocks = vi.hoisted(() => ({
  getComponentStatus: vi.fn(), performComponentAction: vi.fn(),
  getHidHostStatus: vi.fn().mockResolvedValue("未启动"),
  startHidHostEnhancement: vi.fn(),
  getHidHostAutoRestore: vi.fn().mockResolvedValue(false),
  setHidHostAutoRestore: vi.fn(),
}));
vi.mock("../lib/bridge", async (importOriginal) => ({ ...(await importOriginal<typeof import("../lib/bridge")>()), ...mocks }));

const status = { component: "vb_cable", installation: "available", package: "download_available", installedVersion: null, serviceInstalled: true, loaded: true, bound: null, audioEndpointsReady: true, restartRequired: false, reason: "ready", blockers: ["official_wizard_required"], allowedActions: ["open_vendor_wizard"] } as const;

describe("component support", () => {
  beforeEach(() => vi.clearAllMocks());
  it("persists explicit startup opt-in without disguising it as a manual start", async () => {
    mocks.getComponentStatus.mockResolvedValue([]);
    mocks.setHidHostAutoRestore.mockResolvedValue(true);
    const wrapper = mount(ComponentSupportPanel); await flushPromises();
    const checkbox = wrapper.get('input[type="checkbox"]');
    expect((checkbox.element as HTMLInputElement).checked).toBe(false);
    await checkbox.setValue(true); await flushPromises();
    expect(mocks.setHidHostAutoRestore).toHaveBeenCalledExactlyOnceWith(true);
    expect(mocks.startHidHostEnhancement).not.toHaveBeenCalled();
    expect((checkbox.element as HTMLInputElement).checked).toBe(true);
    expect(wrapper.text()).toContain("每次仍可能出现 UAC");
    wrapper.unmount();
  });
  it("shows only backend-authorized HID maintenance and requests that fixed action", async () => {
    const hid = {...status, component:"hid_enhancement",package:"trusted",installation:"not_installed",allowedActions:["install"]};
    mocks.getComponentStatus.mockResolvedValue([hid]);
    mocks.performComponentAction.mockResolvedValue({component:"hid_enhancement",action:"install",outcome:"restart_required",reason:"restart_required",status:hid});
    const wrapper=mount(ComponentSupportPanel);await flushPromises();
    expect(wrapper.text()).not.toContain("尚无 Microsoft 签名");
    expect(wrapper.findAll("button").some(button=>button.text()==="卸载按键增强")).toBe(false);
    await wrapper.findAll("button").find(button=>button.text()==="安装按键增强")!.trigger("click");await flushPromises();
    expect(mocks.performComponentAction).toHaveBeenCalledWith("hid_enhancement","install");
    expect(wrapper.text()).toContain("需要重启");
  });
  it("keeps the wizard-closed result visible after it refreshes detection", async () => {
    mocks.getComponentStatus.mockResolvedValue([status, { ...status, component: "hid_enhancement", installation: "not_implemented", allowedActions: [] }]);
    mocks.performComponentAction.mockResolvedValue({ component: "vb_cable", action: "open_vendor_wizard", outcome: "wizard_closed", reason: "wizard_closed", status });
    const wrapper = mount(ComponentSupportPanel);
    await flushPromises();
    expect(wrapper.text()).toContain("1. 按键增强驱动");
    expect(wrapper.text()).toContain("尚无 Microsoft 签名");
    expect(wrapper.findAll("button").some(button => button.text() === "安装按键增强")).toBe(false);
    expect(wrapper.text()).toContain("2. VB-CABLE");
    expect(wrapper.text()).toContain("点击下方按钮即表示由您主动开始");
    await wrapper.findAll("button").find((button) => button.text().includes("打开官方安装"))!.trigger("click");
    await flushPromises();
    expect(mocks.performComponentAction).toHaveBeenCalledWith("vb_cable", "open_vendor_wizard");
    expect(wrapper.text()).toContain("向导已关闭");
    expect(mocks.getComponentStatus).toHaveBeenCalledTimes(2);
  });
  it("does not imply a timed-out vendor wizard is safe to start again", async () => {
    mocks.getComponentStatus.mockResolvedValue([status]);
    mocks.performComponentAction.mockResolvedValue({ component: "vb_cable", action: "open_vendor_wizard", outcome: "timed_out", reason: "helper_timed_out", status });
    const wrapper = mount(ComponentSupportPanel);
    await flushPromises();
    await wrapper.findAll("button").find((button) => button.text().includes("打开官方安装"))!.trigger("click");
    await flushPromises();
    expect(wrapper.text()).toContain("向导可能仍在运行");
  });
});
