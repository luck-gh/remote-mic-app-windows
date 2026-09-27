# 遥控器语音会话临时输入设备：2026-09-16

状态：本地候选已于 2026-09-16 17:15 安装并完成普通权限启动/载荷/配置核验；新功能默认关闭。真实 setter 与 RC003 功能验收仍待用户操作。

用户明确“允许受限使用，并接受外部改选时让出控制”。默认关闭，目标 capture 由用户选取（VB-CABLE 为 CABLE Output，既有 render 仍 CABLE Input）。语音 DOWN 准备期间确认 Capture 三角色后再发目标快捷键；BLE 通知音频此时只排队。源释放取消启动，正式恢复只能在 BLE owner 已发快捷键 UP 后进行，不改既有正常 drain/UP 时序。设备设置更改在 active 期间拒绝并提示松开后更改。

任一可观察外部角色改变使本次整体让出、全部当前角色保留。正常结束只恢复本次改过且原设备仍 active/name 匹配的角色。每次 setter 前持久化 potential-change journal；崩溃后只提供显式恢复/保留。IPolicyConfig 未文档化、无 CAS/取消契约；读写间极窄竞争、同值用户操作无法完全判定。第三方自行指定输入设备不保证跟随系统默认。

## 软件验证

- `cargo test -p sayall-windows -p sayall-windows-app --lib capture_ -- --nocapture`：12 个路由/取消/门禁用例与 3 个宿主事务/退出用例 passed，exit 0。覆盖冻结前状态机；其后仅追加 generation 日志、重复 START 拒接管和错误分类，最终编译由本地包验证；对应日志增量6个native定向测试也已passed（`frozen-native-tests.log`）。
- `pnpm exec vitest run src/pages/ConnectionPage.test.ts`：5 passed，exit 0。
- `cargo check -p sayall-windows-app`：passed，exit 0。
- `pnpm build`：passed，exit 0。
- 精确候选源保存在 `source/`，14 文件 SHA 在 `source-manifest.json`。最终发现并修正崩溃恢复时未改 role 外部变化反例，新增 1 项定向测试 passed（`crash-recovery-test.log`）。11:33:34+08 冻结 14 文件经独立复核一致，无剩余 P0/P1；最终 ed35/2ae7 源对应 release/NSIS 构建 passed，exit 0，2 分 11 秒（`build-final.log`），构建后 14/14 源哈希一致。中间 09b60 包仅保存在 `pre-crash-fix`，不可安装。

最终待安装包 `无线麦 SayAll_0.2.5_x64-setup.exe`：SHA256 `9a7048a376d97b7197ff339a7322b4d6337de01d6efe1a65288f27a445d96f07`，4,983,725 字节。原始 app SHA256 `d6d83c0c931b1c76cb5f73df8403c13e55f7eae95b8edbf6f2b88a2894bf4a34`。完整载荷见 `build-evidence.json`；安装后须验证唯一允许的 Tauri `UNK`→`NSS` 标识改动。安装脚本 `target/install-capture-input.ps1` SHA256 `4c93e411e7b6348e71074f99d3f11da7f9765dd710bb811d6be9fde027fd85e8`，读取最终包 pin，安装前两次确认应用与安装器零实例。

## 安装 passed，真机验证待执行

用户已明确正常退出。Get-Process 与授权 CIM 查询双重确认 app/installer 为零，安装脚本再次门禁后覆盖既有 `D:\Program Files\无线麦 SayAll`，installer exit 0。第一次沙箱 CIM 查询被拒，未将其缺失结果作为零实例证据；随后授权只读 CIM 查询成功。无强杀、无驱动安装、无默认设备切换。

17:15:39 运行核验：新 PID 39548、session 1、Explorer 用户身份一致、tokenElevated=false、单实例且窗口响应正常。安装 app SHA256 `dd4d916bae44ca5afb53980799a8f47db06115ff3783f623968bfbabe932eda5`，与原始候选逐字节比较只在偏移 13774290 处出现 3 字节 `UNK`→`NSS`；Helper/INF/SYS/CAT 完全匹配。settings/button-mappings/voice-hold-hotkey 的存在状态及哈希在安装与启动后保持一致；私有备份仅本地保留。见 `install.log`、`verify.log`、`delivery-status.json`、`installed-runtime-evidence.json`。

新 PID 启动日志确认原 persisted virtual_cable render 恢复 passed，17:15:20 RC003 ready=true；见 `installed-startup.log`。`post-install-default-state.json` 确认新选项关闭、未由本任务修改配置、SayAllInput 服务数量 0。17:16:24 启动只读观察器（普通用户 launcher 38204/session 1），基线 Console/Multimedia=CABLE Output、Communications=headset，WeType capture inactive。观察器 30 分钟自动上限、支持正常 stop marker；该 idle 快照不能证明真实录音路径。记录在 `capture-route-watch-start.json` / `capture-route-watch.log`。

