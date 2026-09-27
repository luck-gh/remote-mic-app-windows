# 2026-09-15 原选择音频端点恢复候选

## 现场与边界

- 本轮报障包含三键无法使用与语音未启动，分别调查。三键宿主归属候选已 failed 且未接入生产；本修复不改变该事实，实体键盘共享 VK 修复也未完成，RC001 无本轮硬件验收。
- 运行中安装版 0.2.5，PID 75452/session 1，EXE SHA256 `e8375887be4f003fbaacc64eccd8206861866a5be86c4b89c38ab278a4dd4cf7`；本轮只读核对响应与安装路径。15:55:35 重新读取 tokenElevated=false（preinstall-runtime.json）；本次新包仍须另行核验。
- 10:19:20 generation10 开始，10:19:39.178（Asia/Shanghai）内部 worker PCM 队列溢出：queued31810、submitted259550。不是消息 channel 满，也不是 worker 退出；只能证明消费落后，不能断言线程阻塞。
- `fail_audio` 丢弃 sink，原保存 ID/name 仍存在。之后直至15:16用户语音 DOWN 到达，但 begin 立即报 endpoint_not_selected。根因是已选端点资源不可用却无重建路径。
- 配置仅核对哈希/匿名角色，未修改配置、映射、默认音频设备，不记录语音。

## 候选与验证

- 保留原 ID/name，仅下一新会话重建；重新枚举精确匹配且 open 后核 name。仍存在但失效的 sink 在同一新 begin 最多退休/重建一次，无阈值、无重播旧会话。
- 阻塞 WASAPI 准备在快捷键 DOWN 前；每连接 release epoch 与全局 lifecycle epoch 在源 STOP/控制回调错误/断开/睡眠/退出排队前失效。DOWN 前后检查，取消成对释放；正常 STOP 仍 finish/drain。取消 reset 失败也清资源及 generation。
- 增加 failure_origin、accepted/submitted/queued、elapsed、last_nonzero_submit_age、zero_available count/elapsed。未改缓冲限制。
- 定向测试：audio 原12 passed、2 ignored；新增取消清理分支1 passed（cancel筛选2 passed含重复1）；BLE 原5 passed+控制回调错误1 passed；power1 passed。日志分别 `audio-guard-tests.log`、`cancel-cleanup-tests.log`、`control-error-test.log`、`ble-tests.log`、`power-tests.log`。均 exit0；源码身份见 `review-hashes.json`。
- 15:48:19 隔离测试进程 PID14832 在原保存端点主动送入超限静音向量，实际触发 worker_pcm_queue。清理后第一个新 generation21 重建26ms、实际提交1600样本且 padding 排空；generation22 同样1600/排空。端点/会话均 already_ok、未主动切静音，配置 hash 不变。`wasapi-recovery-smoke-test.log` 1 passed/exit0；结果与真实元数据见 `wasapi-recovery-smoke-result.json`、`wasapi-recovery-smoke.log`。
- 上述真实 WASAPI 消费不等同遥控器捕获、非静音回环或第三方文字上屏。真实设备移除/睡眠、RC003 物理语音冷/闲置首用、安装后连续会话待验。

- 独立只读审查最终4/4源码哈希一致，无剩余 P0/P1；见 `review-result.json`。审查不代替实际运行。`cargo fmt --all -- --check` exit0。

## 交付状态

- 16:00:55 本地包构建 exit0（release 应用3m01s；完整 `build.log`）。包 `无线麦 SayAll_0.2.5_x64-setup.exe` SHA256 `fbc792f8fd356870508bbba1362a495fa4b96c6997a7f4b8deb8ae35c89bfe03`，原始 app SHA256 `30fe4a53fd5e69377d0cac98378142b87c44132df3d602c227bae4e91567e698`，见 `build-evidence.json`。
- 最终 Helper PE 已实核 DependentLoadFlags=0x800，无动态 VCRUNTIME/MSVCP 导入；见 `helper-pe-evidence.json`。
- 一次限定托盘退出尝试无法确认 popup，尚未调用 Quit，脚本 exit1，日志 `normal-exit.log`。当时覆盖安装 blocked，保留旧实例，没有运行安装器。随后用户明确回复已退出，16:10:08 系统级查询相关实例0，解除该阻塞。
- 用户语音快捷键专门配置不存在，依据现有默认为左 Ctrl+左 Win，适配微信输入法默认听写；不等同于第三方已启动录音。
- 已准备 `target/install-audio-recovery-candidate.ps1`、`target/verify-audio-recovery-install.ps1`，待退出后管理员双实例门禁、覆盖原目录、Explorer普通启动、NSIS UNK→NSS限定差异及资源/三份配置状态核验。不重建已冻结包。
- 安装后 RC003 按住3–5秒再松开使用持续诊断日志，不设短deadline；实际语音、闲置首用、长会话及第三方结果尚未验收。
- 未安装/加载任何新驱动，未执行宿主注入、修改安全配置、强杀、Git 写入或发布。三键与实体键盘共享VK问题未完成。


