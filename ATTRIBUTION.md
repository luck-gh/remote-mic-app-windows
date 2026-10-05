# 来源与归属

本仓库是面向 Windows 的 Rust/Tauri 工程。

## 统一上游全按键支持（2026-10-03）

- 固定来源为 [GetSayAll/remote-mic-app-windows `4d4de099`](https://github.com/GetSayAll/remote-mic-app-windows/tree/4d4de099f823cc67f334ff2a498e98c2806e4455)，采用 `hardware/RC003/helper` 的 Rust Helper、内嵌 agent、固定 Gadget 构建输入，以及 `rc003_bridge.rs`、`rc003_task.rs` 和授权交互。项目与来源均按已有 GPL-3.0-only 归属保留；Frida 固定 17.18.0 的许可和下载散列随资源锁定及安装包提供。
- 本地整合保留模板/程序关联和 Menu 控制；捕获集合来自实际有效模板与菜单，删除五键专用 Helper、驱动生产链及无来源 LL 兜底。没有映射的键只观察，禁用动作不回放原生按键。基础 ATVV 与音频设备锁定保持独立。上游可选报告层语音合成固定关闭，保留本地设备准备成功后成对 SendInput 的现有语音路径；固定延时不能替代本地设备准备的完成回执。
- 根据 [Frida Thread API](https://frida.re/docs/javascript-api/#thread)，`Thread.sleep()` 接收秒：原 agent 将毫秒直接传入的等待已改为换算秒，并以真实 API 单位更新测试。该修复不调整既定 150 ms 门限，不能算作冷态延迟优化实测。
- 删除强杀共享宿主以更新 Gadget 的恢复路径；旧/不匹配 agent 明确拒绝。关闭时通过实际 agent `stopped` 回执确认撤钩与合成 UP，持键释放未到时保留当前会话等待，不把进程退出码当成功。本地协议将回执绑定宿主 PID/OS 创建时刻、Agent `instance` 与本次 `stop_id`；同实例重新确认可消除其旧未确认状态，另一实例或空启动取消不得代替，旧协议活体的部署限制见 [前台回归记录](Bugs/2026-10-03-foreground-input-regression.md)。清理结果以同一 Helper PID/启动代次写入本机原子回执；Gadget DLL 仍驻留，不能宣称模块卸载。来源选择及逐报告设备归属仍需本机共存验收，不把上游历史实测外推为本地双型号通过。
- 本节按用户新方向替代下文历史同步“不采用计划任务、Gadget 与报告层合成”的实现选择；历史来源与证据保留。当前部署和验证结果见 [既有输入证据](artifacts/hid-gatt-access-20260919/evidence.md)。

### 前台回归调查的参考边界（2026-10-03）

- 2026-10-04 未部署的脚本重载实验曾参考 [Frida Gadget Script](https://frida.re/docs/gadget/#script)，在自有进程中观察到两次不同实例重载。复审发现共享脚本跨宿主、重载握手期限及失败观测证据仍有缺口；用户正常重启后，同一安装版已恢复菜单和增强键，因此撤回实验，不作为当前产品能力。根因和恢复证据归同主题 Bug；保留原 ignore 模式的真实部署边界，不把实验扩大解释为真机更新通过。

- 2026-10-04 异常终止恢复采用 Microsoft [进程终止语义](https://learn.microsoft.com/en-us/windows/win32/procthread/terminating-a-process)与 [WaitForSingleObject](https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-waitforsingleobject)：强制终止不会执行进程清理代码，必须由外部拥有者或下次启动核实；持有的进程对象进入 signaled 才能证明该实例终止。恢复回执持久化 Host PID 与 GetProcessTimes 创建时刻，避免 PID 复用；同实例仍存活则需当前协议的新停止确认。未根据进程名、时间流逝或文件消失推断已撤钩，不终止共享 WUDFHost。Windows 自有子进程与 TCP 实测只证明协议/身份边界，不替代双型号实体按键验收。

- 2026-10-04 一次性恢复审查采用 [Frida Gadget Script 契约](https://frida.re/docs/gadget/#script)：`on_change=ignore` 默认只加载一次脚本；固定 [17.18.0 ScriptRunner](https://github.com/frida/frida-core/blob/17.18.0/lib/gadget/gadget.vala#L876-L918) 持有单个脚本，不能将磁盘更新当作运行实例更新。结合唯一模块、首次加载与完整 HELLO 记录、实际 TCP 两端 PID、同一进程对象及新鲜停止回执，已完成一次性正常停止；工具仅留本机取证，不进入生产兼容层。Microsoft [UMDF pooling](https://learn.microsoft.com/en-us/windows-hardware/drivers/wdf/using-device-pooling-in-umdf-drivers) 允许多设备共享宿主，本机只读确认 7 个活动栈；[PnPUtil 单实例重启](https://learn.microsoft.com/en-us/windows-hardware/drivers/devtest/pnputil-command-syntax) 不能据返回成功保证共享宿主卸载，本轮未执行。恢复与实际验证范围见上述 Bug 记录。

- follow-app 退出判据使用 Microsoft [GetNamedPipeServerProcessId](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-getnamedpipeserverprocessid) 将实际管道服务端与描述文件 PID 核对，并持有进程句柄与创建时刻；[进程退出契约](https://learn.microsoft.com/en-us/windows/win32/procthread/terminating-a-process) 规定进程对象在退出后变为 signaled，[WaitForSingleObject](https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-waitforsingleobject) 的零超时查询用于读回该事实。20 秒仍为首次身份绑定期限及失联诊断阈值；存活 App 的桥接迟到不再等于退出，显式关闭和真实退出仍走同一 Agent 清理回执。Windows 自有管道/自有子进程已验证服务端 PID、持有句柄、正常退出后的 signaled 与实例不替换；不能据此宣称 RC001/RC003 或旧驻留 Agent 的部署恢复通过。

- 现场与前一安装版对照归 [Bug 记录](Bugs/2026-10-03-foreground-input-regression.md)。原语音路径在准备后集中投递时受到 `sync_channel(32)` 的消息条数限制；Rust [sync_channel](https://doc.rust-lang.org/std/sync/mpsc/fn.sync_channel.html) 官方契约确认其容量按消息计、控制消息保持 FIFO。修复复用本项目既有 32000 样本上限与标准库通道，不增加依赖、不增大音频容量；唤醒通知可合并，但生命周期控制不能被丢弃。
- Microsoft [IAudioClient::Start](https://learn.microsoft.com/en-us/windows/win32/api/audioclient/nf-audioclient-iaudioclient-start) 要求渲染流先填充数据再启动，[IAudioRenderClient](https://learn.microsoft.com/en-us/windows/win32/api/audioclient/nn-audioclient-iaudiorenderclient) 规定接口的释放线程边界。继续由原 WASAPI worker 负责设备对象、写入、启动与清理，设备调用不移到 BLE 回调；分段日志用于定位慢调用，不改变快捷键或门限时序。官方 API 成功与单元测试都不证明 RC001/RC003 或第三方输入法真机通过。
- Microsoft [SetForegroundWindow 的异步激活说明](https://devblogs.microsoft.com/oldnewthing/20161118-00/?p=94745)：跨输入队列调用后立即查询前台，目标可能尚未处理激活消息。菜单返回自身主窗口由宿主 UI 线程直接完成，避免 UI 等待后台、后台恢复又需要 UI 泵消息；不使用 AttachThreadInput 或固定 sleep 掩盖。外部窗口路径维持原契约，并记录错误分类，实际返回效果以窗口读回为准。

## 系统任务切换（2026-09-27）

- Microsoft Windows keyboard shortcuts：Ctrl+Alt+Tab 显示应用缩略图并允许箭头选择，不要求持续按住 Alt；Win+Tab 为任务视图。https://support.microsoft.com/en-us/windows/keyboard-shortcuts-in-windows-dcc61a57-8ff0-cffe-9796-cb9706c75eec 。任务视图方向/Enter参考 https://support.microsoft.com/en-us/accessibility/windows/navigate-and-explore-the-windows-taskbar 。CtrlAltTab的确认/Escape不能仅由参考表外推；再次TV取消曾实测failed并按用户要求停止排查。2026-09-28改为TV各手势遵从用户配置，不再把物理TV硬编码取消。
- `smzht/fakeymacs`，固定提交 `f83de826fc7a2ac76a0d5f91f2b54a3f0a50d1e4`，`config.py`（版本20260823_01）的 `is_task_switching_window`：参考其公开进程+窗口类识别方式。https://github.com/smzht/fakeymacs/blob/f83de826fc7a2ac76a0d5f91f2b54a3f0a50d1e4/config.py#L2756-L2764 。本实现不复制其输入框/标题/控件检查，仅取较窄的 MultitaskingViewFrame、TaskSwitcherWnd 类，另以 GetShellWindow 的真实进程所有者及当前 HWND 校验；广义 CoreWindow/InputSite 类未纳入，未知 Windows 实现失败关闭。没有依赖或第三方界面扫描。
- 注入复用现有 Windows SendInput 成对批次；当前物理修饰键按住时拒绝任务快捷键，不通过补发任意 UP 清键、不持续拥有 Alt。公开输入返回成功仅是提交证据，系统界面和 RC003 行为待本机验收。

- 本次官方核对未找到直接开关 CtrlAltTab/WinTab 界面的受支持非按键 API：[GetAltTabInfo](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getalttabinfoa)只读取切换窗口信息；[IVirtualDesktopManager](https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nn-shobjidl_core-ivirtualdesktopmanager)管理窗口的桌面归属而非打开任务界面；[UIA获取元素](https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-obtainingelements)依赖查找控件，本产品不采用。此为本轮有限核对结果，不宣称所有API不存在。
- 23:15真实日志确认同Shell ForegroundStaging导致原控制器误清；该类只作为当前启动代的未确认阶段，不授予导航/注入资格。保留最终严格窗口校验及10秒未确认失败终态；没有引入新接口或第三方内容扫描。真实取消结果仍待本机。

## 鼠标动作扩展

- 鼠标单击/双击参考 AutoHotkey v2 Click 的成对按下/释放行为，不复制其代码或引入依赖；通过 Windows SendInput 单批发送 2/4 个边沿，部分提交时补发释放，不新设双击等待常量。参考： https://www.autohotkey.com/docs/v2/lib/Click.htm 。
- 鼠标移动使用 Microsoft GetPhysicalCursorPos / SetPhysicalCursorPos；本机 150% 缩放实测发现 DPI-unaware 调用的 37 单位会变成约 56 物理像素，因此为该调用显式设置线程级 PER_MONITOR_AWARE_V2，并用 RAII 恢复原线程上下文。修正后右/左 37、下/上 53 物理像素均通过。参考： https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setthreaddpiawarenesscontext 。
- 滚轮动作参考 AutoHotkey v2 的 WheelUp/WheelDown 动作粒度，仅参考行为，不复制实现或依赖 AutoHotkey。来源：`https://github.com/AutoHotkey/AutoHotkeyDocs/blob/v2/docs/lib/Send.htm`。
- 滚轮使用 Microsoft 公开 SendInput / MOUSEINPUT API：INPUT_MOUSE + MOUSEEVENTF_WHEEL，mouseData 是带符号的滚轮位移；一个刻度为 WHEEL_DELTA（120）。来源：`https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-mouseinput`。动作是用户可选配置，不绑定固定遥控器按键、不修改默认配置。测试和首按边界见 `Testing/WindowsMouseActions.md`。

## Windows 注册应用扩展

- 应用发现使用 Microsoft AppsFolder / IShellItem / BHID_EnumItems，启动使用 ShellExecuteExW + SEE_MASK_NOASYNC；只读取系统公开注册的可启动项，不扫描第三方私有文件或修改 Windows 注册。按本机缓存的 Microsoft windows-rs 0.62.2 API 签名核对实现；没有复制外部算法。参考： https://learn.microsoft.com/en-us/windows/win32/shell/knownfolderid 、https://learn.microsoft.com/en-us/windows/win32/api/shellapi/ns-shellapi-shellexecuteinfow 。
- 应用库仅保存在用户确认后的按键配置中；扫描不是启动，多选添加不是绑定。日志只记录数量、阶段和耗时，不记录应用身份或个人路径。验收方法见 `Testing/WindowsRegisteredApps.md`。
- 前台切换依据 Microsoft `SetForegroundWindow` / `GetForegroundWindow` /
  `LockSetForegroundWindow` / `AttachThreadInput` 公共 API 文档：Windows 即使满足常规
  条件仍可拒绝后台进程抢前台，并改为闪烁任务栏；`AttachThreadInput` 只共享输入状态，
  不承诺绕过 foreground lock；用户按 Alt 会解除该锁。本仓库因此以
  `GetForegroundWindow` 的目标窗口读回作为成功判据，常规尝试读回失败后才用成对
  Alt DOWN/UP 包住一次重试，物理 Alt 已按住时跳过，避免破坏用户键态。官方依据：
  https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setforegroundwindow 、
  https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getforegroundwindow 、
  https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-locksetforegroundwindow 、
  https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-attachthreadinput 。

2026-10-02 随上游吸收的窗口筛选使用公开 `GetClassNameW`、`GetWindowRect` 和 `DwmGetWindowAttribute(DWMWA_CLOAKED)`，在身份匹配后排除辅助、未布局和隐藏窗口；`GetWindowTextLengthW` 仅判断特定 Chromium 窗口是否已有标题，不读取或记录标题内容。来源：[GetWindowRect](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getwindowrect)、[DWMWA_CLOAKED](https://learn.microsoft.com/en-us/windows/win32/api/dwmapi/ne-dwmapi-dwmwindowattribute)、[GetWindowTextLengthW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getwindowtextlengthw)。该兼容筛选来自上游现场，不证明当前本机所有应用都可激活。

## 遥控器电量显示

- 2026-10-02 吸收上游标准 GATT Battery Service `0x180F` / Battery Level `0x2A19` 订阅与读取，使用既有 ATVV GATT 会话及公开 WinRT API，不另开 BLE 会话。来源为 Bluetooth SIG Battery Service 标准和上述固定 Windows 上游；其 RC003 通知实验只作为来源，不作为本地候选实测。
- Microsoft 公开 Configuration Manager API `CM_Get_Device_ID_List_SizeW` / `CM_Get_Device_ID_ListW` / `CM_Locate_DevNodeW` / `CM_Get_DevNode_PropertyW`：只枚举当前存在的 BTHLE 设备，按连接所选对端的完整地址组件匹配唯一节点，读取 OS 设备属性。官方文档：`https://learn.microsoft.com/windows/win32/api/cfgmgr32/nf-cfgmgr32-cm_get_devnode_propertyw`。标准 `System.Devices.BatteryLife` / PKEY_Devices_BatteryLife 的 GUID/PID/type 由本机 Windows SDK 10.0.22621.0 `propkey.h` 核对。
- `Gronsten/razer-tray`，提交 `8e7e395417023bf2446779a4c5237716183da69f`，`src/DeviceMonitor.cpp`：参考其使用公开 Configuration Manager API 读取 Windows Bluetooth 电量缓存属性 `{104EA319-6EE2-4701-BD47-8DDBF425BBE5} 2` 的路径和未知值语义；未复制代码、无运行时依赖。该键不是微软承诺跨版本稳定的标准 BatteryLife 属性，故仅作可失败的兼容读取，严格检查 BYTE、长度为 1、0..100；缺失/异常保持未知。
- GATT 通知可用时使用该通知；订阅失败或无 BAS 时保留每 60 秒读取系统缓存的路径，不代表遥控器每 60 秒上报新电量。不访问注册表、第三方 App 数据或设备管理写入 API。连接纪元隔离迟到结果，断连/睡眠后停止订阅与监视并隐藏旧值；可选电量功能不影响语音错误状态。详见 `Testing/WindowsBattery.md`，本轮候选本机验收另记。

## 治理规范迁移

- `HD838A/remote-mic-app`，提交 `b233a88cc4457b00413dda6b37ec8b4af12c5121`：迁移其平台无关的分支/提交纪律、日志脱敏与完整链路记录、Bug 复现取证顺序、测试手册要求、发布来源可追溯和资产不可变原则；本仓库将其改写为 Windows/RC001/RC003、Tauri/NSIS、updater minisign 与 Authenticode 边界。
- 有意排除：Swift/SwiftPM、CoreBluetooth、AppKit/SwiftUI、Developer ID/Apple 公证、Sparkle、DMG/PKG、Apple Team ID、macOS/iOS/Web 专属流程，以及任何 macOS 私有路径或凭据。
- 迁移文档：`LOGGING.md`、`RELEASING.md`、`TECHNICAL.md`、`TROUBLESHOOTING.md`、`Bugs/README.md` 与 `Testing/WindowsRelease*.md`。这些文件记录的是规范与经验，不复制参考仓库业务代码。

## App Logo 版权

- App Logo 与 App Icon 沿用 `HD838A/remote-mic-app` 的版权边界：属于 HD838A 保留版权的专有品牌资产，不纳入 GPL-3.0-only；Windows 版适用范围和授权条件见 [LOGO-LICENSE.md](LOGO-LICENSE.md)。

## 产品与 UI 基准

- `HD838A/remote-mic-app`：无线麦 macOS 原版的信息架构、产品文案、RC003 图片、RC001/RC003 型号识别、ATVV 行为和测试边界；RC001 支持参考提交 `b233a88cc4457b00413dda6b37ec8b4af12c5121`。
  - 2026-09-05 按键映射功能移植补充（均为语义移植，非代码复制）：`RemoteButtonGestureRecognizer` + `HIDRemoteScheduler` 的手势参数（双击窗口 300ms、长按 550ms、连发起始 350ms、返回 50ms/方向与音量 100ms 连发）与"按配置动态启用双击/长按识别、未配置时单击零延迟"的语义；`KeyboardEventSuppressor` 的预测式武装 + 有限窗口匹配吞键模型；`RemoteMappingCanvas` 的按键卡片布局表（锚点/目标 Y 坐标逐键移植）与三态高亮（按下=橙、选中=强调、普通=中性）；`MappingSelectionPolicy` 的"锁定当前按键"默认值。Mac 版 `KeyboardEventSuppressor` 的 UP 沿无配对兜底（DOWN 泄漏+UP 吞下=粘键缺陷）未移植——Windows 版沿用本仓库 2026-09-05 规则（DOWN 漏进 OS 则 UP 必放行）。
  - 2026-09-14 Windows 图形与拖动边界：按键图形从实际渲染图片内容框统一计算热点与连线，不继续复制旧固定像素。程序关联的 HTML5 拖动使用 Tauri 2 主窗口 `dragDropEnabled=false` 的公开配置让 WebView2 页面接收事件；overlay 保持原值，仓库未发现原生文件拖入消费者。隔离浏览器真实 pointer 与 CDP 截获的原生 drag data 验证见 [模板图形与拖动证据](Testing/StructuredTemplatesAndDrivers.md#历史模板验证证据)；该证据不替代安装版 WebView2 的用户实际拖动。
  - 2026-09-09 按键映射配置导入导出补充（本地 Mac 仓库 HEAD `feba1d6`，语义参考，未复制代码）：参考 `AppSettings.exportedConfigurationData/importConfiguration` 的版本化 JSON、导入前完整解码校验与一次性应用，以及 `SettingsView.exportConfiguration/importConfiguration` 的系统文件选择器、用户取消静默、成功/失败反馈。Windows 版仅迁移按键映射，不导入 Mac 专属设置或统计；格式使用独立 `formatVersion: 1` + `buttonMappings` 契约，不宣称与 Mac 配置文件互通。
- RC003 图片 SHA-256：`658d9333853958c13ff721eb76e1a6816c1dbea16006a84e8577ad410812549f`。

## Windows 行为与测试参考

- 2026-09-20 来源恢复竞态依据微软 [CM_Get_Device_ID_ListW](https://learn.microsoft.com/en-us/windows/win32/api/cfgmgr32/nf-cfgmgr32-cm_get_device_id_listw) 的字符长度/当前设备列表契约及本地官方 cfgmgr32.h 的 `CR_BUFFER_SMALL=0x1a`。18:30 实机恢复时 Size→List 之间列表增长，不能把失败枚举解释为空的权威设备树。生产 Helper 独立实现重新取长度与仅该暂态的延后重试，成功仍须精确、唯一、当前节点和 PDO 二次校验；未知期间零接管，停止/宿主退出取消重试。未复制外部实现或扩展来源接受面，软件测试不替代本轮恢复实机。

- 2026-09-20 限定来源B使用微软 [WdfDeviceQueryProperty](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/wdfdevice/nf-wdfdevice-wdfdevicequeryproperty)、[PDOName](https://learn.microsoft.com/en-us/windows-hardware/drivers/install/devpkey-device-pdoname)、[CM_Get_DevNode_Registry_PropertyW](https://learn.microsoft.com/en-us/windows/win32/api/cfgmgr32/nf-cfgmgr32-cm_get_devnode_registry_propertyw) 与 [Bluetooth LE设备接口](https://learn.microsoft.com/en-us/windows/win32/api/_bltooth/) 的公开契约，独立实现当前节点唯一反查。固定UMDF2.15槽31及枚举11，未把KMDF-only IoTarget接口套入UMDF；A所需本设备接口注册未获静态依据，未猜GUID。实机8次精确功能节点/665次其他节点后，生产 `native/hid-host-helper/main.c` / `runtime.js` 只采用精确功能PDO与每代次对象绑定，未采用未实测父节点分支；复用既有固定Frida，不增加依赖或启动下载。来源与未完成的抑制验收分列[原证据](artifacts/hid-gatt-access-20260919/evidence.md#wdf-pdo-source-b)。

- 2026-09-20 WDF 引用生命周期修正依据微软 [EvtCleanupCallback](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/wdfobject/nc-wdfobject-evt_wdf_object_context_cleanup)、[WdfObjectDereferenceActual](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/wdfobject/nf-wdfobject-wdfobjectdereferenceactual) 和 [EvtDeviceReleaseHardware](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/wdfdevice/nc-wdfdevice-evt_wdf_device_release_hardware) 合同。固定驱动 C928/C93E 将 CE60 注册到 PnP callbacks +48（ReleaseHardware），DeviceCreate 的 Cleanup 为 NULL；仅使用现有固定回调入口释放本 Helper 已证明来源并持有的引用，不注册残留回调、不复制框架源码。公开 PnP 通知只失效身份，不能当作对象仍可访问的生命周期证明。槽126/127及五参数保持不变；每报告 PDO 来源、对象代次和失效放行继续生效。实机状态归既有 evidence，不以自动化替代重连验收。
- **登录时自动启动（2026-09-14）**：产品行为参考 macOS 仓库
  `LoginItemService.swift` 的“读取系统状态 → 注册/取消 → 失败反馈”模式；Windows
  不移植 `SMAppService`，改用微软公开的当前用户登录启动项
  `HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run`，只写本应用值且不需管理员权限。
  设置默认关闭，应用启动时以持久化偏好同步系统状态，失败只记录结构化日志、不阻断启动。

- **LL 吞键对 Raw Input 交付影响的本机实证（2026-09-05，`docs/investigations/2026-09-05-ll-swallow-vs-raw-input.md`）**：双线程探针（钩子线程 + Raw Input INPUTSINK 线程分离，key_suppressor 同构）两轮一致证实 **WH_KEYBOARD_LL 返回 1 吞掉的键盘事件不会再投递 WM_INPUT**——按键映射门控（`key_gate.rs`）据此采用"被吞键盘边沿由钩子线程直接喂引擎 + 监听器喂 HID 报文与透传键盘事件"双源合并架构；HID 报文归因武装 + 60ms 有界等待沿用 key_suppressor 实证参数。

- `HD838A/remote-mic-app#249`，提交 `090a3cfc24f0e3e733b2347ee2daf87c60e10097`：Windows 独立实现、ATVV 测试夹具、语音边沿、安装升级、公开边界和 Mac 风格 UI 原型；Raw Input 参考了 `hid_identity.py` 与 `raw_input_windows.py`，SendInput 的批量提交、物理修饰键和失败回滚参考了 `win32_input.py` 与 `win32_keys.py`，均以 Rust/windows-rs 重新实现。
- `GetSayAll/hardware-simulation`，提交 `65248499cac7da3ad46cd0c11dca1478f7733255`：RC001 短语音时间线的控制通知、40 + 80 字节音频拆包和停止通知；本仓库只保留纯 ATVV 回放所需字段。
- `ZSTDJan/windows-remote-mic-app`：WinRT BLE、Raw Input、音频输出、发布门禁和真实硬件验证边界；其语音页按语音程序配置"按住说话快捷键"、按下注入 DOWN/松开释放的行为，是本仓库按住说话快捷键设置的产品参考。Round 1 拆解曾记两项技巧参考，后续实证修正（Round 2/3）：**physicalize 技巧——结构性无效（勿模仿）**：`legacy_key_suppressor_windows.py` L142-155 的做法（仅对自家 keybd_event 注入的带 "RMICRC03" 标记右 Alt，在自家钩子的私有副本上清 INJECTED 标志→转发→恢复）曾被解读为"使下游应用钩子视为物理键，前提是自家钩子位于目标应用钩子之前（链头）"——该解读不成立（Round 2 E 三层实证：LL 钩子每钩子收到私有结构副本，修改不跨钩子传播，CallNextHookEx 转发通道不存在，应用层收到原始键；Round 3 J 语义复查：清标志对下游钩子/应用层均不可见，且 ZSTDJan 进程内也无读者——对声明目标是 no-op；其真正能影响豆包读值的是 `doubao_rpc.py` 的 Frida 版 attach 方案，未接线进生产流程，违反本仓库 A2/A4/A5 边界，仅作机理记录）。**WeType 语音触发配方——本机实证有效（Round 3 J 翻案）**：SendInput 注入 Ctrl+Win 按住（纯 wVk 或扫描码配方均可）可唤起 WeType 语音（会话级 TSF 激活前提下：开麦/吞键/释放关麦全链实证，注入 ground truth 由常驻捕获器独立记录）；**Round 2 F 曾判"三配方无反应"，系其 TSF 激活用了线程级 flags（dwFlags=0，会话级应为 TF_IPPMF_FORSESSION=0x20000000）、WeType 从未真正激活所致——教训：测试 IME 行为前必须以会话级激活 + 行为判据（候选框版式）双重确认活动输入法**。**配方形态约束（2026-09-04 P 实证，evidence/p）：和弦必须逐事件注入且两键间隔 ≥80ms——WeType 拒绝单次 SendInput 批量零间隔提交的 Ctrl+Win（sent=2/2 全到达仍无吞键无开麦；逐事件 80ms 两轮 2/2 触发，A 失败→B 通过→A 失败→B 通过交替序列排除状态漂移）**；应用曾把该配方误合并为单批零间隔导致真机不出字（Bugs\2026-09-04-wetype-zero-gap-injection.md，含第二层缺陷：遥控器 F5 须由抑制器吞掉，否则"额外按键"拒绝；钩子链头 bump 加固同日落地），已修复并 RC001 真机端到端 passed（2026-09-04，用户确认文字上屏）。
- `richlearntodo-debug/vibe-flow`，提交 `047f9d3ead54bf30de9b884adf8f7b5adefe9993`：自然 ATVV 会话、WASAPI 音频生命周期和硬件验收清单。
  - **Windows 深色模式专项调研补充（2026-09-08，本地参考库 HEAD `b47f7cdce8b753fade0c64c97332bebe80f17d2d`；主应用 UI 源码未开源，依据为 `docs/ARCHITECTURE.md`、`docs/PRODUCT_AUDIT_2026-09-01_ZH.md` 与用户指南）**：其产品支持浅色、深色、跟随 Windows 三档且运行中切换不重启 Host/Bridge/Capture；审计结论要求深色采用低饱和中性色层级，并完成各页实际截图检查。本仓库只借鉴“三档主题、主题是纯显示行为、不得重启后台服务”和视觉验收边界，不复制实现；SayAll 将选择器放在“关于”页面，并通过自身 `SettingsStore` 持久化，详见 `docs/plan/2026-09-08-windows-dark-mode.md`。
  - **按键映射/双响应专项调研补充（2026-09-07，本地参考库 `Documents\Codex\reference-repos\vibe-flow`，HEAD `b47f7cdce8b753fade0c64c97332bebe80f17d2d`；主应用源码未开源，依据为其文档 + `scripts/VoxDeckInputBridge.cs` + `driver/rc003-filter`）**：
    1. **用户态"拦截↔设备身份互斥"独立复证**：其 V1.3 根因报告（`docs/V1_3_INPUT_ROUTING_ROOT_CAUSE_ZH.md`）实测——LL 钩子拦截 → Windows 不投递对应 WM_INPUT → Raw Input 拿不到 RC003 设备身份 → 动作永不执行（旧候选日志：钩子暂存 166 边沿/真正到达 Raw Input 4/配对 0/实体路由 0）。与本仓库 2026-09-05 `ll-swallow-vs-raw-input` 实证同结论，两库独立互证；本仓库以 GATT 前信号（0x04 早于 HID 60-90ms）做武装归因，不受此陷阱影响，为同类实现中结构更优。
    2. **其用户态发布版（V1.5）的答案=接受共存**：钩子对非语音映射键一律放行（不拦截、也不暂存回放），Raw Input（INPUTSINK + 设备句柄指纹）负责设备归因与动作执行，明确接受"遥控器原始键系统效果与配置动作同时发生"，文档要求"不应在 UI 或发布说明中描述为精确拦截"；默认 Profile 全部映射=该键原生效果（上→上、确认→Enter）使共存不可见，非默认 Profile（如左→browserback、上→Ctrl+Z）实际存在与本仓库同款的"原生+注入"双响应。语音键（RC003 固件形态=F5）是唯一在钩子层无条件按 VK 抑制的键（物理键盘 F5 冲突被接受，或交由驱动路径解决）。
    3. **唯一彻底解=KMDF per-device upper filter（候选未发布）**：INF 精确绑定 VID 0x2717/PID 0x32B8（不做键盘类过滤器，普通键盘零影响），按扫描码位图抑制 + 全边沿环形队列入队上抛用户态；250ms 心跳、2s 超时 fail-open 全放行、策略 generation 变更清队列防陈旧事件、控制句柄关闭即解除抑制。与本项目 ADR 0002/Helper 轨定位同构；其 `driver/rc003-filter/README.md` 的 10 项发布门禁（SDV/HLK/微软签名/Secure Boot+内存完整性/卸载回滚/万包压测）可作 Helper 轨验收清单参考。
    4. **已退役路线警示**：独占 GATT 抢占 HID 服务/强制禁用 HID 子设备 → Windows 将键盘子设备判为 critical，`/force` 禁用成 reboot-pending 状态而非安全热交接——本仓库 GATT 归因为并行订阅（不禁用系统 HID 栈），勿走独占抢占路线。
    5. **互证数据点**：RC003 返回/音量±/电源键在钩子/键盘 Raw Input/Consumer Raw Input 全通道不可见（HID GATT 0x1812 特征 AccessDenied；厂商服务 8a7a0001-… 的 Notify 无按键事件；Frida 旁路 WUDFHost 监听 IOCTL 无捕获）→ 其结论"硬件能力缺失给诊断、不宣称映射成功"，与本仓库 RC003 返回/音量±格子禁用同构；WeType 配方 Ctrl+Win/toggle/80ms、语音键=F5、"重连后扫描码偶变→持久语音映射保持权威"均与本仓库实测一致或互补。
    6. **工程细节参考**：RawKeyboardEdgeTracker（keysDown 集合按扫描码身份 add/remove，防钩子/Raw Input/驱动多源双触发）；长按 650ms、连发起始 420ms/间隔 80ms；TV=Win+Tab 任务视图且"方向键仅任务视图激活期间执行映射动作"（拥抱原生效果而非对抗）；动作执行回执（真实 SendInput 结果而非排队即成功）。
- `mwlt/Voice_VibeCoding`，提交 `c89410aed3b274fee5e571128b82c9c6e6689715`：Rust/Tauri 模块划分、windows-rs API、音频生命周期和托盘窗口工程经验；其语音键按住注入的 Hold 语义（按下先快捷键 DOWN、松手统一释放、SendInput 互斥降级）是本仓库按住说话快捷键注入时序的参考。Round 1 拆解补充其 **LL 钩子吞键工程细节**（本仓库吞键层设计的参考，非逐行复用）：时序窗吞键（音量 recent 200ms、back/home/menu/tv/power 250ms、方向/OK 200ms 或 tap_ready+自定义位图）、钩子链头 bump（重叠安装：先挂新钩再卸旧钩，消除 LL 吞键空窗）、F5 语音键状态机（sticky/correlate 120ms/tail 3s；DOWN 漏进 OS 则 UP 必放行，防粘键）、音量防双格（Tap 转发 + SendInput VK_VOLUME_* + 200ms 吞固件残留）、Alt 和弦用 SendMessageTimeoutW 直发前台避免系统菜单、自家注入放行（EXTRA_INFO 标记或 INJECTED→CallNextHookEx），及 bump 空窗/sticky 粘键/60ms 去抖门等已踩坑清单。本仓库只使用 SendInput 公共 API，不引入其 WinUHid 虚拟键盘驱动。
- `cgutman/WinUHid`（MIT 许可）：用户态 UMDF 虚拟 HID 键盘/鼠标驱动框架（C++/Win32），无预编译 Release，需自建并签名后使用；ADR 0002 增强轨驱动来源的第一候选（须先审计）。签名成本调研结论（**已闭合，2026-09-04**：UMDF 分发不需硬件计划/EV，OV 级 catalog 签名为最低门槛——三层官方原文支撑，`docs\investigations\evidence\g\signing-policy.md`；残余含混=无单句官方原文直书此结论，装机实测 deferred（调查护栏限制））记录于 `docs\investigations\2026-09-04-avoid-driver-signing-input-paths.md`。未经审计的 WinUHid 二进制不进入仓库。
- **2026-09-15 限定三键宿主旁路参考**：依据用户在获知技术边界后继续要求参考新项目的明确指令，当前限定例外归 [PLAN](docs/PLAN.md#当前范围调整三键宿主-hid-旁路2026-09-15-用户确认)。公开来源固定为 `miaomiaozii/windows-remote-mic-app` [`06e617425f8655d9f440466ceb5c470907877313`](https://github.com/miaomiaozii/windows-remote-mic-app/tree/06e617425f8655d9f440466ceb5c470907877313)，`apps/windows/rc003/src/ovb_rc003/{frida_compat.py,frida_hid_tap_runtime.py,frida_hid_tap_injector.py}`。其归属声明为 GPL-3.0-only，源自 `nijez/open-voice-bridge`，tap 另改编 `xxb26553663-star/remote-bridge-hub` `8a93f321ac71a602300c6cd77f7256fa4b63068e`；本项目亦为 GPL-3.0-only。保留作者、修改说明、许可与对应源码义务，不把重写当作免除许可的理由。
  - <a id="three-key-reference-static-review"></a>**2026-09-19 THIRD_PARTY_NOTICES 来源静态核对**：`remote-bridge-hub` 固定提交 `8a93f321ac71a602300c6cd77f7256fa4b63068e` 是直接三键报告源参考。[hid_tap_runtime.py L106–135](https://github.com/xxb26553663-star/remote-bridge-hub/blob/8a93f321ac71a602300c6cd77f7256fa4b63068e/source/bridges/xiaomi/hid_tap_runtime.py#L106) 钩住 NtDeviceIoControlFile，只筛 IOCTL `0x80018483`、同步成功与请求输出长度9；[L286–325](https://github.com/xxb26553663-star/remote-bridge-hub/blob/8a93f321ac71a602300c6cd77f7256fa4b63068e/source/bridges/xiaomi/hid_tap_runtime.py#L286) 按 RC003 硬件 token 枚举并选首个 HostPid，没有逐 file handle 的设备归属证明。[hid_report_tap.py L78–93](https://github.com/xxb26553663-star/remote-bridge-hub/blob/8a93f321ac71a602300c6cd77f7256fa4b63068e/source/bridges/xiaomi/hid_report_tap.py#L78) 解析 `01 00 00` 加三个小端 u16，识别 F1/80/81；[L163–189](https://github.com/xxb26553663-star/remote-bridge-hub/blob/8a93f321ac71a602300c6cd77f7256fa4b63068e/source/bridges/xiaomi/hid_report_tap.py#L163) 用 active set 去重并产生释放边沿。这些源码事实不等于本项目真机成功或严格逐设备来源闭环。
  - 同一固定提交的 [hid_tap_injector.py L226–259](https://github.com/xxb26553663-star/remote-bridge-hub/blob/8a93f321ac71a602300c6cd77f7256fa4b63068e/source/bridges/xiaomi/hid_tap_injector.py#L226) 使用显式 runas Helper，并有 PID/name/hash 校验；这是用户态注入路线，无需 Microsoft 内核驱动签名，但不因此获得逐句柄归属或本机安全软件放行保证。`miaomiaozii` 是上述能力的整合示例，不能仅凭归属声明把每个被列项目都当作独立三键解决方案。
  - `nijez/open-voice-bridge` 本轮仅核对公开 main，未固定提交，不与本地 v30 字节对应：[Windows README Known gaps](https://github.com/nijez/open-voice-bridge/blob/main/apps/windows/rc003/README.md#L573) 与 [frida_compat.py](https://github.com/nijez/open-voice-bridge/blob/main/apps/windows/rc003/src/ovb_rc003/frida_compat.py) 明确注入尚未实现，`BackKeyCompatLayer.start` 总返回 false，RC003 未完成真机验收，不能作为已解决三键的依据。Frida 是机制库，VB-CABLE 负责音频传输，照片归属不是按键方案。
  - 用户续推后补充限定静态核对：上游只读观察不等于精确抑制，匹配失败也不等于三键不可实现。参考微软 [WdfRequestGetIoQueue](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/wdfrequest/nf-wdfrequest-wdfrequestgetioqueue)、[WdfIoQueueGetDevice](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/wdfio/nf-wdfio-wdfioqueuegetdevice)、[统一设备属性](https://learn.microsoft.com/en-us/windows-hardware/drivers/wdf/accessing-the-unified-device-property-model)、[WdfRequestRetrieveOutputBuffer](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/wdfrequest/nf-wdfrequest-wdfrequestretrieveoutputbuffer) 与本地 WDK UMDF 2.15 头，核对固定系统模块的 WDF 分发表和正常 HID read 完成调用。限定静态核对已补齐 formatter `0x1FEBC` 与同步/异步完成分支：产品候选在 WDF Complete/CompleteWithInformation 进入前，按正常 HID read 的 Information 长度取仍有效的输出缓冲区。每次请求以 queue → device → InstanceId 精确匹配所选 1812 实例，再结合公开 HidP 描述符 gate；空 queue、聚合 device、版本/格式不匹配一律不改写。`native/hid-host-helper/{main.c,runtime.js,runtime_security.h}` 实质复用固定 remote-bridge-hub 的 HostPid 发现、F1/80/81 语义及成对 active-set 思路，并复用本仓库已审计 Frida 载荷 ACL 管理；新契约不是其 9 字节私有 IOCTL 布局。普通 Rust 客户端和显式提权原生 Helper 分离，固定脚本内嵌，不下载运行时。Frida core 17.15.3 使用既有锁定 devkit 静态链接，库许可随本地包附带；固定系统文件还按微软公开 [CryptCATAdminAcquireContext2](https://learn.microsoft.com/en-us/windows/win32/api/mscat/nf-mscat-cryptcatadminacquirecontext2) 与 [WINTRUST_CATALOG_INFO](https://learn.microsoft.com/en-us/windows/win32/api/wintrust/ns-wintrust-wintrust_catalog_info) 验证 catalog 成员散列和信任链，不把仅存在 catalog 当作通过；无微软实现复制或系统二进制分发，未新增探针，尚无真机成功；具体绑定、哈希及边界归 [既有证据](artifacts/hid-gatt-access-20260919/evidence.md#static-feasibility-correction)。
  - 2026-09-15 历史版 `Testing/hid_host_probe.js` 独立实现只读同步完成观察和设备对象校验（随后实际 PDO 匹配失败，2026-09-19 已移除该匹配及 payload 读取分支）；协议事实参考上游：`NtDeviceIoControlFile` / `0x80018483`、9 字节 `01 00 00` 加三个 u16、F1/80/81。新增 IO_STATUS_BLOCK 完成长度核对、每次按 NtQueryObject 对 PDO 名匹配、pending 只计数不保存裸指针、有界运行与撤钩回执。没有复用上游裸 TCP、Gadget 装载器、全键 LL 武装或 Defender 排除；Windows 原生三键消费必须另实测。
  - 本地 v30 PyInstaller 包没有嵌入提交且含公开 HEAD 不存在的模块，不能声称其与上述提交字节对应。只读研究结论用于定位，未运行其 exe/脚本/模块、未移植字节码、未复制包内载荷。
  - 2026-09-19 future-open 元数据版独立实现参考微软 [CM_Enumerate_Classes](https://learn.microsoft.com/en-us/windows/win32/api/cfgmgr32/nf-cfgmgr32-cm_enumerate_classes)、[CM_Get_Device_Interface_ListW](https://learn.microsoft.com/en-us/windows/win32/api/cfgmgr32/nf-cfgmgr32-cm_get_device_interface_listw) 及 [DuplicateHandle](https://learn.microsoft.com/en-us/windows/win32/api/handleapi/nf-handleapi-duplicatehandle)：接口类枚举后按选定 devnode 限定公开符号名，仅精确 future-open 与字面 NT 别名；不接受共享对象目标。Close/duplicate 进入即保守失效、打开跨 mutation epoch 拒绝；只匿名元数据，无 IOCTL buffer 读取。11 项 actual JS 合成测试及 C 构建 passed；唯一120秒真实元数据观察未出现future-open，550次既有调用仍未知，正常撤钩/分离后停止该路径，不能作生产输入来源保证。证据 `artifacts/hid-host-future-open-20260919/evidence.md`。
  - Frida core 官方固定 17.15.3 / [`e0ec2bee624b6ea11b72d5cc74639132449789c2`](https://github.com/frida/frida-core/tree/e0ec2bee624b6ea11b72d5cc74639132449789c2)。[COPYING](https://github.com/frida/frida-core/blob/e0ec2bee624b6ea11b72d5cc74639132449789c2/COPYING) 为 wxWindows Library Licence 3.1（LGPL 2-or-later 基础及 object-code exception），不是 MIT；保留完整库许可及对应源码。官方 Windows x86_64 core devkit 53,667,967 字节，GitHub asset digest 与本地 SHA256 均为 `52d4b60d0fb9f9e69f03c652d50d5f3f22c9c967b3d23ff26d33d3c9039bd2d3`，仅用 tar 解包，未执行自解压 exe。`Testing/hid_host_probe.c` 及后续固定生产候选 `native/hid-host-helper/main.c` 使用其公开 C API 生命周期和 devkit example 的参考调用顺序；本仓库实现为 GPL-3.0-only，无 Frida 库源码改动。
  - 2026-09-15 实际 attach 排障采用同一固定提交的 [`frida-helper-process.vala`](https://github.com/frida/frida-core/blob/e0ec2bee624b6ea11b72d5cc74639132449789c2/src/windows/frida-helper-process.vala)、[`frida-helper-backend-glue.c`](https://github.com/frida/frida-core/blob/e0ec2bee624b6ea11b72d5cc74639132449789c2/src/windows/frida-helper-backend-glue.c) 与 [`host-session-service.vala`](https://github.com/frida/frida-core/blob/e0ec2bee624b6ea11b72d5cc74639132449789c2/src/host-session-service.vala)：同架构先用 NORMAL 进程内 backend，异步握手前 worker 退出不触发 ELEVATED 复制路径；worker 在 LoadLibrary 失败时返回 GetLastError。自有探针仅透明观察自身 CreateRemoteThread 导入并查询其线程退出码，实际得到 exited=true/code5，定位为加载阶段拒绝访问，不能写成 Defender 阻断。方法使用微软 [CompareObjectHandles](https://learn.microsoft.com/windows/win32/api/handleapi/nf-handleapi-compareobjecthandles)、[GetExitCodeThread](https://learn.microsoft.com/windows/win32/api/processthreadsapi/nf-processthreadsapi-getexitcodethread)；私有 runtime 权限验证使用 [AuthzInitializeContextFromToken](https://learn.microsoft.com/windows/win32/api/authz/nf-authz-authzinitializecontextfromtoken) 与 [AuthzAccessCheck](https://learn.microsoft.com/windows/win32/api/authz/nf-authz-authzaccesscheck)，无需复制或模拟宿主 token。实际生成文件 ACL 观察、合成自测、失败与后续门禁分列 `artifacts/hid-host-three-key-20260915/evidence.md`，不以库作者行为证明本机三键成功。
  - 官方 [Injected mode](https://frida.re/docs/modes/)、[Gadget cleanup](https://frida.re/docs/gadget/) 和 [Interceptor](https://frida.re/docs/javascript-api/#interceptor)：script unload/session detach 与模块物理卸载分别记录；不手工 FreeLibrary、不强杀宿主。官方来源/编译通过不保证安全软件放行，实际被拦截时停止并报告，保持 Defender/Secure Boot/HVCI 等配置不变。编译/纯解析通过不是 RC003 捕获或 RC001 支持证据。
  - 14:29 用户确认的 RC003 实体序列有首尾确认键 RawInput 阳性对照，但当前宿主捕获候选 failed。随后 6396 次匿名查询均返回合法非匹配对象名称，未读取未归属 payload。微软 [PDOName](https://learn.microsoft.com/windows-hardware/drivers/install/devpkey-device-pdoname) 只定义 PDO 的名字，[UMDF 架构](https://learn.microsoft.com/windows-hardware/drivers/wdf/detailed-view-of-the-umdf-architecture) 另含 reflector 消息通道；不得假定两者是相同 file object。本机选定 devnode 的两个接口经 [QueryDosDevice](https://learn.microsoft.com/windows/win32/api/fileapi/nf-fileapi-querydosdevicew) 解析仍精确指向该 PDO，不能闭合宿主句柄来源。其公开 PnP 父节点是这只蓝牙遥控器，但父接口尚未与宿主实际句柄关联；此事实不授权放宽。
  - 微软 [UMDF pooling](https://learn.microsoft.com/windows-hardware/drivers/wdf/using-device-pooling-in-umdf-drivers) 及 [INF 指令](https://learn.microsoft.com/windows-hardware/drivers/wdf/specifying-wdf-directives-in-inf-files) 允许按设备设置 ProcessSharingDisabled，默认支持池化。本机所选实例实际使用 hidbthle.inf 的 HidBthLE.NT/Wdf、UMDF 2.15.0，无禁止池化指令；未修改系统配置，也未以当前只有一个匹配实例冒充 OS 隔离保证。系统签名有效的 HidOverGatt DLL 只读导入表确认 CreateFileW/DeviceIoControl，不能证明执行实参、已有句柄或冷首用归属。PSS 仅提供名称元数据且没有 File→devnode 公开映射，未新增 PSS/热换脚本/断连实验；有界检查及正常清理见同日 evidence。
- `QL-4/RemoteMapper`：历史参考提交 `25ca0c13cf2ff2caf7caae3d9690f9629b7c0df0`，2026-09-14 重新核查固定 [be8b57330c26a70d8b8ec9ff1e60c23251a2fc31](https://github.com/QL-4/RemoteMapper/tree/be8b57330c26a70d8b8ec9ff1e60c23251a2fc31)。MIT，Copyright 2026 QL-4；其 `driver/MiRemoteHidFilter/driver.c` 的转发 READ/完成后处理报告思路用于本仓库三键 lower filter，许可保存在 `drivers/SayAllInput/LICENSES/RemoteMapper.txt`。上游仅 RC003 精确 revision、单 byte3 假设和开发 TESTSIGNING；包内 WDKTestCert 不是 Microsoft 内核签名。无正式 release/HLK/双型号原始实机证据；此前“八键/HVCI 已通过”是作者说明，不能记作本仓库 passed。上游 F13–F20 改写、普通键和语音改写均不采用，避免实体键盘冲突与延迟语音。其切换默认音频端点和可配置语音手势不符合本仓库边界。
- **结构化模板、场景与可选增强轨（2026-09-10；场景入口当前延期）**：模板采用独立、可命名和可复制的实体；应用仅绑定一个模板，多个应用可复用同一模板，区域语义、菜单与调节模式归模板而非第三方私有数据。该设计基于 Windows 公开 API（Win32 前台窗口能力及 UI Automation 的用户可见辅助功能界面）和本仓库已有公开快捷键边界；未复制外部实现，也未引入未授权二进制。应用适配只走公开 API、公开协议、全局快捷键或可见 UIA，未采用 AxonKey、Frida 或 Interception 作为基础路径或稳定主路径。当前只开放“程序 → 完整按键模板”；场景实现和持久数据保留，恢复条件由 PLAN 约束。
  - **完整按键模板与应用跟随（2026-09-11）**：Logitech Options 的[应用特定设置说明](https://support.logi.com/hc/en-us/articles/360023184154-Configure-Application-Specific-settings-with-Logitech-Options-software)采用“正在运行的应用优先选择，也可手动添加应用”；Razer Synapse 的[应用关联说明](https://mysupport.razer.com/app/answers/detail/a_id/6157/)与[关联 Profile 启用说明](https://mysupport.razer.com/app/answers/detail/a_id/5930/)采用关联应用进入前台时启用对应 Profile。本仓库据此提供运行中可见程序刷新或手动 `.exe/.lnk` 选择，并以当前前台应用选择完整普通按键模板；预设推荐属于历史场景源码，当前入口隐藏。Windows 身份只来自 Microsoft 公共 API：[EnumWindows](https://learn.microsoft.com/windows/win32/api/winuser/nf-winuser-enumwindows)、[IsWindowVisible](https://learn.microsoft.com/windows/win32/api/winuser/nf-winuser-iswindowvisible)、[GetForegroundWindow](https://learn.microsoft.com/windows/win32/api/winuser/nf-winuser-getforegroundwindow)、[GetWindowThreadProcessId](https://learn.microsoft.com/windows/win32/api/winuser/nf-winuser-getwindowthreadprocessid) 与 [QueryFullProcessImageNameW](https://learn.microsoft.com/windows/win32/api/winbase/nf-winbase-queryfullprocessimagenamew)。窗口标题不参与身份或持久化；既有预设保留逻辑 ID，其他程序使用规范化完整可执行文件路径；日志只记录计数、是否命中和终态，不写路径或窗口内容。2026-09-11 本机公开身份核对补充：Windows Codex 可见窗口使用 `ChatGPT.exe` 宿主，但其 VersionInfo ProductName/FileDescription 为 `Codex`，包族为 `OpenAI.Codex_2p2nqsd0c76g0`，可与普通 ChatGPT 包区分；实现只将该公开 Codex 包目录内的宿主归一化到 `codex`，不把所有 `ChatGPT.exe` 合并为同一预设。
  - **用户主动关联与模板弹窗（2026-09-11）**：Microsoft Windows App SDK 的 [Dialog controls](https://learn.microsoft.com/windows/apps/develop/ui/controls/dialogs-and-flyouts/dialogs)要求弹窗提供安全的取消动作、用具体按钮文字表达提交结果，并保持命令区对键盘输入可达；[List/details pattern](https://learn.microsoft.com/windows/apps/develop/ui/controls/list-details)推荐在窄空间使用从列表进入详情的堆叠路径。本仓库据此把程序发现、模板选择与最终确认放入三步弹窗，主页面只显示已添加关联；取消和返回不提交，最后一步才调用持久化，弹窗正文内部滚动且操作区固定。Microsoft 的 [file picker](https://learn.microsoft.com/windows/uwp/files/quickstart-using-file-and-folder-pickers) 继续作为用户主动选择 `.exe/.lnk` 的公开系统界面依据。上述来源只用于交互模式与公开 API 边界，未复制实现；完整按键与场景模板在数据模型中仍是独立对象，当前关联选择器只开放完整按键模板。
  - AxonKey 的“一键安装”产品体验仅启发受控固定动作、前置校验、明确确认与独立 UAC Helper 的交互顺序；不代表可以分发 VB-CABLE 或任何第三方驱动二进制，也不构成其许可证或分发授权。2026-09-14 固定提交 `5547f7b7601ce7767093ccacdf493aad14b806da` / v0.2.29：三键依赖默认关闭的提权 Frida 蓝牙宿主注入，普通十键使用 Interception，README 自述重连内核故障；仅声明 Windows 11 x64 RC003，因此不采用该路线。
  - Microsoft [Windows-driver-samples kbfiltr/rawpdo.c](https://github.com/microsoft/Windows-driver-samples/blob/67d81f217bc01edf7a4320e4911c11065635acfa/input/kbfiltr/sys/rawpdo.c)，固定 `67d81f217bc01edf7a4320e4911c11065635acfa`：参考按设备创建 raw PDO 私有通道的架构，未复制 MS-PL 源码。公共 [HID 架构](https://learn.microsoft.com/windows-hardware/drivers/hid/hid-architecture)说明键盘由 RIM 独占；本机普通 CreateFile READ 被拒绝不能归因于无报告。[WDF 对象执行级别](https://learn.microsoft.com/windows-hardware/drivers/ddi/wdfobject/ns-wdfobject-_wdf_object_attributes)用于将 Control 限为 PASSIVE，以满足 [IoGetRequestorSessionId](https://learn.microsoft.com/windows-hardware/drivers/ddi/ntifs/nf-ntifs-iogetrequestorsessionid) 的 IRQL 条件；父过滤队列默认非电源管理由 [WDF_IO_QUEUE_CONFIG](https://learn.microsoft.com/windows-hardware/drivers/ddi/wdfio/ns-wdfio-_wdf_io_queue_config)约束。
  - 2026-09-14 实证在 `artifacts/three-key-input-20260914/descriptor-contract.log`：HidP 合成 three_word_array_contract=True，six_byte_array_contract=False；19:42:45–19:47:45 被动窗口没有事件且用户未确认按键，因此实际捕获 deferred。驱动 ABI 3 和三槽状态机仅经过合成测试；公开 NuGet WDK 10.0.26100.6584 author/repository/timestamp 校验成功，构建和 INF/CAT 通过不等于 Microsoft 签名。详见 `drivers/SayAllInput/README.md`。
  - 三键维护使用 [WinVerifyTrust DRIVER_ACTION_VERIFY](https://learn.microsoft.com/windows/win32/api/wintrust/nf-wintrust-winverifytrust) 对固定 catalog 的 INF/SYS 成员作内核驱动策略验证，不采用普通 Authenticode 成功或 signer 字符串作为授权。[SetupGetInfDriverStoreLocationW](https://learn.microsoft.com/windows/win32/api/setupapi/nf-setupapi-setupgetinfdriverstorelocationw) 明确不能按源文件内容搜索，因此先从系统 OEM INF 精确匹配编译期内容，再传真实发布路径并验证 Store 包；[DiInstallDriverW](https://learn.microsoft.com/windows/win32/api/newdev/nf-newdev-diinstalldriverw) 与 [DiUninstallDriverW](https://learn.microsoft.com/windows/win32/api/newdev/nf-newdev-diuninstalldriverw) 使用 flags=0 并保留重启返回值。管理员维护请求通过 [SeTokenIsAdmin](https://learn.microsoft.com/windows-hardware/drivers/ddi/ntifs/nf-ntifs-setokenisadmin) 在已限定 PASSIVE 的队列验证请求进程，不给普通用户永久维护占用。源码和拒绝路径验证不代替系统维护实装验收。
  - 当前三键实现采用上项固定 be8b 源码参考；不采用其预编译包、单槽报告假设或测试签名安装步骤。本机公开 HidP 合成验证为 Report 1、121 字节、三个 16 位 usage 位于 1/3/5；不是此前陈述的 modifiers/reserved + byte3 格式。合成报告只能证明描述符契约，不是实际三键数据。
  - Microsoft [签名选项](https://learn.microsoft.com/windows-hardware/drivers/dashboard/driver-signing-offerings)、[内核签名要求](https://learn.microsoft.com/windows-hardware/drivers/dashboard/code-signing-reqs)、[硬件计划注册](https://learn.microsoft.com/windows-hardware/drivers/dashboard/hardware-program-register)、[Attestation](https://learn.microsoft.com/windows-hardware/drivers/dashboard/code-signing-attestation)：KMDF 不能沿用历史 UMDF/OV 结论。正常安全设置需要 Microsoft 内核签名；Attestation 是测试用途，不证明产品认证。组织账号/EV/签名提交未具备，当前源码和 WDK 构建不代表可安装。
  - VB-Audio 的[官网 Pack45](https://vb-audio.com/Cable/)、[donationware 许可](https://vb-audio.com/Services/licensing.htm)及[参考手册第 4–5 页](https://vb-audio.com/Cable/VBCABLE_ReferenceManual.pdf)允许用户自行安装基础包。Helper 不捆绑或镜像第三方二进制：页面固定说明来源、许可、下载验签、UAC 与官方向导边界，用户点击明确的主按钮后才下载固定官方包并校验 hash、Authenticode 签名、版本和架构；仅校验通过才触发系统 UAC 与官方交互安装/卸载向导。没有可靠 Repair 命令，关闭或超时不代表成功，须重新检测。该路径的实际 UAC、安装与音频验收仍未完成。
- `wasapi-rs` 0.24.0：MIT 许可的 Windows Core Audio 安全封装，用于端点枚举、共享模式渲染与 padding 查询。
- **CABLE Input 端点静音自愈（2026-09-07）**：依据 Microsoft Core Audio `IAudioEndpointVolume` / Endpoint Volume Controls 公共 API（`learn.microsoft.com/windows/win32/api/endpointvolume/nn-endpointvolume-iaudioendpointvolume`、`learn.microsoft.com/windows/win32/coreaudio/endpoint-volume-controls`），共享模式端点的主静音属于端点级状态，不是应用 WASAPI 写入成功即可证明可听。本仓库仅对名称确认的 VB-CABLE 渲染端点在打开时及每次语音会话开始前调用 `GetMute` → 必要时 `SetMute(FALSE)` → `GetMute` 读回确认；不修改物理输出设备，也不覆盖用户音量标量。调用结果、检查点和耗时写入结构化 GATT 诊断日志。
- **SayAll 会话静音自愈（2026-09-07）**：用户现场观察到音量合成器左侧 CABLE Input 端点未静音，但右侧“无线麦 SayAll”应用会话在开始推流后很快重新静音。依据 Microsoft `IAudioClient::Initialize` 文档，渲染会话默认会跨应用重启持久化音量与静音状态；依据 `ISimpleAudioVolume::GetMute/SetMute`，应用会话静音独立于端点主静音。实现使用 `IAudioSessionManager2::GetSessionEnumerator` + `IAudioSessionControl2::GetProcessId`，只锁定当前 SayAll 进程在用户已选 CABLE 端点上的会话；初始化、语音会话开始、`IAudioClient::Start` 后读回，并在推流期间每 100ms 低频检查，发现静音才解除，不修改会话音量、不碰系统声音或其他进程。初始化时另以 `IAudioSessionControl2::SetDuckingPreference(TRUE)` 让 SayAll 会话退出 Windows 默认通信自动压低机制；该预防措施不作为外部静音来源已经归因的证据。官方依据：`learn.microsoft.com/windows/win32/api/audioclient/nf-audioclient-iaudioclient-initialize`、`learn.microsoft.com/windows/win32/api/audioclient/nf-audioclient-isimpleaudiovolume-setmute`、`learn.microsoft.com/windows/win32/api/audiopolicy/nf-audiopolicy-iaudiosessionmanager2-getsessionenumerator`、`learn.microsoft.com/windows/win32/api/audiopolicy/nf-audiopolicy-iaudiosessioncontrol2-getprocessid`、`learn.microsoft.com/windows/win32/api/audiopolicy/nf-audiopolicy-iaudiosessioncontrol2-setduckingpreference`。

## 延迟调研来源（2026-09-05，语音键按下→电平图出现优化专项）

按仓库规则（实现前先调研），本专项调研结论与边界记录如下；对应实测见 `docs/investigations/evidence/p/FINDINGS.md`（端点预热对照实验）：

- **业界 PTT"按下→开麦"模式**：可查证的主流实现均为"音频链路常驻 + 按键只做门控"（Mumble 持续采集+传输模式门控 `mumble.info/documentation/user/audio-settings/`；Zoom 会议内按住空格解除静音 `support.zoom.com` KB0063250；Discord PTT Release Delay 滑杆，页面被反爬，引自搜索摘要）。本仓库渲染端点常驻打开（`audio.rs` SelectEndpoint 打开后跨会话复用）与此一致。
- **WASAPI 冷启动与端点电源**：微软 PortCls 文档——音频设备空闲（示例 1s）进入 D3，恢复 D0 规格要求 ≤35ms/≤300ms（`learn.microsoft.com/windows-hardware/design/device-experiences/audio-subsystem-power-management-for-modern-standby-platforms`）；JUCE 论坛实测 WASAPI 设备冷创建 2-3s、Initialize 数百 ms（`forum.juce.com/t/wasapi-2-3s-delays-on-creating-audio-devices/54971`）；StackOverflow `IAudioClient::Start` 通常 5-6ms（被 Cloudflare 拦截，引自摘要）。**"跨进程保温端点让第三方 Initialize 更快"无公开量化先例**——本仓库已用持锁对照实验自行量化：对 WeType 开麦延迟无效（冷/热中位数差 0.3ms，evidence/p，2026-09-05），该方向就此关闭。
- **WeType/微信输入法语音快捷键形态**：默认按住 Ctrl+Win（微信电脑版 4.1.7+ 同款，可于微信"设置→快捷键"自定义；新浪财经/光明网/callmysoft 报道）；社区帖（linux.do/t/topic/2409202，2026-06-15，早于 2.1.3，引自搜索摘要）称 WeType 语音快捷键"必须以 Ctrl/Alt/Shift 开头，不能设独立单键"——**待 2.1.3 真机复核**；ghxi 评论区提到"单击 Ctrl 触发"模式（懒加载未复核）。讯飞输入法 PC 版默认 F6 单键+长按说话（pconline/3DM/ghxi 教程）——竞品基线，未实测其延迟。
- **竞品/社区对"面板出现延迟"的讨论**：未找到任何量化"按下→微信电平图出现"的公开评测（横评均测识别速度/准确率）；游戏侧有 PTT 激活延迟 1s-5s 的社区案例（Overwatch 官方论坛、Valorant Reddit），第三方全局钩子（如 Razer Synapse）可使 PTT 延迟 3-5s——排查本机钩子干扰的依据。
- **本专项实测结论（evidence/p，2026-09-05）**：注入→WeType 开麦（ConsentStore 精确 FILETIME 判据）稳定 ~163ms（13 试验 ±5ms），端点预热无效；两型号遥控器实际均直接 0x04 开始推流（历史 GATT 日志 0x08 计数为 0，无可并行的开麦往返）；0x04 通知早于 HID F5 键盘事件 60-90ms 到达（evidence/p 2026-09-04 取证），当前"0x04 到达即注入"已是链路最早合法触发点。剩余 ~215-245ms = BLE/固件（~30-60ms）+ 和弦间隔（20ms）+ WeType 内部处理（~163ms，外部不可合法压缩）。
- **macOS 版输入目的地/输入源设计（HD838A/remote-mic-app，本机 clone `Documents\Codex\remote-mic-app`）**：`VoiceInputDestinationCoordinator.swift`——语音触发由"聚焦目的地就绪"门控（AX 系统级聚焦快照：role ∈ {AXTextArea/AXTextField/AXComboBox} + enabled + editable + 非保护内容 + 语义文本不含 password/search/设置 等敏感词；不就绪 UI 提示等待/不可用，5s 超时不注入）；`PreferredInputSourceMonitor`——保证配置的语音工具是活动输入源。**Windows 版 IME 专项（2026-09-05）借鉴其输入源职责**：实测 WeType 语音热键仅在自身为会话活动输入法时生效（微软拼音活跃 2/2 不触发、切回 2/2 恢复、激活后零延迟注入 3/3 触发，evidence/p），已实现 `ime.rs`（TSF `ActivateProfile` + `TF_IPPMF_FORSESSION` 会话级激活，公开 API，失败不阻断）。macOS 的 AX 聚焦目的地门控在 Windows 未采用——焦点实验证明 WeType 开麦不依赖文本焦点（6/6，桌面/资源管理器照常触发），聚焦门控留给未来 UIA 版本按需评估。

## BLE 僵死链路自动恢复调研来源（2026-09-05，重连健壮性专项）

场景：应用被强杀（未走正常关闭）后 Windows 侧残留僵死 GATT/HID 链路或服务缓存，普通重试永不恢复（本机真机取证：CCCD 订阅写入 E_ABORT、HID 接口从系统消失；examples\radio_probe 与 examples\gatt_snoop 探针复现）。已实现 `bluetooth_radio.rs` 自动恢复：重连连续失败达阈值时关开蓝牙无线电；每窗口最多 2 次并在 60 秒冷却后重开窗口，避免无限普通重连；应用启动时预取 Radio 对象与权限，避免故障发生后 WinRT 枚举自身也返回 `0x80070008`。2026-09-13 依据微软 `Close` 所有权边界与 MS Q&A 99038 的 service/device 成对释放结论，进一步补齐连接构建中途失败的全量显式清理；此前仅订阅后期分支清理，会让普通重试自身可能累积 WinRT BLE 资源。真机验证：系统栈健康时无线电开关周期后重连循环立即成功（Testing\investigation\sayall-gatt-20260905-live.log T/C 能力交换取证）；预热缓存和失败路径清理版僵死态仍待安装包复验。关键参考：

- **微软官方 GATT 客户端文档**（Dispose 后系统"小超时"自动断开、重建设备对象按需重连；BluetoothLEDevice.Close 仅当本应用是唯一持有者才关连接；GATT 连接/发现可能因系统队列等待数分钟且当前不能取消）：`learn.microsoft.com/windows/apps/develop/devices-sensors/gatt-client`、`learn.microsoft.com/uwp/api/windows.devices.bluetooth.bluetoothledevice.close`
- **微软 BluetoothLEDevice 构造入口文档**：`FromIdAsync` 明确要求从 UI 线程调用（可能触发访问授权）；`FromBluetoothAddressAsync` 无此线程要求，并支持从已进入系统缓存的配对设备地址重建设备对象。2026-09-12 现场的 MTA `FromIdAsync` 先返回 Windows 资源错误，后续日志时序显示下一次请求占住 BLE 工作线程（阶段日志缺失，属结合代码的推断），故改用配对 AssociationEndpoint ID 内的对端地址调用后者；不记录真实地址。官方依据：`learn.microsoft.com/uwp/api/windows.devices.bluetooth.bluetoothledevice.fromidasync`、`learn.microsoft.com/uwp/api/windows.devices.bluetooth.bluetoothledevice.frombluetoothaddressasync`。
- **MS Q&A 99038**（只 Dispose 设备不 Dispose 服务则无法重连）、**MS Q&A 2280559**（RPA 解析滞后导致进程重启后首次 GetGattServicesAsync 必 Unreachable，官方建议 3 次重试 ×1s + Uncached）、**MS Q&A 1685221**（FromBluetoothAddressAsync 返回 null 僵死 bug，Win11 2024.01D 已修；MaintainConnection 遇 bond 丢失会重连循环）
- **Qt 论坛 156281**（实测：OS 侧服务缓存僵死，重启应用无效，**关开蓝牙是唯一有效修复**——与本机取证一致，是本仓库选择无线电恢复的直接依据）：`forum.qt.io/topic/156281`
- **ZSTDJan/windows-remote-mic-app**，提交 `af54fd8e85a70f5b8f19cd4fa5bf11fe7fe530d6`，`apps/windows/rc003/src/ovb_rc003/ble_transport_winrt.py`（2026-09-14 复核）：参考实现从已配对 BLE selector 取得设备 ID 后调用 `FromIdAsync`，以 Uncached 发现服务；关闭时先取消写入/停止工作线程，再关闭 CCCD、退订事件并依次 Close service/device，且保留关闭失败的所有者供后续再次释放。本仓库据此修正 `BleSession::close` 首次 Close 失败后只回放旧错误、没有真正重试的缺陷。没有照搬其连接入口：同一僵死现场实测该配对 ID 路径返回 `0x80004004`，直接 GATT selector 返回 `0x80070008`，证明换构造入口不能恢复已经失效的系统栈。
- **Windows PnP 自动恢复公开接口**（2026-09-14）：微软 PnPUtil 文档提供 `/restart-device <instance ID>`，设备节点变更需要管理员权限；`ShellExecuteExW` 的 `runas` verb 用于显示系统 UAC 并启动提权操作；SetupAPI `SetupDiGetClassDevsW`/设备属性用于只选择当前存在、服务为 `BTHUSB` 的唯一蓝牙适配器。实现不记录实例 ID、不接受外部命令或路径，并在工具退出后独立用 WinRT Radio 枚举验证，而不信任单独的进程退出码。官方依据：`learn.microsoft.com/windows-hardware/drivers/devtest/pnputil-command-syntax`、`learn.microsoft.com/windows/win32/api/shellapi/nf-shellapi-shellexecuteexw`、`learn.microsoft.com/windows/win32/api/setupapi/nf-setupapi-setupdigetclassdevsw`。

### 2026-09-16 A/B 对照：无线电 Off/On 在僵死态无可观测收益

上节 Qt 156281「关开蓝牙是唯一有效修复」的适用边界已用现场日志划定（证据
`artifacts/ev_stream_raw.txt`，0.2.6，僵死现场，2983 条 `ble_connect` 记录）：

| 组 | 样本 | 恢复 | 恢复率 |
| --- | --- | --- | --- |
| 实验组：Off/On 之后首次重连 | 488 | 3 | **0.61%** |
| 对照组：同事件内普通重试（`attempt>=1`） | 2406 | 15 | **0.62%** |
| （参考）进程冷启动 `attempt=0` | 89 | 26 | 29.21% |
| （参考）Off/On 自身报 `failed` | 345 | 0 | 0.00% |
| （参考）Off/On 自身报 `passed` | 143 | 3 | 2.10% |

两组相差 **-0.01 个百分点**（判定阈值 ±10），双比例 z 检验 z=-0.022 / p=0.982，
95% Wilson 置信区间实验组 0.21%–1.79%、对照组 0.38%–1.03%（大幅重叠）。
即：开关与不开关在统计上不可区分。旁证：Off/On 自身报告成功 143 次，其后也只恢复
3 次——**"WinRT 说开关成功"不等于"碰到蓝牙栈"**，原因是启动预热缓存的 Radio 对象
让 Off/On 命中缓存而非真实栈。这解释了 Qt 156281 的结论只在**系统栈健康**时成立，
僵死态不成立。

引用措辞边界：只能说"无可观测收益，不值得保留一条会打断链路的路径"，
**不能**说成"零效果"（实验组 CI 上限 1.79%）。

据此改为**按错误码分流**（`bluetooth_radio::is_stack_exhausted`）：命中
`windows_resource_exhausted` / `winrt_operation_aborted` 时跳过 Off/On 与 PnP 重启，只留普通
重连，日志落 `ble_recovery_decision action=skip_recovery reason=stack_exhausted_proven_ineffective`；
非僵死码仍走原 Off/On 路径保留兜底。复算脚本 `scripts/analyze-radio-recovery-ab.py`，
操作与判读标准见 `Testing/WindowsBleResourceRecovery.md`。

### 2026-09-10 重连窗口 F5 泄漏补充

- **微软 `RegisterRawInputDevices` 文档**：同一进程、同一 Raw Input 设备类只能
  有一个接收窗口，最后一次注册覆盖前者；文档因此明确警告库内注册会干扰宿主
  自己的 Raw Input 处理。该约束解释了旧版 `key_suppressor.rs` 的键盘注册被
  `raw_input_windows.rs` 覆盖、断线期 F5 设备归因失效：
  `learn.microsoft.com/windows/win32/api/winuser/nf-winuser-registerrawinputdevices`。
- **参考实现复核**：本机 `reference-repos/vibe-flow` 提交
  `b47f7cdce8b753fade0c64c97332bebe80f17d2d` 的 `VoxDeckInputBridge.cs` 对语音
  F5 使用 LL 钩子兜底，并在重连扫描码变化时仍以持久语音映射为准；它接受实体
  键盘 F5 冲突。本仓库采用边界更窄的做法：主 Raw Input 窗口统一归因，只有
  Connecting/Discovering/AwaitingCapabilities/Reconnecting 建链窗口临时兜底，
  稳定状态继续保留实体键盘 F5。
- **记事本行为旁证**：Microsoft Q&A 的 Windows/Notepad 条目确认 F5 会插入当前
  日期时间。2026-09-10 本机现象格式与系统区域格式一致，结合诊断日志
  `seen=74 swallowed=0 leaked=74`，可排除 ASR 把语音识别成日期的解释。
- **Bleak winrt client 源码**（Unreachable 重试 10×1s；断开全量清理序列 CCCD=None→退订→逐服务 Close 带 0.1s 防挂起延迟）、**btleplug winrtble**（Uncached 触发连接、特征发现 5s 超时回退 Cached——#325：部分驱动 Uncached 请求无限挂起，本仓库 connect 尚无该超时，列为后续加固项）、**微软官方 BluetoothLE 示例 Scenario2_Client**（FromIdAsync→RequestAccessAsync→Uncached 发现→清理序列）
- **Windows.Devices.Radios.Radio**（微软 `RequestAccessAsync` / `SetStateAsync`
  文档）：改变无线电前先请求权限并检查 `RadioAccessStatus::Allowed`；
  `SetStateAsync` 返回只表示请求是否获准，实际状态异步转换，应观察
  `StateChanged` 或复读 `State` 确认。2026-09-12 统一包实测旧实现 0-5ms
  即误判两轮恢复失败，据此改为进程内缓存 Allowed、Off/On 有界复读确认。
  微软还说明 `RequestAccessAsync` 可能触发授权，应从可交互 UI 上下文调用；
  Radio 可由 `GetRadiosAsync` 枚举，也可从已知 ID 创建。2026-09-13 现场证明
  系统资源耗尽后这三种取对象入口均返回 `0x80070008`，因此改为 Tauri setup
  阶段先取得并缓存对象，恢复线程只复用缓存；若启动预热失败但 BLE 后续恢复，
  则立即补建缓存。该设计只使用公开 Radio API，不引入提权或驱动。
  官方依据：`learn.microsoft.com/uwp/api/windows.devices.radios.radio.requestaccessasync`、
  `learn.microsoft.com/uwp/api/windows.devices.radios.radio.getradiosasync`、
  `learn.microsoft.com/uwp/api/windows.devices.radios.radio.setstateasync`。
- **Radio 设备查询兜底**（微软 `Radio.GetDeviceSelector` / `Radio.FromIdAsync`
  文档）：官方允许以 AQS + `DeviceInformation.FindAllAsync` 枚举后通过 ID
  重建 Radio，并说明硬件异常/移除场景下它比 `GetRadiosAsync` 更可靠。
  2026-09-12 现场两条路径均返回 `0x80070008`，据此把“公开 API 已穷尽”的
  人工提示边界固定下来。官方依据：
  `learn.microsoft.com/uwp/api/windows.devices.radios.radio.getdeviceselector`、
  `learn.microsoft.com/uwp/api/windows.devices.radios.radio.fromidasync`。

外部实现只作为带来源的参考。第三方应用进程注入、私有配置读取和来源不明二进制不进入稳定主路径。

## Windows 注册应用激活与前台验收（2026-09-26）

- **`IApplicationActivationManager::ActivateApplication`**：微软文档定义它按 AUMID
  激活当前会话中的通用启动契约，并返回承接契约的进程 ID。本仓库用它替代 AppsFolder
  路径中仅投递 `ShellExecuteExW` 的主路径；传统桌面注册项仍保留 Shell 回退。官方依据：
  `learn.microsoft.com/windows/win32/api/shobjidl_core/nf-shobjidl_core-iapplicationactivationmanager-activateapplication`。
- **AUMID 与多进程应用**：微软说明 AUMID 用于把应用的窗口、进程和资源关联起来，
  不依赖应用内部是单进程还是多进程；`GetApplicationUserModelId` 可从公开进程句柄读取
  该身份；窗口级 `System.AppUserModel.ID` 可覆盖进程级身份，用于共享宿主或同进程多应用。
  因此不能假定激活契约 PID 就是主窗口 PID，本仓库先按窗口级、再按进程级精确 AUMID
  枚举，最后以 `GetForegroundWindow` 读回验收。官方依据：
  `learn.microsoft.com/windows/apps/desktop/modernize/package-identity-overview`、
  `learn.microsoft.com/windows/win32/appxpkg/functions`、
  `learn.microsoft.com/windows/win32/properties/props-system-appusermodel-id`。
- **传统 AppsFolder 条目**：微软将 `System.Link.TargetParsingPath` 定义为链接项真实目标
  的 Shell 命名空间路径，文件目标时等同于显示路径；`IShellItem2::GetString` 是读取该
  PROPERTYKEY 的公开接口。本仓库用它取得完整 exe 路径，匹配所有同路径运行进程，
  避免误把 Shell 返回的启动器 PID 当主窗口，也避免仅按文件名造成跨目录碰撞。官方依据：
  `learn.microsoft.com/windows/win32/properties/props-system-link-targetparsingpath`、
  `learn.microsoft.com/windows/win32/api/shobjidl_core/nf-shobjidl_core-ishellitem2-getstring`。
- **边界**：只读取 Windows 公开的应用身份，不读取 ChatGPT 或其他第三方应用的私有
  配置、数据库或进程内存；日志不记录 AUMID、窗口标题、路径或应用名称。

## Windows 系统快捷键录入与锁屏动作（2026-09-10）

- **执行端**：微软 `SendInput` 文档说明它把事件串行插入输入流、受 UIPI 与当前键态影响；`LockWorkStation` 是交互桌面进程可调用的公开锁屏 API，成功返回只表示异步锁屏请求已发起。Hooks 文档说明全局钩子事件局限于调用线程所在桌面。按键映射中的精确 `Win+L` 因而先等待实体键释放、由门控成对处理 DOWN/UP，再调用 `LockWorkStation`；其他快捷键仍走既有 `SendInput` 并保持按下即响应。官方依据：`learn.microsoft.com/windows/win32/api/winuser/nf-winuser-sendinput`、`learn.microsoft.com/windows/win32/api/winuser/nf-winuser-lockworkstation`、`learn.microsoft.com/windows/win32/winmsg/hooks`。
- **录入端**：微软 `LowLevelKeyboardProc` 文档明确低级键盘钩子在按键消息进入目标线程队列前运行，处理后返回非零可阻止继续传递，并要求回调快速把工作移交后台线程。本仓库复用常驻 `WH_KEYBOARD_LL` 门控：录入期间先吞物理 DOWN，所有对应重复 DOWN/UP 即使录入已经结束仍按同一次按住吞完；录入开始前已经按住的键则全程放行，避免不对称边沿。钩子只 `try_send`，Tauri 事件由独立线程发出。官方依据：`learn.microsoft.com/windows/win32/winmsg/lowlevelkeyboardproc`。
- **边界**：`Ctrl+Alt+Del` 等安全注意序列不属于普通快捷键录入能力；Win+L 在当前
  Windows 主机上即使低级钩子返回吞下仍会锁屏。因此自定义录入默认保留直接模式，
  并提供用户显式开启的“界面选择修饰键 + 物理键盘只按主键”安全模式；安全模式
  不在输入流中生成系统组合。
- **钩子链顺序补充（第二轮现场复验）**：微软 Hooks Overview 说明钩子按链调用，
  已处理事件可停止继续传给后续钩子/目标；`LowLevelKeyboardProc` 也明确非零返回
  阻止后续传递。现场观察到本钩子吞下 Win/L 后仍被系统锁屏；链首重挂又导致边沿
  完全丢失，实证 failed 并回退。产品路径不再依赖钩子链顺序屏蔽系统保留组合。
  官方依据：`learn.microsoft.com/windows/win32/winmsg/about-hooks`、
  `learn.microsoft.com/windows/win32/winmsg/lowlevelkeyboardproc`。
- **TV→锁屏的协议选择器兜底（2026-09-12）**：微软 Raw Input 文档明确
  `RIDEV_NOLEGACY` 只适用于鼠标/键盘，不能据此阻止消费控制 HID 的独立 Shell
  动作；`SetWinEventHook` 提供跨进程、out-of-context 的对象事件观察，
  `EVENT_OBJECT_CREATE` 早于 SHOW。现场证明 Windows 会在 SayAll 锁屏约 4 秒后
  由系统服务创建 `OpenWith.exe`；SHOW 阶段隐藏仍偶发闪帧，CREATE 阶段终止精确
  helper 连续四轮无可见弹窗。产品路径只在“已观察 TV→下一次 SayAll 锁屏”的
  15 秒窗口启用，并只处理 Windows `System32` 下映像名精确为 `OpenWith.exe`
  的进程。官方依据：
  `learn.microsoft.com/windows/win32/api/winuser/ns-winuser-rawinputdevice`、
  `learn.microsoft.com/windows/win32/api/winuser/nf-winuser-setwineventhook`、
  `learn.microsoft.com/windows/win32/winauto/event-constants`、
  `learn.microsoft.com/windows/win32/api/processthreadsapi/nf-processthreadsapi-terminateprocess`。

## WeType 热键休眠自动恢复调研来源（2026-09-05，热键休眠专项 v2）

场景：WeType 2.1.3.18 后台约 40 分钟后"TSF 存活但全局键盘钩子休眠"——和弦注入 LWin 穿透、无 0xFC、ConsentStore 时间戳不动；打开 WeType 任意自身界面立即复活（kb-live 会话 23-26 真机取证）。跨进程 `SetProcessInformation(ProcessPowerThrottling)` 解除节流**真机证伪**（对其他进程 E_INVALIDARG 0x80070057，wetype_service 打开即 0x80070005，15:04 live12 取证），该路线已从 `wetype_revive.rs` 移除。v2 已实现（`ble.rs` + `ime.rs`）：检测（注入后 700ms ConsentStore 时间戳未动）→ TSF 配置切换唤醒（`cycle_wetype_profile`：激活微软拼音 80ms 后切回，公开 API）→ 300ms 后经 `WorkerMessage::RetryVoiceChord` 在工作线程释放旧和弦并重注入 → 二次检测未响应才提示人工。关键参考与实测：

- **TSF profile 管理 API**（`ITfInputProcessorProfileMgr::ActivateProfile`/`EnumProfiles`，`TF_IPPMF_FORSESSION` 会话级激活）：微软官方文档 `learn.microsoft.com/windows/win32/api/msctf/nf-msctf-itfinputprocessorprofilemgr-activateprofile`。选择理由：切换配置会向所有 TSF 感知进程广播激活事件，是唯一能从外部触达 WeType 的公开 API 路径。
- **会话 47 真机实测（live13 + kb-live.log 全解码，2026-09-05 15:21）**：休眠中按键 → 检测未响应 → 配置切换真实完成（STA 线程，切微软拼音 clsid 9D2B2E2B 再切回）→ **346ms 后重注入的和弦同样未开麦**（LL 钩子日志见注入的 5B 边沿泄漏可见、无 0xFC 标记）。
- **16:44-17:35 七次休眠发作实测（kb-live.log，2026-09-05 晚间复盘）**：用户大量使用语音键期间钩子反复休眠/复活，七个发作簇全部同构：首和弦失败（5B 泄漏）→ v2 自动重试（cycle+300ms 时序精确吻合）**7/7 失败**→ 用户在 cycle 后 **1.24/1.28/1.68/1.85/1.9/2.28s** 的再按全部成功（FC 标记 + ConsentStore 开麦交叉验证）。**结论：配置切换确实能复活休眠钩子，复活延迟实测 ∈ (300ms, ~2.3s]（一次疑似 ≤6s）**；+300ms 重注入恒过早。注意：该时段应用为并行会话部署的无日志实例（pid 11692，含共享分支上的 v2 代码），cycle 隐形运行——与 kb-live 时序吻合。据此实现重试阶梯（WETYPE_RETRY_SETTLE_MS=[2000,3000]，两轮 cycle+重注入，最后才提示人工）。
- **LL 钩子日志判据（2026-09-05 新增，kb-live.log 全天 128 和弦窗口解码）**：WeType 钩子存活时**消费注入的和弦 LWin 边沿并注入自己的 0xFC 标记对**（每边沿一对瞬时 D/U）；钩子休眠时 5B 边沿泄漏可见、无 0xFC。此判据与 ConsentStore 开麦时间戳 100% 交叉验证一致，成为"钩子死活"的即时观测手段（无需开麦）。全天时间线（毫秒时间戳锚定）：休眠形成于 15:08:21（最后一次成功会话结束）→15:16:10（首次失败）之间的**方向键-only 活动窗口（≤8 分钟）**；此前 14:13、15:04 等休眠段落与手动复活（打开 WeType 界面）全部对齐。**休眠形成是偶发的**：15:21:34 复活后钩子存活 ≥80 分钟（跨两次探针开麦会话、用户离开/打字交替），未再休眠——"40 分钟规律"不成立，形成条件未定。
- **健全性实测（2026-09-05 16:41 持锁）**：cycle profile（STA）后 1s 注入和弦照常开麦——**配置切换不破坏活钩子的和弦触发**，v2 复活路径前提成立。剩余验证：重试阶梯版待下一次自然休眠发作做端到端确认（一次按住内自动恢复）。
- **首按失败根因终局定论与验证（2026-09-05 21:34-21:38，commit 1b55cca 部署后）**：真正的根因是 **F5 泄漏三键拒绝**——遥控器闲置后应用自身被后台节流，0x04→抑制器武装的链路（经工作线程队列）拖 ~120ms，F5 的 60ms 有界等待超时泄漏 → 和弦变成 F5+Ctrl+Win 被微信输入法拒绝；断连重连变体中首个 F5 在 0x04 前泄漏、UP 丢失致 OS 键态粘 F5。修复（三重防线）：GATT 回调线程直接武装 + 和弦前 F5 解粘 UP + 抑制器决策计数日志。**验证结果：4/4 会话首按成功**（含一次 25 分钟闲置后首按），suppressor_stats leaked=0（135 个 F5 全部吞下，其中 1 个冷启动 F5 由 GATT 回调武装+有界等待兜住），kb-live 零 F5 D 泄漏、全部和弦带 FC 成功标记，解粘 UP 按设计仅在需要时进入 OS。"WeType 钩子休眠"理论正式退役：全部证据与 F5 泄漏 + 20ms 间隔冷态拒绝两个机制一致；重试阶梯保留为无害安全网。
- **`SetProcessInformation` 权限边界**：微软文档明确 ProcessPowerThrottling 仅作用于调用进程自身；对其他进程返回 E_INVALIDARG。真机取证一致（live12）。
- **WeType 进程布局（本机取证）**：开麦方为 `wetype_update.exe`（ConsentStore 条目，拥有顶层窗口 StatusBarWnd）；另有 wetype_service/wetype_server/wetype_renderer。休眠的是钩子所在后台进程，TSF DLL 运行于前台应用进程内不受影响——这解释了为何 TSF 路径（中文输入）存活而全局钩子休眠。

## 应用内更新（tauri-plugin-updater + GitHub Releases）调研来源（2026-09-05）

场景：Windows 应用内"检查更新 + 下载安装"（GitHub-only、零自建服务器）。关键行为均以插件源码/官方文档原文核对，非推测：

- **官方 updater 插件文档**（`v2.tauri.app/plugin/updater/`，免费开源，MIT/Apache-2.0）：静态 JSON 端点模式官方示例即 `https://github.com/<owner>/<repo>/releases/latest/download/latest.json`（GitHub 302 到最新**稳定** Release 的资产，草稿/prerelease 不参与 latest）；`latest.json` 必需字段 `version`/`platforms.<target>.url`/`platforms.<target>.signature`，`signature` 为 `.sig` 文件**内容**（非路径）；签名强制不可关闭，私钥丢失即无法再向存量用户推送更新。
- **tauri-plugin-updater 源码**（plugins-workspace v2，`updater.rs`/`config.rs`，按 2.11.0 核对）：`check()` 对 204 返回"无更新"、200 解析 JSON 后按 SemVer `release.version > current` 判定；平台键按 `windows-x86_64-nsis` → `windows-x86_64` 顺序回退查找（latest.json 只需提供 `windows-x86_64`，dev 与 NSIS 安装态通用）；`Update.timeout` 默认 None（下载不限时），builder 的 timeout 只作用于 check 请求；下载完成后**先验签**再安装。**Windows 安装时序**：`install_inner` = 解包 → `on_before_exit` 回调 → `ShellExecuteW` 启动安装器（NSIS 参数 `/P`（passive）+ `/UPDATE` + `/R`（装完自动重启应用））→ `std::process::exit(0)`——**Drop 清理不会执行**，必须把 BLE 断开等成对清理放进 `on_before_exit`（本仓库 2026-09-05"部署不得强杀/强杀残留"教训的更新路径版）；`config.rs::validate_endpoints`：debug 构建 http 端点仅警告放行，release 构建强制 https（本仓库本地 E2E 用 `SAYALL_UPDATER_ENDPOINT` 覆盖端点 + dev 构建走 http，正式配置不含任何 dangerous 开关）。
- **Tauri 2.11.5 正常退出事件**：官方 [`AppHandle::exit`](https://docs.rs/tauri/2.11.5/tauri/struct.AppHandle.html#method.exit) 明确触发 `RunEvent::ExitRequested` 与 `RunEvent::Exit`；官方 [`App::run`](https://docs.rs/tauri/2.11.5/tauri/struct.App.html#method.run) 示例在 `ExitRequested` 中允许 `prevent_exit`。本仓库据此在退出请求时短暂阻止事件循环结束，把有界清理放到后台线程，清理各阶段执行完再调用 `exit`；第二次 `ExitRequested` 读取已执行终态后放行。仅采用公开生命周期模式，未复制外部实现。
- **tauri-cli/bundler 构建约束**（tauri issues #13259、#15638 + 组织讨论 #6013 佐证，并在本机复验）：`tauri.conf.json` 配置 `plugins.updater.pubkey`（及 `createUpdaterArtifacts`）后，构建时缺 `TAURI_SIGNING_PRIVATE_KEY` 环境变量会直接失败——现有 Windows CI 的无签名预览构建必须配套处理（无 Secret 时生成一次性临时密钥保 CI 绿灯；正式 Release workflow 缺 Secret 直接失败，防止发布不可用更新包）。
- **tauri-action**（官方构建+发布 Action，自动生成 latest.json）：评估未采用——它整体接管 build+release，无法嵌入本仓库既有的 verify-windows-bundle、安装生命周期矩阵等既有验收步骤；改为保留既有构建流程 + 自写 `generate-updater-manifest.ps1`（latest.json 生成逻辑对齐 tauri-action 的字段来源：version←tauri.conf.json、signature←`.sig` 文件内容、url←Release 资产直链）。
- **发布资产命名**：NSIS 产物名含中文与空格（`无线麦 SayAll_*.exe`），GitHub 资产直链需 percent-encoding；为消除编码风险，Release 资产在 CI 中复制为纯 ASCII 名（`SayAll-Windows-<version>-x64-setup.exe`）后上传，本地构建产物名不变（CI 全部脚本按 `*-setup.exe` 过滤定位，实测不受新增 `.sig` 影响）。
- **NSIS 与既有安装器门禁的相互作用**：updater 以 `/P`（passive）+ `/UPDATE` 运行，既有 installer-hooks.nsh 的 PREINSTALL SemVer 降级门禁照常生效（升级路径不受影响）；POSTINSTALL 的 VB-CABLE 提示在 passive（非 Silent）模式下仍会弹出——仅影响未装 VB-CABLE 的用户，与首装行为一致，保留。
- **预览版通道（2026-09-08 增补）**：Tauri 官方 Runtime Configuration 文档明确支持通过 `UpdaterBuilder::endpoints` 在运行时选择 stable/beta 等独立通道；本仓库据此保持默认稳定端点不变，仅在用户显式开启“检查预览版更新”后覆盖端点。GitHub Releases 页面公开提供标准 Atom feed（`releases.atom`），包含已发布的正式版与 Pre-release、排除 Draft；实现从本仓库 feed 的 `alternate` 链接读取 SemVer tag，选择最高版本并自行构造本仓库 `https://github.com/GetSayAll/remote-mic-app-windows/releases/download/<tag>/latest.json`，避开匿名 REST API 每 IP 60 次/小时限流。最终安装包仍由 Tauri minisign 强制验签。

## 2026-10-04 覆盖升级与受限回收

- 同日晚按用户进一步明确的安装交互，已有版本提供“安装前卸载”与“请勿卸载”；前者安全退出后卸载并自动续装，后者覆盖。安装权限依据 [NSIS RequestExecutionLevel](https://nsis.sourceforge.io/Reference/RequestExecutionLevel) 与微软 [Running with Administrator Privileges](https://learn.microsoft.com/en-us/windows/win32/secbp/running-with-administrator-privileges)：在既有 currentUser 安装范围采用 `highest`，对同一管理员账户请求可用权限，避免 `admin` 接受另一账户凭据后误用其 HKCU/配置。真正标准账户仍没有管理员写权限，不能把它表述为跨账户安装支持；受保护目录须在卸载旧版之前明确拒绝。主程序继续使用原普通权限 manifest，安装后启动复用现有 `nsis_tauri_utils::RunAsUser`。未引入 NSIS 已弃用的 UAC 插件、未改用户目录 ACL 或扩大产品运行权限。两条交互路径的实测与限制归同主题 Bug。
- 目录权限预检依据微软 [CreateFileW 的目录契约](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew#directories) 与 [File Access Rights Constants](https://learn.microsoft.com/en-us/windows/win32/fileio/file-access-rights-constants)。用 `OPEN_EXISTING`、`FILE_FLAG_BACKUP_SEMANTICS` 和目录新增文件/子目录权限打开现有目录，不创建探针文件、不修改 ACL，也不以运行中 EXE 的映像锁推断目录权限。[GetFileAttributesW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getfileattributesw) 失败时读取错误码，仅文件/路径不存在可检查父目录，拒绝访问等错误直接停止。本机同一普通令牌的受保护安装目录返回 5，工作区目录返回 0，句柄均关闭；这一证据只覆盖目录权限，不替代逐个 payload 写入和回收的实际成功判据。

- 安装模板实质改编自 [Tauri `tauri-cli-v2.11.4` 的 NSIS 模板](https://github.com/tauri-apps/tauri/blob/8909f221d1515955fc843808032bdc5d62209c96/crates/tauri-bundler/src/bundle/windows/nsis/installer.nsi)，固定提交 `8909f221d1515955fc843808032bdc5d62209c96`，上游 MIT / Apache-2.0 双许可证。按 [Tauri 官方自定义模板入口](https://v2.tauri.app/distribute/windows-installer/#custom-installer-template) 配置 `bundle.windows.nsis.template`。本地删除调用旧安装器及默认强制结束进程分支；交互安装按用户选择调用当前包生成的卸载器或覆盖，静默与被动更新覆盖，均保留原安装目录和用户设置。独立卸载器仍保留，不能把覆盖升级测试替代卸载或真实蓝牙验收。
- 旧文件回收使用公开 [IFileOperation::SetOperationFlags](https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-ifileoperation-setoperationflags) 的 `FOFX_RECYCLEONDELETE`、`FOFX_EARLYFAILURE` 与无错误 UI 标志；[IFileOperationProgressSink](https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nn-shobjidl_core-ifileoperationprogresssink) 逐项检查回收语义、成功结果和非空回收目标，再核对原路径消失。覆盖后只处理安装根目录内两个旧 Helper 和三个 SayAllInput 包文件；选择安装前卸载时另回收主程序、当前 Helper、Gadget、两份许可和卸载器六个固定文件。拒绝 reparse 路径，不递归删除、不提供永久删除替代。自建 Windows fixture 实际观察到五个退休文件和六个产品文件进入回收站，配置及未知文件保持；真实安装验收另记于同主题 Bug。
- `verify-capture-cleanup.ps1` 复用本产品新 Helper 的 `--cleanup-only` 协议：等待原助手正常退出后才允许独立提权清理，校验执行者 PID / 进程创建时刻 / 回执代次与终态；超时只阻止文件替换，保留仍等待真实释放的清理者。安装器不生成或修改清理证明。流程分支测试只从 PowerShell AST 提取函数并替换 OS 边界，不执行脚本顶层生产入口；模板 fixture 只编译；另有隔离的宏执行 fixture 验证卸载续装、拒绝写入和子卸载失败，不调用真实产品进程。

## WASAPI 原端点恢复（2026-09-15）

### 2026-10-04/05 麦克风目标与写入端自动配对

采用 [Windows Device Topologies](https://learn.microsoft.com/en-us/windows/win32/coreaudio/device-topologies) 与 [IConnector::GetDeviceIdConnectedTo](https://learn.microsoft.com/en-us/windows/win32/api/devicetopology/nf-devicetopology-iconnector-getdeviceidconnectedto) 公开拓扑查询，再读取适配器 `PKEY_Device_InstanceId` 证明同一条线；不以显示名替换、共同 ContainerId 或唯一播放设备猜配。代码位于 `audio_route.rs`，复用现有 Windows 依赖，没有新增库。

[VB-CABLE 官方参考手册](https://vb-audio.com/Cable/VBCABLE_ReferenceManual.pdf) 第 6–7 页说明 Pack45 的标准输入与 16ch 输入通向同一输出，且不可同时使用。本机只读生产 resolver 实证：一个 Capture、两个 Render，端点拓扑 ID 不同但适配器 PnP 身份一致；默认选标准输入，保留已有同线、精确 ID/名称有效的 16ch 选择。仅按官方适配器与 pin 属性识别 Base/A/B/C/D；多个同类 pin、身份缺失、跨线或设备不可用都不自动选择。其他设备保留显式手动配置并标明未确认配对，不声称支持 HiFi Cable/Voicemeeter 自动配对。此只读取证没有设置系统默认设备、打开音频流或采集语音，不能替代实体收音验收。

采用现有 `wasapi 0.24.0` 依赖，不新增音频引擎或复制外部代码。微软 [Recovering from an Invalid-Device Error](https://learn.microsoft.com/en-us/windows/win32/coreaudio/recovering-from-an-invalid-device-error) 定义释放旧 WASAPI 接口并重新激活设备的恢复模式；本产品保留用户明确选择的精确 ID/name，重新枚举验证并在 open 后再核 name，不切换默认端点。该官方错误场景不是本次内部队列溢出的根因证据。本机 10:19:39 worker queue_overflow 后 `fail_audio` 丢 sink 而后续无法重建的实证、隔离 WASAPI 故障后首用与连续消费、取消边界见 `artifacts/audio-sink-recovery-20260915/evidence.md`。不修改 32000 样本阈值；消费为何落后仍需新增吞吐元数据定位。


本轮持续吞吐调查沿用官方 [Capturing a Stream](https://learn.microsoft.com/en-us/windows/win32/coreaudio/capturing-a-stream) 的 GetBuffer/ReleaseBuffer 公开模式和 [Device Properties](https://learn.microsoft.com/en-us/windows/win32/coreaudio/device-properties) 的 ContainerId；仅 ignored 实验 harness 原生丢弃 capture packets，数据指针不解引用，不保存声音。对端 native GetMixFormat 与源16k分别换算时长。[VB-CABLE 官方手册](https://vb-audio.com/Cable/VBCABLE_ReferenceManual.pdf) 说明格式和buffer行为由player/recorder客户端共同影响，未保证write-only与实际录音等价。2026-09-15 精确旧/新生产源的write-only及明确consumer对照均failed；不能据此宣称缺consumer、采样单位、普通调度或某次更新已被证明为唯一根因。来源内容、SHA、实际拓扑与未完成边界见 `artifacts/audio-sink-recovery-20260915/production-source-audit/README.md`。未复制上游实现。


2026-09-16 ATVV 停止原因只读核对：Google 托管 [Telink固定头文件86f501](https://android.googlesource.com/platform/hardware/telink/atv/refDesignRcu/+/86f501098fb4ba60954cb046201ffe43ca360c3e/application/audio/gl_audio.h#113) 定义reason `0x02=RELEASE_HTT`、`0x08=TIMEOUT`；[固定实现184660](https://android.googlesource.com/platform/hardware/telink/atv/refDesignRcu/+/184660d870ebcfadbef674315a79a80b8c14a754/application/audio/gl_audio.c#582) 的MIC_EXTEND在匹配stream-id后重置传输看门狗，60秒常量用于等待MIC_OPEN，不能当作RC003录音上限依据。本机两次约60秒收到`00 02`、续期正常且无本机MIC_CLOSE/queue overflow；用户明确当时仍按住，因此只能称遥控器报告HTT释放，不能把协议标签当人体松手或已证固件上限。诊断保留原reason数值，无重启voice/改续期周期绕过。证据见 `artifacts/audio-sink-recovery-20260915/sixty-second-20260916/`。


上述ATVV参考与2026-09-16旧/新完整包四轮实测相互校验：两包MIC_OPEN/EXTEND编码、session来源及2.5s续期相同，仅首个deadline安排点不同；每轮23次续期且无本机关闭命令，remote仍报HTT释放。固定参考不能替代RC003厂商实现，GATT成功不是remote采纳证明。没有据此改周期、自动重开或冒称60s是小米官方固定规格。旧包对照后已正常恢复原fbc包；公开Capture session观察只证明一轮WeType采集端点，不读取声音，observer已正常退出。完整证据见 `artifacts/audio-sink-recovery-20260915/sixty-second-20260916/evidence.md`。


### 会话临时 Capture 默认设备（2026-09-16，用户限定例外）

用户明确授权“允许受限使用，并接受外部改选时让出控制”。仅在本功能内独立绑定 `IPolicyConfig::SetDefaultEndpoint`；不复制第三方切换器实现，不扩展其他私有 API。固定 ABI 研究来源：[AudioEndPointLibrary / PolicyConfig.h](https://github.com/Belphemur/AudioEndPointLibrary/blob/4fd74314f7a8e4ceaaa6767cdc9f936c3916a2a8/DefSound/PolicyConfig.h)（SoundSwitch 使用、EreTIk 来源）、[Sunshine 交叉定义](https://github.com/LizardByte/Sunshine/blob/f54f9dfc57848971e85cda7fb4b7723594926422/src/platform/windows/PolicyConfig.h)。两者 GPL；本项目 GPL-3.0-only，当前仅独立声明 IID/CLSID/slot13 ABI 事实，没有拷贝其他方法实现或二进制。SoundSwitch 主仓研究固定点 `9e69fd3ef0d20474684cf6fe2f8789440dd09e4c`。

公开读回与通知复用现有 wasapi 0.24 / windows 0.62 的 MMDevice 接口。[OnDefaultDeviceChanged](https://learn.microsoft.com/en-us/windows/win32/api/mmdeviceapi/nf-mmdeviceapi-immnotificationclient-ondefaultdevicechanged) 不提供更改者；只唤醒三角色实际读回，不按时间窗认领自身写入。IPolicyConfig 没有 CAS/可取消 setter 契约：逐步前后读回与源 epoch 缩小竞态，无法证明外部在读写间或同值选择绝不被覆盖。任一可观察外部冲突整体让出，全部角色保留；写前 journal 和崩溃后显式选择避免无证据自动恢复。源码/模拟事务测试不是默认路由真机或微信输入法实际收音验收。

### 2026-09-16 会话输入切换的角色联动纠偏

9a7048包实测首按在Console setter后被本应用exact-vector中止（3次），第二按仅剩Communications切换；不能标第三方就绪延迟。Windows公开ERole概念不保证本次未文档化setter独立性。SoundSwitch成熟实现按Console/Multimedia/Communications逐项检查并设置（既有固定来源研究），不从其循环推出三个setter互不影响。

作者原始实验参考：[IPolicyConfig from Go](https://zenn.dev/gsnhjj/articles/go-windows-ipolicyconfig-default-audio?locale=en) 直接报告Console/Multimedia双向联动，非微软契约、非本机因果证明。仅用于形成可证伪组模型；本机实际新候选须由匿名before/after掩码证实。未复制该文实现。组事务仍拒绝任何第三值或组外改变；同值外改无actor/CAS不能归因。详细失败与候选验证见 `artifacts/capture-input-session-20260916/first-use-fix/evidence.md`。

### 2026-09-21 增强启动选择与电源异常解释

完整模板菜单采用现有 Tauri 可聚焦窗口与本地键盘事件承接交互；仅主动菜单可获取焦点，短提示仍不激活。[GetForegroundWindow](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getforegroundwindow) 的实际 HWND 用于验证输入确属本菜单；[SetForegroundWindow](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setforegroundwindow) 受系统前台限制，调用成功不替代实际窗口核验。确认等待真实释放，失焦取消不抢回；未扩 HID/WDF 协议、未复制外部实现或添加依赖。

首次菜单修订静态核对固定依赖 tao 0.35.3：其 Windows `force_window_active` 在前台请求被拒时会注入 Alt 再试。本菜单不采用该隐式回退，改为单次公开 SetForegroundWindow 并核实际 HWND；失败取消。确认全释放后先恢复目标、成功后再隐藏活动菜单，避免 hide 先让系统挑下一活动窗口；恢复被拒时保留菜单并提示用户点击目标。该生命周期缺口不等于已证实历史主窗口带起的唯一原因，新进程首次实测仍须确认。

增强启动恢复复用现有 ShellExecuteExW 的 runas，用户 opt-in 默认关闭；普通主程序只请求独立固定 Helper，不建立服务或计划任务，不绕过 UAC。[Microsoft ShellExecuteW / runas](https://learn.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-shellexecutew) 明确该动词触发用户授权或凭据提示。取消后本次不自动重试；关闭只取消未开始的请求，不强制终止活动 Helper。未复制外部代码或引入新依赖。

[Microsoft Bug Check 0x9F](https://learn.microsoft.com/en-us/windows-hardware/drivers/debugger/bug-check-0x9f--driver-power-state-failure) 将参数1=3解释为设备对象阻塞电源 IRP 过久；该定义不能只凭事件41或错误码确定责任驱动。2026-09-21实际WER 1001与新启动事件独立确认该错误；dump内容未取得，责任驱动unknown，不默认归咎系统/硬件或排除当前WDF Helper。全部未执行电源授权撤销、控制器拒绝执行；事实与时间线归既有三键evidence。

2026-09-21连续方向修订复用现有公开UI Automation能力快照：区域与动作来自同一次证据，旧代次结果丢弃；Unknown无交互面板时不接管四方向，沿用原生事件且不补注入。没有复制外部实现、增加依赖或更改HID/WDF/LL来源合同；Codex错配与微信Unknown分别按既有evidence记录，不把原生方向等同全语义支持。

2026-09-21方向低延迟修订依微软[获取UI Automation元素](https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-obtainingelements)所述GetFocusedElement与祖先TreeWalker公开能力，仅MoveCursor重新验证当前输入接收者及目标，不遍历无关全树。继续使用CompareElements/HasKeyboardFocus/IsEnabled与WindowPattern modal证据，注入前目标及取消代次校验保持。未复制外部源码，无新依赖；实际性能必须由闲置首用实测，不能从接口选择推定。

2026-09-22用户明确统一固定键/组合键并删除区域切换。当前源码移除application_control的第三方UIA查询/执行及scene语义worker，只保留公开前台进程身份和本应用菜单窗口恢复；上述UIA参考保留为历史来源，不是继续扫描的需求或兼容路径。新路径实质复用本仓库ButtonMappings/SendInput、SetWinEventHook前台程序通知、既有原子SettingsStore和Tauri菜单，未引入新依赖或外部代码。普通单击配置复用既有立即执行/重复/UP，未新增双击或长按等待。一次配置转换保留原用户备份，无固定等价UI动作置未配置；新契约实机尚待完成。

同日面板内Menu长按切换保存意图，复用本仓库已有GestureRecognizer及原550ms阈值/Long后UP静默契约（原来源为RemoteButtonGestureRecognizer/HIDRemoteScheduler），只增加本应用面板识别标记与当前物理按下代次校验；无新计时器、依赖或外部复制，不改变普通映射时序。实体结果归既有三键evidence。

## Windows 官方上游整合（2026-09-27）

- 来源：<https://github.com/GetSayAll/remote-mic-app-windows>，GPL-3.0-only；固定提交 `74230bf5f841cac2f099d1c6fd25683dac50131d`，比较基线 `6504010828b12713ce033cb3e231087af6a6482f`。这次是同一 Windows 仓库历史整合，不能与 macOS 参考或 remote-bridge-hub 混称。
- 融合模块：`battery.rs`/电量指示，`registered_apps.rs`/应用库，`send_input.rs`/鼠标动作，`key_gate.rs`/系统级录入配对，BLE 部分连接 RAII/资源记录，`graceful_exit.rs`/安装正常退出请求。模板仍只发固定键；应用库只作用户显式通用动作，保持本地草稿保存和可分享导出隐私。
- 保留本地：逐报告 WDF/PDO 当前来源、mask0物理观察不映射、输入设备锁定与唯一 ExitCleanup、程序默认/临时或保存默认、菜单焦点和偏好、底栏。排除上游仅以 BLE 建链状态吞 F5、遥控在线常驻通用门控、安装超时强杀、全局 artifacts 忽略。历史取证当前树已匿名化，不复跑探针、不重新认可被本地事实取代的失败路线。
- 合并候选实机仍 pending；历史 RC001、睡眠、语音结果不外推。本轮没有电源、无线电或驱动安装授权。

### 2026-10-02 增量同步边界

- 同一 Windows 上游固定提交 `c81308e011a757dc22d22f2861b687ef787d295e`，与此前已整合的 `74230bf5f841cac2f099d1c6fd25683dac50131d` 相比新增 154 个提交；许可证仍为 GPL-3.0-only。按现行 PLAN 逐块融合，本地用户配置、固定按键模板、逐报告 WDF/PDO 来源与正常清理契约优先。具体任务及实际验证状态归 TODO；上游作者的实验记录不是本机候选验收。
- 本轮平台融合模块包括 `ble.rs` 的标准 BAS 电量订阅/读取、`application_control.rs` 的已运行应用窗口筛选/激活、`ime.rs` 的会话输入工具与快捷键录入期间让位/恢复，以及微信响应标记和 Vokie 检测。它们仍通过公开 Windows API 或可观察输入事件工作；上游新一轮本机实验结果不外推。保留本地连接到达去重、语音输入设备锁定、直接模板与来源路由。
- 前端融合上游设置页、应用图标选项、侧栏版本/齿轮、系统强调色、完整遥控型号名、输入工具配置及快捷键录入呈现；保留本地驱动/模板页、显式保存、固定底栏与输入设备锁定。上游连接页“支持更多输入工具”的第二增强链开关不采用，不把保留的输入工具选择混称为全按键捕获。
- 设置页与可切换图标直接来源于本次 Windows 上游，按其归属说明追溯至 macOS `e8af2da2` 的 `SettingsView.swift`、`AppIconController.swift` 与 `Resources/AppIcons/faceted-duck.png`；本次不引入 macOS 源码。`src-tauri/icons/app-icons/faceted-duck-{16,20,24,32,256}.png` 和 `public/app-icon-faceted-duck.png` 是上游缩放派生资源，继续遵守 [LOGO-LICENSE.md](LOGO-LICENSE.md) 的专有品牌资产边界。Windows 运行期仅变更窗口/托盘与设置页呈现，不宣称更改安装器、快捷方式或可执行文件内图标。
- **未采用的第二增强链**：`crates/sayall-windows/src/rc003_bridge.rs`、`src-tauri/src/rc003_task.rs`、`hardware/RC003/helper/` 的产品路线不作为本地增强实现。上游 Helper 以共享宿主的 usage 内容代替逐报告设备来源，动态目标包含方向与确认等通用键；其旧 agent 自动刷新调用 `TerminateProcess` 结束 WUDFHost，并通过最高权限计划任务重复启动。上述行为与本地严格来源、显式限定 runas、清理确认后再启及不强杀契约冲突。
- **语音边界**：上游 `rc003_bridge` 下发语音 usage 替换并以命令写出成功置 `voice_synth_active`，BLE 据此停用 SendInput；此报告合成不在本地指定非语音按键旁路例外中，也不能用提交成功证明目标行为。保留本地 ATVV 按下/释放与配对快捷键路径，不把基础语音改成依赖常驻 Gadget。
- **历史材料边界**：本轮上游新增探针与 evidence 中发现个人绝对路径和完整设备接口路径，不能未经脱敏纳入本地新提交的文件树。其源码与历史实验可在上述固定上游提交追溯；不复跑探针，不把既有 IOCTL 假设替代本地设备来源实证，不恢复已停止的任务视图取消调查。

## 2026-09-29 WebView 进程异常处理

- [Microsoft WebView2 进程事件](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/process-related-events)：RenderProcessExited 可用 Reload；BrowserProcessExited 需要重新创建控件，本轮不盲目 Reload 浏览器退出。使用项目现有 Tauri / webview2-com 0.38.2 公开回调，只记录类别/数值，不记 ProcessDescription 或用户地址。每窗口至多一次 renderer 重载；无法恢复提示通过产品正常退出重开，未宣称原崩溃责任已解决。
- [MINIDUMP_EXCEPTION_STREAM](https://learn.microsoft.com/en-us/windows/win32/api/minidumpapiset/ns-minidumpapiset-minidump_exception_stream) 与 [MINIDUMP_MODULE](https://learn.microsoft.com/en-us/windows/win32/api/minidumpapiset/ns-minidumpapiset-minidump_module)：只解释本应用已有 dump 的异常码/模块相对地址，未读取或发布内存正文。
