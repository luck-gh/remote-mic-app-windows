# 主窗口前台时的菜单、按键链路与语音启动异常

## 2026-11-13 升级1.9.0.12后驻留组件版本不匹配

- **failed / 用户反馈**: 1.9.0.12 安装并重开后, 用户报告全按键异常, 返回、音量加、音量减不能正常使用。未提供这轮 Menu、TV 或连续语音的结果, 不沿用旧版通过记录。本次没有完整逐键采样, 不补写次数或持续时长。
- **直接原因 / 日志确认**: 北京时间18:00:40.902与18:00:52.701, Helper55780、20340分别记录 `[AGENT-STALE] running=2026-10-23.auth-lease expected=2026-11-13.personal-source-clear reason=resident_agent_version_mismatch`。两轮均以 `capture_acquired=false terminal_result=agent_version_blocked` 结束, App30024随后记录同一阻塞原因。清理回执为 `agent_version_blocked`, 指向仍存活的WUDFHost3816。新版安装文件与运行身份通过, 不代表宿主内旧 Agent 已换代; 本次全按键未取得捕获, 不是已证明的模板失配、蓝牙断连或麦克风切换故障。
- **未解决边界**: 旧驻留组件与新 Helper 契约不匹配, 仍缺少已验证的安全换代路径。不能据此把所有同版本日常重开异常归于此原因, 也不能删除回执、放宽版本校验、强杀宿主或恢复旧 DLL 私有偏移路径来伪造接管通过。此前安装成功和蓝牙/音频 ready 的证据保留其原范围, G1整体不关闭。
- **deferred / 后续验收**: 用户选择下次自行正常重启 Windows 后再验证; 当前未重启、未热替换、未启停接管。届时先读回新 Agent 身份与接管终态, 再验证返回、音量加减及 Menu、TV、连续语音。即使重启后恢复, 也只证明干净宿主加载后的效果, 不能关闭升级交接及正常退出/重开回归。重启不是产品的常规恢复方案, 仍需在独立全按键组件交接工作项解决。
- **证据与隐私**: 只读汇总个人版当前及轮转诊断日志后提取上述两轮事件, 并核对清理回执与宿主存活; 未复制完整日志、读取凭据或记录设备路径、输入内容。本轮仅维护故障与验收状态, 不修改生产代码、版本号或用户配置。

## 2026-11-02 上游退出后个人版服务发现持续超时

用户报告同时打开两版, 关闭上游后个人版无法重新连接。既有日志只读对齐确认, 北京时间 10:57:10 个人版主动断开并完成会话清理; 10:57:22 上游建链成功。上游 10:59:15 开始 `ble_session_cleanup`, 至 10:59:45 退出仍无完成记录, 随后报告 `shutdown_timeout` / `session_cleanup_unconfirmed`。个人版 10:59:56 恢复尝试, 截至 11:17:12 的 34 次均在 `service_discovery` 约 5 秒超时, 设备对象创建及每次失败后的 `partial_cleanup` 均通过, 30 秒退避和 Helper 续约仍工作。核对时上游主程序/Helper 已不在, 不能解释为个人版界面卡死或旧上游进程仍占用。

上游退出期间 Helper 另记录共享宿主中第二代 Gadget, 但缺少它导致 GATT 阻塞的证据, 不将其确认为根因。完整责任仍未知: 上游清理没有足够分段区分退订、CCCD 提交与本地 Close; 个人版 `wait_async` 超时只请求 Cancel, 尚无取消完成、晚到结果和同轮实际连接状态证明。最新上游 PR315 的断链跳 CCCD 在本地已存在, 本次已知 RC003 也不触发未知型号探测, 不能以移植整项或增大超时宣称本故障修复。

用户随后补充: 手动断开蓝牙再重新连接后, 个人版恢复连接。此为用户观察到的恢复结果, 未核实具体系统操作/API及恢复会话日志, 不改写此前 34 次失败, 不将人工操作设为常规解决办法。当前 `failed`: 两版退出交接后的自动重连; `passed` 仅限上述本地失败收尾/退避与用户报告的人工重连后可用; 完整根因及自动恢复修复仍未闭合。下一最小诊断应观察取消结果、晚到完成与实际连接状态, 不强杀宿主、卸载驻留组件或重置蓝牙。脱敏最小现场留在 `target/dev/upstream-ui-layout/coexist-summary.json`, 在自动恢复闭合前保留, 不依赖该本地报告才能理解上述结论。本轮未改后端连接行为或将 UI 包当作共存修复。

## 2026-11-01 个人版归档根目录隔离候选

继续既有失败项时确认: 文字归档默认根、音频 writer 默认根及应用启动 fallback 仍含上游 SayAll。此问题影响个人版数据隔离, 不据此归因语音未收尾或 Helper 停止未确认。本轮仅将三处根统一为 SayAllPersonal, 同步原有路径说明; 不迁移、扫描或删除旧目录, 不增加兼容读取, 不改变切麦、按键或录音时序。文字侧录音读写/导出/删除通过 self.root/audio 跟随同一根, 正常 LOCALAPPDATA 环境下与 writer 对齐。旧共享目录内容原地保留, 不会自动出现在新列表。

- Node 原隔离断言与新增归档检查得到 2 passed / 2 failed, 修复后 4 passed。
- 两项 Rust 测试分别调用生产默认根函数, 录音与文字各自独立得到 0 passed / 1 failed, 修改后各 1 passed。
- 录音全套在沙箱默认临时目录为 4 passed / 3 failed; 单例复核的既有日志明确 finalize reason=rename_failed。仅将 TEMP/TMP 约束到工作区 `target/dev/microphone-switching/test-temp`, 不改测试或产品超时, 同源录音全套 7 passed, 文字归档全套 16 passed。不能把受控目录通过扩大为所有系统临时目录均通过; 原失败与诊断保留。
- fmt、workspace check、runtime-simulation check 及独立窄审 passed。既有 IME 两步切换的故意 RED 未修改, 不据本次定向结果宣称整个工作区全部通过。

