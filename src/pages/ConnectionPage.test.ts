import { flushPromises, mount, type VueWrapper } from "@vue/test-utils";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type {
  AudioEndpoint,
  AudioSnapshot,
  CaptureSupport,
  ConnectionSnapshot,
  Rc003BridgeSnapshot,
  RuntimeSnapshot,
} from "../lib/bridge";
import { VOICE_HOTKEY_CUSTOM_CAPTURE_ENABLED } from "../lib/feature-flags";
import { effectiveLanguage } from "../lib/language-state";
import ConnectionPage from "./ConnectionPage.vue";
import { reportFrontendEvent } from "../lib/frontend-diagnostics";
vi.mock("../lib/frontend-diagnostics", async (importOriginal) => ({ ...await importOriginal<typeof import("../lib/frontend-diagnostics")>(), reportFrontendEvent: vi.fn() }));

type ShortcutCaptureHandler = (edge: {
  key: string;
  isPressed: boolean;
  source?: "real" | "injected";
}) => void;

/**
 * 落盘稳定窗口（200ms）之后才定稿：外部钩子吞掉完成键的物理边沿后会以注入
 * 副本重放整个组合，副本可能晚于物理松开到达（见 ConnectionPage 的
 * scheduleVoiceCaptureFinish），因此断言落盘前必须等过该窗口。
 */
async function settleVoiceCapture(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 260));
  await flushPromises();
}

/** 当前选中（高亮）的输入工具卡片。 */
function selectedToolCard(wrapper: VueWrapper): string {
  return wrapper.find(".tool-card.selected").find("strong").text();
}

/** 切到"其他工具"并进入自定义组合键录入。 */
async function startCustomCapture(wrapper: VueWrapper): Promise<void> {
  await wrapper
    .findAll(".tool-card")
    .find((card) => card.text().includes("其他工具"))!
    .trigger("click");
  await flushPromises();
  await wrapper
    .findAll(".chip-select .chip")
    .find((chip) => chip.text() === "自定义组合键")!
    .trigger("click");
  await flushPromises();
}

const emptyConnection: ConnectionSnapshot = {
  phase: "idle",
  remoteName: null,
  remoteModel: "unknown",
  capabilities: null,
  voiceState: "idle",
  decodedSamples: 0,
  generation: 0,
  reconnectAttempt: 0,
  powerNotificationsAvailable: false,
  lastError: null,
};

const emptyAudio: AudioSnapshot = {
  phase: "unconfigured",
  selectedEndpointId: null,
  selectedEndpointName: null,
  queuedSamples: 0,
  submittedSamples: 0,
  generation: 0,
  lastError: null,
};

const runtime: RuntimeSnapshot = {
  appVersion: "0.1.0",
  platform: {
    platform: "windows",
    windowsApiAvailable: true,
    bleScanAvailable: true,
    bleVoiceReady: false,
    wasapiReady: false,
    rawInputReady: false,
    sendInputReady: true,
    verificationStatus: "测试",
    connection: emptyConnection,
    audio: emptyAudio,
    rawInput: {
      phase: "stopped",
      matchedDeviceCount: 0,
      rawEventCount: 0,
      semanticEdgeCount: 0,
      lastButton: null,
      lastIsPressed: null,
      activeButtons: [],
      lastError: null,
      staleRemoteEventCount: 0,
    },
    buttonMapping: {
      enabled: true,
      gateActive: false,
      observedButtons: [], listenerActive: false,
      swallowedEdges: 0,
      leakedDowns: 0,
      firedGestures: 0,
      lastFired: null,
      lastError: null,
    },
  },
};

const cableEndpoint: AudioEndpoint = {
  id: "cable-input",
  name: "CABLE Input (VB-Audio Virtual Cable)",
  isVirtualCableCandidate: true,
};

function supportedCaptureSupport(): CaptureSupport {
  return { available: true, reason: null, nativeArch: "x64", helperExpected: "sayall-helper.exe" };
}

function unsupportedCaptureSupport(): CaptureSupport {
  return { available: false, reason: "helper_missing", nativeArch: "arm64", helperExpected: "sayall-helper.exe" };
}

const mocks = vi.hoisted(() => ({
  endpoints: [] as AudioEndpoint[],
  getRc003BridgeSnapshot: vi.fn(),
  getGainDb: vi.fn(), setGainDb: vi.fn(), getCaptureSupport: vi.fn(), getRc003TaskStatus: vi.fn(), enableRc003Capture: vi.fn(), disableRc003Capture: vi.fn(),
  getCaptureInput: vi.fn(), listCaptureInputs: vi.fn(), setCaptureInput: vi.fn(), resolveCaptureRecovery: vi.fn(),
  getDefaultCaptureInputs: vi.fn(), setDefaultCaptureInput: vi.fn(),
  captureEdgeHandler: null as ShortcutCaptureHandler | null,
  getConnectionSnapshot: vi.fn(),
  getAudioSnapshot: vi.fn(),
  getAudioRouteSnapshot: vi.fn(),
  listAudioEndpoints: vi.fn(),
  selectAudioEndpoint: vi.fn(),
  openVbCableDownloadPage: vi.fn(),
  getVoiceHoldHotkey: vi.fn(),
  setVoiceHoldHotkey: vi.fn(),
  getVoiceInputTool: vi.fn(),
  setVoiceInputTool: vi.fn(),
  getVokieInstallation: vi.fn(),
  openVokieHomepage: vi.fn(),
  launchVokie: vi.fn(),
  getOtherVoiceHotkey: vi.fn(),
  setOtherVoiceHotkey: vi.fn(),
  startShortcutCapture: vi.fn(),
  stopShortcutCapture: vi.fn(),
  subscribeShortcutCaptureEdges: vi.fn(),
}));

vi.mock("../lib/bridge", async (importOriginal) => {
  const original = await importOriginal<typeof import("../lib/bridge")>();
  return {
    ...original,
    getRc003BridgeSnapshot: mocks.getRc003BridgeSnapshot,
    getGainDb: mocks.getGainDb, setGainDb: mocks.setGainDb, getCaptureSupport: mocks.getCaptureSupport, getRc003TaskStatus: mocks.getRc003TaskStatus, enableRc003Capture: mocks.enableRc003Capture, disableRc003Capture: mocks.disableRc003Capture,
    getCaptureInput: mocks.getCaptureInput, listCaptureInputs: mocks.listCaptureInputs,
    getDefaultCaptureInputs: mocks.getDefaultCaptureInputs, setDefaultCaptureInput: mocks.setDefaultCaptureInput,
    setCaptureInput: mocks.setCaptureInput, resolveCaptureRecovery: mocks.resolveCaptureRecovery,
    getConnectionSnapshot: mocks.getConnectionSnapshot,
    getAudioSnapshot: mocks.getAudioSnapshot,
    getAudioRouteSnapshot: mocks.getAudioRouteSnapshot,
    listAudioEndpoints: mocks.listAudioEndpoints,
    selectAudioEndpoint: mocks.selectAudioEndpoint,
    openVbCableDownloadPage: mocks.openVbCableDownloadPage,
    getVoiceHoldHotkey: mocks.getVoiceHoldHotkey,
    setVoiceHoldHotkey: mocks.setVoiceHoldHotkey,
    getVoiceInputTool: mocks.getVoiceInputTool,
    setVoiceInputTool: mocks.setVoiceInputTool,
    getVokieInstallation: mocks.getVokieInstallation,
    openVokieHomepage: mocks.openVokieHomepage,
    launchVokie: mocks.launchVokie,
    getOtherVoiceHotkey: mocks.getOtherVoiceHotkey,
    setOtherVoiceHotkey: mocks.setOtherVoiceHotkey,
    startShortcutCapture: mocks.startShortcutCapture,
    stopShortcutCapture: mocks.stopShortcutCapture,
    subscribeShortcutCaptureEdges: mocks.subscribeShortcutCaptureEdges,
  };
});

