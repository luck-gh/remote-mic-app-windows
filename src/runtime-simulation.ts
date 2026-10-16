import { invoke } from "@tauri-apps/api/core";
import {
  connectRemote,
  disconnectRemote,
  getAudioSnapshot,
  getDiagnosticReport,
  getGainDb,
  getRawInputSnapshot,
  getRuntimeSnapshot,
  listAudioEndpoints,
  saveButtonMappings,
  scanPairedRemotes,
  stopRawInput,
  testButtonMapping,
  type PlatformSnapshot,
} from "./lib/bridge";
import { reportFrontendEvent } from "./lib/frontend-diagnostics";

interface RuntimeSimulationReport {
  passed: boolean;
  platform?: string;
  steps: string[];
  error?: string;
}

function assert(condition: unknown, message: string): asserts condition {
  if (!condition) throw new Error(message);
}

/**
 * 阶段标记：写进诊断日志（`runtime_simulation phase=completed reason=<stage>`）。
 *
 * 2026-10-02 教训：本机仿真超时 60 秒时，报告没写、stdout/stderr 全空，日志里
 * 只有每秒一条 `tray_icon_state ... ipc_unavailable`，无法判断卡在哪一步——
 * 是因为那次构建漏了 `VITE_SAYALL_RUNTIME_SIMULATION=1`（仿真前端入口根本没
 * 编进去）。每个阶段入口留一条日志后，同样的现场一次日志拉取就能定位。
 */
function mark(stage: string): void {
  reportFrontendEvent({
    event: "runtime_simulation",
    phase: "started",
    result: "passed",
    reason: stage,
  });
}

async function waitFor<T>(read: () => T | null, description: string): Promise<T> {
  const deadline = Date.now() + 10_000;
  while (Date.now() < deadline) {
    const value = read();
    if (value !== null) return value;
    await new Promise((resolve) => window.setTimeout(resolve, 50));
  }
  throw new Error(`等待 ${description} 超时`);
}

function buttonWithText(label: string): HTMLButtonElement | null {
  return (
    Array.from(document.querySelectorAll<HTMLButtonElement>("button")).find(
      (button) => button.textContent?.trim().includes(label) && !button.disabled,
    ) ?? null
  );
}

async function clickButton(label: string): Promise<void> {
  const button = await waitFor(() => buttonWithText(label), `按钮“${label}”可用`);
  button.click();
}

async function openPage(label: string, heading = label): Promise<void> {
  const button = await waitFor(
    () =>
      Array.from(document.querySelectorAll<HTMLButtonElement>("nav button")).find(
        (candidate) => candidate.textContent?.trim().includes(label),
      ) ?? null,
    `导航“${label}”`,
  );
  button.click();
  await waitFor(
    () => (document.querySelector("h1")?.textContent?.trim() === heading ? heading : null),
    `页面“${heading}”`,
  );
}

