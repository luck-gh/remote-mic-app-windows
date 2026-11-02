# Windows RC001 / RC003 Preview 测试手册

## 2026-10-24 已可读所有权报文的有界交接候选

- passed (真实IPC/自动化, 非遥控器验收): 旧样本过期但同代完整fresh O已可读时, 原代码timeout/available false→true已实证, 正确预期RED后最小有界预读和安全交接GREEN。覆盖旧持键取消/UP、旧E拒绝、新E单次动作、claimed面板配对、无效与半行输入、投递失败、满预算无换行关闭; 记录型注入器不操作真实桌面。完整Rust矩阵与独立review通过, 精确身份和失败夹具记录见原Bug。
- passed (23:14实际部署): 21:22门禁停止后用户明确允许安装, fresh语音配对/正常退出/STOP回执/六配置/覆盖安装与普通Explorer新App身份均已核对。99D3F17A/8a3cf279实装App7521B5BD, Helper/Agent/Gadget与Normal任务不变; 首次观察者权限不足及runtime配置瞬态false保留原记录, 后续只读确认身份和六文件回到基线, 不倒推瞬态差异原因。
- failed (23:15:29持续性): 新候选probe bytes0、buffer/pending0, 旧sample age1500时真实timeout, 同窗Helper wait1795/bridge idle1607ms, 后新O age0恢复不是通过。未启动4小时统计或继续硬件验收, 现有自动化只证明已可读fresh O的安全交接路径; 不把它当成全部现场故障的唯一根因, 反引号/来源隔离仍未修复。

## 2026-10-24 普通调度任务候选

