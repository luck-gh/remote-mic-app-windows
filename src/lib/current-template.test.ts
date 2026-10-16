import { defineComponent } from "vue";
import { flushPromises, mount } from "@vue/test-utils";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { getSceneSnapshot, selectCurrentTemplate, subscribeSceneEvents, type SceneEvent, type SceneSnapshot } from "./bridge";
import { useCurrentTemplate } from "./current-template";

vi.mock("./bridge", () => ({ getSceneSnapshot: vi.fn(), selectCurrentTemplate: vi.fn(), subscribeSceneEvents: vi.fn(), isTauriRuntime: () => false }));
const consumers = new Set<(event: SceneEvent) => void>();
// Mirrors crates/sayall-windows/src/scene_control.rs::mapping_notice_from:
// kind reports mapping availability (direct/common/disabled/unconfigured), templateId is the active identity.
const snapshot = (id: string | null, revision = 1): SceneSnapshot => ({ mappingNoticeEnabled: true, mappingNotice: { kind: id ? "direct" : "common", templateId: id, name: id, actionsAvailable: true }, mappingNoticeRevision: revision, enabled: true, generation: revision, foregroundGeneration: 1, applicationId: "app", templateId: id, panel: null, updateDefault: false, selectedIndex: null, menuItems: [], waitingForRelease: false, voiceActive: false, voiceButtonPressed: false, status: null });
const Harness = defineComponent({ setup: () => ({ state: useCurrentTemplate() }), template: '<span><b>{{ state.applied.value ? state.templateId.value ?? "common" : "unknown" }}</b><i>{{ state.persistenceMessage.value }}</i></span>' });
let actionableState: ReturnType<typeof useCurrentTemplate> | null = null;
const ActionHarness = defineComponent({ setup: () => { actionableState = useCurrentTemplate(); return {}; }, template: "<span />" });

