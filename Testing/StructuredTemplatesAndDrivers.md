# 结构化模板、场景遥控与驱动本地验收手册

本手册用于本地测试包的结构化模板、场景遥控、应用适配和受控驱动能力验收。它不替代 RC001、RC003 真机记录，也不把编译或仿真结果表述为真机通过。

> 2026-09-14 用户恢复固定语义模板：当前统一目录固定置顶 Agent、聊天工具、浏览器三个内置模板；本体只读且可复制为用户语义模板。只有用户开启完整按键模板跟随且当前前台绑定为 semantic 时才运行场景手势，direct/未绑定仍走普通映射。本文 2026-09-11 的隐藏/延期内容保留为历史阶段记录；当前边界见 [PLAN](../docs/PLAN.md)。

> 历史包与证据留存边界（2026-09-19 用户确认）：9 月 11—14 日已结束的模板/UI 过程目录不再保留。历史包身份、必要源码哈希和测量数据归下节，原验证结论不变；原始安装包已清理，哈希不保证能重建相同二进制。三键、音频故障和 capture 输入锁定材料不在本次整理范围。

## 2026-10-04/05 手动当前模板与三入口同步

本次问题来自 `SceneController` 将所有手选都限定到原程序，并在外部程序变化/身份暂失时清空；旧测试甚至要求关闭自动跟随后仍清空。另有菜单开关清空手选和冷启动缺外部 token 时菜单不可开的限制。前端原“编辑配置”只编辑草稿，没有应用当前模板的入口，模板列表也未订阅实际映射结果。

现行规则：自动跟随关闭时，菜单手选在本进程内跨程序保留；重新开启恢复当前程序既有关联/覆盖规则。手选不新增跨重启持久化，默认关联仍只有菜单显式勾选并确认才保存；没有外部程序目标时不能写关联。配置刷新、菜单开关与页面重开不能覆盖有效手选。2026-10-05 用户调整：按键页的“当前使用”以灰色只读提示显示实际应用模板，与独立的“编辑配置”字段靠左依次排列在同一行；编辑配置不切换当前模板，实际应用状态订阅继续保留。模板整框强调色以映射线程确认的 `mappingNotice + revision` 为准，候选、悬停、草稿或迟到快照不代表已应用。

模板选择在锁外等待实际映射确认；超时、前台/配置取消、正常退出恢复旧选择并使迟到确认失效，失败不提前显示成功。菜单真实释放/焦点恢复及原保存默认流程继续保留；按键页只读提示依据确认结果更新。

独立复查另用可控交错证明：旧 Snapshot 回调在读取旧 profile 后暂停，新选择 B 已获真实确认，旧回调恢复却把引擎覆回 A；丢弃旧确认不能阻止旧映射已进入引擎。订阅处用单独短锁串行化“读取最新场景→提交映射→排队确认”，不持场景锁等待确认。新增测试在旧实现实际输出 A 而界面为 B，修复后实际输出 B；并复验既有菜单/按钮/前台序列。

自动化 `passed`：先用 4 项回归复现旧行为，再补确认超时/取消/退出的 red→green；场景测试 46 passed、1 原有 ignored。真实映射工作线程和既有串行 KeyGate 使用记录注入器，观察按键输出 `A→B→B→A→C`（按键页选 A、菜单选 B、跨程序仍 B、重开自动跟随恢复程序 A、显式通用 C），不是仅断言快照。前端全部 255 项、Rust 工作区、Windows 仿真编译检查、格式检查通过。录制注入器和模拟边沿不是实体遥控器或 Windows 实际键盘注入验收；RC003 实体输出与跨页观察、RC001 分别待验。

开发红绿日志、源码差异及本轮构建日志复用 `target/dev/template-audio-ui/`；用于验收对照，完成实体反馈后按 LOGGING 回收可重现的中间材料。本次不提交、推送或发布。

2026-10-05 00:28 本地交付：`0.5.0` 安装包 SHA-256 `908f6ea007decbe373d9dbf019a67433b13c6574fc7f56b2ab64d3dfa5824fbf`（13,028,181 字节），基于 `c089497133c7588b6ea4418d2e824d67a35890f9` 加未提交候选，不是该提交的干净构建。保留原修改，没有创建新提交。覆盖安装 `passed`，设置、按键映射、输入恢复 journal 三文件安装时字节保持；安装文件逐字节核对仅允许 NSIS 标记差异。真实 Explorer 启动、普通主程序令牌 0/独立 Helper 令牌 1及同会话核对 `passed`。Helper 重新构建但 Agent 源码与 Gadget 字节未变，未触发旧模块热重载实验。

`pnpm test` 255 passed；`cargo test --workspace` 的 Windows 库 357 passed/20 ignored、Host 99 passed/1 ignored，其他 suite 通过；`cargo fmt --all -- --check`、`cargo check --workspace`、`cargo check -p sayall-windows-app --features runtime-simulation`、生产前端/NSIS 构建与仿真隔离检查均 passed。真实 WebView/IPC 仿真 19 步于提交串行锁补丁前执行，锁补丁后另跑真实引擎并发回归与完整工作区，不将仿真表述为实体菜单通过。

新安装版三个页面的真实 PrintWindow 截图已检查：声音设备常规区只见目标输入/开关，高级写入端折叠；按键页当前使用与编辑配置分开；模板页此时实际为通用配置，故未高亮任何完整模板卡片，这是正确状态，不能用该截图声称切换高亮已实测。用户已收到实体模板与语音验收步骤，回应前保留 deferred。本地精确源码指纹与安装/令牌读回在同主题 `package-identity.json`、`deployment.json`、`runtime-verification.json`，三张页面图用于布局对照，验收闭合或替代后回收。

2026-10-05 01:42 按用户截图反馈，只删除模板卡片的紫色“当前使用”文字及其专用样式，保留 `aria-current`、实际应用状态和两像素强调色整框；没有修改选择逻辑。`pnpm test` 255 项、生产前端/NSIS 构建与隔离检查通过，新包 SHA-256 `19beb1794be4f2e57fe95d4541a512738bb369f5558d1747552d21bfc1e0d6d8`（13,025,640 字节）正常覆盖安装、三配置保持、普通用户启动核对通过。实际模板页截图看到 Agent 保持紫色边框、文字已消失，选中和未选中卡片高度一致；该新版 `templates.png` 替代前一张通用配置状态截图。精确源码指纹及当前部署报告原位更新，增量日志使用同目录 `label-*`，保留至本主题验收闭合；未新增 Rust 行为，沿用前述测试边界。未提交、推送或发布。

2026-10-05 02:01 按用户新要求，按键页“当前使用”改为灰色只读提示，去掉下拉及页面手选处理；与“编辑配置”固定两列靠左同一行。提示仍只读取实际 `mappingNotice`，编辑草稿不改变提示；未确认/读取失败不假报通用模板。移除已无生产调用的前端选择方法，保留菜单、后端选择和实际映射确认逻辑。相关展示/同步测试调整后 `pnpm test` 255 passed，生产前端/NSIS 构建与仿真隔离检查 passed。最终包 SHA-256 `7416eb9e7a6df64b63f257b59fb664b50ec608caf7e6dafded7437cf8ed60a01`（13,027,781 字节）正常覆盖安装，三配置字节保持；Explorer 普通主程序及独立提权 Helper、实际载荷核对 passed。安装版 UIAutomation 确认当前字段不可聚焦且不是选择框，编辑选择框可用、两个字段同一行且顺序正确；实际截图确认灰色 Agent 提示、没有下拉箭头、与页面内容左缘对齐。`buttons.png` 与现有部署/源码指纹记录原位更新，增量 `readonly-*` 日志按本主题留存期限保留；未新增 Rust 或实体按键/音频结论，未提交、推送或发布。

## 2026-09-27 现行默认模板与系统任务选择

最新实机：2313101f修订版Applications与Desktops再次TV取消均failed；用户明确要求停止排查。此前52047c89诊断证实ForegroundStaging提前清模式，有限握手修正后仍未满足取消预期，完整机制未闭合。软件10项及workspace352项通过不等于实机成功。当前代码保留，不继续此Bug调查/修复/复验，不外推首用/导航/确认，见 [Bug](../Bugs/2026-09-27-task-switch-tv-cancel.md)。

最新默认调整：TV Single=Tab、Long=任务视图、Double未配置；原“任务切换”（Ctrl+Alt+Tab）在动作编辑器中继续可选。当前三内置无用户覆盖，本轮不写配置，不修改副本。 2026-09-28 00:04包1dfdf2e7已正常安装，前端179、workspace352 passed/17 ignored及fmt/check/simulation check通过；App51184普通/Helper45724提权，三配置字节不变，用户已明确确认1dfdf2e7短TV Tab符合预期；长按打开复用未改实现的既有用户观察，不重跑或外推导航/取消。

2026-09-28新契约：长按打开系统任务界面后，TV仍按进入前有效模板的各手势动作执行，默认短按Tab；不再由物理TV DOWN硬编码Escape/取消。普通TV固定键/组合交回既有mapper，原窗口及ForegroundStaging阶段也不受任务导航守卫阻断；仅方向/确认等专用导航及显式任务动作仍核目标守卫，不缓存被拒绝的导航重放。原长按不附加Tab，外部离开取消持有周期，方向/确认导航保持。前端179、workspace353 passed/17 ignored、fmt/check/simulation check通过；00:36最终6584546a已正常安装并核普通App/提权Helper、配置字节未变。视图内短Tab实际效果待验；旧取消失败不改写成已修复。

