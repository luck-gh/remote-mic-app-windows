# Windows 首次设置向导（Onboarding）真机验收手册

## 适用范围

- 仓库：`GetSayAll/remote-mic-app-windows`，分支 `feat/onboarding`
- 硬件：小米蓝牙语音遥控器 2（RC001）与 2 Pro（RC003）**分别执行**；单型号结果不能代替另一种
- 前置依赖：VB-CABLE、三种输入工具（豆包输入法 / 微信输入法 / Vokie）各一个可登录账号
- 相关记录：探针与实现决策见 `docs/investigations/2026-10-04-windows-onboarding-feasibility.md`
- 自动化结果（vitest / Rust 测试 / CI）不替代本手册，任何真机步骤未执行一律记为 deferred

## 2026-10-13 向导语音门禁、跳过与退出

用户明确要求替代此前依赖向导状态/门禁才能前进的流程。最终独立 review 接受当前实现契约；这不代表模拟 App、安装或真机验收完成。七步均有独立 Skip/退出；Skip 记录为 `skipped`、不置 `voiceVerified`，完成页另提供“跳过验证并开始使用”。正常 Continue 离开 voice 步骤会取消真实语音。退出/完成页跳过使用全局重入屏障，先取消语音、关闭按键观察并恢复映射暂挂、等待 pending stage 收尾；屏障期间 mutation 入口 busy/inert，之后回滚未验证 staged 工具/和弦及正式配置并同步 runtime。`completed_version=flow_version` 仅表示本版流程已处理，不是验证成功；保留当前 step，不新增 schema。退出后重启不自动弹向导；从设置重跑仍回到 welcome，且不清除有效设置。显式保存的设备/增强设置保留，未验证工具/快捷键回滚，其他快捷键偏好延至验证通过再提交。mapping 恢复失败会保留状态并重试；`MIC_CLOSE` 失败保留目标/session 身份，可再次清理。语音资源清理失败与设置恢复失败分别提示；失败仍返回主界面。用户显式发起的增强 enable/disable 操作归现有后台所有者继续处理，不属于向导 cleanup 的等待或取消目标；UAC/ShellExecute、`schtasks` 可能长时间不返回。操作期间用户可用主界面，状态提示仍处理中，保留原操作并记录 `passed`/`failed` 结果，迟到成功仍可开启增强；此时设置内重跑向导应明确拒绝，操作结束后可重试。真实 UAC 长等待返回主界面的行为仍 deferred，不声称所有 IPC 有界。跳过不等于遥控器连接、端点就绪、音频输入或语音链路通过。若本次有任一步 Skip，结束日志为 `skipped`。

文字指示只反映本次输入框是否出现文字，不代表整体验证成功；retry/leave 清零，不记录输入内容。对“没有采样但出现真实语音识别文字”的根因仍未知；诊断只记录布尔状态与 attempt/generation/route，不采集文字内容。最终验证：前端28文件388 passed、types/Vite build passed；Rust workspace 716 passed/23 ignored、独立子进程诊断1 passed、fmt/workspace check/runtime-simulation check passed；UAC/增强迟到结果与提示窄测87 passed。真实 UAC 长等待期间返回主界面尚未实测，不能称所有 IPC 有界或授权命令已取消。源码基线 `af710b7` dirty；最终审阅 payload fingerprint `c678ffe7fa32fb67f84714cda733604195be2832ee94974ab2a59c6c3e9160b8`，18/18 被审代码 hash 一致。

2026-10-13 布局后续：退出按钮移到窗口内容顶部的进度栏最左侧，与居中的“准备 › 设置 › 试一下”同行；正文“返回”保留原位，Skip/Continue 仍固定 8px 间距。真实 Edge 测量在 1067×800 与 1150×800 共覆盖 26 个步骤/门禁状态及 2 个 busy 状态：退出可见、与阶段标签不重叠，首步无返回时退出仍可用，busy 时退出禁用；返回位置未变、底部间距为 8px、页面无横向溢出。前端 393/393 passed，`vue-tsc` 与单次 Vite build passed。最新 unsigned 本地 NSIS 候选 `无线麦_0.8.6_x64-setup.exe` SHA-256 `263cea3bdde0625b6f4ece232fde3a792f73ed6c8da87143b33b9376ee5ab7b9`，前端输入 fingerprint `6feaf7139614bdca0a1fd7160cff2ba4954d926f9bef74def278df191795d775`；内容门禁、4 项安装载荷及生产构建隔离核验 passed。raw/installed EXE 仅差 NSIS 三字节标记。管理员覆盖安装后由 Explorer desktop ShellExecute 启动安装版，根 PID 31636，Session 1、Token 0；运行身份与 dirty/local/included 候选匹配。安装后未打开或点击向导，实际窗口位置及用户目视复验仍 deferred；Tab 焦点环和 busy 态视觉未验。

