import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  actionSummary,
  audioPhaseLabel,
  chordLabel,
  connectionPhaseLabel,
  formatDiagnosticReport,
  isRecommendedVoiceEndpoint,
  openLogDirectory,
  setCaptureInput,
  openVokieHomepage,
  openVbCableDownloadPage,
  remoteModelLabel,
  VB_CABLE_DOWNLOAD_URL,
  VOKIE_HOMEPAGE_URL,
  type AudioPhase,
  type ConnectionPhase,
  type DiagnosticReport,
} from "./bridge";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));

describe("mouse actions", () => {
  it("summarizes clicks, movement, and wheel amounts", () => {
    expect(actionSummary({ type: "scroll", direction: "down", steps: 5 })).toBe("滚轮向下 5 格");
    expect(actionSummary({ type: "mouse_move", direction: "left", distance: 75 })).toBe("鼠标向左 75 px");
    expect(actionSummary({ type: "mouse_click", kind: "double_left" })).toBe("左键双击");
  });

  it("summarizes the focus-input action", () => {
    // 动作摘要会显示在按键格子上，文案即用户可见串（product-copy 口径）。
    expect(actionSummary({ type: "focus_input" })).toBe("聚焦输入框");
  });
});

describe("voice endpoint recommendation", () => {
  const candidate = (name: string, isVirtualCableCandidate = true) => ({
    id: name,
    name,
    isVirtualCableCandidate,
  });

  it("recommends only the standard VB-Audio CABLE Input", () => {
    expect(isRecommendedVoiceEndpoint(candidate("CABLE Input (VB-Audio Virtual Cable)"))).toBe(
      true,
    );
    // 后端的候选判定更宽（自动选择与安装检测用）：非标准端点不得进推荐位。
    expect(isRecommendedVoiceEndpoint(candidate("CABLE-A Input (VB-Audio Cable A)"))).toBe(false);
    expect(isRecommendedVoiceEndpoint(candidate("CABLE Input (CI Simulation)"))).toBe(false);
    expect(isRecommendedVoiceEndpoint(candidate("扬声器 (Realtek Audio)", false))).toBe(false);
  });

  it("uses the vendor key names for keys that were confusable in Chinese", () => {
    expect(chordLabel({ keys: ["backspace"] })).toBe("Backspace");
    expect(chordLabel({ keys: ["delete"] })).toBe("Delete");
    expect(chordLabel({ keys: ["control", "c"] })).toBe("Ctrl + C");
    expect(chordLabel({ keys: ["left_windows", "shift", "s"] })).toBe("左 Win + Shift + S");
  });
});

describe("connection phase presentation", () => {
  it("covers every serialized Rust connection phase", () => {
    const phases: ConnectionPhase[] = [
      "idle",
      "connecting",
      "discovering",
      "awaiting_capabilities",
      "ready",
      "streaming",
      "draining",
      "reconnecting",
      "suspended",
      "disconnected",
      "failed",
    ];

    expect(phases.map(connectionPhaseLabel)).toEqual([
      "尚未连接",
      "正在连接遥控器",
      "正在连接遥控器",
      "正在确认语音功能",
      "已连接",
      "正在接收语音",
      "正在结束本次语音",
      "正在等待遥控器重连",
      "电脑已进入睡眠",
      "遥控器已断开",
      "连接失败",
    ]);
  });

  it("展示 RC001、RC003 和未知型号", () => {
    expect(remoteModelLabel("rc001")).toBe("小米蓝牙语音遥控器 2");
    expect(remoteModelLabel("rc003")).toBe("小米蓝牙语音遥控器 2 Pro");
    expect(remoteModelLabel("unknown")).toBe("连接后显示");
  });

  it("covers every serialized Rust audio phase", () => {
    const phases: AudioPhase[] = [
      "unconfigured",
      "ready",
      "streaming",
      "draining",
      "failed",
      "unsupported",
    ];

    expect(phases.map(audioPhaseLabel)).toEqual([
      "尚未选择设备",
      "已就绪",
      "正在写入语音",
      "正在结束",
      "语音设备出错",
      "当前环境不支持语音设备",
    ]);
  });
});

