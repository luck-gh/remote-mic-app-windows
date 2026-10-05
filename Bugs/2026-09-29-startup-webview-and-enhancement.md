# 启动后白屏与增强使用初始空绑定

- 发现日期：2026-09-29。
- 状态：最小修订已安装，自动化与本轮正常启动/增强来源就绪通过；原白屏完整责任链、真实设备晚到启动首用与用户动作/布局验收待定。
- 影响范围：Windows 0.2.6；本机 RC003 增强。两项失败相互独立，不能以 BLE 连接或主进程存活判定就绪。
- 正常预期：界面失败有独立于 WebView 的明确反馈；增强等待同一监听器当前有效绑定，不把初始空路径永久传给 Helper。

## A：白屏证据与边界

当前实例 14:54:04 的 document load、Vue mount、首次 IPC 已通过；14:54:27 本应用 Crashpad dump 记录 0xC0000005，异常地址位于 RTSSHooks64.dll + 0x1490b6。9/27 23:00:20 的旧 dump 同模块同偏移。仅读取标准 dump 头、异常流、模块条目，不读取内存内容；原始 dump 留在本机，不入仓库。普通目录枚举遗漏隐藏文件，因此不能用最初“未看到文件”认定无崩溃。

这证明启动成功后发生 WebView 进程异常，不证明该 DLL 的完整责任链，也不授权修改第三方软件设置。当前实现缺少 ProcessFailed 分类和恢复入口。补复用 Tauri 已锁定 webview2-com 0.38.2：每窗口固定类别/返回码日志；仅 RenderProcessExited 一次 Reload（提交成功不代表显示恢复），BrowserProcessExited、持续失败或不可恢复状态使用原生窗口/托盘及一次原生对话提示正常退出重开。正常退出/销毁不再恢复，不无限重载，不影响基础语音线程，不强杀。无主动崩溃注入；实际原故障未再次复现前不得记 resolved。

## B：已证实的空绑定快照

9/28 17:47、9/29 11:28、11:41、14:54 四实例均在 RawInput 初始无设备时创建增强工作线程。线程复制空 selected_path 后不再更新；后来设备到达只更新 ListenerContext，因此自动请求和两次手动重试仍发空接口。Helper 日志均 target_count=1、process_open_failed=0，peer_verified 后 activation_rejected code=0 并正常 terminal=1；不是未启动/UAC取消，也尚未进入模块签名或逐报告 PDO 校验。

修订让工作线程引用同一监听器持有的当前绑定。空/不在位时保留排队请求而不提权；已确认绑定替换后正常结束旧会话，清理确认后才用新绑定。相同接口的短暂移除/恢复仍归既有 Helper PnP 生命周期，不能把该改动变成每次断连重建且丢首按。新监听器拥有独立状态，旧代消息不进入新映射。原描述符/逐报告来源/原生抑制保护不变；补早退阶段与 CM/Win32 返回码，不记录接口身份。

## 验证

- 空绑定保留自动/手动请求：旧实现真实红，最小实现后绿；当前绑定首次到达/同接口移除恢复/目标替换/独立监听器测试通过。
- WebView renderer 一次恢复、浏览器退出、重复失败、退出时迟到：先红后绿。
- 原生控制器 10 项与清理终态 3 项通过，空接口拒绝原因断言通过；不附加真实宿主。
- 完整矩阵：Rust workspace 356 passed / 17 ignored，fmt/check 与 runtime-simulation check exit 0；前端179项与 build passed。纠正为内嵌资源后，真实 Windows WebView/IPC 14步 passed、验证实例自行 exit 0。最终包与本轮新实例观察见下。不得因新实例正常展示就称原始崩溃根因解决，不执行重启/蓝牙/睡眠实验。
- 隐私：无设备身份、个人路径、窗口标题、语音或第三方正文。

## 本轮验证载体错误（与原故障分开）

15:29 的仓库 debug/runtime-simulation 实例沿 Tauri devUrl 访问 localhost:2430，但未运行 Vite，因此出现 ERR_CONNECTION_REFUSED，用户截图证实。此实例不是 D 安装版，也不是 14:54 的原始 WebView 崩溃；由开发验证入口选择错误造成。该实例已通过产品正常退出事件结束，未强杀、未改网络设置。纠正复用 CI 命令 `VITE_SAYALL_RUNTIME_SIMULATION=1 pnpm tauri build --no-bundle --features runtime-simulation` 的内嵌资源载体；仿真包不安装，最终生产包重新构建并验证仿真隔离。

## 本地安装与当前真实边界

2026-09-29 15:39:53 生产0.2.6包 SHA256 `5ead8a6dd463c8789db29fd9a1d1473bc2de2bbdc9bec2cdc999e7577bd878ed`，生产仿真隔离检查通过，管理员原目录安装exit0。Explorer普通启动App29528（15:44:50，TokenElevation0）与限定Helper12144（15:44:52，TokenElevation1）；安装Helper与构建精确同哈希，主程序除既有NSIS bundle marker UNK→NSS三字节外相同。三份用户配置在退出前、安装后与启动后哈希保持，未改模板/关联/偏好。

同一新App文档加载、Vue及首次IPC成功，观察超过90秒未见新增ProcessFailed或Crashpad（仍仅原两份），这不是原崩溃根因resolved或永久健康证明。自动增强已通过旧activation_rejected位置；mask31回执成功，15:46:20真实selected_instance/request报告ready/all_up=true、physical0、raw_released=true。期间用户自然语音产生报告，开发方没有按键预热；故可证明当前真实来源/释放就绪，不能冒称新开机最先增强首按或设备晚到场景通过。

正式动作使用既有Codex→preset-agent：返回Backspace、音量±系统音量；不增初始化键或写测试映射。真实删字/音量/键盘并用及两页滚动对齐、底栏视觉等待用户反馈。不开机重启、不切蓝牙、不诱发崩溃。