describe("applied template state", () => {
  beforeEach(() => {
    vi.clearAllMocks(); consumers.clear();
    vi.mocked(getSceneSnapshot).mockResolvedValue(snapshot("first"));
    vi.mocked(subscribeSceneEvents).mockImplementation(async callback => { consumers.add(callback); return () => consumers.delete(callback); });
  });

  it("keeps mounted consumers synchronized and rereads the applied value after page reopening", async () => {
    const first = mount(Harness), second = mount(Harness); await flushPromises();
    const changed = snapshot("second", 2);
    for (const callback of consumers) callback({ type: "mapping_applied", notice: changed.mappingNotice!, revision: 2 });
    await flushPromises();
    expect(first.text()).toBe("second"); expect(second.text()).toBe("second");
    first.unmount(); second.unmount(); expect(consumers.size).toBe(0);
    vi.mocked(getSceneSnapshot).mockResolvedValueOnce(changed);
    const reopened = mount(Harness); await flushPromises(); expect(reopened.text()).toBe("second");
    expect(selectCurrentTemplate).not.toHaveBeenCalled(); reopened.unmount();
  });

  it("does not let a late initial read roll back a newer applied event", async () => {
    let resolve!: (value: SceneSnapshot) => void;
    vi.mocked(getSceneSnapshot).mockReturnValueOnce(new Promise(done => { resolve = done; }));
    const wrapper = mount(Harness); await flushPromises();
    for (const callback of consumers) callback({ type: "snapshot", snapshot: snapshot("newer", 3) });
    resolve(snapshot("older", 2)); await flushPromises();
    expect(wrapper.text()).toBe("newer"); wrapper.unmount();
  });

  it("does not let an initial read with the same generation erase a voice observation", async () => {
    let resolve!: (value: SceneSnapshot) => void;
    vi.mocked(getSceneSnapshot).mockReturnValueOnce(new Promise(done => { resolve = done; }));
    let state!: ReturnType<typeof useCurrentTemplate>;
    const wrapper = mount(defineComponent({ setup: () => { state = useCurrentTemplate(); return {}; }, template: '<span />' }));
    await flushPromises();
    const down = { ...snapshot('first'), voiceButtonPressed: true };
    for (const callback of consumers) callback({ type: 'snapshot', snapshot: down });
    resolve({ ...down, voiceButtonPressed: false });
    await flushPromises();
    expect(state.sceneSnapshot.value?.voiceButtonPressed).toBe(true);
    const lateCallback = [...consumers][0]!;
    wrapper.unmount();
    lateCallback({ type: 'snapshot', snapshot: { ...down, voiceButtonPressed: false } });
    expect(state.sceneSnapshot.value?.voiceButtonPressed).toBe(true);
  });

  it("shows common when the mapping engine confirms common is applied", async () => {
    const wrapper = mount(Harness); await flushPromises();
    const changed = snapshot(null, 2);
    for (const callback of consumers) callback({ type: "mapping_applied", notice: changed.mappingNotice!, revision: 2 });
    await flushPromises();
    expect(selectCurrentTemplate).not.toHaveBeenCalled();
    expect(wrapper.text()).toBe("common"); wrapper.unmount();
  });

  it("shows no transient success message and reports only authoritative persistence failure", async () => {
    const wrapper = mount(Harness); await flushPromises();
    expect(wrapper.find("i").text()).toBe("");
    const saving = { ...snapshot("first", 2).mappingNotice!, manualSaveStatus: "saving" as const };
    for (const callback of consumers) callback({ type: "mapping_applied", notice: saving, revision: 2 });
    await flushPromises();
    expect(wrapper.find("i").text()).toBe("");
    const saved = { ...snapshot("first", 2).mappingNotice!, manualSaveStatus: "saved" as const };
    for (const callback of consumers) callback({ type: "mapping_applied", notice: saved, revision: 2 });
    await flushPromises();
    expect(wrapper.find("i").text()).toBe("");
    const failed = { ...saved, manualSaveStatus: "failed" as const };
    for (const callback of consumers) callback({ type: "mapping_applied", notice: failed, revision: 3 });
    await flushPromises();
    expect(wrapper.find("b").text()).toBe("first");
    expect(wrapper.find("i").text()).toContain("未能记住本次选择");
    expect(wrapper.find("i").text()).toContain("重启后将使用之前记住的选择");
    expect(wrapper.find("i").text()).toContain("如无记录则为通用配置");
    wrapper.unmount();
  });

  it("does not let an older activation reply overwrite a newer MappingApplied event", async () => {
    let resolve!: (value: SceneSnapshot) => void;
    vi.mocked(selectCurrentTemplate).mockReturnValueOnce(new Promise(done => { resolve = done; }));
    const wrapper = mount(ActionHarness); await flushPromises();
    const pending = actionableState!.activate("first");
    for (const callback of consumers) callback({ type: "mapping_applied", notice: snapshot("second", 2).mappingNotice!, revision: 2 });
    resolve(snapshot("first", 1)); await pending; await flushPromises();
    expect(actionableState!.templateId.value).toBe("second");
    expect(actionableState!.applying.value).toBe(false);
    wrapper.unmount();
  });

  it("accepts the Rust mapping notice kinds for selected and common templates", async () => {
    vi.mocked(selectCurrentTemplate)
      .mockResolvedValueOnce({ ...snapshot("empty-template", 2), mappingNotice: { kind: "disabled", templateId: "empty-template", name: "空模板", actionsAvailable: false } })
      .mockResolvedValueOnce({ ...snapshot(null, 3), mappingNotice: { kind: "unconfigured", templateId: null, name: null, actionsAvailable: false } });
    const wrapper = mount(ActionHarness); await flushPromises();
    await actionableState!.activate("empty-template");
    expect(actionableState!.templateId.value).toBe("empty-template");
    expect(actionableState!.error.value).toBe("");
    await actionableState!.activate(null);
    expect(actionableState!.templateId.value).toBe(null);
    expect(actionableState!.error.value).toBe("");
    wrapper.unmount();
  });

  it("does not let an activation reply erase a same-revision physical snapshot", async () => {
    let resolve!: (value: SceneSnapshot) => void;
    vi.mocked(selectCurrentTemplate).mockReturnValueOnce(new Promise(done => { resolve = done; }));
    const wrapper = mount(ActionHarness); await flushPromises();
    const pending = actionableState!.activate("first");
    const pressed = { ...snapshot("first", 1), voiceButtonPressed: true };
    for (const callback of consumers) callback({ type: "snapshot", snapshot: pressed });
    resolve(snapshot("first", 1)); await pending; await flushPromises();
    expect(actionableState!.sceneSnapshot.value?.voiceButtonPressed).toBe(true);
    wrapper.unmount();
  });

  it("keeps the previous applied template when a snapshot only changes the candidate", async () => {
    const wrapper = mount(Harness); await flushPromises();
    for (const callback of consumers) callback({ type: "snapshot", snapshot: { ...snapshot("first"), templateId: "candidate" } });
    await flushPromises();
    expect(wrapper.text()).toBe("first");
    expect(selectCurrentTemplate).not.toHaveBeenCalled(); wrapper.unmount();
  });

  it("releases a subscription that finishes after the page has closed", async () => {
    let resolve!: (stop: () => void) => void;
    vi.mocked(subscribeSceneEvents).mockReturnValueOnce(new Promise(done => { resolve = done; }));
    const stop = vi.fn(); const wrapper = mount(Harness); wrapper.unmount();
    resolve(stop); await flushPromises();
    expect(stop).toHaveBeenCalledOnce(); expect(getSceneSnapshot).not.toHaveBeenCalled();
  });
});
