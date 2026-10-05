vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(async (command: string) => command === "get_ui_preferences" ? {lockButtonSelection:true,templatesExpanded:true,associationsExpanded:true} : undefined) }));
import { beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount, type VueWrapper } from "@vue/test-utils";
import ButtonsPage from "./ButtonsPage.vue";

type EdgeHandler = (edge: { button: string; isPressed: boolean }) => void;
type GestureHandler = (gesture: { button: string; trigger: string }) => void;
type ShortcutCaptureHandler = (edge: { key: string; isPressed: boolean }) => void;
type SceneHandler = (event: import("../lib/bridge").SceneEvent) => void;

let edgeHandler: EdgeHandler | null = null;
let gestureHandler: GestureHandler | null = null;
let shortcutCaptureHandler: ShortcutCaptureHandler | null = null;
let sceneHandler: SceneHandler | null = null;

vi.mock("../lib/bridge", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../lib/bridge")>();
  const builtinCatalog = (await import("../../contracts/ipc/template-catalog-builtins.json")).default;
  const userTemplate = () => ({
    ...structuredClone({mappings: builtinCatalog[0]!.buttonMappings}),
    id: "scene-copy",
    name: "我的按键模板",
  });
  return {
    ...actual,
    getSceneSnapshot: vi.fn(async () => ({ templateId: "preset-agent", mappingNotice: { kind: "template", templateId: "preset-agent", name: "Agent", actionsAvailable: true }, mappingNoticeRevision: 1, generation: 1 })),
    selectCurrentTemplate: vi.fn(),
    subscribeSceneEvents: vi.fn(async (handler: SceneHandler) => { sceneHandler = handler; return () => { sceneHandler = null; }; }),
    getMappingConfiguration: vi.fn(async () => ({
      menuTemplateSwitchEnabled: false, mappingNoticeEnabled: true, commonMappings: {
        enabled: true,
        actions: {
          ok: {
            single: { type: "shortcut", chord: { keys: ["enter"] } },
            double: { type: "disabled" },
            long: { type: "disabled" },
          },
        },
      },
      applicationBindings: [],
      templates: [userTemplate(), {
        id: "profile-a",
        name: "模板 A",
        mappings: {
          enabled: true,
          actions: {
            ok: {
              single: { type: "shortcut", chord: { keys: ["escape"] } },
              double: { type: "disabled" },
              long: { type: "disabled" },
            },
          },
        },
      }],
      buttonMappingFollowEnabled: true,
    })),
    getButtonMappingSnapshot: vi.fn(async () => ({
      enabled: true,
      gateActive: true,
      observedButtons: [], listenerActive: true,
      swallowedEdges: 3,
      leakedDowns: 0,
      firedGestures: 1,
      lastFired: null,
      lastError: null,
    })),
    getTemplateCatalog: vi.fn(async () => ([
      ...builtinCatalog,
      { id: "scene-copy", name: "我的按键模板", kind: "direct", builtIn: false, buttonMappings: userTemplate().mappings },
      { id: "profile-a", name: "模板 A", kind: "direct", builtIn: false, buttonMappings: { enabled: true, actions: {} } },
    ])),
    copyTemplateCatalogEntry: vi.fn(),
    saveMappingConfiguration: vi.fn(async (configuration: unknown) => configuration),
    setMenuTemplateSwitchEnabled: vi.fn(),
    saveButtonMappings: vi.fn(async (mappings: unknown) => mappings),
    saveButtonMappingTemplate: vi.fn(async (name: string, mappings: unknown) => ({ id: "new-template", name, mappings })),
    updateButtonMappingTemplate: vi.fn(async (templateId: string, mappings: unknown) => ({ id: templateId, name: "模板 A", mappings })),
    resetButtonMappings: vi.fn(async () => ({ enabled: true, actions: {} })),
    scanRegisteredApps: vi.fn(async () => [
      { name: "Registered Example", path: "shell:AppsFolder\\Example!App" },
    ]),
    testButtonMapping: vi.fn(async () => ({
      available: true,
      submittedBatches: 1,
      submittedEvents: 2,
      lastError: null,
    })),
    subscribeButtonEdges: vi.fn(async (handler: EdgeHandler) => {
      edgeHandler = handler;
      return () => {};
    }),
    subscribeButtonGestures: vi.fn(async (handler: GestureHandler) => {
      gestureHandler = handler;
      return () => {};
    }),
    startShortcutCapture: vi.fn(async () => undefined),
    stopShortcutCapture: vi.fn(async () => undefined),
    subscribeShortcutCaptureEdges: vi.fn(async (handler: ShortcutCaptureHandler) => {
      shortcutCaptureHandler = handler;
      return () => {};
    }),
  };
});

import {
  getSceneSnapshot,
  selectCurrentTemplate,
  getMappingConfiguration,
  getButtonMappingSnapshot,
  getTemplateCatalog,
  saveMappingConfiguration,
  setMenuTemplateSwitchEnabled,
  subscribeButtonEdges,
  subscribeButtonGestures,
  saveButtonMappings,
  saveButtonMappingTemplate,
  updateButtonMappingTemplate,
  startShortcutCapture,
  stopShortcutCapture,
} from "../lib/bridge";
import type { ButtonMappings, RuntimeSnapshot } from "../lib/bridge";

const runtime: RuntimeSnapshot = {
  appVersion: "0.1.0",
  platform: {
    platform: "windows",
    windowsApiAvailable: true,
    bleScanAvailable: true,
    bleVoiceReady: true,
    wasapiReady: false,
    rawInputReady: true,
    sendInputReady: true,
    verificationStatus: "测试",
    connection: {
      phase: "ready",
      remoteName: "小米蓝牙语音遥控器",
      remoteModel: "rc003",
      capabilities: null,
      voiceState: "idle",
      decodedSamples: 0,
      generation: 0,
      reconnectAttempt: 0,
      powerNotificationsAvailable: false,
      lastError: null,
    },
    audio: {
      phase: "unsupported",
      selectedEndpointId: null,
      selectedEndpointName: null,
      queuedSamples: 0,
      submittedSamples: 0,
      generation: 0,
      lastError: null,
    },
    rawInput: {
      phase: "ready",
      matchedDeviceCount: 1,
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
      gateActive: true,
      observedButtons: [], listenerActive: true,
      swallowedEdges: 3,
      leakedDowns: 0,
      firedGestures: 1,
      lastFired: null,
      lastError: null,
    },
  },
};

async function mountPage(model: "rc001" | "rc003" | "unknown" = "rc003"): Promise<VueWrapper> {
  // Each mounted page owns its listeners. Reset the captured callbacks so the
  // readiness check cannot be satisfied by a previous page in a navigation loop.
  edgeHandler = null;
  gestureHandler = null;
  const snapshot =
    model === "rc003"
      ? runtime
      : {
          ...runtime,
          platform: {
            ...runtime.platform,
            connection: { ...runtime.platform.connection, remoteModel: model },
          },
        };
  const wrapper = mount(ButtonsPage, { props: { runtime: snapshot } });
  await vi.waitFor(() => {
    if (!edgeHandler || !gestureHandler) throw new Error("事件订阅未完成");
    if (!wrapper.find(".editing-source-picker select").exists()) throw new Error("配置未加载");
  });
  return wrapper;
}