2026-10-13 向导语音设备补充：音频步骤现展示与设置页共享的“遥控器说话时临时切换麦克风输入”持久开关，按 `capture_input.enabled` 快照回显；选目标仅设置语音通道，不会立即改变 Windows 默认麦克风。启用时沿用既有语音键按下临时切换、释放恢复语义；软件自行指定麦克风的工具不受此开关控制，未改 Rust 生命周期或退出清理。新增保存成功/失败回退、持久状态、busy 与重复输入保护测试；向导定向 65/65、完整前端 397/397、`vue-tsc` 和单次 Vite build passed。独立 headless Edge 直接挂载真实 `AudioStep.vue`，以已保存 enabled=true/目标端点存在的 props，在 1067×800 截图确认勾选、说明可见且无重叠。unsigned 本地包 `无线麦_0.8.6_x64-setup.exe` SHA-256 `1ce8a7ee9a0c8595c9c8209e72a19212d22cc1882847e956216aa0cb45ff29e0`；前端输入指纹 `c3d284140c216f7a0218c50f6c507c49e61aa24b87f2e702216d1f4d0def8b69`，安装运行 source fingerprint `f7c679a6ed822b9a2cbcf1a335fdc3ab06140bef6034825410b81657e95e2b99` 与 dirty/local/included 来源匹配。覆盖安装后由 Explorer desktop 普通权限启动根 PID 53020、Session 1、Token 0；前后设置 SHA-256 `ef2c192517ab16d9bed813ef8741d2c2a8ecab495703d5d1f293fae16ef2dac3` 相同，enabled 与目标/写入端字段均保持。未重测 Windows 音频角色切换或真实语音时序；不将组件截图、自动化或配置保持扩大为硬件验收。

## 隔离 Windows 模拟验收

在真实 Windows WebView 与 Tauri IPC 上运行隔离 walkthrough，14 项检查 `passed`：七步全 skip、七步分别退出、Settings 重跑、门禁仍保持原状，以及未验证 staged 工具/和弦绑定回滚。报告 `target/dev/onboarding-skip-exit/simulation-report.json`；结构化运行日志、驱动及正常关闭日志分别为 `simulation.log`、`simulation-driver.log`、`simulation-close.log`。驱动实际回放 43 项计数仅作模拟证据，不代表遥控器/语音硬件验收。模拟构建身份为 local/not-bundling，source fingerprint `6f7df84e990d89860cc828430e3e18d02768a768e49f3155941299e5768f3698`；App PID 40064、键盘子进程 41204、Vite 52508 均正常退出，App 进程数为0、Mutex 不存在、2430/9222 无监听。隔离 WebView 缓存与多余11张截图已送回收站；保留报告、驱动、日志、最小隔离状态和以下三张必要截图： [voice_test Skip](../target/dev/onboarding-skip-exit/skip-voice_test.png)、[voice_tool 退出](../target/dev/onboarding-skip-exit/exit-voice_tool.png)、[完成页 Skip](../target/dev/onboarding-skip-exit/skip-complete.png)。

此模拟不替代真实 UAC 长等待、正常音频设备配对/语音链路或 RC001、RC003 真机验收；它也未锁定原始“识别有文字但无采样”问题的根因。

## 本地候选安装与启动读回

NSIS 本地候选 12,878,070 B / SHA-256 `b8cc4ade31b207951caf900251dd1daae52f850588cb69246c27d8f1e5d8b809` 已覆盖安装并由真实 Explorer 启动一次。运行版本0.8.6、base `af710b757897cb0567b63c63a7ceb40f62426652`、dirty/local/included、fingerprint `1db54b7b61757f851d74f0e7732767168ff7df3e237830c0da573d5bffc83b06`；根进程31220与子进程21488均来自安装目录、TokenElevation=0，主窗口可见且未最小化，运行时身份字段匹配包身份。payload 4项及10个前端资源匹配 receipt；NSIS 构建隔离核验 passed。raw EXE 20,821,504 B / SHA-256 `5d9e9c75e8f799d1e8617c0b74c5f9e9344712746704ccdf538190b1ec199fe4`，installed EXE 同大小 / SHA-256 `0f1391ffa63dca07c26c11b0ec9e7aceeaf44745aff8ed279a58196e8722439`；动态全量比对仅在 `0xED3A42` 有连续三字节 `UNK→NSS`，其余字节相同。**前端实际嵌入审计纠偏**：该安装包装入旧 `dist-build`，而 `pnpm build` 产物写入 `dist`；本地 Tauri 配置指向 `../dist-build` 且 `beforeBuildCommand` 为空。`verify-frontend-embedded` 仅检查路径字符串，不能验证打包内容一致；审计证据 `target/dev/onboarding-skip-exit/frontend-embedded-audit.json`。所以该包的进程、Rust载荷和安装身份虽读回，不能证明当前新向导前端已安装生效；此前388前端测试与14项隔离 WebView/Tauri模拟仍分别有效，但不属于 B8 包前端实测。B8失败对照 `failed-b8cc4ade-setup.exe` 已核对保留。NSIS 包是本地 unsigned 候选：Tauri 总命令因缺 updater 私钥失败，无 `.sig`，不得当作发布包。