### 16:11–16:12 实际覆盖安装

- 冻结包哈希匹配，启动前双重零实例门禁，管理员覆盖原目录；安装器 exit0。真实 Explorer dispatch 启动后唯一 PID71260/session1，tokenElevated=false、Explorer同用户、窗口响应，核验命令 exit0。
- 已安装 EXE SHA256 `0edbe8c5daf7697b09cb1bf9213a5b87dff781e1df5dc9b4cc0d0603b03c747c`，与冻结原始载荷等长，仅 offset13509554 起 UNK→NSS 三字节；Helper/INF/SYS/CAT逐文件哈希匹配。
- settings、button-mappings 哈希安装前后及启动后完全相同，voice-hold-hotkey 文件仍 absent。详见 `delivery-status.json`、`installed-runtime-evidence.json`。
- 持续诊断16:11:44原保存virtual_cable restore passed（82ms）；16:11:45 RC003 connection Ready。16:14 首次读取本新PID尚无物理语音会话；已请用户按住3–5秒再松开，采用持续日志无短deadline。此处不把用户尚未回复记failed。

### 随后用户实测：短会话恢复，长会话 failed

- 用户已实际操作并反馈“麦克风现在正常，但长录音约20s自动断掉”，不再等待上述短会话回执。generation4约3秒，45600样本实际提交并排空；这是短录音恢复证据，不外推第三方识别文字、闲置或睡眠验收。
- generation5于16:34:13.614收到Control04；begin耗时1ms，13.752快捷键DOWN，13.767首次提交418帧。16:34:32.635先发生`worker_pcm_queue`失败：elapsed19017ms、accepted297120、submitted265274、queued31846，last_nonzero_submit_age15ms。32.742快捷键UP、32.745 interrupt，32.769才收到Control STOP。故此次中断由音频队列先失败触发，并非先释放或断连；对应聚合见`physical-voice-observation.json`。
- 10:19旧版已有同类队列溢出；16:12恢复补丁解决永久丢sink，没有改变稳态消费调度。因此不能把本次新增日志误当新增稳态算法的回归，也不能宣称已消除溢出诱因。

### 隔离持续静音与调度诊断（未打包，均非真机语音通过）

- 原端点16kHz单声道16bit输入，mix48kHz双声道，default_period100000hns（10ms），实际buffer418输入帧，首次写418帧，clock频率32000。库的available/padding/write均以初始化格式帧计，accepted−submitted=queued成立。
- 前三次每秒查询端点属性/clock的实验对渲染线程有干扰（`steady-baseline-*`、`steady-timing-*`、`steady-cached-*`），不作为干净因果基线。
- 去掉Streaming期间Probe调用后的Polling基线`steady-clean-*`仍failed：12.930s，accepted201804/submitted172331/queued29473；下一批将越32000限制。无BLE参与，表明隔离渲染路径也可复现。
- 相同格式/周期/PCM限额的EventsShared实验`steady-event-*` failed于34.358s；`steady-stage-*`失败时5ms事件等待实测最长338037us，而WASAPI写入最长189us、available179us、command195us。等待累计时长包含正常并行播放，不能全部归为阻塞。
- 加入线程级MMCSS Audio也未解决（`steady-mmcss-*`）。同一测试binary经真实Explorer普通用户启动，`steady-explorer-*`仍failed于稳态20.290s：accepted_delta318587、submitted_delta291073、初始queue1750；事件等待最长360195us，超过设备周期的累计超额2465210us。mute watch累计37021us、最长1377us，不支持其为主要差额来源。该runner因PowerShell原生stderr早退未生成result JSON，不能伪报runner退出码；测试日志明确Failed且MMCSS已正常revert。
- 不含音频的Explorer子进程有界等待对照`wait-control-{start,result}.json`：普通session1、MMCSS注册/revert成功；10秒内WaitForSingleObject(5ms)最大225861us、2次超过50ms。所查询Job层CPU flags=0/rate=0，未发现该层CPU控制；不能据此排除嵌套Job或其他系统调度因素，也不能将in_job=true归因为限流。
- 当前只能证明渲染线程/普通等待存在足以耗尽10ms有效余量的长迟到，不能指定是哪一系统组件造成。EventsShared/MMCSS当前源码属于未通过候选，旧4文件审查不覆盖这些增量；未构建或安装新产品包，没有改系统优先级、全局设置、音频格式或PCM队列上限。
- 增加设备buffer必须同时有足够有效预填充lead才可抵抗已测约350ms停顿；这会增加音频延迟。正常STOP源码先释放快捷键再drain，额外lead可能使尾音在第三方停止收音后才输出。不得默认延后UP或把更大buffer当成无行为改变；此权衡尚未形成可交付修复。


### 进一步反证与回归线索（17:29–17:36）

