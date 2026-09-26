# TODO

## v1 决策（2026-09-04）

- 第一版只做微信输入法听写：按住说话快捷键默认 左Ctrl+左Win（适配微信输入法默认语音热键）；语音可用 = 按住语音键 → 注入快捷键 → ATVV 音频经 CABLE Input → 微信输入法麦克风（CABLE Output）→ 云端识别 → 文字上屏。端到端链路音频段已本机实证（调查报告 evidence/n）；注入段配方约束已本机实证（evidence/p：WeType 拒绝单批零间隔和弦，须逐事件注入；间隔 20/40/60ms 均 4/4 触发、零间隔 0/2，~~默认取 20ms 压低按键延迟~~（2026-09-05 更正：20ms 为热态验证结论，冷/节流态必失败已实证并回退 80ms，见下方"性能已知项"与提交 86e5314——延迟优化必须以成功率保证为前提），Bugs\2026-09-04-wetype-zero-gap-injection.md）；**延迟账目已量化（2026-09-05，evidence/p 端点预热对照实验）**：注入→WeType 开麦固定 ~163ms（冷/热端点中位数差 0.3ms，预热无效；WeType 内部处理，第三方边界不可干预），两型号实际均直接 0x04 推流（无开麦往返可并行），0x04 早于 HID F5 60-90ms（触发点已最早），全链 ≈215-245ms 其中应用侧仅和弦 20ms 可控——**应用侧延迟优化到此收敛**；RC001 遥控器端到端真机 passed（2026-09-04，用户确认文字上屏；用户侧前提=输出端点选 CABLE Input + 系统默认录音设备切 CABLE Output，应用不修改系统默认设备）；RC003 真机待验。豆包（注入判死四层闭环）与 WinUHid 增强轨延后，见 ADR 0002 与 docs/investigations/2026-09-04-avoid-driver-signing-input-paths-final.md。

## Windows RC001 / RC003

