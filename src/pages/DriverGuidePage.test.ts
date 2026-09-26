import { flushPromises, mount } from "@vue/test-utils";
import { beforeEach, describe, expect, it, vi } from "vitest";
import DriverGuidePage from "./DriverGuidePage.vue";
import type { ConnectionSnapshot, RuntimeSnapshot } from "../lib/bridge";

const mocks = vi.hoisted(() => ({
  getConnectionSnapshot: vi.fn(), scanPairedRemotes: vi.fn(), connectRemote: vi.fn(), openBluetoothSettings: vi.fn(),
}));
vi.mock("../lib/bridge", async (importOriginal) => ({ ...(await importOriginal<typeof import("../lib/bridge")>()), ...mocks }));

const idle = { phase: "idle", remoteName: null, remoteModel: "unknown", capabilities: null, voiceState: "idle", decodedSamples: 0, generation: 0, reconnectAttempt: 0, powerNotificationsAvailable: false, lastError: null } as const;
const runtime: RuntimeSnapshot = { appVersion: "test", platform: { platform: "windows", windowsApiAvailable: true, bleScanAvailable: true, bleVoiceReady: false, wasapiReady: false, rawInputReady: false, sendInputReady: false, verificationStatus: "test", connection: idle, audio: { phase: "unconfigured", selectedEndpointId: null, selectedEndpointName: null, queuedSamples: 0, submittedSamples: 0, generation: 0, lastError: null }, rawInput: { phase: "stopped", matchedDeviceCount: 0, rawEventCount: 0, staleRemoteEventCount: 0, semanticEdgeCount: 0, lastButton: null, lastIsPressed: null, activeButtons: [], lastError: null }, buttonMapping: { enabled: true, gateActive: false, observedButtons: [], listenerActive: false, swallowedEdges: 0, leakedDowns: 0, firedGestures: 0, lastFired: null, lastError: null } } };

describe("driver guide", () => {
  beforeEach(() => { vi.clearAllMocks(); mocks.getConnectionSnapshot.mockResolvedValue(idle); mocks.scanPairedRemotes.mockResolvedValue([{ id: "rc003", name: "遥控器", model: "rc003", isSupportedCandidate: true }]); mocks.connectRemote.mockResolvedValue(idle); mocks.openBluetoothSettings.mockResolvedValue(undefined); });
  it("keeps pairing user-initiated and labels a scan as Windows pairing rather than connection", async () => {
    const wrapper = mount(DriverGuidePage, { props: { runtime }, global: { stubs: { ComponentSupportPanel: true } } });
    await flushPromises();
    expect(mocks.scanPairedRemotes).not.toHaveBeenCalled();
    expect(wrapper.text()).toContain("菜单和主页");
    await wrapper.get("button:not(.secondary-button)").trigger("click");
    await flushPromises();
    expect(wrapper.text()).toContain("Windows 已配对并被 SayAll 发现");
    expect(wrapper.text()).not.toContain("已完成连接");
  });
  it("keeps pairing guidance and Windows settings available after connection", async () => {
    const connected: ConnectionSnapshot = { ...idle, phase: "ready", remoteName: "遥控器", remoteModel: "rc003", capabilities: { version: 1, codecs: 1, interaction: 1, frameSize: 20, selectedCodec: 1, sampleRate: 16_000 } };
    mocks.getConnectionSnapshot.mockResolvedValue(connected);
    const wrapper = mount(DriverGuidePage, { props: { runtime: { ...runtime, platform: { ...runtime.platform, connection: connected } } }, global: { stubs: { ComponentSupportPanel: true } } });
    await flushPromises();
    expect(wrapper.text()).toContain("已完成连接");
    expect(wrapper.text()).toContain("重新配对方法");
    expect(wrapper.text()).toContain("菜单和主页");
    expect(wrapper.text()).toContain("打开 Windows 蓝牙设置");
    expect(mocks.scanPairedRemotes).not.toHaveBeenCalled();
    await wrapper.findAll("button").find((button) => button.text() === "打开 Windows 蓝牙设置")!.trigger("click");
    expect(mocks.openBluetoothSettings).toHaveBeenCalledTimes(1);
  });
});