describe("diagnostic report presentation", () => {
  it("adds an explicit generation time without changing the captured report", () => {
    const report: DiagnosticReport = {
      schemaVersion: 3,
      appVersion: "0.1.0",
      appBuild: "1001",
      sourceRevision: "0123456789012345678901234567890123456789",
      buildChannel: "local-test",
      windowsVersion: "10.0.26100",
      processArchitecture: "x86_64",
      platform: "windows",
      verificationStatus: "待真机验证",
      capabilities: {
        windowsApiAvailable: true,
        bleScanAvailable: true,
        bleVoiceReady: false,
        wasapiReady: false,
        rawInputReady: false,
        sendInputReady: true,
      },
      connection: {
        phase: "disconnected",
        capabilitiesConfirmed: false,
        sampleRate: null,
        frameSize: null,
        decodedSamples: 0,
        generation: 2,
        reconnectAttempt: 1,
        powerNotificationsAvailable: true,
        errorPresent: false,
      },
      audio: {
        phase: "unconfigured",
        endpointConfigured: false,
        queuedSamples: 0,
        submittedSamples: 0,
        generation: 0,
        errorPresent: false,
      },
      rawInput: {
        phase: "stopped",
        matchedDeviceCount: 0,
        rawEventCount: 0,
        semanticEdgeCount: 0,
        lastButton: null,
        lastIsPressed: null,
        errorPresent: false,
      },
      sendInput: {
        available: true,
        submittedBatches: 0,
        submittedEvents: 0,
        errorPresent: false,
      },
      buttonMapping: {
        enabled: true,
        gateActive: false,
        listenerActive: false,
        swallowedEdges: 0,
        leakedDowns: 0,
        firedGestures: 0,
        errorPresent: false,
      },
  ime: {
    detected: false,
    activeName: null,
    installedNames: [],
    expectedName: null,
    matchesExpected: null,
  },
    };

    const formatted = JSON.parse(formatDiagnosticReport(report, "2026-09-01T00:00:00.000Z"));
    expect(formatted.generatedAt).toBe("2026-09-01T00:00:00.000Z");
    expect(formatted.connection.generation).toBe(2);
    expect(formatted).not.toHaveProperty("remoteName");
    expect(formatted.audio).not.toHaveProperty("selectedEndpointName");
  });
});

describe("VB-CABLE download guidance", () => {
  it("opens only the official VB-Audio page in browser preview", async () => {
    const open = vi.spyOn(window, "open").mockImplementation(() => null);

    await openVbCableDownloadPage();

    expect(open).toHaveBeenCalledWith(VB_CABLE_DOWNLOAD_URL, "_blank", "noopener,noreferrer");
    open.mockRestore();
  });
});

describe("设置页移除普通官方入口，同时保留必要第三方链接", () => {
  afterEach(() => {
    delete (window as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
    vi.mocked(openUrl).mockReset();
  });

  it("保留 VB-CABLE 与 Vokie 的公开入口", async () => {
    const open = vi.spyOn(window, "open").mockImplementation(() => null);

    await openVbCableDownloadPage();
    await openVokieHomepage();

    expect(VB_CABLE_DOWNLOAD_URL).toBe("https://vb-audio.com/Cable/");
    expect(VOKIE_HOMEPAGE_URL).toBe("https://vokie.com/");
    expect(open).toHaveBeenNthCalledWith(1, VB_CABLE_DOWNLOAD_URL, "_blank", "noopener,noreferrer");
    expect(open).toHaveBeenNthCalledWith(2, VOKIE_HOMEPAGE_URL, "_blank", "noopener,noreferrer");
    expect(openUrl).not.toHaveBeenCalled();
    open.mockRestore();
  });

});

describe("诊断日志目录入口", () => {
  afterEach(() => {
    delete (window as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
    vi.mocked(invoke).mockReset();
  });

  it("在 Tauri 运行时按精确命令名交给 Rust，并原样回传目录", async () => {
    (window as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {};
    const directory = "C:\\Users\\probe\\AppData\\Local\\SayAll\\Logs";
    vi.mocked(invoke).mockResolvedValue(directory);

    await expect(openLogDirectory()).resolves.toBe(directory);
    // 命令名写错或 Rust 侧漏注册时这里会红——这是该入口唯一的前端契约。
    // 前端不拼接、不传路径参数：目录由 Rust 从日志初始化结果推导。
    expect(invoke).toHaveBeenCalledWith("open_log_directory");
  });

  it("浏览器预览下明确不可用而不是静默失败", async () => {
    await expect(openLogDirectory()).rejects.toThrow("当前是浏览器预览，无法打开日志目录");
    expect(invoke).not.toHaveBeenCalled();
  });
});

describe("explicit capture and writer transaction", () => {
  afterEach(() => {
    delete (window as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
    vi.mocked(invoke).mockReset();
  });
  it("submits both explicit endpoint choices in one IPC request", async () => {
    (window as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {};
    const config = { enabled: true, endpointId: 'capture', endpointName: 'CABLE Output' };
    await setCaptureInput(config, 'render-16');
    expect(invoke).toHaveBeenCalledExactlyOnceWith('set_capture_input', { config, writerEndpointId: 'render-16' });
  });
});