启动时仓库基线已有的 `startup_recovery` 按 `staged.revert_tool/revert_voice_hotkey` 清理未验证选择。实际读回为 flow 1、completed 0、step `voice_test`、migration 1、staged null、工具未设置、快捷键值2；Page 1554 重新读正式工具/快捷键并重置验证。此处没有手工写回配置或执行迁移。安装前只存有配置 hash、没有内容快照，因此不能声称配置逐字节保持；音频不在这段恢复写入范围，也不能从 hash 推断音频字段没有变化。schema 2模板/7绑定只有当前读回，缺少前快照。这是B8旧前端启动时的历史状态读回；当时用户反馈检测异常，不能代表后续修复候选的向导状态。新候选启动后工具/快捷键读回及当前PC用户验收见下文。真实 UAC 长等待与 RC001/RC003 硬件验收仍 deferred。

## 前端载荷核对与最新语音检测事件

部署后用户反馈检测仍异常。只读审计确认此前 B8 包实际嵌入旧前端 `dist-build`，而构建命令写入 `dist`；local Tauri 配置使用 `../dist-build`，没有 `beforeBuildCommand`。旧 `verify-frontend-embedded` 只验证资源路径字符串，没有比较嵌入资源内容。证据 `target/dev/onboarding-skip-exit/frontend-embedded-audit.json`；因此不能把 B8 安装/启动说成新向导界面通过。此前前端388项自动化及隔离 WebView/Tauri 14项仿真没有被推翻，但仅证明各自源码和隔离构建；不代表 B8 安装包嵌入了同一前端。打包目录及内容门禁已修复并通过独立审查；修复后定向门禁12/12 passed，源码输入 fingerprint `89bed8588d979dbcaa26e4bb69ec23e312bf78825835abe1c10969afb120e6ec`，单次 `pnpm build` passed，当前 HTML 回执 SHA-256 `52ca8bcd996cd5a4598c0d9ee1987f166252339fab3bd3cbc396b4bc3c8a6034`、入口资源 `assets/index-nMprRGS7.js`。两条本地构建入口共用门禁：`scripts/build-local-input-candidate.ps1`、`scripts/build-local-test-package.ps1`。门禁检查从统一 `dist` 输入、fresh 内容回执到实际 EXE 中唯一 OUT 的完整压缩字节及解压后的 HTML/脚本图；此前 B8 raw EXE 被新门禁以 `compiler_input_mismatch` 拒绝。

## 修复候选构建与覆盖安装

新本地 NSIS 候选 `target/release/bundle/nsis/无线麦_0.8.6_x64-setup.exe`：12,879,683 B，SHA-256 `a990aecdbddcd6b1621fccad39c501c9dc5880741ffed01c58dc930f40a805e8`。构建实际 EXE 20,821,504 B，SHA-256 `ea28f456092a669fc71a3d425ff1560d8d554d6cba0f07181253993db4a5ea5`。实际 EXE 前端内容门禁 passed，输入 fingerprint `89bed8588d979dbcaa26e4bb69ec23e312bf78825835abe1c10969afb120e6ec`，HTML SHA-256 `52ca8bcd996cd5a4598c0d9ee1987f166252339fab3bd3cbc396b4bc3c8a6034`，资源 `assets/index-nMprRGS7.js`；runtimeSimulation=false。仿真隔离、bundle metadata、4项原生 payload、PE subsystem、NSIS unsigned 检查均 passed。5.1 PowerShell `Contains` 重载兼容修复已在既有 PowerShell 7 环境通过同脚本验证。身份为 base `af710b757897cb0567b63c63a7ceb40f62426652`、dirty/local/included、sourceCommit=null、source fingerprint `20226cceeccfd35d4dd9194723979a079e62a15b52adcc0f86715f989f658beb`；构建身份和安装前部署状态见 `target/dev/onboarding-skip-exit/build-identity-fresh-dist.json`、`preinstall-deployment-state.json`。

该候选已覆盖安装并由真实 Explorer 正常启动一次：App 根44952/子41248来自安装路径、Session 1、TokenElevation=0；Helper 31972 存在。运行 source fingerprint `20226cceeccfd35d4dd9194723979a079e62a15b52adcc0f86715f989f658beb` 与 release snapshot 一致。raw EXE SHA-256 `ea28f456092a669fc71a3d425ff1560d8d554d6cba0f07181253993db4a5ea5` 与 installed EXE SHA-256 `4953bf00ba1baa36ff2766a1abff122a8f598336531ba21c5b0408027d37333a` 仅在 offset 15549282–15549284 有一组 `UNK→NSS` 三字节标记，其他字节一致；由此确认经门禁核对的新 `dist` 前端载荷连到了安装运行 EXE。日志包含新 UI 字符串及 runtime 诊断，但日志本身不代替可见 UI 验收；以下用户实测提供直接行为证据。

