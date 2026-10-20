import { flushPromises, mount } from "@vue/test-utils";
import { beforeEach, describe, expect, it, vi } from "vitest";
import ButtonActionEditor from "./ButtonActionEditor.vue";
import { getCombinedActions, learnFocusPoint, type MacroDefinition } from "../lib/bridge";

vi.mock("../lib/bridge", async (importOriginal) => ({
  ...await importOriginal<typeof import("../lib/bridge")>(),
  getCombinedActions: vi.fn(),
  learnFocusPoint: vi.fn(async () => ({ xRatio: 0.5, yRatio: 0.5, windowWidth: 1200, windowHeight: 800 })),
  subscribeShortcutCaptureEdges: vi.fn(async () => () => {}),
}));

const definition = (version: string, name: string): MacroDefinition => ({
  schemaVersion: "1",
  macroId: "macro.example",
  version,
  name,
  steps: [],
});

describe("组合动作与个人版编辑契约", () => {
  beforeEach(() => {
    vi.mocked(getCombinedActions).mockResolvedValue({
      formatVersion: 1,
      definitions: [definition("1", "旧组合动作"), definition("2", "新组合动作")],
    });
  });

  it("通用编辑器仅显示最新版本，并把选择交给原有草稿保存路径", async () => {
    const wrapper = mount(ButtonActionEditor, {
      props: { mappings: { enabled: true, actions: {} }, button: "tv", trigger: "single", presetApps: [] },
    });
    await flushPromises();
    expect(wrapper.text()).not.toContain("旧组合动作");
    const macro = wrapper.findAll("button").find(button => button.text() === "新组合动作");
    expect(macro).toBeDefined();
    await macro!.trigger("click");
    expect(wrapper.emitted("update")).toEqual([[{ type: "run_macro", macroId: "macro.example", macroVersion: "2" }]]);
    wrapper.unmount();
  });

  it("个人模板继续只提供固定按键，不载入宏库", async () => {
    vi.mocked(getCombinedActions).mockClear();
    const wrapper = mount(ButtonActionEditor, {
      props: { mappings: { enabled: true, actions: {} }, button: "tv", trigger: "single", presetApps: [], shortcutsOnly: true },
    });
    await flushPromises();
    expect(getCombinedActions).not.toHaveBeenCalled();
    expect(wrapper.text()).not.toContain("组合动作");
    wrapper.unmount();
  });

  it("记录位置使用上游六秒窗口，目标变化后取消迟到结果", async () => {
    vi.useFakeTimers();
    vi.mocked(learnFocusPoint).mockClear();
    const wrapper = mount(ButtonActionEditor, {
      props: { mappings: { enabled: true, actions: { tv: { single: { type: "open_app", target: "notepad" }, double: { type: "disabled" }, long: { type: "disabled" } } } }, button: "tv", trigger: "single", presetApps: [] },
    });
    try {
      await flushPromises();
      await wrapper.get('[aria-label="应用聚焦方式"]').setValue("recorded_point");
      await wrapper.setProps({ mappings: { enabled: true, actions: { tv: { single: { type: "open_app", target: "notepad" }, double: { type: "disabled" }, long: { type: "disabled" } } }, focusProfiles: { notepad: { strategy: "recorded_point" } } } });
      await wrapper.get('[data-testid="learn-app-point"]').trigger("click");
      await vi.advanceTimersByTimeAsync(5999);
      expect(learnFocusPoint).not.toHaveBeenCalled();
      await wrapper.setProps({ mappings: { enabled: true, actions: {} } });
      await vi.advanceTimersByTimeAsync(1);
      expect(learnFocusPoint).not.toHaveBeenCalled();
      expect(wrapper.emitted("focusProfile")).toEqual([["notepad", { strategy: "recorded_point" }]]);
    } finally {
      wrapper.unmount();
      vi.useRealTimers();
    }
  });
});