- RTWQ仅测试原生薄适配使用现有Windows API及AgileReference，Audio shared queue上的serial waiting callback真正执行refill；30ms预填/418帧容量/16k格式/32k队列不变。静态源码/EXE/runner三hash核验准入一次隔离。`rtwq-explorer-result.json` exit101：约15s即队列失败，1273次回调、最长gap422403us、producer最长429501us，不是回调未执行。barrier、Stop、队列解锁、Shutdown均成功；Cancel返回失败码-1072875819，不误记为已取消。此实验也不支持RTWQ直接解决当前故障。
- 随后唯一500ms有效lead实验仅cfg(test)，设备请求600ms、实际capacity9600、首write8172（约511ms）；原PCM32k未改。`lead500-explorer-result.json` exit101，配置不变：23.743s accepted376140/submitted346433/queued29707后下一批溢出，maxpumpgap384842us、producermax387926us。初始有效lead大于实测最大gap仍失败，故“仅小buffer无法承受普通线程长停顿”的因果解释不充分。没有进行更大buffer或延迟产品试验，未改UP、未安装实验版。
- 当前未安装实验源已完整保存在`lead500-experimental-source.zip`（SHA256 e6880e5cbf4b19572073b3646226f0c33bf9df4dbd64083b15847d6e6415c03b），仅用于复现失败，不是交付包。历史测试EXE路径已被后续增量构建复用，身份以当次runner核验及记录的SHA为准，不把旧manifest视为当前路径的持续冻结证明。
- 用户新增明确反馈：今天安装恢复包之前，同一电脑可以连续录超过一分钟。这是本次回归的重要线索，所有调度/lead试验停止，优先比较两份实际生产源；不以10:19单次旧overflow或系统等待迟到否定该反馈。
- 留存日志最长成功会话为9/11 PID43676/gen15：begin02:46:37.931Z→finish02:47:37.887Z，59.956s、957360样本、排空passed（`historical-long-session.json`）。当前保留日志没有严格大于60s且>=960000样本的成功条目；日志覆盖限制不等于用户未成功。
- 精确旧audio源已从只读Git保留blob43c319e36816157c02a58dbb826136fb1caeea80提取，57039bytes/SHA256 7fdac64f87bd1697fb2d4c003a0a9a6e118130d9433401eaa0b5f8f585987f95，与9/14构建manifest匹配；`production-source-audit/old-production-audio.rs`为内容副本。HEAD/index并不匹配，不能冒充旧生产源。新25EE已装源也已从 blob6b2e8ee92d0ab764625fd4bdb9f69602d78f0152 精确提取；工作区 audio.rs 已恢复为该冻结内容，失败实验未进入产品。旧/新BLE源也按SHA提取，完整内容、diff及harness保存于 `production-source-audit/README.md`。


## 精确生产源码与实际 capture 拓扑对照（2026-09-15 18:05–18:25）

- 用户“今天安装前同电脑可连续超过一分钟”作为真实回归线索保留；精确 old7FDA/new25EE audio 和 old65D1/newB531 BLE 内容已恢复保存，不再用 HEAD 猜旧生产源。audio steady 参数与路径相同，新增计数/Instant；BLE持续批次相同，启动准备/取消顺序变化。静态一致性不替代生产验收。
- 已执行优化版同harness两次有界对照：write-only old56.014s/new35.004s均failed；明确CABLE Output consumer old30.008s/new35.005s均failed。后者ContainerId两端非空相等，capture native48k/2ch/32bit，真实取得帧并正常Stop。不能把捕获侧discontinuities下的帧时长差直接当clock漂移；不能把异步fed当accepted。完整数字、拓扑、sourcehash、runner exit1/config一致见 `production-source-audit/README.md`。没有新修复包，没有Git提交。
- 18:05只读idle三角色默认Capture全非CABLEOutput，但WeType在两端均inactive，不能推断真实hold时私选。18:25启动普通Explorer/session1只读持续观察器，PID12484；生产仍PID71260。只枚举公开capture session分类变化，不读音频、写默认或配置。用户真实长按验收等待新会话与回执，尚未判结果。
- 旧包固定0d437来源链：09:25安装后e837载荷核验→09:27PID75452→15:55同PID/路径/session/响应；15:55未重新hash，明确该证据边界。旧包对照/回切仅准备，未安装，须正常退出和双进程门禁。先检查当前新包真实WeType capture路由，再决定是否需要版本对照。


## 2026-09-16 后续真机结论

新、旧固定包各两次真实持续按住均约60s先收到remote`00 02`，续期持续，随后正常UP/drain，无此次queue overflow；用户明确未先松开。旧包其中一轮公开CoreAudio确认WeType实际active于CABLE Output。不能仅从该表现宣称官方硬件60s上限或EXTEND一定被固件采纳。四轮完整时间线和独立20s溢出边界见 [sixty-second-20260916/evidence.md](sixty-second-20260916/evidence.md)。已正常恢复固定fbc新包，PID30468普通Explorer/RC003Ready/配置与载荷passed；只读observer正常exit0且无残留。新增终态诊断3tests/fmt/reviewpassed未安装，BUG未关闭、未Git提交。