- [x] 建立独立 Rust + Tauri 2 + Vue 3 工程结构。
- [x] 建立 Mac 原版风格设置界面骨架。
- [x] 建立 ATVV、ADPCM 和语音会话纯 Rust 核心。
- [x] 建立 Windows CI 和真机测试手册。
- [x] 实现 WinRT 已配对设备扫描、GATT 连接/释放、ATVV 通知和 PCM 解码代码路径；Windows 与 RC001/RC003 运行验收仍待完成。
- [x] 补充 RC001/RC003 设备名称与标准 GATT Model Number（2A24）识别，在连接快照和界面中传递型号；无法识别时保持 `unknown` 且不阻断 ATVV。Windows 双型号真机验收仍待完成。
- [x] 将 RC001 短语音场景作为 JSON 夹具回放，覆盖 40 + 80 字节拆包、20 次极速空会话、20 次完整会话和中断后首个新会话恢复；该回放不代表真实 Windows/RC001 固件验收。
- [x] 将真实连接阶段、能力与解码采样计数接入 Tauri IPC 和连接页面。
- [x] 使用同一 JSON 契约夹具验证 Rust 序列化与 TypeScript 接口的 `PlatformSnapshot`、`PairedRemote`、camelCase 字段及 RC001/RC003/unknown 枚举值；Windows WebView 运行时 IPC 仍待验收。
- [x] 实现显式 WASAPI 输出端点枚举、选择、16 kHz PCM 写入、有界队列和真实 padding 排空代码路径。
- [x] 会话临时Capture输入锁定与长设备名布局：2026-09-19用户明确确认RC003非target冷首按正常、下拉框不越界。默认关闭，仅本功能获准受限IPolicyConfig，结束条件恢复、检测外部改选整体让出；详见 Testing/CaptureInputSession.md。
- [ ] 扩展验收待完成；2026-09-19用户已确认首按及布局通过。原范围：提供“遥控器说话时临时锁定麦克风输入”开关及目标capture设备选择，仅Voice DOWN会话期间生效；区分普通/communications默认角色和目标软件私选，记录原值，UP/取消/断连/睡眠/正常退出恢复，原设备不可用或用户主动改选时不得覆盖；评估异常退出恢复与按住期间设备变化。当前VB路径写CABLE Input、目标采集CABLE Output，但不能据此猜用户选择。2026-09-16用户明确“允许受限使用，并接受外部改选时让出控制”，仅此功能使用IPolicyConfig设置当前用户Capture默认三角色；任何外部改选整次让出并保留全部角色，正常结束仅恢复本次实际改过且仍满足条件的角色；无CAS残余竞态不得宣称消除。崩溃journal须用户可见恢复/保留，不静默恢复。当前源码、隔离测试及定向审查完成；2026-09-16 17:15最终9a7048本地包安装、Explorer普通用户启动、载荷/配置核验passed，新功能默认关闭。RC003真实首按验收failed：normal角色联动被当作外部改选中止，第二次才成功；下拉框另有宽度溢出。当前组事务/在途journal与CSS修复候选19项Windows+5项页面tests passed，18:07修复包970e6c已覆盖安装并核普通权限/原开启配置保留，待非target首按与恢复实测，见 artifacts/capture-input-session-20260916/first-use-fix/evidence.md。不得为快速释放新增阈值、迟到切换或闲置抢占，不由音量键触发；RC003真实hold/release/quick/repeat/idle-first/headset/manual-select/disconnect/disabled验收，RC001本轮硬件deferred。
- [ ] 修复 WASAPI 失败后永久丢失输出 sink：保留原端点选择，下一新语音会话仅重建原精确 ID/name；失败取消不复活旧会话，控制释放/断连/睡眠在排队前取消启动准备。2026-09-15 开发候选：定向逻辑与真实 WASAPI 静音队列故障→首新会话消费/排空 passed；16:12覆盖安装与Explorer普通权限启动/载荷和配置保留 passed；随后 RC003 短会话实际提交/排空、用户反馈麦克风正常；长会话19秒内部PCM队列再次溢出 failed。原端点恢复与持续吞吐问题分开，闲置/睡眠仍未验收；EventsShared/MMCSS隔离候选未解决，不打包。证据见 `artifacts/audio-sink-recovery-20260915/evidence.md`。 2026-09-16固定旧/新包实机均在仍按住时约60s收到远端`00 02`，续期正常、无overflow并正常排空；旧包另有WeType实际CABLE Output路由证据。该停止链路与9/15内部溢出不同，不能宣称硬件上限或修复完成，详见同证据目录 `sixty-second-20260916/evidence.md`。 已正常回切原fbc修复包并核普通权限/配置，observer正常退出；终态诊断3项测试与源码review通过，已随9/16输入锁定候选安装；不代表音频故障修复。输入锁定受限API已获用户明确授权，20s内部溢出未关闭，60s远端停止按用户要求暂停研究。
- [x] 实现用户显式配置的语音键按住说话快捷键（连接页预设：关闭、右 Alt、F5、Win+H、左 Ctrl+左 Win）：按下语音键准备音频资源后注入 DOWN（2026-09-15 候选在阻塞准备前后核对释放/取消，准备完成不等于已输出），释放统一注入 UP，断连/睡眠/中止/退出强制释放；注入时序参考 ZSTDJan 按住说话快捷键与 Voice_VibeCoding 的 Hold 语义，仅使用 SendInput 公共 API（见 ATTRIBUTION.md）。2026-09-04 修复一：和弦改为逐事件提交、事件间 80ms 间隔（WeType 拒绝单批零间隔，evidence/p）。修复二：F5 抑制器会话武装信号误接未启动的旧模块 voice_key_suppressor（ble.rs），遥控器 F5 泄漏进和弦致 WeType "额外按键"拒绝——改接 key_suppressor 并删除旧模块（Bugs\2026-09-04-wetype-zero-gap-injection.md）。加固：钩子链头 bump（会话开始 + 10s 定时）+ Raw Input 归因独立线程。**RC001 真机端到端 passed（2026-09-04，用户确认文字上屏；前提=输出端点 CABLE Input + 系统默认录音 CABLE Output）**；RC003 真机待验。
- [ ] 使用真实 RC001/RC003 和第三方语音程序（微信输入法、Win+H 等）验证按住说话快捷键：DOWN/UP 严格成对、无粘键、无重复音频，且断连和睡眠恢复后不残留按住的快捷键。RC001 基本链路与加固版回归均已 passed（2026-09-04，型号经应用 2A24 显示双证）；RC003 基本链路 passed（连接/触发/MIC_EXTEND 续期正常），音频送达率经**重配对后复测 passed**（55%→98.7%，与 RC001 基准持平，文字"一二三四五六七八九十"全对——初次配对的连接参数带宽不足，重配对即修复，已列为标准处置；Bugs\2026-09-04-rc003-voice-quality.md）。剩余待验：快速连按成对性、断连/睡眠恢复残留复验。
- [x] 实现 RC001/RC003 选择持久化、意外断连指数退避重连和 Windows 睡眠/恢复通知代码路径；真机恢复仍待验收。
- [x] 在 Windows 主机编译 Tauri NSIS Preview 安装包；Windows CI 已生成并复验绑定精确来源 Commit、SHA-256 和未签名状态的 artifact，安装、升级、卸载与正式签名仍待完成。
- [x] 提供去标识化运行诊断摘要和页面内复制入口；自动化已证明不导出设备身份、路径、端点名称或错误原文，Windows WebView 剪贴板仍待运行验收。
- [x] 持久化并展示仅保存在本机的每日按键次数、完整语音会话次数和语音采样时长；Windows/RC001/RC003 真实事件计数与升级保留仍待真机验收。
- [x] 对低于 Windows 10 1809（build 17763）的系统增加 NSIS 安装与应用启动双层拒绝门禁；Windows 10 1809 / Windows 11 提示和安装行为仍待真机验收。
- [x] 在 Windows CI 对 NSIS Preview 执行 `/S` 当前用户安装、启动存活、`/S` 卸载及设置保留边界验证；该自动化不代替可见安装界面、SmartScreen、Windows 10 1809 或真实用户环境验收。
- [x] 在 Windows CI 使用仅测试构建可启用的平台仿真，验证真实 WebView JavaScript → Tauri IPC → Rust command、五页导航、RC001/RC003 扫描、首次 RC001 语音、音频端点、Raw Input、映射、诊断和资源释放闭环；生产 NSIS 已验证不含仿真入口，该结果不代表真实 Windows API 或硬件通过。
- [ ] 使用真实 RC001 验证型号识别、BLE 配对、连接、断开、重连和首次语音。
- [ ] 使用真实 RC003 验证型号识别、BLE 配对、连接、断开、重连和首次语音。
- [ ] 验证 `STREAM_START → AUDIO → STREAM_STOP` 首次会话完整可用。
- [ ] 在 Windows 真机验证 WASAPI 端点初始化、VB-CABLE 回环、欠载恢复与完整尾音。
- [x] CABLE Input 双层静音自愈：端点主静音使用 `IAudioEndpointVolume`；音量合成器里的 SayAll 应用会话静音使用 `ISimpleAudioVolume`，只匹配当前进程在已选 CABLE 端点上的会话。初始化、会话开始、流启动后检查，推流期间每 100ms 低频检查，必要时解除静音并读回；两层均不修改音量。2026-09-07 Windows 真实 CABLE Input 受控复现 passed：端点 open/begin 两检查点，以及应用会话 begin/after_start/stream_watch 三检查点均成功从 muted 恢复到 unmuted；随后真实 RC001 连续 9 次语音均复现“流启动约半秒后会话被外部重新静音”，监视路径 9/9 捕获并恢复为 unmuted（2765 个音频包），9 次开始/停止与快捷键按下/释放均成对；修复版 NSIS 本地包由用户复测确认 RC001 语音功能 passed。边界：完整 RC003 → CABLE → 输入法语音链仍按上一项真机验收。
- [x] 生产诊断日志默认持久化到 LocalAppData：覆盖进程/Tauri/前端/Vue/首次 IPC 启动链，音频端点枚举与 `virtual_cable|bluetooth|other` 脱敏分类、WASAPI 打开/自动转换/推流/排空/中止/失败，以及按键映射和按住说话快捷键的加载、保存、重置；禁止原始音频包、端点名称/ID、自定义应用路径和异常正文进入生产日志。2026-09-08 自动化验证 passed；真实蓝牙耳机故障复现与安装包白屏现场日志验收 deferred。
- [x] 正常退出显式清理：Raw Input 监督线程不再以永久强 `Arc` 保活平台，托盘退出、Tauri `ExitRequested` 与更新安装前退出复用幂等清理；后台依次停止监督线程、同步取消按住映射/场景状态、停止 Raw Input、断开 BLE，并按阶段记录脱敏终态和耗时。生命周期定向 2 项、映射按住/监听停止既有用例、host check 与排除诊断 sink 隔离用例后的 workspace lib 均 passed；最终 NSIS 已构建。安装准备中旧版公开断开和停止监听 passed，持久配置未变；托盘“退出”精确 Invoke 后超过 60 秒进程仍存活，旧版正常退出 failed，故安装 blocked。含修复新包的进程消失、日志阶段顺序与连接中/按住中退出仍 deferred；旧实例当前为断开且监听停止。
- [x] 在可见 NSIS 安装完成后检测 VB-CABLE 服务，未安装时说明第三方来源、管理员权限和重启要求并打开官方下载页；应用首次启动复检唯一 CABLE Input 并在无既有选择时自动配置。静默安装不打开网页，真实安装/重启仍待真机验收。
- [ ] 如未来需要捆绑或自动执行 VB-CABLE 驱动包，先取得与 Pack45 内附许可一致的作者书面授权，并实现来源校验、显式 UAC、结果检测和重启流程。
- [x] 持久化用户选择的输出端点，并在端点消失或更名时失败关闭；Windows 运行时恢复仍待真机验收。
- [x] 实现设备路径 fail-closed、隐藏消息窗口、Keyboard/HID 双来源合并和停止释放的 Raw Input 代码路径；Windows 与 RC001/RC003 真机按键验收仍待完成。
- [ ] 实现按键映射保存、热加载和 SendInput：独立映射文件、显式热加载、批量 SendInput、部分提交回滚和界面测试已完成；真实 Raw Input 边沿自动执行须分别等待 Windows/RC001/RC003 确认 Keyboard/HID 事件形态，避免重复输入。
- [x] 按键映射页增加“保存配置 / 导入配置 / 导出配置”：沿用保存即热加载，导出版本化且稳定排序的 JSON；导入先做 1 MiB 上限、格式版本、动作与快捷键完整校验，落盘成功后才一次性替换运行态，取消选择不报错。Rust/Vue 自动化与 Windows COM 对话框代码路径 passed；可见文件选择器、跨机器迁移及 RC001/RC003 导入后实体按键回归 deferred。
- [ ] 完成 Windows 10 1809 / Windows 11 安装、升级和卸载验证。
- [x] 在 Windows CI 构建较低版本 NSIS 候选，验证当前用户安装、升级后单一安装身份、设置/映射/统计逐字节保留、降级不替换当前版本和最终卸载保留用户数据；该矩阵不代表真实历史二进制、可见安装界面或 Windows 10 1809 / Windows 11 真机验收。Tauri 2.11.1 静默页不会可靠设置内置降级检查所依赖的版本比较结果，已在既有 preinstall hook 中增加独立 SemVer 门禁；Run 33637195089 通过并确认 predecessor `/S` 返回 1638、当前 0.1.0 与用户数据保持不变。
- [ ] 建立自签 Authenticode、证书指纹和 SHA-256 发布流程。
- [ ] 单独评估返回键、音量键等完整 HID 实验能力。
- [ ] 按 ADR 0002 立项可选 Helper（虚拟键盘驱动 + 按设备吞键）：物理按键对照已完成（2026-09-04，豆包/微信物理可唤起、注入不可，见 Bugs/2026-09-04）；待完成 RC001/RC003 按键形态真机确认；驱动来源初查完成（cgutman/WinUHid，MIT，源码小可审计，无预编译 Release 需自构建签名），详见路线图阶段 E；不得进入基础路径，不阻塞 Preview。