async function runJourney(steps: string[]): Promise<PlatformSnapshot> {
  mark("journey_start");
  // 驱动页允许已连接时重新扫描；连接页在自动连接后会禁用扫描入口。
  await openPage("驱动", "驱动与配对");
  const runtime = await getRuntimeSnapshot();
  assert(runtime.platform.platform === "windows-ci-simulation", "应用未使用 Windows CI 仿真后端");
  steps.push("Tauri WebView 通过真实 IPC 读取仿真运行快照");

  const pairing = await waitFor(
    () => document.querySelector<HTMLDetailsElement>(".driver-guide .pairing-guide"),
    "配对与重新连接区域",
  );
  const pairingSummary = pairing.querySelector<HTMLElement>("summary");
  assert(pairingSummary?.textContent?.trim() === "配对与重新连接", "驱动页缺少配对展开入口");
  if (!pairing.open) pairingSummary.click();
  await waitFor(() => (pairing.open ? true : null), "配对与重新连接展开");
  await clickButton("扫描已配对遥控器");
  await waitFor(
    () => (document.querySelectorAll(".driver-guide .device-list li").length === 2 ? true : null),
    "RC001/RC003 扫描结果",
  );
  const remotes = await scanPairedRemotes();
  assert(remotes.length === 2, "仿真扫描没有同时返回 RC001 和 RC003");
  assert(remotes.some((remote) => remote.model === "rc001"), "仿真扫描缺少 RC001");
  assert(remotes.some((remote) => remote.model === "rc003"), "仿真扫描缺少 RC003");
  steps.push("驱动页展开配对区域，经扫描按钮渲染 RC001/RC003 结果");

  await openPage("连接");
  const rc001 = remotes.find((remote) => remote.model === "rc001");
  assert(rc001, "找不到 RC001 仿真设备");
  const connection = await connectRemote(rc001.id);
  assert(connection.phase === "ready", "RC001 仿真连接没有进入 ATVV 就绪");
  assert(connection.capabilities?.sampleRate === 16_000, "RC001 仿真能力不是 16 kHz");
  steps.push("RC001 连接 command 返回 16 kHz ATVV 就绪状态");

  const advancedAudio = await waitFor(() => document.querySelector<HTMLDetailsElement>(".audio-advanced"), "高级声音诊断");
  assert(!advancedAudio.open, "声音写入端不应默认出现在常规界面");
  assert(document.querySelector("#capture-input-target"), "常规界面缺少目标麦克风选择");
  assert(!document.querySelector(".endpoint-list"), "常规界面泄漏了播放设备列表");
  // 仿真后端不提供真实 Capture 拓扑，显式高级选择仅验证原有 WASAPI 通道。
  advancedAudio.open = true;
  advancedAudio.dispatchEvent(new Event("toggle"));
  const renderChoice = await waitFor(() => document.querySelector<HTMLButtonElement>(".audio-advanced .endpoint-list button"), "高级手动写入端");
  if (!renderChoice.disabled) renderChoice.click();
  await waitFor(() => document.querySelector<HTMLButtonElement>(".audio-advanced .endpoint-list button")?.textContent?.trim() === "当前设备" ? true : null, "手动声音写入端确认");
  const endpoints = await listAudioEndpoints();
  assert(endpoints.length === 1, "仿真音频端点数量异常");
  const audio = await getAudioSnapshot();
  assert(audio.phase === "ready", "仿真音频端点没有进入 WASAPI 就绪");
  assert(audio.selectedEndpointId === endpoints[0].id, "仿真 CABLE Input 没有被明确选择");
  steps.push("连接页面仅常规展示目标麦克风；高级显式选择仿真声音写入端，真实通道配对 deferred");

  // 语音增益（2026-10-04，对齐 Mac 设置页「增益」滑块）：0 dB = 原始音量。
  // 拖动（input）只改显示、松手（change）才落盘——这里按真实手势派发两个事件，
  // 再用 IPC 读回持久化值核对。
  const gainSlider = await waitFor(
    () => document.querySelector<HTMLInputElement>('input[name="audio-gain"]'),
    "语音设备卡增益滑块",
  );
  assert(
    document.body.textContent?.includes("0 dB 保持原始音量"),
    "增益说明文案缺失",
  );
  gainSlider.value = "12";
  gainSlider.dispatchEvent(new Event("input", { bubbles: true }));
  gainSlider.dispatchEvent(new Event("change", { bubbles: true }));
  await waitFor(() => (gainSlider.value === "12" ? true : null), "增益滑块显示 12 dB");
  assert(document.body.textContent?.includes("12 dB"), "增益读数没有更新为 12 dB");
  // change 之后的落盘是异步 IPC：必须轮询读回，不能拿本地读数当保存完成的证据
  //（首次实现就这么错过一次：界面显示 12 dB，磁盘当时还是 0 dB）。
  let savedGain = 0;
  const gainDeadline = Date.now() + 10_000;
  while (Date.now() < gainDeadline) {
    savedGain = await getGainDb();
    if (savedGain === 12) break;
    await new Promise((resolve) => window.setTimeout(resolve, 50));
  }
  assert(savedGain === 12, `仿真增益没有经 IPC 落盘：${savedGain}`);
  steps.push("连接页增益滑块经真实 IPC 落盘并读回 12 dB");

  mark("buttons_page");
  await openPage("按键", "按键映射");
  await waitFor(
    () => (document.body.textContent?.includes("按键监听已就绪") ? true : null),
    "Raw Input 就绪状态（随应用自愈启动）",
  );
  const rawInput = await getRawInputSnapshot();
  assert(rawInput.phase === "ready", "仿真 Raw Input 没有进入就绪");
  assert(rawInput.semanticEdgeCount === 2, "仿真 Raw Input 语义边沿数量异常");
  steps.push("按键页面随应用启动展示 Raw Input 仿真状态");
  assert(
    document.body.textContent?.includes("语音键"),
    "按键画布没有渲染语音键卡片",
  );
  assert(
    Array.from(document.querySelectorAll(".mapping-cell")).length === 36,
    "按键画布没有渲染 12 键 × 3 触发方式的单元格",
  );
  steps.push("按键映射画布渲染 12 张按键卡与三列触发单元格");

  await saveButtonMappings({
    enabled: true,
    actions: {
      ok: {
        single: { type: "shortcut", chord: { keys: ["left_control", "c"] } },
        double: { type: "disabled" },
        long: { type: "disabled" },
      },
    },
  });
  const sendInput = await testButtonMapping("ok", "single");
  assert(sendInput.submittedBatches === 1, "仿真 SendInput 没有提交唯一批次");
  assert(sendInput.submittedEvents === 4, "Ctrl+C 仿真没有生成四个按下/释放事件");
  steps.push("映射保存、热加载和 SendInput 记录器通过真实 Tauri IPC");

  mark("templates_page");
  await openPage("模板");
  await waitFor(
    () => document.querySelector(".complete-template-panel"),
    "模板配置与内置目录加载",
  );
  assert(document.querySelector(".run-modes") !== null, "模板页缺少独立模板切换设置");
  steps.push("模板页通过真实 IPC 加载完整按键模板与独立切换设置");

  mark("drivers_page");
  await openPage("驱动", "驱动与配对");
  assert(document.querySelector(".guide-grid") !== null, "驱动页缺少可选增强与配对引导");
  steps.push("驱动页显示可选增强与配对引导，未启动提权 Helper");

  mark("permissions_page");
  await openPage("权限");
  // 诊断摘要 2026-10-01 从关于页迁回权限页（用户指定）：状态与取证入口同页，
  // 用户看到某一项不对时不必跳页。
  await clickButton("生成摘要");
  await waitFor(
    () =>
      document.querySelector(".diagnostic-output")?.textContent?.includes("windows-ci-simulation")
        ? true
        : null,
    "诊断摘要渲染",
  );
  const diagnostic = await getDiagnosticReport();
  assert(diagnostic.platform === "windows-ci-simulation", "诊断摘要没有来自仿真平台");
  assert(diagnostic.capabilities.bleVoiceReady, "诊断摘要没有反映 ATVV 就绪");
  assert(diagnostic.capabilities.wasapiReady, "诊断摘要没有反映 WASAPI 就绪");
  assert(diagnostic.capabilities.rawInputReady, "诊断摘要没有反映 Raw Input 就绪");
  steps.push("权限页呈现蓝牙/按键/音频三项状态并生成去标识化运行诊断摘要");

  // "打开日志目录"刻意**不作成败断言**：它经 ShellExecuteW 交给资源管理器，
  // CI runner 是否有可用的 shell 桌面不在本仓库控制范围内，拿它当门禁只会
  // 制造与本产品无关的红灯。这里只证明 WebView → IPC → Rust → shell 的往返
  // 真的走通（按钮回到可用、消息落在终态），并把结果如实记入步骤文本。
  await clickButton("打开日志目录");
  const logDirectoryMessage = await waitFor(
    () => {
      const text = document.querySelector(".log-directory-message")?.textContent?.trim();
      return text && text !== "正在打开日志目录…" ? text : null;
    },
    "日志目录入口返回终态",
  );
  steps.push(
    logDirectoryMessage.startsWith("已打开日志目录")
      ? "权限页“打开日志目录”经真实 IPC 交由资源管理器打开"
      : `权限页“打开日志目录”返回不可用（deferred）：${logDirectoryMessage}`,
  );

  await openPage("设置");
  mark("settings_page");
  // 2026-10-02 设置页改版（对齐 Mac 新设置页）：顶部模块 = 应用标识 + 版本 +
  // 检查更新；各分组在卡外有分组标题。2026-10-08 起多一个「首次设置」分组
  // （向导重跑入口，随首次设置向导一起交付）。
  const settingsOverview = document.querySelector<HTMLElement>("article.settings-overview");
  assert(settingsOverview !== null, "设置页缺少顶部标识与检查更新模块");
  assert(
    settingsOverview.textContent?.includes("当前版本") === true,
    "设置页顶部没有显示当前版本",
  );
  assert(
    document.querySelector("article.settings-overview .check-button") !== null,
    "设置页顶部没有检查更新入口",
  );
  const sectionTitles = Array.from(
    document.querySelectorAll<HTMLElement>(".settings-section .section-title"),
  ).map((element) => element.textContent?.trim());
  assert(
    sectionTitles.join(" / ") === "通用 / 首次设置",
    `设置页分组标题异常：${sectionTitles.join(" / ")}`,
  );
  const wizardEntry = Array.from(
    document.querySelectorAll<HTMLButtonElement>(".settings-section .settings-row button"),
  ).find((button) => button.textContent?.trim() === "重新运行向导");
  assert(wizardEntry !== null, "设置页缺少「重新运行向导」入口");
  assert(!sectionTitles.includes("问题反馈"), "设置页仍显示官方问题反馈分组");
  assert(!document.querySelector('input[name="app-icon"]'), "设置页仍显示废弃图标选择");
  assert(document.querySelector('img.app-logo')?.getAttribute('src') === '/app-logo.png', "设置页未显示固定个人图标");
  steps.push("设置页固定个人图标，无官方反馈入口及旧图标选择");

  const darkTheme = await waitFor(
    () =>
      document.querySelector<HTMLInputElement>('input[name="theme-preference"][value="dark"]:not(:disabled)'),
    "深色外观选项可用",
  );
  darkTheme.click();
  await waitFor(
    () => (document.documentElement.dataset.theme === "dark" ? true : null),
    "深色外观应用",
  );
  assert(darkTheme.checked, "深色外观保存后没有保持选中");
  assert(!document.querySelector('[role="alert"]'), "深色外观保存后显示错误");

  const systemTheme = await waitFor(
    () =>
      document.querySelector<HTMLInputElement>('input[name="theme-preference"][value="system"]:not(:disabled)'),
    "系统外观选项可用",
  );
  systemTheme.click();
  await waitFor(() => (systemTheme.checked && !systemTheme.disabled ? true : null), "系统外观恢复");
  assert(!document.querySelector('[role="alert"]'), "恢复系统外观后显示错误");
  steps.push("设置页深色/系统外观经 Windows WebView、Tauri capability 与设置持久化闭环");

  await openPage("连接");
  // 重新挂载连接页：增益必须从持久化设置恢复（证明读取路径真的从 IPC 取值，
  // 而不是界面硬编码 0）——上一步已把 12 dB 写入仿真设置目录。
  const restoredGainSlider = await waitFor(
    () => {
      const slider = document.querySelector<HTMLInputElement>('input[name="audio-gain"]');
      return slider && slider.value === "12" ? slider : null;
    },
    "增益从持久化设置恢复为 12 dB",
  );
  assert(restoredGainSlider.value === "12", "增益没有从持久化设置恢复");
  steps.push("重新进入连接页后增益从持久化设置恢复为 12 dB");
  steps.push("六个侧栏页面均在 Windows WebView 中完成导航和渲染");

  mark("voice_session");
  const voice = await invoke<PlatformSnapshot>("run_runtime_simulation_voice_session");
  assert(voice.connection.decodedSamples === 240, "40 + 80 字节语音没有解码为 240 个采样");
  assert(voice.connection.generation === 1, "首次仿真语音会话代次不是 1");
  assert(voice.connection.voiceState === "idle", "仿真语音排空后没有回到 idle");
  assert(voice.audio.submittedSamples === 240, "仿真 WASAPI 没有提交完整 240 个采样");
  steps.push("RC001 首次 STREAM_START → 40+80 AUDIO → STREAM_STOP → DRAIN 闭环完成");

  await stopRawInput();
  await disconnectRemote();
  mark("cleanup");
  const finalSnapshot = await getRuntimeSnapshot();
  assert(finalSnapshot.platform.connection.phase === "disconnected", "仿真连接没有释放");
  assert(finalSnapshot.platform.rawInput.phase === "stopped", "仿真 Raw Input 没有停止");
  steps.push("断开和停止 command 完成资源状态释放");
  return voice;
}

export async function runRuntimeSimulationSmoke(): Promise<void> {
  const report: RuntimeSimulationReport = { passed: false, steps: [] };
  try {
    const snapshot = await runJourney(report.steps);
    report.passed = true;
    report.platform = snapshot.platform;
  } catch (error) {
    report.error = error instanceof Error ? error.message : String(error);
  }
  await invoke("complete_runtime_simulation_smoke", { result: report });
}