安装前正式 App 根31220/子21488/Helper34104均正常退出，日志 overall passed、failed stages 0，supervisor/input/raw-input/BLE 阶段完成，无强杀。新实例日志（北京时间2026-10-13）记录 14:09:46 `voice_test reason=user_continue`、语音门禁 `passed`；14:10:17 controls；14:10:18 complete；14:10:20 binding verified（`hotkey_count=2`、`tool=wechat`）、`event=completed flow=1`、`wizard_finished`。安装后向导读回 `flow=1/completed=1/step=complete/staged=false`，与上述事件一致；日志本身不能证明操作者身份或识别文字来源。按键配置 JSON 的 `keys` 数组实际长度为2；此前对象计数1的误读已撤回，不记为配置丢失。没有安装前全部配置内容快照，不能称全部配置字节保持。Tauri 总命令在 NSIS 生成后因缺 signing private key exit 1，无 `.sig`；此包仅 unsigned local candidate，不是签名或发布成功。

用户随后确认本包、当前 PC/此次遥控器观察 `passed`：遥控器识别文字后文字项打勾且继续可点；退出向导可用；七步各自“跳过此步”均正常。该结果不外推另一型号设备、睡眠/长期稳定或完整硬件矩阵。用户另提出将 Skip 按钮放到 Continue 左侧并排；已在 `OnboardingPage.vue` 用横向动作组实现，提示条位移按用户要求暂留。前端393项、typecheck、单次 Vite build 与 DOM/CSS 核对均 passed；新构建输入 fingerprint 为 `492fa621e6873b98027060b969cfbe78aefffcfd970ef465f14f37b0207adbf8`，HTML SHA-256 `f314c26e0c9e6b84be3e85332120bcbed626becc82ffb54c4e700bbd35bf859b`，入口为 `index-B95e6zLV.js` / `index-D9ZG9tam.css`。这些检查不含真实浏览器几何/截图观察。布局候选 NSIS `无线麦_0.8.6_x64-setup.exe`（12,879,683 B，SHA-256 `e5e0e29caf0b827bd10af0a7d2c62826aa11e63e9f1ffb7875c5833ab6de9630`）已覆盖安装并由 Explorer 启动；安装 EXE 与 raw EXE（20,821,504 B，SHA-256 `052aa1d40e1d3a4047cc52cc11bc6ca285d64f559b9734b3de4c07fcc5bbf616`）仅有 offset 15549346–15549348 的 `UNK→NSS` 三字节标记差异，其余字节一致。实际前端门禁及4项原生 payload 核验 passed；运行 source fingerprint `81d572742528928445dd6e55ecf96cd9410663f5d1b403e185a65b0a033be988` 与 dirty/local/included 包身份匹配，前端输入 fingerprint 为 `492fa621e6873b98027060b969cfbe78aefffcfd970ef465f14f37b0207adbf8`。根进程44536/子进程55256来自安装路径、Session 1、TokenElevation=0，Helper 55164 存在；旧实例正常退出。用户随后实测 E5E0 布局 `failed`：Skip 与 Continue 虽同排，但部分页面两按钮相距过远。针对 E5E0 的间距失败已完成布局修正并通过独立 review：提示文案移至按钮行外，Skip/Continue 固定 8px 间距；顶部“返回/退出”同行，退出复用返回样式。真实 Edge 对 26 个状态、1067×800（支持的最小窗口）及1150×800尺寸核验通过：按钮组同排、完整位于内容区且无横向溢出；welcome 隐藏返回并设 tabIndex=-1，退出可见可聚焦；ready/blocked 主按钮启用状态正确，退出与提示相关逻辑未变。前端393项、typecheck、单次 Vite build passed；新构建输入 fingerprint `f284f0b6f33e1bb6e0afde000639e021589dca61b3e2e89b186501b4eb1ff6fa`，HTML SHA-256 `774421f64acd6a0a31f969ea6a4e9cd461b87cc35ff82a35fe724fc6d1d1b69a`。Tab 焦点环及忙态视觉尚未真测；提示条位置变化仍按用户要求暂留。E5E0 的用户布局失败历史保留。修复版 NSIS 候选（12,879,454 B，SHA-256 `1edd92c0cfd3c3d0290c9e8d1769de70c2395d3902bdb495d36f76b0ebb95a08`）已通过安装器、actual EXE gate、包身份与4项原生 payload 核验并覆盖安装；经已核实的真实 Explorer desktop `Document.Application.ShellExecute` 启动（desktop HWND属 explorer.exe PID 12848）后，主 PID 54292 父进程为 explorer.exe，子 PID 43136与其同安装路径；二者均为 Session 1、TokenElevation=0，Helper在线。运行 fingerprint 与候选一致。raw EXE SHA-256 `6762ecc4a43a4f2afc151b8fb53b38dd95e83fb845820d69a1b4216567716899`，installed EXE SHA-256 `326e780e345ef4df3015cb845497cd2de0e8dcca02986be4d003f65cd9917d0`，仅 offset 15549346 有 `UNK→NSS` 三字节差异。运行 fingerprint `79d648cd13b82c5f049449db851ed89db73c7c01acc35cb9cc3f6d7035ba93cd` 匹配 dirty/local/included 包身份；runtime `capture auto_trigger` 和 `helper_connected` passed。四份存在的配置（settings/onboarding/button-mappings/voice-hold-hotkey）安装前后哈希一致；other-voice-hotkey 缺省。部署材料见 `target/dev/onboarding-skip-exit/preinstall-deployment-state.json`。