### 2026-09-05 附加

- **应用内更新（tauri-plugin-updater + GitHub Releases，新增）**：关于页"检查更新"手动入口 + 启动静默检查（失败完全无声）+ 下载进度 + passive 安装自动重启；默认稳定通道使用 `releases/latest/download/latest.json`，用户可显式开启“检查预览版更新”，经 GitHub Releases Atom feed 选择最高 SemVer 的已发布版本（包含 Pre-release）；开关默认关闭并持久化，两个通道均由 minisign 强制验签。安装器启动前经 `on_before_exit` 显式断开 BLE 链路（插件在 Windows 上 `std::process::exit(0)` 不走 Drop 清理）。待完成边界：① 已安装 0.2.1 不含预览通道开关，无法自行发现 Pre-release，0.2.2 首次引导需单独处理；② GitHub Secret `TAURI_SIGNING_PRIVATE_KEY` 未配置时 CI 用一次性密钥兜底、正式 Release workflow 直接失败；③ Authenticode 代码签名仍待建立（updater minisign 验签独立于 Authenticode）；④ 大陆访问 GitHub 的网络可用性未量化（插件支持多端点兜底与系统代理，已留扩展位）。参考与源码核对记录见 ATTRIBUTION.md 更新调研节。
- **BLE 僵死链路自动恢复（bluetooth_radio.rs，新增）**：应用被强杀后 OS 侧 GATT/HID 链路或服务缓存可能僵死，普通重试永不恢复（真机取证 + Qt 论坛同结论：关开蓝牙是唯一有效公开 API 手段）。重连循环连续失败 5 次（约 60s）自动关开蓝牙无线电一次（Off→2s→On），每僵死周期最多 2 次防抖动，UI 提示全程可见；WinRT Radio API 未打包进程可用、无需提权（真机验证：开关周期后重连立即成功）。详见 docs\investigations\evidence\p\FINDINGS.md 2026-09-05 节与 ATTRIBUTION.md BLE 恢复调研来源。
- 语音键 F5 抑制器补防粘键配对（VVC 同款"DOWN 漏进 OS 则 UP 必放行"）：按下沿 60ms 有界等待超时泄漏时，释放沿放行，杜绝"F5 粘住→和弦全部被拒"的整机失效模式。