三个内置 Agent、聊天工具、浏览器都可直接编辑。稳定 ID、固定目录顺序及程序关联保持；“复位模板”须确认，只恢复该内置当前版本默认。已有用户副本、其他模板、映射缺项和用户设置不被覆盖。导出自改内置携带正文；导入该正文生成独立副本，不覆盖本机内置修改；纯内置引用仍保留引用语义。

| 按键 | 单击 | 双击 | 长按 |
| --- | --- | --- | --- |
| 四方向 | 对应方向 | 未配置 | 未配置 |
| 返回 | Backspace | 未配置 | 未配置 |
| 确认 | Enter | 未配置 | 未配置 |
| 音量± | 系统音量± | 未配置 | 未配置 |
| 电源 | Escape | 未配置 | 未配置 |
| Home | Home | Ctrl+Home | Ctrl+End |
| TV | Tab | 未配置 | 任务视图（Win+Tab） |
| Menu / 语音 | 原菜单 / 按下开始释放结束 | 不变 | 原菜单 / 不增加阈值 |

普通快速键保持按下即执行及已有重复/UP；Home 单击需要现有双击窗口，TV 在释放时判短按或现有长按阈值判长按，不另设时序。Shift+Enter 保留为用户可选换行，不能将 Enter 描述为所有软件无条件发送。自定义确认双/长可能先交付原生 Enter，编辑器明确提示此限制；不扩 Helper 或按 VK 吞实体键盘。

任务模式只接受系统 Shell 进程的已知任务窗口类及本次窗口实例，真实前台改变再复核；未知身份/注入失败关闭本地模式。系统窗口激活不清原程序模板，完成/外部切走恢复真实程序默认。导航复用已交付原生同键，不重复 Enter；前台事件先到的关闭交接仍跳过一次已交付原生导航沿的追加注入，下一次已捕获按压不受时间封锁。无持续 Ctrl/Alt/Win 所有权，检测物理修饰键按住时拒绝任务快捷键，不清用户修饰键。断连/监听停止/正常退出只清本地模式，不向未知前台补 Escape。

软件证据：只读拒绝、任务路由缺失、前台先变原生后到均有预期失败回归，最小实现后转绿；模板页/按键页/catalog45项和完整前端179项通过；工作区348 passed/17 ignored、fmt/check通过。Windows WebView实跑发现第二WebView创建重入时AppState尚未注册，已改为先注册状态，再创建浮层；相关Tauri51项与真实Windows WebView/IPC仿真14步通过；首次仿真被既有安装版单实例保护正常拒绝，旧版正常退出后定位并修复状态注册缺口。最终生产安装另记，不安装仿真二进制。上述均不是实际 Windows 任务界面或 RC003 按键通过。 最终0.2.6包2026-09-27 22:35:16，SHA256 `a3586eedced4c46fbb8e55ed33ccd00d14f5d2acb59bf48885c26a074b1b668a` 已正常覆盖安装，App47868普通权限/Helper41732提权，三配置字节保持；新run来源pending_per_request。首组只在既有Codex→内置Agent测试首TV，不用Notepad测试字母模板，不插准备遥控键。

实际待验：微信独立非私人草稿 Confirm 首用/连续与键态；三内置编辑保存重开/单项复位且其他值不变；任务视图内TV短按Tab选择及Confirm；冷态首用。长按任务视图打开仅复用既有用户观察。再次TV取消已failed且用户停止排查，本轮不再要求复验。不得自动操作电源/无线电，也不读取第三方文字或配置。

## 历史模板验证证据

历史结论继续在本手册按主题维护，必要的机器可核对数据集中在[模板验证证据](investigation/template-validation-evidence.json)：`records` 以原候选短名定位构建身份和源码哈希，`isolated_browser_measurements` 保存四态 bbox、关联排序及真实 pointer 拖动观测，`historical_window_hang` 保存旧版无响应的原始诊断字段。下文提及的 `source-manifest.json`、`source-hashes.json`、`build-evidence.json`、`verification.json` 均指原记录，必要字段已提取；安装结果归正文，不再要求读取已清理的独立 JSON、日志或过程目录。源码身份可能是未提交快照，不能仅凭其基线 SHA 推断对应源码。

历史失败边界：2026-09-11 12:45 的 unified 候选在 UIA/PrintWindow 探针后出现 `WM_NULL` 2 秒超时、`Responding=false`；正常退出请求当时未令进程退出，未强杀，根因仍未隔离。后续新实例响应恢复、13:09 绑定回归可响应，只证明对应检查点；不能覆盖旧失败或推断所有 overlay/回调路径通过。完整 workspace 的日志 sink 隔离失败、错误过滤导致的 0 测试，以及未完成的第三方应用和双型号矩阵，仍按原记录保留失败或 deferred，不因整理转为 passed。

以下截图来自 2026-09-14 的隔离 Edge/mockIPC 预览，逐字节保留并复核不含用户配置、窗口内容或个人路径。大图请求 1304×962、实际 inner 1278×869；最小窗口实际 inner 1029×732。截图只证明可见布局，动作和持久化边界以测量及正文为准，不代表安装版 WebView2 或实体遥控器验收。

| 编辑目标 | 大窗口 | 支持下限 |
| --- | --- | --- |
| 通用配置 | [截图](../docs/screenshots/template-validation/common-1304x962.png) | [截图](../docs/screenshots/template-validation/common-min-supported.png) |
| 普通按键模板 | [截图](../docs/screenshots/template-validation/direct-1304x962.png) | [截图](../docs/screenshots/template-validation/direct-min-supported.png) |
| 内置语义模板 | [截图](../docs/screenshots/template-validation/builtin-1304x962.png) | [截图](../docs/screenshots/template-validation/builtin-min-supported.png) |
| 用户语义模板 | [截图](../docs/screenshots/template-validation/user-1304x962.png) | [截图](../docs/screenshots/template-validation/user-min-supported.png) |

[早期 tagged-action 标签截图](../docs/screenshots/template-validation/semantic-tagged-actions.png) 单独证明 Agent 程序列表的 Up/OK 等动作标签可见，不作为后续几何修复截图；[关联拖动截图](../docs/screenshots/template-validation/association-drag.png) 配合 `association-real-mouse` 数据证明完整原生事件链、一次 reorder IPC 及重载顺序。`centeredButtonDelta=2` 是操作槽与按钮联合 bbox 的中心差，不是文字偏移。普通 CDP mouse move 最初未产生 drop 仍属探针缺口；900×700 请求产生的较小 inner viewport 低于产品支持下限，不计为产品布局失败。

## 记录规则

每条用例均记录：测试包版本、日期、遥控器型号、前置状态、操作、实际观察、相关日志标记（脱敏）、结果和失败复现条件。不得记录蓝牙地址、HID 路径、个人目录、语音内容、聊天内容或网页内容。

- **passed**：该步骤已在指定环境实际执行并观察到预期结果。
- **failed**：该步骤已执行，但结果不符合预期。
- **deferred**：缺少目标遥控器、可用第三方应用、签名授权、Windows 环境或其他必要条件，尚不能执行。

## 当前证据边界