旧 E5E0 正常退出的 App overall `failed_stages=1` 原样保留：App 首次读 Helper 回执时 `valid/matches_process/matches_start=true` 但 `settled=false`；同 Agent/stop_id 的 Helper 随后写入 ACK 终态 passed，含 `hook_detached/released_all`，reviewer确认本轮实际释放。App 的 `QueryFullProcessImageNameW` 路径核验失败后提前返回，未读后到的终态；没有 Win32 错误码，精确 API 失败原因未知。回执无持久 epoch，只能以 PID/start/提交时间关联，不能据此证明驻留模块卸载。该只读确认未修改捕获代码；捕获生命周期开发任务仍暂停。新安装窗口未自然显示，因此当前安装版 Skip/Continue 与顶部返回/退出几何及用户复测 deferred；没有点击或唤起向导。不要把此前 E5E0 的用户布局失败当成本轮新布局的结论，也不要称本轮用户已确认。语音文字勾选/继续、退出功能及七步 Skip 功能仍保持先前用户实测 `passed`；此处失败只针对布局，提示条位移继续按用户要求暂留。该候选为 unsigned local overlay（复用本机候选配置），未改更新源。

最新脱敏事件 `target/dev/foreground-input/incident-detection-20261013T0510Z.json`（白名单24事件）：attempt `id=2` 05:05:47.288 armed，05:05:57.922 `no_samples`（d0/s0）；邻近 native session 92 约64ms 后 started，05:06:02.82 generation 2 实际 decoded/submitted 76,800 samples 且队列0。两者没有直接 attempt↔native session 关联，不能据邻近时间断言是同一链路。原日志仍显示用户看见识别文字但无采样的症状，根因不能仅凭此事件配对得出。

独立回归已确认另一具体源码缺陷：同帧混合 voice idle/audio streaming 可能被读作 start+end，idle/draining 也可能提前进入 `delivery_failed`。最小 ended 一致性修复最初先红4、原69绿，再73项通过；reviewer 随后发现相邻 focus check 可绕过 mixed-start，作者增加第5条回归并修复后，最终窄套74/74通过，完整前端393 passed、类型检查通过，独立审查接受冻结的4个源码/测试文件。此结果只覆盖源码和前端；打包目录/内容门禁仍在开发审查中，不能把393项称为打包门禁或已安装验证。B8失败对照包已核对保留，新包最终身份/安装与用户复验待实现者提供证据后增量更新。

2026-10-13 现场 voice_test 失败证据：用户从“重新运行首次向导”进入后卡在 `voice_test`；现场摘要显示 `onboarding completed=0`、当前步骤 `voice_test`、`staged` 存在。截图可见识别文字，用户确认文字由遥控器语音产生、不是键盘输入，但页面仍显示 no audio / 设备未就绪且继续按钮禁用。现场摘要 `target/dev/foreground-input/onboarding-voice-gate-incident-evidence.json`（4,749 字节）只保留脱敏状态，不记录实际输入内容。日志显示多次语音 stream start，但缺少 attempt、音频帧输出与门禁结果之间的完整关联；无采样与文字同时出现的原因尚未锁定，stream start passed 不能推出音频验证通过。实现者新增的布尔及 attempt/generation/route 诊断不记录文字；完整测试/安装及真机复验待执行。

## 测试前准备

1. 记录提交 SHA、测试包路径与版本号、Windows 版本、VB-CABLE 版本。
2. 安装本地测试包（保留既有用户数据）。本构建已打开向导开关；老机器走「设置 → 首次设置 → 重新运行向导」进入。
3. 打开「权限」页确认日志路径，清空旧日志后开始。
4. 每个工具测试前按向导第⑤步的核对卡逐项确认：工具麦克风 = CABLE Output；工具内语音键与 SayAll 一致；Vokie 保持运行；豆包需开启「支持更多输入工具」。核对卡是编号提示行（**没有勾选框**）——放行只看真实文字上屏。

## 界面结构（2026-10-05 改版，参考 Mac App）

- 顶部仅一行面包屑「准备 › 设置 › 试一下」+ 一条细进度条；页内不再重复产品名。
- 布局自上而下：标题固定在内容列左上角**同一位置**（没有上一步的步骤保留同高占位），
  切换步骤/内容变化时页面不抖动；内容过长时在内容列内滚动。
- 内容列左上角是「‹ 返回」（chevron 图标 + 文字，与正文左对齐）；**没有整幅页脚横幅**。
- 底部动作行在内容列内部：左下「复制诊断信息」（无下划线，含复制反馈），右下主按钮
  （`继续` / `开始使用`，门禁提示语在按钮上方、右对齐）；右侧栏通到窗口底部。
- 字号：标题 27px、正文 14–15.5px、卡片标题 16px。第①步含「第一次使用：遥控器怎么连电脑」
  卡片（长按 TV 配对 / 主页+菜单 重置 / Windows 高级选项），与第②步的详细说明一致。