beforeEach(() => {
  shortcutCaptureHandler = null;
  vi.mocked(startShortcutCapture).mockClear();
  vi.mocked(stopShortcutCapture).mockClear();
  edgeHandler = null;
  gestureHandler = null;
  vi.mocked(getMappingConfiguration).mockClear();
  vi.mocked(getTemplateCatalog).mockClear();
  vi.mocked(saveMappingConfiguration).mockClear();
  vi.mocked(setMenuTemplateSwitchEnabled).mockReset();
  vi.mocked(subscribeButtonEdges).mockClear();
  vi.mocked(subscribeButtonGestures).mockClear();
  vi.mocked(saveButtonMappings).mockClear();
  vi.mocked(saveButtonMappingTemplate).mockClear();
  vi.mocked(updateButtonMappingTemplate).mockClear();
});

describe("buttons mapping page", () => {
  it("shows the applied template separately from the editing draft and follows runtime changes", async () => {
    const wrapper = mount(ButtonsPage, { props: { runtime } }); await flushPromises();
    const current = wrapper.get('output[aria-label="当前使用的按键模板"]');
    expect(current.text()).toBe("Agent");
    expect(wrapper.find('.current-template-display select, .current-template-display button, .current-template-display input').exists()).toBe(false);
    const editor = wrapper.get('.editing-source-picker select');
    await editor.setValue("template:profile-a"); await flushPromises();
    expect(current.text()).toBe("Agent");
    expect(selectCurrentTemplate).not.toHaveBeenCalled();
    sceneHandler?.({ type: "mapping_applied", revision: 2, notice: { kind: "template", templateId: "profile-a", name: "模板 A", actionsAvailable: true } });
    await flushPromises();
    expect(current.text()).toBe("模板 A");
    expect((editor.element as HTMLSelectElement).value).toBe("template:profile-a");
    wrapper.unmount();
  });

  it("shows an unconfirmed state on read failure and updates to common only after application", async () => {
    vi.mocked(getSceneSnapshot).mockRejectedValueOnce(new Error("读取失败"));
    const wrapper = mount(ButtonsPage, { props: { runtime } }); await flushPromises();
    const current = wrapper.get('output[aria-label="当前使用的按键模板"]');
    expect(current.text()).toBe("正在确认当前模板…");
    expect(wrapper.text()).toContain("读取失败");
    sceneHandler?.({ type: "mapping_applied", revision: 2, notice: { kind: "common", templateId: null, name: null, actionsAvailable: true } });
    await flushPromises();
    expect(current.text()).toBe("通用配置");
    expect(selectCurrentTemplate).not.toHaveBeenCalled();
    wrapper.unmount();
  });
  it("saves Menu mode through the shared narrow command without changing drafts or confirming pending state", async () => {
    const base = await getMappingConfiguration();
    const wrapper = await mountPage();
    const menu = () => wrapper.findAll(".mapping-card").find(card => card.find("strong").text() === "菜单")!;
    let resolve!: (value: typeof base) => void;
    vi.mocked(setMenuTemplateSwitchEnabled).mockImplementationOnce(() => new Promise(done => { resolve = done; }));
    try {
      const input = wrapper.get('input[aria-label="菜单键切换模板"]');
      expect(menu().get('.mapping-card-title input').element).toBe(input.element);
      expect(menu().find('.menu-mode-lock .readonly-icon').exists()).toBe(false);
      expect(input.element.closest('[aria-disabled="true"]')).toBeNull();
      await menu().find(".mapping-cell").trigger("click");
      wrapper.findComponent({name:"ButtonActionEditor"}).vm.$emit("update", {type:"shortcut", chord:{keys:["f5"]}});
      await flushPromises();
      await input.setValue(true);
      expect(input.element).toHaveProperty("checked", false);
      expect(menu().find('.menu-mode-lock .readonly-icon').exists()).toBe(false);
      expect(input.element).toHaveProperty("disabled", true);
      expect(wrapper.find(".mapping-editor").exists()).toBe(true);
      expect(wrapper.get(".mapping-actions button").element).toHaveProperty("disabled", false);
      await input.trigger("change");
      expect(setMenuTemplateSwitchEnabled).toHaveBeenCalledExactlyOnceWith(true);
      resolve({...base, menuTemplateSwitchEnabled:true, commonMappings:{enabled:true,actions:{}}});
      await flushPromises();
      expect(input.element).toHaveProperty("checked", true);
      expect(menu().find('.menu-mode-lock .readonly-icon').exists()).toBe(true);
      expect(input.element.closest('[aria-disabled="true"]')).toBeNull();
      expect(menu().get('.mapping-cells').attributes('aria-disabled')).toBe('true');
      expect(input.element).toHaveProperty("disabled", false);
      expect(wrapper.find(".mapping-editor").exists()).toBe(false);
      expect(menu().classes()).not.toContain("selected");
      vi.mocked(setMenuTemplateSwitchEnabled).mockResolvedValueOnce({...base,menuTemplateSwitchEnabled:false});
      await input.setValue(false);
      await flushPromises();
      expect(menu().find(".mapping-cell").element).toHaveProperty("disabled", false);
      expect(menu().find('.menu-mode-lock .readonly-icon').exists()).toBe(false);
      expect(menu().text()).toContain("F5");
      expect(saveMappingConfiguration).not.toHaveBeenCalled();
      expect(saveButtonMappings).not.toHaveBeenCalled();
    } finally { wrapper.unmount(); }
  });

  it("keeps the Menu header control separate from mapping selection and exposes its explanation on focus", async () => {
    const base = await getMappingConfiguration();
    const wrapper = await mountPage();
    const root = wrapper.element;
    document.body.appendChild(root);
    let resolve!: (value: typeof base) => void;
    vi.mocked(setMenuTemplateSwitchEnabled).mockImplementationOnce(() => new Promise(done => { resolve = done; }));
    try {
      const menu = wrapper.findAll('.mapping-card').find(card => card.find('strong').text() === '菜单')!;
      const input = menu.get<HTMLInputElement>('.mapping-card-title input');
      expect(input.attributes('aria-describedby')).toContain('menu-mode-tooltip');
      expect(menu.get('[role="tooltip"]').text()).toBe('启用后，菜单键用于切换模板；关闭后可自定义。');
      expect(wrapper.get('.voice-card').find('input').exists()).toBe(false);
      for (const key of [' ', 'Enter']) {
        const event = new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true });
        const bubbled = vi.fn();
        menu.element.addEventListener('keydown', bubbled, { once: true });
        input.element.dispatchEvent(event);
        expect(bubbled).not.toHaveBeenCalled();
        expect(event.defaultPrevented).toBe(false);
        menu.element.removeEventListener('keydown', bubbled);
      }
      input.element.click();
      await flushPromises();
      expect(setMenuTemplateSwitchEnabled).toHaveBeenCalledExactlyOnceWith(true);
      expect(input.element.checked).toBe(false);
      expect(menu.classes()).not.toContain('selected');
      expect(wrapper.find('.mapping-editor').exists()).toBe(false);
      resolve({ ...base, menuTemplateSwitchEnabled: true });
      await flushPromises();
      expect(input.element.checked).toBe(true);
      expect(menu.classes()).not.toContain('selected');
    } finally { wrapper.unmount(); root.remove(); }
  });

  it("ignores a pre-save Menu poll and still reads later changes from the shared setting", async () => {
    vi.useFakeTimers();
    const base = await getMappingConfiguration();
    const wrapper = await mountPage();
    let finishPoll!: (value: typeof base) => void;
    try {
      vi.mocked(getMappingConfiguration).mockImplementationOnce(() => new Promise(done => { finishPoll=done; }));
      await vi.advanceTimersByTimeAsync(1000);
      vi.mocked(setMenuTemplateSwitchEnabled).mockResolvedValueOnce({...base,menuTemplateSwitchEnabled:true});
      const input=wrapper.get('input[aria-label="菜单键切换模板"]');
      await input.setValue(true);
      await flushPromises();
      finishPoll({...base,menuTemplateSwitchEnabled:false});
      await flushPromises();
      expect(input.element).toHaveProperty("checked", true);
      vi.mocked(getMappingConfiguration).mockResolvedValueOnce({...base,menuTemplateSwitchEnabled:false});
      await vi.advanceTimersByTimeAsync(1000);
      expect(input.element).toHaveProperty("checked", false);
      expect(setMenuTemplateSwitchEnabled).toHaveBeenCalledTimes(1);
    } finally { wrapper.unmount(); vi.useRealTimers(); }
  });

  it("reads back a failed Menu save and keeps it unavailable if the actual state cannot be read", async () => {
    vi.useFakeTimers();
    const base = await getMappingConfiguration();
    const wrapper = await mountPage();
    try {
      const input=wrapper.get('input[aria-label="菜单键切换模板"]');
      vi.mocked(setMenuTemplateSwitchEnabled).mockRejectedValueOnce(new Error("response lost"));
      vi.mocked(getMappingConfiguration).mockResolvedValueOnce({...base,menuTemplateSwitchEnabled:true});
      await input.setValue(true);
      await flushPromises();
      expect(input.element).toHaveProperty("checked", true);
      expect(wrapper.get('.menu-mode-feedback [role="alert"]').text()).toContain("保存未确认");
      vi.mocked(setMenuTemplateSwitchEnabled).mockRejectedValueOnce(new Error("save failed"));
      vi.mocked(getMappingConfiguration).mockRejectedValueOnce(new Error("read failed"));
      await input.setValue(false);
      await flushPromises();
      expect(input.element).toHaveProperty("disabled", true);
      expect(wrapper.text()).toContain("菜单功能读取失败");
      vi.mocked(getMappingConfiguration).mockResolvedValueOnce(base);
      await vi.advanceTimersByTimeAsync(1000);
      expect(input.element).toHaveProperty("checked", false);
      expect(input.element).toHaveProperty("disabled", false);
      expect(saveMappingConfiguration).not.toHaveBeenCalled();
    } finally { wrapper.unmount(); vi.useRealTimers(); }
  });

  it("does not apply a late Menu save reply to a new page or restart reads after unmount", async () => {
    const base=await getMappingConfiguration();
    const wrapper=await mountPage();
    let resolve!: (value: typeof base) => void;
    vi.mocked(setMenuTemplateSwitchEnabled).mockImplementationOnce(() => new Promise(done => { resolve=done; }));
    await wrapper.get('input[aria-label="菜单键切换模板"]').setValue(true);
    wrapper.unmount();
    const next=await mountPage();
    const reads=vi.mocked(getMappingConfiguration).mock.calls.length;
    try {
      resolve({...base,menuTemplateSwitchEnabled:true});
      await flushPromises();
      expect(next.get('input[aria-label="菜单键切换模板"]').element).toHaveProperty("checked", false);
      expect(getMappingConfiguration).toHaveBeenCalledTimes(reads);
    } finally { next.unmount(); }
  });

  it("shows reserved Menu behavior and blocks clicks and unlocked listener selection without changing mappings", async () => {
    const base = await getMappingConfiguration();
    vi.mocked(getMappingConfiguration).mockResolvedValueOnce({ ...base, menuTemplateSwitchEnabled: true, buttonMappingFollowEnabled: false });
    const wrapper = await mountPage();
    const menu = () => wrapper.findAll(".mapping-card").find(card => card.find("strong").text() === "菜单")!;
    try {
      expect(menu().text()).toContain("已启用模板切换");
      expect(menu().text()).toContain("打开 / 取消");
      expect(menu().text()).toContain("按单击处理");
      expect(menu().text()).toContain("切换保存选项");
      expect(menu().text()).not.toContain("未设置");
      for (const cell of menu().findAll(".mapping-cell")) {
        expect(cell.element).toHaveProperty("disabled", true);
        await cell.trigger("click");
      }
      await menu().trigger("click");
      await wrapper.findAll(".toggle-row").find(row => row.text().includes("锁定当前按键"))!.find("input").setValue(false);
      edgeHandler!({ button: "menu", isPressed: true });
      await flushPromises();
      expect(menu().classes()).toContain("active");
      expect(menu().classes()).not.toContain("selected");
      expect(wrapper.find(".mapping-editor").exists()).toBe(false);
      await wrapper.find(".editing-source-picker select").setValue("template:preset-agent");
      expect(menu().find(".mapping-cell").element).toHaveProperty("disabled", true);
      expect(saveButtonMappings).not.toHaveBeenCalled();
      expect(updateButtonMappingTemplate).not.toHaveBeenCalled();
    } finally { wrapper.unmount(); }
  });

  it("closes only a newly reserved Menu editor, rejects its late update, and restores its unsaved draft when released", async () => {
    vi.useFakeTimers();
    const base = await getMappingConfiguration();
    const wrapper = await mountPage();
    const menu = () => wrapper.findAll(".mapping-card").find(card => card.find("strong").text() === "菜单")!;
    try {
      await wrapper.find(".editing-source-picker select").setValue("template:preset-agent");
      await menu().find(".mapping-cell").trigger("click");
      const editor = wrapper.findComponent({ name: "ButtonActionEditor" });
      const custom = { type: "shortcut", chord: { keys: ["f5"] } };
      editor.vm.$emit("update", custom);
      await flushPromises();
      vi.mocked(getMappingConfiguration).mockResolvedValueOnce({ ...base, menuTemplateSwitchEnabled: true });
      await vi.advanceTimersByTimeAsync(1000);
      expect(wrapper.find(".mapping-editor").exists()).toBe(false);
      editor.vm.$emit("update", { type: "disabled" });
      await vi.advanceTimersByTimeAsync(1000);
      expect(menu().find(".mapping-cell").element).toHaveProperty("disabled", false);
      expect(menu().text()).toContain("F5");
      await menu().find(".mapping-cell").trigger("click");
      expect(wrapper.find(".mapping-editor").exists()).toBe(true);
      await wrapper.findAll(".mapping-actions button").find(button => button.text() === "保存当前配置")!.trigger("click");
      await flushPromises();
      expect(updateButtonMappingTemplate).toHaveBeenCalledWith("preset-agent", expect.objectContaining({actions: expect.objectContaining({menu: expect.objectContaining({single: custom})})}));
      expect(saveMappingConfiguration).not.toHaveBeenCalled();
    } finally { wrapper.unmount(); vi.mocked(getMappingConfiguration).mockReset().mockResolvedValue(base); vi.useRealTimers(); }
  });

  it("retains other key drafts and the open editor during Menu ownership changes", async () => {
    vi.useFakeTimers();
    const base = await getMappingConfiguration();
    const wrapper = await mountPage();
    try {
      await wrapper.findAll(".mapping-card").find(card => card.find("strong").text() === "电源")!.find(".mapping-cell").trigger("click");
      const editor = wrapper.findComponent({ name: "ButtonActionEditor" });
      editor.vm.$emit("update", {type:"shortcut", chord:{keys:["tab"]}});
      vi.mocked(getMappingConfiguration).mockResolvedValueOnce({...base, menuTemplateSwitchEnabled:true});
      await vi.advanceTimersByTimeAsync(1000);
      expect(wrapper.findComponent({name:"ButtonActionEditor"}).element).toBe(editor.element);
      expect(wrapper.find(".mapping-editor h2").text()).toContain("电源");
      expect(wrapper.find(".mapping-editor").text()).toContain("Tab");
    } finally { wrapper.unmount(); vi.mocked(getMappingConfiguration).mockReset().mockResolvedValue(base); vi.useRealTimers(); }
  });

  it("does not expose Menu editing before its setting loads or after a read failure, and ignores a late read after unmount", async () => {
    let resolve!: (value: Awaited<ReturnType<typeof getMappingConfiguration>>) => void;
    const base = await getMappingConfiguration();
    vi.mocked(getMappingConfiguration).mockImplementationOnce(() => new Promise(value => { resolve = value; }));
    const wrapper = mount(ButtonsPage, {props:{runtime}});
    const menu = wrapper.findAll(".mapping-card").find(card => card.find("strong").text() === "菜单")!;
    expect(menu.find(".mapping-cell").element).toHaveProperty("disabled", true);
    expect(menu.text()).not.toContain("未设置");
    wrapper.unmount();
    resolve({...base, menuTemplateSwitchEnabled:false});
    await flushPromises();
    expect(subscribeButtonEdges).not.toHaveBeenCalled();
    vi.mocked(getMappingConfiguration).mockRejectedValueOnce(new Error("unavailable"));
    const failed = await mountPage();
    try {
      const card=failed.findAll(".mapping-card").find(item => item.find("strong").text() === "菜单")!;
      expect(card.find(".mapping-cell").element).toHaveProperty("disabled", true);
      expect(card.text()).toContain("读取失败");
      expect(card.text()).not.toContain("未设置");
    } finally { failed.unmount(); }
  });

  it("adds scanned apps to the library without changing button bindings", async () => {
    const wrapper = await mountPage();
    await flushPromises();
    const powerCard = wrapper
      .findAll(".mapping-card")
      .find((item) => item.find(".mapping-card-title strong").text() === "电源")!;
    await powerCard.findAll(".mapping-cell")[0]!.trigger("click");
    await wrapper
      .findAll("button")
      .find((button) => button.text() === "扫描本机应用")!
      .trigger("click");
    await flushPromises();
    await wrapper.get('input[aria-label="全选当前结果"]').setValue(true);
    await wrapper.get(".registered-apps-dialog .primary-button").trigger("click");
    await flushPromises();

    expect(saveButtonMappings).not.toHaveBeenCalled();
    await wrapper.findAll("button").find(button => button.text() === "保存当前配置")!.trigger("click");
    await flushPromises();
    const saved = vi.mocked(saveButtonMappings).mock.lastCall![0];
    expect(saved.applications).toEqual([
      { name: "Registered Example", path: "shell:AppsFolder\\Example!App" },
    ]);
    expect(saved.actions.power).toBeUndefined();
    expect(saved.actions.ok?.single).toEqual({
      type: "shortcut",
      chord: { keys: ["enter"] },
    });
    wrapper.unmount();
  });

  it("uses one measured image frame for connector starts and the 13-key layout", async () => {
    const wrapper = await mountPage();
    const leftCards = wrapper.findAll(".mapping-card.left");
    const rightCards = wrapper.findAll(".mapping-card.right");
    expect(leftCards.map((card) => card.find("strong").text())).toEqual(["电源", "上", "左", "返回", "主页", "菜单"]);
    expect(rightCards.map((card) => card.find("strong").text())).toEqual(["右", "确定", "下", "音量+", "音量−", "TV"]);
    expect(wrapper.findAll(".mapping-card")).toHaveLength(13);
    expect(wrapper.find(".voice-card.center").exists()).toBe(true);
    expect(wrapper.find(".voice-card").attributes("style")).toContain("top: 8px");
    expect(wrapper.find(".remote-photo").attributes("style")).toContain("top: 115px");
    const canvasStyle = (wrapper.find(".mapping-canvas").element as HTMLElement).style;
    const cardHeight = Number.parseFloat(canvasStyle.getPropertyValue("--mapping-card-height"));
    const voiceStyle = (wrapper.find(".voice-card").element as HTMLElement).style;
    expect(cardHeight).toBeGreaterThan(0);
    expect(voiceStyle.width).toBe((leftCards[0]!.element as HTMLElement).style.width);
    let previousBottom = Number.parseFloat(voiceStyle.top) + cardHeight;
    for (let index = 0; index < leftCards.length; index += 1) {
      expect(leftCards[index]!.attributes("style")).toContain(rightCards[index]!.attributes("style")!.match(/top: [^;]+/)![0]);
      const top = Number.parseFloat((leftCards[index]!.element as HTMLElement).style.top);
      expect(top).toBeGreaterThan(previousBottom);
      previousBottom = top + cardHeight;
      expect(previousBottom).toBeLessThan(Number.parseFloat(canvasStyle.height));
      const endpoint = wrapper.findAll(".mapping-connections path")[index]!.attributes("d")!.match(/ ([0-9.]+) ([0-9.]+)$/)!;
      expect(Number(endpoint[2])).toBeCloseTo(top + cardHeight / 2, 1);
    }
    expect(wrapper.find(".voice-card").findAll(".mapping-cell")).toHaveLength(0);

    const start = wrapper.find(".mapping-connections path").attributes("d")!.match(/^M ([0-9.]+) ([0-9.]+)/)!;
    const dotStyle = (wrapper.find(".anchor-dot").element as HTMLElement).style;
    const dotLeft = Number.parseFloat(dotStyle.left);
    const dotTop = Number.parseFloat(dotStyle.top);
    expect(Number(start[1])).toBeCloseTo(dotLeft + 4, 1);
    expect(Number(start[2])).toBeCloseTo(dotTop + 4, 1);
  });

  it("shows every builtin in the ordinary fixed-key editor without a region selector", async () => {
    const wrapper = await mountPage();
    await wrapper.find(".editing-source-picker select").setValue("template:preset-agent"); await flushPromises();
    expect(wrapper.text()).not.toContain("区域动作");
    expect(wrapper.findAll(".editing-source-picker select")).toHaveLength(1);
    expect(wrapper.findAll(".mapping-cell")[0]!.element).toHaveProperty("disabled", false);
    expect(wrapper.findAll(".mapping-card").find(card => card.text().includes("确定"))!.text()).toContain("Enter");
    expect(updateButtonMappingTemplate).not.toHaveBeenCalled();
  });

  it("offers system task actions and explains native Confirm limits while keeping plain Enter immediate", async () => {
    const wrapper = await mountPage();
    await wrapper.find(".editing-source-picker select").setValue("template:preset-agent"); await flushPromises();
    const confirmCard = wrapper.findAll(".mapping-card").find(card => card.text().includes("确定"))!;
    await confirmCard.findAll(".mapping-cell")[0]!.trigger("click");
    expect(wrapper.find(".editor-note[role='note']").exists()).toBe(false);
    await confirmCard.findAll(".mapping-cell")[2]!.trigger("click");
    expect(wrapper.get(".editor-note[role='note']").text()).toContain("首次原生 Enter");
    const tvCard = wrapper.findAll(".mapping-card").find(card => card.text().includes("TV"))!;
    await tvCard.findAll(".mapping-cell")[0]!.trigger("click");
    const editor = wrapper.findComponent({ name: "ButtonActionEditor" });
    await editor.findAll("button").find(button => button.text() === "任务切换")!.trigger("click");
    expect(editor.emitted("update")?.at(-1)).toEqual([{ type: "task_switch", view: "applications" }]);
    await editor.findAll("button").find(button => button.text() === "任务视图")!.trigger("click");
    expect(editor.emitted("update")?.at(-1)).toEqual([{ type: "task_switch", view: "desktops" }]);
    wrapper.unmount();
  });

  it("requires an explicit discard before opening another template over a dirty direct draft", async () => {
    const confirm = vi.spyOn(window, "confirm");
    const wrapper = await mountPage();
    const source = wrapper.find(".editing-source-picker select");
    const okCard = wrapper.findAll(".mapping-card").find((card) => card.text().includes("确定"))!;
    await okCard.findAll(".mapping-cell")[0]!.trigger("click");
    wrapper.findComponent({ name: "ButtonActionEditor" }).vm.$emit("update", {
      type: "shortcut", chord: { keys: ["space"] },
    });
    confirm.mockReturnValueOnce(false).mockReturnValueOnce(false);
    await source.setValue("template:preset-agent");
    await flushPromises();
    expect((source.element as HTMLSelectElement).value).toBe("common");

    confirm.mockReturnValueOnce(false).mockReturnValueOnce(true);
    await source.setValue("template:preset-agent");
    await flushPromises();
    expect((source.element as HTMLSelectElement).value).toBe("template:preset-agent");
    expect(saveButtonMappings).not.toHaveBeenCalled();
    confirm.mockRestore();
  });

  it("renders the 12 documented physical button cards, the voice card and 36 trigger cells", async () => {
    const wrapper = await mountPage();
    expect(wrapper.findAll(".mapping-card")).toHaveLength(13);
    expect(wrapper.findAll(".mapping-cell")).toHaveLength(36);
    const voiceCard = wrapper.find(".voice-card");
    expect(voiceCard.text()).toContain("语音键");
    expect(voiceCard.text()).toContain("按住说话");
    expect(wrapper.text()).not.toContain("静音（设备支持时）");
    const selectors = wrapper.findAll(".mapping-header-controls select");
    expect(selectors).toHaveLength(1);
  });

  it("does not register listeners or polling after unmounting during initial load", async () => {
    let resolveMappings!: (value: Awaited<ReturnType<typeof getMappingConfiguration>>) => void;
    const pendingMappings = new Promise<Awaited<ReturnType<typeof getMappingConfiguration>>>(
      (resolve) => {
        resolveMappings = resolve;
      },
    );
    vi.mocked(getMappingConfiguration).mockImplementationOnce(() => pendingMappings);
    const intervalSpy = vi.spyOn(window, "setInterval");

    const wrapper = mount(ButtonsPage, { props: { runtime } });
    await flushPromises();
    wrapper.unmount();
    resolveMappings({ menuTemplateSwitchEnabled: false, mappingNoticeEnabled: true, commonMappings: { enabled: true, actions: {} }, templates: [], applicationBindings: [], buttonMappingFollowEnabled: false, });
    await flushPromises();

    expect(subscribeButtonEdges).not.toHaveBeenCalled();
    expect(subscribeButtonGestures).not.toHaveBeenCalled();
    expect(intervalSpy).not.toHaveBeenCalled();
    intervalSpy.mockRestore();
  });

  it("immediately releases a listener that resolves after the page is unmounted", async () => {
    let resolveUnlisten!: (unlisten: () => void) => void;
    const pendingUnlisten = new Promise<() => void>((resolve) => {
      resolveUnlisten = resolve;
    });
    vi.mocked(subscribeButtonEdges).mockImplementationOnce(() => pendingUnlisten);
    const stopEdges = vi.fn();

    const wrapper = mount(ButtonsPage, { props: { runtime } });
    await vi.waitFor(() => expect(subscribeButtonEdges).toHaveBeenCalledOnce());
    const intervalSpy = vi.spyOn(window, "setInterval");
    wrapper.unmount();
    resolveUnlisten(stopEdges);
    await flushPromises();

    expect(stopEdges).toHaveBeenCalledOnce();
    expect(subscribeButtonGestures).not.toHaveBeenCalled();
    expect(intervalSpy).not.toHaveBeenCalled();
    intervalSpy.mockRestore();
  });

  it("keeps global mapping save on the dedicated buttons page", async () => {
    const wrapper = await mountPage();
    const button = (label: string) =>
      wrapper.findAll(".mapping-actions button").find((item) => item.text() === label)!;

    await button("保存当前配置").trigger("click");
    await vi.waitFor(() => expect(saveButtonMappings).toHaveBeenCalled());
    await flushPromises();
    expect(wrapper.text()).toContain("配置已保存并生效");

    expect(wrapper.text()).toContain("保存为模板");
    const powerCell = wrapper
      .findAll(".mapping-card")
      .find((card) => card.text().includes("电源"))!
      .findAll(".mapping-cell")[0]!;
    expect((powerCell.element as HTMLButtonElement).disabled).toBe(false);
  });

  it("saves a named template only after dialog confirmation and keeps failures editable", async () => {
    const wrapper = await mountPage();
    const open = wrapper.findAll(".mapping-actions button").find((item) => item.text() === "保存为模板")!;
    await open.trigger("click");
    expect(saveButtonMappingTemplate).not.toHaveBeenCalled();
    await wrapper.find("[role='dialog'] input").setValue("会议控制");
    await wrapper.findAll("[role='dialog'] button").find((item) => item.text() === "保存模板")!.trigger("click");
    await flushPromises();
    expect(saveButtonMappingTemplate).toHaveBeenCalledWith("会议控制", expect.objectContaining({ enabled: true }));
    expect(wrapper.find("[role='dialog']").exists()).toBe(false);

    vi.mocked(saveButtonMappingTemplate).mockRejectedValueOnce(new Error("名称重复"));
    await open.trigger("click");
    await wrapper.find("[role='dialog'] input").setValue("会议控制");
    await wrapper.findAll("[role='dialog'] button").find((item) => item.text() === "保存模板")!.trigger("click");
    await flushPromises();
    expect(wrapper.find("[role='dialog']").exists()).toBe(true);
    expect(wrapper.text()).toContain("名称重复");
    expect((wrapper.find("[role='dialog'] input").element as HTMLInputElement).value).toBe("会议控制");
  });

  it("keeps the open action editor after saving the global mapping", async () => {
    const wrapper = await mountPage();
    const powerCell = wrapper
      .findAll(".mapping-card")
      .find((card) => card.text().includes("电源"))!
      .findAll(".mapping-cell")[0]!;
    await powerCell.trigger("click");
    expect(wrapper.find(".mapping-editor").exists()).toBe(true);

    const save = wrapper.findAll(".mapping-actions button").find((item) => item.text() === "保存当前配置")!;
    await save.trigger("click");
    await wrapper.vm.$nextTick();
    expect(wrapper.find(".mapping-editor").exists()).toBe(true);
  });

  it("loads and saves the explicitly selected template without writing common mappings", async () => {
    const wrapper = await mountPage();
    const source = wrapper.find(".editing-source-picker select");
    await source.setValue("template:profile-a");
    await flushPromises();
    const okCard = wrapper.findAll(".mapping-card").find((card) => card.text().includes("确定"))!;
    expect(okCard.text()).toContain("Esc");
    await okCard.findAll(".mapping-cell")[0]!.trigger("click");
    wrapper.findComponent({ name: "ButtonActionEditor" }).vm.$emit("update", {
      type: "shortcut",
      chord: { keys: ["space"] },
    });
    await wrapper.findAll(".mapping-actions button").find((item) => item.text() === "保存当前配置")!.trigger("click");
    await vi.waitFor(() => expect(updateButtonMappingTemplate).toHaveBeenCalledOnce());
    expect(updateButtonMappingTemplate).toHaveBeenCalledWith("profile-a", expect.objectContaining({
      actions: expect.objectContaining({
        ok: expect.objectContaining({ single: { type: "shortcut", chord: { keys: ["space"] } } }),
      }),
    }));
    expect(saveButtonMappings).not.toHaveBeenCalled();
  });

  it("keeps the selected template and its draft when runtime foreground state changes", async () => {
    const wrapper = await mountPage();
    const source = wrapper.find(".editing-source-picker select");
    await source.setValue("template:profile-a");
    const okCard = wrapper.findAll(".mapping-card").find((card) => card.text().includes("确定"))!;
    await okCard.findAll(".mapping-cell")[0]!.trigger("click");
    wrapper.findComponent({ name: "ButtonActionEditor" }).vm.$emit("update", {
      type: "shortcut",
      chord: { keys: ["space"] },
    });

    await wrapper.setProps({
      runtime: {
        ...runtime,
        platform: {
          ...runtime.platform,
          buttonMapping: {
            ...runtime.platform.buttonMapping,
            firedGestures: runtime.platform.buttonMapping.firedGestures + 1,
            lastFired: { button: "down", trigger: "single" },
          },
        },
      },
    });

    expect((source.element as HTMLSelectElement).value).toBe("template:profile-a");
    expect(okCard.text()).toContain("空格");
    expect(wrapper.text()).toContain("未保存更改");
    expect(saveButtonMappings).not.toHaveBeenCalled();
    expect(updateButtonMappingTemplate).not.toHaveBeenCalled();
  });

  it("requires an explicit save, discard, or cancel before changing editing targets", async () => {
    const confirm = vi.spyOn(window, "confirm");
    const wrapper = await mountPage();
    const source = wrapper.find(".editing-source-picker select");
    const okCard = wrapper.findAll(".mapping-card").find((card) => card.text().includes("确定"))!;
    await okCard.findAll(".mapping-cell")[0]!.trigger("click");
    wrapper.findComponent({ name: "ButtonActionEditor" }).vm.$emit("update", {
      type: "shortcut",
      chord: { keys: ["space"] },
    });
    confirm.mockReturnValueOnce(false).mockReturnValueOnce(false);
    await source.setValue("template:profile-a");
    await flushPromises();
    expect((source.element as HTMLSelectElement).value).toBe("common");
    expect(saveButtonMappings).not.toHaveBeenCalled();

    confirm.mockReturnValueOnce(false).mockReturnValueOnce(true);
    await source.setValue("template:profile-a");
    await flushPromises();
    expect((source.element as HTMLSelectElement).value).toBe("template:profile-a");
    expect(okCard.text()).toContain("Esc");
    confirm.mockRestore();
  });

  it("disables target switching while a template save is pending", async () => {
    let resolveSave!: (value: { id: string; name: string; mappings: { enabled: boolean; actions: Record<string, unknown> } }) => void;
    vi.mocked(updateButtonMappingTemplate).mockImplementationOnce(
      () => new Promise((resolve) => { resolveSave = resolve; }),
    );
    const wrapper = await mountPage();
    const source = wrapper.find(".editing-source-picker select");
    await source.setValue("template:profile-a");
    const okCard = wrapper.findAll(".mapping-card").find((card) => card.text().includes("确定"))!;
    await okCard.findAll(".mapping-cell")[0]!.trigger("click");
    wrapper.findComponent({ name: "ButtonActionEditor" }).vm.$emit("update", {
      type: "shortcut", chord: { keys: ["space"] },
    });
    await wrapper.findAll(".mapping-actions button").find((item) => item.text() === "保存当前配置")!.trigger("click");
    await flushPromises();
    expect((source.element as HTMLSelectElement).disabled).toBe(true);
    expect(okCard.findAll(".mapping-cell")[0]!.element).toHaveProperty("disabled", true);
    wrapper.findComponent({ name: "ButtonActionEditor" }).vm.$emit("update", {
      type: "shortcut", chord: { keys: ["enter"] },
    });
    resolveSave({ id: "profile-a", name: "模板 A", mappings: {
      enabled: true,
      actions: { ok: { single: { type: "shortcut", chord: { keys: ["space"] } }, double: { type: "disabled" }, long: { type: "disabled" } } },
    } });
    await flushPromises();
    expect((source.element as HTMLSelectElement).disabled).toBe(false);
    expect(okCard.text()).toContain("空格");
    expect(okCard.text()).not.toContain("Enter");
  });

  it("marks configured cells and opens the editor with the correct target", async () => {
    const wrapper = await mountPage();
    const okCard = wrapper
      .findAll(".mapping-card")
      .find((card) => card.text().includes("确定"));
    expect(okCard).toBeDefined();
    expect(okCard!.text()).toContain("Enter");

    const singleCell = okCard!.findAll(".mapping-cell")[0]!;
    expect(singleCell.classes()).toContain("set");
    await singleCell.trigger("click");
    expect(wrapper.find(".mapping-editor").text()).toContain("确定 · 单击");
  });

  it("keeps action edits as a draft until the selected target is explicitly saved", async () => {
    const wrapper = await mountPage();
    const powerCard = wrapper
      .findAll(".mapping-card")
      .find((card) => card.text().includes("电源"));
    await powerCard!.findAll(".mapping-cell")[2]!.trigger("click");

    const editor = wrapper.find(".mapping-editor");
    expect(editor.text()).toContain("电源 · 长按");
    // 点击 Esc 只修改当前编辑目标草稿。
    const chips = editor.findAll(".chip");
    const escapeChip = chips.find((chip) => chip.text() === "Esc");
    await escapeChip!.trigger("click");
    expect(saveButtonMappings).not.toHaveBeenCalled();
    await wrapper.findAll(".mapping-actions button").find((item) => item.text() === "保存当前配置")!.trigger("click");
    await vi.waitFor(() => expect(saveButtonMappings).toHaveBeenCalledOnce());
    const saved = vi.mocked(saveButtonMappings).mock.calls[0]![0] as {
      actions: Record<string, { long: { type: string; chord?: { keys: string[] } } }>;
    };
    expect(saved.actions.power!.long.type).toBe("shortcut");
    expect(saved.actions.power!.long.chord!.keys).toEqual(["escape"]);
    await flushPromises();

    // 禁用按键按钮同样只修改草稿。
    const disableButton = wrapper
      .findAll("button")
      .find((button) => button.text() === "禁用按键");
    expect(disableButton).toBeDefined();
    await disableButton!.trigger("click");
    expect(saveButtonMappings).toHaveBeenCalledOnce();
    expect(wrapper.text()).toContain("未保存更改");
  });

  it("configures mouse actions with independent validated amounts", async () => {
    const wrapper = await mountPage();
    await flushPromises();
    const powerCard = wrapper
      .findAll(".mapping-card")
      .find((card) => card.text().includes("电源"))!;
    await powerCard.findAll(".mapping-cell")[0]!.trigger("click");
    const choose = async (label: string) => {
      await wrapper
        .findAll(".mapping-editor button")
        .find((button) => button.text() === label)!
        .trigger("click");
      await flushPromises();
    };

    const saveDraft = async () => {
      await wrapper.findAll("button").find(button => button.text() === "保存当前配置")!.trigger("click");
      await flushPromises();
    };
    await choose("滚轮向下");
    await wrapper.get('input[aria-label="每次滚动格数"]').setValue("5");
    await flushPromises();
    expect(saveButtonMappings).not.toHaveBeenCalled();
    await saveDraft();
    expect(vi.mocked(saveButtonMappings).mock.lastCall![0].actions.power!.single).toEqual({
      type: "scroll",
      direction: "down",
      steps: 5,
    });

    const saveCount = vi.mocked(saveButtonMappings).mock.calls.length;
    await wrapper.get('input[aria-label="每次滚动格数"]').setValue("101");
    await flushPromises();
    expect(vi.mocked(saveButtonMappings).mock.calls).toHaveLength(saveCount);
    expect(wrapper.text()).toContain("请输入 1 到 100 之间的整数");

    await choose("左键双击");
    await saveDraft();
    expect(vi.mocked(saveButtonMappings).mock.lastCall![0].actions.power!.single).toEqual({
      type: "mouse_click",
      kind: "double_left",
    });
    wrapper.unmount();
  });

  it("records a physical Win+L chord directly by default", async () => {
    const wrapper = await mountPage();
    const powerCard = wrapper
      .findAll(".mapping-card")
      .find((card) => card.text().includes("电源"))!;
    await powerCard.findAll(".mapping-cell")[0]!.trigger("click");
    const captureButton = wrapper
      .findAll(".mapping-editor .chip")
      .find((button) => button.text().includes("录入自定义快捷键"))!;
    await captureButton.trigger("click");

    shortcutCaptureHandler!({ key: "left_windows", isPressed: true });
    shortcutCaptureHandler!({ key: "l", isPressed: true });
    shortcutCaptureHandler!({ key: "l", isPressed: false });
    await flushPromises();
    expect(stopShortcutCapture).not.toHaveBeenCalled();
    shortcutCaptureHandler!({ key: "left_windows", isPressed: false });
    await vi.waitFor(() => expect(stopShortcutCapture).toHaveBeenCalledOnce());
    expect(saveButtonMappings).not.toHaveBeenCalled();
    await wrapper.findAll("button").find(button => button.text() === "保存当前配置")!.trigger("click");
    await flushPromises();
    const saved = vi.mocked(saveButtonMappings).mock.calls.at(-1)?.[0] as ButtonMappings;
    expect(saved.actions.power?.single).toEqual({
      type: "shortcut",
      chord: { keys: ["left_windows", "l"] },
    });
  });

  it("records Win+L safely after the user enables fallback mode", async () => {
    const wrapper = await mountPage();
    const powerCard = wrapper
      .findAll(".mapping-card")
      .find((card) => card.text().includes("电源"))!;
    await powerCard.findAll(".mapping-cell")[0]!.trigger("click");
    const safeToggle = wrapper.find(".safe-capture-toggle input");
    const shortcutRow = wrapper.find(".custom-shortcut-row");
    const toggleRow = wrapper.find(".safe-capture-toggle");
    expect(shortcutRow.element.nextElementSibling).toBe(toggleRow.element);
    expect(safeToggle.classes()).toContain("toggle-input");
    expect(safeToggle.element.nextElementSibling?.textContent).toContain(
      "直接录入无法完成或会触发系统动作时再开启",
    );
    expect((safeToggle.element as HTMLInputElement).checked).toBe(false);
    await safeToggle.setValue(true);
    const captureButton = wrapper
      .findAll(".mapping-editor .chip")
      .find((button) => button.text().includes("录入自定义快捷键"))!;
    await captureButton.trigger("click");
    await vi.waitFor(() => expect(startShortcutCapture).toHaveBeenCalledOnce());

    const leftWin = wrapper
      .findAll(".capture-modifiers .chip")
      .find((button) => button.text() === "左 Win")!;
    await leftWin.trigger("click");
    shortcutCaptureHandler!({ key: "l", isPressed: true });
    expect(saveButtonMappings).not.toHaveBeenCalled();
    await vi.waitFor(() =>
      expect(wrapper.text()).toContain("已录入 左 Win + L，松开全部按键后完成"),
    );
    shortcutCaptureHandler!({ key: "l", isPressed: false });
    await vi.waitFor(() => expect(stopShortcutCapture).toHaveBeenCalledOnce());
    await vi.waitFor(() => expect(wrapper.text()).toContain("快捷键已录入：左 Win + L"));
    await wrapper.findAll("button").find(button => button.text() === "保存当前配置")!.trigger("click");
    await flushPromises();
    expect(vi.mocked(saveButtonMappings).mock.lastCall![0].actions.power?.single).toEqual({type: "shortcut", chord: {keys: ["left_windows", "l"]}});
    wrapper.unmount();
  });

  it("highlights the card for a pressed physical button and clears it on release", async () => {
    const wrapper = await mountPage();
    const upCard = () =>
      wrapper.findAll(".mapping-card").find((card) => card.text().includes("上"));
    expect(upCard()!.classes()).not.toContain("active");

    edgeHandler!({ button: "up", isPressed: true });
    await vi.waitFor(() => {
      if (!upCard()!.classes().includes("active")) throw new Error("未高亮");
    });
    edgeHandler!({ button: "up", isPressed: false });
    await vi.waitFor(() => {
      if (upCard()!.classes().includes("active")) throw new Error("未解除高亮");
    });
  });

  it("keeps the selection locked while pressing the remote unless unlocked", async () => {
    const wrapper = await mountPage();
    // 默认锁定：按下"返回"不改变当前选中（未选中任何键时仍为空）。
    edgeHandler!({ button: "back", isPressed: true });
    const backCard = () =>
      wrapper.findAll(".mapping-card").find((card) => card.text().includes("返回"));
    await vi.waitFor(() => {
      if (!backCard()!.classes().includes("active")) throw new Error("未高亮");
    });
    expect(backCard()!.classes()).not.toContain("selected");

    // 解锁后：按下即选中该键的编辑。
    const toggles = wrapper.findAll(".toggle-row");
    const lockToggle = toggles.find((row) => row.text().includes("锁定当前按键"));
    const input = lockToggle!.find("input");
    await input.setValue(false);
    edgeHandler!({ button: "back", isPressed: true });
    await vi.waitFor(() => {
      if (!backCard()!.classes().includes("selected")) throw new Error("未跟随选中");
    });
  });

  it("reconciles an unconfigured enhanced hold from the observation snapshot without moving locked selection", async () => {
    vi.useFakeTimers();
    const wrapper = await mountPage();
    const base = await getButtonMappingSnapshot();
    const backCard = () => wrapper.findAll(".mapping-card").find(card => card.text().includes("返回"))!;
    try {
      vi.mocked(getButtonMappingSnapshot).mockResolvedValue({ ...base, observedButtons: ["back"] });
      edgeHandler!({ button: "back", isPressed: true });
      await vi.advanceTimersByTimeAsync(1000);
      expect(backCard().classes()).toContain("active");
      expect(backCard().classes()).not.toContain("selected");
      expect(wrapper.find(".mapping-cell.flashed").exists()).toBe(false);
      vi.mocked(getButtonMappingSnapshot).mockResolvedValue({ ...base, observedButtons: [] });
      await vi.advanceTimersByTimeAsync(1000);
      expect(backCard().classes()).not.toContain("active");
    } finally {
      vi.mocked(getButtonMappingSnapshot).mockResolvedValue(base);
      wrapper.unmount();
      vi.useRealTimers();
    }
  });

  it("shows the fired gesture feedback from engine events", async () => {
    const wrapper = await mountPage();
    gestureHandler!({ button: "ok", trigger: "single" });
    await vi.waitFor(() => {
      // 手势反馈 = 对应格子出现闪烁态（flashed），600ms 后自动消失。
      if (!wrapper.find(".mapping-cell.flashed").exists()) {
        throw new Error("手势触发后格子未出现闪烁反馈");
      }
    });
  });

  /** 编辑器内按标签找 chip 并返回其禁用态。 */
  function chipState(wrapper: VueWrapper, label: string): boolean {
    const chip = wrapper
      .findAll(".mapping-editor .chip")
      .find((element) => element.text().includes(label));
    expect(chip, `未找到 chip：${label}`).toBeDefined();
    return (chip!.element as HTMLButtonElement).disabled;
  }

  async function openCell(
    wrapper: VueWrapper,
    cardLabel: string,
    triggerIndex: number,
  ): Promise<void> {
    const card = wrapper.findAll(".mapping-card").find((c) => c.text().includes(cardLabel));
    expect(card, `未找到卡片：${cardLabel}`).toBeDefined();
    await card!.findAll(".mapping-cell")[triggerIndex]!.trigger("click");
    expect(wrapper.find(".mapping-editor").exists()).toBe(true);
  }

  it("全开放：确定·单击所有操作可配（注入链路已真机验证）+ 单响应提示", async () => {
    const wrapper = await mountPage();
    await openCell(wrapper, "确定", 0);
    expect(chipState(wrapper, "Enter")).toBe(false);
    expect(chipState(wrapper, "Home")).toBe(false);
    expect(chipState(wrapper, "空格")).toBe(false);
    expect(chipState(wrapper, "Ctrl + V")).toBe(false);
    expect(chipState(wrapper, "录入自定义快捷键")).toBe(false);
    expect(chipState(wrapper, "＋ 添加应用")).toBe(false);
    // 武装族按键显示冷首按原生副作用提示（信息性，不门控）。
    expect(wrapper.find(".mapping-editor").text()).not.toContain("它原本的按键效果");
  });

  it("预设芯片显示实际按键组合，功能描述退为悬停提示", async () => {
    const wrapper = await mountPage();
    await openCell(wrapper, "电源", 0);
    const chips = wrapper.findAll(".mapping-editor .chip");
    const texts = chips.map((chip) => chip.text());

    expect(texts).toContain("Ctrl + C");
    expect(texts).toContain("Alt + Tab");
    expect(texts).toContain("Backspace");
    expect(texts).toContain("左 Win + Shift + S");
    expect(texts).not.toContain("复制");
    expect(texts).not.toContain("退格");
    expect(texts).not.toContain("截图");
    expect(chips.find((chip) => chip.text() === "Ctrl + C")!.attributes("title")).toBe("复制");
    expect(chips.find((chip) => chip.text() === "Alt + Tab")!.attributes("title")).toBe(
      "切换窗口",
    );
  });

  it("all keys share the editor without separate capture paths", async () => {
    const wrapper = await mountPage();
    await openCell(wrapper, "确定", 1);
    expect(chipState(wrapper, "Enter")).toBe(false);
    expect(chipState(wrapper, "录入自定义快捷键")).toBe(false);
    expect(chipState(wrapper, "＋ 添加应用")).toBe(false);
    expect(wrapper.find(".mapping-editor").text()).not.toContain("它原本的按键效果");

    await openCell(wrapper, "TV", 0);
    expect(chipState(wrapper, "Enter")).toBe(false);
    expect(chipState(wrapper, "静音")).toBe(false);
    expect(chipState(wrapper, "录入自定义快捷键")).toBe(false);
    expect(chipState(wrapper, "＋ 添加应用")).toBe(false);
    expect(wrapper.find(".mapping-editor").text()).not.toMatch(/三键|五键|增强就绪/);
  });

  it("all directions remain configurable without legacy capture hints", async () => {
    const wrapper = await mountPage();
    await openCell(wrapper, "左", 0);
    expect(chipState(wrapper, "←")).toBe(false);
    expect(chipState(wrapper, "Backspace")).toBe(false);
    expect(chipState(wrapper, "录入自定义快捷键")).toBe(false);
    expect(wrapper.find(".mapping-editor").text()).not.toContain("它原本的按键效果");

    // 与型号无关：RC001 上左键同样开放。
    const rc001 = await mountPage("rc001");
    const leftCellRc001 = rc001
      .findAll(".mapping-card")
      .find((c) => c.text().includes("左"))!
      .findAll(".mapping-cell")[0]!;
    expect((leftCellRc001.element as HTMLButtonElement).disabled).toBe(false);
  });

  it("电源（直接归因族）全开放且无单响应提示", async () => {
    const wrapper = await mountPage();
    await openCell(wrapper, "电源", 2);
    expect(chipState(wrapper, "Esc")).toBe(false);
    expect(chipState(wrapper, "左 Win + Shift + S")).toBe(false);
    expect(chipState(wrapper, "录入自定义快捷键")).toBe(false);
    expect(chipState(wrapper, "＋ 添加应用")).toBe(false);
    expect(wrapper.find(".mapping-editor").text()).not.toContain("它原本的按键效果");
  });

  it("返回/音量±可配置；RC003 的实际增强执行能力不由编辑器声称", async () => {
    for (const model of ["rc003", "rc001", "unknown"] as const) {
      const wrapper = await mountPage(model);
      const backCell = wrapper
        .findAll(".mapping-card")
        .find((c) => c.text().includes("返回"))!
        .findAll(".mapping-cell")[0]!;
      expect(
        (backCell.element as HTMLButtonElement).disabled,
        `${model} 返回格子应禁用`,
      ).toBe(false);
      const volumeCell = wrapper
        .findAll(".mapping-card")
        .find((c) => c.text().includes("音量"))!
        .findAll(".mapping-cell")[0]!;
      expect(
        (volumeCell.element as HTMLButtonElement).disabled,
        `${model} 音量格子应禁用`,
      ).toBe(false);
      await backCell.trigger("click");
      await wrapper.vm.$nextTick();
      expect(wrapper.find(".mapping-editor").exists(), `${model} 返回格子应打开编辑器`).toBe(true);
      wrapper.unmount();
    }
  });
  it.each([
    ["rc001" as const, "小米蓝牙语音遥控器 2"],
    ["rc003" as const, "小米蓝牙语音遥控器 2 Pro"],
  ])("头部设备胶囊显示遥控器型号（%s）", async (model, expected) => {
    const page = await mountPage(model);
    expect(page.find(".device-chip").text()).toContain(expected);
    page.unmount();
  });

  it("型号未读回时头部设备胶囊退回蓝牙广播名", async () => {
    const page = await mountPage("unknown");
    expect(page.find(".device-chip").text()).toContain("小米蓝牙语音遥控器");
    expect(page.find(".device-chip").text()).not.toContain("连接后显示");
    page.unmount();
  });
});