### 2026-09-05 IME 专项

- **语音"无法唤起"根因 = 会话活动输入法不是微信输入法**（WeType 语音热键仅在自身活跃时生效；焦点无关，桌面/资源管理器聚焦 6/6 照常开麦）。修复（ime.rs）：注入和弦前用公开 TSF API 会话级激活 WeType（TF_IPPMF_FORSESSION，零延迟 3/3 实证），失败不阻断。参考 macOS 版 PreferredInputSourceMonitor 职责设计。

### 2026-09-05 性能已知项（评估归档，供后续修复）

背景：首按失败根因修复（80ms 回退 + F5 三重防线，PR #19）验证通过后，对语音链路做整体性能评估。当前全链延迟 ~300ms（按键→开麦），其中外部因素 ~200ms。逐项账目与处置边界如下，**勿盲改**——每一条都有实测依据。

**外部边界（第三方/固件，不可干预，勿再投入）**：

- WeType 内部识别和弦→开麦固定 ~163ms（evidence/p 13 次实测 ±5ms，端点预热无效）。
- BLE/固件按键→0x04 通知 ~30-60ms（0x04 早于 HID F5 60-90ms，触发点已最早合法位置）。

**正确性取舍（"延迟优化必须保证成功率"规则项，勿回退）**：

- 和弦间隔 80ms：20ms（cef24d3）冷/节流态必失败（2026-09-05 用户实证 7 次发作），86e5314 回退。热态验证 4/4 不代表可交付。
- F5 解粘 20ms（和弦前保险 UP + 间隔）：跨应用重启的 OS 粘键状态无法便宜检测，须无条件执行。