- 第⑤步通过提示为绿色成功卡 + 对勾；最后一步右栏显示应用 logo + 对勾徽标。
- 右栏 = 插图 + 检查卡（按步骤换标题：连接检查 / 音频检查 / 工具检查 / 实时检查 / 按键检查）；
  窗口过矮（高度 ≤ 660）或过窄（宽度 ≤ 1024）时检查卡折到内容下方。
- 按键检测步骤（②遥控器 / ⑥普通按键）期间会**暂挂用户已配置的自定义按键**：只收集
  边沿，不触发任何映射动作；离开这两步立即恢复（日志 `mapping_suspend` / `mapping_resume`）。

## 诊断信息（2026-10-05 新增）

1. 任意步骤点左下「复制诊断信息」→ 应显示「已复制，可直接粘贴发给开发者」；粘贴到记事本检查内容。
2. 文本为**英文**（`SayAll diagnostics` 开头），必须包含：App version、Build（40 位源码修订）、
   Build channel、**Windows version（major.minor.build）**、Architecture、`Wizard step: <token> (n/7)`、
   Gate、Blocked、Remote / Audio / Input tool / Vokie / Buttons observed / Mapping。
3. 隐私检查：不得出现蓝牙地址、HID 路径、音频端点 id / 名称、文件路径、用户名或任何输入文字。
4. 日志核对：`frontend event=onboarding phase=action reason=copy_diagnostics` 成对出现
   （begin `result=unknown` → end `result=passed detail=chars_N`）；剪贴板不可用时 end `result=failed`。
5. 启动日志核对：`app_lifecycle event=process_start ... windows_version=<major.minor.build> windows_build=<build>`
   —— 不再是 `unknown`。

失败判定：复制提示缺失；文本缺 Windows 版本 / Build；出现设备身份或路径；日志缺 copy_diagnostics 对。

## 用例一：进入向导与中断继续

1. 全新状态（或删除向导状态文件）启动：应直接进入向导，停在第①步。
2. 老用户状态启动：不应进入向导；走设置页重跑入口 → 应进入向导并停在第①步。
3. 从任一步选择退出/稍后设置：返回主界面；重启应用不自动弹向导。之后从设置重新运行向导，应从 welcome 开始，既有设备/增强设置保留。退出时先清理活动语音/按键观察/映射暂挂，等待 pending 写入后回滚未验证 staged 工具/和弦并同步 runtime；终态无法确认时显示警告。当前 step 保留供状态读取，但不表示验证完成。
4. 在向导里点内容列左上角的「← 返回」：回到上一步；不丢进度、不自动前进。
5. 每一步都选择 Skip 到下一步，抵达完成页后选择“跳过验证并开始使用”：回到主界面，步骤留为 `skipped` 而非 `passed`，不置语音验证状态；核对语音会话取消、观察关闭、映射恢复、pending 写入收尾、未验证 staged 工具/和弦回滚及 runtime 同步。

预期：全新用户按现有首次进入规则打开向导；老用户可从设置重跑。每步有独立 Skip 且原 Continue 门禁不变；Skip 只记 `skipped`，不置 `voiceVerified`，不把硬件步骤记作 passed。完成页可选择“跳过验证并开始使用”，同样不伪报验证通过。仅正常验证通过提交 staged；跳过/退出回滚未验证工具与快捷键，不清除显式保存设备/增强设置。若其他工具快捷键偏好尚未经验证，不提前提交。`completed_version=flow_version` 只表示本版流程已处理；有任一步跳过时结束日志为 `skipped`。临时资源清理成对执行；清理/保存结果未知则警告，不阻止返回主界面。

失败判定：首次运行应进入向导却停在主界面，或已处理本版流程的用户重启时被自动弹入；退出/稍后设置不能返回主界面；任一步没有独立 Skip；Skip 置 `voiceVerified` 或伪报步骤/硬件 passed；重新运行不从 welcome 开始或清除任何显式保存有效设置；退出/跳过后语音、观察、映射暂挂或 pending 写入未按顺序收尾；未验证 staged 工具/快捷键未回滚/同步；无法确认清理/保存却不警告。

## 用例二：步骤①–③（欢迎 / 遥控器 / 语音设备）

1. 第②步点「打开蓝牙设置」：系统「蓝牙和其他设备」页应真实弹出。
2. 第②步的「首次连接遥控器」卡应写清：长按「TV」约 3 秒进入配对；没有反应时同时长按「主页」+「菜单」
   重置蓝牙并重新进入配对；Windows 列表里找不到时先点开「高级选项」（或“更多蓝牙选项”）再刷新。
3. 扫描并连接遥控器；按一下普通按键：门禁从「按一下普通按键」变为可继续；按语音键不应误判为普通按键。
   此期间按已配置映射的键（如 OK → Enter）**不应触发**用户的自定义动作（检测步骤暂挂映射）。
4. 第③步：检测到带「推荐」标记的 CABLE 设备并自动选中；没有时给出安装与重启说明。设备不可用时用户仍可主动跳到下一步，但该步骤保持未通过，不得因此伪报端点就绪。

## 用例三（核心）：第⑤步「按住说话验证」，三种工具各跑一遍

