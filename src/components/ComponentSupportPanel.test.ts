import { flushPromises, mount } from "@vue/test-utils";
import { describe, expect, it, vi } from "vitest";
import ComponentSupportPanel from "./ComponentSupportPanel.vue";

const mocks = vi.hoisted(() => ({
  getComponentStatus: vi.fn(), performComponentAction: vi.fn(),
}));
vi.mock("../lib/bridge", async (importOriginal) => ({ ...(await importOriginal<typeof import("../lib/bridge")>()), ...mocks }));

const status = { component: "vb_cable", installation: "available", package: "download_available", installedVersion: null, serviceInstalled: true, loaded: true, bound: null, audioEndpointsReady: true, restartRequired: false, reason: "ready", blockers: ["official_wizard_required"], allowedActions: ["open_vendor_wizard"] } as const;

describe("component support", () => {
  it("keeps the wizard-closed result visible after it refreshes detection", async () => {
    mocks.getComponentStatus.mockResolvedValue([status, { ...status, component: "hid_enhancement", installation: "not_implemented", allowedActions: [] }]);
    mocks.performComponentAction.mockResolvedValue({ component: "vb_cable", action: "open_vendor_wizard", outcome: "wizard_closed", reason: "wizard_closed", status });
    const wrapper = mount(ComponentSupportPanel);
    await flushPromises();
    await wrapper.findAll("button").find((button) => button.text().includes("打开官方安装"))!.trigger("click");
    await wrapper.findAll("button").find((button) => button.text() === "继续打开向导")!.trigger("click");
    await flushPromises();
    expect(wrapper.text()).toContain("向导已关闭");
    expect(mocks.getComponentStatus).toHaveBeenCalledTimes(2);
  });
  it("does not imply a timed-out vendor wizard is safe to start again", async () => {
    mocks.getComponentStatus.mockResolvedValue([status]);
    mocks.performComponentAction.mockResolvedValue({ component: "vb_cable", action: "open_vendor_wizard", outcome: "timed_out", reason: "helper_timed_out", status });
    const wrapper = mount(ComponentSupportPanel);
    await flushPromises();
    await wrapper.findAll("button").find((button) => button.text().includes("打开官方安装"))!.trigger("click");
    await wrapper.findAll("button").find((button) => button.text() === "继续打开向导")!.trigger("click");
    await flushPromises();
    expect(wrapper.text()).toContain("向导可能仍在运行");
  });
});