- failed (18:17起用户定向测试): Menu/Back/TV先正常, 语音首用不可用且报告全按键灰态, 重试恢复; 不标整轮通过。旧候选窄窗未观察capture timeout/BLE断连, 音频回调和成对释放不代表第三方实际出字, 灰态及语音失败原因待归因, 详细时间链归原Bug。
- passed (后续 App 诊断自动化/部署): 同轮 sample/decision tick/age 回归真实 IPC RED→GREEN, bridge44项和完整 workspace/fmt/check/runtime-simulation check 通过; 18:46正常升级仅App诊断, 原Helper/Agent/NormalTask保持, 实际resumed两条新增字段算术匹配。首次voice门禁停止及启动后配置总false均保留, 后续逐文件六项恢复基线且无回写; timeout故障分支自然实证仍deferred, 不扩大为持续性修复。字段与源码/包身份见原 G1 记录。
- failed (持续性): 15:26:28 起的连续观察在 16:02:03.103 发生 ownership_timeout, 03.130 恢复不构成通过; 观察35分钟后收口, 未完成4小时。实际 Helper/任务仍 Normal/Priority4, 同窗 renew wait1764ms 与桥接 idle1161ms; 没有故障时 ETW, 不能再次直接认定 Ready 调度或 Agent 消费根因。先前构建短窗 passed 保留, 不扩大为持续性通过。
- passed (源码/自动化): Helper 实际 XML 解析与 UTF-16 BSTR、注册前同用户门禁、App Date/Principal/策略后验、取消/迟到回调; Helper 104/3 ignored, workspace core61+4/Windows619+21 ignored/App189+1 ignored, fmt/check/runtime-simulation check/selftest 和独立 review 通过。精确源码与失败过程归[原 G1 记录](../Bugs/2026-10-03-foreground-input-regression.md#2026-10-23-g1-续约同轮与控制写失败诊断)。
- passed (2026-10-24 15:13–15:16, 本机 RC003 部署): 旧代正常 STOP/退出与安装配置保留, 新 App 由 Explorer 普通权限启动; 经 App 明确授权后实际读回个人任务 action、Principal 当前用户、InteractiveToken/HighestAvailable、Priority4、电池 false/false、PT0S、无 trigger、AllowHardTerminate=false。新 Helper 实际 NORMAL(0x20), renew 相对 priority0, 原 host/Agent 未换代, HELLO 与 owned=7167 成立。此为注册/接管核验, 不是持续性或硬件完整验收。
- deferred: 真实 UAC 取消/注册失败不得 reset/trigger, 原 STOP/回执及恢复仍配对; COM 闭包失败测试不等于实机 API 失败验证。
- passed (本轮短窗, 非完整 G1): 新候选一次真实标准 release 构建 146.66s, 与目标 ETW 行重叠 47.451s; 95 条完整等待链最大 Ready→run 29.882ms/总间隔 530.015ms, 无 >=750ms 链或同窗 timeout/rx_silence/写失败。实际 schema、零丢失/解析失败及自身会话清理均通过独立复核; 构建阶段不同, 不称严格同负载 A/B 或优先级因果全闭合。
- G1 尚未通过: 原普通 release 构建 A 的 4 次 Ready 排队 1.046–2.159s 和 4 次 ownership_timeout 保留。新候选连续观察实际始于北京时间 15:26:28, 原计划上限 19:26:28, 已因16:02故障提前收口, 不再等待原截止或累计为4小时通过; 后续唯一60分钟诊断观察未捕获新故障, 不重启稳定通过统计。主动关闭、睡眠或换代不得累计为连续合格区间; 三轮硬件矩阵与 RC001/跨机/跨 Windows 未验证。

## 2026-10-23 菜单关闭交互复验

- passed: 同机18:54用户确认Menu开/关后鼠标点击输入框不再唤回菜单, 输入与Back正常, 重开上下导航正常; 日志6次hide_completed与visible=false, 首Back直接map_fire, 无close blocked或下一按键清理残留。
- 自动化passed: Controller边界确定性RED/GREEN、所属8项、scene84/1ignored、完整workspace core61+4、Windows619/21ignored、Host185/1ignored及fmt/check/runtime-simulation, 独立review通过。测试不以真实窗口激活验证前台恢复。
- failed/未关闭: 同次自动目标焦点恢复6次window_operation_failed; 同App/Helper16:22:02–18:56:25有界日志29次ownership_timeout、1次rx_silence/断连, gate_write_failed和日志overflow均0。日志文件覆盖区间未观察到轮转缺口, 不能据用户感觉稳定宣布全按键持续性通过。
- 被测setup为`8d77ffa8cf11098dfda5233bac048f83c6899dc23b112366992f548c12974ef0`, dirty fingerprint为`04cac8eb38673df07a56104c91159fa819101d4bc7ce66e81bde1db52820772b`;完整边界见[菜单令牌记录](../Bugs/2026-10-03-foreground-input-regression.md#2026-10-23-菜单返回目标令牌校验)。

## 2026-10-22 连接续约会话代次候选

- **passed（2026-10-23 菜单 token 修复自动化/实装）, pending（精确鼠标复验）**: 确定性Controller边界RED为StaleToken/calls=0, GREEN所属8项与scene84/1ignored; 完整workspace core61+4、Windows619/21ignored、Host185/1ignored及集成、fmt/check/runtime-simulation均通过。原message-only激活假设的full失败保留, 新测试只做validation或注入边界, 不实际激活。16:22正常升级后Explorer普通权限/新指纹/同版Agent附着/owned7167、六配置保留通过, setup `8d77ffa8cf11098dfda5233bac048f83c6899dc23b112366992f548c12974ef0`;身份和ACK字段顺序门禁拒绝/受控续装归原Bug。等待Menu开/关后鼠标点击输入框不再现及焦点/Back确认。旧会话16:09–16:13仍有timeout/断连, 持续性未通过, 不外推菜单修复或短窗0异常为全按键已解决。
- **failed（2026-10-23 新包首轮交互验收）, passed（本轮首 Menu/TV/两次语音）**: 用户观察第二 Menu 视觉消失并丢失输入焦点, 鼠标点击任意输入框使菜单再现, 首 Back 未删而后续关闭菜单后可删。新会话日志有两次关闭时 `stale_window`→保留菜单→首后续按键取消的完整链, 未见同期 ownership_timeout/reader_ended; 用户澄清不是键盘触发, 方向键或鼠标点击另一输入框可使菜单消失; 不能将整体标记通过。精确时序及 Controller 返回目标契约冲突候选归原 Bug; 开关/退出压力测试未测, 未提交/推送。
- **passed（2026-10-23 新包安装与实际加载）, pending（硬件验收）**: 用户正常重启后旧 host 已结束且无 App/Helper/端口占用, 新包正常覆盖安装、六配置逐字保留、Explorer 普通权限启动; 失效 binding 自动失效, 个人端口 47832 的 HELLO 确认为 `2026-10-23.auth-lease`, owned=7167。setup SHA-256 `8e3cc243f3ad5339b86e89a0f62f51c38e44b9d8f327cf121735b536a142a90b`, source fingerprint `1a3b22c8518e1c9500f046f726aa2e8ab2573e855fed0f9c08cda4368fad3b5f`, local/dirty/included; App/Helper 安装 hash 和真实进程身份见原 Bug。本次前端嵌入、签名、生产隔离与安装载荷门禁通过, 不改旧包 manifest。下列未部署/安全关闭描述保留为此前阶段事实, 当前新包已运行等待冷首 Menu、Back/TV、语音及开关边界实测; 旧 bridge3 硬件 failed 不改为 passed, 不提交/推送。
针对用户报告的 Menu 偶发原生行为/恢复后重新映射，现有同会话日志已证实 Helper-Agent TCP 10053 断线、App 所有权缺失及后续恢复，最初断线原因仍 unknown。时间线和累计 STAT 的可信边界归 [原 Bug](../Bugs/2026-10-03-foreground-input-regression.md#2026-10-22-接管瞬断的连接代次隔离与诊断候选)。候选仅修可重复的跨连接 renew fault/授权污染并补诊断，不宣称整体接管或实体反引号已修。

- **passed（失败后的离线切片，不是已安装包验收）**：Agent auth/lease 分离及旧 socket 回调隔离的 116 项 VM 测试通过；Helper 新旧 build 标记真实 RED 后同步到 `2026-10-23.auth-lease`，自检 60 项通过。Helper 空闲通知三项真实命名管道 no-wakeup RED 后，七项数据/仅 O/关闭的等待前后与 sender 断开用例通过，包含 FIFO 数据不丢、O-before-E、真实 BYE/worker 完成；全套 89 passed / 3 ignored、fmt/check 及独立 review 无阻断，原慢日志 sink/持续数据/renew 公平用例仍通过。入口为 `node hardware/RC003/helper/agent/agent_logic_test.mjs`、`cargo test --locked --manifest-path hardware/RC003/helper/Cargo.toml`、同 manifest 的 fmt/check 与 `cargo run --locked --manifest-path hardware/RC003/helper/Cargo.toml -- --selftest`。过程红/绿复用 `agent-marker-*.log`、`bridge-wake-*.log`，首次夹具并行撞名和 newline 比较误差不算业务 RED；最终三项 RED 均先确认真实管道副作用、再因未被通知唤醒失败。
- **passed（Bridge 局部日志隔离，离线）**：真实管道 sink 阻塞的两项业务 RED 后，Bridge 42 项和诊断 18 项通过（重叠，不相加）。进程级唯一容量 256 worker 复用跨连接/Bridge 生命周期，入队前生成原格式时间戳，Full/closed/start_failed/write_failed 分别可观察；append/flush 失败不计成功，恢复后聚合诊断健康记录。UP/inactive 与到期用例均在 sink 释放前观察实际状态/消息：edges_applied 增二且新 epoch 的非零 R 已确认，不以超时、旧消息或仅 bool 判通过。证据在 `target/dev/capture-state/bridge-log-reader-red.log`、`bridge-log-isolation-suite-final.log`、`bridge-log-isolation-diagnostic-final.log`；独立 review 无阻断。仅 Bridge note 流离开 reader/watchdog，mapping/其他同步日志风险未排除，进程退出不保证所有诊断排空。
- **passed（最终组合自动化）**：`cargo fmt --all -- --check`、`cargo test --workspace`、`cargo check --workspace`、`cargo check -p sayall-windows-app --features runtime-simulation` 全部通过，前后四冻结源码散列相同。workspace core 61+4、Windows 615 passed / 21 ignored、Host 185 passed / 1 ignored，其他集成/文档通过；Helper 非成员，复用上述 89/3 与 Agent 116 精确源码证据。完整日志为 `target/dev/menu-capture-reliability/capture-final-combined-{fmt,test,check,simulation-check}.log`，结果/时间为同目录 `capture-final-combined-results.json`。前端未变，不重跑或构建；此次没有新安装 manifest，不复用旧包身份冒充新源码实装。
- **deferred（实际加载与硬件）**：当前源码含新 auth-lease build，未重包/安装，不能复用下方 bridge3 安装 manifest 或硬件结论。现场旧 upstream-usage 驻留 ignore 脚本没有本次已授权热换入口；保持增强安全关闭，不要求用户现在重启或把反复重启/复测当作日常恢复办法。50ms 仅原控制轮询超时，不是 OS 调度保证；本轮没有精确配对跨端唯一 E 测点，仍不能唯一定位 Back 的 646ms 或 2231ms 等待返回区间的具体调度原因。下列已安装包硬件 failed 与安全停用事实继续有效。
- 后续回归审计采用三个等长 15 分钟窗口；timeout 分别 17、0、52，旧包已存在最长 4923ms 的 O 排队，新包故障期最长 3076ms。负载/用户动作未控制，不能由频率确认或排除 1f 回归，也不能归因 CPU。旧/新安装身份、一次有界指纹复算未命中及新增日志热路径风险归原 Bug；不把相同前端资产名称/大小当成全部源码字节一致。
- 后续 bridge3 候选已本机安装、用户硬件验收 **failed**：Helper 接收 HB 时采 GetTickCount64，显式 active/inactive/closed，O/E 冻结 Agent connection、ownership epoch、observation sequence 和已 ACK target，R 仅隔离旧观察代次，不宣称可靠物理来源。latest O 跨越短 inactive 时 App 仍须先撤权/取消/释放 Driver 持键/观察归零；首次/撤权后重新授权建立 sequence floor，连续有效 owned 的 HB 不前移 floor，避免同 epoch 本地超时后的旧 E 被 fresh O 带回执行。正常有效同 epoch E 保持 FIFO，不按年龄丢边沿。新旧线协议拒绝混用，不加 fallback，不改 Agent 或现有 1500ms 阈值。
- **failed（该 bridge3 包本机硬件复验）**：用户确认闲置首 Menu 失败/第二次成功、语音失败、Back 两次删除过多，TV 成功。UTC `15:59` 发生 2231ms sleep 返回区间迟到、OS 10053 与约 15.329 秒 Agent 重连空窗；`16:03` Back 的 Agent DOWN/UP 间隔 228ms，Helper UP 至 App UP 晚 646ms，App 六次 repeat 加两次单击共八次 queued。当前模板仅 Backspace，非 Ctrl+Backspace/双击动作；逐 E 出线/读取/执行分段缺失，不凭后续恢复或自动化通过宣称整体修复。Voice attempt 26 abort、零音频 callback；无 chord_press ok 不能反推没有 DOWN，取消分支可在成功 press 后先 release。原始/最小脱敏证据复用 `capture-state-installed-failure-window.json`、`capture-state-user-failure-window.json`、`capture-state-failure-details.json`，详见原 Bug。
- **passed（故障后正常安全停用，不是功能验收）**：UTC `16:07:28.037` 经已消歧的正常关闭按钮停用；Agent STOP passed/identity_match/hook_detached，匹配 fresh receipt，App release_confirmed，Helper/监听端口均释放，repeat mask 清零。`16:13` 左右 Ctrl/Win/Alt/Shift 与 Backspace 全 UP，UI 实际已关闭；仅正常停用更新增强开关为 false，保留用户映射/模板/音频。保持停用，无重装、采样、重试或新提交；后续诊断须先补足精确失败分段，不扩大既有 passed 范围。
- **passed（后续业务 RED / Helper focused GREEN）**：真实隔离命名管道旧实现实际出现过期 O 仍写出、false HB 无即时撤权、持续 R 队列阻塞 T/W/S 与 Stop，四项因预期业务原因失败；不是仅用 v3 解析失败代替业务 RED。review 另发现取 O 后才取 E 的并发窗；闩锁精确在此窗注入 active+DOWN/UP，真实线输出实际得到 ordered=false。现取到 data 后再读最新 O 并先发送；两个 FIFO 边沿各一次、不重 stamp。当前 Helper `capture_` 为 12 passed、`cooperative_cleanup_tests` 为 22 passed；另含排队 true 被后续 false 替代、真实日志 sink 阻塞/队列满与持续 R 同时发生时，实际管道 O/T/W/S/Stop 及生产 renew 线程 TCP 至少两次续约有进展。最初新增测试缺 BufRead trait 的编译错误已修正，不算业务 RED。隔离目录和唯一测试管道不触生产端口、回执或用户配置。这些 focused 证据本身不能代替后述组合实装，更不能当作硬件持续性通过。
- **passed（bridge3 组合自动化/审查）**：接收端 37 项真实 IPC/边界回归与 setter 1 项通过；Helper 全套 82 passed / 3 ignored、fmt/check 通过；workspace core 61+4、Windows 609 passed / 21 ignored、Host 185 passed / 1 ignored，其余集成/文档通过，fmt/check/runtime-simulation check 通过。三处 P2（首 E 超车 O、本地 timeout 后同 epoch 旧 E 复活、共享锁内同步观测）均经独立 review 闭合。首次 Helper full 仅旧 split fixture 期待 OK2 而输入已为 OK3 失败，修期待后全绿，不算生产故障。前端未改，复用 506 项证据并在打包中重新 build；不运行暂停的 bulk native 或 RawInput 采样。
- **passed（bridge3 实装与有限读回）**：UTC `15:56:26` 正常覆盖安装、Explorer 普通权限 App 26148，Helper 74732 实际安装路径/hash 与精确创建代次读回；六配置、临时 binding/host 精确代次保持，无 SYS。setup SHA-256 `55777dd77d5ff49bee967c0f4225a99091d7f07ec59ad1f81bcffaac4a9385a7`；installed App `2df8aaf8d9a6016a6d2e7017da73e50c465ae4ac39a066dce4f5572ed79ef3ab`；Helper `938d942c30826f985e59d70b8de906f3e6ed7e010fbb65c585442edcabd332b7`。base `1f1baec`、fingerprint `e3284f2c028ce9b1ceada9e8134a7883267636d2ed17b9cfef063ef50e028ad7`，local/dirty/included；后置诊断 example 的离线修订不在这个固定 manifest 中，不覆盖重算为当前工作区身份。真实 descriptor version=3，HB1 false 后 inactive，HB2 true/epoch2/target2 对应 App owned=7167；30 秒聚合样本最大 203ms/write=0。只是实际新链生效，不能代替冷态、持键关闭/重开与长期稳定验收。
- 升级停止边界：最初语音门禁只查当前文件，轮转后零条而拒绝；本地脚本仅补当前 PID+创建时间范围的合法轮转、物理 UP/attempt terminal/chord 清理，7 个正反 fixture 与实际只读检查通过。原 Helper 72348 的 stop 仍 unconfirmed，随后本次正常退出的 App 4180 明确提交 cleanup_only，后继 82392 经 RECOVERY Confirmed→App release_confirmed→整体 shutdown passed，才获准显式续装。Confirmed 分支强制同 instance、本次 stop_id、hook_detached 与 released_all，ACK 正文未单独留证；不是凭 passed 回执或进程退出推断。Task Ready/0 实例、无待启动控制进程、无 App/Helper/端口再次确认；停止意图按契约保留，不清回执、不强杀。本地门禁脚本/测试不纳入源码提交，原两次拒绝证据保留。

- **passed（RED）**：旧授权在新连接上的读取回归实际失败；旧共享 fault 在受控 TCP 新会话已开始后到达，读取循环实际提前退出。首轮合跑第二项受预期失败的测试 Mutex poisoning 干扰，随后单独执行取得真实旧 fault 失败，不把 poisoning 当作产品证据。
- **passed（定向 GREEN）**：Helper 的 `cooperative_cleanup_tests` 22 项与 `capture_log_stall_tests` 3 项通过，包含旧 fault 不关闭新代/同代 fault 仍可结束、授权和旧撤权隔离、正常收尾和日志接收线程不阻塞；新增 HB 诊断回归核对 connection_generation、helper_hb_seq、helper_receive 原点和不泄漏 synthetic token。授权回归还通过真实合成 loopback socket 验证旧代无 renew 字节、当前授权后实际收到完整 renew，再撤权后不再写。
- **passed（独立 review / 完整自动化）**：两源码独立只读 review 未发现 P1/P2；Helper 独立 workspace 全套为 75 passed / 3 ignored，其 fmt/check 通过。根 workspace 为 core 61 + 4、Windows 597 passed / 21 ignored、Host 185 passed / 1 ignored，其他集成和文档用例通过；根 fmt、workspace check、runtime-simulation check 与 diff check 通过。根 workspace 不含 Helper，不能互相代替。前端源码未变，复用同组合前端 33 文件 / 506 项及 build 证据。回归没有实际运行完整续约线程与 accept 交接的全部调度，不将受控 fault/授权测试外推为该范围通过。
- 诊断语义：connection_generation 是 Helper 本次 accept 的本地代次；helper_hb_seq 从每条连接首次收到 HB 开始计数；helper_sample_age_ms 包含 Helper 接收后处理/排队时间，不包含 Agent 生成前或网络到达前时间。O 线格式未变，App 的 previous_rx_age_ms/clock_origin=app_receive 只表示 App 本地接收间隔；不能用这两个本地时钟混充端到端延迟。新 read/renew/reader_ended/retired 日志区分错误码、退出原因、收尾和同代健康年龄，不输出凭据或设备身份。
- **passed（本机正常升级与身份）**：UTC `12:52:22` 完成旧会话正常退出、新鲜匹配 StopACK、Helper 退出和端口释放，安装 exit 0；Explorer 普通权限启动 App 4180，新 Helper 488 的实际安装路径与精确创建时间读回通过，Helper SHA-256 `8a396f0828a245f5f043b68cd0b89e04baee5d957694c59592c2d51a3f183890`。原驻留 host 精确代次与临时 binding 保持，六份 JSON 相对本轮实时基线逐字不变；不恢复历史九关联、不操作真实关联或模板。安装 payload、GUI、前端嵌入、更新签名与 production simulation 隔离通过，无 SYS。隔离检查首次误用 PowerShell 5 因字符串重载不可用失败，改用既有 PowerShell 7 的同一检查通过，未修改产品。
- **passed（新诊断实际读回）**：新 Helper 连接 generation 1 的 started→authorized，首 HB `helper_hb_seq=1 / sample_origin=helper_receive / lease_ok=true`，对应 O 写出 `result=passed` 与 App `clock_origin=app_receive` 已实际出现；`12:52:21.624` target generation 2 / owned mask 7167 恢复。voice gate/synth ACK 已收到，未触发或宣称实际语音输入通过。它证明新载荷和诊断生效，不证明最初 10053 原因已消除。
- 本机实装对象为 `无线麦个人版_1.0.0_x64-setup.exe`，SHA-256 `d6c51e4ffb4967e465235927d82286529eabf6af80320ddcfff68cda8f55e14e`；installed App SHA-256 `960f604ca98890c7fffba1d2a390a6aa5f18886d6c5cdf6ad09787293ed03578`。source base `e19add6034313f616505535f9b8fe98f4b299986`，fingerprint `504e23fbcda4034ea388643a071b071bfc704d51a20ecef9be550965a265005b`，local/dirty/included；包含既有未提交修改，不能称为单一 commit 纯净构建。
- **passed（本机本次用户硬件复验）**：用户明确确认新包闲置首 Menu、Back/TV、按住语音并松开，以及关闭全按键再开启后的首 Menu 均正常。被测包/指纹为上列对象，没有重装、修改映射或换 Agent 后再复用旧验收；仅记录当前 PC/本次遥控器，不外推 RC001、其他 Windows 或来源隔离。
- **deferred（完整矩阵与持续性）**：完整 RC001/RC003、跨 Windows、断连/全部调度等场景仍按矩阵验收，不继续自动轮询或重启。新 schema 存在不等于已捕获下一次 10053 原因。false HB/旧 O 保活、ACK 后首 HB 前快按以及来源隔离不随本笔关闭。旧协议不能逐边沿证明 consumed/passed，后续 true HB 不能安全追认先前 E，缓冲重放不是已成立的首按修复，详见原 Bug。

命令入口为 `cargo test --locked --manifest-path hardware/RC003/helper/Cargo.toml`、同 manifest 的 fmt/check，以及根 workspace 的 fmt/test/check 和 `cargo check -p sayall-windows-app --features runtime-simulation`。必要红/绿完整日志复用 `target/dev/menu-capture-reliability/connection-generation-*.log`，该目录历史材料保持原用途；审查与实装证据提炼后按 LOGGING 回收可重现输出，不删除本次唯一失败证据。

## 适用范围

- 仓库：本 Windows fork；官方上游为 `GetSayAll/remote-mic-app-windows`
- 平台：Windows 10 1809+ / Windows 11 x64
- 硬件 A：小米蓝牙遥控器 2 / RC001
- 硬件 B：小米蓝牙遥控器 2 Pro / RC003
- 当前状态：WinRT BLE / ATVV / PCM / WASAPI、设备名与 2A24 型号识别、端点持久化、退避重连、睡眠恢复、Raw Input、本机使用统计和 Windows 10 1809 双层版本门禁代码路径已实现；Windows、RC001、RC003、VB-CABLE 真机验收均为 deferred

## 2026-10-20 上游整合候选验证

输入为 `ea832bdf2579d1e2f99871bb979727c255abf779` 与 `upstream/main@ab9aa197e59c65b6c7b22e51a968cd0cb4312446` 的真实三方合并，保留原有未提交个人修改；源码整合检查点 `ce7bdc0a26f28120c7ceb88abc0416c43e811744` 已经独立审查并仅本地提交。以下完整验证结论属于恢复这些修改后的组合工作区，不冒充纯提交或已安装候选的证据；限定索引补验单独列出。

- passed：`pnpm test` 471 项、`pnpm build`；P1 宏取消修正后的 `cargo test --workspace`（核心 61、Windows 590/21 ignored、Tauri 174/1 ignored，其他独立测试全部通过）；`cargo fmt --all -- --check`、`cargo check --workspace`、带 `runtime-simulation` 的宿主检查；安装身份/退出静态契约 10 项。
- passed：仿真前端嵌入内容门禁；宿主普通用户实际 Tauri/WebView/IPC 20 步，包括七页导航、模板读取、宏保存/渲染/绑定引用/删除清理及模板保持、启动显示设置往返、240 采样语音和断开清理。宏步骤未执行真实快捷键；未启动提权 Helper。报告终态为 `windows-ci-simulation`，仿真进程已正常自行退出。
- passed（限定候选补验）：获准的九个 tracked 路径临时对齐 merge-only index 后，宿主身份 1、安装器退出契约 5、生命周期 27、模板 16、组合动作持久化 4，以及 Windows 宏 25、模板 6、正常退出 6、模板目录契约 1 项分别通过；测试过滤范围有交集，不累加为独立用例总数。随后九路径逐一恢复，原始字节与规范化 blob 均一致，索引、MERGE_HEAD、四个 stash 和范围外既有修改保持。此补验仍保留其余旧 dirty，尤其 Helper 捕获策略与连接页有实质修改；未执行它们的安装/真机路径，不能称完整 pure-index 全套或独立身份部署通过。
- failed：首次沙箱内启动 60 秒没有前端事件或报告；保留进程后通过正常退出事件退出，未强杀。相同二进制在宿主普通用户环境运行上述 20 步通过，不能将首次失败隐藏成一次通过。
- deferred：宏的真实 SendInput/第三方应用聚焦、按住键期间真实应用退出、RC001/RC003、冷态首用、全按键接管及实体反引号隔离。宏取消/释放/关停超时与关闭后拒绝启动由 fake stepper 覆盖；生产等待取消、启动返回后禁止后续快捷键/聚焦/位置点击、共享取消信号阻断 FocusRunner 重试/队列由单元测试覆盖，不能当作物理键释放或真实 UIA SetFocus 实证；常用语公开组件仍为空壳。

本轮不构建安装包，不替换已安装版本，不修改 Helper/桥的既有接管算法。上游在共同基线之后未改变 Helper 目录、`rc003_task.rs` 或 `rc003_bridge.rs`，两项已知故障不能因此关闭；继续按下文既有失败记录及 TODO 串行验收。

## 测试前准备

2026-10-20 当前候选实体键盘共存已 **failed**：用户报告反引号→Tab，当前Helper TV报告与应用Tab注入链已记录；上游/个人版原始Agent的同报告离线对照均不区分设备来源。语音、UI布局及接管所有权通过不关闭该回归；详情见 [当前回归](../Bugs/2026-10-03-foreground-input-regression.md#2026-10-20-上游规则候选的实体键盘回归)。

2026-10-20 起的上游规则候选另须核对 Agent build=`2026-10-20.upstream-usage` 和 Helper `[CAPTURE] policy=upstream_usage_gate system_module_binding=false`。界面“已开启”须有当前目标代次的实际 ownership ACK，不能只看计划任务启动或桥连接。分别验证目标/非目标键、Menu 模板切换、持键释放/快速连按、冷态首用及普通键盘共存（含反引号、方向键）；同宿主同格式同 usage 不再保证来源隔离，不能复用下文 10 月 12 日旧实现的隔离结论。停止须核对新鲜回执和实际进程结束；记录归 [同主题 Bug](../Bugs/2026-10-03-foreground-input-regression.md#2026-10-20-系统模块变化后恢复上游捕获规则)。

1. 记录 Commit、版本、Windows 版本和电脑蓝牙适配器。
2. 分别准备真实 RC001 和 RC003，并确认两者都可以在 Windows 蓝牙设置中完成配对。不得用其中一种型号的结果代替另一种。
3. 安装待测构建。
4. 若系统尚未安装 VB-CABLE，在 SayAll 可见安装结束提示中选择打开 VB-Audio 官方页面；下载 Pack45，解压后以管理员身份运行对应架构的 Setup，并重启 Windows。SayAll 不复制、下载或执行第三方驱动包。
5. 准备记事本和至少一个真实语音输入应用。
6. 保存测试开始前的应用日志。

## 用例零：Windows 版本门禁

1. 在低于 Windows 10 1809（build 17763）的隔离虚拟机运行 NSIS 安装器。
2. 确认安装器显示最低版本提示并退出，应用文件、快捷方式和卸载条目均未创建；WebView2 bootstrapper 是否已发生动作需单独记录。
3. 将已解压的应用 exe 直接复制到同一虚拟机并运行。
4. 在 Windows 10 1809 x64 和当前 Windows 11 x64 分别安装并启动同一候选。

预期：低于 build 17763 时，安装器在复制应用文件前以退出码 1633 失败关闭；直接运行 exe 时在创建 WebView、BLE、WASAPI 或 Raw Input 资源前显示原生中英文提示并退出。Windows 10 1809 和更高版本不触发版本拒绝，继续进入正常安装与启动流程。

失败判定：只在文档声明最低版本；低版本仍复制应用文件或启动后台平台线程；直接运行 exe 崩溃、静默退出或进入主界面；Windows 10 1809 被误拒绝；把 CI 编译或纯逻辑测试表述为已完成上述虚拟机/真机验收。

## 用例一：首次连接

1. 只配对 RC001，启动无线麦并打开“连接”。
2. 点击扫描，选择 RC001 并连接。
3. 确认界面显示“小米蓝牙遥控器 2（RC001）”。
4. 断开并移除 RC001 配对，改为只配对 RC003，重复扫描和连接。
5. 确认界面显示“小米蓝牙遥控器 2 Pro（RC003）”。
6. 对于名称只能归入白名单、不能确定型号的设备，确认连接时尝试读取标准 Model Number（2A24）；读取失败时显示“型号待设备确认”，但不因此阻断 ATVV。

预期：只显示名称在批准白名单内的 RC001/RC003 候选；零个或多个候选时不猜测。型号来自精确设备名或标准 2A24，不使用两者共用的 HID VID/PID 猜测。页面依次反映连接、特征发现、能力确认和就绪状态；只有收到有效 16 kHz ATVV 能力后才显示“BLE / ATVV 已就绪”。

失败判定：RC001 被标成 RC003，或 RC003 被标成 RC001；仅凭共用 HID VID/PID 判定型号；2A24 不可读时拒绝原本可用的 ATVV；仅进程运行或发现设备名称就显示 ATVV 就绪；把 ATVV 就绪显示成系统麦克风可用；连接失败后需要清配置或重装。

## 用例二：首次语音会话

1. 确认连接页显示“BLE / ATVV 已就绪”。
2. 按住当前型号的语音键并说一整句。
3. 观察页面进入“正在接收遥控器语音”，确认解码采样数增加。
4. 释放语音键。
5. 分别使用 RC001 和 RC003 重复步骤 1–4，两者都必须在第一次会话成功。

预期：第一次会话即完成一组 `STREAM_START → AUDIO → STREAM_STOP`；会话代次只增加一次；解码采样数增加；释放后回到 ATVV 就绪。WASAPI 完成前不要求文本框收到语音。

失败判定：第二次或第三次才开始收到采样、重复启动、释放后仍显示流式接收、旧连接的断开或通知结束了新连接。

## 用例三：极速和连续会话

1. 连续执行 20 次快速按下/释放。
2. 连续执行 20 次 2～5 秒语音。
3. 在上一段文字刚完成时立即开始下一段。

预期：每次按下和释放严格配对，没有卡键、重复音频、跨会话尾音，旧连接通知和上一代停止事件不能结束新连接或新会话。

## 用例四：断开和恢复

1. 建立 ATVV 就绪连接后，让遥控器断电或离开蓝牙范围，记录重连阶段和每次尝试间隔。
2. 保持设备不可用至少 70 秒，确认间隔按约 2、4、8、16、30、30 秒封顶，而不是忙循环。
3. 让遥控器恢复可用，确认一次成功能力协商后重连计数归零，第一次语音即可使用。
4. 点击“断开”，保持应用运行至少 35 秒，确认本次运行不再自动重连；重新选择设备后恢复自动重连。
5. ATVV 就绪时让 Windows 睡眠再唤醒，确认页面先显示睡眠释放，再重新经历连接、发现、能力确认。
6. 关闭再打开 Windows 蓝牙，重复步骤 1～3。
7. 应用退出再启动，确认自动恢复上次明确选择的遥控器及其已识别型号；若首次失败，继续按退避策略重试。
8. 分别使用 RC001 和 RC003 完整执行本用例。

预期：每次失败或睡眠都先释放通知、GATT service、device、ATVV pipeline 和活动音频；旧会话通知不会改变新会话；主动断开不会在本次运行反弹连接；恢复后第一次语音正常，不要求清配置。

失败判定：无间隔忙重试、清理失败后覆盖式创建新代次、主动断开后自动反弹、睡眠后仍显示旧会话就绪、恢复后需要第二次或第三次语音才成功。

## 用例五：音频端点

1. 不安装 VB-CABLE，启动 SayAll，确认连接页自动检测但不会伪造 CABLE Input，并显示“需要安装 VB-CABLE”、官方下载和重新检测入口。
2. 点击“打开官方下载页”，确认由系统默认浏览器打开 `https://vb-audio.com/Cable/`，没有在 WebView 内替换 SayAll 页面。
3. 从官方页面安装 Pack45，但先不重启；点击“重新检测”，分别记录端点是否出现，不把偶然出现视为无需重启。
4. 重启 Windows 后启动 SayAll，确认自动检测到唯一的 CABLE Input；若此前没有保存其他音频端点，应自动选择并显示“WASAPI 已就绪”。
5. 如果此前明确保存过其他输出端点，确认自动检测不会覆盖已有选择，再手动选择 CABLE Input。
6. 按下语音键完成一次语音会话。
7. 在 CABLE Output 侧录制或监听，确认首尾完整且没有重复。
8. 语音期间尝试切换端点，然后在空闲时切换。
9. 退出并重新启动无线麦，确认仍恢复同一个 endpoint ID 和名称，且没有改用系统默认输出。
10. 禁用或移除当前端点后重新启动，再次开始语音。
11. 让同一个 endpoint ID 的显示名称发生变化后重新启动；如果驱动无法稳定复现该状态，记录为 deferred，不得用另一个 ID 替代。

预期：缺少或未选择端点时语音失败关闭但 BLE 连接保持；语音期间禁止切换；正确端点时 16 kHz PCM 进入 CABLE Input，并可从 CABLE Output 收到；`STREAM_STOP` 后等待 WASAPI padding 清零再结束会话；重启只恢复稳定 ID 与原名称都一致的端点；端点缺失或名称变化时保留原选择用于提示，但 WASAPI 不进入就绪，也不退回系统默认输出。应用不读取或修改 Windows 默认输入、输出。

失败判定：自动使用默认扬声器、只按名称匹配导致误选、端点 ID 或名称变化后仍自动恢复、队列无限增长、停止时直接丢弃尾音、端点失败后仍显示系统语音可用。

## 用例六：可靠按键

1. 在“按键”页面点击“启动监听”。
2. 确认页面只匹配一个 RC001/RC003 共用设备族的 Raw Input 路径；零个或多个匹配时记录错误并停止，不得自动选择第一个。
3. 逐个测试 Windows 公共 API 能稳定收到的按键：按下、释放、持续按住、系统重复、快速连续。
4. 对同一实体按键核对原始事件数和语义边沿数，确认 Keyboard 与 HID 同时报告时不会产生两次语义按下。
5. 按住普通按键时点击“停止监听”，再重新启动监听。
6. 按住并释放语音键，确认普通按键最近边沿和语义边沿计数不因语音键变化。
7. 分别使用 RC001 和 RC003 完整执行步骤 1–6，单独记录两种型号的 Keyboard、HID-only 或双路径报告形态。

预期：监听使用隐藏 message-only window 和 `RIDEV_INPUTSINK`；每个事件都按启动时唯一选中的完整设备路径过滤；一次实体动作只产生一次语义按下和一次语义释放；重复 key-down 不重复触发；停止时释放仍处于按下状态的普通键；线程未能停止时页面必须显示失败，不能伪装为已关闭；普通键盘不被映射；语音键不进入普通动作路径。

失败判定：零匹配仍显示就绪、多个匹配时取第一个、其他键盘改变最近按键、一次按住产生多次语义按下、Keyboard/HID 双路径重复触发、停止后残留按下状态、语音键进入普通按键统计或动作路径。

## 用例七：安装升级

1. 安装旧候选并修改配置。
2. 安装新候选。
3. 检查安装条目、配置、映射、统计和日志。
4. 卸载应用。

预期：升级只保留一个安装条目并保留用户数据；卸载范围与界面说明一致。

## 用例八：映射保存与 SendInput

1. 在“按键”页面选择一个普通按键，并从页面内按钮网格选择快捷键预设。
2. 点击“保存映射”，退出并重新启动应用，确认映射从 `button-mappings.json` 恢复。
3. 在记事本中点击“测试当前快捷键”，分别测试单键、`Ctrl+C` / `Ctrl+V` 和 `Win+D`。
4. 测试左右修饰键、方向键、Home/Delete 等扩展键，并核对按下顺序和逆序释放。
5. 通过自动化故障注入模拟 SendInput 只提交部分事件以及调用后抛出未知错误。

预期：一个快捷键的全部按下和逆序释放通过一次 SendInput 批量调用提交；部分提交只释放可能已按下或尚未释放的键；未知交付状态逐键 best-effort 释放全部可能按下的键并报告失败；保存后无需重启即可用于显式测试。真实 RC001/RC003 边沿当前都不会自动执行映射。

失败判定：逐键分别调用 SendInput、按键释放顺序与按下相同、部分失败后修饰键卡住、无效或超过四键的映射被保存、测试失败仍显示成功、未完成真机来源确认就自动把 Raw Input 边沿注入前台应用。

## 用例九：NSIS Preview 安装包

1. 从对应 Windows CI Run 下载名称包含精确 source commit 的 `sayall-windows-unsigned-preview-*` artifact。
2. 确认 artifact 只包含一个 `*-setup.exe`、`SHA256SUMS.txt` 和 `build-metadata.json`。
3. 重新计算安装器 SHA-256，并与两份记录逐字匹配。
4. 确认 metadata 中 productName、identifier、publisher、版本和 source commit 与仓库一致，且 `signatureStatus` 为 `NotSigned`、`distributionStatus` 为 `unsigned-ci-preview-not-for-public-release`。
5. 确认 Windows CI 的 `Test silent current-user install and uninstall` 步骤通过：`/S` 安装后只有一个 HKCU 卸载条目、安装目录位于当前用户 `LOCALAPPDATA`、开始菜单只有一个入口，已安装进程持续运行 8 秒后由测试关闭。
6. 确认同一步骤使用注册表中的真实卸载命令执行 `/S` 卸载，程序目录、开始菜单和卸载条目均移除；位于 Tauri `app_config_dir` 的测试标记仍保留，证明默认静默卸载不会删除当前用户设置目录。
7. 在未安装 VB-CABLE 的隔离 Windows 测试用户中手动运行可见安装器，确认安装完成后提示 VB-Audio、Donationware、管理员权限和必须重启；选择“是”只打开官方页面，选择“否”仍能完成 SayAll 安装。
8. 安装 VB-CABLE 后重新运行 SayAll 安装器，确认检测到 `VBAudioVACMME` 服务后不再显示缺失提示；不得仅通过伪造普通用户注册表值冒充驱动安装。
9. 确认 `/S` 静默安装、升级和 CI 不弹消息框、不打开浏览器；继续检查可见语言选择、安装界面、开始菜单启动、退出、再次启动和卸载界面。

预期：CI 从干净提交构建唯一 NSIS 包；校验脚本拒绝错误产品身份、多个安装器、异常小文件、签名状态不符合当前 CI 边界或摘要不一致。静默生命周期必须证明当前用户安装、使用 Tauri updater 兼容的 `/S /UPDATE` 升级路径在等待旧卸载器退出并经过有界稳定窗口后收敛、单一卸载条目、单一开始菜单入口、进程不立即崩溃、程序文件完整卸载和设置目录默认保留。SayAll 安装器本身无需管理员权限；用户另行启动的 VB-CABLE 官方驱动安装需要 UAC。安装器不允许用旧版本覆盖新版本。

失败判定：artifact 无法绑定精确 commit、存在多个候选安装器、SHA-256 不一致、CI 包意外带未知签名、静默安装非零退出、安装到当前用户目录之外、卸载条目或开始菜单入口不唯一、应用进程立即退出、静默卸载残留程序文件或误删设置标记、安装器要求全局管理员权限、把未签名 artifact 当作公开发布包。

当前自动化状态：第 2 项生命周期矩阵此前失败在降级用例：Tauri 2.11.1 的 `/S` 静默模式不会执行负责设置版本比较结果的重装页，`/S /P` 仍属于 silent，因而内置 `allowDowngrades=false` 检查均未阻止 predecessor。现已在既有 preinstall hook 中直接读取本产品 `DisplayVersion` 并以 Tauri SemVer 比较器拒绝旧版本，静默拒绝约定退出码为 1638；Windows CI 已通过，详见下方 Run 记录。该结果仍不替代真机验收。

已完成：Windows CI Run [`33637195089`](https://github.com/GetSayAll/remote-mic-app-windows/actions/runs/33637195089) 通过全部生命周期步骤。`0.0.1` predecessor 以 `/S` 安装，`0.1.0` 以 `/S /UPDATE` 升级；随后 predecessor `/S` 被 preinstall SemVer 门禁拒绝并返回 1638，安装版本仍为 `0.1.0`，主程序、设置、映射、统计和单一安装身份均未变化；最终静默卸载移除程序、快捷方式和卸载条目并保留用户数据。该 Run 仍不替代可见安装界面、SmartScreen、Windows 10 1809 / Windows 11 真机或历史二进制兼容性验收。第 3 项 Authenticode 演练可在此矩阵通过后开始。

## 用例十：隐私安全诊断摘要

1. 依次在未连接、连接中、ATVV 就绪、语音流式、排空、断开和失败状态打开“权限”页面。
2. 点击“生成摘要”，确认页面内 JSON 的阶段、能力、代次和计数与同一时刻的连接、音频、Raw Input 和 SendInput 状态一致。
3. 搜索摘要，确认不存在真实设备 ID、蓝牙地址、完整 HID 路径、遥控器名称、音频端点 ID/名称、错误原文、窗口标题、语音内容或用户文本。
4. 点击“复制摘要”，粘贴到本地记事本，确认内容与页面内可见 JSON 逐字一致。
5. 在 980 × 700 最小窗口和 1120 × 800 默认窗口重复生成，确认权限状态、按钮、摘要和滚动均可访问，没有横向裁切。

预期：摘要由当前运行快照即时生成，不读取其他 App 或系统私有数据；敏感字段在 Rust 诊断结构生成阶段即被排除。复制失败必须显示错误，不能显示成功或静默上传。

失败判定：摘要包含任何设备或用户身份、路径、端点名称或错误原文；状态与当前快照不一致；浏览器预览被描述为 Windows 可用；未点击时自动读取或复制；剪贴板写入失败仍提示成功；页面出现横向溢出或中文小于 12pt。

## 用例十一：本机使用统计

1. 记录测试前“统计”页面今日、本周和全部的按键次数、语音次数与语音时长。
2. 启动 Raw Input，执行 10 次普通按键完整按下/释放；包含持续按住产生的系统重复，以及 Keyboard/HID 双路径同时报告的按键。
3. 按住并释放语音键 3 次，确认语音键不增加普通按键次数。
4. 完成 2 次包含真实音频、且 WASAPI 排空成功的语音会话；另制造 1 次端点失败或断连中断的语音会话。
5. 保持统计页打开，确认数据在约 1 秒内更新；依次切换今日、本周和全部，并核对最近 7 天对应本机日期。
6. 退出并重新启动应用，确认统计不丢失、不重复累计；跨本机自然日后重复一次普通按键和一次完整语音。
7. 从上一候选版本升级后再次启动，确认旧设置、按键映射和已有统计仍保留。

预期：每个去重后的普通按键语义按下只增加 1 次；系统重复、对应释放、语音键和 Keyboard/HID 双路径不重复累计。只有 `STREAM_STOP` 后 WASAPI 排空及 ATVV drain 均成功的会话增加语音次数；语音时长按该会话 16 kHz 已解码采样数折算。每日数据按本机日期保存，页面提供今日、本周、全部与最近 7 天汇总；退出前最后增量落盘，重启和升级不重置或重复。`usage_statistics` 统计字段不得包含设备 ID、蓝牙地址、HID 路径、遥控器/按键名称、端点身份、语音内容、识别文字或第三方 App 上下文；应用原有的用户明确选择设备设置按既有配置边界保存。

失败判定：一次实体按键增加多次、语音键进入普通按键统计、中断会话被记为完成、页面只显示进程内计数而重启归零、升级丢失或重复统计、日期跨天错误、统计文件含任何设备或用户内容、统计写入阻塞 BLE/Raw Input 回调而造成语音或按键响应退化。

## 用例十二：Windows Tauri/WebView 运行时仿真

1. 以 `runtime-simulation` Cargo feature 和 `VITE_SAYALL_RUNTIME_SIMULATION=1` 构建专用测试程序；普通构建不得包含仿真前端入口或仿真专用 Tauri command。
2. 在 `windows-latest` 启动该程序，并设置唯一的运行报告路径；不得向真实桌面发送 SendInput，也不得尝试扫描真实 BLE、音频或 HID 设备。
3. 由实际 Windows WebView JavaScript 依次通过 Tauri IPC 读取运行快照、首次检测并自动选择唯一仿真 CABLE Input、渲染 RC001/RC003 扫描结果、连接 RC001、启动 Raw Input、保存并显式测试 Ctrl+C 映射。
4. 依次打开驱动、按键、模板、权限、设置和连接页面；在权限页生成诊断摘要，确认平台明确标记为 `windows-ci-simulation`；在设置页确认顶部（应用标识 / 当前版本 / 检查更新）与“通用”“问题反馈”分组标题，切换“应用图标”并确认 IPC 与持久化往返，再点击“官网”“GitHub”两个入口。页面无成功提示只验证交互，IPC 成功只证明系统打开请求已提交；实际浏览器结果须在用例十四核对。被 opener capability 拒绝（`Not allowed to open url`）判失败，runner 上没有可用默认浏览器则如实记为 deferred。
5. 通过测试专用 command 驱动纯 Rust ATVV 管线完成首次 `STREAM_START → 40 + 80 AUDIO → STREAM_STOP → DRAIN`，确认得到 240 个采样、generation 为 1、连接和音频均回到 ready。
6. 停止 Raw Input 并断开，确认最终快照为 `rawInput.phase = stopped` 和 `connection.phase = disconnected`；程序写入报告并自行以成功退出码结束。

预期：真实 Windows WebView、Tauri invoke 和 Rust command 边界完成闭环；六个侧栏页面均能挂载；RC001/RC003 和 IPC camelCase 数据可被 Vue 消费；测试专用 SendInput 只记录批次和事件数，不向桌面注入；普通生产构建经字符串检查不包含仿真平台名称或仿真 command。入口存在不代表本轮已执行，结果按实际报告记录。

失败判定：只调用 Rust 单元测试或浏览器预览而没有启动 Windows Tauri WebView；普通构建能启用仿真；测试误调用真实 BLE/WASAPI/Raw Input/SendInput；WebView 进程存活但没有完成报告；把仿真 240 个采样、就绪状态或页面渲染表述为 RC001/RC003 真机通过。

## 用例十三：NSIS 安装生命周期矩阵

1. 使用配置覆盖构建版本低于当前版本的同源码 NSIS predecessor fixture，将其移出当前 bundle 目录，再按普通生产配置构建当前 NSIS。
2. 静默安装 predecessor，确认只有一个 HKCU 卸载条目和一个开始菜单入口，安装位置位于当前用户目录。
3. 在本应用配置目录写入有效的 `settings.json`、`button-mappings.json` 和保留标记；设置夹具包含遥控器选择、音频端点、增益和每日使用统计，并记录两个 JSON 的 SHA-256。
4. 静默安装当前版本，确认版本升级、安装位置不变、卸载条目和快捷方式仍各一个，两个 JSON 的 SHA-256 逐字节不变；启动升级后程序并确认持续运行 8 秒。
5. 以 `/S` 再次运行 predecessor 安装器，确认退出码为 1638，且当前版本、当前主程序 SHA-256、设置、映射、统计和单一安装身份均未被替换。
6. 通过当前卸载条目的真实静默命令卸载，确认程序目录、快捷方式和卸载条目移除，而设置、映射和保留标记继续存在。

预期：同一产品身份只存在一个当前用户安装；覆盖升级不改变本应用用户数据；较低版本安装器在复制文件前由 preinstall SemVer 门禁以退出码 1638 拒绝，不能把当前版本降级；最终卸载边界与现有约定一致。

失败判定：使用当前版本直接覆盖当前版本冒充升级；升级后出现两个安装条目或快捷方式；设置、映射或统计被重写；降级后版本或主程序发生变化；卸载残留程序身份或误删用户数据；把同源码 predecessor fixture 表述为真实历史版本兼容性验收。

## 用例十四：设置页结构、外部入口与应用图标

1. 打开侧栏最后一项，确认名称是“设置”、图标是齿轮，页面标题为“设置”；顶部一行为应用图标 + “无线麦 SayAll” + “让语音触手可及”，其右为“当前版本”与版本号（应与当前实际安装包版本一致）、“检查更新…”按钮、“检查预览版更新”开关，再往下是分隔线与更新状态。
2. 确认页面里没有说明性小字：按钮旁不再有“默认关闭。预览版包含新功能……”、“外观”下不再有“该选择会在重启后保持。”或“跟随 Windows 的应用颜色模式。”。
3. 确认随后是“通用”（外观 / 启动行为 / 应用图标）与“问题反馈”（官网 / GitHub）两个分组，且诊断摘要与日志目录仍只在“权限”页。
4. 点击“官网”，确认系统默认浏览器打开 `https://sayall.app/?from=win`（地址必须带 `?from=win`）；返回应用后页面**不显示**任何成功提示。
5. 点击“GitHub”，确认系统默认浏览器打开 `https://github.com/GetSayAll/remote-mic-app-windows`，同样不显示成功提示。
6. 通过测试替身覆盖打开失败，确认页面就地显示原因而不是静默无反应；自然出现无默认浏览器等真实失败时另记结果，不为本用例修改系统关联或安全策略。错误若为 `Not allowed to open url`，说明 opener capability 白名单与 `bridge.ts` 的常量不一致，属配置回归而非环境限制。
7. 在“应用图标”里选“几何鸭”，确认三处同时换成几何鸭：本页顶部标识、窗口与任务栏（含 Alt-Tab）图标、通知区域（托盘）图标；切回“默认”确认三处一起还原。日志应出现 `app_icon action=apply ... applied=faceted-duck` 与 `target=window` / `target=tray` 两条结果。
8. 重启应用，确认应用图标与外观、启动行为一样被保持。

预期（2026-10-02 吸收的上游设置页契约，本机结果另记）：顶部模块集中应用标识、版本与检查更新；“通用”“问题反馈”分组卡片与 Mac 新设置页同构，Windows 没有的模块不出现；外部入口只打开 capabilities 白名单内的固定 URL，成功不打扰、失败给原因；应用图标切换对齐 Mac `AppIconController`（稳定语义 ID、未知 ID 回落默认、切换落结构化日志），窗口/任务栏与托盘一起换，安装包与快捷方式图标不随切换变化。

失败判定：WebView 自身跳转，或入口可打开白名单外的任意 URL；点击失败但页面无任何提示；正常安装上出现 `Not allowed to open url`；官网地址缺少 `?from=win`；成功后又出现“已在系统默认浏览器打开…”这类提示；诊断摘要入口重回设置页；侧栏底部仍显示“预览版”或版本号错误；新装没有默认选中“几何鸭”；旧配置升级后仍停在水彩鸭（迁移没有生效）；安装目录里的 exe 或安装包图标不是几何鸭（构建图标回归）；切换应用图标后窗口/任务栏、托盘或快捷方式有一处没换、失败时单选按钮停在未保存的位置、或出现 Mac 菜单栏的单色状态图标（Windows 没有该图标）；把浏览器预览或仿真结果表述为 Windows 真机通过。

## 用例十五：语音增益

1. 打开“连接”，确认“语音设备”卡片里 CABLE 端点下方有“增益”滑块与读数（初始为 `0 dB`），以及说明“0 dB 保持原始音量；数值越大声音越响，也会放大环境噪声。建议先从 6–12 dB 开始。”。
2. 选一种判定口径（按实际使用方式，2026-10-04 Andy 口径）：遥控器语音通常**直接进输入法、操作人听不到声音**，因此默认用 ① **转写文字**——同一句话分别在 0 dB 与 24 dB 下各说一次，比较识别结果并记录；若音频可监听（例如系统直接回放 CABLE Output）才用 ② 听感；需要客观数字时用 ③ 回录电平（`crates/sayall-windows/examples/cable_level_probe.rs` 逐秒输出 CABLE Output 的 RMS/峰值，两段的能量 RMS 比值应显著大于 1；24 dB 理论 ×15.85，若削波会小于理论值但必须明显 >1）。
3. 把滑块拖到 12 dB，确认松手后读数变为 `12 dB`；日志出现 `audio_settings feature=gain action=save phase=completed terminal_result=passed gain_db=12`。
4. 再按一次语音键，用同一音量说同一句话。日志应出现 `audio_gain action=apply phase=completed terminal_result=passed generation=<n> gain_db=12`（每会话一条；会话内再拖滑块会按新值补一条）——这是“该轮会话生效增益”的直接证据，不依赖时间反推；配合步骤 2 的三种判据之一给出结论。
5. 把滑块拖到 0 dB 复测一次；再拖到 24 dB 确认应用不报错（若音频可监听，此时应明显更响/可能削波）。
6. 完全退出并重新打开无线麦，确认读数仍是上一次的值；日志出现 `action=restore ... gain_db=<值>`。
7. 在语音会话进行中拖动滑块（松手生效），确认下一句语音按新增益处理，且会话不中断、按键按下/释放仍成对。
8. 分别用 RC001 与 RC003 执行步骤 2–4 与步骤 7，单独记录两种型号结果（RC003 已于 2026-10-04 完成，见“当前自动化记录”；RC001 仍 deferred）。

预期：滑块拖动过程只更新读数，松手才落盘；生效值以保存返回值为准（越界被钳制到 0–24 dB）；增益在解码后、写入 CABLE 播放端前生效，改动下一批音频立即生效；重启后恢复；会话生命周期、断连与睡眠恢复不受影响。

失败判定：拖动过程中每个刻度都写一次设置文件；保存失败仍显示新值或静默失败；重启后回到 0 dB；增益未生效（回录电平两段比值 ≈1×、或转写结果与 0 dB 无差别且无其他解释）或整段音频被错误静音/清零；只验证了编译、单测或仿真就宣称真机通过。

## 用例十六：启动时自动打开主面板

1. 全新安装（或先退出应用并清空 `%APPDATA%\app.getsayall.remote-mic.windows`）后启动：主面板应自动打开；进入“设置 → 通用 → 启动行为”，确认“登录时自动启动”与“启动时自动打开主面板”**并排**（文字在上、开关在下，中间一条竖分隔线），第二项默认**开启**。
2. 关闭该开关，确认开关保持在关闭状态、页面无错误提示；日志出现 `startup feature=open_window_at_launch action=save phase=completed terminal_result=passed enabled=false`。
3. 从托盘菜单“退出”，再双击快捷方式启动：**不应**出现主窗口；托盘图标存在。日志出现 `startup feature=main_window_visibility action=decide terminal_result=passed visible=false reason=open_window_at_launch_disabled ...`。
4. 单击托盘图标，主面板应打开；再关闭窗口（收进托盘），用托盘菜单“显示主界面”打开一次；再关闭窗口后**再次双击快捷方式**（第二实例）也应打开主面板。三条路径都不受该设置影响。
5. 重新打开该开关，确认保存成功；退出后再次启动，主面板应自动打开。日志 `terminal_result=passed visible=true reason=open_window_at_launch`。
6. 在“设置 → 通用”里点“重新运行设置向导”，随后退出并重新启动应用：即使开关处于关闭状态，主面板也必须显示（否则无法完成向导）。日志 `reason=onboarding_active`。
7. 老配置升级：在已安装旧版本（`settings.json` 无 `open_window_at_launch` 字段）的机器上覆盖安装本版本，首次启动主面板应自动打开，设置页该开关显示开启。
8. 更新完成路径（开关保持关闭）：用上一版本的安装包启动过一次，再用本版本覆盖安装并启动——主面板**必须**显示，且弹出“无线麦已更新 / 已成功更新到版本 X。/ 好”一次；点“好”关闭后刷新或重开主面板不再弹。日志依次出现 `startup feature=completed_update action=decide ... completed_update=true`、`reason=completed_update`、`startup feature=update_completed action=take terminal_result=passed available=true`。
9. 同版本重启（第 8 步之后直接退出再启动）：**不应**再弹更新提示，主面板是否显示回到开关判定（开关关闭则只驻留托盘）；日志 `completed_update=false`、`update_completed action=take ... available=false`。
10. 降级：安装更低版本并启动，同样**不应**弹更新提示。
11. 全新安装（无 `launch-state.json` 且无安装证据）首次启动：**不应**弹更新提示（不能把新装误报成“已更新”）。
12. 若开启了“登录时自动启动”，注销并重新登录（`Win+L` 锁屏不算）后按步骤 3 的判据检查一次。

预期：新装与老配置默认开启；关闭后**普通**启动路径（含手动双击）都只驻留托盘；托盘左键、托盘菜单“显示主界面”和再次启动应用始终能打开主面板；未完成首次设置时无论开关如何都显示主面板；更新后的第一次启动强制显示主面板并提示一次，同版本重启、降级与新装都不提示。

失败判定：关闭开关后普通启动仍弹出主窗口；关闭后完全找不到托盘图标或托盘无法打开主面板；未完成向导时启动不显示窗口；保存失败却显示新状态；老配置升级后默认关闭；更新后首启不显示主面板或不提示；同版本重启/降级/新装误弹“已更新”；更新提示重复弹或点“好”后不消失；只验证了单元测试或仿真就宣称真机通过。

## 日志收集

日志不得包含语音内容、识别文字、真实蓝牙地址、完整 HID 路径、窗口标题或个人文档路径。报告应包含 Commit、版本、时间段、Windows 版本和失败步骤。

## 验证边界

- Mac 自动化：前端构建、Rust 核心测试、格式化和静态检查。
- Windows CI：Windows 编译、自动化测试、测试专用 Tauri/WebView/IPC 仿真、当前用户静默安装/启动存活/静默卸载和设置目录保留边界；不包含可见安装界面、SmartScreen、Windows 10 1809、真实硬件或第三方应用验收。
- 用户/维护者：RC001 与 RC003 分别的型号识别、蓝牙、音频、Raw Input、输入法、睡眠和安装升级真机验收。

## 当前自动化记录

### 2026-10-09 本 fork 合并候选

源码身份：本地 `50f21e005b4f1bf3455268eb013d58259196d6a2` 合并上游 `e225bb4c9791dbfefa37c8fb197bc06e95fca428`（0.8.5），本段随合并提交保存；原有个人版与返回键诊断工作在合并提交之后恢复，不包含在下列基础合并结果中。

- **passed（Windows x64 自动化）**：`cargo test --workspace`：core 33、场景回放 4、Windows 平台 473、平台集成 3、宿主 152 项，21 项硬件/交互测试 ignored。`cargo fmt --all -- --check`、`cargo check --workspace`、宿主 `--features runtime-simulation` 编译通过。
- **passed（Helper 与安装契约）**：Helper release 单元测试 56 passed / 3 ignored；Agent 78、清理 69、受限回收 8、安装覆盖宏/正常退出契约 5 项通过。前端完整 353 项测试及生产构建通过。
- **回归证据**：焦点取消后的排队请求重开、退出后新请求进入两个用例均先 failed，增加代次/退出门后焦点套件 43 passed；按键宿主测试覆盖 DOWN 超时放行后迟到决定不能吞 UP，以及退出/重启跨边沿配对。保留模板实际执行、音频同线配对和失败回滚测试。
- **deferred**：本合并候选的 RC001/RC003 实体按键与物理键盘共存、冷态首用/连续语音/默认麦克风恢复、真实应用焦点、断连/睡眠/持键退出、ARM64 主机及完整安装/卸载矩阵、WebView 真实崩溃恢复。未把单元测试、编译或上游历史实验当作这些场景通过。
- 正常执行前曾出现 sandbox 工具权限/loopback 限制；改在当前 Windows 主机执行后通过。旧测试 PID 占用导致 Helper debug 链接失败，最终 release 测试通过；用户仅授权结束该测试进程后已核其退出，正式应用未被强杀。
- 临时日志复用 `target/dev/upstream-sync/` 和 Helper 核对所用 `target/dev/upstream-merge/`；留到候选验证完成或被替代后回收，不复制源码、缓存或安装包留档。

### 同日恢复原有未提交改动后的验证

本地合并提交为 `9f4ead50bcd429ef9bfe448cec54aeede936bf7f`，父提交分别为原工作基线 `50f21e0` 和上游 `e225bb4`。恢复合并前的个人版品牌与 RC003 返回键诊断补丁后，继续保持未提交；新增向导也沿用个人版显示，固定图标不恢复旧图标选择。原文件存在/删除状态全部保持，个人 SVG 母版 SHA256 仍为 `40cc8382d2e16ef060abac300d5f3fb9bfaf48ece841939d2eeac3151d9d238e`。

最终 **passed**：前端 348 项、类型检查与 Vite 构建；Rust 工作区 660 项（core 32、场景 4、Windows 478、集成 3、宿主 143），21 ignored；fmt、普通与 runtime-simulation 编译检查。测试数变化来自个人版移除已退役图标/反馈流程及恢复诊断回归，不是跳过失败测试。映射诊断与桥接定向 40/25 项通过；真实合成快捷方式的受限回收及 5 项安装脚本契约通过。日志为 `target/dev/upstream-sync/*restored*.log` 与 `target/dev/upstream-merge/*restored*.log`，按本节留存条件收尾。未运行真实安装器、未改变现有安装实例，前述实体按键、真实收音、冷态/睡眠、ARM64 等边界仍 **deferred**。

安装器当前钩子的安装/卸载段汇编为 **deferred**：复用已有 `target/.tauri/NSIS/makensis.exe`，第一次受 PowerShell 原生命令启动器输出编码错误阻止；第二次显式 stdout/stderr 重定向仍报系统 DLL relocation，编译器未启动，未生成或执行安装包。钩子 SHA256 `2e2c4d2c11cc083d4e88975aeb9c15b245d4d45c7f1db724d44e3bd0712af22c`；摘要 `target/dev/upstream-sync/installer-hooks-makensis-summary.log`，本次 4 份探针文件已回收。此环境失败不算 NSIS 语法或生命周期通过，也不影响已通过的 Rust/前端验证；真实出包前须补此项。

### 同日按用户要求覆盖安装并打开（11:28）

用户明确要求“安装并打开”后，以 `9f4ead5` 加已恢复的未提交个人版/诊断工作构建本地 x64 生产包；正常 Windows 构建环境中 `build-local-input-candidate.ps1` 完成 NSIS 出包，替代前述未启动编译器的 deferred 汇编结果，不据此推断那次隔离工具失败的具体责任。

- **passed**：`target/release/bundle/nsis/无线麦_0.8.5_x64-setup.exe`，SHA256 `55d3cf83c192cbaad9cdf106923bbe936e0068da7cf6436adba37c7adb177352`；GUI/版本/许可证与四项载荷校验通过，10 个前端资源均实际嵌入。
- **passed（实际安装）**：原 PID 3976 通过正常退出事件结束；管理员覆盖既有安装目录，注册版本读回 0.8.5，原 JSON 配置在安装前后哈希保持，四项资源逐件 SHA 对齐。主程序与构建产物的差异严格仅为 `__TAURI_BUNDLE_TYPE_VAR_UNK` → `NSS` 的三个字节，其余字节完全一致；已安装 EXE SHA256 `707880d02f6dc40edfe7f4030e3fe73af5c93de7f72b1514d28fe50fc1312f7c`。
- **passed（实际启动）**：由 Explorer 普通令牌启动 PID 10168，TokenElevation=false；一个主窗口及一个明确 `--sayall-key-host` 子进程，主窗口“无线麦 · 个人版”可见、Responding=true、三项导航标签实际存在。初始 IPC 和按键页监听/轮询均有完成日志，短时观察无该进程的 WebView process_failed。
- 初次临时验收脚本错误要求 EXE 字节完全一致，并把同名按键宿主算作第二主实例；逐项只读核对后按已有 Tauri 标记和新进程职责完成验证，没有重复安装或强杀。既有未提交源码继续保持未提交；未推送、发布或新增 worktree。
- 2026-10-10 交接核对：本机 `build-metadata.json` 的 `sourceCommit` 为 `null`，没有形成单一精确提交的源码身份。上述候选为 `9f4ead5` 加当时未提交工作，不能用版本号、HEAD 或空元数据声称可完整重建。下一轮先核对实际载荷及工作区；沿用本页包/EXE 指纹区分现场，构建时按 LOGGING 核对来源字段，不新增整仓库快照。
- 原始摘要为 `target/dev/upstream-sync/` 中的 `local-install-build.log`、`local-install-bundle-check.log`、`local-install-embedded-check.log`、`local-install-result.json` 与 `local-startup-result.json`，留到候选被替代或验收闭合后回收。ARM64 不在本次包中；实体遥控按键、实际收音及完整卸载矩阵仍 deferred，不把可见启动外推这些功能。

### 2026-10-10 三轮启动调查后合并 0.8.6、本地安装与启动

先保护当前及轮转日志，再按本次开机时间核对三轮主程序、Helper、按键宿主、配置与捕获代次。事实与未知点原位归 [前台输入 Bug](../Bugs/2026-10-03-foreground-input-regression.md#2026-10-10-用户报告同次开机的三轮启动部分定位未闭合)；第三轮正常观察不算故障修复。设备宿主始终为同一个 11972，没有因本轮操作被强杀、重启或更换；未操作无线电。

最新上游仍为 `3ec66566ceabf82915dd1b9ff9a3b10ec163ca26`（0.8.6）。本地必要合并提交 `af710b757897cb0567b63c63a7ceb40f62426652` 保留双方父历史，按 [ATTRIBUTION](../ATTRIBUTION.md#2026-10-10-上游-086-评估与合并) 适配焦点恢复、迟到回调与隐私日志。原有 80 项文件存在状态及 52 个已修改文件内容与本轮保护 stash 核对一致（文本只允许换行差异，个人品牌配置仅另改版本号）；全部保持未提交/未暂存，旧恢复 stash 未再次 apply。本轮新增 reset/实际映射与连发诊断也保持未提交，不包含根因未确认的按键修复。

- **passed（自动化）**：`cargo test --workspace` 669 passed（core 32、场景 4、Windows 487、集成 3、宿主 143），22 ignored；不重复计入宿主测试产生的隔离子进程一次执行。`cargo fmt --all -- --check`、`cargo check --workspace`、`cargo check -p sayall-windows-app --features runtime-simulation`；`pnpm test` 348 passed，`pnpm build` 类型检查和生产构建；安装覆盖契约 5 项通过。正常 Windows 环境执行，sandbox 下 IPC 失败/挂起的中间测试未作为产品失败或通过证据。
- **passed（本地生产包）**：既定 `scripts/build-local-input-candidate.ps1` 构建 x64 NSIS，包 `target/release/bundle/nsis/无线麦_0.8.6_x64-setup.exe`，SHA256 `e4d20971632f5bbdaa3b4a1ee43da8bdc81498f477466d182802abcad8753873`；版本/许可证/GUI、四项载荷与十项实际嵌入的前端资源通过。`build_channel=local`，内嵌 revision 为合并提交，但包包含原有未提交内容和本轮诊断；`build-metadata.json` 仍因本地没有 GITHUB_SHA 而 sourceCommit=null，不能把内嵌 HEAD 当作完整源码身份。
- **passed（正常退出及实际覆盖安装）**：原主程序 23708 在 12:10:16 请求正常退出，日志 overall passed、failed_stages=0、728ms；进程观察 913ms 后确认主程序/按键宿主已结束，原 Helper 46672 随后单独只读确认已退出。管理员仅覆盖原安装目录一次，版本读回 0.8.6；settings、button-mappings、capture-input-session、onboarding 四份 JSON 安装前后哈希保持。实际四项载荷逐件 SHA 对齐；主 EXE 与构建产物逐字节核对，仅允许既有 NSIS bundle 标记差异。安装 EXE SHA256 `307a5ebe5720a2b6154e254df91133d3830daf968420331a36ee50b8778147f3`。
- **passed（普通权限启动与诊断副作用）**：首次普通候选 18960 的父进程不能确认，先通过正常退出结束，日志 799ms、failed_stages=0，Helper 39648 的 reset 从 helper_count=1/receipt_settled=false 到 0/true/complete 有实际记录。没有重复安装；改用已有真实 Explorer 窗口的自动化入口于 12:13:44 启动主程序 32448、按键宿主 49576，父进程为 Explorer，双方 TokenElevation=false，个人版窗口实际可见且响应，配置仍保持。启动 reset 记录完整匹配判据并 complete；映射 revision=3 的 execution_mask=0、recognition_mask=256、repeat_single/long_mask=0 与当前配置一致，捕获所有权也确认 256。12:14:17 日志记录正常 hide_to_tray，后续只读 UIA 仅见事件窗口，不能把这个查询扩大解释为导航页面检查通过。
- **passed（同日 13:45 按用户要求核对最新版并退出）**：再次核对当前主程序 32448/按键宿主 49576 来自既有安装目录，版本 0.8.6；安装 EXE、包指纹及四项实际载荷与上述交付一致，没有比 EXE 构建更新的功能源码，复用已完成的打包/安装，不重复构建、恢复或安装。13:45:37 请求应用正常退出，主程序日志 overall passed、failed_stages=0、799ms，增强释放 passed、592ms；984ms 后主程序、按键宿主与 Helper 20032 均已退出，无强杀，未重新打开。安装版保留为当前候选，下次启动仍使用该 0.8.6。最小当前会话日志复用 `target/dev/foreground-input/current-086-session.log`，留到对应未闭合异常完成或候选被替代后回收；不扩大按键/收音验收结论。
- **deferred**：实体键盘实际键位/扫描码及异常复现、第二轮确切模板/前台程序、在同一实际模板下显式关闭/开启连发的实体对照、输入法真实首按/收音和焦点恢复、冷态/闲置后首用、断连/睡眠/持键退出、分别 RC001 与 RC003 的硬件回归、ARM64、完整升级/卸载矩阵。未运行新候选的完整 GUI runtime simulation；其编译及现有隔离 IPC/生命周期测试不替代硬件验收。

安装验证脚本最初因现有载荷校验脚本的默认 StagedDirectory 求值为空而中止；显式传入路径只补校验，未再次安装。临时启动脚本用本地 Shell.Application 的首次普通启动未确认 Explorer 父进程，按上述正常退出后改真实 Explorer 自动化；不把中间校验失败改写成产品故障。当前只创建必要合并提交，未自动提交新增诊断、未推送、PR、合入 main 或发布。

### 2026-10-10 晚共享宿主来源修复、本地安装与旧代次拒绝

**2026-10-11 当日现场追验 failed**：同一 App 50156 / host 11972 仍受旧脚本版本阻断，四次 Menu 成对观察均未启动模板菜单；选项重新勾选保存成功，但捕获所有权为零。实际安装主程序、Helper 和包哈希再次对齐下列身份，未重新构建/安装。根因与恢复限制归 [既有 Bug](../Bugs/2026-10-03-foreground-input-regression.md#2026-10-11-菜单已勾选但触发原生上下文菜单当日现场未恢复)。当日新版实际接管、菜单和键盘共存为 deferred，未代用户重启 Windows；12 日续验见下文，不把安装成功或此前失败混作当前功能结论。

根因及实际键码归 [同主题 Bug](../Bugs/2026-10-03-foreground-input-regression.md#2026-10-10-晚方向键长按与实体键盘共存来源调查中)：20:17 仅实体键盘的测试实际进入 TV/方向映射，注入 Tab/单次方向；组合时公开观察收到实际 C0/scan29 和原生重复。当前源码新增逐 WDF 请求功能 PDO 核验，其他设备/未知来源不改写、不上报，保留配置、按键配对、代次与正常退出契约。未重复迁移、恢复 stash 或改用户模板/连发；固定来源参考与模块/架构范围归 ATTRIBUTION。

- **passed（自动化）**：Agent 95/95、独立 Helper 58 passed / 3 ignored，新增来源与 PDO/私有续约测试先红后绿；Helper fmt/check；工作区 fmt、669 passed / 22 ignored、check；Windows runtime-simulation 编译；前端 348 passed、类型检查和生产构建；覆盖安装契约 5 passed。Frida Python 传输台尚未执行（当前解释器没有 frida），语法检查仅为语法证据。未把单元测试或仿真编译扩大为实体捕获通过。
- **passed（最终包与源码对应）**：沿用 `scripts/build-local-input-candidate.ps1`，最终 x64 NSIS `target/release/bundle/nsis/无线麦_0.8.6_x64-setup.exe` SHA256 `04ba26c64bfcccaff065c00aedf3746672c29e188fbe827c73c87dd7de2bbaf2`；四项载荷、生产隔离、GUI 与实际前端资源通过包检查。Helper SHA256 `f9ede074f96b56c99e2468ca7137af6c45168c557934d6f4186fe7971ef9cce2`；二进制包含最终 Agent 源文件的完整字节（SHA256 `a78d135ad500689ef8960ddca009655839777ae510ade016dae9ff337bc73bfd`），代次 `2026-10-10.request-source`。仍为合并 `af710b757897cb0567b63c63a7ceb40f62426652` 加原有未提交工作及本轮修复；sourceCommit=null，最终运行 build_channel=unknown，不用版本号/HEAD 代替完整内容身份。本次修复未额外提交。
- **passed（旧应用正常退出）**：20:20:46 请求自身退出；旧主程序 38828 与按键宿主 50032 自行结束，Helper 2600 的停止回执 `hook_detached=true / result=passed / module_unloaded=false`，主程序 overall passed、failed_stages=0、554ms。安装前只读确认所有正式应用/Helper 实例均已结束；未强杀宿主或操作无线电。
- **passed（覆盖原目录）**：管理员覆盖原安装目录一次，版本读回 0.8.6。settings、button-mappings、capture-input-session、onboarding 四份 JSON 安装前后逐件哈希一致。四项实际载荷与暂存集合完全对齐，主 EXE 与生产构建仅有既有三个 NSIS 标记字节差异；安装 EXE SHA256 `730c804d38859f321238deed95b69c52e2c3dde8f802f435559a67182bc566ae`。
- **passed（实际启动与拒绝旧脚本）**：20:28 从真实 Explorer 窗口普通用户启动主程序 50156（父 Explorer 12712）和子按键宿主 36976；双方 TokenElevation=0，个人版主窗可见且 Responding=true。新 Helper 46912 记录共享宿主 7/1、固定来源模块核验通过，但实际宿主 11972 仍为 `2026-10-03.cleanup-instance`。新 Helper 明确拒绝，renews_sent=0、capture_acquired=false、terminal=agent_version_blocked 并退出；旧组件已正常撤钩而未卸载，禁止把继续驻留当作新源码生效。没有并起第二套捕获或强杀/重启宿主。
- **deferred（10 月 10 日安装时的新链路真机）**：当时 Windows 宿主生命周期未结束，尚未装载新 Agent；新来源接管与键盘共存没有实际证据，不能把旧捕获撤除/版本拒绝外推为全功能修复。12 日的新代次结果见下文。三轮启动及 17:56 单击退格失效仍按原记录保留未知；此固定来源适配的其他 Windows 模块组合及 ARM64 也未验证，源码为明确拒绝。

**2026-10-12 重启后续验**：用户已自行重启，Windows 启动时间 10:06:55.5（UTC+08:00）。沿用上述安装内容，本次主程序 10780 / 按键宿主 26108 / Helper 29896 / 设备宿主 12192；10:12:01.692 实际鉴权脚本为 `2026-10-10.request-source`，Menu 所有权及后续模板 generation 2→3→4 依次生效，迟到 generation 3 被拒绝。10:12:07–09 完成 Menu DOWN/UP、菜单聚焦读回、真实确认 UP 后选择模板、原窗口恢复/菜单隐藏，最后 `actions_available=true`；用户明确确认“菜单和实体键盘都已正常”，本次 RC003 菜单与当前 PC 键盘观察 **passed**。来源分类、报告清除/恢复及配对边沿均已有实际计数；完整时间线与边界归 [续验 Bug](../Bugs/2026-10-03-foreground-input-regression.md#2026-10-12-用户重启后的来源菜单与连发配置续验)。

当前 Agent 模板返回及四方向没有 `holdRepeat`，实际两个 repeat mask 为 0；847ms/1016ms/1415ms 的方向/返回持有各只触发一次并正常释放，符合关闭配置。用户讨论应保留选择权，本轮未改模板；开启后的连续输出、UP 停止、冷/闲置首用及完整双型号矩阵仍 **deferred**。该阶段另见保存音频端点不可用及设备 Code 10，当时语音原因调查中；随后通道与反馈修复、安装及实测见下节。冻结材料复用 `target/dev/foreground-input/post-reboot-current.log`，按本次 PID 取证；该只读续验阶段未重复构建、迁移或安装，未提交、推送或发布。

10 月 10 日 20:30:03–20:32:03 的安装后公开键盘观察已正常完成撤钩，期间没有目标键事件，当时尚未取得用户单键/长按恢复反馈；因此该窗口不记为键盘实际恢复 passed。安装前观察有事件的材料单独保留为 `keyboard-source-before-fix.log`，安装后空观察为复用的 `keyboard-readonly-probe.log`，不混用两个窗口；10 月 12 日用户确认另见上述续验。

最小过程输出复用 `target/dev/foreground-input/`：前后键盘只读观察、必要用户动作时段、退出/启动日志、安装前配置哈希与包检查；原件及轮转日志未动。旧 `e4d20971` 包仅保留一个准确对照，错误临时副本与空格式输出已回收，不保存源码副本或另一套台账；新来源验收闭合后回收不再需要的原始现场和对照包。长期身份、脱敏计数与边界以上述记录为准。

最小现场复用 `target/dev/foreground-input/`，本次开机过滤日志、第三轮部署前日志和只读配置仅用于未闭合异常；不含本次开机内容的两份大历史副本已移入回收站，原始应用日志未清理。合并保护清单、最终 combined 测试/构建日志、bundle 校验、`boot-merge-install.json` 与安装载荷校验复用 `target/dev/upstream-sync/`，留到相应候选被替代且未知点闭合后回收；不用源码副本、worktree 或重复长期台账代替 Git 与本节结论。

### 2026-10-12 音频通道识别与语音键反馈修复

本候选仍基于合并 SHA `af710b757897cb0567b63c63a7ceb40f62426652` 加保留的未提交工作；新增识别修复、独立语音按键观察与页面反馈也未提交。版本 0.8.6，`build_channel=local`，包元数据 `sourceCommit=null`，不能仅以 HEAD 或版本号表示完整源码身份。

- 安装包：`target/release/bundle/nsis/无线麦_0.8.6_x64-setup.exe`，12,848,921 字节，SHA256 `a3ec31552954347c6b3dbc3b4df426f92ea50fdacc2ccf9a5fcdc9d9dac94b4c`，未签名本地候选，不用于公开发布。
- 安装主程序 SHA256 `10f62ab982b9e962d22ce5a98029807f6d92df4180a86505fc4843d8c69cb297`；与构建主程序等长，仅差 3 个预期 NSIS 标记字节。Helper 仍为 `f9ede074f96b56c99e2468ca7137af6c45168c557934d6f4186fe7971ef9cce2`；四项实际载荷/许可与 staging 一致。
- **passed（自动化）**：新端点识别回归先红后绿；语音观察四项后端回归、前端新增反馈及错误优先回归先红后绿，最终定向 78 项；完整 `pnpm test` 355 项/28 文件，生产构建中的 `pnpm build`、`cargo fmt --all -- --check`、`cargo test --workspace`、`cargo check --workspace` 与 `cargo check -p sayall-windows-app --features runtime-simulation` 全部通过。Windows 库 493 passed / 22 ignored，宿主 143 passed / 1 ignored；不把 ignored 硬件测试或仿真编译当作真实设备验收。
- **passed（正常退出/覆盖/普通启动）**：11:13:22.950–23.635（UTC+08:00）原主程序 10780 与 Helper 29896 正常结束，增强释放、BLE 与输入路由收尾通过，overall 684ms、failed_stages=0。11:14 覆盖安装完成，设置、按键映射、向导、恢复 journal 四文件在安装阶段字节不变。用户明确选择后才更新两端 ID/name 四字段；其他全部设置语义保持、切麦克风仍 enabled=true，模板和向导未变。配置替换第一次因 PowerShell 空字符串参数失败，核实原文件与备份一致后完成待替换文件并读回，未重复安装。11:16:18.412 新主程序 30736 由 Explorer 12848 启动，两者 TokenElevation=0；按键宿主 43264、Helper 42984，原设备宿主 12192 中同版 `2026-10-10.request-source` 再武装和鉴权通过，无强杀/重启/无线电操作。
- **passed（启动路由及 RC003 本机实际观察）**：新选择的 Capture/16Ch 精确同 PnP 适配器，11:16:19.539 startup passed，前端报告 paired。11:19:00–50 四次真实会话分别提交 23,040 / 41,280 / 35,760 / 40,560 samples，均 queued_samples=0 完成；四次先确认默认麦克风角色，再进入语音链；每次结束均恢复 original_mask=7，journal=null。按下/松开观察与前端日志各四组配对；实际自身 UIA 读回收到按键与结束提示，用户明确确认“按键反馈和语音输入都正常”。首轮为启动约 161 秒后的第一次语音，不扩大为所有工具的长闲置冷态通过。
- **仍 failed / deferred**：原保存声卡 Code 10 的底层原因未知；当前另一写入端不能证明为标准 pin，标准自动配对实机仍 RenderMissing，显式 16Ch 才 passed。两端 UI 选择循环及旧失效目标的回滚边界仍归输入切换专项；新包错误通道的受控 UI 实机、RC001、ARM64、断连/睡眠、各输入工具冷态和显式连发开启矩阵未覆盖。模板未改、连发未自动开启。

根因、事件时序与回归边界归 [同主题 Bug](../Bugs/2026-10-03-foreground-input-regression.md#同次重启后的语音端点失效与识别修复)，输入切换归 [专项](CaptureInputSession.md)。最小材料复用 `target/dev/foreground-input/` 的 `audio-feedback-lifecycle.log`、`audio-fix-*`/`audio-feedback-*` 检查、`voice-feedback-*` 回归与包校验；已知菜单/键盘通过的 `source-confirmed-04ba26c6-setup.exe` 仅留作当前差异回退对照。剩余边界关闭且候选被替代后按 LOGGING 回收；不新增源码副本、台账、提交、推送或发布。

收尾已核对引用、用途和占用，将本次不再需要的局部截图、工具启动失败短输出、一次性配置脚本及已被验证的新配置替代的私有旧配置备份移入回收站。先红后绿的真实测试输出、原故障现场与上述必要对照仍保留；未清理应用原始日志、历史材料或构建缓存。

### 2026-10-12 默认长按连续单击修订

用户明确要求未配置长按时默认连续单击，替代此前缺省关闭规则；不是重复迁移或把旧关闭配置误诊为引擎丢拍。模型新增独立关闭值，缺省自动仅作用于原有可重复按键和动作，长按动作优先；编辑器显示自动/关闭/重复单击/重复长按。菜单与任务识别占位明确关闭，原捕获来源、语音、时序和成对释放路径保持。完整规则与分项结果归 [连发手册](WindowsButtonGestureRepeat.md#2026-10-12-用户调整无长按动作时默认连续单击)。

- **内容身份**：合并基线仍为 `af710b757897cb0567b63c63a7ceb40f62426652`，保留原有及本次未提交工作。0.8.6 本地包 SHA256 `5fd60befb83f5889d7b7024cad5c662b886470bff614d37e62a77c5f51e62dd3`，12,853,490 字节；安装主程序 SHA256 `fe177026a96603d4932ca64192ec544b4b3c5aefd8132a64ab4e6ad6c57ba502`，20,740,096 字节。Helper 保持 `f9ede074f96b56c99e2468ca7137af6c45168c557934d6f4186fe7971ef9cce2`。`sourceCommit=null`，运行 `build_channel=unknown`，不能以版本/HEAD 代替完整源码身份；未签名、未发布。
- **passed（自动化）**：默认无字段行为先红；菜单、任务和合并占位三项在新默认下先红后绿。`pnpm test` 360 项/28 文件通过，内置 IPC 夹具仅同步三个 schema 版本后相关 62 项/2 文件复验通过；`cargo test --workspace` 全过（Windows 500 passed / 22 ignored、Host 145 passed / 1 ignored，另有核心与集成测试），`cargo fmt --all -- --check`、`cargo check --workspace`、`cargo check -p sayall-windows-app --features runtime-simulation`、生产 `pnpm build` 和 NSIS 构建通过。初次完整检查只剩内置夹具 schema 旧值失败，修正后重跑通过，不修改模板动作或弱化断言。
- **passed（部署）**：11:56:35.104–.605 原 App 30736 / Helper 42984 正常退出，overall 500ms、failed_stages=0，Helper cleanup passed / module_unloaded=false。覆盖安装四份配置字节保持，四项载荷一致，安装主程序与构建仅三个 NSIS 标记字节差。11:57:35.289 新 App 46728 / 按键宿主 40748 由 Explorer 12848 普通权限启动，父子 TokenElevation=0；Helper 25884 与设备宿主 12192 中相同 `2026-10-10.request-source` 鉴权成功。11:57:36.247 同线音频启动 passed。无强杀、重启电脑或无线电操作。
- **passed（当前 PC / RC003 实际默认连发）**：关闭自动跟随时，重开回到空通用配置属既有会话契约；用户按提示于 11:58:13–15 用菜单选回 Agent，revision 9 的有效单击和自动掩码均为 6385，长按/明确关闭掩码为零。11:58:16.213–34.410 记录七趟连续单击：左 6 拍、右 6/9 拍（100ms 间隔），返回 3/11/5/10 拍（50ms 间隔），共 50 续拍、不含首拍，全部以真实 release 终止。用户确认“连发、松开停止、菜单和键盘都正常”。各趟实际持有时长并非都达到请求的 2 秒，不虚记；本次启动首用与连续操作通过，不外推长闲置或完整双型号。
- **deferred**：新包 Up/Down/音量逐键、明确关闭/重复长按模式的完整实体矩阵、RC001、长闲置、断连/睡眠/持键退出等；对应自动化与已测当前场景分别记录。新包语音同线启动可读回，但未将前包四次语音会话移作新包端到端通过。原 Code 10、UI 音频两端选择/回滚和历史三轮未知点不因本轮连发通过关闭。

本次必要输出仍复用 `target/dev/foreground-input/hold-default-*`，含原状态、先红后绿、完整检查、配置哈希与唯一生命周期日志；当前包仍在既定 `target/release/bundle/nsis/`。上一语音正常包 `voice-confirmed-a3ec3155-setup.exe` 仅作本次默认规则差异对照，关闭剩余模式验收或候选被替代后回收；旧格式无法理解新显式关闭，不能把对照包直接安装当成安全降级。没有源码副本、额外 worktree、自动提交、推送或发布。

收尾回收本次空格式检查输出和已被完整前端检查覆盖的重复定向通过日志；先红证据、最终检查与实体现场保留，未动原始应用日志、用户配置、历史材料或构建缓存。

### 2026-10-12 音频两端恢复与构建身份组合交付

- **音频成果**：未保存的显式 Capture 草稿与高级写入端一次事务提交，严格同线、失败回滚已提交身份；真实控制器阻塞旧设备读回的审查返工已先红后绿并复审关闭。自动化与剩余失效实机边界唯一归 [输入切换专项](CaptureInputSession.md#2026-10-12-两端恢复事务与失败回滚)，不改默认输入授权、模板、捕获协议或更新源。
- **构建身份成果**：包 SHA-256 `228f6221b6d6c4d3a319b1920af91f5942619c7ba43c44f031192187a64f2957`，12,854,543 字节；raw 与安装 EXE 均为 `1ec6e08bf1794a9f1ead82b774b313b1b7c90313e16493e4ccabff89c935c6e0`，20,699,648 字节。基线 `af710b757897cb0567b63c63a7ceb40f62426652`、`sourceCommit=null`、`sourceState=dirty`、输入指纹 `2d04106f183aafa8feb480f70e30f758e8da939b47cd241f4ae3e4dd15a9db6e`、`local`、`payloadInputs=included`。实际安装 App 48072 日志的五字段逐项匹配当轮 manifest/包 metadata，四项 installed payload 匹配 staging 回执；不把单一基线 SHA 当作 dirty 完整可重建提交。输入 allowlist 是约定源码范围，回执纳入实际包载荷，不涵盖任意磁盘文件。
- **passed（检查）**：全部前端 363 项 / 28 文件、类型检查/实际 `dist-build`、Rust workspace 692 passed / 23 ignored、fmt、workspace 与 runtime-simulation check；SHA helper 修复后受影响身份夹具通过。10 个前端资源嵌入与实际打包路径的生产仿真隔离通过。首次严格打包发现 Cargo 调用环境缺 `Get-FileHash`，原作者改 .NET 文件流 SHA256 并复审；没有重跑未变的功能套件。
- **打包边界**：生产编译与严格身份门禁 passed；首次 NSIS 解压跨盘失败，只对本次子进程复用既有 `target/.tauri-local-temp` 与 `target/.tauri`，使用参数文件重封装已编译应用。NSIS 实际生成并独立核验 passed；Tauri 总命令因不存在本地 updater 私钥最终退出 1，无 `.sig`，不能称完整命令 passed。候选未签名，仅本地验证；未生成密钥、改更新配置或发布。
- **passed（正常退出/覆盖/启动）**：15:52:28.414–29.041（UTC+08:00）旧 App 46728、子 40748 与 Helper 25884 正常结束，overall passed / 627ms / failed_stages=0，held_keys=0、capture_route passed，Helper cleanup passed / module_unloaded=false。覆盖后四份配置字节保持（以安装前最新映射为准），四项载荷一致。15:53:28 后通过 `Shell.Application.ShellExecute` 的 Explorer Shell 路径启动 App 48072 / 子 23688，两者 TokenElevation=0；启动父 15400 随后已退出，未取得其 token，不写成仍运行的桌面 Explorer 12848 父子关系。Helper 24372 对原设备宿主 12192 中 `2026-10-10.request-source` 鉴权成功。严格同线 startup passed；真实连接页 UIA 读回目标下拉、paired 与无未保存草稿。无强杀、重启或无线电操作。
- **passed（本包当前 PC / RC003 用户复验）**：用户确认遥控方向/返回连发与松开停止、实体键盘实际键与方向长按、闲置首次与连续语音及反馈正常。本包日志在15:57–58有真实语音 generation 3–5，供音分别193200 / 73440 / 39600 samples，终态队列均0，三次均先默认角色确认，释放后 original_mask=7；不用前包的四次语音或七趟连发充作本包证据。用户闲置首用确认仅属本机此次观察，不外推全部输入工具或更长冷态。
- **failed（本包菜单）**：用户报告 Codex 前台菜单按下出现、释放瞬间消失；误触主页打开无线麦后切回 Codex，菜单随后正常。15:56:21–23首批日志 return_target generation15–18为external_window，focus verified=false、`foreground_verified requested=false/result=false`（旧合并字段，不是单独记录的激活 API 原生结果）；不将 snapshot_applied 或随后恢复当作首次菜单通过。最小现场保留 `audio-recovery-menu-regression-evidence.log`（548行，199,632字节），补齐隐藏、文档加载与主页关键事件，关闭早于UP等时序归 [同主题 Bug](../Bugs/2026-10-03-foreground-input-regression.md)。原始当前及轮转日志未改、未重启重装。旧 Code 10、标准 pin 原因、历史三轮/17:56 未知、RC001/ARM64/完整睡眠断连及暂停的捕获换代/租约边界仍未关闭；后续诊断结果见同页菜单诊断候选节。

包仍位于 `target/release/bundle/nsis/无线麦_0.8.6_x64-setup.exe`；本轮必要红测、manifest、metadata、实际生命周期及回退包复用 `target/dev/foreground-input/`，回退目的和清理条件归输入切换专项。不创建源码副本，不提交、推送或发布。

### 2026-10-12 菜单诊断候选

- **候选身份与部署**：包 SHA-256 `e39e642c42299f440492838a9fb268c9e17a8a0c5f7a0fbdb1e519636f7a374d`（12,858,164 字节），基线 `af710b757897cb0567b63c63a7ceb40f62426652`，source fingerprint `fb31da04e942670b294ec83aa4a0015db84f787d149fc95168f260900b27d77d`，dirty/local/included，`sourceCommit=null`。raw EXE `84cc2f515d014003f47fa010af6ad52cf21e27b721940cf4849f78fa0b5d2219` 与 installed EXE `94a6ab4999e058e537de51ec4de354b2f5558b70625fc970d976cc0c22705f66` 均为 20,696,576 字节。NSIS 标记差异限于安装器 offset 15467490–15467492 的 `UNK`→`NSS`，NSIS build patch 记录确认是构建标记，不是错包。正常退出约 544ms、held=0、各项 cleanup passed；正常覆盖安装四 payload 和四份配置字节均保持。NSIS 生成/核验 passed；Tauri 总命令仍因无 updater 私钥退出 1、无 `.sig`，本地未签名候选。实际启动走 Explorer ShellExecute open，App 36272 / 子进程 21196 均 TokenElevation=0，Helper 20812；五字段运行身份与构建记录一致。
- **passed（自动化）**：Host 全工作区 695 passed / 0 failed / 23 ignored，日志 `target/dev/foreground-input/menu-diagnostics-final-workspace-test-host.log`。一次 sandbox 测试超时后只结束了测试进程；它不是代码回归，也不覆盖上述最终成功运行。
- **failed（当前 PC / RC003 菜单）**：用户在 Codex 前台、未先打开主页或无线麦主窗时，短按 Menu 仍观察到菜单出现后消失。本包仅增加诊断，不改关闭策略。generation 8–18 共 11 次激活尝试中，明确记录窗口 `visible=true`、`minimized=false`、`no_activate=false`、调用方为窗口线程；激活 API 实际返回 false 且真实 foreground 未变，程序随后清面板并隐藏。确认“API 激活拒绝被当作立即取消”是本次菜单直接消失的原因；Windows 拒绝激活的底层原因仍未知。`native focused=true` 不等价于真实前台窗口。菜单事实与脱敏证据 `target/dev/foreground-input/menu-diagnostics-current-repro.log`（80,981 字节，SHA-256 `f322012c6888137f35b964a83e932b19f719e5ca34e4264d82a4fcafd48997b9`）归 [同主题 Bug](../Bugs/2026-10-03-foreground-input-regression.md)。
- **证据修正与范围**：此前 548 行旧现场中的 generation 15–27 `requested=false` 是合并前置窗口操作与 API BOOL 的字段，不能倒推 13 次均调用了激活 API并返回 false；旧字段具体覆盖的阶段以当时日志实现为限。新的 11 次实际 API 结果只适用于本诊断包。本包用户菜单验收仍 failed；上一组合包 `228f6221` 上用户确认的连发、实体键盘和语音结果不外推为本包验收。捕获生命周期 lease/hotreload 仍按要求暂缓，更新源未改。菜单修复及后续新包待独立验收。

### 菜单恢复实现与自动化（局部用户交互已实测）

- 修复源码已最终冻结并经独立审查，无 P0/P1/P2 必修项。五文件变更以 scene control 和只读窗口身份透传为主：SetForegroundWindow 首次被拒时，仅当同次开窗捕获目标仍是实际前台才保留 remote-only 面板；键盘操作/确认匹配精确菜单 HWND；每次遥控/原生手势及 UP 重新读回实际窗口，前台切出即取消。开窗 pending、前台观察、语音/配置取消后的迟到 UP、Menu Long 保存和旧代次清理覆盖于回归。
- **passed（自动化）**：`cargo test --workspace` core 32 passed、Windows 522 passed / 22 ignored、Host 153 passed / 1 ignored；其余 workspace 库与文档测试通过。窄测 71 passed / 1 ignored，Host 菜单专项 7 passed；fmt、workspace check、runtime-simulation check 与五文件 diff check passed。日志位于 `target/dev/foreground-input/menu-fix-workspace-test.log`、`menu-fix-workspace-check.log`、`menu-fix-runtime-simulation-check.log`。`scene_control.rs` 最终 SHA-256 `022F1FE0…375`，源码仍为 dirty 工作区，不能用 HEAD 代替完整身份。
- **deferred（完整内部链路关联）**：e39 诊断包上的用户复现仍为 failed。新候选日志确认实际出现一次 activation API 拒绝后面板保持、snapshot 应用通过；但没有用户动作关联标识，不能断言这就是用户第一项操作，也不能证明“拒绝→继续遥控→确认模板”在同一次真实交互完整串联。未见 remote-only 专属字段。用户三项菜单观察仍按本机本次记录为 `passed`。Windows 拒绝激活的具体原因未知。未修改模板、前端、更新源或捕获 lease/hotreload；后者继续暂缓。

### 菜单恢复修复候选安装（用户观察）

- 本地 NSIS 安装包 `无线麦_0.8.6_x64-setup.exe`：12,870,141 字节，SHA-256 `370b51b214cefac324d6966ee0631188d5155db7faa9b9978e9a7834c264c32b`。基线 `af710b757897cb0567b63c63a7ceb40f62426652`，`sourceCommit=null`，dirty/local/included，source fingerprint `ad1c0484b521c21bd5ebd5a220b66c13c168ee51d857a89021cdda1af56cdaaf`。
- raw EXE 20,737,536 字节、SHA-256 `4279051b62848a9ade0e5c03c9b4820a81ab847d63686714da6c6635bfd15979`；installed EXE 同尺寸、SHA-256 `021706b0d9b48a6048a2d7b2f2bc2e6bfd72530f0559f4bca40697649d30d892`。二者仅安装器标记 offset 15497730–15497732 的 `UNK`→`NSS` 三字节不同；Tauri bundle patch 有记录，其余字节完全一致。
- **passed（安装与运行身份）**：旧版正常退出、failed_stages=0、held_keys=0，三个相关进程正常退出、无强杀；覆盖安装 exit 0 / 约 8.3 秒。四项安装 payload、10 项前端嵌入及 production simulation 隔离检查通过；四份配置安装前、安装后及启动后 SHA 一致。Explorer 普通 ShellExecute(show=1) 启动 App 45320 / 子 10424（安装路径 TokenElevation=0）、Helper 39324；运行五字段与新 manifest 匹配。启动记录有 main_window loaded/focused，但没有人为打开主窗口、主页或菜单，不能视为实际 API 拒绝分支已验证。
- NSIS 生成与独立核验 passed。Tauri 总命令在 updater 签名处因本机缺少私钥失败，监控 shell 回收未取到整体数值退出码；未生成私钥或 `.sig`，安装包未签名。首次 payload 校验调用参数错误，未进入验证；显式提供 StagedDirectory 后通过。首次 EXE 比较复用了旧候选固定偏移并误报，改用动态标记及全文件字节核对后通过。具体过程日志均在 `target/dev/foreground-input/menu-focus-fix-*`。
- **passed（用户实际菜单观察，本机/当前遥控器）**：用户回复“一二三都成功了”：Codex 前台短按 Menu 后松开，菜单保持；遥控方向选择模板并确认，模板正常应用；重新开菜单后切换到其他软件，菜单关闭。此三项是本机本次用户观察，不外推到 RC001、另一台 RC003、长期稳定或全按键新一轮验收。
- **运行日志中的分支证据与关联限制**：同一安装进程 `10:37:17.776Z`（北京时间 `18:37:17.776`）generation 13 激活 API 返回 false、actual_foreground=false；窗口可见、未最小化、no-activate=false、调用方为窗口线程。随后 `activation_denied_policy` 记录 panel_before=true / panel_after=true，`snapshot_applied` passed，确认本轮出现“激活拒绝后面板保留”状态。`10:38:15.075Z` generation 22 API 与实际前台均成功、面板保持；`10:38:16.756Z` generation 23 模板选择 `actions_available=true`，随后有 `focus_ignored hide_completed`。脱敏证据 `target/dev/foreground-input/menu-focus-fix-user-validation-evidence.json`（1,823 字节）。没有用户动作关联 ID，不能唯一断定 generation 13 对应用户第一项操作，也不能证明“拒绝→继续遥控→确认模板”完整同次链；未见 remote-only 专属字段。三项外部行为仍按用户确认记当前本机观察 `passed`。Windows 拒绝激活的底层原因仍未知。
- **材料留存**：保留当前 `370b51b2` 安装候选供当前部署恢复；保留 e39 单安装器失败对照以比较诊断失败现场；保留 `5fd60bef` 基线以对照此前 dirty 音频/构建身份。完成本轮完整链路验收、相关引用与对照唯一性核对后，再按 LOGGING 规则确认是否回收；本轮不删除。

### 本地历史局部验证（不代表当前候选通过）

2026-09-15 登录自启动历史实证：release 安装版 0.2.6 注销重登后由登录 shell 自动启动，父进程为 Explorer；日志链为 `document_load finished → vue_mount(80ms) → initial_ipc_ready(337ms)`，并有 `startup feature=launch_at_login action=sync terminal_result=passed enabled=true`。该结果只证明当时版本的当前用户 Run 项启动；Win+L 锁屏/解锁不会重新运行 Run 项，不能代替注销重登验收，不外推当前候选或硬件功能。

关于页“启动行为”与“软件更新”间12px分组间距的历史局部修改已通过类型检查与 AboutPage 仿真测试；仅证明 CSS/页面边界，无独立硬件验收要求，不作为当前全部设置或安装矩阵通过证据。

### 上游历史记录（不代表本 fork 当前合并候选通过）

以下新导入的上游实验只适用于各段明确的提交与环境；原始材料可追溯到本次固定上游提交。未在本 fork 重做的场景继续 deferred。


2026-10-04 在 Andy 的 Windows 真机上完成语音增益 RC003 对照（操作人执行；遥控器语音直接进输入法、操作人听不到音频，判据 = 回录电平 + 日志 + 转写结果）：

- 本地测试包 `无线麦 SayAll_0.5.0_gain-521cc4b_x64-setup.exe`（SHA-256 `a766c5eb86d732aaa8546538aee7ead5e42e396b0141fe0bdc724463f7f5ffb3`）安装后，同一支 RC003 在 24 dB 与 0 dB 各完成一段真实会话（07:03:40–50Z / 07:04:03–15Z，各约 10–12 秒，共 156,480 / 187,680 个采样）；
- CABLE Output 回录：24 dB 段能量 RMS 2671.8、峰值 16143；0 dB 段能量 RMS 124.1、峰值 1476；能量 RMS 比值 **21.53×**。24 dB 理论倍数为 15.85×，余量来自两遍人声自身约 1.36× 的音量差；若增益未生效，两段只会相差约 1.4×，不可能到 21.5×。24 dB 段峰值 16,143 未削波；
- 日志时间线：`action=restore ... gain_db=24` → `audio_session generation=1`（24 dB 段）→ `feature=gain action=save ... gain_db=0` → `audio_session generation=2`（0 dB 段），两段 `finish terminal_result=passed`、`queued_samples=0`；同日 06:47 连续 10 次会话（含 24 dB 两次）全部 passed，覆盖冷态首用与连续会话；
- 合成信号回环（`gain_effect_probe`，走产品 `process_pcm` 路径）：12 dB → 3.98×、24 dB → 15.85×，与 dB 定义一致；
- 证据目录：`artifacts/local-test/2026-10-04-audio-gain/`（`real-session-level-compare.txt`、`gain-probe-loopback.txt`、`build-metadata.json`）。
- **仍 deferred**：RC001 同链路；睡眠/断连恢复后的增益复测；含每会话 `audio_gain action=apply` 日志（提交 `3332e1c`）的安装包复测与转写文字 A/B。

2026-09-02 在 `windows-latest` 完成 Tauri/WebView/IPC 运行时仿真：

- Windows CI Run [`33611750471`](https://github.com/GetSayAll/remote-mic-app-windows/actions/runs/33611750471) 使用 Tauri CLI 构建仅测试 feature 程序，并在真实 Windows WebView 中完成 11 个结构化步骤；
- WebView JavaScript 通过真实 Tauri IPC 读取仿真快照，渲染 RC001/RC003 扫描结果，连接 RC001、选择仿真 CABLE Input、启动 Raw Input、保存映射并由非注入式 SendInput 记录器验证 Ctrl+C 四事件；
- 五个侧栏页面均完成导航和挂载，权限页生成平台标记为 `windows-ci-simulation` 的去标识化诊断摘要，统计 IPC 返回最近七天结构；
- 首次 `STREAM_START → 40 + 80 AUDIO → STREAM_STOP → DRAIN` 通过纯 Rust ATVV 管线得到 240 个采样和 generation 1，随后 Raw Input 与连接状态均完成释放；
- 同一 Run 重新构建普通 NSIS，二进制和前端资源均确认不含仿真平台名或仿真 command，并继续通过静默安装/卸载回归；
- Run [`33610428178`](https://github.com/GetSayAll/remote-mic-app-windows/actions/runs/33610428178) 首次暴露直接 `cargo build` 生成开发协议程序会等待 Vite dev server；改为 `tauri build --no-bundle --features runtime-simulation` 后修复；
- 该结果不访问真实 WinRT BLE、WASAPI、Raw Input 或桌面 SendInput，不代表 RC001/RC003、音质、VB-CABLE 或真实按键已经通过。

2026-09-02 在 `windows-latest` 完成 NSIS 静默安装与卸载生命周期验证：

- Windows CI Run [`33593663937`](https://github.com/GetSayAll/remote-mic-app-windows/actions/runs/33593663937) 从提交 `5f01fa41cfbd15b967a760e490b9343260e768a6` 构建 `无线麦 SayAll_0.1.0_x64-setup.exe`，SHA-256 为 `a9fa2bfbfd519189629abc3ed91d5a811752081452d93c475c62129dd1e16e44`；
- `/S` 安装以普通 Runner 用户完成，唯一卸载条目位于 HKCU，安装目录为 `%LOCALAPPDATA%\无线麦 SayAll`，开始菜单文件夹内只有一个 `无线麦 SayAll.lnk`；
- 安装后的主进程持续运行 8 秒，未立即崩溃；测试随后关闭进程，并使用卸载注册表中的真实命令执行 `/S` 卸载；
- 卸载后程序目录、开始菜单文件夹和卸载条目均移除，`%APPDATA%\app.getsayall.remote-mic.windows` 中的测试标记保留，确认默认静默卸载不删除 Tauri `app_config_dir`；
- Run [`33591214805`](https://github.com/GetSayAll/remote-mic-app-windows/actions/runs/33591214805) 在安装前暴露空注册表结果的 PowerShell 数组边界，Run [`33592013119`](https://github.com/GetSayAll/remote-mic-app-windows/actions/runs/33592013119) 暴露带引号 `InstallLocation`，Run [`33592799733`](https://github.com/GetSayAll/remote-mic-app-windows/actions/runs/33592799733) 暴露非交互 Runner 无法通过 WScript 读取快捷方式目标；三项均按实际 Tauri NSIS 注册表契约修正；
- 该结果不证明可见安装/卸载界面、语言选择、SmartScreen、Windows 10 1809、Windows 11 真实桌面、签名发布包、RC001/RC003 或第三方应用已经通过。

2026-09-02 在 Apple Silicon Mac 上完成 Rust ↔ TypeScript IPC 契约快照验证：

- 同一份 JSON 夹具覆盖完整 `PlatformSnapshot`、嵌套连接/音频/Raw Input 快照，以及 RC001、RC003、unknown 三种 `PairedRemote`；
- Rust 测试从真实结构序列化并与夹具结构级全等比较，同时拒绝任何 snake_case IPC 字段；
- TypeScript 测试使用前端接口装载同一夹具，核对枚举白名单、精确字段集合、`remoteModel` 和 `isSupportedCandidate` 的 camelCase 边界；
- 该结果只证明仓库两端静态类型和序列化契约一致，不证明 Tauri command、Windows WebView、WinRT 或真机运行时数据已经通过。

2026-09-02 在 Apple Silicon Mac 上完成 RC001 协议场景回放：

- 从 `GetSayAll/hardware-simulation@65248499cac7da3ad46cd0c11dca1478f7733255` 的 RC001 短语音场景提取纯 ATVV JSON 夹具，保留 `STREAM_START`、40 + 80 字节音频拆包和 `STREAM_STOP`；
- 夹具回放确认无主动 `MIC_OPEN`，两段音频恰好组成一个 120 字节帧并解码为 240 个采样，停止时没有遗留半帧；
- 同一夹具扩展覆盖 20 次极速按下/释放空会话、20 次完整语音会话，以及首段 40 字节半帧后中断、下一会话仍首次完整解码；
- 该结果只证明纯 Rust ATVV/帧累积/ADPCM/会话状态机在确定输入下的行为，不证明真实 RC001 固件、WinRT 通知顺序或实际音质。

2026-09-02 在 Apple Silicon Mac 上完成 RC001 支持实现级验证：

- 对照 macOS `b233a88cc4457b00413dda6b37ec8b4af12c5121` 的设备档案、标准 2A24 型号识别和 RC001 首次短语音路径；
- Rust 单元测试覆盖 RC001/RC003 名称、`RC001` / `RC003` Model Number、通用名称不猜测型号、两种型号高半字节优先 ADPCM，以及 RC001 无需主动 `MIC_OPEN` 的 `STREAM_START → AUDIO → STREAM_STOP`；
- `sayall-core` 25 项、macOS `sayall-windows` 24 项、Tauri Host 6 项测试通过，`cargo check --workspace` 和 `x86_64-pc-windows-msvc` 平台层交叉静态检查通过；
- Vue 4 个测试文件共 9 项测试、TypeScript 类型检查、Vite 生产构建和 `tauri build --no-bundle` 通过；
- 以上结果只证明型号字段、WinRT API 类型、共用 ATVV 管线和界面可编译且自动化通过。真实 Windows 上的 RC001 与 RC003 设备名/2A24 值、首次语音、按键报告、睡眠恢复和 VB-CABLE 回环均为 deferred。

2026-09-01 在 Apple Silicon Mac 上完成：

- Vue 导航、连接阶段映射、诊断格式和权限页复制组件测试、类型检查和 Vite 生产构建；
- 900 × 620 与 1080 × 720 浏览器界面检查，全部五个侧栏入口可打开；权限页无横向溢出，诊断摘要可在页面内生成和滚动；
- 浏览器控制台零错误；
- `sayall-core` 22 项测试；macOS `sayall-windows` 20 项平台边界、退避、Raw Input 和 SendInput 纯逻辑测试；Tauri 设置与诊断 4 项测试；Windows CI 额外运行 WASAPI generation、有界队列、持久端点身份、重连调度、电源回调转发、Raw Input 纯逻辑测试和完整 Tauri Host 编译；
- 诊断隐私测试使用设备名称、蓝牙地址、端点身份、用户路径、HID 路径和后端错误作为敏感夹具，序列化摘要均未包含这些值；浏览器生成摘要也未出现 `remoteName`、`selectedEndpointId`、`selectedEndpointName` 或 `lastError` 字段；
- Raw Input 自动化覆盖两种 Windows 设备路径、零/多匹配 fail-closed、6/7/9 字节 HID 报告、RAWHID 批量拆分、重复 key-down、Keyboard/HID 双来源并集、停止释放和语音键隔离；隐藏窗口和真实事件形态仍需 Windows/RC001/RC003 分别验收；
- 纯 Rust 首次 `CAPS → STREAM_START → AUDIO → STREAM_STOP → DRAIN`、同步帧、8 kHz 拒绝和流外音频测试；
- macOS 全 workspace `cargo test`、`cargo check` 和格式化检查；
- `x86_64-pc-windows-msvc` 目标下 WinRT BLE/GATT/ATVV/WASAPI/Raw Input 平台层交叉静态检查。
- Windows CI Run [`33516627294`](https://github.com/GetSayAll/remote-mic-app-windows/actions/runs/33516627294) 通过；该结果证明 Windows 代码和 Tauri Host 可编译、自动化可运行，不证明隐藏窗口收到真实 RC001 或 RC003 报告。
- Windows CI Run [`33522534257`](https://github.com/GetSayAll/remote-mic-app-windows/actions/runs/33522534257) 通过并生成 artifact `sayall-windows-unsigned-preview-e11e460f15287468c01225a8ab86b302d59f09d4`；下载后确认只有一个 4,565,134 字节 NSIS 安装器、`SHA256SUMS.txt` 和 `build-metadata.json`，UTF-8 中文文件名可由 macOS `shasum -a 256 -c` 校验，安装器实算 SHA-256 与两份记录一致为 `e1cccf73c8b20487874eadb12eb9823635f52a0d7664c07fe6b8928532d246d6`，metadata 标记 `NotSigned` 和 `unsigned-ci-preview-not-for-public-release`。
- main Windows CI Run [`33525406955`](https://github.com/GetSayAll/remote-mic-app-windows/actions/runs/33525406955) 从 merge commit `94923dea303a66604b61ca2572fd13d9d48a9d60` 生成同边界 artifact 并通过下载后 SHA-256、metadata、PE/NSIS 文件类型和 UTF-8 校验文件复验。
- main Windows CI Run [`33529876579`](https://github.com/GetSayAll/remote-mic-app-windows/actions/runs/33529876579) 从诊断功能 merge commit `f8a65d33c31283c3db9e7964d9a9a36bc1e9f6c3` 成功完成前端、Rust、Windows Host、NSIS 和 artifact 上传；下载后确认唯一安装器为 4,561,376 字节，SHA-256 为 `cfddc29da4ed40d543039c545a0ab23993165243b95bf4b7335d98ff5ed848b3`，metadata 来源提交、`NotSigned` 和 `unsigned-ci-preview-not-for-public-release` 均一致。

2026-09-02 在 Apple Silicon Mac 上完成本机使用统计实现级验证：

- `sayall-core` 覆盖每日批量累计、今日/周/总汇总、饱和计数、非法持久时长归一化和旧 `settings.json` schema 迁移；
- Windows 平台计数覆盖普通按键按下次数、完整语音会话次数和 16 kHz 采样累计；语音只在成功排空后提交统计，中断路径清除未完成采样；
- Tauri 统计层覆盖进程计数增量、采样到秒换算、周一起始的本周汇总和最近 7 天日期桶；文件写入由独立后台线程执行，不进入 BLE、WASAPI 或 Raw Input 回调；
- Vue 组件覆盖今日、本周、全部切换、时长格式、最近 7 天数据和零数据空状态；900 × 620 与 1080 × 720 浏览器检查无横向溢出，新增统计界面最终字号不小于 16px，控制台零 warning/error；
- `x86_64-pc-windows-msvc` 目标下 `sayall-windows` 平台层静态检查通过；完整 Tauri Host 交叉检查在 Mac 上仍受缺少 `llvm-rc` 限制，交由 `windows-latest` CI 验证；
- Windows/RC001/RC003 真实按键计数、真实语音计数、跨自然日、退出落盘和升级保留均为 deferred，必须对两种型号执行用例十一后才能称为真机通过。

2026-09-02 在 Apple Silicon Mac 上完成 Windows 10 1809 版本门禁实现级验证：

- 纯 Rust 版本比较覆盖 Windows 10 build 17762 拒绝、17763 边界接受、更新 Windows 10 build 和未来主版本接受；
- `sayall-windows` 的 `x86_64-pc-windows-msvc` 静态检查通过，确认 `RtlGetVersion` 封装与原生 `MessageBoxW` 代码可编译；
- Tauri 配置解析、macOS workspace 测试/检查、前端 8 项测试、生产构建和 `tauri build --no-bundle` 通过；
- NSIS hook 的实际编译交由 Windows CI；低版本安装提示、退出码 1633、直接运行 exe、Windows 10 1809 接受和 Windows 11 接受仍为 deferred，必须执行用例零后才能称为运行验收通过。

以上结果只证明 Tauri Windows NSIS 未签名 Preview 可在干净 Windows Runner 生成且产物身份、摘要和来源元数据一致；不能证明安装、升级、卸载、WinRT 运行时、真实 RC001/RC003、WASAPI、Raw Input 或 SendInput 已经通过，这些用例仍为 deferred。
macOS 上对完整 Tauri Host 的 Windows 目标交叉检查需要 `llvm-rc`，本机未提供该 Windows 资源编译器，因此完整 Host 由 `windows-latest` CI 验证。

诊断复制已通过前端组件测试确认向 Clipboard API 写入完整可见摘要；应用内浏览器的剪贴板回读与页面 Clipboard API 不共享同一自动化通道，因此不能把浏览器提示当作真实 Windows WebView 剪贴板验收。Windows 安装包中的生成、复制和记事本逐字比对仍为 deferred。


## 会话临时麦克风输入（2026-09-16 本地候选）

这项验证只覆盖用户明确允许的受限 IPolicyConfig 会话功能，默认关闭；不能外推三键修复、20 秒内部队列溢出或 RC001。未文档化 setter 没有 CAS，通知没有变更者；检测到外部变化后整次让出，不宣称消除全部读写间竞态。

1. 安装前后保存 SayAll 配置哈希、普通用户实例与固定包载荷证据；不回滚用户的 Windows 默认设备选择。记录 Capture 的 Console/Multimedia/Communications 三角色基线。
2. 用户在“连接与语音”读取输入设备，选择 CABLE Output 并显式开启“遥控器说话时临时锁定麦克风输入”。原写入端点仍 CABLE Input；不自动选择目标，不因音量键或闲置改变默认设备。
3. 先在目标软件按住 3–5 秒并松开。日志核实三个角色实际读回目标后才发快捷键 DOWN，源音频提交/排空，快捷键 UP 先于默认角色恢复。公开会话观察器核 WeType 实际 active CABLE Output；用户确认目标收到遥控器声音。API 成功、路由选择或静音帧数均不能代替最终收音。
4. 分别验证快速释放（准备中取消不得迟到 DOWN）、连续会话、真实闲置首用、开关关闭、设备 missing/replaced、耳机插拔或系统手选。检测到任一角色偏离预期后全部角色保持当时现状，不争抢、不再恢复其他角色。会话进行中设置变更须提示松开后操作。
5. 断连、睡眠、正常退出均先走 BLE owner 的快捷键 UP，再做路由恢复。记录未能恢复时的具体错误与 journal 状态。异常退出恢复只在隔离验证环境制造：下次启动必须显示“恢复/保留当前选择”，未点击不得静默改设备；即使未改过的角色发生外部变化，也不能恢复其余角色。
6. IPolicyConfig 不能保证目标软件覆盖其显式指定的麦克风。将三角色默认确认、WASAPI 实际供音、WeType 采集端点和用户可见结果分别记录 passed/failed/deferred；有硬件但未操作的项标未执行，不虚构 deferred 原因。

本候选真实验收证据归 `334c372:artifacts/capture-input-session-20260916/evidence.md`。状态以该记录为准。