对 豆包 / 微信 / Vokie 分别执行：

1. 进入第⑤步：输入框应自动获得焦点；核对卡（编号两行）显示本次工具与快捷键，没有可勾选的复选框。
2. 正常路径：按住遥控器语音键说一整句 → 松开。
   预期：实时状态「正在接收你说的话…」→「正在等文字出现…」；输入框出现文字；结果行显示「成功：文字已经出现在输入框里」；「继续」变为可用。正常验证路径保持，另须提供明确主动跳过；跳过不产生 `voice_attempt passed` 或音频设备 ready 结论。
3. 阴性对照：不碰遥控器，直接用键盘在输入框打字。
   预期：判定失败，文案「检测到键盘输入。这一步请只用遥控器语音键，不要用键盘打字。」；「重新测试」清空输入框和本次文字布尔后重新开始，再跑一次正常路径应通过。离开/Skip 后同样清零，不沿用上次 attempt 的文字状态。
4. 焦点用例：先点输入框以外的地方（失焦）再按住语音键 →
   预期：失败文案「输入框没有聚焦。先点一下输入框…」；测试过程中让窗口失焦 →
   预期：失败文案「测试过程中输入框失去了焦点…」。
5. 快速点按（不足 0.5 秒）：应给出可见失败并可重试，不出现卡死或空白状态。
6. **键盘右 Alt 路径（2026-10-04 钩子迁移修复的核心验收，见
   Bugs/2026-10-04-ll-hooks-break-in-app-ime-voice.md）**：应用窗口前台、焦点在输入框时，
   按住键盘**右 Alt**（即所选工具的按住说话快捷键）→ 语音条应出现；松开 → 语音条消失。
   此前"前台根窗口进程 == LL 钩子所在进程"时该路径必失败；修复后两个 LL 钩子均在
   按键宿主进程（`sayall-windows-app.exe --sayall-key-host`），宿主永不持有前台窗口。
   日志核对：`key_host action=hook_report detail=HOOK kind=f5 installed=1` 与
   `detail=HOOK kind=gate installed=1` 各一条；若缺失，钩子未安装、判定降级。
7. **门控回归（切片 3 门控迁出后）**：遥控器已映射按键（如方向键 / OK）在映射启用时
   仍触发配置动作且不残留原生按键；按住说话快捷键注入成对（按下 DOWN / 释放 UP）；
   断连或退出应用后无粘键（同一按键连按无卡死）。验收时 helper 开 / 关各跑一遍：
   门控迁移与「全按键支持」helper 相互独立，helper 关闭不得影响上述行为。
8. 日志核对（每次 attempt 必须成对）：
   - `frontend event=onboarding phase=voice_attempt result=unknown reason=armed …`（一次 attempt 一条）；
   - 终态一条：`result=passed` 或 `result=failed reason=voice.*`，`detail=d…_s…_q…_drain0|1` 为采样 / 投递 / 队列计数；
   - `input_observation … action=begin/end` 成对出现；`action=end … count=0`（正常路径）。
   - 日志不得包含任何输入文字内容。

## 分步日志（卡住定位速查，2026-10-05 起）

向导每一步都落 `frontend event=onboarding` 结构化日志；**用户报"卡在某一步"时，
按时间取最后几条日志即可定位环节**。约定：

- `phase=action` 成对出现：begin 时 `result=unknown`，end 时 `result=passed|failed`
  且带 `elapsed_ms`；**有 begin 没有 end = 卡在该外部调用**。
  各步的 reason 速查：
  - ② 遥控器：`button_observation`（按键边沿订阅）、`scan_requested`（扫描，`detail=found_N`）、
    `connect_requested`、`open_bluetooth_settings`、`mapping_suspend` / `mapping_resume`
    （检测期间暂挂自定义按键）；
  - ③ 语音设备：`refresh_endpoints`（`detail=auto|manual_total_N_rec_M`）、`select_endpoint`
    （`detail=auto|manual|fallback`）、`open_download_page`；另有 `phase=audio_route`
    （`reason=missing|not_selected|selected|ready`，状态去重）；
  - ④ 输入工具：`read_tool_state`、`stage_binding`（`detail=<工具>`）、`save_other_keys`、
    `vokie_detect`（`detail=installed_0|1_running_0|1`）、`open_vokie_site`、`vokie_launch`；
  - ⑤ 按住说话：`armed` / 终态 / `session_stopped` / `observation_begin_failed` 等
    （见上一条核对）；
  - ⑥ 普通按键：`button_observation`、`mapping_suspend` / `mapping_resume`；
  - ⑦ 完成：`complete_refresh`（失败时 `detail=tool|vokie|capture|audio` 指出来源）。
- `phase=heartbeat` 每 30 秒一条（`reason=alive`）：携带当前 `step`、门禁 `code` 与
  步骤内等待摘要（如 `obs_0_n0`、`streaming`）。**长时间停留某一步时最后一跳就是现场**；
  这是"状态未变化不重复刷"的刻意例外。
