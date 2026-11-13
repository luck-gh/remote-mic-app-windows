// @vitest-environment jsdom

import { flushPromises, mount } from "@vue/test-utils";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { RuntimeSnapshot } from "../../lib/bridge";
import { effectiveLanguage } from "../../lib/language-state";
import PermissionsSection from "./PermissionsSection.vue";

const runtime: RuntimeSnapshot = {
  appVersion: "0.1.0",
  platform: {
    platform: "browser-preview",
    windowsApiAvailable: false,
    bleScanAvailable: false,
    bleVoiceReady: false,
    wasapiReady: false,
    rawInputReady: false,
    sendInputReady: false,
    verificationStatus: "浏览器预览不代表真机通过",
    connection: {
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
      phase: "unsupported",
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

describe("settings permissions section", () => {
  beforeEach(() => { effectiveLanguage.value = "zh-Hans"; });
  it("shows truthful unsupported states", () => {
    const wrapper = mount(PermissionsSection, { props: { runtime } });
    expect(wrapper.text()).toContain("当前电脑不支持");
    expect(wrapper.findAll(".permission-row")).toHaveLength(3);
  });
  it("renders English permission labels", () => {
    effectiveLanguage.value = "en";
    const wrapper = mount(PermissionsSection, { props: { runtime } });
    expect(wrapper.text()).toContain("Bluetooth");
    expect(wrapper.text()).toContain("Not supported on this PC");
  });
});