**待修复项（按优先级）**：

- [ ] **笔记本功耗：后台节流豁免改为条件化**（bf03f0e 当前全局豁免——它是首按修复的组成部分，全局豁免代价是闲置功耗略高）。方向：仅在遥控器连接期间豁免，断连后恢复参与节流；重连可靠性已由 WakeReconnect + 无线电自愈兜底。台式机无影响；上笔记本场景前处理。
- [ ] **失败恢复加速（可选）**：wetype_check 检测判据从 ConsentStore 注册表（700ms 检查窗）换 LL 钩子 0xFC 标记观察（毫秒级，kb-live 已验证与开麦 100% 交叉一致）。仅加速失败路径的重试触发，成功路径零收益；需抑制器/钩子层新增观察通道，注意钩子线程不做 IO。
- [ ] **冷态管道 ~120ms（低优先级）**：闲置后首按应用内部链路（GATT 回调→武装→工作线程→IME 查询→和弦）实测可拖 ~120ms（不失败但慢）。节流豁免已生效仍有首次线程调度延迟；武装已内联到 GATT 回调。进一步压缩收益 ~100ms 冷态延迟，风险中（动的是刚修好的链路），无用户报障不动。

### 2026-09-07 按键映射单响应专项

- **2026-09-14 当前范围调整（用户确认）**：在统一完整按键模板目录中恢复 Agent、聊天工具、浏览器三个固定语义模板，内置本体只读、稳定 ID、可复制；不恢复尚未实测的第三方应用能力声明。当前边界以 [PLAN](docs/PLAN.md#当前范围调整固定语义模板恢复2026-09-14-用户确认) 为准。
- [ ] 逐应用完成场景控制真机验收：分别验证 Codex、微信、浏览器的任务列表/内容区/输入区识别和显式区域切换；任务必须实际选择并打开，识别失败不得干扰 direct 模板或通用映射。固定模板目录开放不代表本项通过。

- **冷首按原生残留（2026-09-08 用户调整策略）**：武装族按键（确定/方向）在闲置 >4s 后的首次按压会附带一次原生按键动作；同键映射由泄漏对冲保证净单响应，不同键映射仍会同时出现原生动作与配置动作。左键已恢复自定义，与上/下/右/确定使用相同的逐键 4s 武装机制。该历史阶段的 Home/TV 方案 C“遥控器优先”常驻抑制已被 2026-09-15 用户明确撤销；当前必须修复实体键盘共存，不能沿用四秒武装作为设备归因。
- **返回/音量±配置保留与增强执行（2026-09-10，替代上述三键禁用结论）**：三键配置继续持久化、导入导出和编辑；基础运行时仅在已确认连接的 RC001 上允许三键执行；2026-09-14 新增按所选 HID 父节点绑定的专用驱动通道，只有报告契约/ABI/显式 CLAIM 就绪才允许 RC003 增强执行，未知型号保持 fail-closed。当前无已签名驱动及实体执行证据，不能宣称已执行。RC001 配置恢复和库测试已 passed，实体按键、冷首按、闲置、断连、睡眠与按住退出仍为 hardware deferred；RC003 增强取决于外部输入能力，当前为 blocked external。
- [x] 固定语义模板软件闭环：Agent、聊天工具、浏览器由唯一推荐定义生成，固定置于统一模板目录前三项；内置本体不落盘且 update/delete/rename/导入覆盖均由服务端拒绝，可深拷贝为独立用户语义模板后编辑、改名、删除。导入导出只携带合法内置稳定 ID 引用；程序在 direct/semantic 两张绑定表间保持互斥。只有用户已开启跟随且当前绑定为 semantic 时启用 scene，direct/未绑定路径保持普通映射。内置调节模式在单次应用运行会话内按模板记忆，重启回默认；用户模板仍持久化。catalog/保护/复制定向 10 项、builtin 导入导出 2 项、scene 14 项、原子 kind 切换 1 项、host simulation check、前端保存保护 28 项与 build passed；最终本地包已管理员覆盖安装并以真实用户非提升启动，安装版前三顺序/只读操作及“浏览器”复制—修改—保存—重开—删除 passed，测试副本 0、内置正文落盘 0、原跟随开关已恢复为 true。内置实际程序绑定、RC001/RC003 和真实第三方应用语义动作仍按后续证据判断。
- [ ] 完成固定语义模板的三区域图形矩阵纠偏与整合验收：继续以 `regionActions` 为唯一真源，按“模板 → 区域”一次编辑一张 13 键 single/double/long 图；内置只读、复制品可改。当前 core 已补齐显式 Disabled、聊天列表 Left/Right、浏览器 `FindPage`、输入区四方向光标语义，以及列表/内容区 Up/Down 长按安全翻页；Menu/Voice/Volume 固定规则与按模板调节模式保留。安装后发现 Rust tagged `SemanticAction` 被 TypeScript 错当字符串，导致非固定语义动作统一显示“不执行”；现已保留磁盘 object 真源并把 bridge/消费者统一为 `{ type: SemanticActionType }`，Rust 生成的三内置 catalog fixture 由 Rust/TS 契约测试逐值锁定。矩阵及能力门控定向测试、Windows 库 check、共享前端定向测试与 build passed；官方 mockIPC 的 common/direct/builtin/user 四态双视口布局和真实动作标签 passed，同尺寸 bbox 差异 0 px、配置写 IPC 0。含本次 ABI 修复的 NSIS 已构建、校验并完成管理员覆盖安装；实际载荷只含预期 Tauri marker 差异，真实 Explorer 用户 Medium 实例启动与有限启动日志 passed，安装前后及启动后两份配置哈希不变。后续按键页统一图形真源已修复旧 CSS 与画布计算的 35 px 纵向偏差，隔离双尺寸四态均显示 13 实体键、选择器 36 px、左右同行差 0 px，最大连线起点/热点中心差 0.054567 px；新包已完成覆盖安装与真实 Explorer 用户非提升启动，配置逐份保留。新版安装版图形、RC001/RC003、Codex/微信/Edge/Chrome 的真实区域、候选框和动作执行仍待验收；自动化不得替代这些结果。
- [ ] 完成统一程序关联排序及企业微信 semantic 能力验收：direct/semantic 两类关联已共享全局 `menuOrder`，完整 ID 集合经单一原子事务校验后才能跨类型移动，类型更换保序且不改变任一运行开关；Rust 15 项、semantic 阻断 1 项、host simulation check、前端 9 项、build 及双视口隔离排序/重载/对齐均 passed。最终本地包已完成管理员覆盖安装与真实 Explorer 用户非提升启动，安装载荷归一化匹配，安装前后及启动后两份配置哈希不变。后续关联 UI 的刷新/busy/拖动保护定向 20 项 passed；隔离浏览器真实 pointer 触发原生 `dragstart`，跨 direct/semantic 投放只发出 1 次全量 reorder IPC，重载顺序保持。主窗口已设置 `dragDropEnabled=false`，仓库未发现原生文件拖入消费者；含此增量的新 NSIS 已完成覆盖安装与真实 Explorer 用户非提升启动，安装前后及启动后配置逐份不变；安装版 WebView2 用户实际拖动仍待验收。企业微信当前路径、绑定、内置聊天模板引用与开关已只读核对，但 `wxwork.exe` 仍走 Generic adapter；现有日志没有企业微信前台事件，有界 UIA 顶层读取也未见列表、输入或滚动 pattern。须在用户明确的安全按键场景中取得同一代 `scene_foreground`、`scene_route` 与动作终态后，才能判断具体能力 passed/failed；不得复用普通微信日志或注入裸方向键冒充支持。
- [x] 完整按键模板软件闭环：按键页可将全部普通按键的单击/双击/长按动作另存为独立模板；模板页可编辑、复制、改名、删除。统一“添加程序”流程只在弹窗内发现/搜索运行中程序或调用 `.exe/.lnk` picker，确认后才保存；主页面以“软件名称 / 选择的模板”列显示已添加关联，取消零写入、后退保留选择、重复关联拦截。自动切换默认关闭，添加模板/程序/关联不暗改开关；关闭保留关联，未命中使用通用配置。2026-09-11 真实桌面先修复 Windows Codex 公开包内 `ChatGPT.exe` 与既有 `codex` 绑定不一致；随后 18:06 新实例日志证明身份与 profile 已命中，却因 BLE 后台从等待能力推进到 Ready 后没有同步映射连接门控，造成通用配置和所有模板均为 `enabled=false`。当前源码由唯一 BLE worker 在锁外发布完整连接生命周期，连接上下文定向 2 项、断连按住释放 1 项、host check 与 fmt passed；不依赖页面查询，重复同步幂等，Shutdown 后不复活。2026-09-14 Q-Dir 旧实例再次实证绑定/profile/实体左键均命中而连接门控仍清空执行能力；同日用户正常退出旧实例后，已用 `artifacts/qdir-kk-profile-final-20260914/` 本地包管理员覆盖安装，普通用户非提升实例日志证明 Ready 后映射恢复为启用、Q-Dir profile 命中、Left 边沿成对并完成注入，用户实际确认右键菜单弹出；未绑定前台回退通用配置也 passed。诊断 chord 字段与持久配置的 `Apps` 枚举仍有不一致，列为日志可观测性缺口，详见 `Testing/StructuredTemplatesAndDrivers.md`。
- [ ] 在 RC001 与 RC003 分别验收完整按键模板：覆盖模板切换前后全部普通按键的单击/双击/长按、按住中切换的取消与全释放门禁、快速/连续按压、断连和即时语音；语音键不得进入模板手势。
- **2026-09-15 三键旁路范围调整（用户确认）**：按 [PLAN](docs/PLAN.md#当前范围调整三键宿主-hid-旁路2026-09-15-用户确认) 验证指定 RC003 宿主只读报告旁路；官方 core devkit 身份核验、探针编译、解析/边沿隔离测试通过，专用 LocalService RX 目录修复载荷加载拒绝访问。14:29 的实际序列获用户确认，RawInput 记录首尾确认键两对 DOWN/UP，但宿主指定报告 0，故捕获候选 **failed**。随后匿名分类证明 6396 次对象查询均成功但合法名称与选定 PDO 不一致；不能接受全宿主或前缀来绕过设备归属。两次 600 秒窗口正常停止/撤钩/分离，JS 分类 4 项测试 passed；没有生产接入或新应用包。逐键映射/抑制未验收，键盘共存未修复，详见 `artifacts/hid-host-three-key-20260915/evidence.md`。主程序与语音独立、安全设置不变；RC001 无硬件不外推，旧驱动未签名事实保留。
- **2026-09-19 三键后续打开元数据候选**：用户要求继续开发后，已移除旧 PDO 等同及 payload 读取分支，新增所选 devnode 公开接口精确 future-open/close/duplicate 生命周期观察；11 项隔离测试和 C 编译 passed；唯一120秒自然观察中2个所选接口的 future-open 为0，550次既有IOCTL仍未知，已正常撤钩/分离并退出。没有来源闭环，停止该用户态路径；不能靠反复按键、等待或重连作为产品解法，未恢复三键或键盘共存。证据 `artifacts/hid-host-future-open-20260919/evidence.md`。
- **2026-09-19 签名驱动候选差距复核**：12项核心源码与既有审查冻结同SHA，SYS/INF/CAT同字节，三键最小通道/抑制/映射候选已存在；当前用户无明确可用Microsoft签名渠道，外部前置不可用，未安装未签名驱动。签名后仍缺实际WDF层/物理报告/逐键与维护验收，TV/Home共存仍非三键mask范围，RC001仍无硬件/INF；详见 `artifacts/hid-host-future-open-20260919/kernel-gap.md`，不采购/申请/上传或扩大本轮提交。
- [ ] 修复实体键盘与遥控器 TV/Home 共存：撤销在线常驻及四秒来源推断，来源未知不吞键；实体反引号、~、Home/组合正常，遥控 TV/Home 映射单次且无原生泄漏。与三键旁路合并验证冷/闲置首用、快按/重复、交替/同时和断连/睡眠/退出；当前源码根因已确认，三个显式 opt-in 回归用例实际 failed，未混入默认稳定入口；尚未实现与真机验收。
- [ ] 在 RC001 与 RC003 分别验收结构化模板和三键：覆盖返回/音量±、冷首按与闲置、断连、睡眠、按住退出、完整三区域实体键矩阵、菜单释放与即时语音。RC003 增强在外部输入能力未就绪前保持 blocked external；验收步骤见 `Testing/StructuredTemplatesAndDrivers.md`。
- [ ] 场景控制恢复前，在真实 Codex、微信和浏览器中验收前台绑定与 UIA：程序列表选择确认、聊天候选/发送、浏览器导航/缩放，以及 overlay 不抢焦点。当前真实 UIA 与第三方应用结果为 deferred。
- [x] 场景浮层、驱动入口与组件软件边界：scene 7 项测试、host Windows check、组件 13 项测试（常规 11、官方已下载包校验/回收 1、固定官网真实下载/校验/回收 1）、Helper cargo check 及前端测试均 passed。驱动页提供状态检测和用户主动配对引导；VB 区域固定说明官网 Pack45 来源、donationware、下载验签、UAC 与官方向导边界，用户点击明确主按钮后才执行原受控命令，校验通过后才触发系统 UAC 和官方交互安装器，旁路参数被拒绝。实机 IPC、UAC、向导实际操作与实体语音仍 deferred。
- [ ] 完成可选 HID 增强驱动与 VB-CABLE 实装验收：VB 不捆绑或镜像二进制、无可靠 Repair 命令，仍需验收安装/卸载、升级和端到端音频；HID 已有三键专用 KMDF/raw PDO 与普通用户接入候选，C 状态机、Windows lib 152 项和 WDK 编译通过；2026-09-15 冻结应用包覆盖安装、普通权限启动、载荷及配置保留通过，未安装内核驱动；正式 Microsoft 签名、Helper 系统实装维护验收、双型号实体报告和生命周期仍未完成，不能承诺按钮可用；详见 drivers/SayAllInput/README.md。