- 脱敏红线：所有 `step/code/detail` 均为稳定 token 或计数，不含设备 id/名称、
  音频端点 id/名称、文件路径、错误原文与任何用户输入内容（自动化用例对
  遥控器/端点身份字段做了"绝不出现"断言）。

失败判定：正常路径判定失败或看不到文字；键盘打字仍判通过；焦点类失败文案缺失；快速点按导致卡死、重复 attempt、或出现两个终态；日志缺 attempt 配对或含文字内容。

## 用例四：步骤⑥⑦（普通按键体验 / 完成）

1. 第⑥步：按除语音键外的 3 个不同普通按键（如 主页 / OK / 方向键）→ 已按下的键在芯片格里点亮、进度点走到 3/3 后可继续（同一键重复按不计）；期间遥控器普通按键不应执行用户已配置的映射动作（临时暂挂）；按语音键出现「这是语音键…」纠偏、且不计入。
2. 正常完成：全部门禁通过后，第⑦步清单全绿，选择「开始使用」；提交已验证 staged 后回主界面「连接」页。另测显式“跳过验证并开始使用”：回主界面但不伪报全绿/硬件通过；若此前有任何 Skip，结束日志为 `skipped`，未验证 staged 工具/和弦回滚。
3. 完成后再从设置页重跑入口进入：应回到第①步（不影响既有设置）。
4. 断开遥控器后打开第⑦步（若可重现）：清单对应项转红，给「去修复」跳回对应步骤。

预期：第⑥步的暂挂只影响向导会话，离开后映射恢复；正常验证通过后才提交已验证的 staged 工具与和弦，未验证快捷键偏好只在验证通过后提交。

失败判定：暂挂期间仍执行映射或退出后映射不恢复；跳过步骤被计为验证通过；未验证 staged 工具/和弦未回滚；已验证提交后的运行态与第④步所选不一致；skip、finish、exit 或稍后设置后的日志/状态不符合各自语义。

## 用例五：两种型号分别记录

- RC001 与 RC003 各自完整执行用例一 ~ 四；分别写明每步结果与失败码。
- 任一型号任一用例失败即整体判失败，不用另一型号的结果顶替。

## 证据与日志收集

- 每轮结束导出 `sayall-diagnostic.log` 相关时间段；报告包含提交、构建路径、Windows 版本、型号、工具、步骤、失败码与日志片段。
- 脱敏边界：不复制语音内容或识别文字；不记录真实设备地址、HID 路径、端点 ID / 名称。

## 模拟硬件信号（无真机跑向导；2026-10-05 新增）

没有物理遥控器时用模拟信号跑完整向导，用于日常回归与问题复现；**不替代**上面的真机用例。

装置：`GetSayAll/hardware-simulation`（Windows 实现）把 Profile / Scenario 导出成"应用信号脚本"，应用在 `runtime-simulation` 构建里按脚本时间线回放，事件走**生产**解析链路（HID 报告 → `decode_report_usages` + `ButtonStateMerger`；ATVV → `AtvvVoicePipeline`）：

```powershell
# 1) 生成信号脚本（任一 scenario 均可）
hardware-sim export-app-script <profile.json> <scenario.json> --out script.json
# 2) 构建仿真可执行文件（一次性）
cargo build -p sayall-windows-app --features runtime-simulation     # 或 --release
# 3) 起前端 dev server（仿真二进制的 devUrl 指向 2430）
pnpm dev
# 4) 回放：隔离状态目录 + 指定脚本（应用会保持运行，便于界面走查）
powershell -NoProfile -ExecutionPolicy Bypass -File Testing\run-hardware-signal-script.ps1 `
    -ScriptPath Testing\hardware-scripts\rc003-onboarding-walkthrough.json -StopExisting
```

- 状态隔离：`SAYALL_RUNTIME_SIMULATION_STATE_DIR` 下生成独立的 `settings.json` / `onboarding.json`，**不动**用户的真实向导状态；向导每次从第①步开始。
- 覆盖：② 的按键门禁、③ 的端点推荐与自动选择、④ 的工具选择、⑤ 的会话与解码采样（`voice_attempt` 终端码）、⑥ 的 3 个不同按键、⑦ 的提交与收尾；日志含 `hardware_script action=apply ... terminal_result=` 逐条记录。
- 转写文字：模拟轨道没有输入法注入，第⑤步的转写由驱动写入输入框并派发 `input` 事件（真机仍必须真人说话）。
- 已跑通记录（2026-10-05，RC003 scenario）：①②③④⑥⑦ 全绿，⑤ `voice_attempt result=passed detail=d240_s240_q0_drain1`，收尾 `wizard_finished elapsed_ms=71920`；截图与日志摘录见 `artifacts/local-test/2026-10-05-onboarding-simulation/`。
- 边界：模拟 ≠ 真机——配对、射频、权限、驱动、真实音质与安装包行为仍按本手册真机执行；两种型号仍要分别做真机用例一~四。

## 验证边界

- 本手册只覆盖向导；语音质量、按键映射日常行为、安装升级矩阵仍按各自手册。
- 自动化（`pnpm test` / `cargo test --workspace` / runtime-simulation 检查）只能证明代码路径，不能替代本手册的真机步骤。