证据复用 `target/dev/microphone-switching/archive-*`。此切片未操作设备或读写实际用户归档; 随后已包含在 11 月 1 日模板页组合候选中安全重建、安装并核对进程身份, 详见 [模板页实装](../Testing/StructuredTemplatesAndDrivers.md#2026-11-01-模板页顶部与说明留白修正)。未执行实际录音归档/文字历史读写, 不把安装成功扩大成归档功能通过, TODO 保持未完成。LOCALAPPDATA 缺失时文字 fallback 与音频禁用的不对称是既有边界, 本轮未改变。归档切片仍未提交, 独立 UI 检查点不包含它, 没有推送或发布。

## 2026-11-01 重启后语音被切麦恢复记录阻断

用户确认已重启 Windows, 随后报告语音键无响应。实际运行仍是 `4a42df15` 的 dirty/local 旧候选, EXE SHA256 `7521b5bd0660cf3e369e125182aa5c40cb9a721d37bb700a77130884ab04c9b3`, 10 月 30 日构建的新包尚未安装。此前卡住的 Helper 已不在, 当前应用与 Helper 为重启后的新进程; 不把本次故障继续归为旧进程残留。

14:18:28 至 14:19:22 的 generation 1 至 8 均收到语音按下与松开, 但 capture prepare 返回 `recovery_required`, roles/name/journal/policy 调用计数全零, 准备即中止。直接阻塞是持久的麦克风切换待恢复事务, 不是已证蓝牙断连、按键没收到、系统 setter 调用失败或 CPU/内存负载。重启结束进程不等于自动处置此事务; 未进一步证明原事务为何没有正常恢复。

14:21:33 恢复请求因 `recovery_external_change` 拒绝; 14:21:36 恢复选择 passed。用户随后明确确认自行点击“保留”, 且语音“现在好像正常了”。generation 9 的三角色切换确认、快捷键 DOWN/UP、音频 finish 与原设备恢复通过: submitted/decoded 为 19680 samples, queued 为零, decode_errors 为零, 82 个回调包。之后只读确认事务内容为 null, 不用文件仍存在判断仍待恢复。用户可见恢复与单会话日志相符, 不扩大成长期稳定或新候选验收。

本轮仅只读诊断与公开辅助功能按钮检查, 没有代点恢复、修改系统设备、删除事务、安装新包或提交 Git。最小匿名计数在 `target/dev/microphone-switching/reboot-voice-result.json`; 后续成功边界及用户确认归本节, 不以末次成功覆盖前八次失败。旧语音未收尾、停止确认及恢复提示交互仍按原范围继续保留。

## 2026-10-30 语音未收尾与停止确认等待

用户先报告打开公开版与个人版后卡在恢复状态, 随后确认实际为遥控按键失效、语音键已松开且无线麦与输入法均持续显示录音。只读调查未启停进程、发送按键或更改系统设备。

- 旧个人实例在 15:04:09 开始 audio generation 66, 15:04:10 的 session 26 快捷键 DOWN 返回成功; 后续保留日志没有对应 ATVV STOP、快捷键 UP 或音频 finish/abort。此前 session 25 的 START、STOP、UP、finish 完整。没有释放日志不等同于直接读取到操作系统键态, 也未观察第三方内部状态。
- 该次未收尾开始早于公开版约 15:13 启动, 因此不能把两版同时开启认作语音故障原因。个人版安装、任务、端点与管道已隔离, 同一物理设备和宿主的并发限制仍存在。
- 补跑 `node --test scripts/test-personal-install-isolation.cjs` 为 2 passed / 1 failed: 安装身份及退出/激活隔离通过, runtime 源码断言在 `src-tauri/src/lib.rs` 新归档 fallback 的 `.join("SayAll")` 处失败。进一步只读源码确认 `TranscriptStore::default_root` 和录音存储仍使用上游根目录。该存储隔离缺口不等同于按键 IPC 混用, 也未证明导致本次录音未结束; 本轮未读取归档内容、迁移个人数据或修改这条独立链路, 不宣称两版全部数据已隔离。
- Helper 在 15:14:34 收到显式停止并结束 reader, 约三秒后写出 unconfirmed 且 completed 为空; 四次应用停止请求各八秒超时。源码要求匹配的 hook_detached/released_all 真实确认, 活 Helper 未确认前继续等待, 不能通过清空回执或超时假定释放。Agent 是否收到停止、为何未确认仍未知。
- 当前安装文件仍为 source revision `4a42df15` 的 dirty/local 旧候选, fingerprint `8a3cf279`、EXE SHA256 `7521b5bd0660cf3e369e125182aa5c40cb9a721d37bb700a77130884ab04c9b3`。安装字节没有音频静默收尾的三项特征字符串, 不将新工作树已有的八秒 watchdog 当成当前运行证明。
- 调查期间旧实例退出, 新实例以同一旧文件启动, audio generation 0 中断成功且没有新语音 START。此事实不证明旧系统合成和弦已释放, 不把新实例状态反推成旧 generation 66 已正常结束。

候选继续沿用上游八秒静默收尾及原快捷键 UP/音频中断链。额外审查发现其仅在 recv_timeout 返回超时时执行, 持续排队消息可以推迟到期检查; 确定性用例得到 1 passed / 1 failed。但单纯到期优先会在队列中有效新音频尚未消费时误判静默, 每次公平处理一条也不足以证明安全。当前没有独立且带连接代次的最新音频到达观察, 因此本轮不采用该修法, 已精确撤回全部实验源码与测试, 不修改现有 BLE 策略或时限。保留最小失败日志 `target/dev/capture-state/ble-deadline-red.log`; 未完成的新鲜音频反证编译不计通过, 不据静态缺口归因旧现场。

Helper 补停止提交、等待、匹配与终态的脱敏阶段日志, 保留原成功条件。阶段日志测试先得到 0 passed / 3 failed, 慢日志恢复测试先得到 0 passed / 1 failed; 最终 cooperative_cleanup_tests 为 25 passed, recovery_tests 为 7 passed / 2 原有 ignored, rustfmt 与独立审查 passed。恢复路径复用现有有界非阻塞日志队列, 匹配 ACK 后先持久化终态回执再排空日志; 慢 sink 测试证明日志写入未解除阻塞时真实终态已写出。submitted 只表示本地写入完成, 不冒认 Agent 已收到; 保留三秒未确认提示、真实 ACK 成功条件与安全等待, 不通过日志改动宣称旧 Helper 已恢复。

旧 generation 66 最后实际 PCM 时刻、源 STOP 缺失原因及 Agent 为何未确认仍未知。16:45 用户确认无线麦和输入法均不再显示录音; 随后只读复核仍是同一 Helper, 回执停在 15:14:37 的 unconfirmed, completed 为空。用户可见录音结束不等于旧按键组件已确认释放, 不能据此覆盖运行组件。新包、真实设备释放、旧残留与双型号结果按实际验收更新, 原 G1/G2 不据此完成。

18:05 用户要求重新安装后, 对 SHA256 `18dc24defe4127aafe9e9e7fee9b9cd52fdb9a948136fa4165b2aca576375622` 的包执行一次同目录管理员静默安装。应用收到正常退出信号, input_quiesce passed; enhanced_capture_release 八秒后 failed 并保留恢复记录, 随后进入 BLE 断连。安装器先在 app_exit 门禁超时, 退出码 1639; 当时三个产品载荷及六个指定配置位置的存在性/哈希均与执行前一致。主程序随后自行退出, 未强杀。再单独执行原 45 秒清理门禁, 退出码 15、phase=waiting_existing/result=unconfirmed; 旧 Helper 仍未给出释放终态, 因此没有重试覆盖安装、运行新 Helper 或启动新候选。保留本主题 `reinstall-result.json` 与 `reinstall-cleanup.log`; 本次重装为 blocked, 不是安装成功或硬件通过。

## 2026-10-26 当前语音不跟手与连续会话只读定位

用户报告有时按两三次仍不出字。本轮没有重启、安装、录音、输入注入或更改配置; 旧历史“重启完成”不作为本轮事实。14:30只读仍为App57624及原创建代次、Helper73548; 实装App SHA256 `7521B5BD0660CF3E369E125182AA5C40CB9A721D37BB700A77130884AB04C9B3`与10月25日闲置首用通过候选相同, 昨晚诊断工具变化不是App升级。当前全按键设置true、微信/hold/Ctrl+Win、临时默认麦克风切换true。14:37公开PnP精确查询两条已配置音频端点均为AudioEndpoint/OK; 这只证明端点当前存在与状态, 不替代当前同适配器拓扑、微信实际选择或收音/出字验收。

- 北京时间14:16:03至28的三个连续会话generation347/348/349: 软件观察到ATVV开始至chord成功分别575/898/212ms, 不是物理按键时刻测量。Capture准备确认376/302/89ms; 第一次prepare_io中roles读回累计254ms、policy调用48ms, 第二次203/24ms。第二次开始前上一会话restore仍在进行, restore总341ms。三次IME均`already_active`(14/54/7ms), 因而本次不支持“冷切换50ms读回失败”归因, 也没有同事件菜单焦点恢复失败证据。
- 三次audio_flow从各自音频会话起点至首次提交666/895/215ms; accepted=submitted分别26880/46080/292320, queued=0且voice_flow decode_errors=0。第一会话700ms响应检查`reacted=false`, 后两次marker=true; marker/PCM提交都不等于识别成功或文字上屏, 不读取语音内容或第三方内部状态。第一会话恢复动作后旧epoch重试因session_replaced退出, 没有凭此证明所有快捷键残留或来源干扰均已排除。
- 11:19:25.416软件观察到下一次start generation328时, 上一generation327恢复仍在串行worker中, 至26.676完成, restore1284ms(其中一次角色setter391ms)。随后328 prepare立即cancelled、voice_flow abort1337ms且未进入IME/chord。10:46 generation295也在prepare阶段cancelled并abort160ms。`prepare_io result=failed`是汇总结果, 两笔均伴随明确cancelled, 不是已证麦克风API失败; 现字段不能单独区分guard释放/epoch/绝对deadline的取消来源。
- 代码契约: Capture manager已有generation/epoch、来源guard与1500ms绝对截止, UP可提前标记取消, 但不能中断正在执行的同步COM恢复; 单worker必须恢复/让出旧默认路由后才处理新Begin。BLE worker同步等待此准备, 然后才对齐IME/注入快捷键/消费排队PCM。上述现场证实这个架构有可感知串行成本, 不是缺少取消机制, 也不能通过删恢复、并发setter或延长截止安全修复。

10:00至14:21现存日志投影82个voice_attempt、19个reacted=false、ownership_timeout分类0; 这不是19次用户确认失败或完整无故障证明。必要脱敏材料在`target/dev/foreground-input/voice-20261026-events.json`(SHA256 `C77FEFBD4E4FB0AF0980DE3BFBBA1B784098FFB7FB2A0D60425CBC16420D698A`)及`voice-20261026-detail-fixed.json`(`B5292C71AB4D6C7C55588413780475FB0BE318823490CD91E57B625FBB00CF7A`); 首次补充提取为空的文件保留, 不当阴性证据。

默认路由写入是个人版既有明确受限授权的未文档化IPolicyConfig ABI, MMDevice只负责公开读回; 这是长期兼容性代价, 本次并没有证明ABI失效或setter不兼容。不能把cancelled说成setter失败, 也不能仅凭当前源码断定近期安装才引入此功能。

用户随后明确切换麦克风更重要、可以接受必要延迟, 本轮保留临时切麦与现工作流, 撤销“先确认固定mic才继续”的人为前提。成功条件分开: A准备期间来源UP/取消不等于COM失败; B持续按住且PCM/和弦存在但没有观察到下游反应, 仍须查恢复和采集就绪; C已有marker/mic反应但真实文字结果未知。切麦耗时只解释跟手成本, 不直接解释所有不出字。直接PCM本地ASR和固定mic仅保留研究历史, 不作为当前必需操作或已选实施。

15:31至15:33 fresh只读仍为原App/Helper代次, App文件SHA未变, capture/临时切麦均true, wechat/hold。新近15:21 generation376/session20已确认三role目标mask7后发chord(41.699), 42.405判reacted=false、42.479实际profile cycle返回ok, 44.666同held内chord_retry成功, 44.804才观察到UP, 44.892释放; 本会话decoded56160、decode_errors0。这证明当前恢复机制确有输入副作用, 不证明它造成了这次无字, 用户也未提供该次文字结果。最新14:16补充与15:00后必要事件归`voice-20261026-stability.json`, 仅匿名计数/时序, 不采音频或文字。

静态边界: Capture单worker先finish旧lease再取Begin, active时拒新Begin; end_epoch在BLE释放和弦后才允许恢复, 未见正常流程旧restore并发覆盖已建立新路由的证据。微信Ctrl+Win不进入voice_verify的focus-heal, 独立wetype_check却在700ms判据后直接执行双profile cycle。该cycle没有session/target ticket; 500ms等待超时也不取消STA, 两次Activate之间没有失效检查。Microsoft文档明确`TF_IPPMF_FORSESSION`影响当前desktop所有线程, 不是仅前台应用; 本进程锁/读回不提供外部选择的CAS或在途COM取消。

经授权只抽取原production两次Activate/原settle的等价私有`cycle_profile_actions`测试seam, 不改变操作顺序/错误短路/运行行为。真实channel barrier在alternative已提交后暂停, 测试模拟外部新选择再放行, 原函数无条件恢复WeType覆盖新选择: 预期contract测试RED(2项等价/失败短路passed、1项failed)。这是记录型TSF边界反例, 没有调用真实TSF、改变输入法或证明14:16现场发生该竞态; 不覆盖500ms超时/前台变化的真实API验收。既有IME匹配测试另跑44 passed/1 ignored(过滤器也匹配macro_runtime名称), 排除这项故意保留的RED; fmt通过。未解决候选不提交、不安装。

最小行为候选是退役held内双profile cycle及对应UP/DOWN重注入, 保留正常按下前IME对齐与临时切麦; 代价是失去这段自动唤醒重试, 不能预称成功率提升。另一方案若仅补ticket, 第一步alt已生效后仍不能同时保证不覆盖外部新选择和完整原子回滚, 因而本轮不提交假安全修法。当前只给边界/RED供决策, 不调80ms/50ms/lease/priority、不追加重试或长采样; G1 failed/G2未解决保持。

## 2026-10-25 G2 公开键盘栈与双设备只读入口准备

用户确认G1保留failed且停止重复采样后先推进G2离线来源切片, 不安装驱动、不改变当前接管。Windows build26200本次11个present Keyboard中有1个既有RC003型号硬件ID候选; PnPUtil XML对同一完整实例标识读回effective stack顺序`kbdclass/kbdhid/mshidumdf`。声明父链为Keyboard/kbdhid、HIDClass/mshidumdf(声明LowerFilter WUDFRd)、Bluetooth/BthLEEnum两级; 与实际栈分栏, 不把注册声明当附着顺序。CompatibleIds后续只读查询为空, 两个标准兼容ID字段false只表示未观察到这些值, 不据此否定Keyboard类或伪造HID描述符证据。型号唯一并非用户所选遥控器绑定证明, 没有注册监听或打开HID输入。

旧inventory文本regex遗漏实际结构, 原failed保留。新工具身份字段负例先证实任意Extra属性可冒充匹配, 修为唯一InstanceId后通过; 首次真实ordinal大小写比较拒绝也保留, 后续只读证实同实例差异仅大小写, 补RED并改完整OrdinalIgnoreCase, 最终8项解析/隐私测试passed。证据在`target/dev/tv-keyboard-source/g2-stack-*`, 不保存原始设备路径、地址或实例值。

既有Raw Input候选工具通过Windows的hDevice/interface/CM devnode链关联, 原Target实际只是model匹配。新增两项真实RED分别是错误target标签及设备变化后仍允许下一次读; 最小修正为model_candidate及立即停止。再将原枚举先于注册的顺序提取到实际调用边界, 顺序与枚举失败清理标志两项RED后改为先注册DEVNOTIFY再建图; 摘要改为注销后输出, 不在监听期间同步写stdout。11项example测试、build、selftest(registration_calls=0)及fmt passed, 独立源码/固定hash入口的隐私与生命周期窄审passed, 没有运行真实capture。私有closure测试只证明顺序/错误传播, 不冒称Win32注册或卸载失败已实机验证。

本轮filter的10个输入及未签名SYS与原指纹`13f78071974cbc2837d73982640a96059e6e89b689b4a099c8470f04c5710f33`完全一致, 复用已通过的WDK编译和真实driver.c离线harness, 不重复构建。当前SDK/WDK存在, 但PoC仍先原样透传再观察已消费prefix, 没有INF、捕获消费者或真实TV扫描码; 不把synthetic scan29/observer.Held当来源授权或吞键配对账本。下一现场仅为用户配合两次最多30秒TV/实体反引号按放的候选观察, 需用户正常关闭全按键支持并确认释放, 避免既有Agent遮住原始输入; 不自动开关。具体入口/隐私/正常停止条件归[候选README](../drivers/SayAllTvKeyboard/README.md#只读双设备候选观察入口)。G1持续性、G2内核绑定/抑制/TV到Tab与跨机验收均未完成, 本轮不提交未验收生产能力。

用户确认关闭接管后, 执行前发现原入口仅在注销后输出, 无法及时证明实际ready, 因此尚未启动监听而停止原调用。补最小公开命名event, 注册/readback/来源表完成后才SetEvent, 不使用监听期间stdout; 两项缺ready顺序/错误传播RED后15项测试passed, 含真实跨进程event且无RawInput。原“入口准备通过”只表示当时离线隐私/生命周期检查, 不代表已能协调现场30秒窗口。原30秒与正常清理不变, 未自动关闭或恢复接管。

22:49本次唯一TV现场观察实际注册/readback/匿名来源表成功并发ready, 最终设备变化通知使工具按契约提前停止; 本次未记录通知处理与ready的精确先后。最终`device_changed`, candidate为空、所有输入计数0、session_error为空、注销读回/其余cleanup无错误; 不能凭exit0认定TV已观察。工具poll一次返回ready和已退出终局, 未发送迟到按键提示, 未启动第二步或自动重试。通知是初始arrival还是真实换代未被本次字段区分, 保留unknown而不忽略通知。当前真实API正常注册/注销路径有实证, TV/反引号来源仍未取得; 需要用户正常恢复原接管, 工具未自动切换配置。

本轮最后离线补丁只给上述终止增加首通知诊断: 公开arrival/removal/unknown类型、现有匿名来源表的model_candidate/other/unknown及ordinal、处理相对时刻。两项输出缺失RED后17项example测试/build/selftest/fmt passed; 所有通知仍立即停读, 不重建/忽略/延长窗口, 无新增现场观察。现有失败无法追补类型, 官方也未承诺首次arrival仅是注册回放, 因此没有已证明可安全取消该终止门禁的修法。

## 2026-10-23 G1 续约同轮与控制写失败诊断

- 2026-10-25 最后一次有终点的4小时自然故障观察于17:13:42.390开始, 21:13:42.568附近请求停止; 同App57624/Helper65452代次, 未重启、构建、增加压力负载或改产品参数。仅本地工具将时长/系统采样容量上限3600/3602改为14400/14402, 两项真实边界RED到GREEN、46项原fixture/4项报告检查及独立审查通过; 频率、65536行ring、30秒前窗/5秒后窗、0.25核/256MiB和原收尾门槛未变。容器分配离线约3.03MB, 最大报告序列化离线0.749秒, 均不替代现场资源验证。
  - 终局not_captured/DecisionFields=null/CSV0, Complete=false保留两个独立限制: 三次日志轮转令触发通知覆盖unknown_after_replacement; 总14418.311秒超过14400+15秒收尾限3.311秒。stop前14400.196秒, 1426预算段峰0.136510核/当前工作集峰122.441MiB, 13205系统点最大间隔5405.165ms、末点距stop713.710ms。系统CPU峰100%是1029.163ms区间, 最低可用RAM2.6680/31.7307GiB、另一点提交峰53.6180/81.7307GiB, 不拼接极值归因或排除分页。loss/parse/overflow0、同代alive、STOP/join/dispose及独立精确QUERY4201均成立; 这些有限事实不将unknown改为稳定passed。
  - 终局现存四段同代日志90638条中, ownership_timeout/ownership_resumed/rx_silence/LOG-QUEUE overflow均0; reset_queued=false或continuity_reset_failed及字面engine_unavailable分类未命中, 不扩大为所有可能失败均不存在。voice_flow control_stop193笔/decode_error0只证明记录终结, 不证明193次识别成功。2863笔节流renew记录中有一笔20:29:33.752的tick26345/conn1, wait/gap886ms、lock/write0、state=ok; 采用终局这一慢轮而非中期零值, 它未达到1500ms且没有保留对应ETW故障包络, 不推断Ready或timer原因。g1-four-hour-window result/cleanup/log-summary与slow-round为必要证据; 本轮结束后暂停同条件自动采样, 无新运行操作或提交。
  - G1尚缺23:15同候选有效样本间隔1797ms的同轮端到端因果证据: Helper等待、App桥接消费与Agent读取/生成所在环节未全部闭合, 已安装的可读fresh O安全交接修复不覆盖当时probe=0的情况。当前没有新的生产失败回归支持仅凭猜测改超时、优先级或管道架构。后续需真实复发的准确时刻/现象及现有日志, 或用户明确调整目标与诊断路线; 不以等待遥控回忆或反复长窗冒充修复。另已有部署证据仅绑定宿主11880代次、auth-lease HELLO标签及磁盘/包内容, 未保存实际Script加载内容hash回执; 驻留Agent JS字节身份为unknown, 不是已证不一致, 不读取私有内存或为核对强制重载。G1 failed、G2反引号来源隔离未解决及完整验收缺口均保留。

- 2026-10-25 16:48至16:59补做一次有区别的正常重开早期观察, 不是重复构建或压力负载。先在旧实例运行时完成工具加载及固定源码预检, 再以真实语音UP/terminal/chord闭合门禁请求正常退出; 本代STOP conn1/receipt、任务idle、进程与端点释放、六配置保持后, 由Explorer普通权限重开同一安装字节。新App57624于16:48:52.134创建, Helper65452/renew63896及两个桥接线程均核原生代次, NORMAL/Task策略/原Agent HELLO/实际owned/配置读回passed。采样Start为16:49:02.828, 启动至采样10.694秒不在ETW覆盖内, 也不宣称刚开始已具有完整旧sample前史。
  - 唯一600秒窗口stop前600.114秒/总602.435秒含清理, not_captured/Complete=true/DecisionFields=null/CSV0, 通知投递unknown。59预算段峰0.158178核、当前工作集峰125.527MiB, 552系统点CPU峰98.4446%, 最低可用RAM3.8963/31.7307GiB、提交峰48.8561/81.7307GiB; 不假设极值同时或据高CPU/内存占用归因。loss/parse/overflow0、同代alive、STOP/join/dispose及独立QUERY4201成立。启动至stop现存同代日志701条(采前247条), 初始ownership_resumed一笔, timeout/rx_silence/overflow/continuity失败均0; 已记录renew121轮最大wait/gap均534ms, 不将节流日志说成每轮无遗漏。证据g1-precompiled-restart的restart/runtime/window/cleanup/log-summary保留, 本次正常重开及工具观察passed仅限各自边界; 23:15同候选真实撤权仍未解释, G1 failed与原完整验收门槛不变, 不自动重采。

- 2026-10-25 16:28:53.637启动同一App/Helper代次的唯一10分钟闲置首用观察, 未重启、构建或制造负载。用户确认本次Menu开关、Back单按、TV Tab及两次语音均正常, 实体反引号仍异常; 仅记本次硬件用例passed, 不覆盖23:15已有失败或G2边界。16:38:31菜单generation72打开, 36.815记录hide_completed的迟到focus忽略; 不据此新增自动焦点恢复通过结论。窗内日志有Back Single三笔、TV Single三笔, 两次语音session24/25分别有ATVV DOWN/UP、chord_release ok与flow generation20/21 control_stop, 不把多笔单击记录解释为用户单按连发。相应五分钟现存日志timeout/resumed/rx_silence/overflow均0, 投递完整性仍unknown。
  - 观察stop前600.114秒, 总615.348秒, 超过既定600+15秒收尾预算348ms, 因此原Complete=false/工具unknown保留; 未捕获故障, 不放宽门槛或重采。59预算段峰0.120167核/当前工作集峰122.730MiB, 553系统点; loss/parse/overflow0、同三角色代次alive、STOP/join/dispose与独立精确QUERY4201成立。该收尾时限不满足不是产品新故障, 也不抹除用户本次实际成功。证据g1-precompiled-idle-first-window result/cleanup及user-detail; G1仍failed, 四小时连续验收及各三轮生命周期未完成。先前15:34至16:19窗现存日志离线对账同代App3155条/Helper823条, timeout/resumed/rx_silence/overflow均0, 未发现已落盘故障而触发器漏报的证据, 仍不能证明未落盘事件不存在。

- 2026-10-25 本地取证工具冷启动开销取得可操作证据, 不改产品。15:01首10.006秒CPU0.346676核超过原0.25停止线, 同窗全进程JIT增加2927.14ms/5056方法, GC暂停10.604ms; 正常清理并独立QUERY4201, 原结果仍unknown。公开计数不能归属模块或与CPU相减。四独立冷进程的普通日志/严格故障解析/managed分发/预算构造仅增加23/48/36/24方法, 不支持Regex优化或把离线结果相加解释实窗。随后同wrapper准备后完全不调用Run/ProcessTrace, 仅等待10秒仍有CPU0.288510核和JIT2829.83ms/4533方法, 证明消费启动不是这些宿主活动的必要条件。
  - 将仅本地工具的consumer/tail/原角色探针逐字同源离线编译为托管DLL, 固定源、依赖与产物hash后加载, 不增加产品DLL依赖、不改JIT设置、不预热或移动预算起点。旧加载器实际载入2个compiler assembly的RED, 新加载器0个GREEN; 新加载路径46项fixture/4项报告检查及独立review通过。相同无trace对照CPU降至0.060894核/JIT553.14ms/1266方法; 非随机化对照, 不精准分摊CPU或认定产品故障原因。15:30:14唯一45秒真实路径stop前45.066秒/总59.140秒含清理, 首段CPU0.129364核、其余三段0.02184至0.02495核, 峰工作集115.773MiB, 原0.25核/256MiB门槛不变。42系统点CPU峰64.86%, 同一点最低可用RAM4.5825/31.7307GiB(14.44%)和提交峰47.8143/81.7307GiB(58.50%); 没有故障包络, 不作因果解释。not_captured/FaultCaptured=false/CSV0、通知投递unknown, loss/parse/overflow0、同代alive、STOP/join/dispose及独立QUERY4201成立。必要证据为g1-host-idle-control、g1-precompiled-idle-control与g1-precompiled-window结果, 实际tool切片通过不等于G1稳定; 原产品failed与反引号未修保持。

  - 同源预编译工具随后只执行获准的唯一45分钟自然窗: 北京15:34:05.537起, stop前2700.050秒/总2713.635秒含清理, 没有重启App、构建或额外压力负载。268预算段峰0.151070核/工作集峰126.383MiB, 2487系统点CPU峰96.923%(1006.411ms区间), 最低可用RAM4.2150/31.7307GiB(13.28%); 另一点提交峰48.7361/81.7307GiB(59.63%), 极值非同时。最大采样间隔1160.668ms, 末点距停止107.116ms。not_captured/DecisionFields=null/CSV0, 通知投递仍unknown, 无观察到的文件切换; loss/parse/overflow0、同三角色代次alive、STOP/join/dispose及独立QUERY4201成立。g1-precompiled-fault-window result/cleanup保留; 本轮证明工具在该窗口内符合原资源门槛, 未取得产品故障调度包络, 不从CPU峰或内存比例归因/排除失效, 不当作G1或4小时验收通过, 不自动重采。

- 2026-10-25 用户要求继续并说明电脑一直有工作负载, 昨晚可能使用过遥控器但记不清, 不再把该记忆作为执行阻断。14:37完成唯一一次定向正常退出重开: 本代语音真实UP/terminal/chord闭合, conn2新STOP/receipt、任务idle、App/Helper/端点释放及六配置通过后, Explorer普通用户启动同一实装App; 新App67048/Helper17960与新三角色native代次、NORMAL、原host/Agent HELLO、配置均实际核验, 没有安装或调整参数。首次组合运行工具因runtime探针与tail同名G1RenewIdentity导致Add-Type失败, 尚未调用StartTrace; 真实离线RED复现TYPE_ALREADY_EXISTS, 仅本地runtime类命名隔离后顺序加载GREEN并独立review通过。原失败保留, 未再次重开App。14:37:08至14:41:10现有日志未观察timeout/rx_silence/renew慢轮, 不等同ETW覆盖完整启动。
  - 此后唯一实际观察于14:43:24开始, 首10.0751183秒observer CPU2843.75ms=0.282255核, 超过用户允许的0.25停止线, 正常提前停止为observer_budget/unknown; 请求600秒不代表实际观察10分钟。stop前10.090秒、总23.345秒含清理, WS198.668MiB未超限。FaultCaptured=false、决定字段null、CSV0, TriggerLogCoverage=delivery_unknown; loss/parse/overflow0、同代alive、STOP/join/dispose及独立精确QUERY4201通过。10个系统点的CPU峰67.26%, 最低可用物理内存3.6281/31.7307GiB(11.43%); 另一点提交峰47.1269/81.7307GiB(57.66%), 极值非同时, 不证明分页或故障因果。证据g1-restart-directed的restart/runtime结果及window result/cleanup保留; 不放宽预算、不重采, 正常重开通过与故障窗口未取得分开。G1仍failed, 未启动4小时验收, 用户本轮硬件反馈尚待, 反引号未修。

- 2026-10-25 用户确认01:21至02:06采样期间确实在进行有负载的工作; 尚未确认该期间是否实际使用遥控器、是否出现失效。因此下述not_captured仍仅为未捕获故障, 不改记稳定passed。下一步优先在可安全中断使用时做一次正常退出重开或全按键关闭再开启的定向场景, 用同代ETW与系统资源捕获单个实际故障; 当前候选确有启动后timeout证据, 但不认定启动为唯一原因。本轮仅记录计划, 未执行启停或新trace; 不无限重复无fault观察、不增加lease、不改优先级, 也不凭Helper电源策略由系统管理就盲改生产。G1仍failed, 实体反引号仍未修复; 完整验收和串行依赖归[TODO](../TODO.md#全按键接管与键盘来源隔离)。

- 2026-10-25 新通知/决定时间候选独立review passed后, 只执行获准的45秒短窗与唯一最长45分钟自然窗, 未制造负载或修改产品。01:19:08短窗stop前45.148秒/总48.568秒, 4预算段峰0.148233核/WS165.820MiB, 42系统点及真实Tick锚点正常; 01:21:41至02:06:41长窗stop前2700.098秒/总2713.422秒, 268段峰0.119987核/171.066MiB, 2482系统点, 最大实际采样间隔1214.753ms。两窗均not_captured、FaultCaptured=false、CSV0, 无观察到的文件切换但TriggerLogCoverage仍delivery_unknown; Complete=true只表示工具观测/清理条件成立, 不证明无产品故障或G1稳定。原三角色代次alive、loss/parse/overflow0、STOP/join/dispose与独立精确QUERY4201均成立。长窗系统CPU峰99.42%(约1000.524ms区间), 最低可用物理内存5.8763/31.7307GiB(18.52%); 另一采样点提交峰44.2580/81.7307GiB(54.15%), 极值并非同时。末系统点距停止545.163ms, 不声称覆盖清理阶段。没有实际fault的Tick包络或Ready链, 不能以CPU峰推断或排除负载相关性, 也不能把提交比例当paging实证。冻结g1-decision-short-window与g1-decision-fault-window的result/cleanup-result; 不自动重采, G1 failed保持, 新单事件捕获分支实机仍deferred。

- 同轮日志轮转工具修正先取得真实跨进程late append/close后的log_replaced RED, 单轮接续及10项边界曾GREEN; 独立review随后提出“中间B轮转因UTC回拨排序被prune”的遗漏风险, 真实文件反例再次RED。保留两组证据, 不把有限归档快照当无损证明; 未获准运行该候选。经明确收敛改为“通知可能漏报, 单个实际故障独立取证”: 删除归档无损算法, 新base双ID核对后从0读取、丢旧partial且显式TriggerLogCoverage=unknown_after_replacement, 计数仅是观察到的切换而非实际轮转总数。无故障且发生缺口不Complete, 不用于全窗故障数或4小时稳定验收。单事件严格读取真实decision_tick_ms、旧sample/age及conn/epoch/Oseq/target字段, 缺失/重复/年龄不符拒绝; 原UTC-only接受缺字段取得业务RED。GetTickCount64与QPC前后夹读, 严格前后tick包络旧sample至decision, 相等tick仅保留一个pending等原循环后续锚点, 不阻塞、不延截止; UTC只作通知显示和原时钟异常检查。最终要求三个role的实际ring记录和系统资源点覆盖该区间及5秒后窗; 否则仍unknown。44项consumer、5项真实文件/跨进程通知、11项原watcher、4项报告检查及后续独立review passed; 获准两次实机窗口结果见前条, 单fault捕获仍deferred, 不改变产品G1 failed结论。

- 2026-10-25 用户明确允许诊断工具停止线从0.15改为0.25单逻辑核, 并要求同故障窗核对系统CPU/内存; 该授权不改变产品优先级、lease或超时, 旧超限证据仍为unknown。工具复用现有约1秒controller周期, QPC夹住GetSystemTimes/K32GetPerformanceInfo, 单processor group、首次CPU基线null、原始计数和实际间隔、最多3602条; 查询成本仍计入原预算, WS上限256MiB不变。38项离线与4项报告检查及独立review passed。00:22:11起唯一45秒短窗工具完整, 无故障/CSV0, observer峰0.2074005核、WS169.527MiB, 系统CPU峰89.19%; 最低可用物理内存4.3569/31.7307GiB(13.73%), 另一个采样点提交峰44.5827/81.7307GiB(54.55%)。总57.433秒含清理, 末系统采样距停止约865.65ms, loss/parse/overflow0、同代alive、独立own-session QUERY4201。高CPU与无故障并存不排除负载关联, 内存占用不证明paging; 本工具CPU也不含全系统ETW成本。
- 随后00:25:29起唯一最长60分钟自然观察在461.318秒停止, 总462.562秒: 项目正常10MiB日志轮转触发log_replaced, Complete=false/FaultCaptured=false/CSV0, 本窗unknown且不自动重采。427系统点中CPU峰100%(实际区间700.575ms), 最长采样间隔1654.928ms; 最低可用物理内存4.4708/31.7307GiB(14.09%), 另一个点提交峰44.1879/81.7307GiB(54.06%), observer峰0.1351415核/167.7734MiB未越预算。无故障锚点, 不能据这些全窗极值归因; loss/parse/overflow0、同代alive、STOP/join/dispose与独立QUERY4201成立。原证据g1-system-resource-short-window及g1-system-resource-fault-window的result/cleanup-result保留。只读轮转元数据与项目rename/prune实现吻合, 但旧held file ID未在失败窗落盘, 不追认跨文件无遗漏; 后续仅本地读取器接续试验, 不修改产品logger或把该窗改成通过。G1 failed不变。

- 随后复用固定TraceEvent 3.2.8的离线fixture对照managed Lookup/Fixup/Dispatch与collector回调, 未运行真实trace: 同进程各5批75万事件/2:1分布/3万事件预热, 固定record/payload生命周期和每次CSwitch真实fixup、最终计数及零目标行断言通过, 32项回归与独立review passed。中位墙钟分别31.7691/34.4603ms, CPU总203.125/171.875ms带15.625ms量化, 两类均0分配; 顺序运行且明显波动, 不据反向墙钟认定框架免费/更快, 也不能可靠量化框架份额或支持“框架是大头”。该对照不包含RawDispatch/native/realtime成本; 固定证据g1-managed-dispatch-benchmark.json, 不据此重采、改预算或重写parser。

- 23:44:20起唯一短窗预算校验仍为unknown, 未修改产品或放宽预算: 本地工具仅把每事件schema字典计数改为4个固定计数, 并补同一预算区间的QPC/EventsSeen起止与启动累计CPU快照。离线真实Dispatch/相邻区间/报告32+4项passed; 回调微基准55.0593→29.2879ms/75万事件只证明局部成本减少, 两版分配均0, 不能外推整体ETW成本。实机首10091.4466ms CPU1718.75ms=0.1703175单核仍超0.15, 同次顺序采样EventsSeen0→1665285约165019events/s; 非原子快照不用于逐事件精确对齐。消费线程启动前累计CPU125ms/单调120.7711ms, 即仅作解释性差分, 后续区间仍约0.159844核, 不能把超限全归启动; worker内JIT与dispatch成本尚未分别测得, 实际预算没有扣除启动。WS169.098MiB未越界, FaultCaptured=false/CSV0, 没有新的故障调度证据。总22.417s含约12.311s停后清理, STOP0/ProcessSuccess/join/dispose/原代次alive、loss/overflow/parse0; 23:45:05独立精确own name/GUID QUERY4201确认清理。原证据g1-resource-interval-short-window-result/cleanup-result, 不重采、不调阈值, G1既有failed不变。

- 23:21:52–23:22:17新代次唯一三角色故障环为unknown, 非产品新失败: 固定工具/30秒前窗+5秒尾窗/65536行/原隐私及清理契约未变, 没有附加构建或改产品。首10.095547秒observer CPU1765.625ms, 等效0.174891单逻辑核, 超过既定0.15预算后主动收口; WS169.613MiB未越256MiB。FaultCaptured=false、故障锚点为空、CSV0, 没有取得故障时timer/Ready分解; 总24.826秒含后续清理, 不拿总均值覆盖活跃首段超限。loss/overflow/parse0, STOP0/Process返回/join/dispose/三role代次仍alive, 23:22:52独立精确own name/GUID QUERY4201确认无残留。冻结结果g1-buffered-ownership-fault-window-result.json及cleanup-result; 不放宽预算或自动重采, G1既有failed不变。

- 2026-10-24 23:14–23:16新候选实际部署passed但持续性failed: 用户明确“可以安装”后, fresh语音UP/terminal/chord真实闭合, 正常退出与同代STOP conn1/回执、任务idle/进程端点清空及安装前后六配置哈希成立。setup99D3F17A对应实装App SHA256 `7521b5bd0660cf3e369e125182aa5c40cb9a721d37bb700a77130884ab04c9b3`, 新普通Explorer App45452实际加载8a3cf279指纹; Helper53412/renew9808仍34F5/NORMAL32/相对0, TaskPriority4及原host11880/auth-lease/Agent/Gadget不变。首次普通观察者无法读取提升Helper路径, 未据此重新安装; 提权只读核验身份一致。runtime原config=false保留, 随后六文件逐项读回均恢复基线, 未写回配置且未捕捉早期差异文件, 不推测原因。
- 同一新代次23:15:29.838真实故障: probe bytes=0/complete_line=false, pending/buffer=0; conn1/epoch7/gen2的旧O91 sample117707531, decision117709031, age1500, current_rx_seq90, 实际available=false。30.133 Helper renew87的wait/gap1795ms、lock/write0; 30.143 bridge单慢轮idle/maxround1607ms、其余phase maxima0。30.145同代新O94(rx91) sample=App消费117709328、age0恢复owned7167。该窗无handoff/failed事件, 预读当时没有取得fresh完整报文, 不是已证明的可读队列误撤权实例; 仍有Helper等待区间迟到, 没有故障ETW则不能进一步认定Ready排队/计时/系统暂停或Agent消费原因。21秒固定窗证据为g1-buffered-ownership-first-fault/detail; 原无故障时窗和本地GREEN保留, 不启动4小时通过统计或让用户继续硬件验收, 不改超时/自动重启。

- 2026-10-24 真实 IPC characterization 已证明一条可达读序缺陷, 不等于历史故障唯一归因: 仅暂停新建测试 peer, 旧O91已实际接受; Windows server PeekNamedPipe确认完整O95的31字节已可读。旧sample108180437、新sample108181937, 释放/判定/消费tick均108181937, 旧age1500/新age0, 原代码仍timeout且映射available false→true。正确预期取得业务RED; 同主题 `g1-buffered-ownership-characterization-configured.log` 和 `g1-buffered-ownership-red.log` 保留实证, 首次0tests与未配置映射的夹具失败不算产品证据。
- 21:22本地候选构建passed, 尚未安装: setup SHA256 `99d3f17acff310d6ae8c30a2aefd64d5f57026094804b676e968ad4b103833f2`, dirty/local/included指纹 `8a3cf279ebaf22384865178f968bba83106bc2744a318ed746b5123d1e0a9385`; 前端嵌入及个人更新签名验签通过, Helper/Agent/Gadget字节未变, 原3C329包独立保留。21:22:55安装预检仍是原App/Helper/host代次和Normal任务; 本代最新21:22:50语音DOWN、attempt及chord尚无配对释放, 门禁safe=false。未请求退出/安装, 保持当前使用并等待用户释放确认; 预检退出码0只表示读取完成, 不是允许安装或功能通过。证据为同主题g1-buffered-ownership-package-result/preflight, 不以旧“已松开”回答代替新会话状态。
- 最小候选实现与自动化边界 (部署及后续failed见本节首条): 只在候选过期时以已有4096字节总预算消费BufReader现成字节及最多一次PIPE_NOWAIT读取, 按原FIFO/parser验证后再判定, 不等待半行/无限drain/调整1500ms。仅同conn/epoch/target且完整有效的新fresh O可安全交接: 同一sender有序CaptureContinuityReset取消旧capture手势/repeat, 保持配置/监听/available, 再释放旧UP并推进新O fence; claimed面板UP仍配对。重叠同语义键的严格来源隔离仍归G2; TCP空buffer内核队列不在本切片探测范围。取消事件投递失败明确engine_closed收尾; 满预算无换行明确line_too_long。真实pipe已改接runtime.sender和记录型injector, 验证旧持键取消无动作、旧E拒绝、新E一次tap和无false; bridge47项、取消2项及最终关闭原因断言通过。无换行/receiver关闭在测试清理前已非Connected, 且真实closed原因分别为line_too_long/engine_closed; 追加断言是GREEN补强, 没有追加mutation RED。冻结bridge `c78cac41d8981862bfec0da0d6f8bcf4cf9ccc63c5a7927d416e1f16637959ce`、mapping `ebab0581ac67c7e90caa471429b8b8cd69aaa4a6737b122751e4e87c4ad70372` 的独立review和完整Rust矩阵passed: core61+4、Windows626/21 ignored、App189/1 ignored及集成/子入口, fmt、workspace check、runtime-simulation check均通过, 日志前缀g1-buffered-ownership。固定预算夹具曾将8192字节同步写入4096管道同时持reader锁而死锁, 仅Ctrl+C结束自有测试并核无残留; 改4096输入后预算16/再次0实测通过, 原interrupted日志保留。16:02持续性failed和18:17语音首用failed不变, 不用本地自动化替代新候选4小时及硬件验收。

- 新App诊断代次的单次自然故障观察已在19:52收口为not_captured: UTC10:52:27.313759起, 计划11:52:27.313759截止, 实际STOP前3600.091s、清理另14.055s, 不宣称硬60分钟返回。公开API唯一确认新三角色并由consumer持有原生身份; 仅更新既有工具代次/prefix, 未构建、造负载或改产品。358个10s区间最大0.086567单逻辑核、当前工作集166.832MiB/进程峰170.020MiB; 预算0.15核/256MiB未越界, 本进程成本不代表系统ETW总成本。ETW387813543事件、目标5015536行尝试、年龄淘汰4974114、剩41422行跨度29.997915s; 无触发故CSV0, 不保存日常调度轨迹。loss/overflow/parse0, 三角色同代存活, STOP0/ProcessSuccess/drain/join/dispose及19:53独立精确QUERY4201确认。原同主题`g1-app-freshness-fault-window-{result,summary,cleanup-result}.json`保留; Complete仅表示本轮观察完整, FaultCaptured=false, 非阻塞日志投递完整性unknown。未取得新的timeout同轮字段与ETW故障配对, G1 failed及timeout故障分支真机deferred不变, 不自动重采或开启4小时通过统计。
- 用户随后明确松开后, 18:46:31–18:46:53正常升级App诊断候选 passed: 本代语音释放、旧Helper conn1 STOP/新鲜receipt、进程/端口/任务退出、六配置安装前后保持、覆盖安装和Explorer普通启动均已读回。新App实际SHA256 `25bf7b811f361316c655ee9d1d08b1599f246039cabf35d5d49ff691a202b86e`, fingerprint `5e20a6d070d1c8a0d7f7b006a6d932596729dce029291cc578a4e56f4d777388`; Helper载荷34f5未变、原host/auth-lease仍驻留, TaskPriority4/实际NORMAL32与相对线程0保持。18:47:17首次运行审计config总false原样保留, 未记录当时差异文件; 18:47:49逐文件6/6已与基线一致, 无回写/豁免, 不回溯猜唯一差异字段。18:46:52–18:48:31真实resumed两条带新sample/consume/age, 分别owned0/age0与owned7167/age16, 算术匹配; timeout0不等于故障分支实机验收, 不人为造超时。原失败脚本证据保留, 后续独立`g1-app-freshness-ready-{install-result,runtime-result,configuration-readback,loaded-diagnostics}.json`标明各阶段事实; 仅诊断部署, G1 failed不变, 不启动新4小时通过统计。
- 首次升级尝试时, App诊断候选已构建但尚未安装: fingerprint `5e20a6d070d1c8a0d7f7b006a6d932596729dce029291cc578a4e56f4d777388`, setup SHA256 `3c329ba16df8f3023d0aa9795188a534caebf2c159c3c4b870327f4c73481bae`; 标准包的前端嵌入、更新签名和local/dirty/included身份通过, Helper载荷仍`34f5da5c37f666da17efa1ae48b096d823a8e59c2f8b74ba9adf8453eda67222`, Agent未改。18:25:35只读预检曾有成对释放, 但真正升级前fresh语音门禁变为physical/terminal/chord均false, 因新会话未配对而停在退出事件之前; 没有退出、安装或任务改动, 不沿用旧释放证据。原`g1-app-freshness-{package-result,preflight,upgrade.log}`与失败现场保留, 旧4DF6包保存在同主题`g1-task-policy-setup.exe`, 新包不可称为语音失败修复。
- 2026-10-24 用户18:17起定向测试仍 failed: Menu/Back/TV先正常, 随后语音不可用且看到全按键 UI 灰色, 重按两次后恢复, 不以重试成功代替首用通过。固定18:15:30–18:20:12旧实装 e9f39a 同代窗口未观察到 ownership_timeout/rx_silence/BLE断连/控制写失败, 56条续约最大wait538ms, 非阻塞日志投递完整性仍unknown; 灰态原因未归因。18:18:29/33/37的语音generation76–78已有非零音频回调、提交及成对chord释放/terminal, 目标响应检查reacted=false, revive API返回ok却两次mic均not_observed; 18:18:49/54变成marker正面证据, mic仍not_observed, 不证明实际出字。18:18:43前端仅该次读回active/passed, 不排除前后瞬态灰色。更早18:18:16菜单返回目标window_operation_failed后已hide_completed, 证明G3焦点失败存在, 但没有语音时目标输入焦点或快捷键接受证据, 不能强串因果或归咎输入法。此次与新包标准Cargo构建18:17:21起时间重叠, 不等于CPU饱和或构建导致故障。原记录 `g1-app-freshness-user-{failure,failure-detail,voice-observation}.json`只留脱敏计数/事件, 新诊断尚未安装, 不使用其字段解释旧实例。
- 2026-10-24 App freshness 诊断切片实现/自动化 passed, 当时尚未安装或取得新故障实证 (后续部署见本节): `rc003_bridge.rs` 在撤权清空前保存同一次 last accepted sample、decision tick 和 checked age, 同一 age 驱动原 `<1500ms` 判据及日志, future sample 的 age 保留 none。附带 current_rx_seq、当前观察 conn/epoch/Oseq/target generation、本地连接号和已有缓冲字节数, 不将 current_rx_seq 冒称最后接受序号, 不记录内容。恢复日志新增 App 消费 tick/样本年龄, 不是 Helper 写入或 Agent 消费时间; 不改变读序、协议、时限、Helper/Agent 或优先级。真实 CapturePeer IPC 回归 RED 因缺失 last_accepted_sampled_tick_ms 失败, 最小实现后 GREEN; bridge44项、workspace core61+4/Windows621+21 ignored/App189+1 ignored、fmt/check/runtime-simulation check 与独立语义 review passed。源码 SHA256 `8b0e805522532cdc197418b8454211ef4464e7485943ff291f9fef4a2a37306a`, 过程日志 `g1-app-freshness-*`。现有顺序先处理 pending 完整行, 再判过期, 最后 read_until; 因此 BufReader/内核管道可能有尚未读取的新 O, 但现场尚未证明, O 也可能尚未生成; buffered_bytes=0 不证明内核管道为空。此切片只补判别证据, 原16:02 G1 failed不变, 自然故障和升级验收仍待完成。
- 2026-10-24 16:02 连续观察 failed: 15:26:28.512 至16:02:11 收口共35分42秒, App在16:02:03.103因1500ms helper_receive_tick样本过期撤权, 03.130恢复 owned7167, 不把27ms恢复算通过。原collector因同batch首条engine unavailable先到记interrupted, 但同批明确timeout, 原文件保留并纠正解释为候选稳定性failed。固定±10s窗只有1timeout、1慢renew, 无rx_silence/连接结束/写失败; 03.362 renew conn1/tick5695 wait1764/lock0/write0/gap1764, 03.128桥接 slow_rounds1/idle_sleep1161/maxround1161且其余phase0。恢复先于此renew成功写232ms, 不拼成唯一因果; 后续05.662 HB lease=true与O聚合58样本/maxage266不证明故障瞬间Agent消费或IPC状态。fresh公开API确认App/Helper/renew同代、Helper实际Normal/相对Normal/Task4, 观察窗System事件日志enabled且两power provider无事件; 本窗无ETW, timer与Ready迟调度仍未知。原A失败及Normal构建短窗通过均保留。过程证据同主题 `g1-task-policy-{stability-result,first-failure,failure-runtime,failure-app-samples}.json`, 无产品参数更改或重启。
- 后续只读公开线程描述已唯一确定Helper桥接TID53628与App连接TID3516, 同原renew33036构成三角色, 两进程Normal/线程相对0。BRIDGE-ROUND的idle_sleep是当前recv_timeout轮, max_*为多轮聚合; 本次slow_rounds1且当前idle1161足以认定此慢轮, 不能进一步代替调度证据。App恢复日志带sampled_tick91702953/rx5927/conn1/epoch2/O5697, timeout未记上一accepted sample和decision tick是明确观测缺口。三角色观测复用成熟consumer, 不改Agent或产品探针; 长观察结果见下文, 不扩展为产品通过。
- 17:59 已收口唯一自然长窗: UTC08:59:23.595开始, 计划截止09:59:23.595, 实际STOP前3600.048s、另清理5.255s, `not_captured`, 未收到同代ownership_timeout触发; 工具采集完整不等于G1通过, 非阻塞日志投递完整性仍unknown。冻结工具每10s预算为0.15单逻辑核/当前工作集256MiB, 实测358区间最大0.10205核、当前工作集最大171.934MiB/进程峰175.867MiB, 本进程成本不代表全系统ETW成本。ETW355503494事件、目标4877984次, 正常年龄淘汰4837622, 最后30s环40362行但未落日常rows; overflow/lost/parse0, 三角色始终同代存活, STOP0/drain/join/dispose和独立QUERY4201确认, CSV0。原`g1-three-role-fault-window-{result,summary,cleanup-result}.json`保留, 无后台trace或命令残留, 不自动延长或重跑; 原16:02产品failed不变, 尚未取得新故障的三线程Wait/Ready因果分解。
- 16:38:31 的唯一三角色短窗因工具 row_capacity 提前收口为 unknown, 不是产品新增失败: 32768 行容量在目标事件QPC跨度24.156s用尽, 32769次目标行尝试中App连接30744、Helper桥接1866、renew159, 年龄淘汰0; 不以包含清理的37.856s总墙钟低估行率。App现有PIPE_NOWAIT空读后sleep2ms契约可产生约500轮/s的out/ready/in, 本窗App约1273行/s不能单独证明忙等或产品根因。工具seen4259837, CSwitch v5/28与Ready v2/8, lost0; observer累计CPU1781.25ms、峰工作集173.79MiB, 仅本进程且含清理, 不能当活跃阶段或全系统ETW成本。STOP0/join/dispose/三目标仍活与独立QUERY4201均确认, 未保存日常调度rows。原`g1-three-role-baseline-{result,resource-summary,cleanup-result}.json`保留。基于实测批准最小离线改为固定65536行、最多30s前窗与5s后窗, WindowEnd后的清理事件不入队; 实际故障前覆盖按UTC/QPC锚点区间报告, 不承诺固定30s。补目标QPC回退拒绝与每10s CPU/工作集、STOP前分段统计, 离线27项及PS报告4项通过并经窄审。再次单独授权的16:53:08.761短窗完整性passed: STOP前45.115s/CPU1656.25ms, 四个约10s区间为0.09788/0.01871/0.01868/0.02320逻辑核, 清理另2.124s/328.125ms CPU; 峰工作集175.633MiB。seen4940419、目标尝试64854、正常年龄淘汰23582、保留41272行跨度29.999892s, lost/parse/overflow均0, 三角色仍同代活着、STOP0/join/dispose与独立QUERY4201通过, CSV仍0行。该`g1-three-role-bounded-baseline-*`只证明工具容量与本进程开销, 不证明全系统ETW成本或G1恢复; 当时长模式预算与执行待确认, 后续唯一长窗授权和结果见17:59条, 不自动重采或延长。
- 2026-10-24 Normal 候选首次同类普通 release 构建短窗 passed: `cargo build --workspace --release --offline --locked` 在 15:26:56–15:29:23 实际编译 18 项, 146.66s/exit0, 不清缓存或造压力。先精确自有 session live QUERY 后构建; ETW 请求45s/实际48.405s, 386目标行、CSwitch v5/28 与 Ready v2/8, 95完整 state5 out→ready→run, 0乱序/重复/未知 flags, 首孤立 ready 与尾未闭合不计。最大 timer wait500.7992ms、Ready→run29.8821ms、总间隔530.0149ms, >=750ms链0; 构建命令QPC与目标行重叠47.4506678s, 不宣称CPU持续满载。对应窄窗11条renew、慢轮/ownership_timeout/rx_silence/写失败/overflow均0, 日志投递完整性仍unknown。ETW零loss/parse/overflow, STOP0→drain/join/dispose、目标代次alive, 独立精确own name/GUID QUERY4201; setup与实装hash不变, 未安装构建产物。独立复算通过, 仅支持本轮候选构建场景, 构建阶段不同不等于严格A/B, 不抹原A失败或宣称G1完成。证据同主题 `g1-task-policy-release-build-*`。
- 同一冻结候选的4小时只读观察实际从 UTC07:26:28.512557/北京时间15:26:28 开始, 预算截止19:26:28, 准备期不计。仅复用有界tail/本代身份观察, 不改运行参数; 意外撤权/断流等停止通过统计, 主动关闭/断连/睡眠与日志缺口另判, 到期也只 pending review, 必须再核清醒/连接与硬件三轮。此记录是观察起点, 不是4小时通过结果。

- 2026-10-24 15:13–15:16 普通调度候选实际部署 passed: fresh 语音释放、旧 Helper conn5 Confirmed STOP/同 host/instance 回执、任务 idle/进程端口释放后管理员正常覆盖安装, Explorer 普通权限启动。setup `4df6f7538562b9fab91f11a28511f2c801afb23cacec9647af36917906981597`, 实际 fingerprint `e9f39a9a7af3e1c5b0a5806431077931b5698e826872286b4184eb1c3fe93446`, App `82558406e6de6a3352cdae2bd0490bfa2fb0c084ddcb6f2f90daead0cee74b0f`, Helper `34f5da5c37f666da17efa1ae48b096d823a8e59c2f8b74ba9adf8453eda67222`。旧任务策略被新 App 拒绝后, 通过 App 开启按钮和预期用户 SID 正常 COM 注册; 实际 Principal 当前用户/InteractiveToken/Highest、Priority4、battery false/false、PT0S、0 trigger、AllowHardTerminate=false、action 均匹配。Helper 25880/renew33036 实际 NORMAL32/相对 priority0, 主程序44780普通权限; 原 host11880/Agent `2026-10-23.auth-lease`、runtime_files_written=false, owned7167。无强杀/手改优先级/Agent 替换。
- 安装前后六配置 hash 完全一致。15:14:42 首次运行审计只保留总配置匹配 false, 不足以逐文件归因; 同窗有真实语音 DOWN15:14:40 至 UP15:14:55.599/terminal15:14:55.765, 运行 journal 随后写回。15:16:07 独立只读复核六文件 hash 全部回到原基线, 无探针回写/字段豁免; 保留早期 false, 不伪称已经证明当时唯一差异文件。旧“同名 App 只能一个”的预检假设已纠正, main/key-host 是既有角色, 不是产品失败。组合部署证据经独立 review 通过; 原 A 失败、真实取消/注册失败及负载/4小时/三轮硬件待验边界不变。过程证据使用同主题 `g1-task-policy-*`, 不以注册正确替代 G1 通过。

2026-10-24 在普通构建 A 窗口已证 Ready 排队后, 进入最小普通调度候选开发, 不把未执行的 B 对照写成通过。注册改为 TaskScheduler COM 内存 XML: Priority4、无定时触发、电池 false/false、PT0S、AllowHardTerminate=false; 个人任务名/当前交互用户/Highest 与既有 STOP/回执保持。放弃临时文件方案以消除用户可写路径通向管理员注册的替换边界, 不另造 ACL/临时目录框架。App 用 TokenUser SID 明确授权用户, Helper 在任何 RegisterTask 前核同一用户, XML 显式 Principal, App 读回用户/登录方式/权限/关键策略; 不记录 SID。COM 对象释放早于 CoUninitialize, 不先删除已有任务, 失败/取消不继续 reset/trigger。旧注册和 Date-only 读回的两项测试真实 RED, 追加 Principal RED 证明旧代码误接受 Password/LeastPrivilege; 候选最终 Helper SHA-256 `674678405703eade2a80d5cde81f5f14de15e39548d4f3a1478abf5734820f2d`, App 任务模块 `7a250db25c7a91b63b2f9238895707f84aedee6d62df14d877922eb8a3154a05`; Helper 5 项定向/104 passed+3 ignored 全套、App 41 项任务定向、workspace core61+4/Windows619+21 ignored/App189+1 ignored、两处 fmt、workspace/Helper check、runtime-simulation check 及 Helper selftest 全部 passed, 独立 review 通过。一次 BSTR 夹具方法名编译失败已按真实绑定修正并保留过程日志, 不冒充业务 RED。COM 默认安全按官方补 PKT_PRIVACY/IMPERSONATE, 初始化失败显式分阶段返回; 日志只报 submitted/requested_priority。测试闭包失败证明调用/返回契约, 不代表真实 COM 注册失败或 UAC 实机验收。离线阶段未安装或改运行任务; 之后的实际部署另记如下, G1 仍未闭合。来源归 ATTRIBUTION, 后续仍须重跑普通构建同类采样和原四小时/三轮矩阵。

- 目的/范围: 已有 29 次 ownership_timeout 与 1 次 rx_silence 仍是失败事实, 本切片仅补 Helper 同轮 wait/lock/write/success_gap 及控制写失败详情, 不修改 Agent、协议、500ms 续约、lease/timeout、菜单或映射。成功 write 只证明本机 socket 提交, 不证明 Agent 已消费; 读端仍未知。
- 真实 RED: 新增 4 项测试先编译成功; 真实 TCP 已收到 renew 后, 首次长 gap 和真实 mutex 阻塞轮均因旧实现仅每 10 轮记录而缺诊断; targets/synth/gate 的实际生产发送入口注入 write/flush IO 错误, 因旧 bool 丢弃细节而失败。红色日志保留在同主题 `g1-renew-diagnostic-red.log`, 非编译错误代替业务 RED。
- 实现: 复用既有 250ms 慢路径诊断阈值, 正常 500ms 等待不标慢; wait/gap 达到 500+250ms、lock/write 达到 250ms 时同轮记录, 健康态仍每 10 轮。第一笔成功无前项/失败轮的 success_gap 为 none, 失败/撤销授权不刷新 last_success; 所有新续约日志在释放 shared guard 后进入既有有界非阻塞 Logger。控制命令只记固定 command、write/flush phase、io_kind/os_code/elapsed, 不记命令正文或 token; 初始化逐条写和末尾单次 flush 次序保持。
- passed(离线): 定向 9 项; 首慢轮真实 wait=500ms/lock=749ms, 相邻成功 gap=2500ms; shutdown TCP 经生产循环得到 write_failed/10058/BrokenPipe、gap=none、计数不增且仅本连接失效。满队列/阻塞日志 sink 时控制发送仍完成并返回原失败, 既有持续数据/Stop/真实 renew 公平测试通过。最终 Helper fmt/test/check/selftest 全过, 全套 98 passed / 3 ignored; 源码 SHA-256 `1827effb16f4f9abf7f6f2a1ef48cb9b027b8d75f53ef88d00ec9c7bfdd7687c`。持锁夹具验证实际测得的阻塞, 不把线程起始 latch 解释为 OS 调度硬保证。
- 离线阶段未安装、未启停运行实例、未提交; 初轮及追加失败轮测试的最终独立复核通过。用户已授权构建并安全部署诊断候选, 不将这些自动化结论视为实际故障窗口定位或 G1 4小时稳定验收, 更不证明来源隔离。后续根据真实同轮分段先证实原因再修复; 不以延长超时或猜测 CPU/磁盘归因替代证据。
- 2026-10-24 北京时间 00:12 正常升级/运行核验 passed: 旧 Helper conn6 的新鲜 Confirmed STOP 与同 host/instance 回执、正常退出整体结果、进程/端口释放均通过; raw stop_id/released_all 只由 Confirmed 代码路径核验, 未伪称日志保留原文。新 App 59784 由 Explorer 普通权限启动, Helper 54196 安装路径/载荷 hash 经管理员只读核实; 六份配置启动后字节保持。实际 HELLO 仍为 `2026-10-23.auth-lease`, `runtime_files_written=false`, owned=7167, 未换 Agent/双注入。setup `6500f5970caf8a40021a28ec71a2d0a80cbe68bc180469ccff298c958ca55513`, App `ae09ab81918ebfd10ab513c48ce630f1bda5064d1e4db5fbf174eaa611486c4d`, Helper `cc4bdfa8d5f9225170474e374b88d01f4623c8851fd74b64a25db0aa9da10288`, 实际 fingerprint `650ef9925f445240d396e1ee74cc49883410bb26c784dc30035defe6bdebdf98`, local/dirty/included。
- 内容与门禁: 用原 Windows PowerShell 指纹算法仅替换 Helper main 的前一 hash 即精确重构旧包 `04cac8...` 指纹, 证明本次源码增量无其他 dirty 漂移; pwsh7 canonical 排序差异的首个重算未匹配不作为来源证据。首个包外层漏设 local/bundle 环境而呈 unknown/not-bundling, 在安装前拒绝并保留失败日志; 之后显式设置环境重建, 核对真实 EXE 中的指纹, 不补写 manifest 冒充产物。普通 token 无法读取 SYSTEM host 创建时间时改用管理员只读公开 GetProcessTimes 精确核代次, 不降低门禁。前包身份记录保持原样。
- 首个诊断实装窗口为 00:12:26–00:17:49, 同 App/Helper/boot, 当前文件覆盖 364 行且无观察到轮转缺口; ownership_timeout/rx_silence/reader_ended/控制写失败/renew 写失败均 0, 异步日志完整性仍 unknown。conn1 tick73 的 wait=893ms/lock=0/write=0/success_gap=893ms, tick177 为 750/0/0/751ms: 这两轮迟到落在 sleep 调用返回区间, 未触发撤权; 不是 mutex/socket 写耗时证据, 也不能归因 CPU 或证明 Agent 消费。等待真实故障窗口关联, 不将 5 分钟无故障视为持续性修复或开始最终 4小时验收。
- 2026-10-24 北京时间 00:28 自然故障已捕获, G1 持续性仍 failed: 冻结截止 00:28:37.136, 同 App/Helper/boot 的实际可用日志覆盖 00:12:26.150–00:28:36.630 共 768 行, 未观察到轮转缺口, 异步投递完整性仍 unknown。8 次 App `event=ownership_timeout`, rx_silence/reader_ended/控制写失败/renew 写失败均 0; conn1 未换代。采集器第一版误以 `reason=ownership_timeout` 筛选而报 0, 已按源码事件名纠正并重算同一截止窗口, 同时用时间戳 min/max 修正乱序落盘的覆盖端点; 不保留错误零计数作为通过证据。首个五分钟窗口仍未出现 timeout。
- 首次撤权发生于 00:28:04.044; 04.366 的 tick1864 同轮 wait/lock/write/success_gap 为 1797/0/1/1799ms, 04.380 恢复。11.181 与 12.744 再撤权; 13.179 的 tick1876 为 3076/0/0/3076ms。13.848 收到 HB1882/lease_ok=false, 13.849 收到 HB1884/true, 13.855 以 epoch7 恢复。23.457 的 HB1904/false 经 23.636 inactive O 后, App 23.638 明确按 inactive 撤权; 24.060 的 tick1887 为 2949/0/0/2969ms, 25.585 HB1906/true 后 25.592 以 epoch9 恢复。HB 时间是 Helper 接收时刻, 不冒充 Agent 产生/消费时刻。
- 诊断边界: 这几轮成功续约间隔增长主要落在 sleep 调用返回区间, 不能归因同轮 mutex 或 socket 写阻塞, 也不能进一步猜测 CPU/磁盘/Agent 消费。App 本地样本超时和 Agent lease=false 都有实证, 但不能据此把 8 次 App timeout 全归因续约。00:28:13.853 的 O 汇总 max_helper_sample_age=1329ms/max_write=13ms 以及 28.407 的 BRIDGE-ROUND 各字段是多轮聚合, 不能拼成同轮因果。已冻结现有主题的 observation/first-failure 证据交独立归因审查, 未改生产行为、未启动最终 4 小时验收。
- 独立复核确认上述冻结故障事实与归因边界。诊断切片已完成自动化及本次实际故障分段验证, 但独立提交依赖未闭合: HEAD 中续约仍为内联线程, 当前 run_agent_renewal 及相关夹具属于更早未提交修改; 不能把整个新增函数或整文件混入本切片提交。暂不提交, 待 G1 完整最小依赖单元具备验收再按授权精确处理; 当前 39 项 dirty 和空暂存区保持。
- 00:44 同实例公开只读调度快照: Helper priority_class=16384/BELOW_NORMAL, 计划任务 Priority=7、RunOnlyIfIdle=false、DisallowStartIfOnBatteries/StopIfGoingOnBatteries=true、ExecutionTimeLimit=PT72H; 四线程相对 priority=0, 进程/线程 power throttling 的 control/state 均为 0。这只是当前策略事实, 不证明故障时 Ready 等待或 CPU 根因; 不修改任务/优先级/节能。00:27–00:29 System 中 Kernel-Power/Power-Troubleshooter 无事件, 不排除其它停顿。首次把 CIM 微秒创建时间与原生 FILETIME 精确比较失败已保留; 后续用同来源 CIM 精确核原实例/路径/hash, 再持句柄读取原生创建时刻。GetThreadInformation 初次因有限查询权限返回 5, 按官方改为 QUERY_INFORMATION 只读权限后成功, 未请求设置权限。
- 线程角色标记最小候选: 现有 main/log/bridge 有公开描述, 剩余 unnamed 不能仅凭 500ms 节奏认定续约线程。新增续约入口一次 `[THREAD] role=renew tid=...`, 不改变线程体系/Agent/协议/时限/调度。真实 worker TID 缺记录与阻塞满队列少一次 drop 两项业务 RED 后通过, 定向 10 passed, Helper 99 passed / 3 ignored、fmt/check 与独立 review passed; 源码 `9f049769d16753fc00570b8f01791a2311dea4ae18838098e8b9964db0a4103a`。setup SHA-256 `b6db65d4a7b907c56485e4a0254bc7e32340b01ad40b4384a5afbe543913e8f5`。此前因缺少语音物理释放证据停装, 没有把无事件默认值当作 inactive; 2026-10-24 用户确认已松开, 北京时间 12:20:38.941 DOWN、12:20:43.396 UP、43.497 chord_release ok、43.500 control_stop 同代证据闭合后继续正常升级。
- TID 候选实装/运行 passed: 12:23:28 正常退出, 旧 Helper 的 conn7 STOP 与新鲜 host/instance/receipt 配对确认, App/Helper/端口释放且任务 idle 后安装; 6 份配置 hash 不变。新 App 普通 Explorer 启动, 运行 fingerprint `987b6432a4f495d6ad6d9c6b20f2a8659792bb30477b3333dcc74dbb68fa899d`, App SHA-256 `0de0826854b278c08f2749f183f7416a967ad1efc75f1fb00ac72b841d626a47`, Helper `ccaf39f6fc1d47777a8a9c256c6ae76529c23267eaef4c619e69c4c892ffbc0c` 与实装文件匹配。12:23:53.749 唯一 renew 角色标记经公开线程句柄验证 owner/创建代次/存活; HELLO 仍为 `2026-10-23.auth-lease`, `runtime_files_written=false`, 未换 Agent/双注入。12:23:55.362 owned7167, 12:24:55.843 active conn1; 12:23:51.919–12:25:19.368 已有日志短窗 timeout/rx_silence 均 0, 不扩大为稳定性通过。真实 ETW 尚待单独 GO, G1 的既有断流失败与 4h/定向验收仍未关闭。
- 调度观察工具仅在现有本地任务目录开发, 不修改产品 Agent/协议/优先级。候选按公开 TDH schema 逐事件读取目标线程 ID, 仅目标事件保留匿名时间/CPU/状态/等待原因, 不推导私有布局、不保存其它线程身份或输入内容。首采前离线 8 项与独立安全边界审查通过, 包含真实 TDH 合成 CSwitch/ReadyThread 解析、截断失败、过滤/容量、清理顺序、目标代次与提前结束判定; sandbox 上下文曾返回 metadata not found (1168), 同字节在 owner 上下文通过, 仅记录上下文差异而不扩大为权限根因。离线清理顺序测试不等于实际资源已释放。候选最多请求 45s 观察并为清理留预算, 必须记录实际耗时; Windows 调度/API 不提供严格硬实时返回保证, 超过 60s 或丢失/解析/清理失败均不得作为完整窗口。join 后再次核对目标存活, 目标已退出不持久化该段调度行; 自建 session name/GUID 留作必要清理核对, 不停止其它会话。采集完整不代表每个等待/就绪因果区间完整, 更不代表 G1 稳定性通过。
- 2026-10-24 北京时间 12:28:33.393 首次获单独 GO 的 45s 实时调度观察 failed: QPC 锚点 788938567801 / 10000000Hz, 工具报告经过 16822ms 后 fatal, seen1/rows0/parse_errors1、overflow 与三项 lost 均 0, 首次 TDH event-info size 阶段被归为 `tdh_schema_size_failed`。旧诊断将 API 状态和尺寸契约失败合并, 未保留真实事件版本/opcode/flags/长度及原始系统码, 所以不能还原具体失败原因, 更不能以离线 1168 代替现场系统码。STOP0 / CloseTrace7007 / ProcessTrace0 后工具正常退出; logman 全列表查询失败的事实保留, 不据零匹配宣称清理。12:29:53 对本次唯一 name/GUID 执行 ControlTrace QUERY 返回 4201 (实例不存在), observer 不在, 清理读回 passed。没有自动重跑、强杀或第二次注入。12:28:30–52 相关 Helper 日志仅有 5 笔正常续约 (tick550–590, wait/gap500ms, lock/write0), 未见 timeout/rx_silence; ETW 无目标行, 无法建立 wait→ready→run 配对, 不能用于调度根因或 G1 已修结论。
- 首采后的本地工具最小诊断补充: 四个 TDH size/data API 边界保留阶段、原始系统码、needed/returned/capacity, 单独区分 API 失败与尺寸契约失败, 失败事件仅保留公开 provider/version/opcode/headerflags/length, 不保存 payload 或非目标线程 ID。具体码丢失、尺寸错误丢失、真实未知 schema 错误丢失三项 RED 后离线 11 项 GREEN, 独立 review passed; 新输出使用同主题的独立 TDH 证据名, 不覆盖首采失败。
- 第二次获单独 GO 的真实观察仍 failed: 2026-10-24 北京时间 12:37:40.780, 同一 Helper/renew 创建代次, QPC 794412371283 / 10000000Hz; 15138ms 后 fatal, seen1/rows0/parse1, overflow/lost 均 0。真实 CSwitch provider `3d6fa8d1-fe05-11d0-9dda-00c04fd7ba7c`, version5/opcode36/headerflags848/length28; `event_info_size` 原始系统码1168, needed/returned/capacity0, 属于 TDH 找不到该事件 schema, 不是已证权限或尺寸根因。首轮未记录原始码, 不将第二轮1168回填首轮。STOP0 / CloseTrace7007 / ProcessTrace0, 精确 own name/GUID 的 ControlTrace QUERY4201 确认会话不存在; `g1-scheduler-tdh-result.json`、`g1-scheduler-tdh-run-result.json` 保留独有证据。两轮均无目标调度行, 不能判断 wait/ready/run 因果, 不自动重跑或套用 V2/24 布局。
- 成熟 consumer 离线候选: 仅本地 `target/dev/menu-capture-reliability/` 固定官方 Microsoft.Diagnostics.Tracing.TraceEvent 3.2.8, 官方包 SHA512 核对、managed DLL Authenticode 与 MIT 来源已核, 不进入产品依赖/安装包, 不安装 SDK。使用公开实时 source + KernelParser(None), 保留原生自有 session 的 CSWITCH|DISPATCHER 0x810, 不调用会自动加入 Process|Thread 的 EnableKernelProvider。
  固定库真实 Lookup/FixupData/Dispatch 的合成夹具验证 v5/28 CSwitch 和 v2/8 Ready, 以及仅目标过滤/容量/两类事件未知版本与长度拒绝/正常与失败清理/代次退出不落行等 15 项离线 passed; reflection 仅用于本地固定版本合成测试, 不用于真实 consumer 或 trace 私有句柄。正常 STOP→Process drain→join→Dispose, 未 join 不 Dispose; Thread.Start 失败不对未启动线程 Join, 仍释放已创建 source。loss 取原生 QUERY/STOP, 不使用库初始化快照。独立安全边界 review passed, consumer SHA-256 `7ffa9171c3337c017a9121c6af4cb9e39f7c80cb7b4bf0d8bf8228ce293b085e`; 真实 ReadyThread 版本与实时权限/依赖/开销/清理仍未验证, 第三次采集待单独 GO。G1 持续性仍 failed, 不把诊断候选当最终 4h 验收。
- 第三次单独 GO 后结果 unknown: 北京时间 12:55:59.792 启动观察进程, 12:56:01.713 已写 own session 身份, 12:56:58.923 读回退出码1, 没有 result/CSV 且重定向日志为空。12:57:52.692 精确本次 name/GUID 的 ControlTrace QUERY4201, 会话不存在读回 passed; 不用进程退出替代清理, 未自动重跑。该窄窗公开 Application 1000/1001/1026 无记录, 不读取 dump。因结果未落盘, 真实 schema/计数/损失/STOP和join阶段/QPC锚点全部 unknown, 不凭约59s进程耗时推断采样或清理成功。
  离线复现发现报告脚本在保存前执行 `$result.OutputRows=@()` 无法赋值给 `IReadOnlyList<G1ScheduleRow>`: SetValueInvocationException (-2146233087), inner PSInvalidCastException (-2147467262)。这是确定的输出边界缺陷, 不据此补造第三次已丢失的采集状态。最小修复只投影 JSON 排除行列表, 保留 typed 列表供 CSV, 脚本异常保存脱敏阶段/类型/HResult, 再只输出固定安全摘要并退出1, 不重抛含路径的原 ErrorRecord。实际 PowerShell 有行/无行/failed 结果、文件失败脱敏和同启动 wrapper 错误传播 5 项 passed; wrapper 实际 DirectoryNotFoundException/-2147024893, 不持久化异常 Message/个人路径。新证据前缀 `g1-traceevent-report-fixed` 避免覆盖三次历史, 第四次真实采集尚未授权, 产品/consumer/parser 均未改变。
- 第四次单独 GO 后诊断工具实际采集/落盘/清理 passed, 但未捕获 G1 异常: 2026-10-24 北京时间 13:05:40.607, QPC 811210442778 / 10000000Hz, 同一 Helper/renew native 创建代次; 请求45s, 含初始化/收尾实际57131ms, 目标事件跨度57074.133ms, 未逐阶段计时, 不描述为只有45s的事件窗。result/CSV 实际存在, seen5300849/matched425、parse/overflow/三项lost均0, duration 结束, native STOP0、Process success/returned、join/Dispose与结束后目标存活均 true。最终 QUERY4201, 13:07:47.212 对精确本次 name/GUID 独立 QUERY4201 再确认不存在; 独立查询脚本初次因 dot-source 参数同名覆盖未写结果, 修正仅本地查询参数后读回, 未重新采样或停止其它 session。
  真实 CSwitch v5/28 的 in/out 各155, ReadyThread v2/8共115且 flags1(ReadiedFromDPC); 114条完整 wait→ready→run, 首个无前置out的ready与尾部未闭合wait排除, 乱序/重复ready/未知flags均0。最长 wait500.7063ms, 对应ready→run0.0192ms; 最长ready→run1.8337ms, 对应wait500.1844ms; 完整链总间隔>=750ms为0。QPC→UTC只是非原子起点近似换算, 不伪造精确跨进程时钟对齐。同期同代 App/Helper 现有日志覆盖13:05:41.626–13:06:36.698, 12笔采样续约tick5000–5110的wait/gap500、lock/write0, slow/ownership_timeout/rx_silence/write_failed均0; 常态十轮日志采样不能精确回填最大区间所属每一tick, 日志投递完整性unknown, 不证明Agent消费。固定证据为同主题 `g1-traceevent-report-fixed-{result,analysis,related,cleanup-result}.json` 与 events.csv; result SHA-256 `4191e1c2bd0b5de976fb9078944fb9b2b4a3863582a67818a903bc88a5a38538`, CSV `b0c1b3c5a1e6907adaaa674d249a74c05c0810411a0401b47b45ac94fac54759`。本次只验证工具能解析本机事件并安全收尾, 不消除原8次timeout证据, G1仍未修复/未开始最终稳定性验收, 不自动重跑。
- 第四次固定证据经独立复算 passed: 完整 wait+ready 间隔最大502.0181ms; 另40段非wait out→in中 state1×26最大1.2325ms、state2×4最大0.0096ms、state3×10最大0.0913ms, 不混进等待三元组。公开 ReadyThread flags1 为 DPC readied, 无0x2/0x4交换状态位, 结合后续真实CSwitch-in支持本窗口ready→run计时, 仍不解释历史异常。
- 正常开发负载检查: 13:14:07.993–13:15:31.277 执行一次 `cargo build --workspace --offline --locked`, 实际编译/链接并于83.284s后退出0; 不清缓存/触碰源码强制重编译/加features, 没有打包安装, 已安装App/Helper hash均保持。采样启动前错误地额外要求瞬时直属rustc存在, 13:14:46虽Cargo仍在运行但rustc0而未启动ETW; 正常构建包含build-script/链接, 此门禁不成立, 后续检查时构建已经结束, 没有造第二负载。该次ETW未执行, 不算正常负载调度测试通过。同代已有日志覆盖13:14:12.663–13:15:27.798, 16笔续约wait/gap500、lock/write0, 未观察slow/timeout/rx_silence/write_failed; 投递完整性unknown, 无同步ETW不能区分未记录的等待/调度阶段。证据保留 `g1-normal-workspace-build.{json,log}`、start/active/related.json, 不把该次日志未见异常当G1修复或4h验收。

- 2026-10-24 启动/续约窄范围历史核对: 已吸收上游 ab9aa197e59c65b6c7b22e51a968cd0cb4312446 的任务 XML 明确 Priority7, 当前 schtasks CLI 注册不指定优先级, 实际任务也为7。公开 GetPriorityClass 对同代 Helper 读回16384/BelowNormal; 不能归为个人版专有降优先级。两边 GUI 均以 schtasks /run 触发最高权限交互用户任务, 不是直接从普通 GUI 启动继承优先级的 Helper; 每次开启显式授权契约亦相同。
  真正遗漏的是上游 cde0d5a 的 XML 注册策略: 上游关闭两项电池条件、ExecutionTimeLimit=PT0S 且无触发器; ce7bdc0 的整合结果及当前实现仍使用旧 once/00:00 CLI 注册。当前任务实读两项电池条件=true、PT72H, 上游的 task_power_policy_matches 对账在当前也不存在。作为独立 G1 风险待补正常注册与实机读回, 本轮未改任务/包; 没有证据将当前3秒等待迟到归因电池或72小时限制。
  上游续约从765d0f37(2026-09-27)已是独立 std::thread::spawn + sleep(RENEW_MS), 不在串行 accept/serve 主循环内; 当前仍相同拓扑, run_agent_renewal 是尚未提交的函数提取及代次/诊断逻辑, 不是个人版新拆线程。双方续约500ms、lease2000ms、HB500ms、读轮询250ms、Agent rx3000ms/auth4000ms、App ownership1500ms均相同。当前续约只允许已通过 HELLO/Agent identity/清理准备的 connection generation, 同代退出撤权; 上游共享 stream 的续约没有该代次授权门禁。当前 Agent 用 handshakeDone 区分尚未鉴权与租约过期, 不再用被过期路径清零的 lastRenewAt 代表尚未鉴权。App 所有权计龄从 Helper 收 HB 的单调 tick 起算并拒旧代/旧样本, 上游从 App 消费 O 的 Instant 起算; 数值相同而样本新鲜度契约不同。这些事实不证明调度改善或造成历史故障, 仍仅已定位到 Helper 等待调用返回区间, 未得到异常 wait→ready→run 链。

- 下一次有界自然触发观察仅完成离线准备, 尚未 armed: 本地 `g1-natural-watch` 从同代 Helper 当前日志 EOF 读新行, 只保留续约字段; wait 或 success_gap 达750ms时至多触发一次既有45s TraceEvent采样, 观察期限20分钟且触发前需保留60s预算。无触发到期为未捕获, 文件替换/截断、目标换代/退出、真实 LOG-QUEUE overflow 或读取错误为 unknown停止; 不修改产品或开启持续追踪。单EOF快照、半行容量、身份终态、一次触发、期限及静态安全错误码等11项离线 passed, 真实overflow格式遗漏曾实际RED并已修, 独立review passed。脚本SHA-256 `01bd780836e4353892e43a938bdb81cd3736c243183e995a44836fbc9728e01b`, 本地输入边界源码 `8036825192473619a8f6ecccf49be5d21592490491db94efc86b76e2945277ef`; 冻结consumer仍7ffa9171。触发日志可能迟到, 只能采其后续窗口, 不能回溯已经结束的触发慢轮; Windows不承诺硬实时截止, 实际耗时/日志投递完整性分别记录。真实观察需另行GO, G1仍failed。

- 随后获一次明确 GO 的自然观察已于北京时间13:38:58.123–13:58:58.128正常到期, 实际1200005ms, observer退出0, status=not_captured/trigger=null。期间目标 Helper/renew 创建代次与日志文件身份边界验证未报错; 最后保留24笔续约为13:56:58.165–13:58:53.485、conn1、wait/gap最大500ms。只有有界末窗留存, delivery_complete仍unknown, 不扩大为20分钟完整日志或持续性通过。本次未调用TraceEvent Run, 未创建session/trace-result/trigger文件, 无需停止不存在的会话; 没有自动延长/重试、生产或调度改动。`g1-traceevent-natural-watch-result.json` SHA-256 `14ba9b318f86d60d8ce1396ca09330cc9891cda6f76110706b434cb099d57688`, 同前缀watch-start/run-result保留唯一窗口和结束证据。本次没有新的异常wait→ready→run链, G1根因仍未闭合。

- 纠正先采样后构建的单次实验已捕获异常: 2026-10-24 北京时间14:05:43.712启动自有CSWITCH|DISPATCHER会话, 精确name/GUID QUERY=0后才开始标准 `cargo build --workspace --release --offline --locked`。构建14:05:44.374–14:08:22.620、158211ms、exit0, 日志确有依赖/工作区编译及release完成, 不是缓存no-op; 没有清缓存、触源码强制构建、加features、打包或安装。构建QPC847246636326–848828767322、频率10000000Hz; 与目标事件实际重叠47611.6572ms, 不把整个构建区间解释为持续CPU满载。B6DB安装包及已装App/Helper哈希保持, 安装指纹仍987b6432; 标准构建生成的release可执行文件与共享build-identity-current不是当前安装身份。
  采样请求45s、实际48730ms, 同代Helper20988/renew39612; seen5017158、目标647行、乱序/overflow/parse failure/三项lost均0。CSwitch v5/28 in/out各282, Ready v2/8共83; 82条完整state5 out→flags1 Ready→run, 首孤立Ready与末未闭合wait排除。native STOP0、Process成功、join/Dispose、结束身份存活均通过; 14:07:40.884独立精确QUERY4201确认自己的session已不存在。冻结结果/CSV SHA-256分别`8c6507701c90a3bdd1a2e21477fc021e40a82447bca1e35196578d96d115658c`、`2ddfd648e8544c8066a2c196f1b5f0f56bf1d66383a9206adbe0789705e1a887`, 证据前缀`g1-traceevent-release-build`。
  4条完整异常段分别为 wait500.2097/500.1865/500.2487/500.1816ms, Ready后等运行1046.5246/1831.7846/1360.1106/2159.1939ms; 对应Helper tick12247/12249/12256/12258的wait1546/2332/1860/2659ms, lock均0、write0/0/0/2ms。末段out/ready/run QPC为847625997469/847630999285/847652591224。UTC锚点映射与日志约差4–8ms, 不是逐tick同QPC打点; 4段都位于同一构建QPC区间。固定同代日志窗口14:05:44.191–14:06:31.004有13笔RENEW、4慢轮、4次ownership_timeout及恢复, rx_silence/写失败为0, 异步投递完整性仍unknown。独立复算确认本窗口主要迟到在已Ready后等待运行, 而非500ms timer本身迟到; 不能仅凭构建重叠认定CPU饱和或Priority7是唯一根因, 更不能外推全部历史timeout。
  后续仅在同一次构建仍活跃时临时NORMAL并恢复原BelowNormal的B观察获得授权, 但收到授权时构建已经正常完成, 因此B未执行、没有改变任何优先级或制造第二负载。A证据保留, G1仍failed且最终稳定验收未开始。

## 2026-10-23 菜单返回目标令牌校验

- 修复范围: 菜单打开后的前台观察不再使原目标令牌因“不是最后观察窗口”而被拒绝; 保留公开 IsWindow/owner PID 校验、前台激活读回和动作发送的前台约束。未增加同进程 HWND 复用的完整生命周期保证。
- 验证: 真实 Controller/TokenState 的确定性 RED 为 StaleToken/calls=0, 修复后边界恰调用一次; 所属8项、scene84项通过(1 ignored), 完整workspace core61+4、Windows619/21ignored、Host185/1ignored及fmt/check/runtime-simulation通过, 独立review通过。早期message-only窗口激活结果不稳定的完整测试失败已保留, 后续窗口测试仅validation, 不实际激活。
- 本机2026-10-23北京时间18:54用户确认开菜单、再次关闭、鼠标点击输入框不再重现、正常输入/Back、重新打开上下导航均正常。日志6次hide_completed随后visible=false, 首个Back直接map_fire, 无残留菜单取消兜底或close blocked, 此交互验收passed。
- 独立限制: 同窗口6次自动恢复目标焦点返回window_operation_failed, 未区分SetForegroundWindow返回失败与即时读回不同, 不宣称自动焦点恢复API已通过。同App/Helper的16:22:02–18:56:25有界日志有29次ownership_timeout、1次rx_silence/断连、0次gate_write_failed、0次日志overflow; 当前文件覆盖该区间且未观察到轮转缺口, 异步日志仍不保证无缺项。持续接管稳定性仍failed, 与本令牌修复分开。
- 被测安装包SHA-256为`8d77ffa8cf11098dfda5233bac048f83c6899dc23b112366992f548c12974ef0`, fingerprint为`04cac8eb38673df07a56104c91159fa819101d4bc7ce66e81bde1db52820772b`, local/dirty/included; 安装包还包含其他未提交改动, 本独立提交不是整包快照。

- 发现日期：2026-10-03。
- 状态：部分修复、剩余边界调查中；历史菜单、音频队列、助手误退及状态界面修复和验证各归下文对应候选。2026-10-12 用户重启后，0.8.6 来源修复候选已实际加载新 Agent，菜单完整链与用户确认的菜单、实体键盘恢复通过。同日修复音频端点名称识别与语音键 UI 反馈耦合，经用户明确选择新同线通道、正常安装后，四次语音会话及用户确认的按键反馈/语音输入通过；原声卡 Code 10 原因、显式连发开启、完整双型号矩阵及历史三轮/单击退格异常仍未全部闭合。
- 影响范围：Windows 本地 0.5.0 候选；用户报告在无线麦主窗口为当前激活窗口时较易发生。不是“始终置顶”功能。RC003 统一捕获链与原 ATVV 语音路径；不外推 RC001。
- 被测身份：基于 `c089497133c7588b6ea4418d2e824d67a35890f9` 的未提交统一按键候选，安装包 SHA256 `02515fb8d0b73373c7738172a528bbf202093ab180f5b64188ef94ac17548c51`，固定上游来源 `4d4de099f823cc67f334ff2a498e98c2806e4455`。包与安装验证归既有输入 evidence。
- 功能点：完整模板菜单的返回焦点；报告 Agent → Helper → App 传输和硬件观察；语音启动准备及音频消息队列。
- 现象：Ctrl+Win 录音曾无法正常持续，之后恢复但用户感觉响应变慢；Menu 偶发闪烁、反应迟缓；返回/音量键硬件响应偶发缺失。
- 复现条件：保持原配置，在主窗口前台操作实体按键。当前证据不是一次严格控制前后台变量的实验，不能把全部失败归因于前台状态。
- 正常预期：Menu 可一次打开、一次关闭并返回真实打开前窗口；各报告边沿及时到达且 DOWN/UP 配对；录音准备期间积压不应因消息条数限制意外中止，释放/取消仍须清理。

## 2026-10-22 接管瞬断的连接代次隔离与诊断候选

- 续约窄窗独立复核补充: 本次旧会话出现 rx_silence 与 3135ms 的末次成功 write 间隔, 桥接 stop 文件检查区间3440ms、idle区间3375ms。这些只证明具体观测区间迟到, 不能唯一归因CPU、磁盘或某条锁, 更不能归因尚未部署的菜单修复。后续诊断方案仅在Helper记录同一慢续约轮实际wait、成功write gap、lock/write耗时, 并保留控制写失败阶段的IO kind/os_code/elapsed, 复用非阻塞日志; 暂不改阈值、不建通用框架、不改需重新加载的Agent。此处记录方案不表示已实现或验证。
- 2026-10-23 菜单 token 新包实装 passed, 精确鼠标复现验收 pending: 自动化/嵌入/签名/生产隔离通过后, 北京时间16:18正常退出完成匹配 StopACK/receipt与整体 shutdown。首次本地脚本因误限定 ACK 字段顺序拒绝续装; 修解析及5项夹具后按本轮完整链续装, 不重启旧 App、不强杀。conn20 ACK同条passed/hook_detached/identity_match; stop_id/released_all由Confirmed路径校验, 原始正文未留存。16:22 Explorer普通权限App46104启动, Helper51732沿用同host的auth-lease Agent, runtime_files_written=false, 无新注入; owned=7167。六配置保持, Helper hash与前包一致; 截至16:23一次短窗timeout/reader_ended/cleanup_pending=0。setup SHA-256 `8d77ffa8cf11098dfda5233bac048f83c6899dc23b112366992f548c12974ef0`, App `9825b0d584da3e8755443f921c7c34482907a305547b9e936f01eaff08c01c1c`, fingerprint `04cac8eb38673df07a56104c91159fa819101d4bc7ce66e81bde1db52820772b`, local/dirty/included; 源码仍为下述2f0d冻结版本。证据为原主题menu-return安装/身份/运行/Helper审计/启动摘要, 未提交或推送。
- 持续接管仍 failed/待定位: 菜单新包安装前, 旧App37632/Helper38232于北京时间16:09–16:13仍多次ownership_timeout, conn16/19 peer_closed、conn17/18 gate_write_failed; conn19停止unconfirmed, 后续conn20本轮正常退出confirmed, 不抹去早先失败。这些发生在菜单修复未部署时, 不归因此次代码; 构建同期也不是CPU/调度根因证明。一次有界提取保存在menu-return-preupgrade-session-events.log, 不盲改阈值或要求用户反复测试。
- 菜单 token 最小修复已完成确定性验证: 真实 ApplicationController/TokenState 观察原窗口后再观察菜单, 恢复原 token 的私有 FnOnce 边界在旧门禁下实际 StaleToken/calls=0 (RED), 删除 current 相等门禁后同 PID/不同 PID 切换均恰调用一次且收到原 token。公开恢复入口委托同一边界, 平台 IsWindow/owner 与 SetForeground/前台读回原样保留; send_task_keys 和取消策略未改。真实自有 HWND 的有效/销毁/错 owner 用例仅运行提取的 validation, 不再执行前台激活。所属 8 passed, scene 84 passed / 1 ignored, 完整 fmt/workspace test/check/runtime-simulation check passed, 增量 review 无阻断。新源码 SHA-256 `2f0dcee41122768c8ac846913437154fbfd30afc978ee5c00553a6e17ff8c30a`; 这些结果不代表桌面焦点恢复已经用户验收, 也不声明同进程 HWND 复用完全受保护。前一版夹具 full failed 继续保留, 新证据为 menu-return-boundary-*。
- 菜单返回目标测试边界纠正: 第一版真实 message-only HWND 回归定向取得有效目标被 current 门禁拒绝的 StaleToken RED, 去门禁后定向通过, 但完整 workspace 中同测试真实返回 Ok 而非预期 WindowOperationFailed, Windows 617 passed / 1 failed / 21 ignored。由此撤回“message-only 必然无法激活/该试验保证不扰动前台”的假设, 不放宽断言掩盖差异, 停止真实 SetForeground 测试与部署。原 menu-return-token-*/menu-return-test.log 保留; 后续改用真实 Controller/TokenState 加私有 FnOnce 平台边界验证委托, 自有 message-only HWND 仅验证公开 IsWindow/owner, 不激活。此测试修正不授权删除发送动作前台检查或声称同进程 HWND 复用已有完整保护。
- 2026-10-23 新包首轮用户硬件反馈: 首 Menu 正常打开、第二 Menu 视觉消失, 鼠标点击任意输入框时菜单再次出现; 首 Back 未删除, 后续 Back 关闭菜单后删除正常; TV/两次语音正常。整体 failed, 开关压力边界未测。北京时间 15:09:14/gen34 打开后, 15:09:23 第二 Menu 的关闭恢复返回 `stale_window`/0ms, 随即 `restore_failed_menu_retained` 和 `close blocked`; 15:10:12 首 Back 因非前台检查取消残留菜单而被消费, 15:10:14 后才实际 map_fire Back。15:12:54/gen42 与 15:12:57 重复同一恢复失败/保留菜单链, 15:13:37 才取消。期间未见新 opened 支持“鼠标点击新产生 Menu”结论; 当前证据更直接证明关闭后状态恢复保留, 不能把画面重现唯一归因键盘误捕获或桥接重放; 用户已明确是鼠标点击输入框触发视觉重现, 不是实体键盘输入触发。取证范围 App37632/Helper38232 本次创建后至约15:14, 未见 ownership_timeout/reader_ended, 不外推长期稳定。只读源码发现 restore_target_foreground 先 identify_foreground 会把 ApplicationController.current 换成菜单, 随后 restore_foreground 又要求 current 与原目标一致, 存在自我失效的可测试契约冲突; 当前日志未区分内部拒绝与公开 IsWindow/owner 拒绝, 仍须业务 RED/精确诊断。只记录不改生产实现, 不提交, 不让用户反复复现; 脱敏事件保留于原主题 post-reboot-menu-events.log。
- 2026-10-23 正常重启后的新包实装与加载 passed, 硬件验收 pending: 用户自行正常重启后, 确认新 boot、旧 host 已结束且 App/Helper/双端点均空闲; 失效 binding 由现契约记录 host_instance_ended, 未删除或覆盖上游资源。UTC 06:52 正常覆盖安装后六配置逐字保持, Explorer 启动 App 37632/elevation=0; Helper 38232 实际安装路径及载荷散列经管理员只读核验一致。HELLO 为 build=2026-10-23.auth-lease/auth=true, 个人端口 47832, 随后 engine_ownership=7167 和启动 passed。一次截至 UTC 06:54 左右的短窗检查 timeout/disconnect/cleanup_pending 均 0, 不代表冷首用或持续性通过。setup SHA-256 `8e3cc243f3ad5339b86e89a0f62f51c38e44b9d8f327cf121735b536a142a90b`, App `0dee08420222bc48305b7bc1cab77e41415cb3714d21c96bb4559422f7e64428`, Helper `c6e271749ef07b49d11cf53488ee441b9cbe988ad51d1e40b614da36d71a84c4`; source base `1f1baec`, fingerprint `1a3b22c8518e1c9500f046f726aa2e8ab2573e855fed0f9c08cda4368fad3b5f`, local/dirty/included。新证据为原主题目录 post-reboot-*; 旧 bridge3 manifest/failed 保持不变, 不提交或推送。下列未部署描述为此前阶段事实, 当前新版已经运行等待用户验收。
- 现场 failed 后的离线进展（未部署）：Agent 将认证身份与捕获 lease 状态分离，隔离旧 socket 回调，116 项 VM 测试通过并经独立 review；新 `AGENT_BUILD=2026-10-23.auth-lease` 与 Helper 标记已严格同步。现有自检先实际因标记失配失败，修正后 60 项通过；另将既有个人路径自检失配同步到当前 SayAllPersonal 契约，不改生产路径。当前旧驻留仍为 upstream-usage，ignore 模式无已授权热换入口，不把磁盘源码/标记同步当作现场换代或要求用户反复重启测试。
- Helper wake 离线候选复用标准库容量 1 的合并通知：E/R/A 成功入 FIFO、O 发布并释放锁、stop=true 之后才通知；Full 不丢或重 stamp 数据，worker 空闲用 recv_timeout 保留 50ms 控制轮询，断开正常退出，O-before-E/逐 data 控制公平不变。真实管道三项 no-wakeup RED 后，七项等待前/中数据、仅 O、关闭及断开发送端回归通过；Helper 全套 89 passed / 3 ignored、fmt/check 与独立 review 无阻断。测试加长等待仅为 cfg(test) 夹具，不改变生产阈值；不保证 OS 调度或宣称已解释 2231ms/646ms。本节上列离线源码不同于下列已安装 bridge3 manifest，保留现场 failed/安全关闭，不覆盖包身份，不提交或部署。
- Bridge 局部诊断隔离也已完成离线 RED/GREEN：原 CaptureNotes 即使锁外写日志，仍在 reader/watchdog 同步文件 I/O；真实管道阻塞 sink 的两项 RED 分别阻住 UP/inactive 处理和所有权到期。现使用进程级唯一、容量 256 的专用 worker，不随连接/Bridge 重启重复创建；入队前沿用原诊断时间戳，只有诊断可因 Full 丢弃并计数，不影响 O/E/控制数据。queue_full/queue_closed/start_failed/write_failed 可读回，成功写入后聚合 diagnostic_recovered；写入及 flush 失败均计数，不伪称成功。强化断言在 sink 释放前实际观察 edges_applied 增二、epoch3 非零 R 与 O 的真实处理，非仅等待超时。42 项桥接、18 项诊断（互有重叠）及独立 review 通过；只隔离本 Bridge 诊断，mapping/其他 gatt_note 仍可能同步阻塞，进程退出也不保证队列排空，不能宣称全部日志风险或现场延迟根因已关闭。
- 最终冻结组合自动化 passed：四源码散列前后相同，根 fmt、workspace test（core 61+4、Windows 615 passed / 21 ignored、Host 185 passed / 1 ignored，其他集成/文档通过）、workspace check 及 runtime-simulation check 通过；Helper/Agent 复用上述精确冻结证据，没有重复前端测试、打包、部署或启用。正常冷首用、语音、Back 和关闭重开仍没有新包硬件通过证据；现场旧驻留的本次换代入口尚不具备，不要求用户现在重启或把重启当作日常恢复。真实红/绿与独有失败现场继续保留，故障关闭后再回收可重现过程输出，不新增源码快照或覆盖安装 manifest。
- 回归归因复核：三个等长 UTC 15 分钟窗口分别为旧包 `12:37:18–12:52:18`、新包 `12:52:24–13:07:24`、新包故障期 `13:45:33–14:00:33`。所有权 timeout/resume 分别为 17/17、0/0、52/52；Agent disconnect 为 7、0、14，auto_trigger submitted 为 4、0、4；已记录 O 队列最大年龄为 4923ms、无慢样本、3076ms，均无达到现有慢阈值的 bridge write 记录。旧包已有严重积压，新包也确有频繁失效；用户操作、输入负载和调度未控制，计数不能证明或排除某提交回归，更不能归因 CPU。
- 部署对照边界：旧安装 App/Helper SHA-256 前缀为 `ed96fa08`/`f157298b`，新安装为 `960f604c`/`8a396f08`。前端八个 JS/CSS 资产名称/大小一致；安装后新增 RawInput example、Console feature 及未构建 bridge3 不在运行包中。一次有界内存复算按现有算法处理 303 个输入、排除上述后置输入并替换两源码与 Helper payload 后，新旧均未命中既有 manifest；没有逐文件历史清单，不能宣称其余 dirty 字节完全相同，不继续猜测输入。
- 新增观测开销仍须审视：1f 的逐 O 年龄分配位于 App 共享锁内，Helper 慢 O 格式化位于唯一 worker，retired 日志位于 renew 共享锁内，reader_ended/authorized 日志也可能延后必要撤权/首次续约。尚无证据证明这些导致秒级停顿，亦不能用非阻塞落盘排除格式化/分配的放大效应。后续候选将观测改为状态变化/聚合、移出关键锁并先完成控制动作，保留 GEN 隔离；不以此宣布 10053/10054 初因已定位。
- 本次 UTC `14:00` 已证 Helper 及时收到 O90 对应 HB，但 O 排队 2275ms 后 write=0ms 才到 App 并续权；随后 false HB94 未立即撤权。`14:12` 自动恢复中的 release_pending 提示也由用户确认并未手动关闭。下一切片分别验证新鲜度/显式撤权、跨连接/ownership epoch 配对清理与持续 E/R 下控制公平性；旧协议没有逐边沿原生处置证明，绝不缓存未知边沿补执行或声称来源已隔离。
- 后续源码候选采用单版 bridge3：O latest 携 Helper 接收时开机毫秒/状态/连接/epoch/sequence，E 携接收入队冻结的连接/epoch/sequence/已 ACK target；首次及撤权后重新授权建立 sequence floor，连续有效授权不前移，不能让 fresh O 使本地超时前的旧 E 恢复执行。review 另实际复现 O→取 E 并发窗，现先取 data 再读最新 O 并先发，不改 stamp 或正常 FIFO。Helper focused 12 项及 cleanup 22 项、接收端 37 项与 setter 1 项、完整组合检查/独立 review 已通过；UTC `15:56:26` 已正常安装，随后用户硬件验收 **failed**，不提交或交付本候选。新增诊断改为首慢/状态变化及聚合，先控制后日志、锁外格式化；它既修已证契约，也为下一次确定具体阻塞阶段留证，不等于已解释最初 TCP 中止或所有秒级延迟。
- bridge3 包 SHA-256 `55777dd77d5ff49bee967c0f4225a99091d7f07ec59ad1f81bcffaac4a9385a7`、固定 fingerprint `e3284f2c028ce9b1ceada9e8134a7883267636d2ed17b9cfef063ef50e028ad7`（base 1f1baec、local/dirty/included），实际 App/Helper 散列、矩阵与有界首心跳读回归 Preview。六配置与旧驻留绑定/host 精确代次保持。最初旧 Helper 停止未确认，后继仅清理模式经当前 Agent/request 的 Confirmed 分支、App release_confirmed 与整体 shutdown passed 后才显式续装；ACK 正文未留存，不将第一次未确认改写成成功。轮转语音前置查询和此后继门禁只在本地过程脚本修订，不改产品或清回执。来源/实体反引号仍 failed，采样未启动。
- 本包实际复发：用户确认闲置首 Menu 失败、第二次成功，语音失败，Back 两次删除过多，TV 成功。UTC `15:59:44.627` App timeout，`45.757` 唯一 slow round 的 `idle_sleep_ms=max_round_ms=2231`，请求 sleep 为 50ms；这只定位到该等待返回区间，不证明 CPU 原因。`45.766` O403 短暂恢复，`45.767` false HB/读错误 `ConnectionAborted / OS 10053`，随即 closed；`16:00:01.099` 才出现下一连接，相隔 15.329 秒。cached auth_rejected 增一且新连接有 renew:first，但窄窗没有 dropped:auth_mismatch 原因记录；参考 Agent 的 lease 失效后认证误判/退避仅为强匹配候选，驻留字节未知。false HB 的 renew_age=0 可能由原始 -1 被解析压零，Helper 写成功也不证明 Agent 已处理续约。pending 统计不含 O slot，round 聚合字段不普遍属于同一轮，不能用零值排除全部调度/日志/锁耗时。
- Back 原边沿与执行时序：Agent 时间 DOWN `16:03:08.181`、UP `08.409`，Helper 接收为 `08.182/08.413`；App 为 `08.237/09.059`，UP 接收后至 App 边沿晚 646ms，将 228ms 的 Agent 报告间隔放大为 822ms。当前实际记忆模板单击仅 Backspace、双击/长按禁用，缺省自动单击连发；App 在此段产生六次 repeat，加两次初始单击共八次 queued 提交，UP 后没有继续连发。不能归为用户真实长按，亦不能用 queued 证明最终删除或原生已抑制；缺少逐 E 出线与 App 读取/处理分段，尚不能将 646ms 唯一归在任一端。
- Voice 对应 attempt 26 为 abort，callback_packets/bytes/decoded_samples 均零，未记录 chord_press ok，但有 chord_release ok。成功 press 后 guard 取消分支可先 release/abort 而不记录 press ok，现有证据不能唯一确定各 modifier DOWN 时点或把零音频强归同因。UTC `16:07:28.037` 经唯一且祖先已核对的正常关闭按钮停用，随后 Agent STOP passed/identity_match/hook_detached、fresh receipt、App release_confirmed，Helper 和端口均释放；映射 repeat mask 清零。`16:13` 只读左右 Ctrl/Win/Alt/Shift 与 Backspace 全 UP、UI 已关闭；仅正常停用改变增强开关为 false，保持关闭，不改映射/模板/音频、不强杀。
- 调度策略只读边界：个人/上游任务有效 Priority 均 7、RunOnlyIfIdle=false；个人电池禁止启动/切电池停止均 true、执行上限 72h，上游均 false/无上限。当前创建参数未显式指定这些策略；未证明它们造成此故障，未修改。UTC `15:59:35–16:00:05` System 查询没有返回事件，不据此排除线程调度迟到。保留上述最小脱敏窄窗及安全停用证据，反引号和实际采样继续暂停。

- 用户新报告：遥控 Menu 首先打开原生右键菜单，多按后恢复，Back 又能按模板删字；恢复后实体键盘反引号仍变 Tab。两现象分开处理，之前菜单重开导航修复不代表来源隔离或接管持续性已修。
- 同一 App/Helper 会话的 UTC 时序：`12:05:28.695` Helper 续约写入 tick 4050 为 ok；`33.065` App 所有权清零；`34.172` gen 2/rx 2806 短暂恢复；`34.758` Helper TCP 读返回 OS 10053，`34.759` 断线并在其后记录未确认的 stop 收尾；`36.685` 再清零。`49.352` 同 Helper 重新 accept，HELLO 鉴权/targets gen 2 确认后，`49.846` 首个 lease_ok=true HB，`49.860` 恢复 mask 7167。第二段无本地所有权约 13.175 秒；`49.452` 已收到 Back 边沿但 owned_mask=0、手势为 unconfigured，恢复后后续 Back 正常映射。用户给出约数时间，不能将该窗口强行等同首 Menu 的精确时刻。
- Agent 缓存 STAT 在 `12:05:22.841–49.846` 的累计 lease_expired 从 35 到 44、auth_rejected 从 82 到 83、send_dropped 增加 49；不是九次独立故障，也不是每条含 lease_expired 的 HB 都发生一次过期。同期 Helper 记录队列最长 2147ms、接收年龄最长 2236ms，不能据此直接归因 CPU、杀软或整个十三秒。实时 false HB 未在此窗口落盘，10053 的发起者、断线后真正 connect 发起时刻/等待分布以及驻留 Agent 精确字节仍 unknown；当前磁盘 Agent 仅作协议参考。
- 已可重现的独立缺陷：旧 renew 故障共用全局布尔值，迟到故障会结束新连接；旧授权布尔值也可能让新 stream 在当前 HELLO 前续约。新增受控 TCP/故障闩锁及授权回归，旧实现分别实际失败；第一次合跑的 Mutex poisoning 干扰已通过单独重跑故障项排除。现将 stream、授权及故障绑定 connection generation，旧撤权也不能清掉新授权；同代故障仍及时结束其自身读取循环。该契约修复尚不能解释本次最初 10053，本会话没有 CONN-STALE 记录，不能宣布“接管瞬断已修复”。
- 新诊断限定于 App/Helper：匿名连接 started/authorized/reader_ended/retired、读/续约写 OS 类别与时长、同连接最后 HB/续约年龄；HB 的 helper_hb_seq 与 O 队列/写出采样年龄明确以 Helper 接收为原点，绝不称 Agent 原始采样时钟。App 原有 O 确认日志附本地接收间隔，线协议不改。日志采用既有非阻塞队列和降噪条件；不加 token、设备路径、窗口标题或用户模板内容。
- 当前状态：源码候选已通过独立 review、完整 Helper/workspace 自动化与仿真检查，UTC `12:52:22` 正常升级并以 Explorer 普通权限恢复。已读回新 Helper 实际安装路径/hash、fresh connection/HB/O 诊断及 owned mask 7167，六配置、binding 与 host 精确代次保持；未强杀、不修改 Agent、驱动、超时常量、模板或用户配置，硬件持续性仍待验。受控测试未覆盖完整续约线程与 accept 交接的全部调度。ACK 到首 HB 之间的恢复快按竞争、false HB/迟到 O 契约及实体反引号分别保留，不混入本笔。包指纹、定向证据与门槛归 [Preview](../Testing/WindowsRC003Preview.md#2026-10-22-连接续约会话代次候选)，过程材料复用 `target/dev/menu-capture-reliability/`，不另建重复长期台账。
- 构建前旧实例又发生恢复：UTC `12:39:51` 旧 Helper 断线，随后 App 在 `12:40:26` 和 `12:41:07` 明确记录 `auto_trigger reason=helper_disconnected`，两轮各有 task_trigger，最终 `12:41:15.728` 所有权恢复。用户随后明确确认北京时间 `20:39–20:41` 没有手动开关或退出操作；与已有自动恢复链一致，不再将此段换代归为用户干预。Helper 单元测试已在 `12:38:21` 结束，workspace 此时仍在编译，实际测试在其后运行；已核测试桥使用临时目录/随机端口、退出事件有 test 后缀，未发现触及生产固定资源的路径。不能把编译时间相关性解释成 CPU 根因；部署须重新读取当前会话和配置基线。
- 首按恢复的协议边界：现有 Agent 先 emitEdges，之后才检查 lease、尝试清报告并验证结果；相同 ACK→E→true HB 顺序既可能对应该边沿已被原生抑制，也可能对应该边沿已透传、后续才续约恢复。旧协议没有逐边沿 consumed/passed 事实，未来 true HB 或累计 clears 不能安全追认先前 E；Helper/App 缓冲重放会有重复原生动作风险，本轮不实施。任何后续首按执行方案须先获得实际捕获协议的逐边沿处置证明，不能用当前磁盘 Agent 推定驻留字节已具备该能力。
- 本轮有界收口：正常升级与约两分钟匿名诊断采样通过；用户明确确认新包闲置首 Menu、Back/TV、按住语音并松开，以及关闭全按键再开启后的首 Menu 均正常，本机本次遥控器这一组硬件验收 passed。它不外推 RC001、其他 Windows 或全部调度/故障恢复场景；10053 最初原因、完整接管持续性、首按协议缺口与实体反引号来源仍未关闭，不继续自动轮询或重启。

## 2026-10-22 模板菜单重开后的导航冻结候选

- 状态：本项本机本次遥控器用户验收通过。用户明确确认默认选中/高亮当前模板、切换后立即重开 Menu 可上下导航、切回原模板后按住语音并松开正常结束。原现象为重开后暂不导航、稍后恢复；与捕获所有权失效导致 Menu 本身不能打开分别取证。
- 现场证据：UTC `04:07:20.939` 模板确认后原窗口恢复返回 `stale_window`，宿主保留交互菜单；`04:07:23.805` generation 14 再次打开，`04:07:25.181–26.980` 多次上下 `map_gesture` 均为 `accepted=true / fired_count=1`，同次打开没有 `activation_result`，直到 `04:07:39.543` `native_focus_verified` 才确认菜单焦点。此链说明输入已到识别器，不能把这一段归因为捕获缺失或实体键盘来源未知。
- 已确认缺陷：scene 每次真正打开都会清除本次焦点确认，host 却只按上一轮 `interactive` 是否为 false 决定激活；恢复失败保留旧交互状态后，新菜单跳过激活与读回，遥控路由因焦点未确认而拒绝。窗口恢复为何报 `stale_window` 仍独立未知，不因本修复而关闭。
- 最小修复：宿主按实际 `menu_open_generation` 区分真正新开菜单并重新激活、核验实际前台；同一菜单内容/偏好更新不重复抢焦点。`SceneSnapshot.menuGeneration` 为必需的可空会话字段，菜单或关闭收尾保留原打开 ID，完全释放后为空；全局 `generation` 继续用于映射和 IPC。持键取消会改变全局代次但不改变打开 ID，不能被误当成新菜单；覆盖 remote-only 模式下持方向键开始语音及 native 持键期间配置取消，避免抢走语音目标焦点。显示缩放以独立布局失效标志重新排版，不清会话身份或再次激活；此 DPI 回归在身份混用实现实际失败后修正。迟到旧代次回调仍按原会话守卫拒绝。
- 同轮当前模板问题：原菜单强制初选首项，并把请求态模板标为当前。现以映射引擎已确认的 `mapping_notice.template_id` 初始化游标及当前项；强调边框代表当前模板，底色代表导航选择，两者独立。确认回执可刷新当前标记，旧 revision 不覆盖新状态，取消不应用；通用配置、未确认或当前模板不在目录时不伪标其他模板，暂不可用的已确认模板仍保持身份。
- 日志新增脱敏的 `template_menu phase=opened|navigation|gesture_rejected`，记录打开代次、游标、数量和拒绝原因；能区分焦点未核验、native 已路由/持键、语音及等待释放。不记录窗口标题、应用正文、模板名称或设备身份。
- 用户验收对象：当前安装包 SHA-256 `8d448f9dfbb0d717446961f10f6fa6174d220ecdfac09267d2ac1dbd2febacb0`，运行指纹 `d6cb322399425a317cd76e937c280ae6efe8eb84871fba626637a8ac772d3186`，local/dirty/included。结论只覆盖本机/本次遥控器及上述三项，不外推 RC001、其他 Windows、冷/闲置首用完整矩阵或来源隔离；原 `stale_window` 的独立根因仍未关闭。
- 验证边界：初版回归实际观察到 host 1 项及 scene 3 项预期失败，再作最小实现；完整自动化、真实 Edge 受控快照渲染和剩余硬件门槛归 [模板专项](../Testing/StructuredTemplatesAndDrivers.md#2026-10-22-菜单重开导航与当前模板候选)。候选基于 `01d5a397b722437994d55dfaf2d20d240e6124a4` 加本轮及原 dirty，未提交，不能用该 HEAD 代表完整源码。菜单过程证据复用 `target/dev/template-menu-navigation/`；组合安装身份与验证边界归同一模板专项，不因安装成功宣称导航或来源隔离已验收。

### 同轮安装与配置变化核对

- 组合候选在 UTC `10:01:59` 完成正常覆盖、Explorer 普通权限启动及身份读回，安装前六份 JSON 逐字不变。之后批量取消脚本错误地再次拿安装前 hash 作最终基线，报告配置变化；这不是取消误写的证据。
- 同一新实例的日志先记录 `10:02:49` 复制模板、`10:02:50–55` 未关联模板的确认删除、`10:02:59` 与 `10:03:00` 两次单项关联移除；用户明确确认这些是本人主动测试。当前保留用户的一份模板、七个关联，不恢复原九关联。批量打开/取消在 `10:03:26–27`，两次取消相对脚本开始 hash 均不变，未观察到后端批量请求。
- 安装前快照仅有六文件 hash，没有 JSON 正文，不能声称已完成字段级 diff。后续自动化以各自开始时的最新配置为基线，记录开始/结束时间；旧 UIA 全窗同名首项定位有歧义，须改为关联表、批量操作表头及当前对话框内唯一控件定位后再运行。该核对不授权生产批量写入。
- 后续紧凑组合包正常安装后，UTC `10:30:35–43` 已以最新七关联为基线完成精确限域全选/更换取消/移除取消，实际选择数七、六配置与当前模板保持、无批量后端请求。新主实例的普通权限、载荷/指纹、52.8px 底栏与验证缺项归 [模板专项](../Testing/StructuredTemplatesAndDrivers.md#2026-10-22-菜单重开导航与当前模板候选)。真实 bulk 事务夹具在 imported 后、mounted 前失败并正常退出；离线确认其非法重写 Tauri 只读 `invoke`，已修夹具但未重跑，不将静态预期 TypeError 冒充现场异常或产品失败。
- 菜单用户确认后再获准的一次独立隔离窗口已实际完成三个 synthetic 关联全选更换、部分两个移除、更换/移除取消零写、真实 Windows 文件锁两类失败零写及解锁重试；整体夹具仍 failed，最后刷新未等 busy 终态/对话框出现，当次 `native.mjs:220` 确认选择器为空，未执行剩余一项清空/无效 ID 原生段和底栏原生焦点。不将夹具错误报成产品批量失败，亦不据前段通过提交 bulk。已正常退出测试并在 UTC `11:31:18` Explorer 普通恢复同包，六最新配置、七用户关联、binding 和精确宿主代次保持；详情及未覆盖归同一模板专项。后续只离线补刷新终态/控件可见可用等待和安全异常 frame，不再次停机或运行。
- 来源隔离仍 **failed**：用户实装报告实体键盘仍参与遥控监听。只读调查中 Helper EDGE 可关联到 88 个 observations，但没有贯穿链路的事件 ID/可靠来源；UI 观察并集合入 mapped 与 physical，`rawInput.activeButtons` 也可能经 DriverEdge/merger 写入，不能仅换 UI 快照宣称已隔离。Agent 在目标 usage 清除前报告 observed，十二个捕获键的独立 Raw Input 完整覆盖尚未证明；目前没有已验证的全键 UI 替代来源。先补齐可靠来源证据，不扩大捕获、不安装过滤器；本轮底栏迁移不修改此逻辑。

## 2026-10-20 上游规则候选的实体键盘回归

### 双安装对照准备（2026-10-20 后续）

- 用户授权安装两个独立版本进行实体反引号对照，并提出个人版后续发展为独立项目；本轮只隔离安装/配置/运行身份，不改按键规则。
- 上游为官方正式版 1.0.0，Tag/source revision `a4c1311d405509daf486fff3c82f7c8ec76f03fe`，安装包 SHA-256 `e36853bf4723e4c6e428120afb391fad4a059cc27b836574b5e9e9f0036632d2`。GitHub asset digest、官方 SHA256SUMS 与上游公钥的 updater 签名均通过；官方声明无 Authenticode。运行日志读回同一 revision，不等同于最新上游 main。
- 个人版隔离包仍为 1.0.0，本地 dirty 基线 `ea832bdf2579d1e2f99871bb979727c255abf779`，source fingerprint `56bdd5f8f5dec2aa651345382966d15ae7a2269bae48d793e57b25b44d9df812`，安装包 SHA-256 `39d2a579fe0112e7c01ec882194268b7dfc402959a946b477b73c5eed0d8b3ab`。包签名通过，安装 EXE 与 raw 仅有既有 NSIS 三字节标记差异；Helper/Gadget 使用原有暂存链。
- 两份安装均为独立 D 盘目录、卸载项和快捷方式；分别显示“无线麦个人版”/luck-gh 和“无线麦 SayAll”/GetSayAll。两版主/子进程均实际经 Explorer 普通权限启动；个人版日志位于 `SayAllPersonal`，上游位于 `SayAll`。旧 Roaming/Local 应用数据原位移至个人 identifier，安装前后 5 份 JSON 哈希一致，上游重新生成独立配置。之后通过个人版设置页暂时关闭登录自启；映射、语音快捷键、会话恢复及向导四份文件仍逐字不变。现有个人签名密钥原位移入个人运行目录，公钥不变，未输出密钥。
- `node --test scripts/test-personal-install-isolation.cjs` 新增 3 项先红后绿，已接入 preflight/CI。完整相关预检除初次格式检查外全部通过；随后 `cargo fmt --all` 与 `cargo fmt --all -- --check` 通过。Helper 59 passed/3 ignored，安装契约 10、清理 69、退役文件规则 8 通过；本地完整出包及前端嵌入校验通过。
- 旧个人实例在 08:10:17 UTC 正常退出、全部清理阶段 passed，旧 Agent 钩子停止，但共享宿主仍存在。隔离个人版随后出现显式开启全按键的请求；新 Helper 端口 47832 无法收到仍驻留在旧端口的 Agent，首轮 `agent_lines=0 authenticated_ever=false`，随后清理因 `recovery_connect_timeout` 保留 `unconfirmed`。08:26:48 UTC 新个人实例退出时 BLE/键态收尾通过，增强释放失败（`failed_stages=1`），Helper 最终自行退出；未强杀、未删除回执、未伪造释放。这是双安装不等于驻留 Agent 可直接交接的实证。
- 08:28:27 UTC 已打开上游原版，检查时无运行 Helper，两版登录启动项均不存在，旧宿主仍在。当前仅安装与 UI 启动 passed；上游自身 Agent 的全按键、实体反引号及两版切换实机均 **deferred**，必须先取得干净宿主环境并读回实际 Agent 身份。不能把借用旧个人 Agent 的运行称为上游复现，也不能把模块驻留误报为卸载。未擅自重启电脑或重置硬件。
- 后续用户报告：上游初装时返回/音量加减不能正常用，打开全按键后正常，实体反引号不受影响。08:30:14 UTC 原版 Helper 的真实 HELLO 为 `auth=true ... build=2026-10-20.upstream-usage`，证明复用了旧个人版驻留 Agent，而非原版载荷；CONFIG 仅含 `report_usages=0x0080,0x0081,0x00f1`，没有 TV 的 `0x0035`。上游独立配置目录没有 `button-mappings.json`，源码缺文件时采用默认映射；个人版原问题则有 TV→Tab 模板及包含 TV 的目标集合。用户观察作为当前组合下三键/键盘表现保留，不解释为“纯上游已经排除问题”。已请求保持该组件不变，将上游 TV 单击同样设为 Tab 后在空白文本框复测，以隔离配置变量；此步骤仍不能替代纯上游 Agent 验收。两个旧“无线麦”别名快捷方式已逐个核对目标后送入回收站，仅保留明确的个人版/上游入口。
- 同条件复测已复现（用户确认“变成 Tab／焦点跳走”）：上游保存的映射读回 `enabled=true`、TV single 为 shortcut/Tab，double/long 均 disabled。08:37:06–09 UTC 日志出现同一链路：Helper `usages=0x0035 buttons=tv` → 上游主程序 `map_fire button=Tv trigger=Single action=shortcut chord=Tab` → `map_inject ... last_vk=0x09 last_scan=0x0F`。因此在同一驻留 Agent 下，差异由是否接管并映射 TV 触发，个人模板管理代码不是复现的必要条件；这不是用户配置错误，实体键盘仍不应进入遥控器映射。当前组合的键盘隔离验收为 failed，纯上游 Agent 实机结论仍 deferred；不得据此宣称原始上游二进制全链路已经复现。
- 本地对照包、官方包及验签公钥片段、安装记录与最小部署脚本保留在既有 `target` 输出和 `target/dev/upstream-key-capture/`，用途是本次双版本对照/恢复；对照完成、必要结论归档并核对回退需要后再按回收站规则清理。首次部署脚本因 Windows PowerShell 5 的 UTF-8 中文路径解释失败，未运行安装器；切到 PowerShell 7 后两次安装退出码及文件/注册读回均通过。

- **failed（用户键盘共存复验）**：用户报告 Esc 下方、数字1左侧的键再次变成 Tab。当前 App 3576 于14:18启动，运行 fingerprint=`8778a888e989703c5a88dfcf6b6a4e5f58774246dc241a147aa6ca8396671e9e`，Helper 18492、Agent=`2026-10-20.upstream-usage`，不是旧 PDO 候选驻留。14:18:50 Helper 明确记录共享宿主7个成员、仅1个RC003，策略 `continue_with_usage_gate`。
- **当前日志链**：15:29:38–15:29:41 多次 Helper `usages=0x0035 buttons=tv`，随后 App `map_fire button=Tv trigger=Single action=shortcut`，并记录 `map_inject last_vk=0x09 last_scan=0x0F`。这是捕获报告→TV语义→当前快捷键Tab的链，不是系统键盘布局被改成Tab；当前模板跟随关闭且有手动记忆模板。没有在本轮重新采集物理扫描码，用户所报键位的历史实体 `VK_C0/scan29` 证据见本文件10月10日记录，不将历史探针当成本次新执行。
- **根因与旧修复关系**：此前逐报告来源核验阻止共享宿主内其他设备进入遥控器捕获；本轮将固定DLL版本门禁与WDF/PDO来源实现一起移除后，只保留报告格式/usage条件，必要的来源隔离职责没有替代实现，旧误接管再次暴露。恢复系统版本适配、捕获所有权和语音均不能证明实体键盘共存安全。
- **上游对照**：本轮只读 fetch 后最新上游仍为 `df0d3a4316f02594f30129216693b73ac2058e88`。上游与本地均放行共享宿主，使用相同的 IOCTL、输入/输出长度、操作码、report ID 与动态usage条件，均将 `0x0035` 标为TV；两边都没有在该入口核验逐设备身份。上游共享宿主日志仍称“仅F1/80/81”，但实际白名单和动态targets包含TV等普通键，不能用这条旧说明证明实体键盘不受影响。
- **离线对照实验**：复用既有逻辑台的Frida桩，将Git中的上游原始Agent和当前本地Agent分别原样加载，分别喂入两个虚构handle上的同一份9字节 `0x0035` 报告；4个组合均上报TV并在提交前清零。此实验验证两者存在同一识别缺口，不是安装上游后的真机验收，也不宣称任何设备都会复现。复现入口 `target/dev/upstream-key-capture/compare-usage-gate.mjs`，结果 `usage-gate-comparison.log`；未注入系统进程或修改配置。
- **剩余分歧的范围**：个人版的模板/菜单决定当前动作和动态捕获集合；停止回执、实例身份、持键清理、观察事件和非阻塞日志仍为本地差异。它们不提供设备来源身份，删除这些差异也不会消除本次已在上游原始Agent复现的分类结果。Tab是当前生效映射，不将某个上游测试中的Tab断言误认成上游出厂配置。
- **本轮边界**：只调查和补充证据，未更改捕获代码、用户映射或安装包，未强杀或禁用设备。修复方案尚未确定；不能用追加当前系统DLL哈希、删除TV用户映射或报告形态特判冒充可维护的设备来源隔离。当前候选键盘共存明确 failed，不能继续以“向上游靠齐”作为无回归依据。

### 旧来源修复与开关生命周期复核（2026-10-20，仅调查）

- 17:03–17:06 本地时间切回个人版后的现场复核：原安装包 `39d2a579` 的实际 EXE/Helper/Gadget 与既有身份一致，上游主程序正常退出后其 Helper 仍存活，经原有停止文件协议自行退出；个人版主程序 PID 41432 与按键子进程 50600 均为普通权限，窗口可见。用户随后报告 Agent 模板 TV 未发 Tab、Menu 打开原生上下文菜单，驱动页“关闭待完成”。只读配置确认 `rememberedTemplateId=preset-agent`、模板名 Agent、自动跟随关闭、TV 单击为 Tab、`menuTemplateSwitchEnabled=true`，不是模板丢失。09:03:40Z 的 `template_selection source=remembered ... actions_available=false`、09:03:42Z 的 `execution_mask=0 recognition_mask=0 enhanced=false`，以及 09:05Z TV/Menu 的 `configured=false fired_count=0` 构成同一证据链；本次主进程截至复核无 `map_fire`。`InputConfiguration::apply_input_capabilities` 按 RC003 的实际 captured_mask 裁剪运行期动作，不修改保存模板。09:03/09:04Z 两轮清理 Helper 均 `recovery_connect_timeout`，最新回执仍 unconfirmed、原 host 11240 仍未交接。故当前现象是接管不可用时的原生透传，不是模板选择错误；与“实体键盘被误接管成 TV→Tab”是相反方向的现象，不通过改模板、反复安装或删除回执修复。本轮只调查并记录，未修改配置或实现。
- 范围：用户要求参考此前修复并贡献上游，同时不得损害全按键开启体验；随后明确先只调查、不建 worktree。本轮只读源码、历史提交和现存日志，更新原位记录；不改捕获实现、配置或运行状态，不创建分支/worktree、提交或 PR。
- 来源隔离：`01ac762` 的 Agent 在 WDF 请求完成作用域内取得当前请求的 PDO，并按线程嵌套上下文、来源代次及所选 PDO 校验，来源不符不读改报告。这与现行报告格式/usage 判定不是同一保证；反引号与 TV 的 `0x0035` 重合时，仅靠 usage 无法证明来自遥控器。旧实现同时依赖 HidOverGatt 内部 `0x32dc0/0x32dc8` 偏移、WDF 表槽位和两份系统 DLL 精确 SHA-256，不能将其整段恢复当作跨系统版本修复。
- 开关差异：固定参考 `upstream/main@df0d3a4316f02594f30129216693b73ac2058e88`，上游 Agent 的 `disarm/bye` 暂停接管并释放逻辑边沿，`arm` 重新武装，不要求拆钩回执；App 关闭写停止信号并调用 `schtasks /end`，开启清除停止信号、重新授权后触发任务。个人版 `enable_capture` 先 `reset_capture`，等待旧 Helper/清理回执；Agent 停止须等待持键释放、拆钩并发送绑定实例及 stop_id 的 `stopped`。这是实际分歧，但不能仅凭额外等待就断言它是所有开启失败的原因，也不能以删掉释放确认或改为强杀来追求表面一致。
- 当前可读日志全量聚合：`SayAll/Logs` 的 6 个日志中，2026-10-16 09:16:28Z 至 10-20 08:10:16Z 共 17 次 `action=reset phase=completed`，全部 passed（6 次经过 recovery），未见该事件的 timeout；这只证明这些清理轮次，不证明随后捕获成功。10-20 02:07:33Z–03:35:06Z 共 187 次 `source_module_version_unverified`，与下节最初 25 次的短时间窗口并不矛盾。当天初始“等待启动”有模块门禁证据，不应误归因于清理超时。
- 历史重复开启失败确有另一条证据：本文件 10 月 4 日记录四次开启在 23–44 ms 内被旧 unconfirmed 回执拒绝，尚未启动新 Helper；它发生在后续恢复机制部署前，不能当作当前实现必然重现。此次隔离包的 `SayAllPersonal/Logs` 则有 2 次 reset passed、1 次 reset timeout，以及 4 次 `recovery_connect_timeout`；新端口无法认证仍驻留旧端口的 Agent 是本次独立对照环境中的明确限制，不将其解释为身份拆分前的全部开关故障。
- 结论边界：修复必须分别验证逐设备来源隔离与重复开启生命周期；恢复旧精确散列门禁会重新引入已证的系统更新拒绝。当前无新的硬件操作或性能常量实验，纯上游 Agent 实机、修复后的开关循环、持键关闭、冷态首用及双型号矩阵仍 deferred。只读调查不重跑无实现变化的构建/测试，不新增过程产物；必要计数和版本身份归本节，现有对照材料按上一节清理条件保留。

#### 双版本交接阻塞的代码归因

- 直接触发条件不是模板或蓝牙掉线，而是隔离后的 Helper 与仍驻留旧运行目录的 Agent 不属于同一通信身份。16:23:27 本地时间，个人 Helper 16912 新建自身令牌，扫描到旧目录 Gadget 并警告非本助手组件，随后 `STALE-TAP` 退出 13；16:23:31 下一轮 28988 因新令牌文件已经存在，进入 `Attach`、跳过注入，监听新端口 47832。16:24:01 的实际汇总为 `agent_lines=0 renews_sent=0 edges=0 authenticated_ever=false how=attached_existing_tap`，随后写入 `unconfirmed`。这不是慢握手或等待用户松键；本轮根本没有拿到 Agent 会话。
- 第一处缺口在 `resident_taps` / `decide_plan`：模块发现按 `frida-gadget*.dll` 基名匹配；目录归属不符只打印警告，没有排除该组件，计划仍只用“存在驻留模块”和“本目录已有令牌文件”决定 Attach。该粗粒度发现/假定逻辑上游也存在，不能全部归为个人新增；本次身份拆分暴露了其不适用于跨产品交接的边界。日志同轮先称“不是本助手注入”，后称“已有我们那一代 tap”，也揭示归属与计划未保持一致。
- 第二处缺口在个人版终态分类：Attach 分支没有注入，只有认证后 `serve_connection` 才调用 `cleanup.begin_capture()` 并允许 renew；但统一收尾用 `!cleanup.is_clean() || !saw_authenticated` 把本轮“从未接管”写成 `unconfirmed`。目前仅特判有先前终态证明的版本拒绝，没有区分此类外来组件/未认证启动失败。`cleanup_failed` 又先于 `no_hello` 返回，原始握手失败被清理失败遮盖。不能因此把所有未认证情况一概当成安全：已注入未握手、已有真实未释放会话及来源不明必须分别保留清理责任。
- 第三处是上述误分类的持续放大：App 每次开启先 `reset_capture`；看到 unconfirmed 就只跑 cleanup-only。恢复 Helper 仍使用个人运行目录的令牌/监听端口，而旧 Agent 的通信身份没变，故重复 `recovery_connect_timeout`，无法结清该记录，也就到不了实际启动。App 每轮等待 8 秒、Helper 等握手 30 秒使界面更早报失败，但不是端口/身份不一致的解决办法；没有实测依据，不调整常量。`ComponentSupportPanel` 优先显示 cleanup_pending，使用户只看到“关闭待完成”，看不到最初“从未接管”的原因。
- 与上游的实际差异：安装版 `a4c1311` 与所查 main `df0d3a4` 都有 Attach 握手核验及条件化宿主重建路径；个人版未采用结束宿主/设备节点恢复，并新增持久清理回执和启动前门禁。上游重建也要求宿主只有一个 RC003 成员，不适用于本机已记录的共享宿主，不能直接搬来或据此承诺恢复。此次上游 Helper 的成功则有更直接证据：16:30:14 使用原通信身份认证到同一旧个人 Agent，并发出 arm/config；它没有验证新个人身份的交接能力。上游没有本地这条持久回执门禁，不能据界面更易开启推断清理已被确认。
- 归因结论：本次持续不可用由“隔离身份未完成驻留组件交接”触发，被“未接管误记为待清理”及重试同一不可达通信身份放大。此前将其只概括为端口限制不够完整；个人版状态与交接设计存在需要修正的缺口。正确修复需分开组件归属、是否真正取得捕获资源、启动失败与释放失败；消除错误清理债务不等于已经完成跨版本交接，也不能靠删除回执、关闭释放校验或强杀共享宿主假装恢复。本轮仍只调查，未改实现、运行状态或配置；代码路径与现场日志相互印证，修复后行为尚未验证。

#### 上游整合后的终态分类候选（尚未部署）

- 在中文合并检查点 `ce7bdc0a26f28120c7ceb88abc0416c43e811744` 上保留原有 dirty 修改继续修复。Helper 终态改为：有先前有效终态证明、Attach 整轮未注入/未接纳身份/未授权，且无未知身份或多实例时，握手超时记录 `not_started`，不再以清理失败掩盖 `NO-HELLO`；已经注入、缺先前证明或存在未知身份时仍保留 `unconfirmed`。旧 `agent_version_blocked` 与真实停止 ACK 的 `passed` 继续分别记录，不把启动未取得资源称为撤钩成功。
- 驻留计划必须先完成归属核验，再读取或创建本目录令牌：唯一 Gadget 的真实规范路径须在当前 runtime 根目录内，外来/多个/路径不可核验的模块显式停止；模块枚举失败不再当作“没有模块”注入。新增外来模块回归先实际生成令牌并以错误 `STALE-TAP` 失败，再经最小修正通过；覆盖相似前缀的兄弟目录、已存在令牌不被读取、唯一 owned 可 Attach 与干净目录首次 Canonical。
- 独立审查后收紧先前证明：回执必须关联当前 host PID、创建时刻及有效 Agent instance；异 host、PID 复用、缺失资源身份或未知实例不构成未捕获证明，已拒绝版本的 HELLO 还必须匹配先前 instance。无捕获终态继续保留该已证明资源身份，避免后续重试丢失关联。模块枚举在 `Module32NextW` 返回 FALSE 后立即保存 GetLastError，只有 `ERROR_NO_MORE_FILES` 视为正常结束；中途失败不得将已枚举的局部列表当作完整快照。
- Helper 全套 **66 passed / 3 ignored**、独立 crate `cargo check` **passed**；矩阵覆盖缺先前证明、已注入未鉴权、已接纳未停止、未知身份不放行、真实 loopback 监听独占及释放后重绑、同 Agent 停止失败恢复与成功停止后再次开启/停止（每轮真实收到 `arm`）。新增异 host/PID 复用证明与部分模块快照两个回归均实际 RED 后 GREEN；部分快照不得读取/创建 token。清理日志增加取得资源/鉴权/先前终态/身份未决的布尔事实。独立 crate 格式检查仍因既有身份隔离的4处格式漂移 **failed**，本轮新增段已依检查输出修正，不扩大修改旧段。完整构建和真实按键验收仍 **deferred**，不据单测判定接管已恢复，不自动提交这项未闭合修复。
- 驻留交接仍是独立阻塞：当前 Script 配置未启用 `on_change=reload`，Agent 协议没有脚本重载或端点迁移命令；[Frida 官方 Script 契约](https://frida.re/docs/gadget/#script) 明确默认只加载一次，配置中的 `teardown` 默认也不是完整卸载。修改磁盘脚本/配置不能证明旧实例换代，不能手工卸载旧模块或创建第二个 Gadget 来冒充修复。是否允许跨产品共享捕获端点尚待产品边界确认；本候选未读取外来令牌、未实现旧目录 fallback、未改变运行态或清理旧回执。

#### 2026-10-21 明确授权的临时驻留资源借用

- 用户明确今晚不能重启，并批准本次临时借用已驻留旧组件及原端点；安装身份、用户配置和个人任务保持独立。只允许 Helper 内部读取现有凭据、连接个人 bridge，不输出凭据、不覆盖旧 script/config、不终止宿主；不是遇到外来模块就自动共享的产品策略。
- 跨日提升只读复核仍为同一 host `11240`、创建时刻 `134369504523963312`，与待清理回执一致；唯一 Gadget 来自旧运行目录，无 Helper、两个产品端口均未监听。冻结 DLL/config/script 的大小、SHA-256 与修改时间，不读取或保存凭据原文。
- **真实正常恢复 passed**：02:02:49.553Z 新 Helper `64436` 等待 Agent，02:02:50.061Z 请求停止，02:02:50.065Z 记录 `result=passed capture_authorized=false`。当前恢复实现只有 TCP 双端进程、host 创建时刻、build、instance 与本次随机 stop_id 的 `stopped`（hook_detached/released_all）全部匹配才进入该终态。个人回执从旧 `unconfirmed`/空 instance 变为本轮 Helper、新时间及已知 instance 的 `passed`；Helper 正常退出，端口释放，旧三文件 SHA-256 与修改时间逐项不变。不是删除旧回执，也没有启动捕获或证明 UI 已恢复。
- Attach 只读候选已先红后绿：旧实现实际覆盖合成 script、在 attach-only 中生成 token 后失败；修正后 Attach 在任何运行文件写入前直接返回，attach-only 禁止 new-token/new-generation、缺少 resident/现有 token 时显式失败。默认任务不改变，借用日志和 panic 经过 descriptor 的唯一 log 字段核实为个人日志位置，显式 `--log` 仅作个人位置的备用落点。
- UI 闭环候选使用人工授权流程在当前用户个人目录原子创建的 `rc003-resident-binding.json`，仅含 host PID/创建时刻、Agent instance、模块绝对路径和端口，不含 token。Helper 程序只读该文件，不自动创建、改写或寻找旧目录；严格拒绝非法/部分/多余字段。活 host 必须唯一真实 canonical 模块一致，强制 Attach，持有个人默认端口单例锁及借用监听端口；恢复和捕获继续校验同实例，捕获握手增加实际 TCP owner 核对。进程对象证明旧 host 已结束/PID 被复用时，仅记录绑定失效并回到原独立流程，仍须满足零 resident 和原清理门禁；其他身份/观察异常明确停止，不将失效等同清理成功。该文件不会随安装 payload 分发，人工流程在本次借用结束后核对再回收。
- 绑定与实例错配回归均实际 RED 后 GREEN；Helper **72 passed / 3 ignored**、独立 `cargo check` passed，覆盖严格输入、异 host/PID 复用、多个/不同模块、端口独占、取消时不发命令、同实例停止失败恢复及成功后再 arm/disarm。格式仍仅保留前述4处既有 dirty 漂移。独立审查无 P1/P2；个人任务 principal 与普通 App 用户一致，任务仍指向正常安装 Helper，未添加旧 runtime/端口默认参数。
- **正常整包安装 passed**：显式 local/bundle 环境构建 x64 NSIS 包（SHA-256 `91d3a00d832e29e9d8aa6d7402cb974dd5a260239f4464bbb6185fecc2a60fa6`），02:33Z 通过正常退出事件等待旧 App/按键宿主退出后管理员覆盖安装，不强杀。5 个 JSON 配置文件安装前后字节保持；已安装 App 与 NSIS 标记归一后的构建载荷相符，4 项 staging 载荷散列一致。Explorer 普通用户启动的 App `61684`/按键宿主 `58356` 均未提权，运行身份为 base `ce7bdc0`、dirty fingerprint `61da086dada5209dc645af01d546145b6dd9c87f26af98339d7f6eece1ab44a9`、local、payload included；不是 pure-index 构建。首次遗漏构建环境的产物未安装，已由本次正确包覆盖，不引用其验证结论。
- **实际 UI 关闭再开 passed**：首次 UI 开启选择人工临时绑定，Helper `25160` 认证当前旧实例并取得 ownership；02:35:00.130Z 新停止确认记录 `hook_detached=true identity_match=true unresolved_multiple=false`，随后 CLEANUP passed、Helper 正常退出、UI 关闭完成。再次 UI 开启由新 Helper `54808` 选择同一绑定、只读 Attach，02:35:15.661Z 认证实际旧 Agent，02:35:16.163Z ownership generation=4/mask=7167；没有以单次 CLI 接管替代 UI 验证，也没有修改超时常量。
- **RC003 核心按键及闲置后语音 passed（用户实按确认）**：用户确认当前 TV→Tab、Menu 模板菜单、返回/音量±、闲置后语音出字及松开结束均正常。本窗口语音 generation=5/6/7 均实际 `terminal=control_stop`、`decode_errors=0`，输入角色恢复均 passed。菜单另有 `target_restore_failed/menu_retained` 日志，不以菜单开关正常扩大为所有目标窗口焦点恢复通过。02:43:05Z 提权只读复核仍为同 host 代次、唯一模块，旧 DLL/config/script 的大小、SHA-256、mtime 全部不变；当前一个 Helper 持有个人单例端口及借用端口，无并起第二 Helper。
- **后续三项用户效果 confirmed passed**：用户明确确认持方向键关闭后松开无连发/卡键、开启处理中关闭正常、重新开启后托盘正常退出再开首键与语音正常。日志 observed：03:25:45.741Z Helper `29904` 新停止确认、CLEANUP passed，03:25:46.094Z App 观察 helper_count=0/reset passed；03:26:08.645Z Helper `31120` 同样新确认后退出、reset passed。03:26:39.420Z Helper `52048` 新停止确认及 CLEANUP passed，03:26:39.579Z App reset 观察 helper_count=0/completed passed、BLE cleanup completed；新 App `60000` 03:26:45Z 启动 reset passed，03:26:55.798Z 重获 ownership=7167，03:27:00.679Z 语音 generation=1/control_stop/decode_errors=0。无卡键、首键及出字效果以用户确认计，不由日志单独推出。
- **日志边界与残留状态异常**：本次“开启处理中关闭”的两个 enable 在日志中均先 completed，随后才 disable，故用户效果通过不等于已观测到真正 in-flight cancel 竞态。03:26:39.509Z 并发 status 曾报 `process_observation_failed`，前端随即报 cleanup_blocked；真正退出 reset 随后 passed，不能把短暂状态错误写成真实清理失败，也不能隐藏误报。源码进程观察仅对 OpenProcess 的 INVALID_PARAMETER 放弃旧条目，路径/创建时间查询失败均报错；缺少失败 API 阶段/Win32 码，具体失败点尚未证明，不猜定根因。较早 02:46:53–02:47:11Z 另有 multiple_helpers、旧 Helper 退出未确认及停止未确认、随后恢复 ownership 的窗口，发生于此次三步骤前；没有证据把它归给用户此次动作，亦未证明该旧窗口的竞态已解决。需最小日志/回归进一步区分终止中进程与真正身份观察失败，不用更长等待或 UI 掩盖。
- **退出观察追加修复（上述首轮实装验收之后）**：仅继续修改 `src-tauri/src/rc003_task.rs` 的进程观察边界。先补阶段/Win32/同句柄退出查询诊断，独立受控进程自然 exit0 后保留原 handle，实际观察 `stage=image win32=31 exit_code=0`；原行为仍阻断，新增3项回归实际 RED。修复只有同 handle 的 GetExitCodeProcess 成功且非 STILL_ACTIVE 才忽略已结束条目；仍活或退出状态未知继续 block，路径/时间失败不单独作为退出证明。PID 复用仍以新创建时间关联回执；仅名称命中 Helper 后才处理其 session 错误，已知其他 session/无关名称不扩大查询。Process32NextW 仅 ERROR_NO_MORE_FILES 为正常结束，中途错误不得交回局部列表；session 未知/部分枚举两个回归另实际 RED 后 GREEN。所有观察失败记录 API 阶段与 Win32 码，不调 timeout、不读内存、不终止进程。
- 该追加修复宿主 **181 passed / 1 ignored**（进程/任务组37项）、工作区普通/仿真 check 与工作区 fmt passed；未受影响 crate 复用本轮紧邻工作区全套61/4/590（21 ignored）等通过证据，Helper72项对应代码未变不重复跑。隔离测试子进程内实际 `cancel_pending_start` 与延迟完成回调用通道同步，确认取消后不持久化/激活，未触碰真实任务或 stop 文件；这补自动化取消竞争覆盖，不替代真实 UI in-flight 观察。修正后的自然退出实验实际记录 image/31、同句柄退出0、`outcome=ignored`；历史03:26现场无失败码，不能据此断定其必为同一 API 错误。旧02:46窗口有限日志核对见三个临时绑定 Helper PID、无 TASK 标签，不能把它解释成已证明的注册助手误计数；多 Helper 仍明确阻断，未宣称该历史窗口已修复。独立审查允许部署，生产改动随后保持冻结。
- **追加修复整包部署及最小差异复验 passed**：明确 local/bundle 构建、前端嵌入门禁和实际签名校验通过，当前 x64 包 SHA-256 `b67d1757946d794a69302630e8e15f840e1d99f9e0023e0e1934886a4efc283a`。03:56Z 旧捕获先取得同 Helper/started/host 代次/instance 的新停止 ACK 与终态，再确认 App/Helper 退出及两监听端口释放，才管理员覆盖安装；本次6个 JSON 配置安装前后字节保持。实际安装 App SHA-256 `ca88fb9c9da5f90a40778d96c63726c147a9f2941fe32bf54349cc2ee598a08f`，4项载荷与 staging 一致；Helper SHA-256 仍 `d6fc8191c14d5096f4b59801f0267eb3859ef5e035d85d38dfc4aa99a5e6c874`。Explorer 普通权限 App/按键宿主的真实运行 fingerprint 为 `ea4b3e793e5b84f327b82c5b1d96e258f7720ce3b3ddf935f3a4571ea530b66e`，base `ce7bdc0`、dirty、local、payload included。
- 新版实际主窗 UI 关闭后 Helper `64108` passed 并退出/释放端口，再由 UI 开启新 Helper `66720`，取得 generation=4/mask=7167，页面实际“已开启”。本地 UIA 工具起初将 `MainWindowHandle` 的同 PID 场景菜单误认作主窗，未执行任何开关；改为当前 PID+精确主窗名称定位后完成上述动作，未改变产品或采用盲坐标。04:04:16Z 起通过真实驱动页“刷新状态”9次与自家正常退出事件并发，取得同实例新停止 ACK、回执 passed，App/Helper 正常退出、两端口释放。实际诊断 04:04:18.461Z 为 `helper_pid=66720 stage=complete win32=0 exit_code=0 process_state=exited outcome=ignored`；该窗口 process_observation_failed=0、process_observation blocked=0，不把成功观察到退出等同重现历史未知失败。04:04:59Z Explorer 重开新普通 App `66480`/按键宿主 `37332`，同 fingerprint，新 installed Helper `63276` 的实际路径/散列匹配载荷，ownership=7167；04:05:49Z 提权只读复核同绑定 host 代次、唯一模块、旧 DLL/config/script 的 SHA/mtime 不变，当前仅一个 Helper。
- 用户物理键/闲置语音及三边界的 visible confirmed 证据属于前一 `61da086d` 实装；当前 `ea4b3e79` 只追加进程观察边界，未改输入/映射/语音路径，完成的是 UI 开关、并发 status 正常退出与重开差异复验，不伪称再次做了所有实体按键/语音测试。真实 UI in-flight cancel 仍未捕捉，维持自动化受控竞争 passed、硬件 deferred。当前正常会话保留，无安装/构建/恢复在途；修复依赖旧 dirty 身份/bridge 契约，未单独提交，也未启动过滤器 PoC 或改实体反引号算法。
- 提交前补核 Agent 字节边界：当前源码 SHA-256 `4d114348d961a98e9d3fdd6fab55155befe9f76931c7b406e6fdbe539ce863f3`，已冻结**旧 runtime 磁盘 script** SHA-256 `180a502acbdf55cf7273090b250009f1375cf5d0ce81f344468fb96544940863`；原字节及统一 LF/CRLF 后均不匹配。有界只读 AST/diff 比较证实不只是注释：磁盘版 build=`2026-10-05.gate-delay-units`，缺候选中的 instance/stop_id 及完整持键清理等协议；真实 HELLO 则为 `2026-10-20.upstream-usage`，不能把这份磁盘文件当作驻留内存来源证明。Helper 使用编译期 include_str 并直接写入 AGENT_JS；ignore 模式下磁盘可曾重写而内存仍旧，运行 build/instance 也不是内存脚本完整字节散列。本轮实机覆盖当前 Helper/App 与已标识运行实例的协议和效果，不证明当前 Agent 源码全量新注入通过。HEAD 的 `2026-10-10.request-source` 源码还要求 source_pdo，不能与当前无 PDO 的 renew/已移除模块门禁混搭；因此提交需包含配套 usage 基线源码和89项逻辑回归，但新注入硬件验证仍 deferred，不冒充反引号修复。
- 这是当前明确 host 代次的临时接管恢复里程碑，不是永久独立升级/在线脚本换代已解决；三项用户确认不消除上述日志限定。RC001、跨 Windows 构建/非开发 PC、ARM64 及真正 in-flight cancel 竞争覆盖仍 **deferred**，整项仍不勾选。不阻止后续只读调查来源问题，不提交依赖旧 dirty 的片段；Helper 候选依赖此前尚未提交的个人身份、bridge/清理回执契约，不能以提交 main.rs 整文件夹带这些旧修改。
- 现有旧 Agent 仍为 ignore 模式，本轮没有装入新的来源隔离脚本；实体反引号问题明确仍未修，当前旧驻留脚本不能作为后续新脚本修复的验收环境。最小过程证据复用 `target/dev/upstream-key-capture/`，实际当前包为 `target/release/bundle/nsis/无线麦个人版_1.0.0_x64-setup.exe`，最终结论归本节，未提交或发布；当前会话刻意保持开启，无在途安装、恢复或后台构建操作。

#### 2026-10-21 来源隔离的只读可行性结论（未实施）

- 用户要求实体反引号不再进入 TV→Tab 映射，当前明确仍 failed；本轮只读调查没有修改 Agent、用户映射、驱动或系统安全设置。旧 request→queue→device→PDO 逐报告语义曾有精确来源实证，但取得请求、driver globals 和函数表的旧入口依赖私有偏移，不能把“使用公开 WDF 方法”外推为整个入口均有公开稳定契约。
- 本机磁盘 DLL 的公开 PE 导出只读核查：`WUDFx02000.dll` 仅有 `FxFrameworkEntryUm`、`Microsoft_WDF_UMDF_Version`；`HidOverGatt.dll` 仅有 `FxDriverEntryUm`。未发现所需请求操作、函数表或 driver globals 的独立导出。微软公开头通过 `WdfFunctions[slot](WdfDriverGlobals, ...)` 调用；公开请求 API 以驱动已取得有效请求/框架绑定为前提。本轮未找到供外部 Agent 枚举既有驱动绑定、从当前私有 IOCTL 取得 `WDFREQUEST` 的公开支持入口，不宣称已证明所有可能公开路线均不存在。PDB、特征扫描或反汇编定位仍是私有实现依赖，不能包装为不受 Windows 更新影响的修复。
- 设备专属 UMDF 2 过滤器与仓库既有 KMDF 候选是**待决策、未验证的研究方向**，不是本轮已授权改造计划。微软支持 UMDF 设备专属上/下过滤器，但有限制和 INF 层位契约；本机现有 `hidbthle.inf` 为 HidOverGatt UMDF 栈，`AllowKernelModeClients`、MethodNeither Copy 配置只说明值得评估，不证明新过滤器已能安全接入。既有 KMDF 候选仅覆盖三键 mask=7，未签名/未加载，不能当作全按键方案已具备。
- 两个候选均须另行决定与授权后，先证明独立安装/卸载、微软 base driver 与其他 filters 共存、合法签名和正确请求/报告完成层位，再谈零改写观察及实际 TV/键盘隔离；本轮不安装测试驱动、不改安全设置，也不把“需评估”登记成实施承诺。官方来源及适用边界归 [ATTRIBUTION](../ATTRIBUTION.md#2026-10-21-来源隔离只读研究边界)。当前旧 ignore Agent 无法热载新来源脚本；临时接管恢复不构成反引号修复验收。
- **同日层位核实**：只读 `Get-PnpDevice` / `Get-PnpDeviceProperty` 按既有 INF 的精确 RC003 硬件版本匹配到一个 collection；匿名属性链为 `Keyboard/kbdhid`（无设备 Upper/LowerFilters）→ `HIDClass/mshidumdf`（LowerFilters=WUDFRd）→ `Bluetooth/BthLEEnum`。未输出实例路径或读取凭据；这是设备属性，不是全部已加载对象/类过滤器的枚举。当前 `hidbthle.inf` 声明 `UmdfServiceOrder=HidOverGatt`。UMDF filter 要列完整服务顺序，Extension INF 会覆盖同名设置且多扩展应用顺序不确定，未找到无覆盖追加既有 UMDF 服务列表的支持契约；不能用 AddFilter 代替此 UMDF 内部顺序，也不创建猜测 INF。
- **旧 KMDF 候选不能直接改 Upper**：`drivers/SayAllInput/driver.c` 是 `EvtIoRead`→下层 `IRP_MJ_READ` 完成链，IOCTL 只用于查询 collection 描述符及私有控制，非 `IOCTL_HID_READ_REPORT` 捕获。原 INF 的 LowerFilter 位于 kbdhid 下方的 TLC 栈；这不同于 HIDCLASS 与 transport minidriver 之间，不能混用微软对后者的“不推荐”结论。但微软明确列举的键鼠过滤位置是 kbdhid/kbdclass 上方；改为 Upper 后不能假定仍收到同一份 121 字节 HID 报告。现 mask=7、HidP 合成验证、取消/租约/持键状态机与旧离线测试仍不证明 TV、实际完成层或 WDF 移除时序通过。
- **收敛候选（未选择、未实施）**：仅 TV 使用精确 RC003 TLC 的设备专属 KMDF UpperFilter，采用公开 `IOCTL_INTERNAL_KEYBOARD_CONNECT` / `CONNECT_DATA` / `KEYBOARD_INPUT_DATA` 回调链，在 kbdhid 转换后、Kbdclass 消费前处理；来源来自该设备的回调上下文，不靠全局 UnitId、时间窗口或最近遥控活动。其余键继续既有 Agent；这只解决 TV/反引号目标，不宣称全部共享键的来源隔离。微软 kbfiltr 样例原本针对 PS/2，只借鉴 CONNECT、服务回调及私有 raw PDO 模式，不能照搬 i8042 ISR/初始化或样例安装文件。TV 在此层实际 MakeCode/Flags 及 DOWN/UP 仍未观察，不能将预期 scan=0x29 当作实证。
- **最小代码/离线边界**：若用户选择新增签名组件，才实现设备局部 TV 边沿、原 callback 转发及私有通道，复用现有 WDK/MSVC 和应用映射引擎，不重写 HID 解析或在驱动硬编码 Tab。RED/GREEN 应覆盖其他设备同码透传、MAKE/BREAK 与重复、E0/E1 区分、混合批次与下游部分消费、持键启停/失租/移除、取消后 UP 归属和通道溢出；已有三键报告测试不能替代。当前仅核实锁定 WDK 头存在及 `scripts/build-input-driver.ps1 -TestsOnly` 的离线入口，未改源码、未新增测试或重跑旧构建，未声称新候选编译通过。
- **切换与部署门槛**：现 Agent 源码支持从 report/clear 两集合排除 `0x0035`，混合报告只清目标 slot；这不证明当前旧驻留内存字节。未来切换必须先正常 stop 并确认旧持键释放，再验证新 targets ACK/目标读回及 TV 原生透传，才允许新通道取得 TV 所有权，不能边持键边双捕获。新增内核组件、正式签名渠道/成本与部署均待用户分别决定；不得开启 TESTSIGNING、关闭安全策略或强制重启设备。安装可能要求设备重新启动乃至系统重启，今晚不能重启时保持 deferred。放行前须有签名验证、准确层位、TV→Tab/实体反引号并发、三键及实时语音回归、取消/持键退出、冷态首用、安装卸载与不同 Windows build/非开发机证据；RC001 另验。本轮未改变当前 App/Helper/host/任务/binding/runtime，没有在途构建或安装。

#### 2026-10-22 已选择签名过滤器路线的离线观察候选

- 用户已选择独立、正式签名的设备专属键盘过滤器路线；当前仅授权最小源码、编译和离线验证。新增 `drivers/SayAllTvKeyboard` 与 `scripts/build-tv-keyboard-driver.ps1`，不修改旧 `SayAllInput`、当前 App/Helper/Agent 或生产 targets。没有 INF、CAT、签名、安装/设备写入，未触碰任务、binding、运行实例或安全设置。
- 最小成果是可编译的真实 KMDF CONNECT/service callback 与生产共用观察状态机，**零 packet 改写、零吞键、无用户态捕获通道**。显式设备局部候选 profile 才匹配，缺失不判 TV，坏值显式拒观察并透传；`0x29` 只存在于合成 fixture，不作为当前遥控 TV 事实。设备归属来自回调上下文，不使用 UnitId/时间猜测。
- TDD 实跑：先以无实现观察器在 `TvObserverConfigure` 断言 RED，再补实现 GREEN；同一 `driver.c` 的回调空实现使原上层/原缓冲转发断言 RED，再补 callback GREEN。最终两套 EXE passed：纯观察器覆盖 profile/设备隔离/边沿/重复/flags/持键停止；生产 driver harness 覆盖 CONNECT 成功、发送失败/下层取消回滚、重复/短缓冲/空字段、混合/部分/零消费与后缀重试、物理同码/Shift 原样透传、坏 profile、D0/重复移除、上层重入/换代迟到与越界消费。harness 的 NT/WDF 是测试替身，不是内核执行证据。
- 独立代码审查发现初稿普通内部请求仅发送、没有 completion 或 `SEND_AND_FORGET`，可能悬挂；旧 harness 只断言已发送而未模拟下层完成，掩盖了该缺口。已扩展替身与回归，先实跑 RED（原请求未完成），再用最小 `SEND_AND_FORGET` 修正透明分支。普通请求同步/异步成功或取消四组合均由下层恰好完成一次，发送失败由本过滤器完成一次；CONNECT 仍为独立 completion，非 forget，覆盖同步/异步完成及失败回滚。离线两套与 WDK 重建通过；独立复审已关闭该 P1，离线观察 PoC 收口，硬件/部署与反引号修复仍未验收。
- `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/build-tv-keyboard-driver.ps1` 实际通过离线两套测试，以及本机已有 WDK `10.0.26100.6584` / SDK `10.0.26100.0` 的真实 x64 KMDF 编译链接（`/W4 /WX /kernel /GS /guard:cf`）。被测基础 HEAD `7f6a777c3d5d87c5086e57f2c7b47047ad2a0686` 不能还原新 dirty 源码；修正完成归属后的精确 C/H/测试/构建脚本组合指纹 `13f78071974cbc2837d73982640a96059e6e89b689b4a099c8470f04c5710f33`，SYS SHA256 `460003ec893a421efe0967d0a8937163e4063f50d461b3a54d72415c9a1bab63`。`source-identity.json` 记录输入相对路径及逐文件 hash，指纹编译进 ETW；文档变化不改变该二进制输入身份。
- PE 只读核对：x64 Native、ASLR/NX/CFG，导入仅 `ntoskrnl.exe`/`WDFLDR.SYS`。`signtool verify /kp /v` 实跑 exit=1、0 文件验证通过/1 error，Authenticode 状态 `NotSigned`，是预期的签名拒绝证据，不是加载通过。当前 SYS 仅供编译/代码复核，未安装。无提交/推送。
- 内核 provider `WirelessMic.TvKeyboard` 的非分页 typed ETW、匿名 ordinal/sequence、候选已消费边沿与取消/重叠/迟到计数已实现并编译。provider 注册/注销控制流通过同源码 harness；真实 provider 启用、ETW 丢失率、DISPATCH_LEVEL 调度/锁、WDF 取消/移除、准确过滤层、TV MakeCode/flags、原生抑制/模板 Tab、冷首用/语音及 RC001/RC003、跨 Windows build/非开发 PC 均 **deferred**。重叠或换代产生明确观察缺口，不伪造 UP。
- 当前实体反引号回归仍 **failed**，不是本阶段已修。后续须另授权合法签名与精确设备绑定/加载，先以实际操作、候选匹配/消费及 EventsLost 验证只读观察；捕获还须独立消费者/成对取消/溢出处理和旧 Agent stop ACK、释放、排除 TV 后的所有权交接，不能从此只读候选直接启用吞键。完整契约及引用归 [候选说明](../drivers/SayAllTvKeyboard/README.md) 与 [ATTRIBUTION](../ATTRIBUTION.md#2026-10-22-设备专属键盘观察候选)。
- 最小构建日志、测试 EXE、当前 SYS、PE/签名检查与身份记录复用 `target/dev/tv-keyboard-filter/`，用于本候选独立 review 与可复现实验；没有源码副本或安装包。候选被替代且独有结论已归档后按回收站规则清理，其他任务材料不动。

## 2026-10-20 系统模块变化后恢复上游捕获规则

- 同日后续 UI 候选 `8bc6d9c4` / fingerprint `8778a888` 已替代本节最初安装包：用户报告按住语音键时“请松开语音键后更改会话输入设备”导致下方整体位移。来源为个人版条件插入反馈网格行，上游无此功能；仅移除重复忙态行，保留禁用控件、原位状态及错误/恢复。418项前端、48状态实际浏览器布局和安装版三次实按位置观察通过，用户确认布局与语音正常。完整新包身份及证据原位归 [输入切换专项](../Testing/CaptureInputSession.md#2026-10-20-语音忙态不再推动下方布局)，下文最初候选的故障与证据仍按原身份保留。

- 状态：实现、自动化、本地构建安装及真实接管通过；用户初次复验语音失败，已定位到保存的音频两端失效，经用户授权重新绑定后，通道就绪、实际语音及松开结束复验通过。后续实体键盘反引号→Tab回归 failed，见上一节；其他物理按键与完整矩阵仍待验。
- 现场身份：安装版 1.0.0，基线 `8b9dd31460f1b3b4c6859b2216de61ede0966206`，dirty/local/included，fingerprint `6ac22ec4cf4c22228a306a690f848dee8aef33014bf798991f673280d022e304`；Windows 26200.9711 x64。
- 现象：蓝牙已连接，驱动页全按键停在“等待启动完成”；本地时间 10:07:33–10:13:57 记录 25 次 `source_module_version_unverified`。计划任务只读结果为退出码 16，主程序稳态每 30 秒再次触发。
- 已确认根因：Helper 将 HidOverGatt/WUDF 两份系统 DLL 的完整 SHA-256 固定为已验证版本；当前分别为 `09648a65…bd61ad`、`7fdd6b19…26e7ff`，均不匹配，Authenticode 签名均 Valid。系统显示当天安装 KB5127690/KB5127648，但没有将某个具体 KB 认定为两份文件改变的唯一原因。Helper 在来源校验失败后直接退出，未建立桥接或写本轮启动终态，App 因而保留等待并重试；这与蓝牙是否连上不同。
- 用户决策：采用固定上游 `df0d3a4316f02594f30129216693b73ac2058e88` 的捕获规则，个人版专注模板等独立能力。此决定替代旧逐报告 PDO 隔离实现；10 月 10 日误接管的实证保留，同宿主同格式同 usage 的设备仍不能由内容规则区分，不伪称该限制已解决。
- 语音复验 **failed**：用户报告“语音按钮无法正常工作，UI 闪烁一下”。按当前及轮转日志聚合，新 App 52960 的 generation 1–6 均在音频 begin 阶段以 `endpoint_not_selected` 拒绝；12:27–12:28 的 ATVV START 已观察到，随后会话中止且音频包为 0。启动配对先报 `capture_missing`，继而清空运行态 writer；换包前 App 32316 / 33272 于 09:50 / 10:08 已报同一缺失，不归因为新 Agent 未收到语音键。
- 音频只读核对 **passed**：自身设置保存的 Capture 与 Render 在 Windows MMDevices 均为状态 4（NOTPRESENT），现有活跃 CABLE 两端的身份与保存值不同。PnP 同时存在一个正常和一个启动失败的 VB-Audio 节点；未证明失效节点或系统更新是端点变化的唯一原因。复用当轮测试二进制执行 `audio_route::tests::actual_vb_cable_topology --ignored --nocapture`，标准 writer、显式 16ch 的同适配器配对均通过（2 passed），系统默认设备写入 0、音频流打开 0。
- 授权后恢复通道 **passed**：用户允许重新绑定当前 VB-CABLE，12:39:42 通过现有 UI 选择活跃 Capture，应用自动配对同适配器标准 writer；实际 WASAPI client 初始化、restore、配置保存和前端 paired 均通过。仅更新本应用的两端身份/名称，输入切换保持启用，模板、快捷键、向导和 journal 哈希均不变。配置操作未修改系统默认麦克风、未重装驱动，没有新增实现。随后用户确认两次输入与松开结束正常；12:41–12:42 实际六段会话均提交音频并以零队列正常结束，原三角色均恢复，journal 最终为 null。此次首用与连续语音恢复 passed，完整型号/闲置/睡眠矩阵仍 deferred；具体证据归 [CaptureInputSession](../Testing/CaptureInputSession.md#2026-10-20-保存端点失效与语音复验失败)。
- 最小实现：删除 Helper 的固定模块校验、PDO 查询和来源续约字段，以及 Agent 的固定 WDF 表偏移/来源作用域/计数；直接按导出符号挂钩，继续检查外部 IOCTL/报告结构和动态目标。保留停止回执、持键 UP、实例身份、非阻塞日志、模板/菜单和原 ATVV 路径。新代次 `2026-10-20.upstream-usage`，新增 `[CAPTURE] policy=upstream_usage_gate system_module_binding=false` 说明实际策略；没有追加新哈希或旧实现 fallback。
- 已执行验证：旧 Agent 基线 95/95 passed；更新契约后，旧实现因无法仅凭 ntdll 导出安装捕获钩子而失败；生产实现修改后 89/89 passed，包含普通续约、动态目标、非目标/格式不匹配报告与原停止/释放用例。删除的来源隔离断言不再代表当前契约。`scripts/verify-rc003-helper.ps1` passed（Helper 59 passed / 3 ignored，Agent 与两套 fmt）；`scripts/ci-preflight.ps1` 11 步 passed：前端 416、Rust 工作区 740 passed / 22 ignored（子进程结果不重复计数）、workspace check/runtime-simulation check、覆盖安装契约 7、清理状态机 69、退役路径 8 均通过。日志保留现有无关 warnings，未顺手修改。
- 当前 Windows 只读实测：11:25:00（UTC+08:00），新 release Helper PID 7136 执行 `--dry-run`，实际日志记录 shared host 7/1、`continue_with_usage_gate`、`system_module_binding=false`，宿主定位/Gadget 校验/主程序桥描述读取通过；未准备运行目录、未注入、未监听。此证明当前系统文件下不再被旧哈希拒绝，不证明按键已接管。
- 本地包部署 `passed`：`scripts/build-local-test-package.ps1` 生产构建、前端嵌入内容门禁、个人更新公钥实际验签通过；x64 安装包 SHA-256 `84ecdbbab1e34acef01252a073d20366971300dacbfc7f00bf88965a396cc1a6`，基线 `ea832bdf2579d1e2f99871bb979727c255abf779`，dirty/local/included，fingerprint `89427d24b945b4678df9026527bd4b9c2bdc3cdf91774ecf9e72c3529513214e`。11:35:07 正常退出旧 App，overall passed / failed_stages=0 / 340ms，清理与 BLE 断开均完成，没有强杀；管理员覆盖安装的 installer-result 为 completed，四项载荷匹配 staging。安装 EXE SHA-256 `458a1587bd517a91162322be83685bf9bd4936d644f4bff8554d76fe10a1dc30`，与 raw EXE 同为 21,027,840 B，全字节比对只在 `0xEFB442` 有三字节 `UNK→NSS` 标记变化。四份存在配置和原本不存在的 other-voice-hotkey 文件状态全部保持。部署脚本首次将没有 native exit code 的纯 PowerShell 校验误判为失败；修正调用方后只重做读取核验，没有重复安装。
- 实际运行与接管 `passed`：11:36:31 通过真实 Explorer desktop 的 `Document.Application.ShellExecute` 启动，App52960/子8060 均 TokenElevation=0、实际路径为安装目录，运行五项身份与包一致。Helper36652 在当前宿主11964完成鉴权，HELLO build=`2026-10-20.upstream-usage`；11:36:34.260 当前 generation2 实际 `ownership_resumed owned_mask=7167`（12个当前目标键），11:36:35.230 前端状态转为 `active/passed`，11:36:37.059 常驻对账确认 `helper_connected trigger_attempts=1`。这证明原先“等待启动完成”的链路已恢复，不以计划任务退出码替代实际接管。
- 待验：RC003 首用/连续/快速释放/菜单/键盘共存；RC001、ARM64、睡眠恢复不由本机软件测试外推。当前只生成 x64 载荷，未具备 ARM64 构建工具链。`agent_selftest.py` 因当前 Python 缺少 frida 绑定退出 2，传输台 `deferred`；Node 逻辑台和安装版接管不能替代全部真实传输故障实验。未提交、推送或发布。
- 过程材料复用 `target/dev/upstream-key-capture/`，只保留支撑当前失败/对照的最小日志至本候选验收闭合或被替代；长期事实以本节为准。未复制用户配置或完整诊断日志，正文不包含设备身份、语音或个人路径。

## 已冻结的现场事实

以下时间为 UTC（本地时间加 8 小时）。按 App PID 51996 的完整会话聚合，再定位 13:18–13:24 窗口；不能仅凭尾部两次成功排除前面的失败。

| 路径 | 可观察事实 | 结论边界 |
| --- | --- | --- |
| 菜单 | 13:19:05.586 与 13:19:39.165 从 `before=main` 打开；随后合计 6 次 `target_restored=false`、`close blocked menu_retained=true`，又显示菜单 | 确认关闭被返回焦点失败阻断；原日志未给出 Win32 失败分类，不能断言旧窗口已销毁 |
| 菜单边沿 | 整轮执行边沿 25 DOWN / 25 UP 严格交替 | 未见物理观察 R 与执行 E 双路重复触发菜单的证据 |
| 报告传输 | Menu DOWN 的 Agent 时间 13:19:06.135，Helper 收到于 13:19:08.155，迟到 2020 ms；对应 UP 迟到 1834 ms。另一次 UP 迟到 4271 ms | 证明 Agent 生成后的链路迟到；尚不能区分调度、日志 IO、socket 消费等具体阻塞点 |
| 捕获失联 | 13:19:13.862 与 13:19:51.852 App `silence_timeout`；恢复各约 15 秒。完整 App 会话有 49 次 `ownership_timeout` | 有实际失联窗口，不是仅 UI 高亮问题 |
| 失联期间硬件 | 首次断窗前后 Agent IOCTL 计数 16→71，edges_sent 11→40，send_dropped 165→222，lease_expired 0→14 | 设备仍有报告，出站存在丢弃；57 条各类出站丢弃不能等同 57 次按键 |
| 语音 | generation 1/2/4/5/8 均 `chord_press result=ok` 后 2/3/7/2/2 ms 出现 `audio_pipeline failure_origin=message_channel error_code=queue_overflow`，随即释放快捷键并 abort | 已确认失败机制是音频通道溢出中止；SendInput 成功不证明第三方录音成功 |
| 后续正常语音 | 13:23:40.872→13:23:41.311、13:23:52.856→13:23:53.265 为按下到快捷键发送完成，约 439/409 ms；后续观察到 WeType marker 与麦克风活动 | 仅证明后续这两轮恢复；尚无同配置、同条件旧包延迟对照，不能判定性能回归幅度 |

语音五次失败中的输入设备准备耗时分别 472/488/504/403/425 ms，按下到快捷键完成为 699/888/829/680/677 ms。BLE worker 在输入准备、IME 和快捷键完成前暂存音频通知，完成后突发解码投递；32 条消息的 `try_send` 队列先于既有 32000 样本队列溢出。首次 WASAPI 启动日志均出现在溢出之后，消费者具体耗时需进一步分段取证。

旧安装版 PID 14824（2026-10-02 15:44Z 至 2026-10-03 07:50Z，同为 0.5.0、virtual_cable、tool=unset）有 98 次确认输入角色并正常 control_stop 的会话、0 次溢出。其输入准备 p50/p90/p95 为 65/124/155 ms，`audio_begin` 到快捷键完成为 178/258/278 ms，最大 436 ms；IME p50/p95 为 4/15 ms。四次闲置 2372/33979/4176/5307 秒后的启动仍为 169/182/175/168 ms。新成功样本也明显慢于旧分布，但计时入口与前台/系统负载并非完全受控，不直接等同迁移因果。更早 0.2.6 的 PID 54060 在 2026-09-30 09:46:07.123Z、09:46:09.391Z 已出现相同消息通道溢出，证明这一队列边界缺陷不是本次新引入。

已排除当前记录支持的两个直接猜测：本轮快捷键录入始终 `capture_active=0`；Helper 合成为 off，`synth_hits=0/gate_delays=0`，上游报告语音 gate 没有参与当前语音启动。五次溢出均是 IME `already_active`，不等于自身前台跳过 IME 激活导致失败。

## 根因与修复边界

- 菜单：模板跟随故意忽略自身前台，但菜单返回焦点复用了旧外部模板目标，未单独保存真实打开前窗口；恢复失败调用 `open_panel` 又生成新代次并重新显示。`scene_control.rs` 已单独保存返回窗口，在锁外查询并复查代次；返回自身窗口由 Host UI 线程直接执行，避免等待工作线程时阻塞自身窗口激活；外部窗口仍检查代次与请求有效期。失败保留原菜单和选择，不重开、不增加代次，取消后迟到结果不得复活菜单。`src-tauri/src/lib.rs` 同步取消失败后的重复显示。
- 音频：已确认准备积压突发投递撞到消息条数上限。`audio.rs` 已将 PCM 放在共享样本队列，待写与正在写入合计仍受既有 32000 样本预算限制；通道只承载可合并的唤醒和不可丢弃的生命周期消息。满通道下的唤醒可合并，待处理 PCM/错误仍驱动工作线程轮询，避免早退控制消息造成永久等待。结束、取消、失败先关闭入队，WASAPI 调用不持队列锁；不增加音频容量、不提前播放、不改 Ctrl+Win 时序。`ble.rs` 将迟到代次计入会话 `stale_audio_samples`，不打逐包日志或中止新会话。
- 报告延迟：Helper 当前基础优先级 6、App 8，计划任务 Priority 7，仅是候选影响因素。没有当时整机调度/资源证据，不能认定“电脑卡顿”或单凭优先级解释全部迟到。
- 物理观察重连：静态检查发现 R 集合仅变化时发送，Helper 未缓存重放，持续按住时重连可能丢失观察；尚未证明为本次主因。
- 升级清理边界：旧 Helper 的 `CLEANUP_FAILED` 在断线时设置后永久保持，即使之后连接收到真实 `stopped` 也不复位，最终仍生成 unconfirmed。现场两次 `[AGENT-STOP] result=unconfirmed` 与源码共同证明当前进程存在此状态；尚未为验证而关闭正在使用的旧实例。源码已改为绑定宿主 PID/OS 创建时刻与跨重连稳定的 Agent `instance`，只有同一实例回显本次 `stop_id` 且确认撤钩/释放的 `stopped` 才清除未确认状态；不同身份使本轮保持阻断。协议 build 为 `2026-10-03.cleanup-instance`，空启动取消不能覆盖旧 requested/unconfirmed 或损坏回执。`cargo test --manifest-path hardware/RC003/helper/Cargo.toml --locked` 33/33、Helper `--selftest`、独立 `cargo fmt --check` 及 Agent 逻辑测试 78/78 为 passed，覆盖失败后同实例 socket 重连、旧 ACK、持键停止重试与损坏回执；可选 Frida 传输自检仅语法通过，实际执行与新包真机验证 deferred。旧活体既无稳定实例标识也不回显停止请求编号，不能用新协议倒推其清理证明；相同 build/token、模块数量或 uptime 连续不足以替代该证明，现场和原回执保持不变，安装门禁仍阻断。

新增 Helper 慢路径诊断只记录 Agent 到达年龄、处理一行、stdout、落盘、桥队列、桥写入的慢项次数与最大耗时；250 ms 为观测筛选，最多 10 秒一条聚合。时钟倒退/不连续单独计数，不当作传输延迟。保持原优先级、租约、超时和行为；现有日志不足以证明同步 stdout/落盘就是本次阻塞点。空闲时一次 5 秒只读线程采样未定位阻塞栈，也不否定历史迟到。

收尾新增现场：14:21:43.101Z 旧 Helper 37764 记录 `[FOLLOW-APP] event=exit idle_ms=21118 grace_ms=20000`；14:21:43.217Z App 51996 已再次认证该 Helper，14:21:45.560Z Helper 也记录桥接恢复，但主循环已决定退出，14:21:45.788Z 留下 unconfirmed 回执。主程序进程仍存活并继续记录心跳。旧逻辑仅凭桥接最后连通时间推断“主程序已退出或开关关闭”，证明存在通信迟到触发的误退；不能因为随后没有 Helper 进程就认定钩子已清理。真实 stop intent 文件不存在，该窗口也无 STOP-SIGNAL。14:15:50–52Z 的 Helper 自检把停止标记写在独立临时目录，未操作真实进程、stop intent 或回执；自检会追加共享诊断日志，不能声称完全不影响日志载体，也不能据此推断共享 IO 与后续迟到有无因果关系。此窗口冻结在本机 `regression-helper-exit.log`，随问题闭合按同一留存规则处理。

`--follow-app` 的误退修复复用已有桥接线程：核对描述文件 PID 与实际命名管道服务端 PID，持有对应进程句柄及创建时刻；明确停止请求或该进程对象实际终止才进入退出/真实撤钩流程。主程序存活时的桥接中断沿用既有暂停与重连，不再凭最后连通时刻宣布主程序退出；原 20 秒值作为失联诊断及首次绑定失败的有界收尾边界保留。查询失败记录 Unknown，不伪称进程已退出；首次绑定失败也不覆盖未确认清理。只有 follow-app 模式固定首个 App 身份，手动运行模式仍可连接重启后的 App。未新增线程、配置、旧协议兼容或调大超时。

新增 `capture_input_windows.rs` 的每代输入准备汇总与 `audio_startup` 的首写、启动、取消静音分段耗时，用于区分下一次冷态的具体阻塞点。当前音频修复针对意外中止，不宣称已恢复旧版启动速度。

## 2026-10-04 再次报告与状态界面

用户再次报告返回/音量三个按键不可用，截图同时显示“未开启”、启用清理失败和 BLE 已连接，并要求整体整理布局与状态提示。当前仍是原 App 51996（启动于 10 月 3 日 08:03:49Z）与旧安装文件，没有运行中的 Helper；上述修复包尚未部署。本日四次启用分别在 08:25:00.982、08:25:09.475、08:25:54.420、08:27:08.670Z 开始，23–44 ms 即被旧 unconfirmed 回执拒绝，并未走到新 Helper 启动。另一次关闭也失败；停止意图已保存。WUDFHost 11412 创建于 10 月 2 日 13:05:45.8332121Z，始末仍为同一存活实例，不能用自然退出或开机更换解释旧钩子已消失。

UI 的确定缺陷是仅以持久化 `enabled=false` 显示“未开启”，忽略尚未完成的关闭；刷新后操作错误也会丢失。后端状态新增结构化 `cleanupPending` 与 `canRetryCleanup`，复用 `lastError` 传达原因：正常运行中尚无终态回执不算清理失败；助手仍在且可继续关闭时提供重试；助手已退、未确认或读取未知时阻止开启，不给无法实际执行的重试关闭。状态日志仅在分类变化时写入，包含阻断与重试原因。前端同时整理标题/状态/操作、折叠次要说明，单独表达蓝牙连接与按键支持，避免重复成功提示。

一次性停止恢复已完成：只读审计显示同宿主 47 个模块中唯一非 Windows 模块为固定 Gadget，runtime 无 generation 子项，使用固定单脚本和默认 `on_change=ignore`。首次 Helper 56676 的加载日志与当前旧脚本/Gadget 哈希一致，唯一一次实际注入后只有 attach，全部 18 次 HELLO 为同宿主/旧 build。独立审阅据此确认可以检验同实例正常停止，缺少 nonce 本身不作为无限期阻断的唯一理由；这补充了前一日按新协议无法反向证明的限制，不引入生产旧协议兼容。

10 月 4 日一次性 controller 46716 持有原 Host 进程对象，复核创建时刻、模块/脚本/配置指纹与实际 TCP 两端 owner，认证新连接后只发送一次 `disarm`。观察到本连接 `cmd:disarm → stopped(hook_detached=true,released_all=true)`，及随后 `disarmed=true/lease_ok=false` 心跳；全过程没有 Helper，原失败回执哈希不变。独立证据 SHA256 为 `3222e36354e6a73f1c422f7dbc1b1c9bdeb7ec0b7947bdc82c9509e4d031b561`。经再次审阅，以真实 controller PID 46716、真实开始/结束时刻 1791104175551/1791104176331 发布新的四字段 passed 回执，原 37764 失败回执原文和指纹另存结算证据；没有冒用旧 Helper 身份或挪动时间。发布前核实没有新 Helper/控制连接和代次变化；残留 Agent 向固定端口的 `SYN_SENT` 只是未建立连接，不是重新武装。随后原安装版的新进程 49276 通过 `Local\\SayAll-GracefulExit` 正常退出，未强杀。

停止并不等于 Gadget 卸载。退出后原 Host 11412 仍在；只读 CM 查询确认其承载 7 个 present/started、problem=0 的 UMDF HID 服务节点，RC003 只占 1 个。精确 PnP 重启该单节点也不能保证共享宿主终止，未执行该操作，更未强杀、FreeLibrary、并行注入 Gadget 或做电源/无线电试验。新 Helper 拒绝旧 build 的后续启动边界仍须等待原宿主自然生命周期结束；真实按键恢复不能据本次停止或安装宣布通过。

状态界面回归覆盖正常启动尚无 Helper/receipt 的过渡、旧清理债务、状态读取失败、操作后旧轮询迟到、关闭超时后的真实成功以及开启成功后的读取失败。实际就绪仍要求桥接 connected 且 owned 覆盖 target；BLE 连接单独显示。深浅色、940/1100 宽度的真实浏览器页面布局与操作检查通过，截图为模拟状态的视觉验证，不是硬件成功证据。

首个 UI 候选 `f03b01a6` 正常安装后，09:13:01Z 的真实开启进一步暴露收尾分类缺陷：Helper 64104 已认证到原 Host 的旧 `unified-input`，因 build 不匹配拒绝；汇总为 `renews_sent=0/edges=0/authenticated_ever=false/how=attached_existing_tap`，却因过早写 requested 和拒绝后的统一收尾再次写成 unconfirmed。一次性 controller 61652 再次从同一真实连接取得 `cmd:disarm → stopped(true,true) → disarmed`，独立证据 SHA256 `cb7540945bd3a6ae20c1177e4023b8e7267e52b02c7c16255573f6a763961a99`；按真实本轮 PID/开始 1791105652255/完成 1791105652871 发布新清理回执，保留原 64104 回执。App 41312 随后正常退出：捕获清理 22 ms、BLE 断开 89 ms、整体 122 ms、失败阶段 0。

最终 Helper 将 Attach 的 requested 延后到完整当前协议身份通过、第一次 arm/允许 renew 之前；新注入仍先写 requested 再注入。只有前一轮确有有效终态、当前为 Attach、明确旧 build 与正确 agent/PID、整轮未注入/授权/取得捕获时，才写四字段 `agent_version_blocked`，退出 16；这表示启动未取得资源，不伪称撤钩 passed。缺失/空 build、错误身份、既有捕获、pending 或未知状态都不能进入该分支。Host 与安装器按同一 PID/代次/完成时刻核对这一明确终态，允许正常结束，同时 UI 显示旧组件仍被 Windows 占用的具体原因；不再让下次正常启动永远被无资源失败阻断。生产路径不发送旧协议控制命令、不兼容旧 Agent，也不增加 sidecar 或假释放回执。顺带移除 Helper 三处 TOKEN 来源日志的值字段，来源仍可诊断，真实凭据不进入新日志。

`f03b01a6` 上另观察到两轮用户语音：均正常 `control_stop/normal_finish`，队列最终为 0，无溢出或迟到样本；首提交 182/180 ms、最大工作队列延迟 17/175 ms。仅为这两轮现场样本，不能替代严格冷态/闲置后对照，也不把 SendInput 日志当第三方实际录音成功。

## 2026-10-04 退出、异常终止与覆盖升级

用户要求开关与再次启动自动恢复残留状态，退出不再被模态提示阻断，安装改为覆盖并清理退休文件。本次只读现场仍为同一次 Windows 启动，当前回执为有效 `agent_version_blocked`；截图中的未确认故障不能解释为已经重启后仍失败。

日志进一步确认 08:46:38–08:49:48Z 的 5 次增强释放失败均在 `capture_release_unconfirmed` 处提前返回，没有进入 `raw_input_stop` 或 `ble_disconnect`。09:02:56、09:22:55、09:33:12Z 才有完整正常收尾。09:30/09:33 两轮自动启动各重复提交 4 次任务，即使已经确认旧 Agent 版本不匹配仍继续重试。源码另确认：旧回执只保存 Helper 身份，Helper 又在监听恢复通道前拒绝所有未确认回执，形成异常终止后无恢复入口的闭环；仅等待租约失效不能证明已经撤钩。

本轮改为持久化原宿主 PID、OS 创建时刻与 Agent 实例；清理专用 Helper 在独占通道内重新核实旧会话，只有新的停止回执，或旧进程对象确已结束的证明，才能结清。启动先恢复，手动开启只授权固定 Helper，再恢复并启用；关闭/退出不发起新的捕获。退出立即取消迟到启动，重复退出复用同一意图；增强恢复失败仍继续本地键态、Raw Input、BLE 和音频收尾，保留未确认记录供后续恢复，不伪称释放通过。菜单仍等待其真实原生 UP，不用隐藏窗口代替边沿配对。版本明确拒绝后停止自动重复启动。

覆盖升级统一走当前安装器，保留配置与安装位置；不先运行旧卸载器，不强杀进程，不清理未知文件或系统 Driver Store。独立卸载保留。安装器每个 Helper 清理阶段有界等待 45 秒；Helper 的连接与 HELLO 共用既有 30 秒绝对截止时间，停止阶段仍等待真实 UP，45 秒不能视为“必定完成”的承诺。未确认时不替换文件，不终止仍承担释放责任的 Helper。

`passed`：退出失败路径回归先观察到只执行 quiesce 的旧行为，再确认失败后仍依序停止 Raw Input、断开 BLE 和恢复音频，保持失败终态而可正常退出。排队 Helper 的出生时间落在原提交与恢复提交之间的竞争回归先红后绿；重复/并发退出、UAC 排队取消和旧 ACK 不能关闭新代次均覆盖。最终工作区测试为 core 31+4、Windows 329（19 ignored）与 3 项独立集成、Host 92（1 个由父测试执行的子入口 ignored）；fmt/check 通过。首次工作区运行在既有 bridge kill 测试中收到 UP 后立刻读取计数发生竞争，源码明确计数在边沿发送后更新；仅将测试改为分别有界等待最终快照，没有改生产时序。修后完整工作区通过。

`passed`：前端 243 项与生产构建；实际 Windows WebView/IPC 仿真 19 步。Helper 47 项与 Agent 78 项通过，Helper 的 3 个 ignored 仅为父用例执行的自有子进程入口。Windows 实际自有子进程 kill → 新恢复 Helper → 双向 TCP 所有者核验 → fresh stop_id/ACK → 原子终态已观察；另覆盖宿主退出/创建时刻不符、错误实例零控制命令、未知旧身份不改回执及清理模式不捕获。Agent 生产源码与租约时间未变。安装器 63 项隔离协调检查、9 项 SelfTest、2 项模板检查、真实 NSIS 编译，以及 5 个自建退休文件实际进入回收站通过；不外推为真实产品安装。

`passed`（当前恢复实现的真实 Windows 部署）：10:20Z 本地包 SHA256 `ee5db8eb696634e0b64e02f7f831125d1a293c1cb33acb24f6507e235cbd8c2a` 正常覆盖安装，主程序由 Explorer 普通权限启动；设置、映射及输入锁定三份配置字节保持，无重新授权标记。Helper 源码 SHA256 `81e2bf75f0273f9f72b9c550eed60ae585d844873201bc77deff2ac726093114`，构建/暂存/安装载荷一致，生产隔离检查通过。App 52824 在 10:20:54.279Z 开始 startup_reset，54.300Z 成功；仅一次任务提交，Helper 25720 明确版本拒绝，`capture_acquired=false`，未捕获或续约，App 停止重试。安装器在 10:22:39.338Z 请求正常退出，增强清理 20 ms、Raw Input 10 ms、BLE 154 ms、overall 186 ms/failed_stages=0，未出现阻断退出弹窗。

覆盖追加验证保留一次 `failed`：10:22Z 程序已正常退出，但同包静默 `/S` 安装返回 1603，未到完成新卸载器写入；现有证据不能确定具体失败文件或原因。10:24Z 同包重新以运行中直接 `/S` 安装，未先执行卸载器、测试器未发退出信号，安装器请求 App 45228 自行退出（126 ms、failed_stages=0），随后安装退出码 0、用户配置保持，Explorer 重开为 App 45560，普通权限与实际载荷核验通过。第二轮测试器不再持有旧 Process 对象，但两种独立自有进程/真实 NSIS 文件覆盖实验均证明持有已退出进程句柄时可写入，因此不能将首次失败归因为句柄；重试通过也不等于该写入异常已经定位。App 45560 在 10:24:56.875–56.896Z 再次 reset 通过；10:26:29.224Z 实际状态读回 `cleanup_pending=false can_retry_cleanup=false reason=agent_version_blocked`，驱动页截图显示明确旧模块占用原因和可用关闭入口，蓝牙连接单独正常显示。

最终交付仅追加安装阶段诊断，复用上述相同 App/Helper 字节，通过 `pnpm tauri bundle --bundles nsis --config target/sayall-input-candidate.json --ci` 重封装为 SHA256 `7f1ddde8c087c0b365430d45d6fd79db946c3adb57ca3d7c927d841b52d9d7c0`（12,998,849 字节）。10:38Z 在 App 45560 仍运行时直接 `/S` 覆盖，安装器自身请求正常退出，返回 0，三份配置字节保持，无重新授权标记；Explorer 普通权限重开为 App 69216，可见窗口和实际载荷匹配。原 App 45560 在 10:38:46Z 完整退出（188 ms、failed_stages=0）；新实例 startup_reset 通过，仅一次版本阻碍，10:38:57.291Z 实际读回 cleanup_pending=false、can_retry_cleanup=false，无重试循环。新的 `installer-result.log` 实际读回 `event=start component=installer` → `event=completed component=installer`。组件日志已用真实 NSIS 自建夹具验证锁定目标时失败并点名 `app`，解除锁后写入成功并覆盖旧结果；日志目录不可写时保持原错误码、错误标志与寄存器。模板契约 3 项和生产模板编译通过，独立审查无确定问题；首个 1603 的具体原因仍未知。日志只写固定阶段/组件，不含个人路径，规范见 [LOGGING](../LOGGING.md)。预检与 CI 已接入隔离清理、模板和路径检查，安装阶段追加真实夹具回收、NSIS 编译及文件锁诊断验证；本轮未触发新 CI，不将入口存在标为 CI 已通过。

仍需区别真正未知的边界：若 Helper 在已记录准备注入但完整 HELLO 尚未发生时被终止，宿主仍活且没有可验证 Agent 响应，就不能自动证明没有残余钩子；记录保持未确认，应用仍完成本地主链后退出。旧协议驻留模块不热换、不伪造新协议清理证明。实体 RC001/RC003 持键退出、异常终止后按键复用、冷态首用仍单列 `deferred`。

## 2026-10-04 晚：普通双击安装的权限失败

用户在 21:23/21:24（本地时间）重试增强仍得到旧宿主版本不匹配；这两次 `cleanup_pending=false`，21:24:47 的应用退出各阶段通过。随后 21:25:14–21:25:35 的交互安装记录 `event=failed component=app`，截图明确为无法打开主 EXE 写入。两者是独立故障，不用旧模块占用解释安装写入失败。

只读检查与零字节写访问实验确认：`7f1ddde8` 安装包实际 PE manifest 为 `asInvoker`；既有安装目录及主 EXE 继承管理员/System 可写、Users 只读 ACL。无 App/Helper 进程时，同一实际用户普通 token 打开既有 EXE 的写访问仍得到 `UnauthorizedAccessException` / `0x80070005`，不是只有运行中映像文件锁才能触发。原安装 EXE 仍为 20,024,320 字节、SHA256 `c76a94dfe4420069a2817f185440136b6a829d8c9f143e910c8dc735b8f7818b`，没有因失败被替换。此前部署脚本显式 RunAs 掩盖了普通双击路径的权限缺陷；不能把管理员部署通过当作普通安装入口通过。18:22 那次管理员流程 1603 仍缺精确系统错误，保持原 unknown，不追溯改写为此原因。

用户最终明确安装流程：检测已有安装后选择“安装前卸载”或“请勿卸载”；前者安全退出、卸载后自动继续安装，后者覆盖，两者保留用户配置。当前用户安装使用 `highest`，主程序仍由既有 `RunAsUser` 普通启动。目录权限检查早于退出/卸载，只有属性查询明确为文件/路径不存在时才退查父目录；其它错误停止。仅在记忆路径下实际有主 EXE 时显示已有安装选择，避免独立卸载保留路径注册项造成误判。安装前卸载使用当前包生成的临时卸载器，将六个明确产品文件及五个已知退休文件回收，不删除设置、授权、未知文件或系统驱动。真正标准账户与跨账户管理员凭据不视为同一场景，不通过管理员账户的 HKCU 覆盖用户配置。

最终本地 0.5.0 包 SHA256 `56fec037a49f0803fc226546bc31f0643ef353150e7d8f48bffaf1f39f854cc6`，12,998,453 字节，由同一已验证 App/Helper 重新封装；未改输入实现。原始 App SHA256 `357ff613cd66a077afd4a8921caf50233a7a9f6b054338a481a14e7f426da938`，安装 App SHA256 仍为 `c76a94dfe4420069a2817f185440136b6a829d8c9f143e910c8dc735b8f7818b`；差异为既有 NSIS 标记。模板 SHA256 `9a09e0796ca2170b4fdc64e1a9633975477c95edd640e78f4e203ace6a178e59`，hooks `7e97cb9f4c003ee085deee98c2666a7036b74be1023fd79710248247959be067`，recycler `db256771d9e8717496f9ca31e298ac62b8430da7e98821dc5dca4fd42df46370`。

`passed`：4 项模板契约先红后绿；真实 NSIS 模板编译、63 项清理协调器、锁文件/实际写入/日志错误标志及寄存器保持检查通过。最终实际宏 fixture 六场景（覆盖、卸载续装、非目录、属性查询异常、子卸载失败、新目录）通过；13 项回收检查实际观察到自建的五个退休文件和六个产品文件进入回收站，设置与未知文件保持。沙箱账户最初回收 E_ABORT 后没有永久删除，真实桌面用户执行通过。新增非法路径测试曾被 NSIS 自动过滤 `$INSTDIR` 字符导致误测，改为普通变量传入生产宏后实际错误 123 被拒绝；不是放宽生产检查。

`passed`（真实交互安装，两次均从 Explorer COM 的 `open` 普通双击入口启动，无 `/S` 或对安装包显式 `RunAs`）：

- 14:28:59Z 选择“安装前卸载”。App 30152 于 14:31:18.863Z 收到安装器正常退出请求，完整收尾 172 ms，failed_stages=0。安装结果依序为 `directory_access passed`、`reinstall_uninstall start`、`product_cleanup completed`、`reinstall_uninstall completed`、`installer completed`。自动继续安装并由完成页打开 App 61736；三份配置字节保持，所有产品载荷匹配。
- 14:33:21Z 选择“请勿卸载”。App 61736 于 14:33:28.601Z 正常退出，完整收尾 181 ms，failed_stages=0；日志为 `directory_access passed`、`overlay selected`、`installer completed`，没有卸载阶段。完成页打开 App 30112，三份配置字节保持，所有产品载荷匹配。
- 两次安装器均实际读回 TokenElevation=1 且与 Explorer 同用户；完成页应用均为同用户、TokenElevation=0，安装器退出码 0。随后按原验证入口正常关闭并通过真实 Explorer 再打开最终 App 60640，14:35:17.879Z 创建，父进程 Explorer 65116，二者 TokenElevation 均 0、主窗口可见、构建/暂存/安装载荷一致。完成页启动的父进程可能已退出，不能用“找不到存活父进程”否定其实际普通令牌。

验收脚本曾因 UIA 不暴露 NSIS 控件在欢迎页等待超时，actions=0，未执行安装；该向导正常取消。改用 Win32 精确进程/控件核验后继续；首轮非激活窗口的 `BM_CLICK` 对安装/完成按钮无效，向已核对按钮的父窗补发标准 `WM_COMMAND/BN_CLICKED` 后完成；最终覆盖轮使用修正脚本一次通过，21,837 ms。没有强杀应用、安装器或宿主，也没有手工发送退出事件来替代这两轮安装器自己的退出请求。

`deferred`：真正标准账户、需交互确认的 UAC 取消和其它 Windows 版本未实测。安装修复未解除旧设备宿主内的 Agent 版本阻碍：14:33:39Z 再次实际读回 `cleanup_pending=false can_retry_cleanup=false reason=agent_version_blocked`，并停止重试。实体 RC001/RC003、冷态语音和持键异常退出仍须待 Windows 正常重启清除旧模块后验证，本轮未重启电脑或强杀 WUDFHost。

## 2026-10-04 深夜：菜单与增强按键再次不可用

用户反馈菜单不能打开模板选择、原三个增强键同时不可用。当前实例 69376 于 14:48:21.808Z 启动；14:48:24.991Z 实际状态为 `cleanup_pending=false can_retry_cleanup=false reason=agent_version_blocked`，14:48:27.747Z 启动监督以 `retryable=false` 结束。当前无 Helper 存活；共享设备宿主 11412 的 OS 创建时刻仍为 2026-10-02T13:05:45.833212Z，Windows 启动时刻为同日 13:05:33.5Z。没有发生能卸载旧模块的宿主代次变更。此前已核实的旧 `2026-10-03.unified-input` Agent 与当前 `2026-10-03.cleanup-instance` 契约不匹配仍成立，不是新一轮“按键尚未释放”债务。

本次只读聚合的 554 条应用事件中有 44 个实际 Menu 观察边沿，按下/松开均到达；无模板菜单打开。统一输入路径在增强未取得捕获所有权时暂停配置动作，这同时解释菜单及三个增强键不可用。公开前台程序观察仍有 5 次变化（14:48:23.002Z、14:48:46.645Z、14:48:50.884Z、14:49:00.809Z、14:49:05.550Z）；不能仅凭最初一次 `foreground_generation=0 actions_available=false` 认定整段运行没有模板目标。主窗冷启动、尚未观察过外部程序时无法选择程序模板是另一个待确认边界，本轮不凭推测新增全局模板或修改窗口显示次序。

代码复核另见 `rc003_auto_trigger_reconcile` 在版本拒绝或首次桥接 Connected 后结束的边界；曾针对它尝试持续监督，并开发受控脚本重载，但均未打包安装。复审发现重载共用脚本可能涉及另一宿主、新实例等待仍借用旧期限、错误脚本实验可能读到旧心跳等未闭合项；用户要求停止其余改动并先正常重启。因此这些实验不能解释本次恢复，也不作为已交付能力保留。没有强杀、无线电开关或设备重启实验。

### 正常重启后的根因闭合与撤回决定

`passed`（2026-10-04 23:24 后，本次现场范围）：Windows 的实际启动时刻变为 15:24:22.500Z，设备宿主均为本轮新进程。App 8356 于 15:27:57.377Z 启动，Helper 26852 于 15:28:02.326Z 启动；实际 Helper 日志读到 `2026-10-03.cleanup-instance`，已不是旧 `unified-input`。应用 15:28:04.318Z 显示 `session_running`、`cleanup_pending=false`，15:28:07.173Z 为 `helper_connected`。截至本次只读采样，版本拒绝 0 次、Menu 观察边沿 8 个；15:28:28.732Z 与 15:28:30.148Z 两次菜单焦点恢复实际通过，分别 13 ms、17 ms。用户明确确认“菜单不能切换模板、原三个增强键不可用”两项现象都消失。

重启前后的安装 App SHA256 均为 `c76a94dfe4420069a2817f185440136b6a829d8c9f143e910c8dc735b8f7818b`，Helper 均为 `65bbaa4311b8b4e8a36c47cb609ae1935439f1f4e6850672538c68a8d288890d`，与 `56fec037` 安装包载荷身份一致；没有安装上轮实验。因而本次持续不可用的直接原因闭合为旧 Agent 驻留、版本不匹配导致统一捕获被拒绝，正常重启加载当前 Agent 后恢复，不需要改写模板映射或恢复旧五键方案。

用户据此要求重新评估刚才改动。本轮选择撤回未部署的持续监督、热重载、对应测试及自动恢复文案，按现有交付 manifest 的源码 SHA256 精确恢复，保留此前已验证的启动/停止 reset、成对释放、正常退出和安装器修复。未来若再次升级驻留 Agent 契约仍可能需要正常重启；当前版本的日常使用不应据此要求反复重启，也不能承诺未来任何升级都不会遇到驻留版本边界。

此次用户反馈和日志仅关闭上述持续不可用现象，不扩大为双型号完整验收。重启后另见少量捕获所有权超时和一次桥接失联后恢复；属于既有链路时序观察，不能混为已经消除的版本拒绝，也不在用户已收束的本轮继续扩展调查。持键退出/kill、冷态语音及完整 RC001/RC003 矩阵仍按原记录保留未验边界。

撤回已执行：Helper 主文件、Agent、Agent 测试、Host 主文件、任务控制和组件文案测试共 6 个既有文件，全部命中部署 manifest 的原始 SHA256；完整清单 31 个文件也均一致。新增重载脚本及临时候选、夹具等 25 个文件（24,564,136 字节）经既有 IFileOperation 回收入口确认进入回收站，未永久删除。实验及撤回都没有产生或安装新包，当前已安装程序继续运行。

本次撤回验证采用精确源码身份比对，复用上述同一交付身份已有的软件验证。追加 `cargo fmt --all -- --check` 在工具启动阶段被环境拒绝访问，`pnpm build` 在 pnpm 自身版本获取阶段网络失败，均未进入产品编译；未通过提权重放失败工具、安装依赖或修改环境。相应小日志保留在本机同主题 `revert-fmt-check.log`、`revert-frontend-build.log`，不记为新的构建通过，也不影响已核对的源码及正在运行的安装载荷。此次未重跑完整测试或重装；新的根因与恢复证据为上文真实重启、同载荷运行日志及用户确认。

## 2026-10-06 朋友的 RC003 返回键报障（待现场日志）

- 用户确认型号为 RC003；现象为本地版返回键无法正常使用、官方版正常。尚无朋友的安装包身份、映射配置、全按键支持状态及现场日志，音量键表现未知，不能归因到单一环节或宣称修复。
- 核对本地 `50f21e005b4f1bf3455268eb013d58259196d6a2` 与第二父提交 `a3ff225b37fd15e0f60b6b07afb0e27426e6859e`，源码树一致。额外 worktree 的归档没有修改此源码树，不存在“合入 main 漏带返回键”的差异证据。
- 官方 `v0.5.0` 标签精确指向 `e5374a9d9232afaf1024d4545353a4463608d11e`；本地与该版本的返回 usage `0x00F1`、9 字节报告槽及白名单相同。两边新配置均默认不启用增强，不能将此默认值说成版本差异。本地 RC003 映射额外要求当前目标已获捕获所有权；蓝牙连接或 Helper 连接本身不是就绪证据。官方与本地 Agent build 不同，旧模块拒绝也是需要由现场日志确认的候选，不能恢复强杀设备宿主的做法。
- 本地三个内置模板的返回为 Backspace，浏览器导航需显式配置 Alt+Left；动作已发送和目标程序实际响应分开判断。本机现有日志（2026-10-06 07:35 UTC，PID 42296）可见 `Back=true → map_route PassThrough → map_fire Backspace → map_inject result=ok → Back=false`，只证明该实例有实际输入和注入提交，不代表朋友环境、物理按键验收或浏览器导航通过。
- 本轮仅补充桥接边沿与映射决策的脱敏日志，不改捕获授权、默认配置、协议、Helper/Agent、按键动作或等待时序。重点区分目标过滤、所有权未确认、识别配置为空、手势等待、取消后等待释放、场景消费、无动作和注入结果；测试使用真实映射 runtime 与记录型 injector 核对动作数量，而非只核对 UI。
- 首按候选边界：Helper 在 `targets_ack` 后等 `lease_ok` 心跳才发送所有权确认，而边沿可能先到。此顺序可能使启用/切换窗口的首按未执行，但不能解释稳定状态持续失败。本轮记录桥接消息序号、代次及所有权用于取证，不据推测修改该时序。

朋友取证步骤：

1. 安装本次本地诊断包，记录“驱动”页全按键支持状态与“按键”页当前使用模板，不重置或替换原配置。
2. 在原来失效的程序中短按返回 3 次（每次间隔约 1 秒），再试一次音量加、音量减；记录测试时间、是否有按键页响应和实际结果。第一次失败后先取日志，不先重连或反复安装。
3. 打开“权限 → 诊断摘要”，选择“生成摘要 / 复制摘要”和“打开日志目录”，提供 `sayall-diagnostic.log`（较大可压缩）、摘要和上述现象即可；无需提供个人配置、录音或第三方应用数据。日志默认持续记录，不需提前开启开关。

`passed`（自动化，2026-10-06）：新增映射诊断 2 项、桥接诊断 3 项均先红后绿；映射测试覆盖未授权、等待手势、场景处理/阻止、无动作、注入成功/失败 7 条路径，以及返回/菜单/音量±的重复边沿、取消释放和相同状态去重，并断言实际 injector 调用数。`cargo fmt --all -- --check`、`cargo test --workspace`、`cargo check --workspace` 与仿真 feature 编译检查通过；主要测试分组为 core 31 + 4、Windows 362 passed / 20 ignored、Host 99 passed / 1 ignored。最终快照开销修正后，桥接套件 25/25 再验通过。受限环境最初 6 个桥接夹具因临时描述文件不可见失败，正常宿主环境全套通过；这不是实体按键故障复现。既有 ignored 项不改记为通过。

`passed`（本机构建与部署，2026-10-06）：既有 `scripts/build-local-input-candidate.ps1` 构建 NSIS，前端构建及生产仿真隔离检查通过。0.5.0 诊断包为 13,036,910 字节，SHA256 `308b73469efdbb1acd777e04b105b8ef33b88d2a7af92f82c9961b5044de394c`；日志标识 `build=rc003-back-diagnostics-20261006 build_channel=local release_tag=none`。源码基线为上文 `50f21e0` 加当前两处诊断工作区差异，无自动提交、推送或发布。安装时观察原 App/Helper 正常退出，管理员覆盖后原配置哈希一致；安装 EXE 与构建 EXE 除预期 NSIS 标记外逐字一致，Helper/Gadget 与包内资源哈希一致。Explorer 启动 PID 3092，实际令牌查询为非提权；真实 UIAutomation 读到六个页面入口，并点击“生成摘要”后观察到报告显示。生产日志记录 Vue 挂载、首次 IPC、RC003 连接和当前菜单目标所有权确认。以上不是返回键或输入法的物理验收。

朋友现场 RC003、冷/闲置首用以及 RC001 仍为 `deferred`。本次只在原工作目录修改，Git 注册工作树仍为一个；原有 `artifacts/` 保留。临时验证复用 `target/dev/back-key-diagnostics/`：`verification.json` 记录源码指纹、包及部署身份，五份检查/构建日志保留至朋友问题归因闭合后按 LOGGING 回收可再生输出。当前安装包仅保存在既有 `target/release/bundle/nsis/`，不复制第二份；红绿失败判据已在本节保留，不另建源码快照或 worktree。

## 2026-10-10 用户报告：同次开机的三轮启动（部分定位，未闭合）

用户在准备下一任务时补充如下观察；报告时尚未核对三轮时间、PID、安装载荷与配置，不能据此认定与历史故障同根因：

1. 第一次打开无法正常使用；按音量加减，驱动页有异常显示，全按键支持无法正常开启，随后关闭。
2. 第二次打开时，实体键盘数字 1 左侧的键输出成 Tab；方向键/返回键不能按住连续移动或删除，但返回键与音量加减的普通使用正常。用户称该实体键为“单引号”，后续须核对实际键位/键码，不能直接按引号键建立测试。长按异常对应的当前模板、显式连发开关和焦点场景也未确认。
3. 第三次打开，用户观察目前所有功能正常；这仅描述当前正常边界，不证明前两轮问题已修复或已完成完整矩阵。

本轮在重启、构建和合并前冻结当前及轮转日志，并按系统本次启动时间 `2026-10-10T02:15:20.500Z` 提取时间线。现场安装 EXE SHA256 为 `707880d02f6dc40edfe7f4030e3fe73af5c93de7f72b1514d28fe50fc1312f7c`，与 10/9 候选一致；源码身份仍是 `9f4ead5` 加未提交工作，不因版本号或空 `sourceCommit` 推断精确内容。以下本地时间均为 UTC+8；三轮与用户观察按顺序关联，不声称用户每次操作的时刻已被直接记录。

| 轮次 | 主程序 / 按键宿主 | Helper / 设备宿主 | 配置与捕获 | 退出终态 |
| --- | --- | --- | --- | --- |
| 第一轮 10:19:32 | 27224 / 15164 | 后续 10:26:46 才见 Helper 5700；设备宿主 11972 | 启动 reset 提交 cleanup-only，8 秒后 timeout，recovery_attempted=true；后来 generation=1 的 owned_mask=0，另有 ownership_timeout；未见映射动作/连发 | 10:34:39 socket_closed；10:34:41 Helper 跟随主程序结束并清理 passed；缺少主程序完整 shutdown 终态，退出方式未知 |
| 第二轮 10:34:56 | 11672 / 1448 | 29220 / 11972 | 11:10:33 菜单手动应用模板，selection generation=101/102、foreground generation=113；捕获 target generation=4，requested/owned mask=7167；出现方向/返回等 map_fire，未见 map_repeat | 11:20:00 请求正常退出；按键宿主 shutdown_requested；Helper explicit_stop 清理 passed；整体 shutdown 938ms、failed_stages=0 |
| 第三轮 11:20:09 | 23708 / 32756 | 46672 / 11972 | common 无配置动作、自动跟随关闭；requested/owned/recognition mask=256（保留 Menu），execution mask=0；无 map_fire/map_repeat | 取证时仍在运行；12:10:16 部署前正常退出，overall passed、728ms；用户观察正常只覆盖当时操作 |

三个 Helper 均报告 `2026-10-03.cleanup-instance`；第二、三轮复用设备宿主 11972，不等于旧版本阻塞。第一轮可定位到启动清理未确认及后续捕获所有权未就绪，但现有日志没有当时 Helper 进程/回执匹配判据或任务启动失败码，无法确定为何提交后超时。当前任务状态和当前回执不能替代历史事实；TaskScheduler Operational 当时未启用，未为调查改其设置。

第二轮实际记录返回持有 2941ms、左方向 2227ms、右方向 1680ms，没有连续拍。稍后只读配置快照为 schemaVersion=1，common 和六个模板均没有显式 holdRepeat，按当前契约表示关闭；映射 tap 只执行一次与这个设置一致。第三轮没有同样的动作执行配置，可由原生路径表现出按住重复，不能用它证明第二轮已修复。历史模板 ID/完整配置及前台程序没有足够日志，也未收到用户确认，因此“第二轮确切模板与场景”仍未知；未开启连发、未改用户模板、未再次执行迁移。

实体键盘数字 1 左侧的实际键帽、布局与扫描码仍待用户确认，不能写成单引号键，也不能确认就是 VK_OEM_3。代码中 TV 候选 VK=0xC0/usage=0x0035 可映射 Tab，但这是遥控器映射事实，不能证明实体键盘被识别成 TV。LL 门对该类必须有报告来源的实体边沿放行，并排除注入边沿；Raw Input 又按所选设备过滤。历史日志不足以把该次物理键、报告来源、注入回声或残留键态串成因果链，暂不修改来源识别或吞键逻辑。

仅新增脱敏诊断，继续保持未提交：reset 在观测变化时记录 Helper 数量、预期 Helper PID、回执存在/有效/完成/可恢复/匹配、新旧启动和排队恢复判据；map_reconfig 记录引擎 revision、实际 execution/recognition 位图、enhanced 及显式单击/长按连发位图。无需记录设备路径、模板名称、窗口类哈希、用户组合键或字符内容。下一次异常先读取这些判据，再结合有界按键观察及用户前台场景复现；确认根因后先补失败回归再修复。本轮没有把上游输入法首按恢复当作上述异常的修复。

最小现场留在既有 `target/dev/foreground-input/`，长期结论归本节；构建/安装结果归 [Preview 验证记录](../Testing/WindowsRC003Preview.md)。实体键盘共存、异常轮次复现、明确模板下显式关闭/开启的长按对照、冷态首用与双型号硬件边界继续 deferred。

当前可对照基线为 10 月 9 日实际安装的 0.8.5 个人版，包与安装 EXE 指纹、配置保持及启动观察归 [安装记录](../Testing/WindowsRC003Preview.md#同日按用户要求覆盖安装并打开1128)；新任务仍须只读核对实际运行载荷，不能假定期间未变化。原修改的保护 stash `0d401298648e18b66685e9e0b116c97b9d52a8c9` 属于合并前备份，原修改已在 `9f4ead5` 合并后恢复并保持未提交，不再次盲目 apply。保护 manifest 留在本机 `target/dev/upstream-sync/merge-20261009.json`，不以 stash 代替当前工作区。

## 2026-10-10 17:56 Codex 返回键失效后菜单确认恢复（根因未闭合）

用户报告：同一次应用运行中此前返回键确实可以退格；17:56 左右在 Codex 中短按和按住均无法退格，立即按菜单键、直接按确定，随后退格恢复。此前成功操作的准确时间与输入路径未知；不能因启动配置为空而否定这项观察，也不能把菜单确认后的恢复当作已经找到根因。

现场身份为已安装 0.8.6（EXE SHA256 `307a5ebe5720a2b6154e254df91133d3830daf968420331a36ee50b8778147f3`），主程序 38828、按键宿主 50032；同一主程序自 13:47:09 运行至本轮取证。先冻结当前日志及相关时段轮转检查，再只读核对实际配置；没有重启、构建、安装、修改模板或开启连发。

| 本地时间（UTC+8） | 可确认事实 |
| --- | --- |
| 13:42:28 → 13:45:38 → 13:47:09 | 上一主程序 32448 菜单手选后正常退出；38828 重新启动时实际来源 common、自动跟随关闭、execution_mask=0。手选只在进程内保留，重开不恢复手选是现行契约，不是本轮新发现的运行中自动切换 |
| 13:47:09 至 17:56:39 | 仅见 common 来源，未见 program_default 或其他模板选择请求；各次映射重配 execution_mask 均为 0。保存配置的 common actions 数为 0，模板内容和自动跟随开关与本日早先快照一致 |
| 17:31:16–17:32:00 | 增强桥接静默超时，两次 Helper 重建并恢复；当前恢复者 39152 复用设备宿主 11972。恢复前后捕获仍只有 Menu（mask=256、target generation=2），没有接管 Back；不能据时间相邻推断此次恢复导致退格失效 |
| 17:56:35.889/36.041、38.575/38.726 | 两组 Back DOWN/UP 的观察边沿均存在，但未进入 map_fire/map_inject；当时 Back 不在有效执行映射与捕获所有权中 |
| 17:56:39.437–41.512 | Menu 打开，确认提交 source=menu、generation=285（随后配置确认 286）；目标捕获更新为 7167，执行位图变为 6911，实际映射确认 actions_available=true。菜单初始 selected=0，目录首项为内置 Agent，其保存覆盖的 Back Single 仅为 Backspace。默认关联保存不等于重新开启自动跟随 |
| 17:56:42.275、46.291 | Back Single 注入 queued，均观察到 echo_delta=2、VK=0x08、scan=0x0E；注入回声只证明按键链提交/被钩子观察，用户进一步确认 Codex 实际退格恢复 |

结论：当前日志没有支持“同次运行中模板异常自动切走”的证据；菜单确认确实建立了此前不存在的退格映射。现行手选不跨重启、关闭自动跟随忽略程序默认关联，可解释当前 common 状态，**不能独立解释用户明确观察到的同次运行先成功、后失败**。此前成功退格的原始键码、来源（原生/映射/其他）、焦点与修饰键状态没有对应对照记录，仍未知；不把未知成功路径、Helper 重连、模板启动规则或菜单窗口恢复失败串成根因。菜单本次 `restore_completed ... failed reason=window_operation_failed` 是独立焦点边界，不能解释此前的失败。

短按失败与切换后短按恢复为用户实际观察；长期按住是否连续删除仍未验证。当前 repeat_single_mask=0、repeat_long_mask=0，Agent 保存覆盖无 holdRepeat，不能自动开启连发作为修复。现有 `restarting_runtime_keeps_preferences_and_defaults_but_not_manual_selection` 在本日同源码的 `cargo test --workspace` 记录中通过，只证明启动契约；本轮只读分析不新增修复通过结论。后续对照须补齐已选设备的原生返回/Backspace 键码与路由、同代实际 Back 动作类别、成功/失败两次的焦点及修饰键分类；现有日志不能追溯补回这些历史事实。

必要现场仅本机留于既有 `target/dev/foreground-input/template-switch-current.log` 与 `template-switch-configuration.json`，前者保留 13:42:25 至本轮冻结终点的完整相关会话，后者用于核对用户映射并比较早先快照；含本机敏感配置与旧日志字段，不提交原件。本页仅记录脱敏时间、PID、位图、标准退格键码、包指纹及用户观察。保留至该次正常/异常路径对照闭合，随后按 LOGGING 回收，不新增重复台账。

### 2026-10-13 当前启动与菜单手选（旧启动契约与新记忆要求）

用户补充确认：当前重选使用模板后正常响应；按键被收到与实际存在映射须分开判断。本轮只读核对安装身份 `f7c679a6ed822b9a2cbcf1a335fdc3ab06140bef6034825410b81657e95e2b99` 的 App 53020 当前生命周期，不重启、不改配置、不模拟按键。北京时间17:07:07启动，17:07:08实际应用 `source=common / generation=2 / actions_available=false`；自动跟随关闭、common actions为0，捕获随后仅有Menu（mask=256），17:07:25的Left已收到但 `configured=false`。17:17:43首次且唯一手选请求为 `source=menu / common=false / scope=session / persisted=false`，generation19→20；17:17:44捕获确认后 `actions_available=true`。随后Back有142条手势/执行/提交事件，17:20:03–06多次Single为fired/queued；最后17:20:45仍是 `source=menu / generation=33 / actions_available=true`，未见重新应用common。用户确认手选Agent后的可用性是实际观察；queued本身只证明提交。

当前事实符合连续覆盖安装后的新启动不恢复上次进程内手选这一旧明确契约，不是0.8.6随机覆盖；它不证明以前17:56同运行先成功、后失败的原因。菜单请求日志不含template ID，不能只凭 `source=menu` 断言确切模板；当前保存的Agent覆盖确为单键Backspace，与用户说明一致。未直接调用活体SceneSnapshot IPC或采集UI像素，不将日志检查当作灰标签显示核验。编辑源选择只更新前端草稿；向导语音绑定只推送语音设置，映射暂挂只设置suspended；本轮未发现它们清除当前手选的新代码/日志竞争证据。17:17–18分钟另有菜单返回目标恢复失败/保留面板日志，是独立焦点边界，不据此解释当前映射丢失。

用户随后明确要求替换旧契约：关闭自动跟随时，手动使用经映射引擎确认后记住，重启恢复；明确选通用清除记忆。开启跟随清除全局记忆，按已有程序规则选择；再次关闭从通用开始，程序默认关联不改。候选只新增 `rememberedTemplateId` 设置，缺字段为None；删除/无效ID安全回通用，重命名/编辑保留同ID。菜单与UI共用引擎确认后的保存事件，保存只更新记忆字段，不重新套用整套映射；整包编辑保存合并最新记忆。`manualSaveStatus=saving/saved/failed` 区分实际应用与持久化结果；保存失败保留当前已应用模板，重启仍使用上次保存的选择，不谎报回滚或已记住。迟到旧保存核对本次手选代次，未确认新选择的previous可结清旧保存；引擎超时/拒绝不请求保存。捕获租约/驻留换代开发继续暂停。

定向验证：初始四项重启/首次配置/开关/ACK回归先红后绿，旧保存与未确认新请求交错另先红后绿；场景套件79 passed、0 failed、1个既有真前台探针ignored。真实Host的四项记忆事务测试passed，覆盖旧编辑合并/重开/通用、迟到与无效选择、重命名/删除/开关以及自有文件共享锁造成的真实写入失败后旧字节保持。独立有界review发现启动记忆来源被旧分支误标program_default，补断言先红后修正为remembered，最终记忆定向9/9 passed。冻结源码完整矩阵passed：前端407项/28文件、类型检查/构建；Rust workspace728 passed/23 ignored（Windows531/22、Host158/1、core32、scene4、集成3），另有独立诊断子进程1项；fmt、普通/仿真check；Helper58/3 ignored及父测试实际执行的子入口、Agent95、身份夹具及前端嵌入内容门禁12项。真实隔离WebView/Tauri模拟18步passed且正常清理，但模拟接口不支持select_current_template，不把它当作本次模板记忆验收。沙箱内tauri-build/rustfmt的os error5不计产品失败，实际检查由真实宿主执行。

本地候选运行身份 `d884cf3d7399e892c0bbc2ab9cb87879886ca744888d9baa6187e1017e7242fc` 安装后，用户明确确认模板列表鼠标“使用”和按键页当前使用下拉均正常、编辑目标不变，当前PC此次两入口交互passed。用户另反馈成功提示闪动；仅删除成功提示，不更改应用/保存链，重跑前端检查后正常覆盖安装后续静默候选：setup SHA-256 `1a2a01a08ce5effda6260b4d52cff5a625b2ead404bef9c7d166a5162161505c`，运行 source fingerprint `8a38c43cce592fff5636e0aa9aafa06abd38f0b6092b21e73e34cff1cc774007`，base `af710b757897cb0567b63c63a7ceb40f62426652`、dirty/local/included。六个相关Rust文件hash保持，因此复用上述同内容Rust/Helper/模拟证据；不凭新前端包自动扩大硬件结论。

真实退出/安装/重启分层证据：旧App23840正常退出overall passed、failed_stages=0、808ms；安装前记忆为browser，安装前后五份配置哈希保持。Explorer12848启动安装版59956，Session1、TokenElevation0、主窗可见，实际身份与静默候选匹配、Helper connected。北京时间18:25:49–52新进程首先读回 `source=remembered / generation=2 / actions_available=true`；18:25:55.096随后有 `source=buttons_page / generation=3` 请求、应用确认及记忆保存passed，稍后读回Agent。用户最终明确说明Agent是自己启动后手动选择，并确认再次切换不闪动。启动初始browser恢复依据是安装前后配置与新进程日志，不声称用户亲眼确认了该初始画面；之后Agent是已确认的人为使用，不是自动覆盖记忆。本次两入口、正常重启记忆恢复和去成功提示的限定验收passed；它不关闭旧17:56同进程失映射、其他设备/完整硬件矩阵或暂停capture边界。

最小现场为既有主题下 `target/dev/foreground-input/template-current-session-evidence.json`（6,933字节），保留35条脱敏生命周期/应用/捕获事件及Back首尾样本；当前与最近轮转原件原位保留，无整树复制。归因已在本节独立保存；本次入口验收闭合后按LOGGING回收，不提交原始配置或语音内容。红绿与场景日志复用同目录 `remembered-template-*`，不新增重复台账。

## 2026-10-10 晚方向键长按与实体键盘共存（来源调查中）

用户再次报告遥控器方向与返回按住不能连续执行，并补充实体键盘也异常：ESC 下方、数字 1 左侧的键变成 Tab（用户称 Table），方向键按住只移动一次，实体 Backspace 仍能连续退格。此补充扩大到键盘共存，不能只以遥控器模板连发关闭解释整组现象；尚未取得本轮只按实体键盘的准确时段及连接方式。

现场仍为同一安装 0.8.6、主程序 38828、按键宿主 50032、EXE SHA256 `307a5ebe5720a2b6154e254df91133d3830daf968420331a36ee50b8778147f3`，源码基线 `af710b7` 加既有未提交工作。Helper 2600 接管宿主 11972。先冻结当前及轮转日志，再只读核对配置、系统键盘设置与源码；未重启无线麦、构建安装、改模板、重做迁移或开启连发。

- **遥控器实际配置**：冻结的 `button-mappings.json` 与 17:56 调查快照字节一致，最后写入为 17:56:40.857；Agent 保存覆盖的方向/Back 只有单击，双击/长按 Disabled，没有 holdRepeat。17:56:41 的 revision=17 和 18:02:33 恢复后的 revision=19 均为 execution_mask=6911、recognition_mask=7167、repeat_single_mask=0、repeat_long_mask=0。19:38:00.471–01.836 的 Left DOWN/UP 相隔约 1.36 秒，只有首次 Single 注入，无续拍。冻结终点之前同主程序无 map_repeat。只能确认该配置不会连续执行，不能证明这些边沿都来自实体遥控器。
- **未见运行中关闭连发的证据**：17:56 确认后至最初冻结时段未见新模板选择请求，配置也未变化。随后现场 19:40 又出现菜单选择记录，属于初次冻结之后，不能倒推为 19:38 的原因；具体是谁操作仍以用户对照为准。此前原生/映射路径正常的时间和来源仍未知。
- **系统设置只读结果**：真实桌面用户 KeyboardSpeed=31、KeyboardDelay=1，FilterKeys Flags=126 的启用位为 false，系统 Scancode Map 不存在。源码检索未发现写入这些键盘配置的操作。这不排除钩子、报告拦截或其他软件改写输入，亦不把注册表读回扩大为所有系统状态正常。
- **确认的源码来源缺口**：18:02:31 Helper 已记录 selected host 的 members=7、rc003_members=1，七项均属 BTHLEDevice，仍以 `continue_with_usage_gate` 继续。`rc003_agent.js::installHook` 只检查 IOCTL、输入长度/内容、报告 ID 与 usage，没有在上报或清空前证明逐报告属于所选 RC003；FileHandle 仅被记录用于停止释放核对。主程序后续的所有权位图确认的是允许的按键，不能补回设备来源。共享宿主与标准方向/TV usage 的组合构成实际来源隔离缺口。
- **隔离代码实验**：复用现有 agent_logic_test.mjs 的可控桩，在内存中运行当前 agent 源码，不接触活体宿主。下发 TV=0x0035、Right=0x004F 后，同报告分别使用模拟句柄 10、11，四项均被清为 0；两项普通 Backspace=0x002A 保持原值。实验确认源码会按内容接管不同句柄且保留普通 Backspace；它与用户描述相符，但模拟句柄没有真实设备身份，不能据此宣称本轮实体键盘根因已闭合，更不能外推三轮启动或 17:56 单击失败同根因。
- **实际键位边界**：两分钟公开 API 只读探针成功注册独立 Raw Input 与始终放行的 LL 观察钩子，当前前台布局对 scan=0x29 返回 VK=0xC0、language=0x0804。观察窗口 19:49:17–19:51:17 未收到所查键的事件，随后正常撤钩结束；这只是布局查询，尚非用户实体按压的实际键码。暂不将该键定为单引号、反引号或遥控 TV。

- **20:17–20:18 实体键盘对照**：用户按只用键盘的观察指令操作，追加反馈为该键与左右方向键组合时能够打出反引号。公开只读观察实际收到非注入 `VK_C0 / scan=0x29`，不再仅依赖布局查询。20:17:45.233 的 Helper Left 与 45.277 的映射注入、20:17:47.486 的 Right 与 47.557 的映射注入对应；此窗口各有一组成对的注入 Left/Right。20:17:57.559 Helper TV=0x0035，57.735 主程序记录 TV Single 注入 Tab；观察器 57.726 收到注入 Tab。整个观察有 11 对注入 Tab、102 次非注入 C0 DOWN、33 次非注入 Left DOWN、69 次非注入 Right DOWN；原生重复主要出现在用户组合按压期间。源码缺口与这次键盘误路由的实机证据闭合。多键改变报告形态是解释组合差异的推断，未读取其他设备原始报告，不为此增加多键特判。键盘连接类型尚未由用户确认。
- **最小源码修复**：先补来源回归，旧实现七项来源隔离断言失败，再引入已验证的固定 WDF 完成请求来源。每个请求同步查询 queue→device→功能 PDO，以线程嵌套作用域关联复制 IOCTL；只接受当前 Helper 从已启动功能节点读回的精确 PDO。其他设备、缺失/失败查询、不同线程、旧来源代次和结束后的作用域不改写、不上报、不改变遥控器持键状态；原有同 handle 完成释放与停止回执保留，停止时配对撤除新增钩子。Helper 对固定 x64 系统模块路径及 SHA256 持锁核验，未知模块/架构零注入，不猜偏移；沿用租约传递私有身份，不记录 PDO/实例或控制帧原文。Agent/Helper 代次同步为 `2026-10-10.request-source`。这不是恢复已删除的旧生产链，也没有修改模板、连发或系统键盘设置。
- **自动化**：Agent 95/95，包括真实生产回调的来源、零帧、嵌套、线程、节点失效、按键释放及日志隐私；Helper 58 passed / 3 ignored，独立 fmt/check；工作区 669 passed / 22 ignored，fmt/check 与 Windows runtime-simulation 编译；前端 348 passed，生产构建；覆盖安装契约 5 passed。测试均在必要时使用正常 Windows 环境，sandbox 的访问/套接字失败不计为产品失败。Python Frida 传输自检适配了私有续约、测试进程来源桩和真实绑定拒绝用例，但现有 Python 无 frida，执行 `deferred`，不能以语法检查替代其运行。

状态：本次实体键盘误接管根因已确认，源码修复及自动化完成；10 月 10 日安装时新 Agent 未实际接管的边界保留在下文历史记录，10 月 12 日重启后的接管和用户观察见本节续验。当前模板没有显式连发，遥控器方向/返回不重复的配置行为与来源错误分别记录；此前为何能重复、三轮启动及 17:56 单击退格失效仍未全部闭合，不外推为同根因。安装和实际代次归本轮 Preview 记录。

2026-10-12 有限代码复核未发现当前来源分支绕过精确 PDO/线程作用域/代次校验而再次接管其他键盘的明确路径；这不等于全库或所有调度边界通过。另发现待回归的租约边界：Agent 在报告回调内先 `emitEdgesIfChanged`，后检查 `leaseOk` 决定是否清除原报告；Helper 对已鉴权边沿继续转发，false 租约心跳不立即撤销主程序捕获所有权，主程序另按所有权超时清除。续约迟到时是否会在原生已放行、映射仍认为持有的窗口双执行，尚未用可控调度回归或实机证实；原生首拍对冲对该路径的覆盖也未闭合。仅记录静态风险，不将它认定为本次历史症状根因，不据此修改生产代码、放宽来源检查或调整超时常量。当前菜单恢复不关闭下述驻留组件升级限制；音频 UI 两端选择循环及旧端点回滚仍归输入切换专项。

本轮最小材料复用 `target/dev/foreground-input/keyboard-coexist-current.log`、`hold-repeat-configuration.json`、`keyboard-readonly-probe.ps1/.log`、`source-keyboard-observation.log` 与 `agent-source-boundary-check.mjs/.log`。只读探针第二轮 20:16:36–20:18:36 有实际键盘对照并正常撤钩；第一轮无事件结果已记于正文，过程日志复用同一路径。准确旧包 `source-gap-baseline-e4d20971-setup.exe` 只用于本次源码不可由单一提交还原的故障对照，待新来源验收闭合后回收。原始日志/配置只留本机且不提交；正文保留脱敏字段、时间与计数。最初临时复制的三份历史轮转日志逐份哈希确认与原件一致后移入回收站，原件未动；相邻完整日志副本在提取相关时段后回收。只保留独有实验和必要包身份，不新增源码副本或过程台账。

### 2026-10-11 菜单已勾选但触发原生上下文菜单（当日现场未恢复）

冻结当前完整日志后按主程序 PID 聚合，仍为 10/10 20:28 启动的 50156、按键宿主 36976；设备宿主 11972 创建于 10/10 10:15:37，没有宿主代次变化。11:48:08、11:49:19、11:49:57、11:49:58 四次 Menu 均有成对 DOWN/UP，全部 `configured=false fired_count=0 reason=unconfigured`，无模板菜单打开。11:49:45 用户关闭再开启菜单选项，两次保存成功；最终配置 `menuTemplateSwitchEnabled=true`、自动跟随关闭、六个模板。开启后 `requested_mask=256 captured_mask=0 recognition_mask=0`。勾选已保存，但捕获权限没有取得，不是按键未收到或设置自动取消。

部署根因明确：上轮正常退出已撤除旧钩子，但未卸载旧驻留模块。当前清理回执仍为 Helper 46912 / host 11972 的 `agent_version_blocked`；启动记录确认 running=`2026-10-03.cleanup-instance`、expected=`2026-10-10.request-source`、`capture_acquired=false`。本次实际安装主程序/Helper 与候选 SHA256 均与 Preview 最终包一致，排除误开另一安装版本。**安装文件更新成功不等于新捕获脚本已加载；上轮菜单功能交付尚未完成。**

代码对应：`InputConfiguration::apply_input_capabilities` 在 RC003 所有权为零时移除菜单识别；统一报告模式的 LL 路径保留实体键盘原生输入，场景控制也拒绝对已经进入系统的 Menu 再次执行模板菜单。因此原生 `VK_APPS (0x5D)` 进入上下文菜单，与用户“像鼠标右键”的观察一致。不能删除门控或重新启用旧脚本恢复菜单，否则重现已证实的键盘误捕获或原生/映射双执行。

只读核对旧脚本命令集及 Frida 生命周期：当前 script/默认 ignore 实例无 reload 命令或 Frida 控制服务，写磁盘配置、重开主程序/Helper 不能追溯赋予热更新能力；来源归 ATTRIBUTION。不通过第二份 Gadget 或强制卸载共享宿主试错。恢复需旧宿主正常结束、新 Agent 加载，再观察配置 ACK 与实际菜单/键盘行为。Windows 正常重启需用户明确许可，本轮未执行；新来源链路未实测，不能预先保证重启后功能通过。

本次仅只读调查和文档更新，未改代码、模板、连发、系统键盘配置或运行状态，未重复构建/安装；沿用同内容自动化证据。`failed`：四次菜单无法打开；`passed`：保存状态、事件成对到达、载荷身份与版本阻断核对；`deferred`：新版实际接管、菜单短按/确认/长按、实体键盘共存和冷首用。最小现场复用 `target/dev/foreground-input/menu-source-current.log`，留到新来源验收闭合；原日志及轮转未动，不新增台账。

### 2026-10-12 用户重启后的来源、菜单与连发配置续验

本节缺省连发关闭是当时的规则与观察；用户随后明确要求“未设置长按时连续单击”，新规则与对应验证归下述补充及 [连发手册](../Testing/WindowsButtonGestureRepeat.md#2026-10-12-用户调整无长按动作时默认连续单击)，不反改当时证据为引擎丢拍。

本节时间均为北京时间（UTC+08:00）。用户已自行重启，公开启动时间为 10:06:55.5；本次主程序 10780、按键宿主 26108、Helper 29896、设备宿主 12192，区别于前两日旧进程。冻结的 `target/dev/foreground-input/post-reboot-current.log` 包含当天旧会话，以下结论按本次 PID 筛选，不将重启前记录混入新代次。沿用已安装的 `04ba26c6` 包及下文 Preview 中的完整载荷身份，本次未为观察重复构建或安装。

- **passed（新来源实际接管）**：10:12:01.692 Helper 与宿主完成鉴权，实际 Agent 为 `2026-10-10.request-source`，不再是旧版本拒绝；10:12:02.202 Menu 目标 generation 2、owned_mask=256 生效。10:13:02.449 心跳记录 `source_selected=74 / source_other=198 / source_unknown=120`，`clears_ok=23 / clears_fail=0 / restores_ok=23 / edges_sent=46`，证明当前来源判定与报告处理已实际运行；这些计数本身不替代用户可见输入结果。
- **passed（菜单打开、确认、恢复与用户观察）**：10:12:07.511 Helper 收到 Menu，07.564 主程序接受 DOWN，07.570 路由为 `Handled / native_delivered=false`；07.613 前台读回由 main 变为 menu，07.614 菜单快照应用。07.764 收到 Menu UP，取消等待释放状态且未追加手势。08.625 确认键 DOWN 被菜单处理，08.776 真实 UP 后，08.777 才请求 `source=menu` 模板选择；08.798 原窗口恢复读回通过、08.816 `target_restored=true`、08.817 隐藏菜单。用户随后明确确认“菜单和实体键盘都已正常”。此为本次 RC003 与当前 PC 的观察，不外推每种组合键、每个前台程序或 RC001。
- **passed（配置代次隔离）**：打开菜单使捕获目标 generation 2→3，确认模板使 3→4；08.877 拒绝迟到 generation 3 回执，09.384 当前 generation 4 恢复 owned_mask=7167，09.385 `template_selection source=menu actions_available=true`。中间 `actions_available=false` 是重新确认捕获期间的状态，不是又切回通用模板。菜单确认还记录保存默认请求完成，属于用户菜单操作，不据此认定发生自动异常切换。
- **passed（缺省被解释为关闭的实际配置与行为）**：只读核对当前 `preset-agent`：返回 Single=Backspace、四方向 Single=对应方向，五键 Double/Long 均 disabled、`holdRepeat` 均缺省，按当时 schema 解释为关闭。实际 revision 9 的 `repeat_single_mask=0 / repeat_long_mask=0`；Left 10:12:24.744–25.591 持有 847ms、Right 25.897–26.913 持有 1016ms、Back 30.206–31.621 持有 1415ms，各只执行一次 Single 并收到正常 UP，期间没有重新配置或丢失捕获所有权。现有证据符合缺字段在当时代码中的解释，不证明用户主动关闭过连发，也不支持“连发已开启但引擎丢重复”的归因。
- **deferred / 未知**：用户关于连发应由用户选择的讨论，不构成修改五键配置的授权；本轮保持模板与系统键盘设置，不重复迁移。开启“重复单击动作”或“重复长按动作”后的实体持续输出、UP 停止、冷/闲置首用、断连与完整双型号验收仍待验证。10 月 6 日上游历史验收不外推当前包。三轮启动及 17:56 单击退格问题保留各自未知点。

同次启动的语音问题独立调查如下；菜单、实体键盘恢复不说明语音已恢复。以上为已部署来源修复的现场续验，未新增修复提交；必要脱敏结论原位保留，原始日志仍仅本机留存至相关边界闭合。

#### 同次重启后的语音端点失效与识别修复

10:11:57.481–57.489 启动恢复先报告 `endpoint_identity_mismatch / persisted_endpoint_missing_or_changed`，严格配对随后因 `capture_missing` 失败并清空运行期写入端。之后本次进程十次语音开始均在 `audio_session begin` 因 `endpoint_not_selected` 中止，尚未进入默认麦克风切换、目标快捷键或输入法恢复。因此语音键已经到达应用，当前阻断点是音频路由，不据此修改语音按键或输入法时序。

只读公开 CoreAudio/PnP 检查确认：保存的两个端点均为 `NOTPRESENT`（状态 4）、`IsPresent=false`，它们的 PnP Parent 均精确对应启动失败的 VB-Audio 适配器，而非仅时间相邻。该适配器为 Code 10、ProblemStatus=`0x80000011`；同机另一实例 Code 0，驱动包同为 `oem88.inf` / 3.3.1.7。错误码表示设备忙，不足以解释为何产生两个实例或为何原实例启动失败，这一层根因仍未知。未禁启、重装设备或操作无线电。

当前可用实例的官方属性还暴露两处应用识别缺口：接口友好名带 Windows 数字前缀 `2- `，以及实际 pin 名为 `CABLE In 16 Ch`；旧代码分别只接受无前缀家族名和无空格的 `16ch`。先新增两项回归并观察失败，再最小修改 `audio_route.rs`，仅接受纯数字前缀与该精确官方拼写，仍以精确 PnP 适配器身份配对，不按相似名称替换用户目标。配对日志增加所选 Capture 是否存在、元数据/家族是否识别和已知端点计数，不记录名称或身份。

13 项路由单测通过。生产 resolver 的真实只读检查证明：显式指定当前 Capture 的同线 16Ch 写入端可以配对，零默认设备写入、零音频流；未指定时，另一写入端仅能读到“扬声器”，没有足够证据证明标准 CABLE Input pin，仍按 `RenderMissing` 拒绝自动配对。此边界不通过新增 Speakers/扬声器 fallback 绕过。原保存目标与新实例并非同一身份，调查期间未擅自替换；用户随后明确同意使用已核实的新同线通道。正常退出后只更新四个端点字段，保留临时麦克风切换与全部其他设置；启动严格配对、真实语音和四次原默认角色恢复已通过，打包部署与完整身份归 [Preview](../Testing/WindowsRC003Preview.md#2026-10-12-音频通道识别与语音键反馈修复)、输入切换专项。该次显式配置恢复不代表两端 UI 选择循环或旧目标失效的回滚缺口已经修复。

用户同时报告语音键没有界面反馈。代码确认 `ButtonsPage.voiceActive` 原先仅判断 `connection.voiceState=streaming`，音频准备失败退回 Idle 后，收到的按键也不再显示。最小修复新增独立 ATVV Start/Stop 观察，经现有 scene snapshot 即时传递，不改变录音状态或快捷键控制。观察位于当前连接代次检查之后、Idle 状态的 Stop 早退之前；断连、连接失效和退出清除。该状态表示已归因的遥控语音控制生命周期，不声称能读到实体触点或在设备自动停止后继续代表持有。独立字段契约回归先失败，修复后四项控制器/管线组合测试通过；旧代次与清理接线另经代码核对，未以单测冒充真实 BLE worker 失败注入。

前端复用现有 scene 订阅，以独立观察高亮、以 streaming 显示语音进行中；同步观察快速 DOWN/UP 后仍保留顶部收到按键说明，不延长按住状态。初始读取与新事件同代但迟到时不覆盖；卸载后回调失效。另复现并修复旧 streaming 状态导致本次错误被“语音已结束”遮蔽的问题，当前错误优先于结束提示，不推断未证实的开始事实。最终定向 78 项、全前端 355 项通过。

11:19:00.519–49.903 主程序 30736 观察四组 Start/Stop，前端每组都在约 2–5ms 后记录 pressed/released；真实 UIA 观察到按下 badge=received、收到按键说明，结束后“已收到语音键，语音已结束。”与“按住说话”可见。短期 UIA 连续观察因空控件名提前结束，没有将它记作完整 45 秒采样；事件链与终态读回独立有效。用户明确回复“按键反馈和语音输入都正常”。四组音频均有非零提交、队列排空与原三角色恢复，恢复 journal 为 null；本次当前 PC/RC003 正常通道的反馈与语音 **passed**。错误通道在新安装包上的受控实机显示、真实断连/睡眠中清理、其他输入工具长闲置首用和 RC001/ARM64 仍未验证；不据此关闭整个历史 Bug。

最小脱敏证据复用 `target/dev/foreground-input/post-reboot-audio-{topology,metadata,association,route-red,route-green,route-readonly}.log` 与 `voice-button-feedback-{red,green}.log`，用于复核本次未知端点与回归。最终实机用例已分开，`explicit_16ch` passed 与 `standard_writer` failed 分别记录，避免一条用例混写终态。另保留既有菜单/键盘已确认候选 `source-confirmed-04ba26c6-setup.exe`（SHA256 `04ba26c64bfcccaff065c00aedf3746672c29e188fbe827c73c87dd7de2bbaf2`）供新增语音修复的回退对照。新候选现场验收与未知边界闭合后按 LOGGING 提炼/回收；未创建源码副本或重复台账。

## 验证与留存

2026-10-12 后续两端恢复事务：旧 Capture 缺失、只有显式 16Ch 可用时，两个保存入口循环要求旧另一端可用的根因已由隔离回归复现；当前以用户明确选定的未保存 Capture 草稿和高级 writer 一次严格配对提交解决。旧目标不可用时的配置身份回滚另有独立恢复路径，不依赖硬件读回；审查发现的旧名称读回超时缺口已用真实 controller 阻塞注入先红后绿修复，保留取消/超时与语音互斥。组合自动化、失效场景的 mock/controller 边界和实际部署归 [输入切换专项](../Testing/CaptureInputSession.md#2026-10-12-两端恢复事务与失败回滚)。不修改当前正常通道来制造故障，不把旧 Code 10、三轮启动/17:56 或暂缓的捕获安全换代与租约问题整体关闭。

2026-10-12 15:53 后组合候选已安装，真实日志五字段与 manifest/载荷一致，完整内容身份归 [组合 Preview](../Testing/WindowsRC003Preview.md#2026-10-12-音频两端恢复与构建身份组合交付)。用户确认该包方向/返回连发松开、实体键盘实际键和方向长按、闲置首次/连续语音及反馈正常；菜单首次在 Codex 前台出现后释放瞬间消失，误开主页后恢复，切回 Codex 后仍正常，菜单当次验收 **failed**。旧现场 generation15–27 的 `requested=false` 将窗口操作前置结果与激活 API BOOL 合并，不能据此断言 13 次都实际调用 API 并返回 false；首hide在21.436、UP在21.564，确认关闭早于UP，但不能归为UP触发。文档已在15:53加载；主页与后续 foreground/focus 时间相关不证明恢复因果。最小脱敏现场 `target/dev/foreground-input/audio-recovery-menu-regression-evidence.log` 为548行/199,632字节，保留身份、代次、菜单、隐藏、主页及焦点关键事件。

### 2026-10-12 菜单诊断候选与根因边界

诊断包 `e39e642c42299f440492838a9fb268c9e17a8a0c5f7a0fbdb1e519636f7a374d`（12,858,164 字节）基线 `af710b757897cb0567b63c63a7ceb40f62426652`，source fingerprint `fb31da04e942670b294ec83aa4a0015db84f787d149fc95168f260900b27d77d`，dirty/local/included，`sourceCommit=null`。raw EXE SHA-256 `84cc2f515d014003f47fa010af6ad52cf21e27b721940cf4849f78fa0b5d2219`，installed EXE `94a6ab4999e058e537de51ec4de354b2f5558b70625fc970d976cc0c22705f66`，各 20,696,576 字节；除 NSIS offset 15467490–15467492 的 ASCII `UNK`→`NSS` 构建标记外与 raw 一致。正常退出约544ms、held=0、cleanup passed；覆盖安装四 payload/四配置字节保持，Explorer ShellExecute 普通权限启动 App 36272/子21196、Helper20812，五字段身份匹配。NSIS 生成及核验 passed；Tauri 总命令仍因缺 updater 私钥退出1，无 `.sig`，未签名本地候选。本包仅加诊断，没有改变关闭策略。

用户在 Codex 前台未先打开主页/主窗时再次短按 Menu，仍观察到菜单出现后消失，验收 `failed`。现场 `target/dev/foreground-input/menu-diagnostics-current-repro.log`（80,981字节，SHA-256 `f322012c6888137f35b964a83e932b19f719e5ca34e4264d82a4fcafd48997b9`）记录 generation8–18 共11次：面板 visible=true、minimized=false、no_activate=false，调用方为窗口线程；激活 API 实际返回 false，实际 foreground 未变，随后程序清面板并隐藏。由此确认直接消失原因是激活拒绝被本地策略当作立即取消；native focused=true 不等同真实前台。只读 Git blame 核实，该 `requested && overlay_is_foreground` 门禁及失败后清理/隐藏策略来自本地 `7d9b068d`（2026-09-27），清理与返回目标处理强化来自本地 `51f0f5c5`（2026-10-05），不是 0.8.6 合并新引入；来源见 [ATTRIBUTION](../ATTRIBUTION.md#2026-10-12-菜单激活失败后的本地关闭策略来源)。Windows 为何拒绝激活仍未知，尚无行为修复。Host 全工作区 695 passed/0 failed/23 ignored（`menu-diagnostics-final-workspace-test-host.log`）；一次 sandbox 超时只结束测试进程，不构成代码回归。上一组合包通过的连发、实体键盘和语音观察不外推到本诊断包。捕获 lease/hotreload 依用户要求暂缓，更新源未改；后续菜单行为修复归当前诊断任务，不在本记录声称已解决。

### 2026-10-12 菜单恢复实现与自动化（局部用户交互已实测）

经独立审查后，菜单行为修复的五文件工作区已冻结，无 P0/P1/P2 必修项。源码变化集中在 `scene_control.rs`、`application_control.rs` 的只读 window ID getter、Windows library 透传、Tauri platform 透传及 Tauri menu host。首次 `SetForegroundWindow` 被拒时，仅当同次开窗捕获目标仍为实际前台才保持 remote-only 菜单；真实键盘操作及确认严格使用该菜单 HWND。每次遥控/原生手势与 UP 重新读回实际窗口，前台切出即取消；开窗 pending、前台观察、语音/配置取消后的迟到 UP、Menu Long 保存和旧代次清理均有回归覆盖。未改模板、前端、更新源或捕获 lease/hotreload。

自动化 **passed**：`cargo test --workspace` core 32 passed、Windows 522 passed/22 ignored、Host 153 passed/1 ignored；其余 workspace 库及文档测试通过。窄测 71 passed/1 ignored、Host 菜单专项 7 passed；`cargo fmt --all -- --check`、`cargo check --workspace`、`cargo check -p sayall-windows-app --features runtime-simulation` 与五文件 diff check passed。日志位于 `target/dev/foreground-input/menu-fix-workspace-test.log`、`menu-fix-workspace-check.log`、`menu-fix-runtime-simulation-check.log`。最终 `scene_control.rs` SHA-256 前缀/后缀为 `022F1FE0…375`；仍是 dirty 工作区，不以 HEAD 代替完整内容身份。

上一个 e39 诊断候选上的用户菜单复现仍是 `failed`，不因新包观察回写。修复候选 `无线麦_0.8.6_x64-setup.exe`（12,870,141 字节，SHA-256 `370b51b214cefac324d6966ee0631188d5155db7faa9b9978e9a7834c264c32b`）已正常覆盖安装并普通权限启动；安装/启动身份、四配置保持和 payload 核验通过，精确身份及签名边界见 [最终候选 Preview](../Testing/WindowsRC003Preview.md#菜单恢复修复候选安装用户观察)。用户明确回复“一二三都成功了”：Codex 前台短按 Menu 后松开菜单保持；遥控方向选择模板并确认后正确应用；重新打开菜单后切换到其他软件，菜单关闭。记录为本机本次用户观察 `passed`，不外推 RC001、其他 RC003、长期稳定或完整按键矩阵。

结构化运行日志由 `build_identity_fix` 只读核对，五个身份字段与包记录一致（base `af710b757897cb0567b63c63a7ceb40f62426652`、dirty、sourceCommit=null、fingerprint `ad1c0484b521c21bd5ebd5a220b66c13c168ee51d857a89021cdda1af56cdaaf`、local、included）。generation 13 于 `10:37:17.776Z`（北京时间 `18:37:17.776`）activation API 返回 false / actual_foreground=false；窗口可见、未最小化、no-activate=false、调用方为窗口线程。随后 `activation_denied_policy` 记录 panel_before=true / panel_after=true，`snapshot_applied` passed，确认本轮真实运行出现“激活拒绝后面板保留”状态。generation 22 于 `10:38:15.075Z` API 与实际前台均成功且面板保持；generation 23 于 `10:38:16.756Z` 模板选择 `actions_available=true`，随后有 `focus_ignored hide_completed`。脱敏证据为 `target/dev/foreground-input/menu-focus-fix-user-validation-evidence.json`（1,823 字节）。没有用户动作关联 ID，不能唯一断定 generation 13 对应用户第一项操作，也不能证明“拒绝→继续遥控→确认模板”完整同次链；日志无 remote-only 专属字段，不能宣称该语义路径专项覆盖。Windows 拒绝激活的具体原因仍未知。捕获 lease/hotreload 暂缓，更新源未改。

当前候选的用户三项操作已按上段记为本机观察 passed；对应细节、范围与保留条件见 [最终候选 Preview](../Testing/WindowsRC003Preview.md#菜单恢复修复候选安装用户观察)。保留 `370b51b2` 当前部署候选、e39 失败对照和 `5fd60bef` 音频/构建身份基线，待完整链路验收及引用/唯一性核对后再决定回收；当前不删除。

- `passed`（2026-10-04 最终候选）：本地 0.5.0 NSIS 包 SHA256 `bd79c2c1eb3d4c1ed48e905761a812803e026e8a7adba234085c85b718f844b0`，12,582,424 字节；Helper `f02adfc9ba114cdd6362e238de16b2bca333e57c68728e3618453f1c7d956912`，内嵌 Agent 字节已核。09:30:52Z 完成管理员覆盖安装，App 53572 经真实 Explorer 普通用户启动，主程序和 Explorer 的 TokenElevation 均 0，安装 App `a3e6e36f57cbd2c45a76acb89784be550461b97faf49247ec40cac547f62946d` 与构建载荷除 NSIS 标记外一致；三份用户配置在安装阶段保持，主窗口可见。
- `passed`（同候选真实故障分支）：Helper 68852 确认原宿主旧 build 后退出，写 `agent_version_blocked`、`capture_acquired=false`；App 状态为 `cleanup_pending=false`，实际 UIA 文本和截图显示旧组件被 Windows 占用的具体说明及可用关闭按钮，蓝牙连接单独显示。随后请求自身正常退出、实际 Installer 清理检查返回通过，再由 Explorer 普通权限重新打开；没有强杀或再次手工改回执。此为真实版本拒绝与收尾验证，不是新 Agent 接管或三个按键功能通过。
- `passed`（2026-10-04）：前端完整 `pnpm test` 242 项/21 文件、`pnpm build`；后端状态专项先红后绿，17 项通过；`cargo fmt --all -- --check`、工作区完整测试及最终普通/仿真 check 通过，最终 Host 全套 82 passed / 1 ignored。Helper 全套 40 passed / 1 ignored、独立 fmt/check 通过，Installer SelfTest 9 项通过。Helper 冻结源码 SHA256 `bd2b0b45b82f0972843720812f16093a8e4638b176fd7d0810509781ed8a144b` 经独立复核；新增回归先红后绿，未知身份修正已验证。此前核心/Windows 分组未再改动。两次一次性停止工具的 12/15 个隔离测试与上述真实撤钩/释放通过；停止不外推模块卸载。
- `passed`（2026-10-04）：Windows Debug Tauri/WebView 真实 IPC 仿真 19 步通过。初跑在已连接状态的连接页扫描按钮禁用处失败，改走驱动页、实际展开配对区并扫描后通过；这是测试导航修正，生产连接逻辑未改。最终具体错误文案和无资源终态由对应自动化及安装现场另验，未把前一仿真运行外推为这些新增分支通过。
- `failed`：上述用户操作及对应失联、菜单关闭阻断、五次语音中止。
- `passed`：后续两轮出现实际 WeType 响应证据；此前安装、普通权限启动、授权/正常撤钩验证仍仅覆盖其原范围。
- `passed`：2026-10-03 修复源码执行 `pnpm test`（21 个文件、232 项）、`pnpm build`、`cargo fmt --all -- --check`、`cargo test --workspace`、`cargo check --workspace`、`cargo check -p sayall-windows-app --features runtime-simulation`。工作区测试主要分组为 core 31 + 4、Windows 329（19 ignored）与 3 个独立集成用例、Host 77（1 ignored）；既有 warning 不作为功能通过依据。音频专项 21 项覆盖突发入队、样本预算、取消/迟到代次、合并唤醒和失败冻结；菜单专项 33 项覆盖真实返回目标、失败保留、锁外查询及代次保护。关键回归用例均先观察到旧实现失败，再修复转绿；独立代码审查发现的满队列唤醒与失败统计竞态也已修正。
- `passed`：追加 follow-app 修复后，独立 Helper 工作区 `cargo test --manifest-path hardware/RC003/helper/Cargo.toml --locked` 为 36 passed / 0 failed，`cargo fmt --manifest-path hardware/RC003/helper/Cargo.toml -- --check` 通过，源码 SHA256 `d11597f04aa8910dc6eb98fe8b0b3e53cbf50216a36e4bbf24d4ac7464d2d767` 经独立审阅确认。1 个 ignored 项是父测试明确启动的子进程入口，父测试已观察其正常退出码，不代表漏验该子进程。真实 Windows 自有命名管道和自有子进程覆盖服务端 PID 核对、存活、正常退出后句柄 signaled、原实例身份保留与手动模式重连；不触及现有设备或主程序，不能代替 RC003 验收。此前清理修订的 `--selftest` 结果保留在其原范围；本轮不为重复自检追加运行现场日志。
- `deferred`：真实窗口焦点探针执行两次（终端启动、真实 Explorer 普通桌面用户启动），均在测试准备阶段因 `SetForegroundWindow(main)=false` 且实际前台未改变而失败，未进入修复路径。测试窗口均通过 RAII 销毁，未修改系统前台策略。不能用这些失败推断修复无效，也不能将单元测试扩张为真实窗口行为通过。对应测试为 `scene_control::tests::own_menu_return_uses_real_windows_foreground_on_the_calling_ui_thread`。
- `deferred`：修复后原始前台复现、严格前后台对照、冷/闲置首用、RC001 与 RC003 分别验收。未执行电源/无线电实验。
- `passed`（2026-10-03 当时构建）：设置 `SAYALL_BUILD_CHANNEL=local` 后执行 `pwsh -NoProfile -File scripts/build-local-input-candidate.ps1`，生成包含 follow-app 修复的 0.5.0 本地 NSIS 包（12,586,448 字节），SHA256 `ce4ba00185dad28c1e537e6dde3bd967df75b8bbea6bacaba2e51aa128394e12`。该中间包已被本页 10 月 4 日最终候选替换，不另存副本；安装状态不外推。
- `blocked`（2026-10-03 当时安装步骤，已由 10 月 4 日真实停止证明解除）：旧 Helper 37764 误退并留下 unconfirmed，当时 App 51996 保持运行，未安装新包，仿真仅完成编译检查。新代码本身不能反向证明旧实例安全释放，不能据此强杀或伪造回执；后续恢复必须采用上文独立实证。整个修复未创建提交、推送或发布。
- 完整现场只在本机 `target/dev/unified-input/regression-app.log` 与 `regression-window.log` 保存，旧版聚合方法与脱敏结果为同目录 `voice-baseline-aggregate.ps1`、`voice-baseline-summary.json`。已保留唯一故障安装包 `regression-02515fb8-setup.exe` 供必要对照，SHA 与上文一致；问题闭合后保留本页脱敏计数和必要片段，其余按 LOGGING 回收。必要候选差异/包身份沿用同主题记录，不复制整仓库。
- 新包位于既有 `target/release/bundle/nsis/` 输出目录，源码及包身份为本机同主题 `regression-package-identity.json`；最终测试、构建、真实窗口前置条件失败日志及回归 red 证据保留至原问题验收。菜单精确差异已刷新并反向只读校验；音频/捕获旧源码可由基准 Git 对象恢复。已被最终测试覆盖的过渡日志、重复源码快照与空格式日志回收，不保留第二份修复包。
- 10 月 4 日留存补充：同目录 `cleanup-ui-*`、`frontend-layout-*`、`helper-activation-*` 和 `version-blocked-*` 保留当前红绿/编译结果；`helper-activation.diff` 已校验可反向恢复，重复 before 源码与空/过渡日志回收。`legacy-agent-*`、`legacy-cleanup-*` 与一次性工具仅用于核查两次真实恢复及原回执来源；`regression-deployment.json`、`regression-runtime-verification.json` 和 `installed-driver-layout.png` 留作实际安装/布局证据，三个 `layout-*.png` 为模拟状态对照。相关问题验收闭合后保留本页脱敏结论，其余按回收站规则整理；本轮已回收 9 个核对过的重复过程文件及 Helper 合成测试目录，未清理其他任务或历史 `host-ui-*` 材料。
- 隐私检查：本页仅包含时间、临时进程号、计数和源码/包指纹，不含个人路径、设备身份、窗口标题、语音内容或凭据。
- 晚间安装验收继续复用本机同主题目录：`interactive-install-*.json` 与两张选项截图保留两条真实向导的同用户权限、配置保持和文件状态证据，`reinstall-flow-test.log`、`reinstall-recycle-test.log` 与当前部署/运行/包身份摘要保留至本安装问题验收闭合；随后保留本页结论并按回收站规则整理。已核对并回收九个不再使用的临时界面检查/导航脚本、摘要和可再生 NSIS fixture 文件，共 70,246 字节；没有移除本地交付包、硬件故障材料或含 junction 的测试边界目录。
- 本轮恢复与覆盖安装过程仍复用 `target/dev/unified-input/`：保留 `reset-*`、`helper-reset-*`、`installer-*` 中必要红绿/构建/诊断日志、已验证可反向重建的最小 Helper 差异及现有部署/运行/包身份摘要；重复 Helper 源码与两份空格式日志已回收；安装器的 19 个已核对普通再生产物（3,179,218 字节）亦经 IFileOperation 确认进入回收站，必要句柄实验日志及两份小探针保留用于首次 1603 复核。当前包只在既有 NSIS 目录留一份，不复制旧候选；首次 1603 与两次成功对照的脱敏结论已写在本页。含 junction 的隔离回收夹具保持为测试边界证据，不整体递归清理。留存仅用于当前旧宿主边界、覆盖异常与实体验收，闭合后按同一规则回收，不扩展至历史任务材料。

### 2026-10-12 后续要求：缺省长按连续单击

历史归因补充：`9f4ead5^1` → `9f4ead5` 的手势代码对比确认，0.8.5 合并把“只配单击时默认连发”改成了“必须显式指定连发槽位”；本地同时保留旧模板内容、不应用上游隐式连发迁移，导致无字段模板的实际行为改变。此前仅归纳为“当前配置关闭”不完整，不能归责为用户改错设置。这项变化早于 0.8.6；它解释缺字段模板不重复，但不补全此前每次成功输入的原生/映射路径，也不关闭 17:56 短按失效或首次启动所有权问题。来源边界见 [0.8.5 合并记录](../ATTRIBUTION.md#2026-10-09-上游-085-合并)。

用户确认语音反馈与输入恢复后，再次指出方向/返回长按不连续，并明确要求未配置长按动作时默认连续执行单击。11:37:51.963 当前主程序 30736 的 revision 38 仍为 repeat_single_mask=0 / repeat_long_mask=0，符合旧缺字段关闭规则；没有证据证明已开启连发被丢失。新预期是默认规则调整，不归因于旧宿主、输入法焦点或音频通道。

最小改动以已有手势状态机执行默认自动规则，明确关闭单独保存为 off，显式单击/长按保留；时间常量、动作白名单、来源捕获、实际应用/编辑分离及语音路径保持。历史缺字段无法区分未设置与明确关闭，按用户新要求解释缺省；不批量改模板、不新增迁移。只用于场景/任务识别的占位动作明确关闭重复，诊断记录有效单击/长按、自动与明确关闭掩码。新增无字段默认长按行为先实测失败（350ms 没有续拍）；新增菜单/任务/合并占位三项在新默认下亦先红，随后加窄护栏。完整自动化、候选与实体结果统一归原连发手册和 Preview。

`5fd60bef` 包正常覆盖安装后，新普通权限主程序 46728 于 11:58:15 应用 Agent，有效默认单击连发掩码为 6385；随后左/右/返回七趟共 50 续拍均以 release 终止，用户明确确认连发、松开停止、菜单和实体键盘均正常。本次当前 PC/RC003 默认规则 **passed**；明确关闭/长按模式的完整实体矩阵、其他按键逐项、RC001、长闲置及断连睡眠等仍 deferred。该结果不关闭原三轮启动未知、音频旧设备 Code 10 或其他独立问题。完整身份、命令、安装和剩余边界归 [本轮 Preview](../Testing/WindowsRC003Preview.md#2026-10-12-默认长按连续单击修订)；现场复用 `target/dev/foreground-input/hold-default-lifecycle.log`。

### 2026-10-15 Marvis 前台失效：管理员窗口输入边界

- 状态：权限差异已确认，恢复验证 `deferred`；本轮只读诊断，未修改生产实现、第三方程序或运行权限。
- 复现：用户确认北京时间 15:55 再测仍失败。Marvis 中无线麦可见语音键按下和收音，但微信输入法绿色话筒未显示；实体键盘 Ctrl+Win 可用，遥控 Menu 能打开模板面板，其他映射无效。设备具体型号本轮未核，不外推双型号。
- 被测运行身份：主程序 PID 59956，0.8.6，基线 `af710b757897cb0567b63c63a7ceb40f62426652`，`source_state=dirty`、`payload_inputs=included`、`build_channel=local`，指纹 `8a38c43cce592fff5636e0aa9aafa06abd38f0b6092b21e73e34cff1cc774007`。这是安装实例身份，不代表当前工作区所有字节。
- 日志证据：默认诊断日志 07:55Z（北京时间 15:55）全分钟共 23 条 `map_inject`，其中 8 条 `echo_delta=2`，15 条 `echo_delta=0`；零回声集中于 07:55:50.174–07:55:54.379，覆盖方向、确认及音量映射，均已到达 `map_fire`，不能归为未配置或设备没收到。session 93/94/95 均在 800ms 与约 2500ms 记录 `mic=not_observed`，对应 generation 339/340/341 实际收到 236/142/90 个音频包，解码错误为 0，并以 `control_stop` 收尾。07:55:56.495 模板菜单 `actual_foreground=true result=true`。同分钟 session 92 曾 `mic=observed`，因此不能把整分钟都描述成失败，或在没有逐次目标身份日志时认定每一条都发生在 Marvis。
- 权限实测 `passed`：通过公开 OpenProcess / OpenProcessToken / GetTokenInformation 只读查询，Marvis 26192、MarvisAgent 61200、MarvisHost 75376 均为 `elevated=1 integrity_rid=12288 ui_access=0`；无线麦主程序 59956 及按键宿主 34392 均为 `elevated=0 integrity_rid=8192 ui_access=0`。微信输入法 renderer 为中完整性且 `ui_access=1`，不能以其工作状态反推普通无线麦具有相同跨权限能力。
- 可执行文件公开清单实测 `passed`：仅以数据资源方式读取 EXE 的 RT_MANIFEST，Marvis 明确声明 `requestedExecutionLevel level='requireAdministrator' uiAccess='false'`；Agent 为 `asInvoker`，Host 未找到此声明。当前 Marvis 主程序要求管理员权限，不能把“取消快捷方式的管理员勾选”当作已经成立的解决方案。未读取第三方私有配置、数据库或进程内存，未更改清单或尝试绕过提权。
- 归因：管理员目标与普通无线麦之间的完整性差异是已确认的兼容边界；[Microsoft SendInput 契约](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendinput) 明确限制注入到相同或更低完整性的应用。该限制能同时解释普通映射和模拟 Ctrl+Win 不响应、实体键盘仍可用，以及应用内部 Menu 仍能打开。当前日志不能证明第三方具体消费情况，零回声本身也不能独立证明 UIPI；本轮尚未做同目标/同配置的受控权限对照，不将推断写为修复通过。
- 验证边界：`failed` 为用户复现及上述语音/映射结果；`passed` 只覆盖日志分层、进程令牌和公开清单读取；`deferred` 为兼容方案、Marvis 实际恢复、冷态首用及双型号矩阵。主程序保持普通权限，现有硬件捕获 Helper 不自动等于已经实现管理员窗口按键投递。本轮没有运行电源、无线电、额外输入注入、重启或部署实验，没有生成过程文件；日志中必要脱敏计数已归本节，不复制完整日志。

### 2026-10-15 晚间 Menu 未唤醒：捕获续约超时与桥接重连

- 状态：已完成最小修复、自动化与负载对照、正常覆盖安装和运行身份验证；2026-10-16 用户确认 VSCode 菜单和语音正常、无调试弹窗，实际日志支持菜单打开与四段语音正常结束，最长33.895秒。超过45秒实体语音、菜单关闭恢复及双型号完整矩阵仍未通过；新实例仍有两次短暂所有权超时，详见本节末尾。以下为最初只读调查事实，当时未改捕获生命周期、配置或部署；同晚后续授权、修复及验收分别见本节末尾。
- 现象：用户反馈切换到新界面后偶发按键功能对不上、Menu 无法唤醒模板切换，询问是否因模板过多；随后明确补充“没有模板切换的弹出，但是有鼠标右键的那个菜单弹出”。这是用户对实际可见结果的确认，符合捕获失效时原生 Menu 透传到前台程序的行为；没有据此认定应用注入了鼠标右键。尚未获得每次失败的目标程序或精确动作对应，不把所有“功能对不上”归为单一原因。
- 被测身份：实际运行 App PID 64252，1.0.0，基线 `8b9dd31460f1b3b4c6859b2216de61ede0966206`，指纹 `4487283408be7d7c703c2fcd9be074b5003c1c1f57ae3129c5b8f0048d4014c9`，dirty/local/included；Helper PID 78564，设备宿主的已鉴权 Agent 报告 build=`2026-10-10.request-source`。身份来自活体日志，不以当前工作区字节或版本号替代。
- 方法与范围：读取默认诊断日志及现存五份轮转日志，先按小时/PID 聚合，再核对失败窗口。统计固定截止 2026-10-15 14:25:00Z（北京时间 22:25:00）；该 App 当前保留记录从 09:15:34.197Z 起，累计 71 次 `ownership_timeout`、32 次 `closed reason=silence_timeout`、73 次所有权恢复（含初始取得）。不同事件不能按一对一事故计数；恢复事件也不单独证明按键实际生效。
- 本机配置只读摘要：6 个模板，`menuTemplateSwitchEnabled=true`、`buttonMappingFollowEnabled=false`，存在手选模板记忆。本次失败前后 14:16:45Z–14:25:00Z 没有 `targets_changed` 或 `template_selection phase=requested`；目标代次保持 4。没有因模板数量进行删减或改配置。
- 对应失败链（北京时间）：

  | 时间 | 实际观察 |
  | --- | --- |
  | 22:17:00.500–00.506 | 蓝牙仍为 connected=true、requested_mask=7167，但 captured_mask/execution_mask/recognition_mask 全部为 0；模板来源仍为 remembered，actions_available=false；随后明确记录所有权续约超过 1500ms，原生输入放行 |
  | 22:17:01.986 | 桥接超过 3000ms 没收到数据，silence_timeout 断开 |
  | 22:17:02.215–10.567 | 四次 Menu DOWN/UP 共八条边沿均收到，但 configured=false / fired_count=0 / reason=unconfigured，没有进入菜单打开流程 |
  | 22:17:04.454–14.934 | Helper 已重新鉴权；Agent 回执尚未恢复，出现 voice_synth_resend 与 agent_never_acked，说明“桥接已连”仍不是捕获 ready |
  | 22:17:17.335–18.273 | 所有权恢复，实际映射重新可用；下一次 Menu fired，菜单实际前台核验与 native_focus 均通过 |
  | 22:22:56.060–22:23:34.683 | 另一次所有权中断后经历重复重连，至恢复共 38.623 秒；这段间隔直接来自运行日志，不是推算的自愈承诺 |

- 直接原因：`rc003_bridge.rs` 所有权回执超过 1500ms 会撤销捕获资格，超过 3000ms 收不到桥接数据则断开；`button_mapping.rs::InputConfiguration::apply_input_capabilities` 在 RC003 captured_mask=0 时过滤执行和菜单映射。因此已抓到的四次 Menu 失败发生在模板选择之前，属于捕获资格暂时失效；保存的模板数据并没有被清空。复核 22:17:00–18 的四次失败按压共八条 Menu 边沿，均为 configured=false，同窗口没有 map_fire button=Menu；结合 native=passthrough 和用户观察的上下文菜单，可解释为原生 Menu 进入前台程序，未进入模板执行链。用户未给出逐次时间及目标，故不将右键菜单观察与某一条边沿强行一一关联；其他未对齐目标/时间的失败仍未知。
- 深层归因边界：同一统计窗口 Helper 诊断已观察到 append 最大 9191ms、bridge_queue 最大 10742ms、agent_rx_age 最大 9672ms、bridge_write 最大 823ms。源码 `Logger::line` 和 `latency_summary` 同步执行文件追加，桥接队列也确有秒级延迟。22:23:29.770 记录 bridge_queue=5931ms、agent_rx_age=6597ms，随后桥接再次超时。这些是已测事实和需要继续验证的阻塞方向，尚无同次受控实验将具体续约超时归到日志、线程调度或底层 Agent；不能把最大耗时简单相加，也不能认定前台切换触发断链。维持原超时与安全失效策略，不凭推理延长常量或绕过所有权校验。
- 独立菜单关闭边界：22:17:19.677 原目标恢复失败，reason=stale_window；菜单按现有策略保留，随后 close 被 target_restore_failed 阻止。它发生在菜单已成功打开之后，需要与前述“未唤醒”分开判断；目标具体为何失效尚未知。
- 验证：`failed` 为上述四次实体 Menu 手势未触发、续约/桥接超时与一次恢复原窗口失败；`passed` 仅指日志聚合、配置摘要与源码分支相互印证，以及随后菜单实际前台核验；`deferred` 为底层阻塞的受控归因、修复、冷态/连续/断连/睡眠和双型号回归。本轮没有生产代码变更，不重复构建或运行与定位无关的测试。
- 隐私与产物：只保留上述脱敏计数、时间、运行身份和最小事件链，不保存模板名称/自定义按键、目标窗口内容、设备身份或个人路径；未复制全量日志、生成临时脚本或新增过程台账。既有现场材料和其他任务未提交修改保留。

#### 同晚获准修复：受控日志阻塞与最小候选

- 用户随后明确“现在修复这个问题”，本段仅推进上述 Menu 未唤醒故障，不恢复其他已暂停的捕获设计或电源实验。
- 已确认缺陷：`handle_line` 的 HB 首条/状态变化日志先同步输出，再投递所有权；edge 也先日志再转发。桥接线程与持有续约流锁的续约线程同样同步输出。初次独立闩锁实验中旧同步实现实际在 1.71 秒后仍没有投递所有权；随后审查收紧实验同步：先确认共享 sink 闩锁已经阻塞，再接收心跳和真实 Menu usage `0x0065` 的按放。最终可复核红测仅临时将捕获线程的 Logger 置回旧同步输出路径，实际 1.92 秒后在所有权断言失败（退出码 101）；移除该注入后绿测通过。此受控实验结合现场 append 秒级延迟证明日志阻塞能直接触发该失败链；没有证明原现场 71 次超时均且仅由日志造成，也没有将各延迟最大值相加。
- 最小修复范围：仅 `hardware/RC003/helper/src/main.rs` 的日志投递与收尾。捕获开始前建立单个 256 条有界日志 worker，接收/桥接/续约线程共享 `try_send`；stdout 与共享日志追加/轮转/回退都离开控制路径。满时只丢诊断并恢复后记计数；所有权、绝对按键状态及释放仍用原通道。日志线程无法启动则拒绝开始捕获。Agent/桥接清理后关闭日志队列，正常排空后 join；永久慢 sink 最多等 2 秒，只 detach 诊断线程，不伪称取消磁盘 I/O 或清理成功。原所有权/桥接超时、鉴权、代次、来源和边沿配对均未更改；`stale_window` 仍是独立未解决边界。
- 自动化：最终精确 Menu 夹具红测 `capture_log_stall_tests::slow_log_sink` 实际 1 failed（预期）；移除同步输出注入后完整 Helper 套件为 61 passed / 3 ignored，新 3 项覆盖受控慢 sink 下所有权和真实 Menu usage 的 DOWN/UP 仍在 1500ms 内转发、满队列的有界丢弃/恢复计数与共享 worker、超时/重复关闭及迟到 sink 不重开队列。异步共享日志成功及共享失败转备用落点沿用实际文件读回测试。
- 候选仍为当前 dirty 工作区，不以 HEAD 宣称可重建；本候选及后续 Voice/audio 扩展已完成独立审查与最终工作区检查，部署与当前实体验收边界见本节末尾。未改用户六模板、跟随开关或手选记忆，不把自动化通过当作 RC001/RC003 实体通过。
- 本轮过程输出复用 `target/dev/menu-capture-reliability/`：`red.log` 保留预期失败对照；`helper-test.log` 为当前完整套件；其他成功编译/首轮绿测输出待审查结束按 LOGGING 收尾，仅长期保留必要脱敏结论。未保存源码快照、全量用户日志或设备身份。

#### 同晚用户纠正：长按语音中断后 VSCode 调试提示

- 用户进一步明确，新一次是“长按语音键时，录音断了，然后就一直弹窗”，并给出 VSCode 的“没有用于调试 Markdown 的扩展”提示。因此此现象不能继续称为 Menu 原生上下文菜单；此前真实 Menu 的 `configured=false` 证据仍成立，属于另一条链。未读取 VSCode 的私有配置、窗口内容或内部状态，未向用户前台注入测试键。
- 就近旧 App 64252 的实际语音链：15:02:10.260Z 开始 generation 17；15:02:38.670Z `shared_pcm_queue` 溢出，`pending_samples=31774 incoming_samples=240 limit_samples=32000`；38.686 `wasapi_pipeline_failed`，38.787 正常释放语音和弦，38.841 `voice_flow terminal=abort`。该会话 28.732s、1897 回调、455280 解码样本，最大 callback gap 198ms / worker 队列延迟 149ms；音频统计 accepted455040 / submitted423684 / queued31356，最后非零提交年龄 0ms、累计 zero-available 1149 次/9211ms。它不是“WASAPI 从未输出”或仅模板失配。用户没有逐动作关联 ID，以上是与新反馈一致的就近日志实例，不强行认定每次弹窗的唯一时间对应。
- 原生语音键配对缺口：停止/中止会撤销 session 武装，250ms 宽限过后，旧 `decide` 忽略已接管的物理 hold，仍按住时的重复 DOWN 开始透传。39.185Z 观察到 DOWN `swallow=0 hold=2`；随后宿主连续报告 `requested=1 actual=0`，直到 44.413Z UP `swallow=0 hold=2`。宿主持续放行已泄 DOWN 的重复和 UP 是正确配对，不能通过强吞 UP 或中途接管已泄按住修复。最小修改 `key_suppressor.rs`：全吞 DOWN 的当前 hold 锁存到真实 UP；已泄 hold 保持原生，未知首沿仍由 session/grace 判定。没有延长语音生命周期、增加长按阈值或重建设备宿主。
- Voice 回归实际先红后绿：新增“abort+grace 已结束但物理仍按住”的重复/真实 UP/下一次无武装按压，以及“已泄 hold 不被新 session/armed 中途接管”两项，旧逻辑 2 failed，候选 6 passed（包含既有配对测试）。Helper 日志隔离不能单独解释或宣称修复该录音中止。
- 音频受控验证只用自家保存的同一个 CABLE 端点与自有 WASAPI session。开始前确认旧 App 最近语音和弦已释放且超过 10 秒，拒绝已静音端点；发送纯零值 PCM，不录音、不改配置、不操作第三方程序。输入节奏为 240 样本/15ms、45 秒共 720000 样本，采用绝对截止时刻，避免 Windows sleep 粒度让生产端自动降速。无构建负载基线实际全部提交/排空，45.08 秒 passed，设置文件前后 SHA 相同，未复现固定格式/持续速率不匹配。
- 在必需的真实 release 构建负载期间，同节奏重跑于 44.759 秒实际溢出：accepted715200 / submitted683400 / queued31800。新终态诊断读回 buffer418 帧（16k 格式下 26.125ms）、最大 pump gap99ms、超 buffer 时长的 gap64 次/累计 excess1152ms、空 render buffer 观察73次；空间查询/写入最大 0ms，会话静音检查最大98ms。耗时含线程调度，不能单凭 98ms 区分 COM 等待与被剥夺 CPU。失败后真实 interrupt1ms、worker正常退出，配置不变。
- 最小生产改动使用现有 windows API 在渲染 worker 注册官方 MMCSS `Audio` 任务，不改采样率、重采样、32000 样本队列或5ms轮询，不改系统全局设置。RAII 不可 Send/Sync，正常/异常离开作用域在同线程撤销；失败记录 HRESULT 并保留原功能路径。相同本机/端点/45秒输入，在另一轮必需 release 构建负载下实际 passed：45.308秒后720000样本全部提交并排空，最大pump gap20ms、late-pump0/excess0、静音检查最大1ms；注册与撤销均 passed。停止请求前有4702样本排队，后续已实际排空，不能把该中间快照误写成丢样本。两轮编译阶段/CPU强度不保证完全相同，此对照支持本机候选效果，不证明所有负载/长时语音或双型号稳定性。
- 音频诊断保持最小：open 读回 mix rate（本机48000）、实际 buffer frames/period；pump 只更新内存计数，在正常 finish 与失败终态输出。`empty_render_buffer_count` 是观察次数，不等同断音次数；`late_pump_excess_ms` 不是直接丢帧数。最大耗时包含失败调用，WASAPI 写入计时在失败日志之前结束。普通 interrupt/Shutdown 不额外宣称有该汇总。
- 证据：`voice-hold-red.log` / `voice-hold-green.log`、基线 `paced-audio-test.log` / `paced-audio-diagnostic.log`、无 MMCSS 的负载 `paced-audio-loaded-before-*`、MMCSS 候选 `paced-audio-loaded-*`。前后负载实验在本机保留作为当前未完成硬件回归的必要对照，后续原位结论充分且不再需要复核时按 LOGGING 回收。未保存源码副本、真实语音或端点身份。最终源码仍 dirty，计时修正后的最终检查及部署已完成，VSCode 实体 Menu/长语音仍 deferred；不将整项标为通过。

#### 最终本地候选与安装读回（2026-10-15 23:41–23:43，UTC+08:00）

- 源码/审查范围为 `hardware/RC003/helper/src/main.rs`、`crates/sayall-windows/src/key_suppressor.rs`、`audio.rs`。最终 SHA-256 分别为 `17a5dde1a4fc440a3d456d8060594a5e9a01abdebc56d4a9b2a4d3d44ab0be40`、`13cb8f5853bb5c747aff746ad7f04a8b28deff32f114c8c5ab1b39c7a3399a7c`、`afdf28271df37e9055110194705953d8c014a3f7bcccfbbb4c43a32cc47332e5`。独立审查已闭合受控 sink 建立时序与失败调用计时问题，无未处理的必修项；未提交、推送或发布。
- **passed（最终自动化）**：Helper 独立 workspace `cargo test` 61 passed / 3 ignored；最终 `cargo fmt --all -- --check`、`cargo test --workspace`（core32、Windows541/21ignored、Host160/1ignored，其他库/契约通过）、`cargo check --workspace`、`cargo check -p sayall-windows-app --features runtime-simulation`，以及受影响文件 `git diff --check` 均通过。Helper 使用同一精确源码的既有证据，没有为收尾重复执行。45秒 CABLE 基线与负载前后实测按上述各自范围记实；不能扩大为遥控器/转写通过。
- **passed（本地包）**：唯一当前安装器为 `target/release/bundle/nsis/无线麦_1.0.0_x64-setup.exe`，SHA-256 `ea99d929d74112698f52218f36fe096a3ce256f7e30d72d094b06c8d2aa91f0d`；实际个人 updater 公钥验签、fresh 前端嵌入内容门禁及 production simulation 隔离通过。本轮为 x64 四项载荷（Helper、Gadget、两许可）；ARM64 增强未覆盖。基线 `8b9dd31460f1b3b4c6859b2216de61ede0966206`，`sourceCommit=null`、dirty/local/included，源码/实际载荷指纹 `6ac22ec4cf4c22228a306a690f848dee8aef33014bf798991f673280d022e304`；不以基线代表完整未提交源码。
- **passed（安装/启动）**：先确认当前语音已结束，再以真实桌面同账户管理员覆盖；安装器退出0/12.495秒，旧 App64252、子75624、Helper78564均正常退出，没有强杀。四项实际载荷/Helper PE 机型与本轮 staging 对齐；主 EXE 全文件比较仅 `UNK→NSS` 三字节安装标记差。五份现有配置安装前、安装后及新启动后哈希完全一致。安装后首次控制器在子 PowerShell 验证调用处报告 RemoteException；随后只读地用显式 StagedDirectory 继续验证并通过，没有重复安装，未据该验证器异常判为安装失败。
- **passed（实际新实例）**：真实 Explorer6432 的桌面 `Document.Application.ShellExecute` 启动 App79676，精确安装路径匹配、TokenElevation0；Helper76260精确安装路径/TokenElevation1。实际日志读回的五个构建身份字段与上述最终 manifest 一致，Helper 新 `[LOG-QUEUE] started capacity=256`、MMCSS 注册及捕获 ownership ready 均已观测。至23:43:24，累计137秒（含闲置）没有 ownership timeout、bridge silence timeout、日志队列 overflow 或音频队列失败；此窗口 Menu fired0、voice attempt0，因此只证明启动/空闲保持，不证明实体行为。
- **deferred（用户实体/兼容性）**：已请用户在 VSCode 短按 Menu 选择确认/连续开关，再按住语音超过45秒并松开，核对录音连续、释放及调试弹窗是否再现；截至上述观察尚无相应新 PID 动作或用户回复。本轮 RC001、RC003 完整冷首用/连续/断连/睡眠矩阵、ARM64及第三方实际转写仍未通过；睡眠/无线电实验许可仍未恢复。原 `stale_window` 返回失败与未对齐的其他前台配置问题保持独立未知。
- **收尾**：保留唯一当前包及当前红测、最终套件、安装/运行读回和负载前后必要对照，均在同一任务目录；未保存源码快照。过时首绿、重复成功检查和中间包构建输出在核对无正式引用后按回收站规则处理。后续用户动作可在当前安装版继续核查，不自动重启/重装或扩展发布。

#### 用户实体反馈与日志复核（2026-10-16，UTC+08:00）

- **passed（用户报告）**：用户回复既定 VSCode 复验请求：“菜单和语音都正常，没有调试弹窗”。请求包含菜单选择确认、连续开关及超过45秒语音，但回复没有逐项次数或持续时长，因此只记录实际反馈，不据此补写这些细节。未读取 VSCode 私有数据或注入按键。
- **身份/取证范围**：仍为 App79676、Helper76260，启动时间均为10月15日23:41，主程序日志指纹仍为 `6ac22ec4cf4c22228a306a690f848dee8aef33014bf798991f673280d022e304`。本次只读核对安装后默认诊断日志，统计固定截止10月16日09:48:27.194（01:48:27.194Z）；不混入旧 App64252 的失败。长期证据以本节脱敏摘要为准，未复制完整日志或语音。
- **passed（可观察语音链）**：四段 `voice_flow terminal=control_stop` 分别在09:45:30.439、09:45:52.135、09:47:08.285、09:47:54.891结束，持续33.895、7.092、9.509、18.433秒；对应音频 `normal_finish` 接收/实际提交样本分别为540000、111600、149760、293040，全部 `queued_samples=0`。四次语音和弦释放均 `result=ok reason=released`；2033次原生语音键 DOWN（含自动重复）及4次UP均 `swallow=1 hold=1`，没有 `requested=1 actual=0` 或 ASK_TIMEOUT。四次 pump 最大间隔20–21ms、late计数0；没有 queue_overflow、wasapi_pipeline_failed 或 voice abort。日志支持这些会话正常结束与排空，第三方实际体验以用户报告为准；实体超过45秒仍 `deferred`，此前独立45秒静音负载对照保持其原 `passed` 范围。
- **菜单成功与独立残留并存**：09:46:43–55有7条 `Menu configured=true fired_count=1`，generation22/26实测 `visible=true`、`foreground_verified result=true`，支持模板面板能被唤醒；用户报告菜单正常。但09:46:44.410及09:46:55.101再次出现 `stale_window`，09:46:45.167出现 `return_target_unavailable`，随后关闭记录 `blocked reason=target_restore_failed`；09:46:51与58另有完成隐藏记录。这是已知前台返回/关闭边界，不能把连续开关或每次确认全部记为日志通过，也不能改称捕获未唤醒复发。
- **捕获超时并未完全消失**：新实例在09:38:35.365及09:48:24.699各记录一次 `ownership_timeout`。从有效mask归零到恢复7167分别为09:38:35.363→36.138（775ms）、09:48:24.696→24.752（56ms）；若从超时日志行计则为773ms、53ms。安装后至固定截止没有 bridge `silence_timeout` 或 Helper `[LOG-QUEUE] result=overflow`。两次窗口（09:38:33–39、09:48:22–27）没有 Menu 边沿/手势记录，无法与用户某次动作对应；不能声称所有续约故障已消失，也不据后台短暂事件自行扩大修复。
- **验收边界/收尾**：本次用户体验通过，以上日志成功与失败分别保留；整体TODO仍未完成。RC001/RC003完整冷首用、连续、断连、睡眠、超过45秒实体语音、ARM64及第三方转写逐项矩阵不由本反馈替代，`stale_window`仍独立未解决。只更新原Bug、TODO及当前能力入口，不改生产源码/ATTRIBUTION、不重建或重启/重装，不提交或发布。