describe("VB-CABLE first-launch guidance", () => {
  beforeEach(() => {
    mocks.getDefaultCaptureInputs.mockResolvedValue({console: {id: 'pc', name: 'PC microphone', isVirtualCableCandidate: false}, multimedia: {id: 'pc', name: 'PC microphone', isVirtualCableCandidate: false}, communications: null});
    mocks.setDefaultCaptureInput.mockResolvedValue({console: {id: 'capture', name: 'CABLE Output', isVirtualCableCandidate: true}, multimedia: {id: 'capture', name: 'CABLE Output', isVirtualCableCandidate: true}, communications: {id: 'capture', name: 'CABLE Output', isVirtualCableCandidate: true}});
    mocks.endpoints = [];
    mocks.getGainDb.mockResolvedValue(0);
    mocks.setGainDb.mockImplementation(async value => value);
    mocks.getCaptureSupport.mockResolvedValue({ available: true, reason: null, nativeArch: "x64", helperExpected: "sayall-helper.exe" });
    mocks.getRc003TaskStatus.mockResolvedValue({ installed: false, authorizationRequired: true, enabled: false, lastError: null });
    mocks.enableRc003Capture.mockResolvedValue({ installed: true, authorizationRequired: false, enabled: true, lastError: null });
    mocks.disableRc003Capture.mockResolvedValue({ installed: true, authorizationRequired: false, enabled: false, lastError: null });
    mocks.getCaptureInput.mockResolvedValue({ settings: { enabled: false, keepSelected: false, endpointId: null, endpointName: null }, phase: "disabled", recoveryPending: false, lastError: null });
    mocks.listCaptureInputs.mockResolvedValue([{ id: "capture", name: "CABLE Output", isVirtualCableCandidate: true }]);
    mocks.setCaptureInput.mockImplementation(async settings => ({ settings, phase: "idle", recoveryPending: false, lastError: null }));
    mocks.resolveCaptureRecovery.mockResolvedValue({ settings: { enabled: false, keepSelected: false, endpointId: null, endpointName: null }, phase: "idle", recoveryPending: false, lastError: null });
    mocks.captureEdgeHandler = null;
    mocks.getConnectionSnapshot.mockResolvedValue(emptyConnection);
    mocks.getAudioSnapshot.mockResolvedValue(emptyAudio);
    mocks.getAudioRouteSnapshot.mockResolvedValue({ phase: "unconfigured", reason: null, captureEndpointId: null, captureEndpointName: null, renderEndpointId: null, renderEndpointName: null });
    mocks.listAudioEndpoints.mockImplementation(async () => mocks.endpoints);
    mocks.selectAudioEndpoint.mockImplementation(async (endpointId: string) => ({
      ...emptyAudio,
      phase: "ready",
      selectedEndpointId: endpointId,
      selectedEndpointName: cableEndpoint.name,
    }));
    mocks.openVbCableDownloadPage.mockResolvedValue(undefined);
    mocks.getVoiceHoldHotkey.mockResolvedValue({
      keys: ["left_control", "left_windows"],
    });
    mocks.setVoiceHoldHotkey.mockImplementation(async (hotkey) => hotkey);
    mocks.getVoiceInputTool.mockResolvedValue("wechat");
    mocks.setVoiceInputTool.mockImplementation(async (tool) => tool);
    mocks.getVokieInstallation.mockResolvedValue({ installed: true, running: true });
    mocks.getOtherVoiceHotkey.mockResolvedValue(null);
    mocks.setOtherVoiceHotkey.mockImplementation(async (keys: string[]) => keys);
    mocks.openVokieHomepage.mockResolvedValue(undefined);
    mocks.launchVokie.mockResolvedValue(undefined);
    mocks.startShortcutCapture.mockResolvedValue([]);
    mocks.stopShortcutCapture.mockResolvedValue(undefined);
    mocks.subscribeShortcutCaptureEdges.mockImplementation(
      async (handler: ShortcutCaptureHandler) => {
        mocks.captureEdgeHandler = handler;
        return () => {};
      },
    );
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it("exposes separate upstream output and optional Windows microphone controls", async () => {
    mocks.endpoints = [cableEndpoint];
    const wrapper = mount(ConnectionPage, { props: { runtime } }); await flushPromises();
    expect(wrapper.get('.audio-output-settings').text()).toContain('声音写入设备');
    expect(wrapper.get('.windows-default-settings > summary').text()).toContain('Windows 默认麦克风');
    expect(wrapper.find('#audio-output-target').exists()).toBe(true);
    expect(wrapper.find('.audio-advanced #audio-output-target').exists()).toBe(false);
    expect(wrapper.find('.voice-device-columns').exists()).toBe(false);
    expect(wrapper.find('.voice-device-section > .gain-row').exists()).toBe(true);
    wrapper.unmount();
  });

  it("selects the upstream output without configuring the Windows microphone", async () => {
    mocks.endpoints = [cableEndpoint];
    const wrapper = mount(ConnectionPage, { props: { runtime } }); await flushPromises();
    await wrapper.get('#audio-output-target').setValue(cableEndpoint.id); await flushPromises();
    expect(mocks.selectAudioEndpoint).toHaveBeenCalledExactlyOnceWith(cableEndpoint.id);
    expect(mocks.setCaptureInput).not.toHaveBeenCalled();
    expect((wrapper.get('#audio-output-target').element as HTMLSelectElement).value).toBe(cableEndpoint.id);
    wrapper.unmount();
  });

  it("keeps the confirmed output selected after a failed output save", async () => {
    mocks.endpoints = [cableEndpoint, { id: 'second-output', name: 'Second cable input', isVirtualCableCandidate: true }];
    mocks.getAudioSnapshot.mockResolvedValue({ ...emptyAudio, phase: 'ready', selectedEndpointId: cableEndpoint.id, selectedEndpointName: cableEndpoint.name });
    mocks.selectAudioEndpoint.mockRejectedValueOnce(new Error('output unavailable'));
    const wrapper = mount(ConnectionPage, { props: { runtime } }); await flushPromises();
    await wrapper.get('#audio-output-target').setValue('second-output'); await flushPromises();
    expect((wrapper.get('#audio-output-target').element as HTMLSelectElement).value).toBe(cableEndpoint.id);
    expect(wrapper.get('.audio-output-feedback').text()).toContain('操作没有完成');
    expect(mocks.setCaptureInput).not.toHaveBeenCalled();
    wrapper.unmount();
  });

  it('keeps an output selection failure visible alongside an unavailable route', async () => {
    mocks.endpoints = [cableEndpoint, {id: 'second-output', name: 'Second cable input', isVirtualCableCandidate: true}];
    mocks.getAudioSnapshot.mockResolvedValue({...emptyAudio, phase: 'ready', selectedEndpointId: cableEndpoint.id, selectedEndpointName: cableEndpoint.name});
    mocks.getAudioRouteSnapshot.mockResolvedValue({phase: 'unavailable', reason: 'render_missing', captureEndpointId: null, captureEndpointName: null, renderEndpointId: cableEndpoint.id, renderEndpointName: cableEndpoint.name});
    mocks.selectAudioEndpoint.mockRejectedValueOnce(new Error('output selection failed'));
    const wrapper = mount(ConnectionPage, {props: {runtime}}); await flushPromises();
    await wrapper.get('#audio-output-target').setValue('second-output'); await flushPromises();
    expect(wrapper.get('.audio-output-feedback').text()).toContain('操作没有完成');
    await wrapper.get('.capture-feedback-details').trigger('click'); await flushPromises();
    expect(wrapper.get('.audio-advanced').text()).toContain('操作没有完成');
    expect(wrapper.find('.audio-advanced [data-audio-route-error]').exists()).toBe(true);
    wrapper.unmount();
  });

  it('does not erase an audio snapshot failure when device enumeration succeeds', async () => {
    mocks.getAudioSnapshot.mockRejectedValue(new Error('snapshot failed'));
    const wrapper = mount(ConnectionPage, {props: {runtime}}); await flushPromises();
    expect(mocks.listAudioEndpoints).toHaveBeenCalled();
    expect(wrapper.get('.audio-output-feedback').text()).toContain('操作没有完成');
    wrapper.unmount();
  });

  it("guides direct routing without requiring an optional microphone", async () => {
    const wrapper = mount(ConnectionPage, { props: { runtime } }); await flushPromises();
    (wrapper.get('.audio-advanced').element as HTMLDetailsElement).open = true;
    await wrapper.get('.audio-advanced').trigger('toggle'); await flushPromises();
    expect(wrapper.get('#audio-output-target').text()).toContain('请选择声音写入设备');
    expect(mocks.setCaptureInput).not.toHaveBeenCalled();
    wrapper.unmount();
  });

  it("reports a direct route read failure without referring to an optional microphone", async () => {
    mocks.getAudioRouteSnapshot.mockRejectedValue(new Error('snapshot unavailable'));
    const wrapper = mount(ConnectionPage, { props: { runtime } }); await flushPromises();
    expect(wrapper.get('.audio-output-feedback').text()).toContain('暂时无法确认声音写入设备状态');
    wrapper.unmount();
  });

  it('keeps routine copy compact and puts actual Windows roles in diagnostics', async () => {
    const wrapper = mount(ConnectionPage, {props: {runtime}}); await flushPromises();
    expect(wrapper.find('.audio-output-help').exists()).toBe(false);
    expect(wrapper.find('.capture-input-settings .windows-default-inputs').exists()).toBe(false);
    expect(wrapper.get('.capture-target-heading .capture-target-help').text()).toBe('立即生效，自动切换设置不变');
    expect(wrapper.get('.capture-feedback-text').text()).toBe('');
    await wrapper.get('.capture-feedback-details').trigger('click'); await flushPromises();
    const advanced = wrapper.get('.audio-advanced');
    expect(advanced.get('.windows-default-inputs').text()).toContain('PC microphone');
    expect(advanced.text()).not.toContain('手动写入端只用于诊断或已有特殊配置');
    expect(advanced.text()).not.toContain('当前写入端');
    expect(mocks.setDefaultCaptureInput).not.toHaveBeenCalled();
    expect(mocks.setCaptureInput).not.toHaveBeenCalled();
    wrapper.unmount();
  });

  it('shows actual Windows defaults separately from the saved target', async () => {
    mocks.getCaptureInput.mockResolvedValue({settings: {enabled: false, keepSelected: false, endpointId: 'capture', endpointName: 'CABLE Output'}, phase: 'disabled', recoveryPending: false, lastError: null});
    const wrapper = mount(ConnectionPage, {props: {runtime}}); await flushPromises();
    await wrapper.get('.capture-feedback-details').trigger('click'); await flushPromises();
    expect(wrapper.get('.windows-default-inputs').text()).toContain('PC microphone');
    expect(wrapper.get('.windows-default-inputs').text()).toContain('未设置');
    expect(wrapper.get<HTMLSelectElement>('#capture-input-target').element.value).toBe('capture');
    wrapper.unmount();
  });

  it('changes Windows defaults only on explicit request without changing automatic routing', async () => {
    const wrapper = mount(ConnectionPage, {props: {runtime}}); await flushPromises();
    await wrapper.get('#capture-input-target').setValue('capture'); await flushPromises();
    expect(mocks.setDefaultCaptureInput).not.toHaveBeenCalled();
    expect(mocks.setCaptureInput).not.toHaveBeenCalled();
    await wrapper.get('#set-windows-default-input').trigger('click'); await flushPromises();
    expect(mocks.setDefaultCaptureInput).toHaveBeenCalledExactlyOnceWith('capture', 'CABLE Output');
    expect(mocks.setCaptureInput).not.toHaveBeenCalled();
    expect(mocks.selectAudioEndpoint).not.toHaveBeenCalled();
    await wrapper.get('.capture-feedback-details').trigger('click'); await flushPromises();
    expect(wrapper.get('.windows-default-inputs').text()).toContain('CABLE Output');
    expect(wrapper.get<HTMLInputElement>('#capture-input-switch').element.checked).toBe(false);
    wrapper.unmount();
  });

  it('keeps manual default errors visible without pretending the target is current', async () => {
    mocks.setDefaultCaptureInput.mockRejectedValueOnce(new Error('write failed'));
    const wrapper = mount(ConnectionPage, {props: {runtime}}); await flushPromises();
    await wrapper.get('#capture-input-target').setValue('capture'); await flushPromises();
    await wrapper.get('#set-windows-default-input').trigger('click'); await flushPromises();
    expect(wrapper.get('.capture-input-feedback').text()).toContain('操作没有完成');
    await wrapper.get('.capture-feedback-details').trigger('click'); await flushPromises();
    expect(wrapper.get('.windows-default-inputs').text()).toContain('PC microphone');
    expect(mocks.setCaptureInput).not.toHaveBeenCalled();
    wrapper.unmount();
  });

  it('ignores an old default read that finishes after an explicit change', async () => {
    let finishRead!: (value: unknown) => void;
    mocks.getDefaultCaptureInputs.mockImplementationOnce(() => new Promise(resolve => { finishRead = resolve; }));
    const wrapper = mount(ConnectionPage, {props: {runtime}}); await flushPromises();
    await wrapper.get('#capture-input-target').setValue('capture'); await flushPromises();
    await wrapper.get('#set-windows-default-input').trigger('click'); await flushPromises();
    finishRead({console: {id: 'old', name: 'Stale microphone'}, multimedia: null, communications: null}); await flushPromises();
    await wrapper.get('.capture-feedback-details').trigger('click'); await flushPromises();
    expect(wrapper.get('.windows-default-inputs').text()).not.toContain('Stale microphone');
    expect(wrapper.get('.windows-default-inputs').text()).toContain('CABLE Output');
    wrapper.unmount();
  });

  it('shows a default read failure instead of displaying the saved target as current', async () => {
    mocks.getDefaultCaptureInputs.mockRejectedValueOnce(new Error('read failed'));
    const wrapper = mount(ConnectionPage, {props: {runtime}}); await flushPromises();
    await wrapper.get('.capture-feedback-details').trigger('click'); await flushPromises();
    expect(wrapper.get('.windows-default-inputs').text()).toContain('读取失败');
    expect(wrapper.get('.windows-default-inputs').text()).not.toContain('CABLE Output');
    wrapper.unmount();
  });

  it('does not allow manual changes during voice or pending recovery', async () => {
    mocks.getCaptureInput.mockResolvedValue({settings: {enabled: false, keepSelected: false, endpointId: 'capture', endpointName: 'CABLE Output'}, phase: 'recovery_required', recoveryPending: true, lastError: null});
    const wrapper = mount(ConnectionPage, {props: {runtime}}); await flushPromises();
    expect(wrapper.get<HTMLButtonElement>('#set-windows-default-input').element.disabled).toBe(true);
    expect(mocks.setDefaultCaptureInput).not.toHaveBeenCalled();
    wrapper.unmount();
  });

  it('disables manual default changes while the remote voice session is active', async () => {
    const wrapper = mount(ConnectionPage, {props: {runtime}}); await flushPromises();
    await wrapper.get('#capture-input-target').setValue('capture'); await flushPromises();
    await wrapper.setProps({runtime: {...runtime, platform: {...runtime.platform, connection: {...emptyConnection, voiceState: 'streaming'}}}});
    expect(wrapper.get<HTMLButtonElement>('#set-windows-default-input').element.disabled).toBe(true);
    expect(mocks.setDefaultCaptureInput).not.toHaveBeenCalled();
    wrapper.unmount();
  });

  it('collapses optional Windows microphone settings without changing the saved policy', async () => {
    mocks.getCaptureInput.mockResolvedValue({settings: {enabled: true, keepSelected: false, endpointId: 'capture', endpointName: 'CABLE Output'}, phase: 'idle', recoveryPending: false, lastError: null});
    const wrapper = mount(ConnectionPage, {props: {runtime}}); await flushPromises();
    const disclosure = wrapper.get<HTMLDetailsElement>('details.windows-default-settings');
    expect(disclosure.element.open).toBe(false);
    expect(disclosure.get('summary').text()).toContain('Windows 默认麦克风');
    for (const open of [true, false, true]) {
      disclosure.element.open = open;
      await disclosure.trigger('toggle'); await flushPromises();
      expect(wrapper.get<HTMLInputElement>('#capture-input-switch').element.checked).toBe(true);
      expect(wrapper.get<HTMLInputElement>('#capture-input-restore').element.checked).toBe(true);
    }
    expect(mocks.setCaptureInput).not.toHaveBeenCalled();
    expect(mocks.setDefaultCaptureInput).not.toHaveBeenCalled();
    wrapper.unmount();
  });

  it('keeps a recovery warning visible on the collapsed microphone summary', async () => {
    mocks.getCaptureInput.mockResolvedValue({settings: {enabled: true, keepSelected: false, endpointId: 'capture', endpointName: 'CABLE Output'}, phase: 'recovery_required', recoveryPending: true, lastError: null});
    const wrapper = mount(ConnectionPage, {props: {runtime}}); await flushPromises();
    expect(wrapper.get('details.windows-default-settings > summary').text()).toContain('上次会话未完整恢复');
    await wrapper.get('.capture-feedback-details').trigger('click'); await flushPromises();
    expect(wrapper.get('.capture-input-diagnostics').text()).toContain('保留当前选择');
    wrapper.unmount();
  });

  it('shows a default-device read failure even while the optional settings are collapsed', async () => {
    mocks.getDefaultCaptureInputs.mockRejectedValueOnce(new Error('read failed'));
    const wrapper = mount(ConnectionPage, {props: {runtime}}); await flushPromises();
    expect(wrapper.get('.windows-default-settings > summary').text()).toContain('读取失败');
    wrapper.unmount();
  });

  it.each([false, true])('marks the entire restore row disabled only while automatic switching is off: %s', async enabled => {
    mocks.getCaptureInput.mockResolvedValue({settings: {enabled, keepSelected: true, endpointId: 'capture', endpointName: 'CABLE Output'}, phase: enabled ? 'idle' : 'disabled', recoveryPending: false, lastError: null});
    const wrapper = mount(ConnectionPage, {props: {runtime}}); await flushPromises();
    expect(wrapper.get("label[for='capture-input-switch']").text()).toBe('语音开始时切换麦克风');
    const restore = wrapper.get("label[for='capture-input-restore']");
    expect(restore.text()).toBe('语音结束时恢复麦克风');
    expect(restore.attributes('aria-disabled')).toBe(String(!enabled));
    expect(restore.get<HTMLInputElement>('input').element.disabled).toBe(!enabled);
    wrapper.unmount();
  });

  it('keeps feedback beside the settings and reveals complete recovery actions through diagnostics', async () => {
    mocks.getCaptureInput.mockResolvedValue({settings: {enabled: true, keepSelected: false, endpointId: 'capture', endpointName: 'CABLE Output'}, phase: 'recovery_required', recoveryPending: true, lastError: 'normal_roles_split'});
    const wrapper = mount(ConnectionPage, {props: {runtime}}); await flushPromises();
    expect(wrapper.find('.capture-input-behavior .capture-input-feedback').exists()).toBe(true);
    expect(wrapper.find('.capture-input-settings > .capture-input-feedback').exists()).toBe(false);
    expect(wrapper.get('.capture-feedback-text').text()).toContain('上次会话未完整恢复');
    expect(wrapper.find('.capture-input-diagnostics').exists()).toBe(false);
    await wrapper.get('.capture-feedback-details').trigger('click'); await flushPromises();
    expect(wrapper.get<HTMLDetailsElement>('.audio-advanced').element.open).toBe(true);
    expect(wrapper.get('.capture-input-diagnostics').text()).toContain('普通输入的两个系统角色');
    const keep = wrapper.findAll('.capture-input-diagnostics button').find(button => button.text() === '保留当前选择')!;
    await keep.trigger('click'); await flushPromises();
    expect(mocks.resolveCaptureRecovery).toHaveBeenCalledWith(false);
    expect(mocks.setDefaultCaptureInput).not.toHaveBeenCalled();
    wrapper.unmount();
  });

  it("groups each status dot with its heading for vertical alignment", async () => {
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();

    const headings = wrapper.findAll(".status-heading");
    expect(headings).toHaveLength(2);
    for (const heading of headings) {
      expect(heading.find(".status-dot").exists()).toBe(true);
      expect(heading.find("strong").exists()).toBe(true);
    }
    wrapper.unmount();
  });

  it("keeps voice devices inside the remote connection card and input-tool setup separate", async () => {
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();
    const connectionCard = wrapper.findAll("article.card").find(card => card.find("h2").text() === "遥控器连接")!;
    expect(connectionCard.find('section[aria-labelledby="voice-devices-title"]').exists()).toBe(true);
    expect(connectionCard.get('#voice-devices-title').text()).toBe("声音传输与麦克风");
    expect(connectionCard.find('#capture-input-target').exists()).toBe(true);
    expect(connectionCard.get('details.audio-advanced').attributes('open')).toBeUndefined();
    expect(connectionCard.find('.setup-card').exists()).toBe(false);
    expect(wrapper.get('.setup-card h2').text()).toBe("语音输入设置");
    expect(mocks.setCaptureInput).not.toHaveBeenCalled();
    expect(wrapper.get(".capture-feedback-text").text()).toBe('');
    expect(mocks.selectAudioEndpoint).not.toHaveBeenCalled();
    wrapper.unmount();
  });

  it("does not guess a render endpoint from the only listed cable when no microphone was configured", async () => {
    mocks.endpoints = [cableEndpoint];
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();

    expect(mocks.selectAudioEndpoint).not.toHaveBeenCalled();
    expect((wrapper.get('#audio-output-target').element as HTMLSelectElement).value).toBe('');
    expect(wrapper.get('details.audio-advanced').attributes('open')).toBeUndefined();
    expect(wrapper.text()).toContain("请选择声音写入设备");
    expect(wrapper.text()).not.toContain("需要安装 VB-CABLE");
    expect(wrapper.text()).not.toContain("系统语音输入");
    wrapper.unmount();
  });

  it("waits for the saved endpoint and does not replace an existing selection", async () => {
    const savedAudio: AudioSnapshot = {
      ...emptyAudio,
      phase: "ready",
      selectedEndpointId: "saved-speaker",
      selectedEndpointName: "已保存的扬声器",
    };
    let resolveAudio: ((snapshot: AudioSnapshot) => void) | undefined;
    mocks.endpoints = [cableEndpoint];
    mocks.getAudioSnapshot.mockImplementationOnce(
      () =>
        new Promise<AudioSnapshot>((resolve) => {
          resolveAudio = resolve;
        }),
    );

    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();
    expect(mocks.listAudioEndpoints).not.toHaveBeenCalled();

    resolveAudio?.(savedAudio);
    await flushPromises();

    expect(mocks.listAudioEndpoints).toHaveBeenCalledOnce();
    expect(mocks.selectAudioEndpoint).not.toHaveBeenCalled();
    wrapper.unmount();
  });

  it("shows the official installation action when VB-CABLE is unavailable", async () => {
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();

    expect(mocks.selectAudioEndpoint).not.toHaveBeenCalled();
    expect(wrapper.text()).toContain("需要安装 VB-CABLE");
    expect(wrapper.text()).toContain("完成后需重启电脑");

    await wrapper.get(".vb-cable-callout .primary-button").trigger("click");
    await flushPromises();
    expect(mocks.openVbCableDownloadPage).toHaveBeenCalledOnce();
    wrapper.unmount();
  });
  it("keeps capture locking disabled and requires a separate capture selection", async () => {
    const wrapper = mount(ConnectionPage, { props: { runtime } }); await flushPromises();
    expect(mocks.setCaptureInput).not.toHaveBeenCalled();
    expect(wrapper.get('.capture-input-settings input[type="checkbox"]').attributes("disabled")).toBeDefined();
    await wrapper.get('.voice-device-section > .card-title-row button').trigger('click'); await flushPromises();
    await wrapper.get('#capture-input-target').setValue('capture'); await flushPromises();
    expect(mocks.setCaptureInput).not.toHaveBeenCalled();
    await wrapper.get('.capture-input-settings input[type="checkbox"]').setValue(true); await flushPromises();
    expect(mocks.setCaptureInput).toHaveBeenLastCalledWith({ enabled: true, keepSelected: true, endpointId: 'capture', endpointName: 'CABLE Output' });
    wrapper.unmount();
  });

  it.each([
    { enabled: false, keepSelected: false, restore: false },
    { enabled: true, keepSelected: true, restore: false },
    { enabled: true, keepSelected: false, restore: true },
  ])("renders the saved microphone mode $enabled/$keepSelected", async ({ enabled, keepSelected, restore }) => {
    mocks.getCaptureInput.mockResolvedValue({ settings: { enabled, keepSelected, endpointId: "capture", endpointName: "CABLE Output" }, phase: enabled ? "idle" : "disabled", recoveryPending: false, lastError: null });
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();
    expect(wrapper.get<HTMLInputElement>("#capture-input-switch").element.checked).toBe(enabled);
    expect(wrapper.get<HTMLInputElement>("#capture-input-restore").element.checked).toBe(restore);
    expect(wrapper.get<HTMLInputElement>("#capture-input-restore").element.disabled).toBe(!enabled);
    expect(wrapper.get(".capture-input-target").text()).not.toContain("已关闭");
    expect(wrapper.get('.capture-feedback-text').text()).toBe('');
    wrapper.unmount();
  });

  it("requires restoring to be selected explicitly and clears it when switching is disabled", async () => {
    mocks.getCaptureInput.mockResolvedValue({ settings: { enabled: false, keepSelected: false, endpointId: "capture", endpointName: "CABLE Output" }, phase: "disabled", recoveryPending: false, lastError: null });
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();
    await wrapper.get("#capture-input-switch").setValue(true);
    await flushPromises();
    expect(mocks.setCaptureInput).toHaveBeenLastCalledWith({ enabled: true, keepSelected: true, endpointId: "capture", endpointName: "CABLE Output" });
    expect(wrapper.get<HTMLInputElement>("#capture-input-restore").element.checked).toBe(false);
    await wrapper.get("#capture-input-restore").setValue(true);
    await flushPromises();
    expect(mocks.setCaptureInput).toHaveBeenLastCalledWith({ enabled: true, keepSelected: false, endpointId: "capture", endpointName: "CABLE Output" });
    await wrapper.get("#capture-input-switch").setValue(false);
    await flushPromises();
    expect(mocks.setCaptureInput).toHaveBeenLastCalledWith({ enabled: false, keepSelected: false, endpointId: "capture", endpointName: "CABLE Output" });
    expect(wrapper.get<HTMLInputElement>("#capture-input-restore").element.checked).toBe(false);
    expect(wrapper.get<HTMLInputElement>("#capture-input-restore").element.disabled).toBe(true);
    await wrapper.get("#capture-input-switch").setValue(true);
    await flushPromises();
    expect(mocks.setCaptureInput).toHaveBeenLastCalledWith({ enabled: true, keepSelected: true, endpointId: "capture", endpointName: "CABLE Output" });
    wrapper.unmount();
  });

  it.each([true, false])("preserves keepSelected=%s when the target changes", async keepSelected => {
    mocks.getCaptureInput.mockResolvedValue({ settings: { enabled: true, keepSelected, endpointId: "old", endpointName: "Old microphone" }, phase: "idle", recoveryPending: false, lastError: null });
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();
    await wrapper.get("#capture-input-target").setValue("capture");
    await flushPromises();
    expect(mocks.setCaptureInput).toHaveBeenLastCalledWith({ enabled: true, keepSelected, endpointId: "capture", endpointName: "CABLE Output" });
    wrapper.unmount();
  });

  it("serializes mode changes and reads the confirmed mode after a save failure", async () => {
    const previous = { settings: { enabled: true, keepSelected: true, endpointId: "capture", endpointName: "CABLE Output" }, phase: "idle", recoveryPending: false, lastError: null };
    mocks.getCaptureInput.mockResolvedValue(previous);
    let rejectSave!: (reason: Error) => void;
    mocks.setCaptureInput.mockImplementationOnce(() => new Promise((resolve, reject) => { rejectSave = reject; }));
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();
    await wrapper.get("#capture-input-restore").setValue(true);
    expect(wrapper.get<HTMLInputElement>("#capture-input-switch").element.disabled).toBe(true);
    expect(wrapper.get<HTMLInputElement>("#capture-input-restore").element.disabled).toBe(true);
    expect(wrapper.get<HTMLSelectElement>("#capture-input-target").element.disabled).toBe(true);
    rejectSave(new Error("save_failed"));
    await flushPromises();
    expect(mocks.getCaptureInput).toHaveBeenCalledTimes(2);
    expect(wrapper.get<HTMLInputElement>("#capture-input-switch").element.checked).toBe(true);
    expect(wrapper.get<HTMLInputElement>("#capture-input-restore").element.checked).toBe(false);
    expect(wrapper.get(".capture-input-feedback").text()).toContain("save_failed");
    expect(wrapper.text()).not.toContain("尚未保存");
    expect(wrapper.get<HTMLDetailsElement>("details.audio-advanced").element.open).toBe(false);
    expect(wrapper.get<HTMLInputElement>("#capture-input-restore").element.disabled).toBe(false);
    wrapper.unmount();
  });

  it("keeps an unsaved microphone choice for explicit same-line writer recovery", async () => {
    const previous = { settings: { enabled: true, keepSelected: false, endpointId: 'missing', endpointName: 'Old cable' }, phase: 'failed', recoveryPending: false, lastError: null };
    mocks.getCaptureInput.mockResolvedValue(previous);
    mocks.endpoints = [cableEndpoint];
    mocks.setCaptureInput.mockRejectedValueOnce(new Error('未找到同一条虚拟音频线的声音写入端。'));
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();
    await wrapper.get('#capture-input-target').setValue('capture');
    await flushPromises();
    expect(wrapper.text()).toContain('尚未保存');
    expect((wrapper.get('#capture-input-target').element as HTMLSelectElement).value).toBe('capture');
    await wrapper.get('details.audio-advanced').trigger('toggle');
    await flushPromises();
    await wrapper.findAll('.audio-advanced button').find(button => button.text().includes('选择'))!.trigger('click');
    await flushPromises();
    expect(mocks.setCaptureInput).toHaveBeenLastCalledWith({ enabled: true, keepSelected: false, endpointId: 'capture', endpointName: 'CABLE Output' }, cableEndpoint.id);
    expect(mocks.selectAudioEndpoint).not.toHaveBeenCalled();
    wrapper.unmount();
  });

  it("retains the draft after transaction failure and serializes explicit recovery", async () => {
    const previous = { settings: { enabled: true, keepSelected: false, endpointId: 'missing', endpointName: 'Old cable' }, phase: 'failed', recoveryPending: false, lastError: null };
    mocks.getCaptureInput.mockResolvedValue(previous);
    mocks.endpoints = [cableEndpoint];
    mocks.setCaptureInput.mockRejectedValueOnce(new Error('render_missing'));
    mocks.setCaptureInput.mockRejectedValueOnce(new Error('disk_failed'));
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();
    await wrapper.get('#capture-input-target').setValue('capture');
    await flushPromises();
    const choose = wrapper.findAll('.audio-advanced button').find(button => button.text().includes('选择'))!;
    await choose.trigger('click');
    await flushPromises();
    expect(wrapper.text()).toContain('尚未保存');
    expect(wrapper.text()).toContain('操作没有完成，请稍后重试。');
    expect(wrapper.text()).not.toContain('disk_failed');
    expect((wrapper.get('.capture-input-settings input[type="checkbox"]').element as HTMLInputElement).checked).toBe(true);
    let finish!: (value: typeof previous) => void;
    mocks.setCaptureInput.mockImplementationOnce(() => new Promise(resolve => { finish = resolve; }));
    await choose.trigger('click');
    await choose.trigger('click');
    expect(mocks.setCaptureInput).toHaveBeenCalledTimes(3);
    finish({ ...previous, settings: { enabled: true, keepSelected: false, endpointId: 'capture', endpointName: 'CABLE Output' }, phase: 'idle' });
    await flushPromises();
    expect(wrapper.text()).not.toContain('尚未保存');
    wrapper.unmount();
  });
  it("does not silently restore a crash journal", async () => {
    mocks.getCaptureInput.mockResolvedValue({ settings: { enabled: true, keepSelected: false, endpointId: 'capture', endpointName: 'CABLE Output' }, phase: 'recovery_required', recoveryPending: true, lastError: null });
    const wrapper = mount(ConnectionPage, { props: { runtime } }); await flushPromises();
    expect(mocks.resolveCaptureRecovery).not.toHaveBeenCalled();
    await wrapper.get('.capture-feedback-details').trigger('click'); await flushPromises();
    const keep = wrapper.findAll('.capture-input-diagnostics button').find(b => b.text() === '保留当前选择')!;
    await keep.trigger('click'); await flushPromises(); expect(mocks.resolveCaptureRecovery).toHaveBeenCalledWith(false);
    wrapper.unmount();
  });

  it("disables device changes without inserting a feedback row during voice activity", async () => {
    mocks.getCaptureInput.mockResolvedValue({ settings: { enabled: true, keepSelected: false, endpointId: "capture", endpointName: "CABLE Output" }, phase: "active", recoveryPending: false, lastError: null });
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();
    try {
      const section = wrapper.get(".capture-input-settings");
      const originalChildren = section.element.children.length;
      for (const phase of ["streaming", "draining", "ready", "streaming", "ready"] as const) {
        await wrapper.setProps({ runtime: { ...runtime, platform: { ...runtime.platform, audio: { ...emptyAudio, phase } } } });
        expect(section.get(".capture-feedback-text").text()).toBe("本次说话正在使用所选输入设备");
        expect(section.element.children).toHaveLength(originalChildren);
        expect(section.text()).not.toContain("请松开语音键");
        expect(section.get('.capture-input-feedback[aria-live="polite"] .capture-feedback-text').text()).toBe("本次说话正在使用所选输入设备");
        const busy = phase !== "ready";
        expect((section.get("select").element as HTMLSelectElement).disabled).toBe(busy);
        expect((section.get('input[type="checkbox"]').element as HTMLInputElement).disabled).toBe(busy);
      }
      expect(mocks.setCaptureInput).not.toHaveBeenCalled();
      expect(mocks.selectAudioEndpoint).not.toHaveBeenCalled();
    } finally {
      wrapper.unmount();
    }
  });

  it("keeps capture errors and recovery actions visible during voice activity", async () => {
    mocks.getCaptureInput.mockResolvedValue({ settings: { enabled: true, keepSelected: false, endpointId: "capture", endpointName: "CABLE Output" }, phase: "recovery_required", recoveryPending: true, lastError: "normal_roles_split" });
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();
    await wrapper.setProps({ runtime: { ...runtime, platform: { ...runtime.platform, audio: { ...emptyAudio, phase: "streaming" } } } });
    expect(wrapper.get('.capture-input-feedback').text()).toContain('上次会话未完整恢复');
    await wrapper.get('.capture-feedback-details').trigger('click'); await flushPromises();
    const feedback = wrapper.get(".capture-input-diagnostics");
    expect(feedback.text()).toContain("普通输入的两个系统角色指向不同设备");
    expect(feedback.text()).toContain("保留当前选择");
    expect(feedback.text()).toContain("尝试恢复会话前设备");
    expect(feedback.text()).not.toContain("请松开语音键");
    expect(mocks.resolveCaptureRecovery).not.toHaveBeenCalled();
    wrapper.unmount();
  });


  it("exposes upstream output choices while keeping detailed diagnostics collapsed", async () => {
    const cableA: AudioEndpoint = {
      id: "cable-a",
      name: "CABLE-A Input (VB-Audio Cable A)",
      isVirtualCableCandidate: true,
    };
    const speaker: AudioEndpoint = {
      id: "speaker",
      name: "扬声器 (Realtek Audio)",
      isVirtualCableCandidate: false,
    };
    mocks.endpoints = [cableA, cableEndpoint, speaker];
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();

    expect(mocks.selectAudioEndpoint).not.toHaveBeenCalled();
    expect(wrapper.find('.endpoint-list').exists()).toBe(false);
    expect(wrapper.get('#audio-output-target').text()).toContain("扬声器 (Realtek Audio)");
    const advanced = wrapper.get('details.audio-advanced');
    (advanced.element as HTMLDetailsElement).open = true;
    await advanced.trigger('toggle');
    await flushPromises();

    const items = wrapper.findAll(".endpoint-list li");
    expect(items).toHaveLength(3);
    expect(items[0].text()).toContain("CABLE-A Input");
    expect(items[1].text()).toContain("CABLE Input (VB-Audio Virtual Cable)");
    expect(items[2].text()).toContain("扬声器");
    expect(items[0].text()).not.toContain("推荐");
    expect(items[2].text()).not.toContain("推荐");
    expect(items[2].text()).toContain("仅用于手动诊断");
    wrapper.unmount();
  });

  it("retries the saved audio route without changing the microphone switch or target", async () => {
    const settings = { enabled: true, keepSelected: false, endpointId: "capture", endpointName: "CABLE Output" };
    mocks.getCaptureInput.mockResolvedValue({ settings, phase: "disabled", recoveryPending: false, lastError: null });
    mocks.getAudioRouteSnapshot.mockResolvedValue({ phase: "unavailable", reason: "audio_unready", captureEndpointId: "capture", captureEndpointName: "CABLE Output", renderEndpointId: null, renderEndpointName: null });
    const wrapper = mount(ConnectionPage, { props: { runtime } }); await flushPromises();
    expect(mocks.setCaptureInput).not.toHaveBeenCalled();
    await wrapper.get('.capture-feedback-details').trigger('click'); await flushPromises();
    const retry = wrapper.findAll('button').find(button => button.text() === "重新连接声音通道");
    expect(retry).toBeDefined();
    await retry!.trigger('click'); await flushPromises();
    expect(mocks.setCaptureInput).toHaveBeenCalledExactlyOnceWith(settings);
    expect((wrapper.get('.capture-input-settings input[type="checkbox"]').element as HTMLInputElement).checked).toBe(true);
    wrapper.unmount();
  });

  it("does not offer inactive microphone re-pairing for a failed direct output", async () => {
    mocks.getCaptureInput.mockResolvedValue({ settings: { enabled: false, keepSelected: false, endpointId: 'old-capture', endpointName: 'Old microphone' }, phase: 'disabled', recoveryPending: false, lastError: null });
    mocks.getAudioRouteSnapshot.mockResolvedValue({ phase: 'unavailable', reason: 'audio_unready', captureEndpointId: 'old-capture', captureEndpointName: 'Old microphone', renderEndpointId: 'chosen-writer', renderEndpointName: 'Chosen writer' });
    const wrapper = mount(ConnectionPage, { props: { runtime } }); await flushPromises();
    expect(wrapper.findAll('button').some(button => button.text() === '重新连接声音通道')).toBe(false);
    expect(mocks.setCaptureInput).not.toHaveBeenCalled();
    wrapper.unmount();
  });

  it("can retry the same failed writer without pairing the inactive microphone", async () => {
    mocks.endpoints = [cableEndpoint];
    mocks.getAudioSnapshot.mockResolvedValue({ ...emptyAudio, phase: 'failed', selectedEndpointId: cableEndpoint.id, selectedEndpointName: cableEndpoint.name });
    mocks.getAudioRouteSnapshot.mockResolvedValue({ phase: 'unavailable', reason: 'audio_unready', captureEndpointId: null, captureEndpointName: null, renderEndpointId: cableEndpoint.id, renderEndpointName: cableEndpoint.name });
    const wrapper = mount(ConnectionPage, { props: { runtime } }); await flushPromises();
    (wrapper.get('.audio-advanced').element as HTMLDetailsElement).open = true;
    await wrapper.get('.audio-advanced').trigger('toggle'); await flushPromises();
    const retry = wrapper.get('.endpoint-list button');
    expect(retry.attributes('disabled')).toBeUndefined();
    await retry.trigger('click'); await flushPromises();
    expect(mocks.selectAudioEndpoint).toHaveBeenCalledExactlyOnceWith(cableEndpoint.id);
    expect(mocks.setCaptureInput).not.toHaveBeenCalled();
    wrapper.unmount();
  });

  it("logs route changes once without endpoint identities or polling duplicates", async () => {
    const wrapper = mount(ConnectionPage, { props: { runtime } }); await flushPromises();
    const refresh = wrapper.findAll('button').find(button => button.text() === "刷新设备列表")!;
    await refresh.trigger('click'); await flushPromises();
    const routeReports = () => vi.mocked(reportFrontendEvent).mock.calls.map(([event]) => event).filter(event => event.event === "audio_route_status");
    expect(routeReports()).toHaveLength(1);
    mocks.getAudioRouteSnapshot.mockResolvedValue({ phase: "unavailable", reason: "capture_missing", captureEndpointId: "private-endpoint", captureEndpointName: "private-name", renderEndpointId: null, renderEndpointName: null });
    await refresh.trigger('click'); await flushPromises();
    expect(routeReports()).toHaveLength(2);
    expect(routeReports()[1]).toMatchObject({ result: "failed", reason: "unavailable_capture_missing" });
    expect(JSON.stringify(routeReports())).not.toContain("private");
    wrapper.unmount();
  });

  it("does not report ready from a working render endpoint when pairing is ambiguous", async () => {
    mocks.endpoints = [cableEndpoint];
    mocks.getAudioSnapshot.mockResolvedValue({ ...emptyAudio, phase: "ready", selectedEndpointId: cableEndpoint.id, selectedEndpointName: cableEndpoint.name });
    mocks.getAudioRouteSnapshot.mockResolvedValue({ phase: "unavailable", reason: "ambiguous", captureEndpointId: "capture", captureEndpointName: "CABLE Output", renderEndpointId: null, renderEndpointName: null });
    const wrapper = mount(ConnectionPage, { props: { runtime } }); await flushPromises();
    expect(wrapper.text()).toContain("无法明确配对");
    expect(wrapper.text()).not.toContain("语音设备已就绪");
    expect(wrapper.text()).not.toContain("声音传输已就绪");
    expect(mocks.selectAudioEndpoint).not.toHaveBeenCalled();
    wrapper.unmount();
  });

  // 输入工具卡片（2026-09-30 设计稿 v3）：选工具即自动落该工具的快捷键。
  // 顺序固定为 豆包 > 微信 > 其他（Andy 要求豆包排第一）；选中态跟随持久化的
  // 工具选择，而不是卡片顺序。
  it("工具卡片顺序为豆包/微信/其他，选中态跟随已保存的工具选择", async () => {
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();

    const order = wrapper.findAll(".tool-card strong").map((node) => node.text());
    expect(order).toEqual(["豆包输入法", "微信输入法", "Vokie", "其他工具"]);
    expect(selectedToolCard(wrapper)).toBe("微信输入法");
    // 输入工具配置保持基础语音路径，不启动按键增强 Helper。
    expect(wrapper.find(".capture-switch").exists()).toBe(false);
    expect(mocks.enableRc003Capture).not.toHaveBeenCalled();
    wrapper.unmount();
  });

  it("未选择工具时只按当前快捷键展示引导，不在初始化时写入配置", async () => {
    mocks.getVoiceInputTool.mockResolvedValue(null);
    mocks.getVoiceHoldHotkey.mockResolvedValue({ keys: ["right_alt"] });
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();

    try {
      expect(selectedToolCard(wrapper)).toBe("豆包输入法");
      expect(mocks.setVoiceInputTool).not.toHaveBeenCalled();
      expect(mocks.setVoiceHoldHotkey).not.toHaveBeenCalled();
    } finally {
      wrapper.unmount();
    }
  });

  it("读取输入工具失败时提示读取失败，不把共享右 Alt 推断写回覆盖已存工具", async () => {
    mocks.getVoiceInputTool.mockRejectedValue(new Error("读取输入工具失败"));
    mocks.getVoiceHoldHotkey.mockResolvedValue({ keys: ["right_alt"] });
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();

    try {
      expect(mocks.setVoiceInputTool).not.toHaveBeenCalled();
      expect(mocks.setVoiceHoldHotkey).not.toHaveBeenCalled();
      expect(wrapper.find(".voice-hotkey-message").text()).toContain("读取输入工具失败");
    } finally {
      wrapper.unmount();
    }
  });

  it("点豆包卡片：自动把快捷键设为右 Alt、落存工具选择并显示该工具的清单", async () => {
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();

    await wrapper
      .findAll(".tool-card")
      .find((card) => card.text().includes("豆包输入法"))!
      .trigger("click");
    await flushPromises();

    expect(mocks.setVoiceInputTool).toHaveBeenCalledWith("doubao");
    expect(mocks.setVoiceHoldHotkey).toHaveBeenCalledWith({ keys: ["right_alt"] });
    expect(selectedToolCard(wrapper)).toBe("豆包输入法");
    expect(wrapper.text()).toContain("按住说话快捷键已设为 右 Alt");
    expect(wrapper.findAll(".checklist li").map((item) => item.text())).toEqual([
      "1豆包麦克风选择与上方相同的目标；使用系统默认时开启麦克风切换",
      "2豆包长按语音键选 右 Alt",
      "3切到豆包后，按住遥控器语音键说话",
    ]);
    expect(wrapper.find(".capture-switch").exists()).toBe(true);
    wrapper.unmount();
  });

  it("「其他工具」记住按键：切去豆包再切回来恢复上次选择（含「不按键」）", async () => {
    mocks.getVoiceInputTool.mockResolvedValue("other");
    mocks.getVoiceHoldHotkey.mockResolvedValue({ keys: ["left_alt"] });
    mocks.getOtherVoiceHotkey.mockResolvedValue(["left_alt"]);
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();

    const otherCard = wrapper
      .findAll(".tool-card")
      .find((card) => card.text().includes("其他工具"))!;

    // 切去豆包（会改写按住说话快捷键），再切回「其他工具」：按记忆恢复左 Alt。
    await wrapper
      .findAll(".tool-card")
      .find((card) => card.text().includes("豆包"))!
      .trigger("click");
    await flushPromises();
    expect(mocks.setVoiceHoldHotkey).toHaveBeenLastCalledWith({ keys: ["right_alt"] });

    mocks.getOtherVoiceHotkey.mockResolvedValue(["left_alt"]);
    await otherCard.trigger("click");
    await flushPromises();
    expect(mocks.setVoiceHoldHotkey).toHaveBeenLastCalledWith({ keys: ["left_alt"] });

    // 在「其他工具」面板里改选「不按键」：立即生效并记住（[] = 明确不按键）。
    mocks.getVoiceHoldHotkey.mockResolvedValue(null);
    const noneChip = wrapper
      .find(".setup-col:nth-child(2)")
      .findAll("button")
      .find((button) => button.text().includes("不按键"))!;
    await noneChip.trigger("click");
    await flushPromises();
    expect(mocks.setVoiceHoldHotkey).toHaveBeenLastCalledWith(null);
    expect(mocks.setOtherVoiceHotkey).toHaveBeenLastCalledWith([]);
    wrapper.unmount();
  });

  it("「其他工具」从未选过按键：切回来保持现状，不擅自改写", async () => {
    mocks.getVoiceInputTool.mockResolvedValue("other");
    mocks.getVoiceHoldHotkey.mockResolvedValue({ keys: ["left_alt"] });
    mocks.getOtherVoiceHotkey.mockResolvedValue(null);
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();

    await wrapper
      .findAll(".tool-card")
      .find((card) => card.text().includes("其他工具"))!
      .trigger("click");
    await flushPromises();

    expect(mocks.setVoiceHoldHotkey).not.toHaveBeenCalled();
    wrapper.unmount();
  });

  it("点 Vokie 卡片：自动设为右 Alt、落存工具选择并显示 Vokie 的准备清单", async () => {
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();

    await wrapper
      .findAll(".tool-card")
      .find((card) => card.text().includes("Vokie"))!
      .trigger("click");
    await flushPromises();

    expect(mocks.setVoiceInputTool).toHaveBeenCalledWith("vokie");
    expect(mocks.setVoiceHoldHotkey).toHaveBeenCalledWith({ keys: ["right_alt"] });
    expect(selectedToolCard(wrapper)).toBe("Vokie");
    expect(wrapper.findAll(".checklist li").map((item) => item.text())).toEqual([
      "1Vokie 麦克风选择与上方相同的目标；使用系统默认时开启麦克风切换",
      "2Vokie 快捷键保持默认的 右 Alt",
      "3在要写字的地方按住遥控器语音键说话",
    ]);
    // 已安装且在运行（默认 mock）：不显示官网入口，也不提示未运行。
    expect(wrapper.text()).not.toContain("没有检测到 Vokie");
    expect(wrapper.text()).not.toContain("Vokie 没有运行");
    // Vokie 面板不显示“支持更多输入工具”开关（2026-10-01 Andy 要求）。
    expect(wrapper.find(".setup-col:nth-child(2) .capture-switch").exists()).toBe(false);
    wrapper.unmount();
  });

  it("已安装但没运行：提示先启动 Vokie（没运行不会响应右 Alt）", async () => {
    mocks.getVokieInstallation.mockResolvedValue({ installed: true, running: false });
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();

    await wrapper
      .findAll(".tool-card")
      .find((card) => card.text().includes("Vokie"))!
      .trigger("click");
    await flushPromises();

    expect(wrapper.text()).toContain("Vokie 没有运行");
    expect(wrapper.text()).not.toContain("没有检测到 Vokie");

    // 没运行 → 第 ② 步给出「打开 Vokie」按钮：点它调用 launchVokie 并提示重新检测。
    const launchButton = wrapper
      .findAll("button")
      .find((button) => button.text().includes("打开 Vokie"))!;
    expect(launchButton).toBeDefined();
    await launchButton.trigger("click");
    await flushPromises();
    expect(mocks.launchVokie).toHaveBeenCalledTimes(1);
    expect(wrapper.text()).toContain("已打开 Vokie");

    // 启动后点“重新检测”：提示消失。
    mocks.getVokieInstallation.mockResolvedValue({ installed: true, running: true });
    mocks.getOtherVoiceHotkey.mockResolvedValue(null);
    mocks.setOtherVoiceHotkey.mockImplementation(async (keys: string[]) => keys);
    const panel = wrapper.find(".setup-col:nth-child(2)");
    await panel
      .findAll("button")
      .find((button) => button.text().includes("重新检测"))!
      .trigger("click");
    await flushPromises();
    expect(wrapper.text()).not.toContain("Vokie 没有运行");
    wrapper.unmount();
  });

  it("第 ② 步常显提示：避免其他 App 同键 + 当前输入法不对时再按一次", async () => {
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();
    expect(wrapper.text()).toContain("避免其他 App 使用同一个快捷键");
    expect(wrapper.text()).toContain("可能需要再按住一次才能正常使用");
    wrapper.unmount();
  });

  it("选豆包且 Vokie 正在运行：豆包面板给出“右 Alt 会被 Vokie 抢先”的提示", async () => {
    mocks.getVoiceInputTool.mockResolvedValue("doubao");
    mocks.getVokieInstallation.mockResolvedValue({ installed: true, running: true });
    mocks.getOtherVoiceHotkey.mockResolvedValue(null);
    mocks.setOtherVoiceHotkey.mockImplementation(async (keys: string[]) => keys);
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();

    expect(selectedToolCard(wrapper)).toBe("豆包输入法");
    expect(wrapper.text()).toContain("检测到 Vokie 正在运行");
    wrapper.unmount();
  });

  it("未安装 Vokie：显示官网入口与重新检测，点官网按钮调用 openVokieHomepage", async () => {
    mocks.getVokieInstallation.mockResolvedValue({ installed: false, running: false });
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();

    await wrapper
      .findAll(".tool-card")
      .find((card) => card.text().includes("Vokie"))!
      .trigger("click");
    await flushPromises();

    expect(wrapper.text()).toContain("没有检测到 Vokie");
    const panel = wrapper.find(".setup-col:nth-child(2)");
    const siteButton = panel
      .findAll("button")
      .find((button) => button.text().includes("打开官网"))!;
    expect(siteButton).toBeDefined();
    await siteButton.trigger("click");
    await flushPromises();
    expect(mocks.openVokieHomepage).toHaveBeenCalledOnce();

    // 装好之后点“重新检测”：提示消失。
    mocks.getVokieInstallation.mockResolvedValue({ installed: true, running: true });
    mocks.getOtherVoiceHotkey.mockResolvedValue(null);
    mocks.setOtherVoiceHotkey.mockImplementation(async (keys: string[]) => keys);
    await panel
      .findAll("button")
      .find((button) => button.text().includes("重新检测"))!
      .trigger("click");
    await flushPromises();
    expect(wrapper.text()).not.toContain("没有检测到 Vokie");
    wrapper.unmount();
  });

  it("切换输入工具期间不闪「未同步」黄标（两段 IPC 之间按乐观态显示）", async () => {
    // 按住说话快捷键写入挂在半路：复现"工具已切、快捷键还在写盘"的中间态。
    const gate: { release: (() => void) | null } = { release: null };
    mocks.setVoiceHoldHotkey.mockImplementation(
      (hotkey: unknown) =>
        new Promise((resolve) => {
          gate.release = () => resolve(hotkey);
        }),
    );
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();

    // 起点：微信 + 左 Ctrl + 左 Win（beforeEach 默认），面板显示"已自动设置"。
    const panelText = () =>
      wrapper.find(".setup-col:nth-child(2)").text();
    expect(panelText()).toContain("已自动设置");

    // 点豆包：点击后 nextTick 已过、快捷键尚未写盘——旧实现这里会闪"未同步"。
    await wrapper
      .findAll(".tool-card")
      .find((card) => card.text().includes("豆包"))!
      .trigger("click");
    expect(panelText()).not.toContain("未同步");

    gate.release?.();
    await flushPromises();
    expect(panelText()).toContain("已自动设置");
    expect(panelText()).not.toContain("未同步");
    wrapper.unmount();
  });

  it("切换输入工具但快捷键写盘失败：回到「未同步」而不是停在乐观态", async () => {
    mocks.setVoiceHoldHotkey.mockRejectedValue(new Error("写入失败"));
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();

    await wrapper
      .findAll(".tool-card")
      .find((card) => card.text().includes("豆包"))!
      .trigger("click");
    await flushPromises();

    expect(wrapper.find(".setup-col:nth-child(2)").text()).toContain("未同步");
    wrapper.unmount();
  });

  it("点其他工具卡片：不改变当前快捷键，改为提供按键芯片与自定义录入", async () => {
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();

    await wrapper
      .findAll(".tool-card")
      .find((card) => card.text().includes("其他工具"))!
      .trigger("click");
    await flushPromises();

    expect(mocks.setVoiceInputTool).toHaveBeenCalledWith("other");
    expect(mocks.setVoiceHoldHotkey).not.toHaveBeenCalled();
    const chips = wrapper.findAll(".chip-select .chip").map((chip) => chip.text());
    expect(chips.slice(0, 3)).toEqual(["右 Alt", "左 Alt", "不按键"]);
    expect(chips[3]).toBe(VOICE_HOTKEY_CUSTOM_CAPTURE_ENABLED ? "自定义组合键" : "自定义组合键（暂未开放）");
    expect(wrapper.find(".chip-select .chip:disabled").exists()).toBe(!VOICE_HOTKEY_CUSTOM_CAPTURE_ENABLED);
    wrapper.unmount();
  });

  it("其他工具：选左 Alt 落盘左 Alt，选不按键落盘关闭", async () => {
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();
    await wrapper
      .findAll(".tool-card")
      .find((card) => card.text().includes("其他工具"))!
      .trigger("click");
    await flushPromises();

    await wrapper
      .findAll(".chip-select .chip")
      .find((chip) => chip.text() === "左 Alt")!
      .trigger("click");
    await flushPromises();
    expect(mocks.setVoiceHoldHotkey).toHaveBeenLastCalledWith({ keys: ["left_alt"] });
    expect(
      wrapper.find(".chip-select .chip[aria-pressed='true']").text(),
    ).toBe("左 Alt");

    await wrapper
      .findAll(".chip-select .chip")
      .find((chip) => chip.text() === "不按键")!
      .trigger("click");
    await flushPromises();
    expect(mocks.setVoiceHoldHotkey).toHaveBeenLastCalledWith(null);
    expect(
      wrapper.find(".chip-select .chip[aria-pressed='true']").text(),
    ).toBe("不按键");
    wrapper.unmount();
  });

  it("默认工具为微信时，面板显示 左 Ctrl + 左 Win 且标记已自动设置", async () => {
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();

    const panel = wrapper.find(".setup-col:nth-child(2)");
    expect(panel.text()).toContain("左 Ctrl");
    expect(panel.text()).toContain("左 Win");
    expect(panel.text()).toContain("已自动设置");
    wrapper.unmount();
  });

  it("工具选择保存失败时回退到原工具并显示错误", async () => {
    mocks.setVoiceInputTool.mockRejectedValueOnce(new Error("磁盘写入失败"));
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();

    await wrapper
      .findAll(".tool-card")
      .find((card) => card.text().includes("豆包输入法"))!
      .trigger("click");
    await flushPromises();

    expect(selectedToolCard(wrapper)).toBe("微信输入法");
    // 原始 IPC 错误只进控制台：界面显示与语言一致的短句（英文界面不夹带后端原文）。
    expect(wrapper.text()).toContain("操作没有完成，请稍后重试。");
    expect(wrapper.text()).not.toContain("磁盘写入失败");
    expect(mocks.setVoiceHoldHotkey).not.toHaveBeenCalled();
    wrapper.unmount();
  });

  it.skipIf(!VOICE_HOTKEY_CUSTOM_CAPTURE_ENABLED)("accepts a lone modifier as the hold-to-talk hotkey (长按右 Alt 一类)", async () => {
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();

    await startCustomCapture(wrapper);

    mocks.captureEdgeHandler!({ key: "right_alt", isPressed: true });
    await flushPromises();
    expect(mocks.setVoiceHoldHotkey).not.toHaveBeenCalled();

    mocks.captureEdgeHandler!({ key: "right_alt", isPressed: false });
    await settleVoiceCapture();
    expect(mocks.setVoiceHoldHotkey).toHaveBeenCalledWith({ keys: ["right_alt"] });
    expect(wrapper.text()).toContain("按住说话快捷键已设为 右 Alt");
    // 其他工具的按键芯片应与落盘结果一致（误录可一键换回）。
    expect(wrapper.find(".chip-select .chip[aria-pressed='true']").text()).toBe("右 Alt");
    wrapper.unmount();
  });

  it.skipIf(!VOICE_HOTKEY_CUSTOM_CAPTURE_ENABLED)("cancels capture with Esc and keeps the current hotkey", async () => {
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();

    await startCustomCapture(wrapper);

    mocks.captureEdgeHandler!({ key: "escape", isPressed: true });
    await flushPromises();
    expect(mocks.setVoiceHoldHotkey).not.toHaveBeenCalled();
    expect(mocks.stopShortcutCapture).toHaveBeenCalledOnce();
    expect(wrapper.find(".voice-hotkey-capture").exists()).toBe(false);
    expect(wrapper.text()).toContain("已取消录入");
    wrapper.unmount();
  });

  it.skipIf(!VOICE_HOTKEY_CUSTOM_CAPTURE_ENABLED)("records a custom hold-to-talk chord and only saves it after every key is released", async () => {
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();

    await startCustomCapture(wrapper);
    expect(mocks.startShortcutCapture).toHaveBeenCalledOnce();
    expect(mocks.captureEdgeHandler).not.toBeNull();
    expect(wrapper.find(".voice-hotkey-capture").text()).toContain(
      "请按下你输入工具当前设置的语音键",
    );

    mocks.captureEdgeHandler!({ key: "right_alt", isPressed: true });
    mocks.captureEdgeHandler!({ key: "d", isPressed: true });
    await flushPromises();
    expect(wrapper.find(".voice-hotkey-capture").text()).toContain("右 Alt + D");
    // 物理键未松开前不落盘：Win+L 一类组合不会在录入中提前生效。
    expect(mocks.setVoiceHoldHotkey).not.toHaveBeenCalled();

    mocks.captureEdgeHandler!({ key: "d", isPressed: false });
    expect(mocks.stopShortcutCapture).not.toHaveBeenCalled();
    mocks.captureEdgeHandler!({ key: "right_alt", isPressed: false });
    await settleVoiceCapture();

    expect(mocks.setVoiceHoldHotkey).toHaveBeenCalledWith({ keys: ["right_alt", "d"] });
    expect(mocks.stopShortcutCapture).toHaveBeenCalledOnce();
    expect(wrapper.find(".voice-hotkey-capture").exists()).toBe(false);
    expect(wrapper.text()).toContain("按住说话快捷键已设为 右 Alt + D");
    wrapper.unmount();
  });

  it.skipIf(!VOICE_HOTKEY_CUSTOM_CAPTURE_ENABLED)("asks to release pre-held keys first and saves the full chord after arming", async () => {
    // 2026-09-27 回归：录入开始时仍有按键按住（preheld），其边沿对录入
    // 不可见；后端等 preheld 全部松开后才投递边沿，前端先提示松手，
    // 杜绝把新组合截断成半截（"只剩左 Ctrl"）。
    mocks.startShortcutCapture.mockResolvedValue(["left_windows"]);
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();

    await startCustomCapture(wrapper);
    expect(wrapper.find(".voice-hotkey-capture").text()).toContain("请先松开所有按键");
    expect(mocks.setVoiceHoldHotkey).not.toHaveBeenCalled();

    // 后端武装后的第一条边沿：提示清除，正常进入录入。
    mocks.captureEdgeHandler!({ key: "left_control", isPressed: true });
    await flushPromises();
    expect(wrapper.find(".voice-hotkey-capture").text()).not.toContain("请先松开所有按键");

    mocks.captureEdgeHandler!({ key: "left_windows", isPressed: true });
    mocks.captureEdgeHandler!({ key: "left_control", isPressed: false });
    await flushPromises();
    // 组合未全部松开前不落盘（此处 Win 是松手后重新按下的新鲜按键）。
    expect(mocks.setVoiceHoldHotkey).not.toHaveBeenCalled();

    mocks.captureEdgeHandler!({ key: "left_windows", isPressed: false });
    await settleVoiceCapture();
    expect(mocks.setVoiceHoldHotkey).toHaveBeenCalledWith({
      keys: ["left_control", "left_windows"],
    });
    wrapper.unmount();
  });

  it.skipIf(!VOICE_HOTKEY_CUSTOM_CAPTURE_ENABLED)("keeps every modifier of a modifier-only chord when keys are released out of order", async () => {
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();

    await startCustomCapture(wrapper);

    // 回归（Bugs/2026-09-27）：左 Ctrl + 左 Win 先松 Win、后松 Ctrl，
    // 不能把组合截断成只剩 Ctrl。
    mocks.captureEdgeHandler!({ key: "left_control", isPressed: true });
    mocks.captureEdgeHandler!({ key: "left_windows", isPressed: true });
    mocks.captureEdgeHandler!({ key: "left_windows", isPressed: false });
    await flushPromises();
    expect(mocks.setVoiceHoldHotkey).not.toHaveBeenCalled();

    mocks.captureEdgeHandler!({ key: "left_control", isPressed: false });
    await settleVoiceCapture();
    expect(mocks.setVoiceHoldHotkey).toHaveBeenCalledWith({
      keys: ["left_control", "left_windows"],
    });
    expect(mocks.stopShortcutCapture).toHaveBeenCalledOnce();
    expect(wrapper.text()).toContain("按住说话快捷键已设为 左 Ctrl + 左 Win");
    wrapper.unmount();
  });

  it.skipIf(!VOICE_HOTKEY_CUSTOM_CAPTURE_ENABLED)("saves the full chord when the swallowed key only arrives as a replayed injected copy", async () => {
    // 2026-09-27 真机回归（Bugs/2026-09-27-ime-chord-hook-eats-active-hotkey-capture.md）：
    // 微信输入法的语音和弦就是默认的 左 Ctrl + 左 Win。按下该组合时它吞掉
    // 左 Win 的物理边沿、随后把整个组合以注入副本重放——真实到达后端的只有
    // 左 Ctrl 的真实边沿 + 左 Win 的注入副本，且副本可能晚于物理松开。
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();

    await startCustomCapture(wrapper);

    // 物理按下：只有左 Ctrl 的真实边沿（左 Win 被输入法吞掉）。
    mocks.captureEdgeHandler!({ key: "left_control", isPressed: true, source: "real" });
    // 物理松开：左 Ctrl 的真实抬起先到——此时组合看似"只剩左 Ctrl"。
    mocks.captureEdgeHandler!({ key: "left_control", isPressed: false, source: "real" });
    await flushPromises();
    // 稳定窗口内不得提前落盘（否则就是"只剩左 Ctrl"）。
    expect(mocks.setVoiceHoldHotkey).not.toHaveBeenCalled();

    // 输入法重放的注入副本晚到：完整的两键组合。
    mocks.captureEdgeHandler!({ key: "left_control", isPressed: true, source: "injected" });
    mocks.captureEdgeHandler!({ key: "left_windows", isPressed: true, source: "injected" });
    mocks.captureEdgeHandler!({ key: "left_windows", isPressed: false, source: "injected" });
    mocks.captureEdgeHandler!({ key: "left_control", isPressed: false, source: "injected" });
    await settleVoiceCapture();

    expect(mocks.setVoiceHoldHotkey).toHaveBeenCalledWith({
      keys: ["left_control", "left_windows"],
    });
    expect(wrapper.text()).toContain("按住说话快捷键已设为 左 Ctrl + 左 Win");
    wrapper.unmount();
  });

  it.skipIf(!VOICE_HOTKEY_CUSTOM_CAPTURE_ENABLED)("keeps the whole modifier-only chord when the first-pressed modifier is released last", async () => {
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();

    await startCustomCapture(wrapper);

    mocks.captureEdgeHandler!({ key: "left_windows", isPressed: true });
    mocks.captureEdgeHandler!({ key: "left_control", isPressed: true });
    mocks.captureEdgeHandler!({ key: "left_control", isPressed: false });
    mocks.captureEdgeHandler!({ key: "left_windows", isPressed: false });
    await settleVoiceCapture();

    expect(mocks.setVoiceHoldHotkey).toHaveBeenCalledWith({
      keys: ["left_windows", "left_control"],
    });
    expect(wrapper.text()).toContain("按住说话快捷键已设为 左 Win + 左 Ctrl");
    // 默认组合不需要"与微信输入法语音键不一致"的提醒（顺序无关比较）。
    expect(wrapper.text()).not.toContain("不一致将无法生效");
    wrapper.unmount();
  });

  it.skipIf(!VOICE_HOTKEY_CUSTOM_CAPTURE_ENABLED)("infers the WeType chord when only one edge survived but WeType voice was triggered", async () => {
    // 2026-09-27 探针结论：微信输入法吞掉其语音热键组成键的物理边沿发生在
    // RIT 层，对本进程零/半截边沿（低级钩子、Raw Input、GetAsyncKeyState 都
    // 看不到）；其麦克风在录入期间被触发（observed）是唯一旁证 → 推断用户按
    // 的就是微信输入法语音热键，落盘产品默认组合并说明原因。
    mocks.stopShortcutCapture.mockResolvedValue({ wetypeVoice: "observed" });
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();

    await startCustomCapture(wrapper);

    // 只有左 Ctrl 的真实边沿到达（左 Win 被吞，半截会话）。
    mocks.captureEdgeHandler!({ key: "left_control", isPressed: true, source: "real" });
    mocks.captureEdgeHandler!({ key: "left_control", isPressed: false, source: "real" });
    await settleVoiceCapture();

    expect(mocks.setVoiceHoldHotkey).toHaveBeenCalledWith({
      keys: ["left_control", "left_windows"],
    });
    expect(wrapper.text()).toContain("已按微信输入法语音键默认值 左 Ctrl + 左 Win");
    wrapper.unmount();
  });

  it.skipIf(!VOICE_HOTKEY_CUSTOM_CAPTURE_ENABLED)("keeps a lone modifier when WeType voice was not triggered", async () => {
    // 单修饰键（豆包"长按右 Alt"一类）合法：微信输入法语音未被触发时不得推断。
    mocks.stopShortcutCapture.mockResolvedValue({ wetypeVoice: "not_observed" });
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();

    await startCustomCapture(wrapper);

    mocks.captureEdgeHandler!({ key: "right_alt", isPressed: true });
    mocks.captureEdgeHandler!({ key: "right_alt", isPressed: false });
    await settleVoiceCapture();

    expect(mocks.setVoiceHoldHotkey).toHaveBeenCalledWith({ keys: ["right_alt"] });
    expect(wrapper.text()).not.toContain("已按微信输入法语音键默认值");
    wrapper.unmount();
  });

  it.skipIf(!VOICE_HOTKEY_CUSTOM_CAPTURE_ENABLED)("warns that a non-WeType chord will not trigger hold-to-talk voice", async () => {
    // Win + 右 Ctrl 不是微信输入法热键：边沿透传能录上，但按住说话靠注入该
    // 组合唤起微信输入法语音，不一致就无法生效——必须把这一后果告诉用户。
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();

    await startCustomCapture(wrapper);

    mocks.captureEdgeHandler!({ key: "right_windows", isPressed: true });
    mocks.captureEdgeHandler!({ key: "right_control", isPressed: true });
    mocks.captureEdgeHandler!({ key: "right_control", isPressed: false });
    mocks.captureEdgeHandler!({ key: "right_windows", isPressed: false });
    await settleVoiceCapture();

    expect(mocks.setVoiceHoldHotkey).toHaveBeenCalledWith({
      keys: ["right_windows", "right_control"],
    });
    expect(wrapper.text()).toContain("已设为 右 Win + 右 Ctrl");
    expect(wrapper.text()).toContain("若与微信输入法语音键不一致将无法生效");
    wrapper.unmount();
  });
});

/**
 * 遥控器型号显示（2026-10-01 Andy 要求）：已识别型号时状态条标题直接显示型号，
 * 未识别（GATT 2A24 未读回）或未连接时退回蓝牙广播名 / 阶段文案。
 */
describe("connection page remote model", () => {
  beforeEach(() => {
    mocks.getConnectionSnapshot.mockResolvedValue(emptyConnection);
    mocks.getAudioSnapshot.mockResolvedValue(emptyAudio);
    mocks.listAudioEndpoints.mockResolvedValue([]);
    mocks.getVoiceHoldHotkey.mockResolvedValue({ keys: ["left_control", "left_windows"] });
    mocks.setVoiceHoldHotkey.mockImplementation(async (hotkey) => hotkey);
    mocks.getVoiceInputTool.mockResolvedValue("wechat");
    mocks.setVoiceInputTool.mockImplementation(async (tool) => tool);
    mocks.getVokieInstallation.mockResolvedValue({ installed: true, running: true });
    mocks.getOtherVoiceHotkey.mockResolvedValue(null);
    mocks.setOtherVoiceHotkey.mockImplementation(async (keys: string[]) => keys);
    mocks.openVokieHomepage.mockResolvedValue(undefined);
    mocks.startShortcutCapture.mockResolvedValue([]);
    mocks.stopShortcutCapture.mockResolvedValue(undefined);
    mocks.subscribeShortcutCaptureEdges.mockImplementation(
      async (handler: ShortcutCaptureHandler) => {
        mocks.captureEdgeHandler = handler;
        return () => {};
      },
    );
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it.each([
    ["rc001", "小米蓝牙语音遥控器 2"],
    ["rc003", "小米蓝牙语音遥控器 2 Pro"],
  ] as const)("%s 连接后标题显示型号", async (model, expected) => {
    mocks.getConnectionSnapshot.mockResolvedValue({
      ...emptyConnection,
      phase: "ready",
      remoteName: "小米蓝牙语音遥控器",
      remoteModel: model,
    });
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();

    expect(wrapper.find(".connection-title").text()).toBe(expected);
    wrapper.unmount();
  });

  it("型号未识别时退回蓝牙广播名，未连接时显示阶段文案", async () => {
    mocks.getConnectionSnapshot.mockResolvedValue({
      ...emptyConnection,
      phase: "ready",
      remoteName: "小米蓝牙语音遥控器",
      remoteModel: "unknown",
    });
    let wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();
    expect(wrapper.find(".connection-title").text()).toBe("小米蓝牙语音遥控器");
    wrapper.unmount();

    mocks.getConnectionSnapshot.mockResolvedValue(emptyConnection);
    wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();
    expect(wrapper.find(".connection-title").text()).toBe("尚未连接");
    wrapper.unmount();
  });
});

/**
 * 「支持更多输入工具」开关（2026-09-29）：与按键页「全按键支持」同一设置项
 * 的连接页入口。判据（授权确认、失败回退、权威对账）与 ButtonsPage 同源。
 */
describe("connection page rc003 capture switch", () => {
  const taskStatus = (
    overrides: Partial<import("../lib/bridge").Rc003TaskStatus> = {},
  ) => ({
    installed: true,
    authorizationRequired: false,
    enabled: false,
    helperPath: null,
    lastError: null,
    ...overrides,
  });
  /** 桥接快照夹具（本页只关心 phase 与 captureHealth，其余字段按"空"填）。 */
  const bridgeSnapshot = (
    phase: Rc003BridgeSnapshot["phase"],
    captureHealth: Rc003BridgeSnapshot["captureHealth"] = "off",
    captureRevoked: Rc003BridgeSnapshot["captureRevoked"] = "none",
    captureUnavailable: Rc003BridgeSnapshot["captureUnavailable"] = "none",
  ): Rc003BridgeSnapshot => ({
    phase,
    port: 0,
    helperPid: 0,
    acceptedTotal: 0,
    deniedTotal: 0,
    replacedTotal: 0,
    edgesApplied: 0,
    usagesDropped: 0,
    malformedTotal: 0,
    watchdogReleaseTotal: 0,
    pressedUsages: [],
    lastRxAgeMs: null,
    targetGeneration: 0,
    targetUsages: [],
    ownedUsages: [],
    captureHealth,
    captureUnavailable,
    captureRevoked,
  });

  afterEach(() => {
    // 本 describe 不继承上面 VB-CABLE describe 的 afterEach（作用域只在
    // 各自 describe 内）；没有它，spy 调用计数跨用例累计，"未被调用"
    // 断言会吃到上一个用例的调用记录（本次失败实证）。
    vi.clearAllMocks();
  });

  beforeEach(() => {
    // vi.clearAllMocks 只清调用记录不清实现（ButtonsPage 2026-09-28 教训）：
    // 每个 beforeEach 显式重置实现。
    mocks.getConnectionSnapshot.mockResolvedValue(emptyConnection);
    mocks.getAudioSnapshot.mockResolvedValue(emptyAudio);
    mocks.listAudioEndpoints.mockResolvedValue([]);
    mocks.getVoiceHoldHotkey.mockResolvedValue({ keys: ["left_control", "left_windows"] });
    mocks.setVoiceHoldHotkey.mockImplementation(async (hotkey) => hotkey);
    // 开关只在需要它的工具面板里渲染：这里固定为豆包（要求开启）。
    mocks.getVoiceInputTool.mockResolvedValue("doubao");
    mocks.setVoiceInputTool.mockImplementation(async (tool) => tool);
    mocks.getVokieInstallation.mockResolvedValue({ installed: true, running: true });
    mocks.getOtherVoiceHotkey.mockResolvedValue(null);
    mocks.setOtherVoiceHotkey.mockImplementation(async (keys: string[]) => keys);
    mocks.openVokieHomepage.mockResolvedValue(undefined);
    mocks.startShortcutCapture.mockResolvedValue([]);
    // 本机支持情况：默认可用；不可用分支由 issue #206 的用例覆盖。
    mocks.getCaptureSupport.mockResolvedValue(supportedCaptureSupport());
    mocks.stopShortcutCapture.mockResolvedValue(undefined);
    mocks.subscribeShortcutCaptureEdges.mockImplementation(
      async (handler: ShortcutCaptureHandler) => {
        mocks.captureEdgeHandler = handler;
        return () => {};
      },
    );
    mocks.getGainDb.mockResolvedValue(0);
    mocks.setGainDb.mockImplementation(async (gainDb: number) => gainDb);
    mocks.getRc003TaskStatus.mockResolvedValue(taskStatus());
    mocks.enableRc003Capture.mockResolvedValue(taskStatus({ enabled: true }));
    mocks.disableRc003Capture.mockResolvedValue(taskStatus({ enabled: false }));
    // 生效状态默认"没开"（关闭态）：开着却没生效的分支由 2026-10-13 的用例覆盖。
    mocks.getRc003BridgeSnapshot.mockResolvedValue(bridgeSnapshot("stopped", "off"));
  });

  it("助手启动失败时显示诊断入口，不建议系统级重启", async () => {
    mocks.getRc003TaskStatus.mockResolvedValue(taskStatus({ enabled: true }));
    mocks.getConnectionSnapshot.mockResolvedValue({ ...emptyConnection, phase: "ready" });
    // 桥在监听（助手每次起来就退出）→ 后端报 inactive + startup_failing。
    mocks.getRc003BridgeSnapshot.mockResolvedValue(
      bridgeSnapshot("listening", "inactive", "none", "startup_failing"),
    );
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();
    expect(wrapper.text()).toContain("“支持更多输入工具”暂时没能启动");
    expect(wrapper.text()).toContain("请查看诊断信息");
    expect(wrapper.text()).not.toContain("重启这台电脑");
    // 关键断言：不能让用户去找"是哪个程序"（他无从定位，2026-10-27 反馈）。
    expect(wrapper.text()).not.toContain("其他程序");
    wrapper.unmount();
  });

  it("「开着但没生效」给一句用户语言（2026-10-13）：只在后端说没生效时出现", async () => {
    mocks.getRc003TaskStatus.mockResolvedValue(taskStatus({ enabled: true }));
    // 遥控器链路正常（语音功能就绪）+ 后端说没生效 → 显示提示句。
    mocks.getConnectionSnapshot.mockResolvedValue({ ...emptyConnection, phase: "ready" });
    mocks.getRc003BridgeSnapshot.mockResolvedValue(
      bridgeSnapshot("connected", "inactive"),
    );
    let wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();
    expect(wrapper.text()).toContain(
      "“支持更多输入工具”还没有生效，请查看诊断信息。",
    );
    wrapper.unmount();

    // active：真的生效 → 收回提示（本页提示只在开关旁那一位）。
    mocks.getRc003BridgeSnapshot.mockResolvedValue(bridgeSnapshot("connected", "active"));
    wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();
    expect(wrapper.text()).not.toContain("还没有生效");
    wrapper.unmount();

    // 遥控器链路本身断掉（idle）：不显示这句——那种情形由连接区的既有文案说明，
    // 也不把用户往遥控器侧引（2026-10-13 定稿）。
    mocks.getConnectionSnapshot.mockResolvedValue(emptyConnection);
    mocks.getRc003BridgeSnapshot.mockResolvedValue(
      bridgeSnapshot("connected", "inactive"),
    );
    wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();
    expect(wrapper.text()).not.toContain("还没有生效");
    wrapper.unmount();
  });

  // 2026-10-24 现场：遥控器 + 另一支 BLE 键盘共存时，音频只剩约 1/3，
  // 豆包语音条出现却写不出字。连接页要给一句能执行的提示，不能只留在日志里。
  it("最近一次语音音频降级：连接页显示蓝牙共存提示（满速时不显示）", async () => {
    mocks.getConnectionSnapshot.mockResolvedValue({
      ...emptyConnection,
      phase: "ready",
      voiceAudioHealth: "degraded",
    });
    let wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();
    expect(wrapper.text()).toContain("蓝牙音频不完整");
    expect(wrapper.text()).toContain("关闭或移开同时连接的其他蓝牙设备");
    wrapper.unmount();

    mocks.getConnectionSnapshot.mockResolvedValue({
      ...emptyConnection,
      phase: "ready",
      voiceAudioHealth: "full",
    });
    wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();
    expect(wrapper.text()).not.toContain("蓝牙音频不完整");
    wrapper.unmount();
  });

  it("启动对账把开关关掉时说明原因（2026-10-15，issue #264 附加观察甲）", async () => {
    // 开关关闭态 + 后端报了回落原因 ⇒ 开关旁一句解释（本页名称是「支持更多输入工具」）。
    mocks.getRc003TaskStatus.mockResolvedValue(taskStatus({ enabled: false }));
    mocks.getRc003BridgeSnapshot.mockResolvedValue(
      bridgeSnapshot("listening", "off", "task_power_policy_mismatch"),
    );
    let wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();
    expect(wrapper.text()).toContain(
      "“支持更多输入工具”因为系统授权需要重新确认，已经被关掉了；重新打开一次就能继续使用。",
    );
    wrapper.unmount();

    // 没有原因（none）：一个字都不显示。
    mocks.getRc003BridgeSnapshot.mockResolvedValue(bridgeSnapshot("stopped", "off"));
    wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();
    expect(wrapper.text()).not.toContain("重新打开一次");
    wrapper.unmount();
  });

  it("挂载对账后开关显示权威状态，对账失败显示占位符", async () => {
    mocks.getRc003TaskStatus.mockResolvedValue(taskStatus({ enabled: true }));
    let wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();
    expect(wrapper.find(".capture-switch").exists()).toBe(true);
    expect((wrapper.find(".capture-switch").element as HTMLInputElement).checked).toBe(
      true,
    );
    wrapper.unmount();

    // 对账失败（IPC 抛错）：开关不猜状态，渲染占位符且不出可点的 input。
    mocks.getRc003TaskStatus.mockRejectedValue(new Error("ipc down"));
    wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();
    expect(wrapper.find(".capture-switch").exists()).toBe(false);
    expect(wrapper.find(".toggle-placeholder").exists()).toBe(true);
    wrapper.unmount();
  });

  it("开关与状态提示合并成一个开关（2026-10-10）：未开启时没有状态字、也不劝开启", async () => {
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();
    // 关闭态：开关本身即状态，一个字都不显示。
    expect(wrapper.find(".capture-switch").exists()).toBe(true);
    expect(wrapper.find(".switch-state").exists()).toBe(false);
    expect(wrapper.text()).not.toContain("需要开启");
    expect(wrapper.text()).not.toContain("未开启");
    expect(wrapper.text()).not.toContain("还差一步");
    expect(wrapper.text()).not.toContain("建议开启");

    await wrapper.find(".capture-switch").setValue(true);
    await flushPromises();
    // 2026-10-03 起每次开启都先弹确认（与按键页同源）：确认后才真正开启。
    await wrapper
      .findComponent({ name: "EnhancedCaptureConfirmDialog" })
      .vm.$emit("confirm");
    await flushPromises();
    // 开启态同样没有状态字（「已开启」不再渲染）。
    expect((wrapper.find(".capture-switch").element as HTMLInputElement).checked).toBe(true);
    expect(wrapper.find(".switch-state").exists()).toBe(false);
    expect(wrapper.text()).not.toContain("已开启");
    wrapper.unmount();
  });

  it("正在操作：开关置灰并出现转圈，且不配任何文字（2026-10-10）", async () => {
    // enable 挂起 = 操作进行中（真机上这一段是授权窗口期间）。
    let release!: (value: Awaited<ReturnType<typeof mocks.enableRc003Capture>>) => void;
    mocks.enableRc003Capture.mockImplementation(
      () =>
        new Promise((resolve) => {
          release = resolve;
        }),
    );
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();
    await wrapper.find(".capture-switch").setValue(true);
    await flushPromises();
    await wrapper
      .findComponent({ name: "EnhancedCaptureConfirmDialog" })
      .vm.$emit("confirm");
    await flushPromises();

    const input = wrapper.find<HTMLInputElement>(".capture-switch");
    expect(input.element.disabled, "进行中必须置灰").toBe(true);
    expect(wrapper.find(".toggle-switch-slot .toggle-spinner").exists()).toBe(true);
    expect(wrapper.text()).not.toContain("正在操作");
    expect(wrapper.text()).not.toContain("正在启动");

    release(taskStatus({ enabled: true }));
    await flushPromises();
    expect(wrapper.find(".toggle-spinner").exists()).toBe(false);
    wrapper.unmount();
  });

  it("每次开启都先弹确认：已授权（authorizationRequired=false）也一样，确认后才 enable", async () => {
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();
    await wrapper.find(".capture-switch").setValue(true);
    await flushPromises();

    // 未确认：不开确认弹窗前不调 enable，开关 DOM 写回关闭。
    const dialog = wrapper.findComponent({ name: "EnhancedCaptureConfirmDialog" });
    expect(dialog.exists()).toBe(true);
    expect(mocks.enableRc003Capture).not.toHaveBeenCalled();
    expect(mocks.disableRc003Capture).not.toHaveBeenCalled();
    expect((wrapper.find(".capture-switch").element as HTMLInputElement).checked).toBe(
      false,
    );

    await dialog.vm.$emit("confirm");
    await flushPromises();
    expect(mocks.enableRc003Capture).toHaveBeenCalledTimes(1);
    expect((wrapper.find(".capture-switch").element as HTMLInputElement).checked).toBe(
      true,
    );
    wrapper.unmount();
  });

  it("需要授权时先弹确认弹窗，确认后才调用 enable", async () => {
    mocks.getRc003TaskStatus.mockResolvedValue(
      taskStatus({ installed: false, authorizationRequired: true }),
    );
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();

    await wrapper.find(".capture-switch").setValue(true);
    await flushPromises();
    // 未确认：不调 enable，开关 DOM 写回关闭。
    expect(mocks.enableRc003Capture).not.toHaveBeenCalled();
    const dialog = wrapper.findComponent({ name: "EnhancedCaptureConfirmDialog" });
    expect(dialog.exists()).toBe(true);
    expect((wrapper.find(".capture-switch").element as HTMLInputElement).checked).toBe(
      false,
    );

    // 确认后走 enable，开关随结果开启。
    await dialog.vm.$emit("confirm");
    await flushPromises();
    expect(mocks.enableRc003Capture).toHaveBeenCalledTimes(1);
    expect((wrapper.find(".capture-switch").element as HTMLInputElement).checked).toBe(
      true,
    );
    wrapper.unmount();
  });

  it("操作失败（lastError）：开关回退原状态，并只显示普通用户能读懂的原因（2026-10-10）", async () => {
    mocks.enableRc003Capture.mockResolvedValue(
      taskStatus({ enabled: false, lastError: "授权未完成（UAC 被取消）" }),
    );
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();

    await wrapper.find(".capture-switch").setValue(true);
    await flushPromises();
    // 每次开启都先弹确认：确认后 enable 失败（UAC 被取消）才走到失败回退。
    await wrapper
      .findComponent({ name: "EnhancedCaptureConfirmDialog" })
      .vm.$emit("confirm");
    await flushPromises();

    expect((wrapper.find(".capture-switch").element as HTMLInputElement).checked).toBe(
      false,
    );
    // 失败文案是映射后的用户语言：说清结果与下一步，不带内部原文。
    expect(wrapper.find(".capture-failure").text()).toBe(
      "没有完成系统授权，“支持更多输入工具”保持关闭。",
    );
    expect(wrapper.text()).not.toContain("UAC");
    expect(wrapper.text()).not.toContain("授权未完成");
    wrapper.unmount();
  });
});

/**
 * 语音增益（对齐 Mac 设置页「增益」滑块）：0 dB = 原始音量，0–24 dB 步进 1。
 * 拖动只改显示，松手（change）才走 IPC；保存失败必须回到上一次生效值并给原因。
 */
describe("connection page audio gain", () => {
  beforeEach(() => {
    mocks.getConnectionSnapshot.mockResolvedValue(emptyConnection);
    mocks.getAudioSnapshot.mockResolvedValue(emptyAudio);
    mocks.listAudioEndpoints.mockResolvedValue([]);
    mocks.getVoiceHoldHotkey.mockResolvedValue({ keys: ["left_control", "left_windows"] });
    mocks.setVoiceHoldHotkey.mockImplementation(async (hotkey) => hotkey);
    mocks.getVoiceInputTool.mockResolvedValue("wechat");
    mocks.setVoiceInputTool.mockImplementation(async (tool) => tool);
    mocks.getVokieInstallation.mockResolvedValue({ installed: true, running: true });
    mocks.getOtherVoiceHotkey.mockResolvedValue(null);
    mocks.setOtherVoiceHotkey.mockImplementation(async (keys: string[]) => keys);
    mocks.openVokieHomepage.mockResolvedValue(undefined);
    mocks.startShortcutCapture.mockResolvedValue([]);
    // 本机支持情况：默认可用；不可用分支由 issue #206 的用例覆盖。
    mocks.getCaptureSupport.mockResolvedValue(supportedCaptureSupport());
    mocks.stopShortcutCapture.mockResolvedValue(undefined);
    mocks.getGainDb.mockResolvedValue(0);
    mocks.setGainDb.mockImplementation(async (gainDb: number) => gainDb);
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it("读取已保存的增益并渲染滑块与说明", async () => {
    mocks.getGainDb.mockResolvedValue(12);
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();

    const slider = wrapper.find<HTMLInputElement>('input[name="audio-gain"]');
    expect(slider.exists()).toBe(true);
    expect(slider.element.value).toBe("12");
    expect(wrapper.text()).toContain("12 dB");
    expect(wrapper.text()).toContain("0 dB 保持原始音量");
    expect(wrapper.find('[role="alert"]').exists()).toBe(false);
    wrapper.unmount();
  });

  it("拖动松手后才保存，并以保存返回值回显", async () => {
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();
    const slider = wrapper.find<HTMLInputElement>('input[name="audio-gain"]');

    await slider.setValue("18");
    await flushPromises();

    expect(mocks.setGainDb).toHaveBeenCalledTimes(1);
    expect(mocks.setGainDb).toHaveBeenCalledWith(18);
    expect(wrapper.text()).toContain("18 dB");
    wrapper.unmount();
  });

  it("保存失败：回到上一次生效值并就地给出原因", async () => {
    mocks.getGainDb.mockResolvedValue(6);
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();
    const slider = wrapper.find<HTMLInputElement>('input[name="audio-gain"]');

    mocks.setGainDb.mockRejectedValueOnce(new Error("保存增益设置失败：磁盘只读"));
    await slider.setValue("18");
    await flushPromises();

    expect(wrapper.find('[role="alert"]').text()).toContain("增益没有保存");
    expect(wrapper.find('[role="alert"]').text()).not.toContain("磁盘只读");
    expect(slider.element.value).toBe("6");
    expect(wrapper.text()).toContain("6 dB");
    wrapper.unmount();
  });

  it("以保存返回值回显（后端钳制过的值才显示）", async () => {
    mocks.getGainDb.mockResolvedValue(0);
    // 后端把请求值钳到 24：界面必须显示 24，而不是用户拖到的原值。
    mocks.setGainDb.mockImplementationOnce(async () => 24);
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();
    const slider = wrapper.find<HTMLInputElement>('input[name="audio-gain"]');

    await slider.setValue("18");
    await flushPromises();

    expect(slider.element.value).toBe("24");
    expect(wrapper.text()).toContain("24 dB");
    wrapper.unmount();
  });
});

/**
 * 「支持更多输入工具」在本机不可用（2026-10-07 issue #206，Windows 11 ARM64）：
 * 与按键页「全按键支持」同一个设置项，必须同样置灰并说清原因与恢复方式，
 * 文案与按键页互相一致（见 docs/product-copy.md §2 名称表与 §7 决定索引）。
 */
describe("connection page capture support unavailable (issue #206)", () => {
  /** 2026-10-07 定稿：本机不可用时的开关悬停提示（原因 + 恢复方式）。 */
  const UNSUPPORTED_TITLE =
    "“支持更多输入工具”在这台电脑上暂不可用；新版本支持后会恢复正常。";
  /** 2026-10-07 定稿：本机不可用时的就地说明（与按键页同一句，仅名称随本页）。 */
  const UNSUPPORTED_NOTE = "这台电脑暂不支持“支持更多输入工具”，其他按键不受影响。";

  const taskStatus = (
    overrides: Partial<import("../lib/bridge").Rc003TaskStatus> = {},
  ) => ({
    installed: true,
    authorizationRequired: false,
    enabled: false,
    helperPath: null,
    lastError: null,
    ...overrides,
  });

  beforeEach(() => {
    mocks.getConnectionSnapshot.mockResolvedValue(emptyConnection);
    mocks.getAudioSnapshot.mockResolvedValue(emptyAudio);
    mocks.listAudioEndpoints.mockResolvedValue([]);
    mocks.getVoiceHoldHotkey.mockResolvedValue({ keys: ["left_control", "left_windows"] });
    mocks.setVoiceHoldHotkey.mockImplementation(async (hotkey) => hotkey);
    // 开关只在需要它的工具面板里渲染：这里固定为豆包（要求开启）。
    mocks.getVoiceInputTool.mockResolvedValue("doubao");
    mocks.setVoiceInputTool.mockImplementation(async (tool) => tool);
    mocks.getVokieInstallation.mockResolvedValue({ installed: true, running: true });
    mocks.getOtherVoiceHotkey.mockResolvedValue(null);
    mocks.setOtherVoiceHotkey.mockImplementation(async (keys: string[]) => keys);
    mocks.openVokieHomepage.mockResolvedValue(undefined);
    mocks.getGainDb.mockResolvedValue(0);
    mocks.setGainDb.mockImplementation(async (gainDb: number) => gainDb);
    mocks.getRc003TaskStatus.mockResolvedValue(taskStatus());
    mocks.enableRc003Capture.mockResolvedValue(taskStatus({ enabled: true }));
    mocks.disableRc003Capture.mockResolvedValue(taskStatus({ enabled: false }));
    mocks.startShortcutCapture.mockResolvedValue([]);
    mocks.stopShortcutCapture.mockResolvedValue(undefined);
    mocks.subscribeShortcutCaptureEdges.mockImplementation(
      async (handler: ShortcutCaptureHandler) => {
        mocks.captureEdgeHandler = handler;
        return () => {};
      },
    );
    // 本机不支持：本组用例的基线。
    mocks.getCaptureSupport.mockResolvedValue(unsupportedCaptureSupport());
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it("开关置灰、保留「本机暂不支持」说明；不再有状态字与劝开启文案（2026-10-10）", async () => {
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();

    const label = wrapper.find('label[for="capture-switch-doubao"]');
    expect(label.attributes("title"), "开关悬停提示指向原因与恢复方式").toBe(
      UNSUPPORTED_TITLE,
    );
    const input = wrapper.find<HTMLInputElement>(".capture-switch");
    expect(input.element.disabled, "不可用时开关必须置灰").toBe(true);
    // 状态字已并入开关；本机不可用仍保留一句能力边界说明（issue #206 承诺）。
    expect(wrapper.find(".switch-state").exists()).toBe(false);
    expect(wrapper.text()).toContain(UNSUPPORTED_NOTE);
    expect(wrapper.text()).not.toContain("需要开启");
    expect(wrapper.text()).not.toContain("还差一步");

    // 合成事件会绕过原生 disabled：动作入口必须自守，不弹确认、不动 IPC。
    input.element.checked = true;
    await input.trigger("change");
    await flushPromises();
    expect(mocks.enableRc003Capture).not.toHaveBeenCalled();
    expect(mocks.disableRc003Capture).not.toHaveBeenCalled();
    expect(
      wrapper.findComponent({ name: "EnhancedCaptureConfirmDialog" }).exists(),
    ).toBe(false);
    wrapper.unmount();
  });

  it("「其他工具」面板的开关同样置灰并说明；说明句与豆包面板一致", async () => {
    mocks.getVoiceInputTool.mockResolvedValue("other");
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();

    const label = wrapper.find('label[for="capture-switch-other"]');
    expect(label.attributes("title")).toBe(UNSUPPORTED_TITLE);
    expect(wrapper.find<HTMLInputElement>(".capture-switch").element.disabled).toBe(true);
    expect(wrapper.text()).toContain(UNSUPPORTED_NOTE);
    expect(wrapper.text()).not.toContain("建议开启");
    wrapper.unmount();
  });

  it("available=true 时：开关可点、无状态字，开启路径照旧", async () => {
    mocks.getCaptureSupport.mockResolvedValue(supportedCaptureSupport());
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();

    const label = wrapper.find('label[for="capture-switch-doubao"]');
    expect(label.attributes("title")).toBeUndefined();
    expect(wrapper.find<HTMLInputElement>(".capture-switch").element.disabled).toBe(false);
    expect(wrapper.find(".switch-state").exists()).toBe(false);
    expect(wrapper.find(".capture-failure").exists()).toBe(false);
    expect(wrapper.text()).not.toContain(UNSUPPORTED_NOTE);

    // 开启路径照旧（每次开启先弹确认）。
    await wrapper.find(".capture-switch").setValue(true);
    await flushPromises();
    await wrapper
      .findComponent({ name: "EnhancedCaptureConfirmDialog" })
      .vm.$emit("confirm");
    await flushPromises();
    expect(mocks.enableRc003Capture).toHaveBeenCalledTimes(1);
    expect((wrapper.find(".capture-switch").element as HTMLInputElement).checked).toBe(true);
    expect(wrapper.find(".switch-state").exists()).toBe(false);
    // 一次成功的操作不留任何文案（"其余情况不显示文案"）。
    expect(wrapper.find(".capture-failure").exists()).toBe(false);
    wrapper.unmount();
  });

  it("支持情况读不到（IPC 失败）时不置灰：不因一次失败把功能锁死", async () => {
    mocks.getCaptureSupport.mockRejectedValue(new Error("ipc unavailable"));
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();

    expect(wrapper.find<HTMLInputElement>(".capture-switch").element.disabled).toBe(false);
    expect(wrapper.find(".switch-state").exists()).toBe(false);
    expect(wrapper.find(".capture-failure").exists()).toBe(false);
    wrapper.unmount();
  });
});

/**
 * 界面语言（2026-10-24 全界面 i18n）：t() 读取全局有效语言，模板中的调用
 * 会随语言变化重渲染；默认 zh-Hans 下既有中文断言不受影响。
 */
describe("connection page interface language", () => {
  beforeEach(() => {
    mocks.getConnectionSnapshot.mockResolvedValue(emptyConnection);
    mocks.getAudioSnapshot.mockResolvedValue(emptyAudio);
    mocks.listAudioEndpoints.mockResolvedValue([]);
    mocks.getGainDb.mockResolvedValue(0);
    mocks.setGainDb.mockImplementation(async (gainDb: number) => gainDb);
    mocks.getVoiceHoldHotkey.mockResolvedValue({
      keys: ["left_control", "left_windows"],
    });
    mocks.setVoiceHoldHotkey.mockImplementation(async (hotkey) => hotkey);
    mocks.getVoiceInputTool.mockResolvedValue("wechat");
    mocks.setVoiceInputTool.mockImplementation(async (tool) => tool);
    mocks.getVokieInstallation.mockResolvedValue({ installed: true, running: true });
    mocks.getOtherVoiceHotkey.mockResolvedValue(null);
    mocks.setOtherVoiceHotkey.mockImplementation(async (keys: string[]) => keys);
    mocks.openVokieHomepage.mockResolvedValue(undefined);
    mocks.getRc003TaskStatus.mockResolvedValue({
      installed: true,
      authorizationRequired: false,
      enabled: false,
      helperPath: null,
      lastError: null,
    });
    mocks.startShortcutCapture.mockResolvedValue([]);
    mocks.getCaptureSupport.mockResolvedValue(supportedCaptureSupport());
    mocks.stopShortcutCapture.mockResolvedValue(undefined);
    mocks.subscribeShortcutCaptureEdges.mockImplementation(
      async (handler: ShortcutCaptureHandler) => {
        mocks.captureEdgeHandler = handler;
        return () => {};
      },
    );
    effectiveLanguage.value = "zh-Hans";
  });

  afterEach(() => {
    effectiveLanguage.value = "zh-Hans";
    vi.clearAllMocks();
  });

  it("英文模式渲染英文，切回中文立即恢复（无需重新挂载）", async () => {
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();

    effectiveLanguage.value = "en";
    await flushPromises();

    // 静态区块随语言重渲染；逐块断言，避免依赖某个中文 key 最终由哪个
    // fragment 提供（共享字典按中文原文去重，个别 key 由其他页面提供）。
    const han = /[\u4e00-\u9fff]/;
    expect(wrapper.find("h1").text()).not.toMatch(han);
    expect(wrapper.find(".setup-card h2").text()).not.toMatch(han);
    expect(wrapper.find(".gain-label").text()).not.toMatch(han);
    expect(wrapper.find(".gain-help").text()).not.toMatch(han);
    expect(wrapper.get(".capture-input-behavior").text()).not.toMatch(han);
    expect(wrapper.get("label[for='capture-input-switch']").text()).toBe("Switch microphone at voice start");
    expect(wrapper.get("label[for='capture-input-restore']").text()).toBe("Restore microphone at voice end");
    expect(wrapper.find(".usage-hint-details summary").text()).not.toMatch(han);
    for (const card of wrapper.findAll(".tool-card")) {
      expect(card.text()).not.toMatch(han);
    }
    for (const item of wrapper.findAll(".checklist li")) {
      expect(item.text()).not.toMatch(han);
    }

    effectiveLanguage.value = "zh-Hans";
    await flushPromises();

    expect(wrapper.find(".setup-card h2").text()).toBe("语音输入设置");
    expect(wrapper.find(".gain-label").text()).toBe("增益");
    expect(wrapper.get("label[for='capture-input-switch']").text()).toBe("语音开始时切换麦克风");
    expect(wrapper.get("label[for='capture-input-restore']").text()).toBe("语音结束时恢复麦克风");
    expect(wrapper.find(".usage-hint-details summary").text()).toBe("常见问题（点开查看）");
    wrapper.unmount();
  });

  it("动态文案用占位符插值，设备名与数量来自运行时数据", async () => {
    effectiveLanguage.value = "en";
    mocks.listAudioEndpoints.mockResolvedValue([
      cableEndpoint,
      { ...cableEndpoint, id: "cable-input-2" },
    ]);
    const wrapper = mount(ConnectionPage, { props: { runtime } });
    await flushPromises();

    const summary = wrapper.find(".scan-summary").text();
    expect(summary).toContain("2");
    expect(summary).not.toMatch(/[\u4e00-\u9fff]/);
    // 整页在英文模式下不得残留中文（设备名、型号等数据本身是英文/数字）。
    expect(wrapper.text()).not.toMatch(/[\u4e00-\u9fff]/);
    wrapper.unmount();
  });
});
