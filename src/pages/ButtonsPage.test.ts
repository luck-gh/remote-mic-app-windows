vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(async (command: string) => command === "get_ui_preferences" ? {lockButtonSelection:true,templatesExpanded:true,associationsExpanded:true} : undefined) }));
import { beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount, type VueWrapper } from "@vue/test-utils";
import ButtonsPage from "./ButtonsPage.vue";

type EdgeHandler = (edge: { button: string; isPressed: boolean }) => void;
type GestureHandler = (gesture: { button: string; trigger: string }) => void;
type ShortcutCaptureHandler = (edge: { key: string; isPressed: boolean }) => void;

let edgeHandler: EdgeHandler | null = null;
let gestureHandler: GestureHandler | null = null;
let shortcutCaptureHandler: ShortcutCaptureHandler | null = null;

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
      { id: "scene-copy", name: "我的按键模板", kind: "direct", readOnly: false, buttonMappings: userTemplate().mappings },
      { id: "profile-a", name: "模板 A", kind: "direct", readOnly: false, buttonMappings: { enabled: true, actions: {} } },
    ])),
    copyTemplateCatalogEntry: vi.fn(),
    saveMappingConfiguration: vi.fn(async (configuration: unknown) => configuration),
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
  getMappingConfiguration,
  getButtonMappingSnapshot,
  getTemplateCatalog,
  saveMappingConfiguration,
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
  vi.mocked(subscribeButtonEdges).mockClear();
  vi.mocked(subscribeButtonGestures).mockClear();
  vi.mocked(saveButtonMappings).mockClear();
  vi.mocked(saveButtonMappingTemplate).mockClear();
  vi.mocked(updateButtonMappingTemplate).mockClear();
});

describe("buttons mapping page", () => {
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
    expect(wrapper.findAll(".mapping-cell")[0]!.element).toHaveProperty("disabled", true);
    expect(wrapper.findAll(".mapping-card").find(card => card.text().includes("确定"))!.text()).toContain("Shift");
    expect(updateButtonMappingTemplate).not.toHaveBeenCalled();
  });

  it("requires an explicit discard before opening a read-only template over a dirty direct draft", async () => {
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
    expect(wrapper.find(".mapping-editor").text()).toContain("原生按键动作");
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

  it("全开放：确定·双击与 TV 所有操作可配 + 各自的单响应提示", async () => {
    const wrapper = await mountPage();
    await openCell(wrapper, "确定", 1);
    expect(chipState(wrapper, "Enter")).toBe(false);
    expect(chipState(wrapper, "录入自定义快捷键")).toBe(false);
    expect(chipState(wrapper, "＋ 添加应用")).toBe(false);
    expect(wrapper.find(".mapping-editor").text()).toContain("原生按键动作");

    await openCell(wrapper, "TV", 0);
    expect(chipState(wrapper, "Enter")).toBe(false);
    expect(chipState(wrapper, "静音")).toBe(false);
    expect(chipState(wrapper, "录入自定义快捷键")).toBe(false);
    expect(chipState(wrapper, "＋ 添加应用")).toBe(false);
    expect(wrapper.find(".mapping-editor").text()).toContain("显式启动三键增强");
  });

  it("左键与其余方向键同样开放自定义并显示结构性泄漏提示", async () => {
    const wrapper = await mountPage();
    await openCell(wrapper, "左", 0);
    expect(chipState(wrapper, "←")).toBe(false);
    expect(chipState(wrapper, "Backspace")).toBe(false);
    expect(chipState(wrapper, "录入自定义快捷键")).toBe(false);
    expect(wrapper.find(".mapping-editor").text()).toContain("原生按键动作");

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
    expect(wrapper.find(".mapping-editor").text()).not.toContain("原生按键动作");
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
});