2026-09-15 用户新增明确验收：实体键盘与 RC003 同时使用，旧 Home/TV“遥控器优先”常驻吞键取舍已撤销，决策真源为 [PLAN 的宿主旁路专项](../docs/PLAN.md#当前范围调整三键宿主-hid-旁路2026-09-15-用户确认)。当前源码 Raw Input 的 selected_path 过滤能区分设备，但 LL 钩子将已映射 Home/TV 在线状态或四秒武装当作来源，仍可能吞实体键盘并触发映射；尚未修复/验收，不把历史映射通过外推到键盘共存。最终在 TV/Home 映射开启且遥控器连接时，分别验证键盘反引号、~、Home/组合，刚用遥控立即打字、闲置首用，以及遥控 TV/Home 单次且无原生泄漏；补快按、重复、两设备交替/同时及断连/睡眠/退出。日志须区分真实设备来源、吞放、映射与释放，不改真实配置 GUI 来规避问题。RC001 无当前硬件仍 deferred。

同日 14:29 实体序列已执行：首尾确认键各有 RawInput DOWN/UP，用户确认中间返回/音量±已按。宿主捕获候选 failed，未接纳报告；14:42 分类实验的 6396 次 NtQueryObject 成功且名称合法，但均非所选 PDO、也非其子路径。该结果定位到归属门之前，不证明三键实际报告格式或系统消费情况。两次窗口均正常 stop ACK/unload/detach/exit；证据见 `../artifacts/hid-host-three-key-20260915/evidence.md`。生产共享 VK 抑制逻辑未改，三个 opt-in 键盘共存回归仍 failed。

2026-09-19 一次120秒 future-open 元数据观察：按selected devnode枚举2个公开接口，但没有新打开，550次既有IOCTL仍无来源；没有读取payload。stop/unload/detach/exit均正常，停止这条用户态路径，不要求用户重复按键或重连。详见 `../artifacts/hid-host-future-open-20260919/evidence.md`；不能推断三键不存在或永久无法重开。

| 证据 | 当前结果 | 边界 |
| --- | --- | --- |
| 固定内置模板统一目录软件闭环 | passed（源码、自动化、本地安装与有限 UI） | 目录/只读/复制/绑定 10 项、内置引用导入导出与篡改拒绝 2 项、scene 14 项、direct/semantic 原子切换 1 项及 host simulation check passed。前端 catalog、真实语义编辑、保存期保护 28 项和 `pnpm build` passed。内置调节模式只在单次应用运行会话按模板记忆，重启回预设。最终 NSIS 构建及 PowerShell 7 校验 exit 0；管理员覆盖安装、普通用户非提升启动、前三顺序/只读操作及复制—修改—保存—重开—清理均实际 passed。 |
| 前端单元测试（既有 79 项；当前范围定向 11 项）与 `pnpm build` | passed（限定命令） | 既有范围含完整按键模板 39 格编辑/持久化与主动添加流程；当前两文件定向 11 项覆盖只开放完整按键跟随、保留场景配置、初始无关联空态、“软件名称 / 选择的模板”列、运行中程序与 `.exe/.lnk` 两种添加入口、隐藏场景关联及加载失败反馈。定向测试与构建通过，不代表遥控器或真实 WebView/UIA。 |
| Rust workspace lib（core 26、windows 126；app 排除日志 sink 隔离用例后 26 passed、1 ignored、1 filtered）与真实 Win32 窗口发现/路径匹配探针 | passed（限定命令） | 覆盖完整按键模板、大小写变体下的单程序唯一关联、冲突替换不改开关、运行时热切换、前台回调重入、按住中取消/释放门禁及公开 API 身份链。完整 workspace 命令实际为 failed：`updater_notes_land_in_diagnostic_log` 与本轮生命周期日志共用进程级 OnceLock sink；该失败用例单独精确执行 passed，不能据此把完整命令改记 passed。 |
| 正常退出生命周期定向测试（2 项） | passed | 覆盖 supervisor 停止后不再重启监听、Weak 不保活平台、并发重入只执行一次、失败终态保留；host `cargo check` 与 fmt check passed。尚不代表安装实例、真实 BLE、按住实体键或进程消失已验证。 |
| 历史本地 NSIS 包（本地 EXE 清理，记录保留） | passed（构建） | 当时包身份见 [program-associations-presets-final-20260911](investigation/template-validation-evidence.json)，4,759,334 bytes，SHA-256 `27d3a9cecf8d781dd0eb476411a850d8e8bfc65c74c440f8253b60f2aa353950`，Authenticode `NotSigned`；只证明当时构建产物，不证明安装或运行。 |
| scene（7 项）、application/input 定向测试与 host Windows check | passed | 覆盖场景启动失败的代码路径；不代表真实 UIA、前台窗口或硬件。 |
| 组件支持（13 项）与 Helper cargo check | passed | 常规 11 项、官方已下载包校验/回收 1 项、固定官网真实下载/校验/回收 1 项均通过；不代表已安装。 |
| 本机 VB 服务、加载及 render/capture 端点检测 | passed | 仅检测层；不代表安装流程、实体语音或端到端音频。 |
| RC001、RC003、真实 UIA、Codex/微信/浏览器交互 | deferred | 尚无本机真机和第三方应用观察证据。 |
| VB-CABLE 安装/卸载向导与端到端音频 | deferred | Pack45 官方包可由用户自行安装；页面固定展示来源、donationware、下载验签、UAC 与向导说明，用户点击明确主按钮后执行受控下载校验并启动官方交互安装器，但尚未验收实装、升级或端到端音频。 |
| HID 增强驱动 | 真机 blocked external；开发候选验证 passed | 已有RC003精确revision的KMDF三键过滤器、ABI3通道及Helper维护候选，纯C状态机/WDK构建/INF与CAT生成通过。2026-09-19复核核心12项源码和包身份同既有冻结；SYS/CAT仍未Microsoft签名、从未内核加载，用户暂无明确可用签名渠道。实际过滤层、每键报告/抑制/映射及维护仍未验收；RC001无INF绑定/硬件，TV/Home与共享VK键盘共存也未完成。见[驱动说明](../drivers/SayAllInput/README.md)和[精确差距](../artifacts/hid-host-future-open-20260919/kernel-gap.md)，不能承诺可安装或三键可用。 |

## 2026-09-14 固定内置语义模板恢复

- `get_template_catalog` 返回固定顺序：Agent、聊天工具、浏览器三个 `kind=semantic`、`readOnly=true` 的内置项，其后才是用户语义模板和普通按键模板。内置身份使用稳定 ID，用户创建同名模板不改变保护规则。
- `copy_template_catalog_entry` 对内置项执行深拷贝，生成独立用户 ID；副本不继承程序绑定，也不改变前台跟随或通用映射。服务端拒绝对内置 ID 的 update/delete/rename，完整配置或导入文件也不能把内置正文写进用户模板数组。导出仅记录合法内置引用，导入预览和应用继续走现有事务与陈旧 token 门禁。
- 前台切换以一次原子更新同时选择 direct profile 与 semantic recognition：direct 绑定不启用 scene，semantic 绑定只扩展识别而不替换通用执行映射，未绑定回通用配置；切换类型沿用取消当前按住手势、等待全部释放的门禁。完整模板跟随关闭时两类绑定都保留且均不自动执行。
- 内置模板正文不可写，不妨碍运行时调节菜单。每个内置稳定 ID 在当前应用进程内分别记忆音量/翻页/缩放模式；切换 A→B→A 会恢复 A 的会话内模式且不发正文持久化事件。用户语义模板仍发原有持久化事件。应用重启后内置模式回到推荐定义默认值，此限制不得表述为跨重启记忆。
- 自动化通过只证明模型、事务和运行路由。安装版仍须有限验证：前三顺序、只读操作不可用、复制后可编辑保存并重新打开；只清理由本次测试创建且未被用户继续修改的临时副本。Codex、微信、浏览器公开 UIA 与 RC001/RC003 实体矩阵继续 **deferred**，不能用 catalog 可见或复制成功替代。
- 当时冻结本地包的身份记录为 [builtin-template-catalog-final-20260914](investigation/template-validation-evidence.json)，4,795,670 bytes，SHA-256 `c220bbb46fd14deae679e58836e7517d2c1a8fd7a526103fc3e63c7a3d889f0f`；raw EXE SHA-256 `51086b855b995508418451fa9ad9c1c5faddb9dc757811b919a7e2d1ea645765`，98 个产品文件聚合 SHA-256 `65ffdc4fddfa40d515aa4e1942f91e1a6532522eac3a2d41e0951187d54768f4`。`pwsh scripts/verify-windows-bundle.ps1` exit 0；Windows PowerShell 5 因脚本 UTF-8 中文常量误解码 exit 1，不是产品校验失败。用户正常退出旧实例后，管理员安装器 PID 69984 exit 0；installed/raw 仅 Tauri `UNK→NSS` 连续三字节差异，归一化 SHA-256 相同。新实例 PID 73992 为真实 Explorer 用户、Session 1、非提升且响应正常。
- 安装版公开 UIA 实测前三行为 Agent、聊天工具、浏览器；三行各有“查看配置/复制”，同排无编辑、重命名或删除。唯一测试对象“浏览器 副本”把调节模式改为翻页，保存关闭后重开仍选中翻页，随后删除且剩余 0；内置正文落盘 0。一次关闭弹窗脚本误匹配页面同名“关闭自动切换”按钮，脱敏日志证明 `follow_enabled` 从 true 暂时变 false；随后只恢复该开关，最终配置为 true，未改其他模板或绑定。`delivery-evidence.json` 保存安装与清理事实。未在安装版实际创建内置程序绑定，也未执行 Codex/微信/浏览器语义动作或 RC001/RC003 实体矩阵，这些保持 **deferred**。

### 默认三区域语义矩阵（2026-09-14 纠偏）

`MappingTemplate.regionActions` 仍是唯一语义真源，不增加基础映射、覆盖层或迁移。图形编辑按“模板 → 区域”选择，一次显示 `application_list`、`content`、`input` 之一；每区 13 个实体键均保存明确的 single/double/long。所有未列动作均为 `Disabled`，全部 double 为 `Disabled`；long 包含 Menu 固定打开调节列表、Agent/聊天 Input 的 OK 发送一次，以及下表标注的安全翻页动作。

| 模板 / 区域 | 已确认的 single 动作 |
| --- | --- |
| Agent / ApplicationList | Up/Down 选择上一/下一项，long 分别向上/下翻页；Left 对已展开项调用 UIA Collapse，否则选择父项；Right 调用 UIA Expand；OK 激活；Back 取消待选；Home 聚焦列表；TV 聚焦输入；Power Esc |
| 聊天工具 / ApplicationList | Up/Down 选择上一/下一会话，long 分别向上/下翻页；Left 聚焦列表；Right 聚焦内容；OK 激活；Back 取消待选；Home 聚焦列表；TV 聚焦输入；Power Esc |
| Agent、聊天工具 / Content | Up/Down 滚动，long 分别向上/下翻页；Left、Back、Home 聚焦列表；Right、OK、TV 聚焦输入；Power Esc |
| Agent、聊天工具 / Input | 四方向移动光标；OK 以 Shift+Enter 换行、long 经唯一发送控件发送一次；IME 候选状态的 OK 仅以 Enter 确认候选；Back 退格；Home 聚焦列表；TV 聚焦输入；Power Esc |
| 浏览器 / ApplicationList | Home 聚焦地址栏；TV 页内查找；Power Esc；其余非固定键 Disabled |
| 浏览器 / Content | Up/Down 滚动，long 分别向上/下翻页；Left/Right 上一/下一标签页；OK 激活；Back 网页后退；Home 聚焦地址栏；TV 页内查找；Power Esc |
| 浏览器 / Input | 四方向移动光标；OK 原生 Enter；Back 退格；Home 聚焦地址栏；TV 页内查找；Power Esc |

Menu 不读取区域配置：single 固定打开程序列表、double 固定 Disabled、long 固定打开调节模式列表。VolumeUp/VolumeDown 仅 single 按当前模板的 `adjustmentMode` 执行音量、翻页或缩放的正/反向动作，double/long 固定 Disabled；Voice 不进入图形矩阵，继续按下开始、释放结束。Agent/聊天的列表与内容区及浏览器内容区，Up/Down long 分别为 `PageUp`/`PageDown`，复用现有公开 ScrollPattern 能力门控；输入区仍保留方向键按住移动。其余 double 保持 Disabled，避免给常用 single 增加约 0.3 秒判定延迟；没有稳定跨应用用途的 long 也保持 Disabled。因此上述固定格必须显示真实结果且不可编辑，不能保存一个运行时会忽略的值。VolumeMute 没有 RC001/RC003 实体支持证据，图形不应显示；除这些固定例外外，用户副本的每个格保存后由现有区域路由执行。

新增的 `FindPage` 只对浏览器适配器及已确认区域提供公开 `Ctrl+F`，未知焦点、IME 候选、模态窗或其他应用均不执行；`MoveCursorUp/Down/Left/Right` 只在已确认 Input 区通过公开方向键执行。聊天列表不再复用 Agent 的 Left/Right。默认矩阵、浏览器查找门控与输入光标门控定向测试均 passed，Windows 库 check passed；这些是软件证据，Codex/微信/Edge/Chrome 的真实 UIA 可达性、候选框行为和 RC001/RC003 实体按键仍为 **deferred**。

本轮冻结包位于 [semantic-region-matrix-final-20260914](investigation/template-validation-evidence.json)，4,800,722 bytes，SHA-256 `f99044801c403999eb9ea20075ab375b6cc3757bf44b6b3dcef272ca70bfe15b`。管理员覆盖安装 exit 0；已安装 EXE 为 17,962,496 bytes、版本 0.2.5、SHA-256 `d6e76f68f812141745895120e84ad33166ff6c478ca45e1472fa3f20e833e67a`，与冻结 raw EXE 等长且仅有连续三字节 `UNK` → `NSS` 的 Tauri bundle marker 差异。真实 Explorer 用户普通权限启动 PID 33972，Session 1、非提升且响应正常。`settings.json` 安装前后 SHA-256 均为 `0ae84360862d568d17a75b780079259998233f2e316b3e58aed0c3f078c2b6a5`。

已在安装版实际进入“按键映射”，观察到模板、区域、默认调节模式三个图形选择控件、语音键和保存控件。进一步自动化切换模板时，HTML `select` 的 UIA `ValuePattern.SetValue` 没有可靠触发 Vue `change`；随后对第一个返回的 ComboBox 发送 `Home`、`Down`、`Enter` 后出现未保存更改提示。由于控件重绘后无法稳定取得同一对话框的唯一取消按钮，测试按有界失败停止，不能把该提示归因为应用启动即有草稿。前三内置逐项显示、只读、复制、编辑、保存重开及清理因此仍为 **deferred**；没有创建测试副本，也没有向第三方发送动作。

按键配置文件在上述 UI 自动化期间由安装前 SHA-256 `227be4a0517e98fc050a02e24ad21dd521a153e5e887a484495ff5e90bae598a`、15,459 bytes 变为 `432e932ec94c0a9404b90320521c88c7d3b42945a6c4b1cfdb7745da08121a38`、15,457 bytes。诊断日志在 `2026-09-14T05:23:48.693Z`（本地 `13:23:48.693+08:00`）记录 `template_configuration phase=persisted terminal_result=passed`，同毫秒的 `scene_control result=configured` 是保存后的运行态应用，不是独立调节来源。只在内存中逐一替换合法模式值后，唯一能恢复安装前 SHA 的候选是一个用户语义模板 `adjustmentMode: page` → `volume`，候选长度也精确恢复为 15,459 bytes；因此已确认本轮自动化把该字段从 `volume` 保存成 `page`。现有日志没有记录调用方或模板 ID，不能进一步宣称由哪一次确认分支触发。common/scene/follow 开关、模板/绑定数量和内置正文落盘数量均未变。本轮只读归因未恢复配置；为保持用户原始值，后续应将该唯一字段恢复为 `volume`，且执行前须再次核对当前 SHA 仍为 `432e932e…`，避免覆盖后续用户修改。

上述矩阵的后续增量包冻结于 [semantic-template-controls-final-20260914](investigation/template-validation-evidence.json)，4,802,677 bytes，SHA-256 `afd50b082d0e5f03f10b423a2d70548fea4a0fbac18b3a6901b880ba3b2906a2`，`NotSigned`；raw EXE 为 17,962,496 bytes、SHA-256 `3a9805b61e6e9e941c20c8055e9d41ff5eb4271136a78d4dbbae6ae5783a198f`。98 个生产文件构建前后聚合 SHA-256 均为 `24ca991cf8b24e406871fd7ea6dc91802ef5ee34f4a6553a1e37ab8a87c2d18f`，NSIS build 与 bundle verify 均 exit 0。共享模板编辑器定向 6 项、按键页既有 24 项、矩阵精确 1 项、`pnpm build` 和增量格式检查 passed。普通浏览器隔离预览在 1304×962 与 900×700 两个视口观察到标题/遥控器状态同行、选择器另起一行、不适用项禁用且静音卡隐藏；该预览使用非 Tauri 内存 fallback，只证明 common 布局，semantic 图形仍为 deferred。

用户正常退出旧实例后，配置恢复严格以当前 SHA `432e932e…` 为 CAS 门禁；唯一 `adjustmentMode: page` → `volume` 候选写入后重读为原 SHA `227be4a0517e98fc050a02e24ad21dd521a153e5e887a484495ff5e90bae598a`、15,459 bytes。误改版仅在用户临时目录保留本地备份，未进入仓库。随后管理员覆盖安装 exit 0；已安装 EXE 为 17,962,496 bytes、SHA-256 `cea58a2d0456bda1ab796fe2f7d00129ab6ab7c20dd2522e7f96c5b19339731a`，与冻结 raw 仅有偏移 13,389,730 的连续三字节 `UNK` → `NSS` Tauri marker。真实 Explorer 用户普通权限启动 PID 71060，Session 1、非提升、版本 0.2.5 且响应正常；前端挂载、初始 IPC、按键页资源和 RC003 Ready 输入上下文日志均 passed。启动后 `settings.json` 仍为 `0ae84360…`，按键配置仍为恢复后的 `227be4a0…`。未再对真实下拉框执行 UIA；内置 semantic 图形、实体手势及第三方应用行为继续 deferred。

安装后截图中 Agent 程序列表的普通键错误显示为“不执行”，根因不是推荐矩阵为空、区域键名或用户同名模板。Rust 的 `ControlRegion` 与 `RemoteButton` 键按 snake_case 序列化，真实路径为 `semanticTemplate.regionActions.application_list.up.single`；但 `SemanticAction` 是稳定的 tagged object（例如 `{ "type": "select_previous" }`），当时 TypeScript 却把它声明并按字符串查标签，查找失败统一回落为“不执行”。Menu 与 Volume 由固定文案单独显示，因而没有暴露同一问题。当前契约保留 Rust 与磁盘 object 真源，前端改用 `SemanticActionType` 与 `{ type: SemanticActionType }` 读写，不增加字符串兼容或默认矩阵 fallback。`contracts/ipc/template-catalog-builtins.json` 由 Rust canonical `template_catalog()` 生成，不含用户数据；Rust 与 TypeScript 各有逐值契约测试且均 passed，Agent 的 Up/OK/Power 明确断言为非 Disabled。共享组件定向 9 项与 `pnpm build` passed。此前 common-only 双视口截图不能证明 semantic/direct/user 四态；四态隔离浏览器视觉仍待官方 mockIPC harness 实际结果，本段不把它记为 passed。

官方 mockIPC 隔离预览随后完成 common、direct、内置 Agent、用户 semantic 四态在 1304×962 与 900×700 两个请求视口的验证，8 次页面均精确返回目标 source，未知 IPC 为 0，查看切换配置写 IPC 为 0。同一尺寸内标题行、设备状态、选择器行和首卡的 bbox 最大差异均为 0 px；实际 inner viewport 分别为 1278×869 与 874×607。Agent 截图已显示 Up“选择上一项”、OK“打开/激活”、Power“Esc”等真实动作。截图与 bbox 位于 [历史模板验证证据](#历史模板验证证据)，没有覆盖早先 common-only 文件。含 ABI 修复的本地包身份记录为 [semantic-template-tagged-actions-final-20260914](investigation/template-validation-evidence.json)，4,803,758 bytes，SHA-256 `a06ee7468723b8241a5a3bf3b488061e7f422e2fe77705c75eaa030e98bd8fc1`，`NotSigned`；raw EXE 为 17,963,008 bytes、SHA-256 `07b89b9d9d2d73082fe550c54043c75709b1d37cb1809bdab3d3bfbacbd06e40`。98 个生产文件聚合 SHA-256 为 `c487ee8fb0ed03e56129423e517f4df6c7681485547ce307fe0b1110f9f26018`；NSIS build 与 bundle verify 均 exit 0。隔离预览不等于安装版 UI、RC001/RC003 实体按键或第三方应用动作通过。

用户正常退出旧实例后，本次安装前只备份并记录当时的两份配置，不恢复任何历史值：`settings.json` SHA-256 为 `0ae84360862d568d17a75b780079259998233f2e316b3e58aed0c3f078c2b6a5`，按键配置为 `227be4a0517e98fc050a02e24ad21dd521a153e5e887a484495ff5e90bae598a`。冻结包管理员覆盖安装 exit 0；已安装 EXE 为 17,963,008 bytes、SHA-256 `127f1664b1deafe6d888df35044ae1390dc99504e6f79f73cbffeba77d7606d2`，与冻结 raw 仅有偏移 13,389,970 的连续三字节 `UNK` → `NSS` Tauri marker。真实 Explorer 同用户 Medium 上下文启动后唯一 PID 85496，Session 1、非提升、版本 0.2.5 且响应正常；本次诊断段中的 process start、前端入口、Vue mount、runtime snapshot、按键页资源和 input context Ready 均 passed。安装后与启动后两份配置 SHA 均保持安装前值。没有执行真实下拉、按键或第三方动作，安装版 semantic 图形及实体行为仍按上一段边界 deferred。

同日企业微信关联排查只读确认：持久配置中 `wxwork.exe` 的规范化完整路径与运行进程路径精确一致，关联引用内置聊天模板且 catalog 引用有效；`templateControlEnabled` 与普通模板跟随开关均为开启。该绑定保存后的既有日志没有任何 `adapter=Generic` 前台事件；紧随其后的 `adapter=WeChat` 实际属于 `WeChat.exe`/`Weixin.exe` 规则，不能当作企业微信证据。当前源码把 `wxwork.exe` 归入 Generic：仅 Escape、音量快捷键无条件可用；输入、列表、滚动、发送和缩放均须当前前台公开 UIA 区域及对应 pattern 证据，浏览器专用标签、后退与页内查找明确不支持。一次不改变焦点的公开 UIA 顶层两级读取只见 3 个 Pane、0 个可聚焦/已聚焦控件及 1 个 Transform pattern；该结果只证明本次有限范围未见列表、输入或滚动能力，不能外推整个控件树永久不可达。用户尚未说明具体按键，真实企业微信前台匹配、区域、能力、路由和动作均为 **deferred**。

为让下一次单次按键日志直接归因，当前源码在既有诊断体系中增加匿名 `scene_foreground`（adapter、binding kind、region、capability query）和 `scene_route`（button、trigger、semantic action、disposition、内部原因）事件，不记录应用 ID、路径、窗口文字或消息内容。程序关联顺序同时统一为 direct/semantic 两表共享的 `menuOrder`：旧 direct 缺字段时按 0 读取并保持原向量稳定顺序，跨 kind 更换保留原位置，新项追加；唯一 reorder IPC 必须提交当前全部 application ID 且集合精确相等、无重复，配置事务才把全列表写为连续顺序。Rust 模板测试 15 项、semantic unsupported 阻断回归 1 项及 Windows host runtime-simulation check passed；前端、整包和安装版结果须在对应源码冻结后另记，不能沿用上述旧安装包。

统一关联列表的同源前端定向测试 9 项与 `pnpm build` passed。隔离预览使用内存中的 3 条 direct 与 1 条 semantic 关联，跨类型上移只发出 1 次包含全部 4 个 application ID 的 reorder IPC，模拟持久化后重新加载仍保持顺序；在 1278×869 和真实支持下限 1029×732 的 inner viewport 中，模板列与四个操作槽对齐差均为 0 px 且没有横向裁切。900×700 低于主窗口 `minWidth: 1029` / `minHeight: 732`，只保留为范围外证据，不作为产品失败。截图和 bbox 见 [历史模板验证证据](#历史模板验证证据)。包含本轮源码的未签名本地 NSIS 当时通过 `pwsh` 校验，身份记录为 [wxwork-unified-associations-final-20260914](investigation/template-validation-evidence.json)；用户正常退出旧实例后，管理员覆盖安装 exit 0，安装 EXE 与冻结 raw EXE 等长且只含连续三字节 Tauri `UNK→NSS` 标记差异，归一化 SHA-256 完全一致。真实 Explorer 用户 Medium 实例 PID 81192 在 Session 1 启动、响应正常；同一 PID 的 single-instance、Tauri setup、Vue mount 和 runtime snapshot 启动阶段均 passed，有界日志内 panic 与失败终态均为 0。安装前、安装后及启动后 `settings.json` 与 `button-mappings.json` 哈希逐份不变。企业微信真实前台及实体按键路由继续为 **deferred**。

后续按键页布局增量消除了连接线与遥控器图片的两个纵向真源：画布计算使用的图片顶部为 115 px，而旧 CSS 固定为 80 px，导致全部热点与连线统一错开 35 px。当前实现从实际渲染图片内容框统一派生热点圆点与 SVG 连线起点，并由 `ResizeObserver` 在容器变化时重测；404×820 图片按 202×410 等比显示，不移动图片内容。布局固定为左侧 6 键、图片上方独立语音卡、右侧 6 键，静音不计入 RC001/RC003 的 13 个实体键；三个编辑选择器为 36 px。按键页定向测试 25 项 passed。官方 mockIPC 隔离预览覆盖 common/direct/builtin/user，在 inner 1278×869 与主窗口支持下限 1029×732 均为 13 卡、左右同行差 0 px、最大连线起点/热点中心差 0.054567 px、配置写 IPC 0，详见 [图形与拖动证据](#历史模板验证证据)。

同一增量把程序关联的 HTML5 拖动限定在主 Tauri 窗口：`dragDropEnabled=false` 让 WebView2 页面接收拖动事件，overlay 不变；仓库未发现原生文件拖入消费者。第一次普通 CDP mouse move 没有产生 drop，只作为探针缺口保留；针对性复验由真实 pointer 触发原生 `dragstart`，再用 CDP 截获的同一 drag data 完成浏览器原生投放，记录完整 `pointerdown → mousedown → dragstart → dragenter/dragover → drop → dragend`，目标反馈出现、恰好 1 次全量 reorder IPC、跨 direct/semantic 顺序重载保持。前端关联/刷新/busy 定向 20 项和整合 `pnpm build` passed。未签名 NSIS 的身份记录为 [button-map-layout-association-final-20260914](investigation/template-validation-evidence.json)，4,810,061 bytes，SHA-256 `0d71d5d87db4c18aa50f1b499833681bbef3f9dd3874d73f87c11f794a96568c`；raw EXE 为 18,036,736 bytes、SHA-256 `dc7a8be8ca210839055d4fc1804ef10f6d3b7a1db21a4bb1445a76269f786a4c`，98 个生产文件聚合 SHA-256 为 `1d56ea7e15e6a50271a8595c6b952888d02f05d0d0dfcb920975d0f738c97a33`，NSIS build 与 bundle verify 均 exit 0。用户正常退出旧实例后，管理员覆盖安装 exit 0；已安装 EXE 与 raw 等长且仅有偏移 13,442,594–13,442,596 的连续三字节 `UNK→NSS` Tauri marker 差异。真实 Explorer 同用户 Medium 实例 PID 81844 在 Session 1 启动、版本 0.2.5、响应正常且其自身 token 明确为非提升；同一最新日志段中的 single-instance、Tauri setup、Vue mount 与 runtime snapshot passed，panic 与失败终态均为 0。安装前、安装后及启动后两份配置哈希逐份不变。安装版 WebView2 用户实际拖动仍为 **deferred**；企业微信真实前台与动作同样继续 deferred。

## 2026-09-11 历史主动关联与推荐模板源码收口（已被当前范围替代）

本节记录“两套模板同时开放”阶段的源码与冻结包，已被本文开头及 PLAN 的当前范围调整替代。其场景入口、推荐区和两类关联选择只作为历史实现证据保留，不表示当前工作区仍向用户开放；当时的最终本地包也未安装或启动。

- 模板页的“程序关联”主列表只读取并展示已经持久化的 `applicationBindings` 与 `buttonMappingBindings`。运行中窗口发现、预设应用探测、搜索及 `.exe/.lnk` picker 只在用户点击“添加程序”后进入“选择程序→选择模板→确认关联”流程时发生；上一步保留选择，取消不调用保存，重复的同一关联在确认时拦截；场景关联继续提供上移/下移并持久化菜单顺序。
- 同一选择页并列展示“完整按键模板”和“场景模板”的真实能力说明；二者继续使用既有独立模型，不互相转换。更换另一种模板必须到最终确认才调用互斥 upsert，原模板对象不删除；应用 ID 大小写变化也按同一程序处理。
- “完整按键模板自动切换”和“场景控制”分别显示、分别保存。模板创建、程序选择与关联保存均不更改两个开关；关闭时保留关联，未命中或相应开关关闭时使用通用配置。
- 推荐区通过既有 IPC 读取 Agent、聊天工具与浏览器预设，并以 Codex、微信、Edge/Chrome 展示适用程序和主要行为；用户命名并确认后才复制为独立可编辑场景模板。推荐加载失败有明确错误与重试，创建失败保留名称输入；刷新不重新应用预设。
- 完整按键模板、场景模板及按键页“保存为模板”复用同一弹窗壳：小窗限制高度、正文内部滚动、操作区固定，提供明确取消、Esc、初始焦点和保存中禁关闭。模板保存失败时编辑窗口与草稿保留。
- 关联发现、推荐读取、模板创建和关联保存写入脱敏结构化前端诊断；宿主关联 upsert/remove 与推荐创建记录契约、替换分支、终态和耗时，不记录程序 ID、路径或模板名称。

## 2026-09-11 Windows Codex 包身份与“退格未生效”调查

当前安装实例仍是 14:59 启动的旧 `0.2.5` 进程，EXE SHA-256 为 `b6adc1b290d505629a20d2d364de26fa76add90295b10edaca5fc7ef7bbc8451`；它不等于上一节冻结包的 raw EXE，也不包含本节源码修复。持久配置脱敏核对结果为：通用配置已启用但上键未配置；完整按键模板“退”的上键单击为 `Backspace`，Codex 绑定指向该模板，完整按键模板自动跟随已开启，场景绑定为空。编辑页的 `editingSource` 只决定保存哪个对象，不直接选择运行 profile。

经真实用户 Session 1 的精确 PID/HWND 公开 UIA 读取，旧实例当时为“已连接”，按键页为“按键监听已就绪”；上一轮主动断开和停止监听已经不是本次观察时的门禁。公开进程与包身份读取显示：当前 Windows Codex 的可见主窗口由 `ChatGPT.exe` 承载，VersionInfo 的 ProductName/FileDescription 均为 `Codex`，PackageFamilyName 为 `OpenAI.Codex_2p2nqsd0c76g0`；三个无可见顶层窗口的 `codex.exe` 属于后台 CLI，不进入运行中程序发现。旧源码只按可执行文件名把 `Codex.exe` 归一化为 `codex`，因此可见 Codex 窗口被当成普通完整路径身份，与持久 `codex` 绑定不相等，运行时回退到上键未配置的通用配置。这是本次退格一直未生效的具体失败环节。

当前源码在既有 `application_identity_for_path` 中仅识别公开 `OpenAI.Codex_*__2p2nqsd0c76g0` 包目录内的 `ChatGPT.exe`；保留独立 `Codex.exe`，普通 `OpenAI.ChatGPT-Desktop` 包及任意其他 `ChatGPT.exe` 继续使用规范化完整路径。运行中程序发现、`.exe/.lnk` picker（快捷方式先解析目标 EXE）和前台身份均复用该函数，避免保存身份与运行身份分叉。`cargo test -p sayall-windows identity_uses -- --nocapture` 为 2 passed；合并当前范围后 `cargo test -p sayall-windows scene_control::tests --lib -- --nocapture` 为 11 passed，其中普通模板命中/回退在 UIA 不可用时仍通过；`cargo fmt --all -- --check` passed。一次错误使用 `--exact` 的命令实际执行 0 项，已纠正且不计入通过。

当前“仅程序关联完整按键模板”的产品源码冻结证据归 [button-mapping-only-source-20260911](investigation/template-validation-evidence.json)：`source-hashes.json` 只覆盖本轮 8 个产品/定向测试文件，`verification.json` 记录前端定向 11 项、scene 定向 11 项、身份定向 2 项、当前前端构建复用及格式检查。该证据与后续文档修改分开，不把共享脏工作区或未重跑的完整 workspace 检查描述为通过。

当前安装实例的默认诊断日志在该 PID 启动前已停止更新，无法用它证明 `map_profile_switch`；调查时系统前台为未绑定程序，也不能把身份规则修复直接记作 Codex profile 已实机命中。含本节修复的新包安装启动、切到真实 Codex 后的 `profile_active=true`、切到未绑定程序后的回退，以及遥控器上键实际产生退格，均保持 **deferred**。

## 2026-09-11 新安装版“不同模板均未执行”调查与源码修复

当前真实实例已变为 PID 54728（本地时间 18:06:08 启动），安装 EXE 为 17,814,528 bytes、SHA-256 `2fa901d295a490178de4d3f5feaeda564a053f28a9e7a259455a88df6ca39286`，不是上一节记录的旧 PID 17840。持久配置只读核对显示完整按键自动跟随已开启，Codex 绑定指向一个已启用且配置了普通按键动作的模板；配置本身不是空模板。

该进程的生产日志已经把失败点分层锁定：真实前台先被识别为 `Codex`，随后记录 `map_profile_switch ... profile_active=true`，证明程序身份和绑定已命中；实体方向键持续产生 `map_edges`，证明 Raw Input 收到了物理边沿；但每次 profile 切换后的最终引擎配置均为 `map_reconfig enabled=false buttons_configured=0`。本进程没有任何 `map_input_context` 记录。根因是映射执行层按连接状态 fail-closed，而原宿主只在 `connect/restore` 命令返回时同步一次连接快照；该返回仍处于 `AwaitingCapabilities/Reconnecting`，BLE 工作线程随后在后台进入 `Ready/Streaming` 时没有再次通知映射层，因此运行时永久保留 `connected=false`，通用配置和任意程序模板都会被统一清空执行能力。

当前源码改为由唯一 BLE 工作线程在自身事件循环边界发布连接上下文：不依赖页面查询或主窗口可见性，覆盖等待能力后就绪、流式/排空、断连、自动重连、睡眠和型号变化；相同 model/connected 不重复重配。回调在连接快照锁外调用，退出 `Shutdown` 先终止发布并同步离线，之后的迟到快照不能重新启用映射。宿主层旧的一次性同步已删除，避免两套真源及退出期间迟到 IPC 回调。每次 phase/model 变化写 `input_context_sync`，实际 model/connected 改变仍由既有 `map_input_context` 和 `map_reconfig` 串起结果。

`cargo test -p sayall-windows connection_context_ --lib -- --nocapture` 为 2 passed，覆盖 RC001/RC003 从等待到 Ready、模板启用、重复同步幂等、断连禁用、重连恢复和 Shutdown 后不复活；既有 `cancelled_input_holds_do_not_transfer_or_fabricate_clicks` 1 passed，覆盖断连释放不制造点击。`cargo check -p sayall-windows-app` 与 `cargo fmt --all -- --check` passed。以上证明状态同步与纯执行配置，不代替新包安装后的生产日志、物理按键实际注入、RC001/RC003 或冷/闲置首按，后者保持 **deferred**。

本修复当时的包身份记录为 [connection-input-context-final-20260911](investigation/template-validation-evidence.json)，4,758,043 bytes，SHA-256 `3a13acac186f9aab41dd570d3762682b1f3d69ceeba23b3aa8ecf43615bc6821`；NSIS 构建 exit 0。`source-manifest.json` 记录 98 个生产文件，构建前后聚合 SHA-256 均为 `870d566f89782c0d29cb543635fdca901cf2fa8142dfab794b8d934763768690`。当时 PID 54728 仍正常响应；两次有界桌面截图均返回无效句柄，精确调用“显示隐藏的图标”后也未出现可核验的托盘溢出窗口，因此未点击退出、断开或停止监听，未运行安装器、请求 UAC、启动新实例或写入配置。安装状态为 **pending_user_exit / blocked**；当时基线为 `button-mappings.json` 9,279 bytes、SHA-256 `5f6c3839f3fe8916207f5da64fa5dac3aed4a0cddb569b5e1e98e7574a204461`，`settings.json` SHA-256 `0ae84360862d568d17a75b780079259998233f2e316b3e58aed0c3f078c2b6a5`。该旧包不再作为待安装入口；当时的 `input_context_sync ... connected=true`、`map_reconfig enabled=true`、普通权限启动和实体按键仍记录为 **deferred**，后续独立包的实际结果见下一节。

2026-09-14 用户在 Q-Dir 再次报告已关联模板的左键“右键菜单”无效果。当前真实用户 Session 1 仍运行上述旧 PID 54728、EXE SHA-256 `2fa901d295a490178de4d3f5feaeda564a053f28a9e7a259455a88df6ca39286`。脱敏配置核对为：完整按键模板自动跟随开启；Q-Dir 的规范化完整路径身份精确命中一条绑定；绑定模板存在且启用；左键单击动作是 `Shortcut(Apps)`。生产日志多次记录 Q-Dir 前台 `map_profile_switch ... profile_active=true`，实体左键也有成对 `Left=true/false`，但每次随后均为 `map_reconfig enabled=false buttons_configured=0`，且没有 `map_fire` 或 `map_inject`。因此本次不是前台身份、绑定、模板内容或实体边沿缺失，而是旧安装版的连接门控仍把执行映射清空；不需要用户在旧版重复按键取证。

当前工作区的 `ble.rs`、`button_mapping.rs`、Windows core `lib.rs` 与 host `lib.rs` 分别与上一段连接上下文修复清单逐字节 SHA 匹配，复用其连接上下文 2 项、断连按住释放 1 项、host check 和 fmt passed 证据。结合当前前端三处主操作样式调整构建的本地包身份记录为 [qdir-kk-profile-final-20260914](investigation/template-validation-evidence.json)，4,761,185 bytes，SHA-256 `b5beaacab46478d487fa10a5e86fd5611499667e0231b741ea988aacb927fce3`；raw EXE 为 17,825,280 bytes、SHA-256 `a84645f404fd3d97ceb04501cd74117d7b94d95444155cd48be3ea6575821464`。`source-manifest.json` 记录 98 个文件，构建前后聚合 SHA-256 均为 `19fd72a6bfa43b799e4c220e674c7c9527dacdde8284d8957e2d3ff47c291cff`，NSIS build exit 0、`NotSigned`、未发布。

用户正常退出旧实例后，2026-09-14 管理员更新安装 exit 0；已安装 EXE 为 17,825,280 bytes、SHA-256 `878f32c9a5b21756171ee873a38179b936eda9c1690765e991cc8d55222147b2`，含唯一 Tauri `NSS` bundle marker，将其归一化为 `UNK` 后 SHA 与冻结 raw EXE 完全一致。普通用户启动的 PID 72888 属于 Explorer 用户、Session 1、非提升、版本 0.2.5 且窗口响应正常。启动日志实际观察到 `input_context_sync phase=ready ... connected=true` 后 `map_reconfig enabled=true buttons_configured=1`；Q-Dir 前台随后为 `profile_active=true` 且 `buttons_configured=3`，实体 Left DOWN/UP 成对、`map_fire` 与 `map_inject result=ok` 均存在，用户实际确认右键菜单已弹出，因此原连接门控故障的端到端修复为 **passed**。离开 Q-Dir 后又观察到 `profile_active=false` 和通用配置 `buttons_configured=1`，前台回退为 **passed**。

仍保留两条可观测性事实：持久配置当前仍是自动跟随开启、Q-Dir 唯一绑定命中启用模板且 Left 为 `Shortcut(Apps)`；但诊断日志把本次 Left 的 chord 写成 `Backspace`，与配置枚举及用户看到的右键菜单不一致，后续日志修正前不能用该字段单独证明实际键值。独立 `button-mappings.json` 在应用运行期从安装前 SHA-256 `7d7157746c4ba01333dee513c0836f095ddaa3955e827a84a17050251da6323c` 等价重写为 `a70bbc8e8a8a9c64b2c7ad79ec39348f5e0ace3ccea06e6a284b8dd86b4e465a`；目标绑定、模板、Left `apps` 与跟随开关经脱敏复核保持，不能把字节级哈希表述为未变。普通设置文件 SHA-256 `0ae84360862d568d17a75b780079259998233f2e316b3e58aed0c3f078c2b6a5` 与既有交付证据一致。

## 2026-09-11 历史本地包与安装边界

历史交付记录标识为 [button-mapping-only-final-20260911](investigation/template-validation-evidence.json)。`build-evidence.json` 记录 `pnpm tauri build --bundles nsis --config target/sayall-local-bundle.json --ci` exit 0；安装包 `无线麦 SayAll_0.2.5_x64-setup.exe` 为 4,756,331 bytes，SHA-256 `fc8f4087add50e37fbfa45bf0a67af61e1ab854fda7c5b63b744a8846edffafc`，`NotSigned`。`source-manifest.json` 记录 98 个构建范围文件的聚合 SHA-256 `987465f2ba073cdf5793d32a2dd4d47785b16065cda3dbd56c3d3aa91e5c5d52`，并在构建后复核聚焦 8 个实现/测试文件为 8/8 匹配、0 mismatch；该记录不声称共享脏工作区的其他文件未变。

安装准备先确认旧实例处于 Idle，再经公开 UI 执行 Disconnect 和 Stop RawInput Listener：连接终态显示已释放且停止自动重连，监听按钮恢复为“启动监听”，两步均 **passed**。随后以真实通知区域截图锁定 SayAll 托盘菜单中的“显示主界面 / 退出”，仅对精确“退出”项 Invoke 一次；超过 60 秒后同一 PID、启动时间和可执行文件身份仍存活且响应，因此旧实例正常退出为 **failed**，管理员覆盖安装为 **blocked**。未发送 `WM_QUIT`、未强杀、未运行安装器，也未启动新构建；旧实例当前保持断开且监听停止，不能沿用先前 connected / Ready 描述。

交付过程未写持久配置。button mappings 前后 SHA-256 均为 `adbfb1d641d257ddac48b44ad04c29a6e3fea186423dcf6ac0512884fa31ed95`，设置文件前后 SHA-256 均为 `0ae84360862d568d17a75b780079259998233f2e316b3e58aed0c3f078c2b6a5`。两份文件均 unchanged，用户 17:07 基线、完整模板跟随开关和既有映射得到保留；运行态断开与停监听不属于持久配置写入。

含当前源码的新安装版页面、场景入口隐藏与运行停用、Codex 前台命中既有“退”模板、未绑定前台回退、通用/模板保存隔离、遥控器上键产生 Backspace，以及 RC001/RC003 均为 **deferred**。隔离 headless 视觉捕获因 Chrome sandbox/GPU 启动失败两次，没有生成 PNG，不能记为视觉 passed；当时生成的静态布局预览仅用于布局核对，现已清理，不代表真实应用。

模板前端源码自动化结果：`pnpm test` 14 个文件 79 项 passed；`pnpm build` passed。退出清理改动后的 Rust 精确结果见下一段；上述前端结果证明软件事务与页面编译闭环，不证明新弹窗在安装版 WebView 的尺寸/焦点、前台命中/回退或任何 RC001/RC003 和第三方应用实体行为。

退出清理增量在同一源码上增加了 2 项生命周期测试：Raw Input supervisor 使用 `Weak` 且有 stop 信号，停止完成后不会再启动监听；正常退出在后台按 supervisor、输入失活屏障、Raw Input、BLE 顺序 best-effort 清理，重复/并发退出复用同一结果，只有全部阶段执行后才记录完成，逐阶段失败仍保留。`cargo test -p sayall-windows-app --lib lifecycle_tests -- --nocapture` 2/2 passed；`cargo test --workspace --lib -- --test-threads=1 --skip updater::tests::updater_notes_land_in_diagnostic_log` exit 0；完整 workspace 与串行完整 workspace 均因进程级诊断 sink 隔离令该日志落盘测试 failed，其精确单测 exit 0。新包退出、真实连接与按住状态仍需现场验证。

历史交付记录标识为 [program-associations-presets-final-20260911](investigation/template-validation-evidence.json)：`source-manifest.json` 覆盖 98 个生产文件，fingerprint `175f5c8e9120cdd0b6168269fa0a408f734cafdfcf9498ce23abe421f15e54fd`；`build-evidence.json` 记录 `pnpm tauri build --bundles nsis --config target/sayall-local-bundle.json --ci` exit 0。安装包如上表；raw executable 为 17,826,304 bytes，SHA-256 `e2324b9b38a62e7d496c4818a8089f8d2c62efb8578efc56e963fde84f4da745`，同为 `NotSigned`。当时的 `delivery-status.json` 记录了安装边界：旧安装实例 PID 17840 仍运行且响应，未使用 `WM_QUIT` 或强杀，用户尚未完成手动托盘退出，因此新包安装为 **blocked**，普通用户启动、唯一实例、新关联/推荐/弹窗 GUI 与双硬件验收均为 **deferred**。持久配置未被交付验证写入，button mappings 仍匹配记录的测试前基线 SHA-256 `7a3cb3b39df8dd386db24d23545614c036488f2fa6d81ecb1a73c2e1fe4bd0b1`。

此前 21 项 `records.template-opt-in-ui-source-20260911` 的源码哈希 只证明构建前实现与文档快照；本段是构建后的文档追加，不据此重述当前工作区为 21/21 匹配。最终包的生产源码身份以 98 文件 `source-manifest.json` 为准。

## 2026-09-11 旧包安装版交互桌面记录

当时使用的冻结本地包身份见 [button-templates-opt-in-final-20260911](investigation/template-validation-evidence.json)（SHA-256 `9a62df0b510cda43432ce13833886e59cad71bb5e16c5faa38d9f15766f38e27`）。包的 `build-evidence.json` 记录前端 66 项、scene 11 项、settings 模板 9 项和按住切换 1 项通过；`source-hashes.json` 是该包对应工作区快照的真源，不应只用当时的 Git HEAD 推断包内容。安装 EXE 的 Tauri `NSS` 标记在内存中归一化为构建侧 `UNK` 后，SHA-256 与冻结 raw EXE `968e6fff25a33bcdb612fcd0c9a3c370971c52462a81117adfa4f91dafacdc43` 一致。

| 检查 | 实际观察 | 结果与边界 |
| --- | --- | --- |
| 安装实例健康 | 真实交互用户 Session 1 的单一安装实例持续 `Responding=true`；一次 `WM_NULL` 1 秒有界检查成功。 | passed；未退出、重启或强杀应用。 |
| 按键与模板切页 | 公开 UIA 实际执行 12 次“按键”与“模板”往返，目标标题均出现，单次 157–365ms，末次窗口仍响应。 | passed；证明该桌面导航路径没有复现窗口卡死，不等同于触发 scene overlay 隐藏或所有跨线程回调。 |
| 显式编辑目标 | “编辑配置”公开 ComboBox 实际显示 3 项（通用配置与两个既有模板）；依次选择两个匿名模板后返回通用配置，持久文件未因选择而写入。 | passed；只验证选择与草稿挂点。模板单目标保存、串写隔离、保存中禁切换及保存/放弃/取消保护未在安装版实际执行。 |
| 自动切换默认关闭与保留关联 | 测试前 `buttonMappingFollowEnabled=false`、既有绑定 1 条。实际开启后绑定、通用映射和两个模板对象均保持不变；收口时仅关闭该开关，实时配置 SHA-256 精确恢复为测试前备份 `7a3cb3b39df8dd386db24d23545614c036488f2fa6d81ecb1a73c2e1fe4bd0b1`。 | passed；证明默认关闭状态下的保留数据与开关事务。已绑定前台命中模板、未绑定前台回退通用配置仍未实机观察。 |
| 前台按键 profile 诊断 | 运行时代码在 `button_mapping.rs` 记录 `map_profile_switch ... profile_active=true/false`，但本次安装实例的诊断文件在测试时没有该 PID 的新行。 | deferred；不得用 scene snapshot、编辑草稿或单元测试代替真实前台命中/回退证据。下一次先确认当前实例诊断 sink 可写，再做绑定与未绑定前台切换。 |
| 正常退出清理 | 两次有界托盘 UIA 与一次任务栏视觉检查均未精确定位旧包退出入口；未发送 `WM_QUIT`，旧实例仍存活。公开 UI 曾执行断开与停止监听，但该 PID 的日志 sink 没有新行，后续只读 UIA 也未再次命中对应终态。 | deferred；旧包不能作为清理证明。须用含退出修复的新包观察 `app_shutdown` 各阶段、进程正常消失及 BLE/按键释放。 |

三处问题的当前证据边界如下：

1. `src-tauri/src/lib.rs` 的 scene 事件订阅通过 `run_on_main_thread` 调用 overlay 更新和前端事件发送；冻结包包含该实现。12 次普通切页未复现卡死，但未实际打开/关闭 overlay，故该特定触发仍为 **deferred**。
2. `crates/sayall-windows/src/scene_control.rs::emit_snapshot_for` 在调用订阅者前先构造拥有所有权的 snapshot，避免订阅者重入 `active_button_mapping` 时仍持有 scene `MutexGuard`；scene 11 项包含对应重入回归。安装版未用可观测日志强制触发该回调链，真实触发仍为 **deferred**。
3. `src/pages/ButtonsPage.vue` 用独立 `editingSource` 区分 `common` 与 `template:<id>`，`persist` 按捕获的 source 分别调用通用或模板保存，并在请求仍属当前目标时更新草稿。安装版已完成三目标选择且没有落盘；模板单目标保存命令在用户要求安全收口后未执行，因此不能宣称安装版保存隔离已通过。

复核本段历史结果时，读取 [program-associations-presets-final-20260911](investigation/template-validation-evidence.json) 中的身份数据及本节安装边界；该候选和 [button-templates-opt-in-final-20260911](investigation/template-validation-evidence.json) 均不再提供本地安装包。继续验收须使用另行核对身份的现有候选，再按需读取本节及当前源码，不能将历史结果直接套用于新包。当时的测试前配置备份位于 `%TEMP%/sayall-button-mappings-before-validation-20260911-150546.json`；当时已确认实时配置与该备份字节哈希一致，不保证临时备份仍存在，不得在应用运行时用整文件覆盖来“恢复”。

## 安装与基线

1. 记录测试包版本，按产品正常退出路径完成安装或升级并启动；确认没有替换运行中的连接会话。
2. 在设置页确认通用映射仍可查看、编辑和保存；模板场景未绑定或场景总开关关闭时，按键继续使用通用映射。
3. 分别连接 RC001 和 RC003，记录型号识别和连接状态。RC003 增强能力未真机验收前，界面不得暗示其已执行。
4. 对每个型号执行一次冷启动后的首按，并在闲置后再执行一次首按；两次均须记录实际动作和日志标记。
5. 分别在空闲、已连接、Raw Input 监听中和普通键按住中，从托盘选择“退出”；确认进程正常消失且日志依次出现 supervisor、input quiesce、Raw Input、BLE 的 started 与 passed/failed 终态。重复触发退出不得重复执行清理或循环阻止退出；任一步失败必须保留 failed，不能只凭 `process_exit` 记 passed。

## 模板、绑定与配置事务

以下每项分别在 RC001 和 RC003 的已连接状态执行；没有实机时保持 **deferred**。

完整按键模板与既有三区域语义场景分别验收：前者保存每个普通按键的单击、双击、长按动作，语音键不出现在编辑矩阵；后者继续按 `application_list`、`content`、`input` 区域工作。一个应用只能关联其中一种，冲突对话框取消后必须保持原绑定和运行态。

| 用例 | 操作与预期观察 |
| --- | --- |
| 创建、改名、复制、删除 | 创建非空唯一名称；重名应保留原配置并显示错误。复制后获得独立模板，编辑副本不影响原模板。删除已绑定模板时，选择替换模板或明确解除绑定；取消不写入。 |
| 推荐预设 | 用户选择 Codex、微信或 Edge/Chrome 推荐项并指定名称后，创建独立新模板；检查适用程序和主要行为可见，创建后配置可编辑；不得覆盖任一模板、通用映射、程序关联或两个运行开关。名称冲突须保留输入供用户处理。 |
| 应用绑定与前台跟随 | 一个应用至多绑定一个模板，多个应用可共享模板。切到已绑定的前台应用后验证模板、区域和调节模式；未绑定应用回退通用映射。 |
| 完整按键模板切换 | 刷新运行中可见应用，并分别用预设、`.exe`、`.lnk` 补充；关联后切到该程序，验证普通按键使用模板，切到未绑定程序后恢复通用映射。标题变化不得改变绑定；无法查询路径的窗口应跳过。按住普通键时切换前台，应立即取消旧手势，该次释放只解除门禁，不落到新模板；下一次完整按压才执行新模板。 |
| 场景与通用开关 | 关闭通用映射而保持场景开启，验证已绑定前台应用仍按模板工作；关闭场景后验证回到通用映射。 |
| 保存失败 | 制造可恢复保存失败后，验证磁盘配置与运行态均保持上一次确认值，界面保留草稿并显示错误；成功保存后一次性切换运行态。 |
| v1 导入 | 导入 v1 文件，仅更新通用映射；本机模板、绑定、语音和音频设置保留。通过预览后再确认，取消不写入。 |
| v2 导入导出 | 全量导出不得包含绝对路径、设备身份、驱动授权或运行态。导入先预览格式、冲突和未解析应用；确认令牌过期、重复使用或取消均不得写入。 |
| 批量模板传输 | 选中模板导出只含选中模板且不含本机启动目标。追加导入须勾选源模板、指定名称、默认不替换绑定；预览列出最终名称、新模板、新增/替换/跳过/未解析绑定，确认结果必须与预览一致。 |
| 陈旧预览 | 在预览后另行保存配置、改选模板、改名称或改替换选项，再确认旧预览；应被拒绝并要求重新预览，旧配置不变。 |

## 实体按键矩阵、区域与生命周期

对每个模板的 `application_list`、`content`、`input` 三个区域，逐项记录单击、双击、长按结果：方向上/下/左/右、确认、返回、主页、直播、音量加、音量减、电源、菜单；同时记录语音键按下与释放。不得只验证可见的一部分按键。

1. 在应用列表验证上下选择、确认切换目标、返回取消；在内容区验证导航或滚动；在输入区验证光标移动、确认短按换行、确认长按发送、返回删除。浏览器输入焦点使用公开快捷键进入地址输入，不伪称页面编辑已通过。
2. 菜单短按打开程序列表，菜单长按打开音量、翻页、缩放调节列表。长按释放后不得再触发短按；面板关闭后必须等待所有实体键释放，避免遗留按住边沿。
3. 音量加减仅按当前调节模式工作：音量、页面或缩放。切换模式后记录内容区与输入区的差异。
4. 语音键必须立即按下开始、释放结束，不等待双击或长按；在菜单打开时按语音应先取消面板并立即开始语音。断连、睡眠、恢复、应用退出和“按住中退出”均检查开始/结束和组合键注入严格成对清理。
5. 分别在冷首按、闲置后首按、连续按、断连、睡眠/唤醒和按住键退出应用后复测返回、音量加减和语音。任何一次失败均记录模型、前置状态和脱敏日志，不能由热态连续成功替代。

## 第三方应用与遥控场景

下列均需在测试人员已登录、可见且获授权的本地应用中执行；不读取私有配置、数据库或内存。

| 场景 | 可执行验收 |
| --- | --- |
| Codex | 打开真实程序列表，遥控选择 Codex 并确认；前台切换后验证对应模板与区域，随后返回取消不启动额外程序。 |
| 微信聊天 | 在可见聊天窗口中验证候选列表导航、内容区滚动、输入区光标、确认短按换行和长按发送；仅记录动作是否生效，不记录消息或候选内容。 |
| 浏览器 | 在公开可见网页验证页面导航、地址输入焦点、返回网页后退和缩放；记录浏览器类型与结果，不记录网址或页面内容。 |
| 场景浮层 | 打开程序或调节菜单，检查浮层展示当前选择和运行状态；浮层不得抢占前台焦点，不得响应鼠标执行，返回可取消。普通入口与 overlay 独立入口均应可启动。 |

## 驱动与虚拟音频线边界

1. 首先读取状态卡：HID 增强与虚拟音频线分别显示安装、包、服务、加载、绑定、音频端点和重启需求。音频端点就绪不能表述为实体语音链路已验收。
2. VB-CABLE 使用固定官网 Pack45：页面常驻说明来源、donationware、下载验签、UAC 与官方向导边界；用户点击明确的安装/卸载主按钮后才下载并校验 hash、签名、版本和架构，通过后才触发系统 UAC 并启动官方交互向导。不捆绑或镜像二进制，不使用不可靠的静默 Repair 参数。关闭或超时后必须重新检测，不能视为成功。
3. HID 增强已有专用 KMDF/raw PDO 与普通用户接入开发候选，组件入口明确缺少 Microsoft 签名并拒绝安装、修复、卸载。纯 C、Windows lib 与 Helper、WDK/INF/CAT 静态检查通过；正式包、Helper 系统实装维护验收与双型号报告仍缺失。源码及准确分层边界见 [驱动候选说明](../drivers/SayAllInput/README.md)，不得用目录生成或应用安装替代内核验收。
4. 卸载 SayAll 时验证精确范围：不得删除共享虚拟音频线、其他产品组件或用户设置；独立移除虚拟音频线时须二次明确确认并记录受影响组件。

## 结果记录模板

```text
用例：
测试包版本：
日期：
遥控器型号：RC001 / RC003 / 无
前置状态：
操作：
实际观察：
脱敏日志标记：
结果：passed / failed / deferred
失败复现条件或 deferred 原因：
```

只有上述真机与真实应用步骤逐条完成并留下脱敏观察记录后，才可更新相应用例为 **passed**。