安装后用户在“连接与语音”读取输入设备，明确选择 CABLE Output，再开启“遥控器说话时临时锁定麦克风输入”。只在真实语音按下时执行 setter。先记录三角色基线；首个 3–5 秒按住/释放联结路由准备、hotkey DOWN/UP、真实 ATVV/WASAPI 提交排空、WeType active CABLE Output 与用户听写反馈。持续只读观察器复用已验 SHA 15b4f50a…12e7323，普通 Explorer 启动，30 分钟自然上限/同目录 stop marker 正常退出，音频内容不读取不保存。

随后同一候选验证快速释放、连续、闲置首用、disabled、耳机/外部手选让出、断连/睡眠/正常退出和崩溃可见恢复。RC001 缺硬件 deferred，不外推 RC003。当前全部真实路由和目标收音项 pending。

三键捕获与键盘共存仍未完成；20 秒内部 PCM 溢出仍独立 unresolved；60 秒设备停止按用户要求暂停研究。既有诊断代码包含于候选源但不作为上述问题修复。无 Git 提交/推送/发布。

## 2026-09-22 正常释放后原端点短暂不可用，恢复门禁未重新对账

本段为当日新故障，前文历史验收范围不被覆盖。已安装576b5aeb候选、App10384（17:29:58）/Helper51572原实例；用户明确只有语音键失效，其他按键不重验。22:10–22:24同run前7次正常语音有ATVV包及capture restore passed；第8次22:27:39.204 prepare default_roles_confirmed，实际15.839秒/1047包，22:27:54.974 control_stop。22:27:55.102恢复在原精确端点active枚举校验返回capture_endpoint_missing，未进入任何恢复setter；journal generation8保留changed三角色、expected全target、transition无。22:29:59起新Voice请求明确到达，但prepare=recovery_required后2–8ms abort、0音频包。因此不是单凭UI未高亮推断按键没到，也不是本轮安装正常退出残留；故障发生前该时段没有配置保存事件，不据此推断所有历史原因。

22:44:04一次与产品相同MMDevice公开只读核对，限定journal原/目标精确endpoint ID与三角色：original_mask=7、target_mask=0、expected_mask=0，原与目标均state=1 active；journal前后hash未变。当前实际已回到原设备，但旧应用recovery gate只接受target/expected，无法认出已完整还原。未读取声音、输出标识或设置默认设备；没有关闭输入锁定或直接删除journal。

最小修复：complete_if_already_restored必须原三角色ID全部精确相等、原端点active且名字仍与原记录一致、无未决transition，并在落盘前重读角色、检查请求有效；成功只完成journal，不调用setter。finish失败在当前进程持有具体事务，现有通知/下一次Begin/正常退出仅在磁盘仍等于该事务时检查此零写完成条件；成功后的同一次Voice DOWN继续正常准备，不消耗初始化按键。启动时加载journal不取得这个所有权，未知/崩溃遗留保持显式恢复/保留；显式恢复也支持已还原零写分支。混合/外部角色、原设备不可用/改名、未决transition、读写失败均不自动清记录，不移除既有外部改选保护。recovery_required不再覆盖更具体原失败reason。

`cargo test -p sayall-windows capture_input --lib` 23项passed：本次新增原端点消失→回来且三角色已还原零setter、仍target/混合/外部值不清、原不active/读失败/落盘失败/取消/改名保留、未决transition拒绝；既有按下释放与组事务用例复用。本次未模拟设备变更、无线电或电源，也未据此把真实语音恢复标passed。旧应用公开UIA根不可访问，一次只读检查后停止；主控请求用户通过现有“保留当前选择”产品入口确认现状（零setter），等待完成后核日志；未擅自触发。组合包与安装身份归同日hid-gatt-access evidence，临时日志voice-recovery-tests与voice-recovery-menu-release保留到修复验收关闭。

用户提及未开启应用时语音键会刷新网页：既有BLE/key_suppressor源码明确该遥控器语音还上报原生F5，未运行SayAll时无其抑制，与浏览器刷新行为相符；本次日志缺该次刷新逐F5边沿吞放，不能将具体每次刷新精确归因。此轮不扩F5抑制、HID来源或Helper用途。

23:01用户回复“完成，点击后好像好了”。一次日志核对：23:01:53.250旧产品recovery_choice=passed，该操作无setter；journal正常写为空。其后23:02:18及22起用户已有两次真实语音（generation32/33，1.694s/102包与2.340s/146包），prepare默认角色确认分别59/57ms，chord_press/chord_release均ok，音频实际started，UP后原三角色均original_mask7、restore passed（66/81ms）。输入锁定保持true。这是用户处理后旧现场语音恢复的直接证据，不宣称新自动零写分支或整矩阵passed。

23:05:18通过应用托盘真实退出，capture_route passed、overall failed_stages0/146ms，journal为空；Helper正常cleanup/unload/detach/terminal code0。39e5b0fb候选随后安装成功；新App59576（23:07:20）普通、Helper42676（23:07:22）提权。新进程空journal、原锁定启用、初始化无错误，BLE/input_context ready与虚拟音频端点restore passed；idle按正常空记录初始化契约确认，未另用UIA读取状态快照。23:08:42.289后新包语音一段及Menu偏好组待用户，不自动按键/预热，也不重现端点消失。最新配置/安装身份详见同日hid-gatt-access证据。
