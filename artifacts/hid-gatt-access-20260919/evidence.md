# RC003 公共 GATT 只读核验终态（2026-09-19）

用户要求停止新增诊断后，本轮已经收尾，无待退出探针，不再修工具、构建或重试。三键仍未恢复；没有支持新的最小产品修复的直接报告证据。

## WinRT：实际系统拒绝

普通 Explorer 用户 STA、未打包进程（probe/app package query=15700），19:27:45–19:27:47：选定设备精确匹配、Connected，device access=Allowed；唯一 HID 1812 服务枚举 Status=Success、ProtocolError=null，RC003 revision 验证通过；该服务 RequestAccessAsync=DeniedBySystem。立即停止，exit 1，没有枚举后续特征或读取静态值。

只释放本次自己的一个 service 与 BluetoothLEDevice，配置哈希一致；未关闭其他对象。完整匿名事件见 winrt-result.json / winrt-metadata.log；精确源码与启动壳保留为 winrt-probe-frozen.ps1 / winrt-runner-frozen.ps1，对应 freeze.json。

这次系统拒绝只覆盖 WinRT。旧“空集合即 OS HID 占用”推论已在历史调查文档原位纠正，不能外推所有公共 API。

## Win32：原生调用未执行

C 候选编译及脚本语法通过；SDK结构、固定 GATT 服务接口、GENERIC_READ/shareRWD、Characteristics(NULL parent)、Descriptors 元数据与软期限设计经5项hash定向审查。对应 win32-freeze.json，原生日志/运行成功不能从源码审查推导。

19:43:48–19:43:49，普通 Explorer PS5 入口再次取得选定设备唯一1812定位成功，但在启动 native 之前出现 RuntimeException（0x80131501），stage=win32_metadata，exit 1。无 win32_started/native 事件。随后无设备调用的类型反射证实 Windows PowerShell5/.NET Framework 的 ProcessStartInfo 不提供脚本设置的 StandardInputEncoding 属性（False）；这是启动壳兼容缺陷。没有执行 CreateFile/BluetoothGATTGetCharacteristics/GetDescriptors，不能记成 Win32 access denied 或证明 Win32 路线不可用。

用户要求立即收敛，故保留失败候选和完整 win32-result.json / win32-metadata.log，不修工具、不重编、不再次启动。Win32访问结论仍为 unknown / not executed。

## 清理与状态

19:45:01 post-run-state.json：PS探针已退出，native探针进程数0；原安装应用仍存活、响应、Session1、安装路径精确匹配。Win32终态配置哈希一致。没有强杀、pending操作或后台新探针；未写CCCD、注册通知、读取Report值或语音，未改变安全配置。

仅释放自有对象不等于证明物理BLE连接绝对未变化，保留 physicalConnectionUnchangedProven=false。当前应用可响应也不能冒充三键或语音的新实测。

## 卡点与停止条件

- 此RC003在当前WinRT服务权限检查明确DeniedBySystem。
- Win32最后候选因自有脚本兼容错误未到服务访问，按用户指令停止，未知仍未知。
- 既有用户态宿主归因没有公开接口→实际报告闭环，先前120秒550次调用全部未知，不重新附加。
- 现有KMDF三键候选源码/构建证据存在，但Microsoft签名渠道当前不可用，未加载、未实机验收；签名不是“已修复”的替代证据。
- 没有新增产品修复、安装、Git提交或发布。保留已完成capture功能提交85fffc84d41c3bdaf822e49ab115e1d922c75e42，不重复验证。

## 2026-09-19 实施收束复核

本次只读复核固定上游、既有现场证据和当前候选，不重开设备实验。来源与许可仍归 [ATTRIBUTION 三键静态核对](../../ATTRIBUTION.md#three-key-reference-static-review)：固定 `8a93f321ac71a602300c6cd77f7256fa4b63068e` 的 hook 未将 file handle 关联到选定设备，HostPid、IOCTL 与格式过滤不能证明每份报告来源；上游 active set 产生边沿也不能替代设备精确抑制。旧 PDO 匹配失败及 future-open 零打开证据继续有效，不撤销来源门禁。

现有 `Testing/hid_gatt_metadata.c` 只设计精确服务接口的 CreateFile、特征和描述符枚举，没有报告读取、通知或原生输入抑制。修正启动壳只能使元数据访问有机会执行，仍需新的探索才能形成产品方案；因此 Win32 保持 unknown / not executed，既不记成系统拒绝，也不扩展本次范围重试。

三键逐键映射、快按/重复/长按释放、闲置首用、键盘交替/同时及断连/睡眠/按住退出均没有新增成功证据，保持未验收；既有捕获候选 failed 不改记为本次执行失败。现有内核候选另受 Microsoft 签名渠道不可用限制，且三键 mask 不含 TV/Home。此前将完整技术方案表述为解除外部阻塞条件过早：方案判定属于开发方职责，用户无需寻找接口或实现；最新静态判定如下，不证明所有方案不可能。

本次只同步 TODO、FEATURES 与本证据；没有新增代码、探针、包、安装、配置或设备状态变更，没有撤销已有暂存改动。HEAD 与 `test/capture-input-session` 均为 `85fffc84d41c3bdaf822e49ab115e1d922c75e42`，实际当前分支为 `test/bootstrap-project-context`，未切换分支。没有本次临时产物需要清理；既有现场材料继续用于解释未解决失败，未移动或删除。

<a id="static-feasibility-correction"></a>
## 静态可行性纠正：来源与抑制分开

2026-09-19 用户纠正并续推后的限定静态结论：**内部技术判定未完成，非缺用户外部条件；三键仍未交付**。仅阅读既有源码、冻结证据、已安装系统二进制和公开接口，没有新增/执行探针、附加宿主、产品代码、设备操作、构建安装或实机验收。

### 逐报告来源归属

要求每份报告属于当前选定物理设备及其连接代次，不能由 HostPid、VK、遥控器在线或时间相关代替。现有 RawInput 以 header.hDevice → RIDI_DEVICENAME → 选定 HID 路径闭合其自身来源，但这个身份没有进入 WH_KEYBOARD_LL 的 KBDLLHOOKSTRUCT，不能自动移交给任意宿主 IOCTL。

具体候选是 WDFREQUEST → WdfRequestGetIoQueue → WdfIoQueueGetDevice → WdfDeviceQueryPropertyEx(DEVPKEY_Device_InstanceId)，再与公开 CM/SetupDi 枚举的选定 RC003 1812 节点及必要的精确 PnP 关系比对。它不依赖 IOCTL file object 名称等于 PDO。WdfRequestGetIoQueue 对驱动创建或已完成请求可返回 NULL；队列的 device 也可能是聚合父设备，不能因 API 类型正确就确认来源。必须拒绝 NULL、非预期驱动 globals、无法唯一关联的聚合设备、属性不符与过期连接代次；失败时不改报告、不发映射、不吞键。公开 PnP 父节点确为遥控器，但其接口与旧 IOCTL 句柄没有已证关联，仍不能放宽旧门禁。

本机固定 Microsoft.Bluetooth.Profiles.HidOverGatt.dll SHA256 为 372c3628e3366c18152199eb8fb554d27bb4f02d1356b1f514f711328df4a8d6；WUDFx02000.dll 为 e97a8bbbda4dd4989d9b3063cbfea05df9f60cdc7758fd21b163ebe3ce6786e0。后者没有各 WdfRequest API 的独立导出。既有 WDK UMDF 2.15 头说明其为 WdfFunctions[index](WdfDriverGlobals, ...) 包装；本机前者静态调用中表指针 RVA 0x32DC0、globals RVA 0x32DC8，与入口绑定和实际完成调用一致。DeviceQueryPropertyEx / IoQueueGetDevice / RequestGetIoQueue 槽为 54 / 90 / 175；RetrieveOutputBuffer / GetInformation / Complete / CompleteWithInformation 为 169 / 171 / 163 / 164。未来绑定必须校验精确模块身份、表和目标地址，不能在其他系统版本套用这些 RVA。这些是静态接口和绑定依据，尚未证明本机真实 read 请求的设备属性恰好满足来源契约。

### 原生动作精确抑制

固定参考仅在 NtDeviceIoControlFile 返回同步成功时读取 9 字节并 send，没有写报告。该返回点即使可改缓冲，也没有证明不存在已经获通知的其他消费者；pending 更不能保存裸指针后猜测完成时刻。因此“能观察 DOWN/UP”不等于“能在 Windows 消费前精确抑制”。

具体候选是在 WdfRequestComplete / CompleteWithInformation 进入前，对同一已归属的 HID read 请求用 GetParameters 确认请求类型，RetrieveOutputBuffer 及 Information 约束长度，先保存物理 usage 集合，仅移除已配置并已取得所有权的 usage，然后保留状态、长度及其他字段调用原完成入口。该请求的输出缓冲在完成前有效，完成后不再可访问；仍须证明选定入口位于目标报告实际交付前，而不是另一个复制/取消路径。

限定静态核对已见 IOCTL_HID_READ_REPORT 0xB000B 分派至 RVA 0x1F588，使用手动队列 RetrieveNextRequest 槽 91 得到 WDFREQUEST，调用格式化函数 RVA 0x1FEBC，再 SetInformation 槽 170 和 Complete 槽 163，故不是只找到取消完成。**尚未闭合格式化函数的输入/输出布局、完整异步报告回调及全部正常交付分支**。参考的 9 字节 GATT 数据和现有内核候选的 121 字节 HidClass 合同处于不同层，不能直接套用到这个缓冲。当前不写猜测性解析器，也不宣称已证明抑制可用。

若上述契约闭合，最小产品流是：每报告验证来源 → 保存物理状态 → 清除本次由增强负责的 usage → 原请求完成 → 带代次/序号的 DOWN/UP → 现有 DriverEdge、映射识别器和动作执行器。返回 F1、音量 80/81 之外必须覆盖 TV 35、Home 4A；现有 mask=7 不能直接满足。对应 key_gate 的在线常驻/时间窗吞键必须撤销，否则即使新旁路正确仍会误吞实体键盘。重复报告只更新物理状态，长按重复由已有配置执行，UP 立即取消；接管时已漏出的 DOWN 不得独吞 UP，断连、睡眠、序号缺口、正常退出必须成对释放并处理仍按住的物理键。以上是待实现和真实验证的产品条件，非已通过行为。

### 证据排除范围与下一步

| 路线 | 当前分类与边界 |
| --- | --- |
| 所选 PDO 名称直接等于宿主 IOCTL 对象名 | 6396 次合法非匹配明确否定此等同假设；不排除其他设备/请求关联 |
| 仅靠附加后自然 future-open 建立全部身份 | 唯一 120 秒零打开、550 次既有 IOCTL 未知，不能为当前既有通道及冷首用提供闭环；不证明永无未来打开 |
| LL 的 VK、在线状态或时间窗归因 | 不含逐设备身份，不能满足本任务严格并用条件 |
| WDF 请求设备链加完成前抑制 | 接口与固定模块绑定、正常 read 结构有静态依据；报告契约、异步路径、设备属性及生产生命周期验证仍未完成 |
| 父接口、其他现有句柄关联 | 公开 PnP 关系可查，但实际句柄成员关系未证明；不能写成已排除或已可用 |
| WinRT / Win32 公共 GATT | 前者仅该次访问被拒；后者未执行。元数据读取不等于报告获取，通知观察本身也不能取消原生 HID 消费 |
| 现有 KMDF 过滤候选 | 每设备完成回调中抑制的结构已存在；Microsoft 签名仅约束这条加载路线。TV/Home、物理报告和生命周期仍缺实现或验收 |

下一项开发方内部工作已限定为同一固定模块 RVA 0x1FEBC 格式化及异步完成数据契约，确认输出来源、布局、长度、生命周期和所有目标正常完成入口；再决定是否实现带严格绑定、来源失败放行、释放及日志的生产 Helper。它不是让用户另找方案，也不是恢复通用探针。本轮有限静态材料尚不足以闭合，按限定核对边界交付此判定，不继续无界反汇编。

**当前没有阻止上述内部工作的用户外部条件。** 若最终选择内核路线，合法 Microsoft 签名渠道才是该路线的真实外部条件；不能覆盖用户态路线。RC003 已有硬件，待应用与日志就绪后才协调用户实体动作；RC001 无硬件仍 deferred。

本轮原位纠正 TODO、FEATURES、PLAN、ATTRIBUTION 与本证据，保留此前历史结果、原有改动和 85fffc84。静态 dumpbin/读取成功只支持以上接口事实，不算产品测试或真机 passed；没有新增包、安装运行版本或临时产物。

### 2026-09-19 22:23 产品候选进展（尚未真机通过）

仅沿已限定格式化缺口读取固定模块：`0x1FEBC` 按 `end-begin + (reportId!=0)` 向 WDF 输出缓冲区写 ID 和原 payload；正常 read 的同步分支在 dispatcher Complete 前设置 Information，异步队列分支在 CompleteWithInformation 交付。缓冲区只在完成入口、调用原函数之前访问，不保留 pending IOCTL 输出指针。

已实现固定原生 Helper + 内嵌脚本，复用锁定 Frida 17.15.3，无启动下载。主程序只经显式按钮 runas 启动该固定哈希 Helper；管道互验进程并锁定 Helper 文件。宿主和两系统模块校验路径、签名与固定 SHA。每报告需要正常 HID_READ_REPORT、非空 queue/device 和完全相同的 DEVPKEY_Device_InstanceId；Helper 另从所选 HID interface 公开 CM 祖先链证明该 1812 实例，并用 HidP 合成/清除 usage 验证 3 个 u16 槽。最大 collection 长度 121 不当作实际长度；只接受 Report 1、长度 7..121、三槽 usage 范围及零尾部，其余不改写。

来源及格式门禁共同满足后，仅清除配置接管的 F1/80/81/35/4A。Helper 持有配对 WDF device 引用、精确实例 PnP 通知、配置代次及原生 hold 所有权；取消时先撤映射，再维持已吞按住的抑制直至物理释放。普通客户端复用原映射引擎，TV/Home 不再从 LL 在线/武装时间窗接管。版本/来源未通过不意味着三键不可实现，也不允许放宽 gate。

截至本条：7 项脚本契约测试（包含真实适配脚本的 WDF 内存替身）、原生 Helper 编译、Rust 三键解析/边沿 3 项、映射引擎 15 项、LL 8 项及新增五键跨重启双边沿 1 项、面板 3 项、前端 build 和 Tauri host check 均实际 exit 0。范围外两个历史 opt-in 缺陷未执行。本次新 Rust 片段格式已修正，`cargo fmt --all -- --check` exit 0。本地 NSIS 包构建 exit 0，产物 `target/release/bundle/nsis/无线麦 SayAll_0.2.5_x64-setup.exe`，SHA256 `8629cc17bae2d84be8f5301244e68f55d4cc30ea0644b77adbfccbfb969b334c`；应用 SHA256 `9b0d66fd469ea51b19055032a43f97b72d5a9defb835a0078fec93de301124b3`，随包固定 Helper SHA256 `85366a9976b8870fb0fd9d0a80cb4c10db5bb6ad80044fb9d8bf5013f560b3c8`。该包已于 22:39 管理员安装 exit 0。安装 EXE 与原件仅 NSIS 类型标记 UNK→NSS 三字节不同，安装 SHA256 `6d392abc23273f239bcaa7054869c81fd7d584fc0aa340519743e3c71cd87fc6`。旧 PID 10324 通过真实托盘菜单命令正常退出，清理 overall failed_stages=0（101 ms）；一次普通启动纠正为现有 Explorer 桌面调度后，新 PID 38676 的父进程为 explorer PID 8544、TokenElevation=false，RC003 自动连接 ready。22:45 经自有 UI 显式启动 Helper：管道身份、唯一 RC003、所选接口祖先链、HidP 描述符/五键合成契约实际通过；宿主身份/模块验签 gate 拒绝，Helper 自行退出且未附加、未改写报告。公开 Authenticode 检查确认三文件 Valid/Catalog，随后补充正确的 catalog 成员验证与分项拒绝日志，修订候选待重装验证；上述第一包身份保留为历史失败对照，当前产物路径将由修订候选替换，不宣称旧包继续是交付字节。

临时构建日志、固定 Helper 包及单测结果归 `target/dev/rc003-three-key/`，用于本次候选定位；验收收尾后只保留交付包及必要摘要，其余按 LOGGING 回收。原冻结证据保留。实机三键、快速/长按/闲置首用、键盘交替/同时、断连/睡眠/正常退出、语音与输入锁定回归均未执行，不得以以上静态与测试结果标为 passed；RC001 无实机仍 deferred。首次接管须先取得该精确实例的真实全释放报告，接管准备动作不计正式按键验收。

22:55 修订门禁：catalog 成员验签实际 passed；随后 WUDFx 哈希常量检查拒绝。定位为本记录及实现手工抄写多出一位 b（65 位），已按固定文件实际 64 位 SHA256 原位纠正，增加 C 编译期长度断言。重包前一次核对两模块实际 SHA、10 个 WDK 分发表槽、两个固定 RVA、IOCTL、参数类型/布局和 Device InstanceId key，均与已有静态契约相符；没有改变白名单版本或削弱比较。逐报告来源及真实动作仍未开始验证。

23:04 常量修订后的产品启动已通过固定模块/签名、双向管道进程身份、LocalService RX 载荷 ACL、Frida 附加及固定 WDF 脚本绑定（bound contract=1）；普通客户端提前关闭后，Helper script_unload/session_detach/terminal 均 code=0。根因已由新增真实 Windows 命名管道单测复现：Rust File 的 PIPE_NOWAIT 暂空读取被转换为 Ok(0)，原代码误当关闭。修为 PeekNamedPipe 区分暂空/断开后，真实管道的暂空、消息、正常关闭及非阻塞写满/关闭两测试均 passed（修前 1 failed/1 passed，修后 2 passed）。这是产品针对性验证，无设备/探针；逐报告来源、按键与抑制仍未实际验收。修订包重新构建中。

### 23:18 最终候选部署与实机等待

最终包包含上述管道修复和五键编辑说明纠正；包路径仍为 `target/release/bundle/nsis/无线麦 SayAll_0.2.5_x64-setup.exe`，本地时间 2026-09-19 23:18:32 +08:00，26,326,523 字节，SHA256 `7a20c2f26e99cee11643193c81677260f8248dcb1ce6ccbd4b08032062e7a0de`。最终前端构建及 NSIS 构建均 exit 0；未把 23:12 中间包用于部署。构建 EXE SHA256 `b88c42c69a17dcf1d23b7ea19e5241f62024f0af349672f2c2276bc4ba14428d`。

23:19:39 旧实例 59992 经真实托盘“退出”菜单命令进入既有清理，overall passed、failed_stages=0、95 ms；Helper 已退出才启动安装器，管理员覆盖既有目录 exit 0。新实例 43604 的实际安装 EXE 为 0.2.5，SHA256 `bf9870acc652daf7ab99753da075b74a092e953855410a7a375df0ba15af6a98`；与构建 EXE 逐字节比较仅 NSIS 标记 UNK→NSS 的三字节差异。父进程为现有 Explorer 8544，TokenElevation=false。固定 Helper 安装文件与构建文件同 SHA256 `9317e1e409e92683ca42138f7cff5e427a0465e53eed9f5b8ff41648d649439a`。以上证明部署身份及普通权限，不证明按住退出场景。

23:20:35 从应用自身显式按钮启动提权 Helper 77780，run_id `f20f67ef830f4e0b8aaaabda6877d12d`。普通客户端 peer_verified；所选唯一实例、接口祖先链、HidP 五键描述符、catalog 成员签名、固定模块、载荷 ACL、Frida 附加和固定 WDF 绑定通过。23:20:36 为 `bound source=pending_per_request contract=1`；没有真实报告状态，不能写成来源、吞键或按键 passed。收尾核对两个进程仍运行，管道没有再误判暂空关闭，正常日志持续可用。未取得全释放所有权时 owned=0，不改写报告；五键 LL 两边沿放行。没有生成输入或试探硬件。

当前为 **NEEDS_USER：实机操作等待，目标未完成**。用户不在电脑前且验收映射选择尚未回复，两份原配置的大小/修改时间保持不变；没有写模板。待用户回来先准备独立临时模板及空白测试文档、把焦点移离增强启动按钮，再做一次明确的准备按放以取得真实全释放报告；这次准备不能算正式首按通过。只有日志确认当前精确实例和 ready 后才开始正式三键/TV/Home、快速/重复/长按释放、真实闲置首用、实体键盘交替/同时、断连/睡眠、按住正常退出及受影响语音/输入锁定回归。上述全部实机项仍未执行；RC001 无硬件仍 deferred。

本轮产物核对：临时目录仅保留当前原生 Helper 构建输出、各针对性测试摘要/失败复现及最后构建日志，用于同一候选待验收和问题定位；没有新增重复包目录或全仓副本。旧同路径中间包已由最终包替换，无额外副本待清理。真实验收结束后按 LOGGING 回收不再需要的过程文件，保留交付包及必要冻结摘要；现有历史证据不动。HEAD 与 `test/capture-input-session` 仍为 `85fffc84d41c3bdaf822e49ab115e1d922c75e42`；Git index SHA256 仍为 `109eee75fdf888d7b4e656decf6f04f788b4ea18ee667b551eb8487a3d152469`，既有暂存内容未变，无 Git 写操作。

<a id="wdf-instance-property-result"></a>
### 2026-09-20 来源属性查询结果与停止收尾

用户明确可以测试后，正常退出旧应用与 Helper，仅追加独立记事本验收模板（返回/音量±/TV/Home → b/u/d/t/h）。全部原配置逐值比对保留，精确恢复件仅当前用户/SYSTEM 可访问，放于忽略的任务临时目录，不纳入证据正文。创建空白测试文档后，由用户点击正文并切英文；用户确认完成一次返回短按释放准备。09:56 一次读取见文档为空、记事本模板有命中记录，但来源拒绝只有 `source_property native=passthrough`，没有报告解析、all-up、ready 或映射。不能从该粗粒度日志判定完整 DOWN/UP 属于此次返回键；准备未通过，正式短按不计 passed。

仅对已定位函数核对本地 WDK 与微软 [WdfDeviceQueryPropertyEx](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/wdfdevice/nf-wdfdevice-wdfdevicequerypropertyex)、[PROPERTY_DATA](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/wdfdevice/ns-wdfdevice-_wdf_device_property_data) 和 [InstanceId 属性](https://learn.microsoft.com/en-us/windows-hardware/drivers/install/devpkey-device-instanceid)：槽 54、七参数签名、结构大小 24/指针偏移 8、LCID/Flags=0、属性 key、字符串类型 18 和字节长度契约吻合。产品拒绝现区分返回码、缓冲不足/溢出、类型、长度、终止符和实例比较，输出缓冲先初始化；失败不读部分字符串、不替换身份属性。每次配置/设备代次重新记录首次完整拒绝，避免旧去重掩盖新操作；不输出设备 ID、路径或属性内容。脚本 8 项（含真实脚本适配器 ABI/失败分支）、实际命名管道 2 项、fmt、前端和本地包构建均 exit 0；未重复无影响功能测试。

修订包时间 2026-09-20 10:08:31 +08:00，同一 NSIS 路径，SHA256 `8e1a6e5831dcb8006d13af99839c6b53b55cf89cb97203c415cd27433076115a`，替代昨日路径中的包。10:09:15 旧应用 62088 正常清理 passed、125 ms；Helper unload/detach/terminal 均 0 后安装 exit 0，配置文件字节未变。普通 Explorer 新实例 70128 的安装 EXE SHA256 `caafb63b84c7fa61adbe69b209080df2d6bac0e3cc09ccc9e2f525caa853969f`，只与构建 EXE 存在正常 NSIS 三字节标记差异；固定 Helper SHA256 `5d9294e3cc32d3ae5098502120c1f02d5c55a457bd02cc761c3031463ece42fb` 与构建一致。

10:09:44 Helper 68528 达到 bound，run_id `ea50f796c98548029d747eaa093df9b4`。**尚未发出第二次准备请求，启动核对即从自然到达的完成请求取得完整失败：** `reason=source_property_status property_status=0x80070002 property_type=0 property_bytes=0 source_relation=unverified native=passthrough`（10:10:07 等记录）。这是 HRESULT 风格的返回值；只证明当前所取 WDFDEVICE 的这次 InstanceId 查询没有成功，不是已取得字符串后比较失败。不能直接推断 API 普遍不支持、所有三键不可实现、缺少权限或 Microsoft 签名。来源未成立，不进入 usage 解析/修改，不能给该报告指定物理按键来源。

按已明确停止条件结束当前来源链，不继续加探针、换路线或请求重复按放。WDF device target/PnP 等其他关联仍是开发方尚未研究的候选，本轮未验证亦未排除；没有已知必须让用户提供的外部技术条件，不要求用户寻找方案。返回准备的来源 gate 为 failed；返回正式单击、音量±、TV/Home、快速/连续/长按释放、闲置首用、实体键盘交替/同时、断连/睡眠/按住退出及受影响语音/输入锁定回归均未执行或 deferred；RC001 无硬件仍 deferred。

10:12:53 应用 70128 正常清理 passed、128 ms，Helper script_unload/session_detach/terminal 均 code=0；session_detached reason=1 为主动分离。确认两个进程退出后，核对没有用户独立配置改动，只移除本次临时模板和记事本绑定，原配置文件**字节完全恢复**；受限恢复件已按规则送回收站。随后经真实 Explorer 启动普通应用 59368，版本 0.2.5、上述安装 SHA、TokenElevation=false；10:14:05 RC003 connected=true，增强未启动，Helper 进程数 0。基础语音没有进行新的实际验收，不以 connected 状态代替通过。

当前使用边界：已安装候选在 RC003 `enhanced=false` 时不仅移除返回/音量±执行能力，也从执行映射移除 TV/Home（`button_mapping.rs` 的 `apply_input_capabilities`）；保存配置不删除。五键 LL 两边沿放行、Helper 已停止，原生动作按代码策略放行；不能把恢复原配置说成恢复旧 TV/Home 映射行为。语音原实现保留，没有新增语音实机 passed。

产物继续复用 `target/dev/rc003-three-key/`，保留当前构建/针对性日志供该失败候选复核；没有新通用诊断工具或重复包副本。空白测试文档 0 字节，仍由用户可见记事本打开，暂留以免改变用户窗口；受限配置副本已回收。既有冻结证据和其他任务产物不动，后续关闭本主题后按 LOGGING 回收不再需要的构建过程文件。

本轮收尾再次只读核对：HEAD 与 `test/capture-input-session` 仍为 `85fffc84d41c3bdaf822e49ab115e1d922c75e42`，index SHA256 仍为 `109eee75fdf888d7b4e656decf6f04f788b4ea18ee667b551eb8487a3d152469`。没有暂存、提交、分支切换等 Git 写操作，未强杀、改安全设置或安装驱动；既有改动保留。

<a id="wdf-pdo-source-b"></a>
### 2026-09-20 A/B 定向来源验证与生产选型

用户随后明确授权 A（本 WDFDEVICE 注册接口）后 B（WDFDEVICE PDO → 公共 PnP 树），替代上一节停止实验的范围。A 静态前提未成立：固定 HidOverGatt 的 WdfFunctions 表引用及五处寄存器转移展开后，未发现槽27/28/29的本设备接口注册/启用/提取调用，未取得 GUID/reference string；未猜 GUID，不排除其他层接口。B 为 request→queue→device→WdfDeviceQueryProperty 槽31、六参数、DevicePropertyPhysicalDeviceObjectName=11、2048字节缓冲；最低UMDF2.0，PREFAST类型位不是运行时枚举。公开 CM_DRP_PHYSICAL_DEVICE_OBJECT_NAME=0xF 返回REG_SZ。此为请求设备PDO，与旧IOCTL file object名字不同；未使用KMDF-only IoTarget API。

10:51最小入口在Frida初始化前调用其GLib散列异常退出，未附加；已纠正顺序。首版B取到成功属性后因直接不等提前结束，遗漏唯一反查/其他设备分类，不能作为B全链失败。修正版补齐公共当前节点唯一反查和精确设备边界：功能节点/子节点，或恰为1812直接父节点且官方LE设备接口精确证明的远端物理节点；共享radio/总线/Root、仅共父节点、ContainerId或型号均拒绝。11项原生规则自验证exit0。重复请求聚合，不设短窗口；资源上限触发记incomplete，不判来源失败。

**B实机来源分类passed，正式映射仍未验收。** 独立B Helper SHA256 `d576c4756a9fde2936310bc1131491658968ce29e8a375f974fa8e2a9d35aa6f`，run_id `4b9c7edc270acf4ab67a3ca891cb500d`，11:11:11 bound。用户完成RC003返回/音量+/音量−各按放、实体键盘反引号/Shift+反引号/Home、RC003再返回按放，并确认键盘字符及Home正常。11:14:37经既有命名事件正常停止后一次读取：

```text
source_b_result object=1 generation=1 status=0 bytes=34 classification=other_device exact_matches=1 relation=5 current=true
source_b_result object=2 generation=1 status=0 bytes=34 classification=selected_function direct_match=true exact_matches=1 relation=1 current=true
source_b_summary requests=673 selected=8 other=665 unknown=0 generation=1 records=4 incomplete=false suppression=0 mapping=0
source_b_script_stopped
source_b_unload code=0
source_b_detach code=0
source_b_terminal code=0
```

公共查询皆status=0/type=1/34字节、格式有效且各唯一反查；8次为精确所选1812功能节点，665次为其他设备，说明池化宿主首条请求不能代表RC003。未读payload，计数不代表四对DOWN/UP已解析；用户键盘观察仅限零抑制阶段。父节点分支未实机命中，生产只采用实证的精确功能节点PDO。

生产最小候选已替换旧InstanceId gate：原生侧启动/PnP变化时重核选定节点当前PDO及唯一性；脚本每报告查PDO，同代次持有首次验证WDFDEVICE引用、拒绝新对象继承旧关联。来源失败不改报告，复用既有五键解析/抑制/映射。9项真实适配契约（对象替换、移除后旧PDO、重新全释放）、原生编译/fmt exit0；实机重连、抑制及映射仍未通过。此时安装版仍原PID59368，原配置未改、两个Helper退出，新生产修改待部署。B固定二进制/专用代码及有限静态输出暂留同主题目录供复核，接入收尾按LOGGING整理实验入口。

### 11:31生产候选与12:39跨接管释放修复

生产包11:31:00、SHA256 `af61590eb2a5e85f59c8c6c65a3973a7c57768d5203e04a41be2c59e1880fe55` 构建/安装exit0；旧59368托盘正常退出，overall failed_stages=0/97ms。新69596由Explorer8544启动、TokenElevation=false，安装App SHA `d6a9e25a5e169e08ad639ed94712a9de2bbda8270691cce19b066009d3c49339`，仅正常NSIS标记差异；固定Helper SHA `9a16d6797e7c471b5cca79bd93aa831ef370db8bbc26e0cae103ca141fc9a4ff`。正常退出后追加隔离Notepad五键b/u/d/t/h模板，原配置逐值保留，精确恢复件受限于用户/SYSTEM。Helper80152/run_id `bb7403519d2f4d8fa86d1c7016cf18e7` 于11:35:36 bound，公开PDO精确唯一预检与描述符/签名/模块/管道通过。

用户确认生产准备完成后一次核验：真实Report1为9字节，符合三个u16槽和零尾部；generation7有ready=true及physical=0释放，也有返回/音量状态经清除后发映射边沿。额外按键不能归为所请求的单次准备，空文档也不是准备失败依据；正式文字送达、模板命中及五键单次结果尚未验收。日志同时明确暴露跨接管错误：12:39:27.960原生Home DOWN进入引擎，12:39:28.184 Helper全释放后增强启用，但没有对应Home=false；后续map_cancel持续held_keys=1/waiting_release=1。原因是增强分支把所选RawInput旧UP也丢弃。

修复仅允许已通过RawInput精确设备路径且merger确实持有该来源旧DOWN的UP清理；孤立同VK UP不能清除DriverEdge hold，双来源重叠不产生重复UP。`cargo test -p sayall-windows host_takeover_ --lib` 两项真实合并逻辑用例通过（含32次快速交接），fmt检查exit0。另增加生产配置ack的mask/代次及state的all_up日志，仍复用原协议/执行器；语音/输入锁定未扩展。

12:56:11修订包SHA256 `ebb55f65546c6ed606d3dd7c82433157e87ffd2074b16ac5f37452ac9d461582` 安装exit0；旧应用69596正常退出overall passed/failed_stages=0/116ms，Helper卸载/分离/终态code=0。普通Explorer启动80448，安装App SHA `ea2f4ee57f73a6a56b13893a9dcfe3bbc87a4be81440e5020455b5a1877d120e`，Helper23860/run `b81d94f158cd443e983be66d27856b0d`。13:09:13.116来源已锁定的Home DOWN、13:09:13.356 UP，随后held_keys=0/waiting_release=0，精确PDO的9字节报告ready=true/all_up=true/physical=0；跨接管旧UP清理实机passed。此为准备操作，空测试文档符合预期，不计正式Home动作通过；其他来源不匹配原生放行。

正式组准备时发现另一具体缺陷：13:11:49至13:11:50前台切回Notepad产生mask0→31，configure无条件清ready导致下一首按会被放行。修复在同一有效来源/设备代次、最后已验证全报告释放且owned=0，并经既有映射线程barrier确认五键旧RawInput为空时保留同步；mask0仍持续解析每份目标报告但零新所有权/映射。配置ack后发送明确标记`configuration_reuse`的缓存释放状态来初始化客户端边沿解码器，不伪称新的实物报告。持有、未知、对象/连接代次与异常取消均不复用；其他来源不污染目标状态。13项脚本适配测试、1项真实映射线程barrier测试及fmt检查exit0；不增加定时等待或改语音路径。

13:24:37修订包SHA256 `8070ca15c91526e2fb20ce300a86c716eb079be6a16b29f7a43a119341290440` 构建/安装exit0（原位替换同一路径），原配置字节不变。旧80448正常退出overall passed/failed_stages=0/124ms，Helper卸载/分离/终态code0。新71512由Explorer8544启动，0.2.5、TokenElevation=false，安装App SHA `eb10ddceb937601938474856f384e78d9beaf95cae378d6a5405689aae787088`，仅正常NSIS标记差异；Helper SHA `b90a802f83f0c4f72c9e4f03bd6b9b165a2b15572bd4f610d3d0c0bd7aa65af9`。Helper81692/run `1327f6fb89b64672a5fe25f0d32796f4` 于13:26:55 bound，RC003已连、精确PDO公共唯一预检/描述符/签名/管道通过；13:27:35专用Notepad为空且已聚焦、profile_active=true、raw_release_known=true/raw_released=true、configuration1/mask31 ack成功。新Helper初始尚无物理报告，等待本实例首次Home按放准备；旧实例准备成功不充作新实例同步，正式budth及完整矩阵仍未验收。

用户确认本实例Home初始化完成后统一核对：13:36:20.837精确PDO目标Home DOWN，13:36:22.021旧RawInput UP，13:36:22.027真实all_up=true/physical=0；当时mask0，零抑制/映射。之后多次mask0↔31保留缓存释放；13:39:58实际切回Notepad为profile_active=true、configuration15/mask31 ack、configuration_reuse ready=true/all_up=true，raw释放barrier已确认，held/waiting_release均0。13:41:05前台74396、专用正文空且光标在末尾，正式budth组已就绪；初始化及配置复用状态passed，实际五键文字/原生抑制仍待用户本组结果，额外Ok日志不计本组验收。

13:41:05基线后的正式五键组，用户回复完成后一次核对：公开Notepad正文恰为`budth`（5字符），光标选择起止均等于文末。13:42:56 Notepad profile命中；同一run、generation19/configuration17/mask31，每份报告均为精确所选PDO、Report1长度9。

| 按键 | DOWN / UP（本地时间） | DOWN physical/suppressed/mapped | 实际配置动作 |
|---|---|---|---|
| 返回 | 13:43:00.536 / 13:43:00.797 | 1/1/1 | 一次Single B、注入ok、正文b：passed |
| 音量+ | 13:43:02.154 / 13:43:02.393 | 2/2/2 | 一次Single U、注入ok、正文u：passed |
| 音量− | 13:43:03.563 / 13:43:03.753 | 4/4/4 | 一次Single D、注入ok、正文d：passed |
| TV | 13:43:05.522 / 13:43:05.919 | 8/8/8 | 一次Single T、正文t无反引号：passed |
| Home | 13:43:07.277 / 13:43:07.496 | 16/16/16 | 一次Single H、正文h且光标留末尾：passed |

每个UP均回到all_up=true、physical/suppressed/mapped=0，并各产生一次引擎UP；13:43:23切回聊天held_keys=0/waiting_release=0。其他来源不匹配均明确native=passthrough。以上证明本组配置字符单次执行、报告抑制及TV/Home可观察单响应；音量OS提示/实际音量是否变化尚缺用户观察，不以抑制日志外推。真实快速/长按连发停止、启用后的键盘交替/同时、闲置、断连/睡眠/按住退出及语音/输入锁定仍待分组验收，不据本组标整个目标完成。

实际临时配置核对五键single=b/u/d/t/h、double/long均disabled；原执行器此时350ms后连发，返回间隔50ms、音量±100ms、TV/Home不连发。下一组无需改配置：快速bud两遍、三键各长按释放、键盘反引号/波浪/Home交替及Shift与TV并用，等待用户完成后统一核对；程序和正文未被修改。

用户随后反馈三键实际正常并确认长按连续输出。一次公开UI核对正文31字符，为`budbud`加10个b、8个u、7个d，光标在末尾。13:49:13–20六次短按各135–199ms、各一次Single；13:49:29.672返回持有至30.526（引擎854ms）共10次，31.275音量+至32.302（1027ms）共8次，32.949音量−至33.909（960ms）共7次。首次重复分别357/364/353ms，符合既有350ms起始契约；UP之前最后一次触发分别4/11/64ms，至13:49:48无任何脱离实际DOWN的映射。31次注入ok、0错误、0剩余hold，与31字符一致；后续held_keys/waiting_release=0。短按配对/单发、三键现有配置下按住连发与释放停止在此实测passed，不要求重复前两段。当前正文无反引号/波浪或TV/Home并用证据，第三段仍未确认；音量OS现象仍待用户观察。

为后续闲置首按仅核现有活动日志：最后五键释放13:49:33.909，但之后有语音活动，不能从前一按键或聊天经过估算。最后语音control_stop及active=false为13:56:01.835、正常流终态13:56:02.049；13:58:19.427核对此前无新的RC003按键或语音活动，输入闲置至少137秒且已全释放。此为下一次首按的准备证据，不证明遥控器硬件睡眠，也不把语音活动本身算成完整语音/锁定回归。后续仅待实体键盘反引号/波浪/Home交替及Shift/Home同时场景，首个遥控返回合并检验闲置首按；本轮无配置、设备或前台变更。

本候选未提交源码SHA256：`runtime.js=d8f8dde2c12c5fda2ae2fc08ed43b6907db2724a1fc6b3c5f0020d9c2242ea78`；`main.c=d48ead135ab849f803ceb3f6ada07019921b4d6096e7ea2f377dadb8eb0dd368`；`button_mapping.rs=878062d9a9c4375bad2759df1ee55c9f3407b7f5b35a4608aaa0a378bbb335fd`；`hid_host_windows.rs=ca3ddaed18c8e17d393db62a5e12095d24dede3a4c82a26c445ee9ff34454363`；`hid_host.rs=5128b7e735636235e8126c2a2b937c4b9e8bb0a6aec76627bc0a5f5bfe13a449`。散列用于关联当前工作区，不单凭散列宣称可重建。构建/13项脚本/1项barrier日志复用同主题target目录；受限原配置恢复件及B来源实验最小材料继续为未结束验收保留，收尾再依LOGGING回收，不清其他任务产物。

### 14:00闲置/键盘实测与14:10无线电恢复缺陷

仍为13:24包、App71512/Helper81692及run1327f6fb89b64672a5fe25f0d32796f4。最后语音正常终态13:56:02.049，首个返回DOWN14:00:44.360，中间无其他RC003活动，真实输入闲置282.311秒后一次b成功；不是遥控器硬件睡眠证明。该组公开正文为`hb'"th"T`：单/双引号U+0027/U+0022不能充作反引号/波浪。5次遥控动作均有精确PDO、配对抑制、Single和注入ok；用户确认Shift+TV大写T、实体Home与遥控Home并用结果符合预期，held/waiting_release=0。随后仅补真实键盘反引号/Shift+反引号，用户明确反馈`~与预期相同；14:07:47公开正文末行U+0060/U+007E，补测区间遥控映射/注入均0、无持有。产品没有非目标键盘VK/scan日志，故上述证据为用户物理操作与实际字符，不伪称捕获了0xC0。闲置首按、这些键盘交替/同时场景passed；音量OS提示/实际音量变化仍缺观察。

用户无可取电池，实际手动电脑蓝牙OFF/ON，记录为无线电重启测试，不冒充取电。14:10:02.904公共选定设备present=false，14:10:05.717连接丢失；14:10:10.855自动connected=true，14:10:11.541公开PDO重新唯一匹配、present=true。应用与Helper原进程未更换，无再次UAC；自动连接及来源重建passed。首次真实返回14:10:57.061物理DOWN=1但ready=false/suppressed=0/mapped=0，57.176释放才ready；故恢复后第一配置动作failed，不能以后续音量±/返回成功代替。没有再要求用户准备键来隐藏首按丢失。

两处确定缺陷进入最小修复：客户端四条cancel路径丢弃Edges.cancel返回的Driver UP；正常映射器回归实证旧Back取消后新一次仍仅计1次（应2）。统一先取消手势、再交付仅本Driver持有的UP，重复取消不单击、不清理重叠RawInput。Helper重连只在断连前有效全释放/零所有权/Raw释放屏障、同Helper持续覆盖完成点、新公共PDO及逐报告对象重新验证、该新对象无身份空窗原生放行记录时接管首DOWN；不发送虚构all-up。未知队列/报告/过多未知对象保守拒绝，持有断连先等真实释放；其他池化对象不能取消目标持有。客户端接受已验证且确实被抑制的首DOWN，取消清旧边沿，不复用旧正向设备关联。

该修复18项实际脚本适配测试、6项映射/跨接管定向测试、5项边沿及真实命名管道检查、fmt均exit0；18项中曾发现无效报告后旧all-up可能被复用，已补有效ready条件并复验通过。当前开始一次必要重包，尚未安装或重连实测，不能把这些软件结果记作硬件修复passed。睡眠、持有断连/正常退出与受影响语音/输入锁定回归仍待完成；临时模板和受限原配置恢复件继续保留。

14:36:53修订NSIS包（同一交付路径）SHA256 `1d4b6a79940c4b8fb999aa06b2617cb2c32c92ec4fd62f4b2e411bd5d404d3ef` 构建exit0。14:37:41原71512通过真实托盘正常退出，overall passed/failed_stages=0/123ms；Helper卸载、主动分离与终态code0后管理员覆盖安装exit0，原配置字节未变。新App81016于14:38:23由Explorer8544启动，安装路径仍为既有SayAll目录、0.2.5、TokenElevation=false；App SHA `a6d144c2b195922335de76396deca5f48e0c4a2d481a47da36898ca12c37843c` 与构建仅正常NSIS三字节差异，固定Helper SHA `3ceef8cffcfd1447447ad1bedc2a0b19b7c3786b40bb44941cf311b27b337823` 一致。

Helper74108/run `54dc5676e0134bb2a43578851e37aabe` 于14:38:44.901 bound，公共PDO当前唯一/五键描述符/catalog签名/固定模块/ACL/管道门禁通过。14:39:24.460自然取得精确来源真实9字节报告、all_up=true/physical=0；14:39:53 configuration3/mask31回执、configuration_reuse ready=true及Notepad profile_active=true，held/waiting_release=0。因此没有要求额外初始化键。测试文档仍原43字符，仅通过公开UI将正文聚焦，配置不变。基线14:40:03之后仅待用户复验radio OFF/ON恢复后的第一返回及音量±，预期新行bud；不在重连后插入准备键，等待用户完成统一读结果。新修复尚未实机passed。

本修订关键源码SHA256：`runtime.js=e2194a2e705023e032f5c882f93bbe3ab5b214362cae790d03c8b2cd9cdd5832`、`hid_host.rs=28539458135d4c6c4d25a92376e70d5e5007a3c7eeda46b4953cd43b6c4dd1e6`、`hid_host_windows.rs=8aeb1eed128babdf3263ac722b16e3fc444f154a0cf901dc18936c7f3aaf86db`、`button_mapping.rs=05e927f42b5ea369bd82995a7a2b777df993da8cfcdf4eb286fa21b53c83579d`；main.c仍为上述d48ead散列。构建和受影响检查继续复用同主题日志，未另存中间安装包；全部原配置/隔离模板/恢复件和必要B证据保留至剩余验收结束，无Git写入、强杀或安全设置改变。

### 14:41修订版恢复失败与控制链异常候选

用户反馈“没有字符输出，但上下左右正常”。统一核14:40:03基线后：测试文档仅从43变44字符（末尾空行，无bud），本组三键配置动作failed。14:41:45.223公共设备present=false，45.225出现`phase=script_error`、45.226进入`stopping`；此后配置5–9只有发送屏障，没有回执，没有增强state/映射/SendInput。14:41:54.064公开PDO仍重新唯一命中、present=true，故不是公共来源匹配失败，也不能把方向键原生可用当增强ready。

该已安装版丢弃了Frida错误description/line，原始异常类别和准确抛出行无法从现有记录还原；不得猜成Reference/Dereference ABI错误。已静态核本地固定UMDF2.15头：槽126/127、五参数均正确。确定的控制缺陷是任一device_reset异常会跳过末尾一次性recv重注册；原生stop只投递一次，随后无脚本接收器，永远等stopped。14:47:51仅执行一次应用正常退出：App81016 overall passed/failed_stages=0/105ms并结束；Helper74108等待10秒仍在，无卸载/分离终态。现有正常IPC stop/父进程退出均汇入同一已置位stopping分支，没有独立的维护停止事件或可重达的脚本接收器。未重复stop、强杀、操作宿主、关闭无线电或覆盖安装。

源码候选修复该控制缺陷：configure/device_reset/stop独立类型接收器，异常finally重注册；控制异常立即取消映射、拒绝新所有权，已抑制持有仍等UP后正常卸载。原生停止立即发客户端cancel且只投递一次stop；脚本异常仅记录白名单类别/代码行/操作阶段，不输出description、堆栈、地址、路径或设备标识。WDF Dereference若抛错不盲目重复调用；明确cleanupErrors和失败终态，不把未知部分执行当引用已释放。该行为是故障收敛和可定位性修复，**不是原始device_reset异常根因已查明或已修复**。

软件验证：实际runtime脚本适配器21项exit0（接收器按Frida一次性语义，含移除引用抛错、持有时配置异常后的独立stop、退出引用失败），直接包含生产main.c控制函数编译执行3项exit0（异常脱敏、一次停止及清理失败/宿主分离状态），客户端与实际命名管道5项exit0，fmt exit0。原生控制测试没有附加进程或设备访问；这些测试不能代替本机Frida异常恢复实测。只构建仓库内候选，不覆盖现有安装；旧Helper清理未完成前禁止安装验证。

为未解决原异常复现，单独保留刚失败的安装包`failed-1d4b6a79940c-setup.exe`，内容仍为1d4b6a79940c4b8fb999aa06b2617cb2c32c92ec4fd62f4b2e411bd5d404d3ef。它不是新交付候选；原异常定位及新候选实机通过后核引用并回收。未复制整仓或新增重复台账。临时验收模板、原配置受限恢复件和当前现场继续保留，等待后续受限恢复；没有用户签名/权限/技术资料缺口，当前现场约束是旧Helper既有正常停止链不可达。

15:00:10仓库内最新NSIS候选构建exit0，绝对路径为`D:\BaiduSyncdisk\01_Code\GIT\remote-mic-app-windows\target\release\bundle\nsis\无线麦 SayAll_0.2.5_x64-setup.exe`，26,332,043字节、SHA256 `d56ae04412f5dd2b5b6d4cad7f79b2a5e4491ab18f59be21dc0b717386fa3817`。**未安装、未启动、未做该候选实机验收。** 收尾一次核对App进程数0，旧Helper74108仍原创建时间14:38:43.347；实际安装EXE仍0.2.5/a6d144c2b195922335de76396deca5f48e0c4a2d481a47da36898ca12c37843c、Helper仍3ceef8cffcfd1447447ad1bedc2a0b19b7c3786b40bb44941cf311b27b337823，均不是新候选。旧实例成功撤钩前不覆盖这些文件。

新候选源码身份：`runtime.js=14b41625392780933177b0875baadcb93d7b22de6f0e90004e010cdb0b6d03c9`、`main.c=6967f57eac61d2323c6e511932d78a4035251e0e026c3c0f916d9f130554a2de`、`hid_host_windows.rs=126432760ab781c7eef6b0ec1dbb46bc2925e02c9c4027abe7ca44ed479f37d6`；取消UP及mapper部分仍为14:36候选所记散列。新增`lifecycle.test.c`只调用生产控制函数的软件测试，未创建通用探针；其编译/结果日志及21项/5项摘要继续留同主题target供本未完成故障复核。ATTRIBUTION已核，本轮无新增外部实现/依赖，不作无关修改。
收尾只读配置比对：从内存副本仅去除本次临时模板及Notepad绑定后，与受限原配置逐值一致（original_values_preserved=true）。当前磁盘仍保留临时模板1份/绑定1份，未作恢复写入；原文件精确恢复件继续保留，不对外输出配置内容或身份。
15:05:32按最终收尾授权恢复测试配置：重新读取最新文件，仅移除任务ID对应的临时模板1份及Notepad绑定1份；写入前校验文件未并发变化，写入后其余配置逐值不变，且与任务前原配置逐值一致。没有用旧备份整体覆盖，未接触Helper/宿主或重启应用；受限原配置恢复证据继续保留。该配置恢复不解除旧Helper未撤钩及新包未安装的阻塞。

### 15:22用户自行重启后安装控制修订候选

用户明确表示已自行重启并恢复目标；15:20一次只读核旧PID74108不存在、当前App/Helper均0，不能把电脑重启终止旧现场记成Helper正常撤钩。现有15:00包仍为d56ae04412f5dd2b5b6d4cad7f79b2a5e4491ab18f59be21dc0b717386fa3817，未重建或重跑同包测试。管理员覆盖既有目录exit0，原配置文件字节不变；重新读取最新配置仅追加独立BUDTH模板/Notepad绑定，其余值逐项一致，当前精确恢复件另留受限private-config目录，未覆盖上一恢复证据。

新App18460创建15:22:06.401，由真实Explorer11928启动，安装路径为既有SayAll目录、0.2.5、TokenElevation=false；安装App SHA256 `1aa4e98c0c0b85ed9045b7822f69a679ec3f8dca1a01f89dbedfa05c8a2d59e6` 与构建仅正常NSIS三字节标记差异，安装Helper SHA256 `992e4106ff25b2f8cff4ccb473ef1c926e520fe2439302f5e229ec9640cdbbd2` 与构建一致。应用自身按钮显式启动Helper2500（15:23:15.393），run `d29115eabf5340a0979d33cbc9146316` 于15:23:18.084 bound；RC003 connected=true，公共PDO当前唯一、五键描述符/catalog签名/固定模块/ACL/管道门禁通过。

Notepad31748的RC003-acceptance正文仍原44字符，仅通过公开UI聚焦；15:24:10 profile_active=true、configuration7/mask31 ack、旧RawInput为空。15:24:35核对新实例target state=0，尚无真实逐报告来源或all-up，不把公共PDO预检当报告ready。故只请求一次新实例返回按放初始化，用户完成后统一核对；之后radio复验后的第一bud不得插准备键。重启由用户自行执行，不授权后续自动重启/强杀；旧异常原因仍unknown，本包异常恢复与完整生命周期尚未实机通过。
用户确认15:24:35之后新实例准备完成后一次核对：同一App18460/Helper2500；15:24:51.695已有精确PDO的9字节真实全释放，随后15:25:31.671–32.298、38.855–39.002、41.935–42.049三次返回DOWN/UP均配对，最后physical/suppressed/mapped=0、ready/all_up=true。实际操作多于请求的一次准备，首段持有约627ms并触发现有重复，故不把该组算正式单击验收。15:26:06.901 configuration13/mask31 ack、configuration_reuse ready仍保留，RawInput屏障为空、held/waiting_release=0；无本run脚本异常或清理失败。公开正文末尾b加换行，光标在文末，不以准备中文字计正式动作通过。正式radio OFF/ON后直接第一bud组已交用户，禁止插准备键；等完成后按新present代次核第一DOWN及原生音量观察。


### 2026-09-20 15:29 重连引用异常定位与修正候选

已安装 d56ae04412f5 候选 run d29115eabf5340a0979d33cbc9146316、App18460/Helper2500。用户 radio OFF/ON 后反馈三键均无反应，方向键正常。15:29:05.047 public present=false，.052 明确 `control_failure operation=device_reference_release class=access_violation code_line=80`，即运行脚本 WdfObjectDereference 调用。`script_stopped cleanup_errors=1`；原生 script_unload=0、session_detach=0，15:29:05.640 terminal=1，Helper实际退出。防卡死正常卸载已生效，但 WDF 引用清理失败不能称整体 cleanup passed。App连接在15:29:08.226断开、15:29:12.653恢复；增强已退出，正文无本组三键字符，因此恢复首bud failed，无需用户重按或重启。

官方合同和本地UMDF2.15头确认槽126/127五参数正确；cleanup后外部消息不得继续访问对象。固定驱动 WdfDeviceCreate 属性+8 Cleanup=NULL、+16 Destroy=B6D0；同一初始化 C928/C93E 将 CE60 写入 PnP callback +48并由槽19注册，为 EvtDeviceReleaseHardware。该入口在正常/意外移除时仍持有效Device，早于对象销毁。候选 runtime.js 仅在这个固定回调对精确匹配的已持引用成对Dereference，公共PnP仅失效来源、等待该释放；通知先后及新present先到均不复用旧引用。正常退出先释放活引用再撤钩；其他设备回调不取消目标；短引用调用保持JS所有权事务原子。新增脱敏 reference_released 日志。24项真实脚本状态机测试 passed（含通知两种顺序、同地址复用、持有移除、其他设备和停止等待）；这证明受控模型中的实现，修订版重连及完整生命周期实机仍待安装后验证。日志留 target/dev/rc003-three-key/reference-lifecycle-tests.log，后续同主题收尾按LOGGING清理；旧失败实证和受限配置备份保留。


15:51:03.083最终修订包0.2.5 SHA256 `adfced53440f79fbe37f511bf2d7cd168ece168186a9e36369a3af0f798f9e31`，路径仍为仓库 `target/release/bundle/nsis/无线麦 SayAll_0.2.5_x64-setup.exe`。MSVC单个字面量限制通过现有嵌入脚本分段解决；实际Helper二进制内找到runtime.js完整字节加NUL的唯一匹配。最后日志白名单改动已通过增量构建（exit0）；格式检查与客户端5项测试exit0。过程日志 `reference-client-tests.log`、`build-reference-final.log` 留同一target/dev目录供本轮复验故障定位，非独立台账。

旧App18460于15:52:21正常退出overall passed/failed_stages=0/70ms；无存活Helper后管理员覆盖原安装目录exit0，配置前后哈希一致，临时BUDTH模板及其他用户值保留。15:53:09.316新App3488由真实Explorer11928启动，路径 `D:\Program Files\无线麦 SayAll\sayall-windows-app.exe`、0.2.5、TokenElevation=false；安装App SHA256 `b296e86812b522c854103730e0690ffdcc710c9add637e1c3e57cf7d78011a2b`，与构建exe仅NSIS的UNK→NSS标记差异。Helper SHA256 `989b87a09a8c2200ba00ac6dfd6df4a35d7b753e511108dd62f87f462c034cdb`与打包载荷一致。显式启动Helper35916/run `0b5835a2a27e4f72b6abd143b0f805a5`；公共当前精确PDO唯一1、描述符/签名/管道/附加passed，逐物理报告与all-up仍待用户。普通Notepad31748专用窗口正文末尾空行，15:55:59.003 profile命中后的mask31/config1 accepted，旧RawInput held/waiting_release0。已请求合并初始化→radio OFF/ON→无准备首bud组；初始化不计正式动作，按真实ReleaseHardware/present代次分段，收到用户完成后才核日志。当前等待实机，修订版恢复首按尚未passed。


### 2026-09-20 16:04 radio现场反馈与断开状态发布

adfced候选/run0b5835…合并组过程中用户截图反馈关闭蓝牙后仍显示已连接，未声明正式首bud完成。一次只读核对：16:04:35.914公开WinRT GetRadiosAsync返回唯一Bluetooth radio=On；后端最后connected=true，当前两者不矛盾。历史15:59:59.510已取得真实 `reference_released basis=release_hardware held=0`，无异常，Helper35916持续原实例。15:59:59.617公共节点消失、16:00:01.008后端断开；16:00:03.058节点恢复、04.159连接恢复。第二轮16:00:19.701节点消失，到29.720才发布connected=false，存在约10秒滞后；31.279节点恢复、34.277连接恢复。来源门禁继续有效，不因同宿主其他source_mismatch取消目标。此证据支持引用释放修正实机生效，不代表全部生命周期已通过。

15:59:47返回按放建立all-up，之后两次返回映射均发生在断连前。两轮恢复至16:01:08语音操作前无三键DOWN/映射；公开专用正文无新bud。正式恢复首bud保持pending，不因额外语音或正文旧字符算passed，也不让用户重做已证明短按/长按组。

确定的实现缺陷：ConnectionChanged(Disconnected)先同步invalidate_connection/close，结束后才发布Reconnecting；close在已断开链路上还等待audio/control两个远端CCCD None写，block_on无本地超时。前端每1秒读snapshot，清理期间后端仍为旧Ready。旧日志缺回调到达和逐项清理耗时，不能将全部10秒精确归于某次GATT写。修订仅在当前连接代次Disconnected分支先发布非Ready快照，再保持原语音按住/输入锁定/本地解绑和Close清理；已证物理断开时跳过远端CCCD，正常退出或主动断开且链路仍在时继续协议清理。新增callback、event_queue_ms、cleanup_completed和CCCD分支耗时供下一实机区分事件迟到与清理等待。7项BLE定向状态/取消/重连测试与fmt通过；修订待本地安装及实机验证，不调整轮询常量或扩大音频调查。


16:15:24.545最终0.2.5增量包 SHA256 `d6b1681b32b4f9bb64e3adc1122ddffa7065938f906777705f29cce7f04ad7f8`，仍在仓库 `target/release/bundle/nsis/无线麦 SayAll_0.2.5_x64-setup.exe`。7项BLE测试、加强Streaming初值的单项复验、fmt及构建均exit0；Helper保持989b87a09a8c2200ba00ac6dfd6df4a35d7b753e511108dd62f87f462c034cdb同字节。旧App3488正常退出；run0b5835… Helper35916实际script_cleanup=0、script_unload=0、session_detach=0、terminal=0并结束，完整正常清理passed（不是按住退出验收）。管理员覆盖exit0、配置哈希不变。真实Explorer11928启动App23124（16:16:12.257、普通权限、0.2.5），安装路径仍为既有Program Files目录，安装App SHA256 `0fd77b3f79ea49491bde7b35d94d9e47ef3b38f65e90d7ab514ec7629e340aac`，仅NSIS标记与构建exe不同。

显式Helper32796/run `18ead39437234105bf15f1b8969fc690`启动门禁通过；16:17:13.671自然真实选定PDO Report1/9字节已有all_up/ready=true，physical/suppressed/mapped0。16:19:11.305专用Notepad模板profile_active=true、scene_active=false；11.326配置12/mask31 accepted、configuration_reuse ready/all_up=true，确认返回测试窗口不需新初始化键。随后其他前台mask0为正常范围策略。用户下一组仅radio关闭时状态观察、恢复后无准备首bud和原生音量观察，等待反馈后一次核对；不把目前准备状态算恢复动作passed。临时模板/绑定和原配置继续保留用于未完实机，受限恢复备份未删除，未改无线电/强杀/重启/Git。


16:52用户报告当前三键无作用后的单次核对：App23124/Helper32796仍为原创建时间实例，安装App/Helper哈希分别仍0fd77b…/989b87…，无新脚本异常。16:50:42–59真实返回/音量±及TV/Home报告全部有精确selected_instance来源和配对释放，但当时前台未命中普通/语义模板，configuration23/mask0已确认，suppressed/mapped均0、没有映射或SendInput。本次只读当前前台为未绑定程序，不能倒推此前每刻具体窗口。配置核对：通用总开关true，但五个目标键均无动作条目；独立BUDTH single映射及Notepad绑定仍完整，跟随开关true。故本次具体不执行环节是有效配置mask0，不是来源失败或新控制异常；不将该解释等同三键完整验收通过，也未擅自给用户全局添加b/u/d。run18ead…启动以后没有radio设备移除/恢复事件，断开显示和恢复首bud组仍pending。已说明按原隔离模板在专用Notepad完成真实radio恢复首bud，不要求额外初始化或重复已过矩阵；本轮无代码、配置、设备或安装变更。

### 2026-09-20 17:05 d6b168 正式 radio 恢复三键

被测包/安装载荷仍为上述 d6b168，App23124/Helper32796 保持原创建时间，run `18ead39437234105bf15f1b8969fc690`，独立 Notepad BUDTH 模板未改。用户明确完成手动 radio OFF/ON，并反馈三键输入正常、口述为 bud。17:04:52.073 `reference_released basis=release_hardware held=0`，52.589 当前公共来源不存在/present=false；17:05:17.313 属性 status=0/type=1/bytes=34，17.332 `exact_matches=1 selected_function=true current=true`、present=true。无需重启 Helper 或再次 UAC；没有本组脚本异常、停止或清理失败。

17:05:24.693 configuration50/mask31 accepted。恢复后首个实际目标报告为17:05:42.939音量＋DOWN：`source=selected_instance report_basis=request length=9 ready=true physical=2 suppressed=2 mapped=2 reconnect_first=true prior_released=true native_gap=false coverage_known=true raw_released=true`；43.147真实全释放。45.257–45.362音量−、49.494–49.703返回分别配对。实际日志顺序为UDB，三键各一次map_fire、各一次map_inject=ok，均native_delivered=false；最终physical/suppressed/mapped=0、held/waiting_release=0。首目标前没有已匹配返回DOWN或ready=false目标报告，只有其他未匹配来源的放行记录，不将未知来源当返回。恢复首个实际目标不丢失及三键单次/释放passed；不把音量＋首按写成“首返回passed”。用户口述bud与日志顺序分别保留；此刻公开UIA确认专用窗口标题，但正文接口返回空，未得到可独立证明当前字符顺序的文本，不以此要求用户重跑。

UI观察：用户称关闭蓝牙时未立即显示重连、恢复后才显示，明确暂不关注；不猜休眠原因，不标及时显示passed，也不继续研究或修改。已有日志仅记录17:05:16.199收到Disconnected回调并同毫秒发布非Ready（event_queue_ms=0），真实断开分支跳过远端CCCD、5ms完成本地清理；不能据此把此前24秒解释成已定位的系统原因。系统音量条/OSD仍缺用户观察。下一组基线17:11:13+08，仅手动Windows睡眠/唤醒后无遥控准备的首bud及音量＋、−分开观察，等待用户完成后核系统电源事件与产品生命周期；尚未执行/通过。无代码、配置、设备、构建或安装变化，原配置恢复件和现有证据继续保留，无新临时产物或重复台账。

17:12用户说明PC仍有其他工作，无法睡眠，取消17:11:13操作请求；睡眠验收deferred，未执行自动睡眠/重启，不把此前等待算完成。改为17:15:25.843+08基线的最小非破坏组：专用Notepad中音量＋、−分开观察OSD/语音误触，随后单次短语音按住/释放、切回英文实体x。只读当前设置确认原左Ctrl+左Win默认和弦、CABLE Input播放、会话输入锁定enabled/目标CABLE Output均未改。最后产品实际默认输入恢复记录17:12:44.826为original_mask=7/target_mask=0；此为最近证据而非实时重新枚举。本组将以会话写前真实baseline判断是否实际切换，若原已同目标仅记零切换，不伪造切换恢复通过，也不改变用户其他任务输入设备。等待用户完成才统一核日志，当前无本组验收结论。

### 2026-09-20 17:17 音量隔离与受影响语音/输入锁定回归

同一d6b168/App23124/Helper32796，用户对17:15:25.843基线后的音量＋/−分别观察OSD/系统音量、短语音按放及英文x组明确“这些测试都通过了”。本组实际有额外按键，不按请求数量伪造实测次数：17:16:28–38共10次音量map_fire；17:17:10.699–12.031两次＋/−交替共4个精确来源DOWN/UP、4次单次注入，所有权2/4→0，voice/chord/capture_input事件0。该明确音量OS观察组的用户确认，加上逐报告抑制及零语音/锁定事件，支持音量原生现象/不误触语音锁定passed；不以＋/−终值相抵代替观察。

紧接音量段的实际短语音generation9：17:17:18.794控制START同毫秒进入语音/audio begin，21.210控制STOP同毫秒结束语音；18.968 chord_press=ok、21.299 chord_release=ok。此处保留原设备准备/和弦间隔，不声称零毫秒完成注入，也没有双击或长按门槛。写前18.799 baseline target_mask=0/original_mask=7，实际两次公开路由写把三角色切至target_mask=7，default_roles_confirmed；21.359真实回读restored target_mask=0/original_mask=7、21.362 restore passed。该项是实际非目标→目标→原值回归，不是零切换；用户确认语音正常及松开后英文x正常。额外generation6–11均有成对START/STOP、和弦释放和三角色恢复，不采集语音内容，不用它们外推睡眠/持有断连。原配置、85fffc84既有功能与测试不重跑、不改。

17:21:37.520按住退出验收已armed：仅当前App/Helper和Notepad创建代次、现有隔离配置哈希不变时，预取真实托盘退出命令并收起；一次性读取现有产品日志，等待专用Notepad内新的音量＋精确物理持有/所有权2和同次重复映射后，直接发已有正常退出，不切前台，不forcekill。用户任意时刻按住并在出现重复u后继续约3秒再松开，无短准备窗口。只为本组的编排脚本/result在target/dev/rc003-three-key/held-exit-acceptance.ps1及held-exit-result.log，非产品常驻机制/来源探针；当前只是armed、尚无退出验收结论。待本组完成核清理并归档独有结果后按LOGGING核引用回收该一次性脚本，未删除原配置恢复件。

### 2026-09-20 17:24 持有正常退出与模板提示范围

一次性会话64167实际exit0。17:24:23.232音量＋精确DOWN、physical/suppressed/mapped=2，23.241与23.594各一次u注入；23.678在同次持有358ms/2次映射后通过预取真实菜单ID发正常退出。请求前没有前台模板切换或UP。23.689 input_quiesce时held_keys=1/waiting_release=1，23.690 cancel_release=1且VolumeUp UP交付，23.741两者归零，此后无映射注入；23.814 App整体清理passed/failed_stages0/127ms。Helper script_cleanup/script_unload/session_detach/terminal均code0，原进程均消失、配置哈希不变。用户明确x正常；没有将他未明确描述的“字符停止”捏造成额外口述，停止证据来自最后注入和退出清理。该组持有正常退出passed，不是切系统窗口先取消映射后的假持有。当前仍未做持有断连，睡眠因用户其他工作deferred。

用户随后反馈有时字符不符并要求能看见当前模板；本退出区间的停止是已授权主动退出、无模板异常证据，不按猜测改切换逻辑。用户最终明确“仅切换时短暂显示，平时保持安静”。新增范围仅生效模板可见性：普通/语义/通用或无动作/停用摘要，同一既有状态锁取选择与名称，经映射引擎消费配置后确认，旧代次拒绝，相同摘要去重；复用scene-overlay短暂2.4秒提示，菜单优先、无焦点/提示鼠标穿透，不新增常驻模式、菜单多功能或持久配置。初始软件结果：通知/基线3项、受影响场景16项、Vue交互5项、前端类型构建passed；待最终候选安装及真实可视验证，不把代码/测试算已交付。

17:28:56通过真实Explorer11928恢复原d6b168普通App24700，0.2.5/TokenElevation=false、原App/Helper载荷分别0fd77b…/989b87…未变；17:29:43显式启动Helper39228。未触碰用户正文或原模板，未重建未受影响HID Helper。一次性持有退出脚本已自然结束，无遗留触发器；该脚本/result暂留用于本次独有边沿与正常退出证据复核，最终引用核对后回收脚本。模板提示第一包构建误取debug组件Helper哈希，未安装；最终构建必须使用bundle配置确切release组件载荷哈希，不把中间包当交付。

17:48:47.525短暂模板提示最终NSIS包0.2.5完成（build-mapping-notice-final.log exit0），SHA256 `d7dd42e3fe7e1449ff6466cdcaf8600180c7ca94b9ed59319f211f3c4bd4ceb5`，绝对路径`D:\BaiduSyncdisk\01_Code\GIT\remote-mic-app-windows\target\release\bundle\nsis\无线麦 SayAll_0.2.5_x64-setup.exe`。最终编译使用bundle声明的release组件Helper SHA `a2059e9824c6ae7d5baf32e30246ce61ace68a64bcb4518aff874f4a804376e4`；安装后该载荷与release文件哈希相同。HID Helper保持989b87…同字节。误参中间包未安装、未另留副本；最终包覆盖同一构建输出，原构建缓存职责不变。

原App24700正常退出，Helper39228/runbc5c5af3c88a4af78235d2486102376e实际script_cleanup/script_unload/session_detach/terminal均code0；管理员安装exit0、配置哈希不变。新App25704创建17:49:34.0339920，真实Explorer11928启动、TokenElevation=false、安装路径仍`D:\Program Files\无线麦 SayAll\sayall-windows-app.exe`，SHA `e0c12c1a9ebd5e8b6c66fdf6584b2f9f77595e70b08aade8744a20e429569bb2`，与构建仅NSIS正常标记差异。Helper35956创建17:50:28.5835080，由自身UI显式启动。新实例已自然取得精确来源全释放，17:52:08.634配置17/mask31回执及configuration_reuse ready/all_up true，physical/suppressed/mapped0，不需额外初始化按键。

生产已记录实际mapping_notice applied→scene_overlay notice→约2.4秒hidden；这只证明事件/窗口调用/自动收起链，不代替用户看到内容或焦点实测。一次受限只读可视校验尝试激活指定Notepad，被Windows前台规则拒绝，立即停止并恢复原前台，没有再次抢焦点/输入，也未产生截图。已请用户自然点击专用Notepad及原聊天/浏览器窗口，确认实际文字、短暂消失、不抢焦点/不挡点击；等用户完成后统一核本窄组，不提前读操作日志。菜单优先已由Vue/既有场景测试证明软件行为，实际菜单观察尚未执行。模板提示不是常驻条，未改用户菜单模式、模板绑定或语音策略，原RC003目标仍未全部完成。


### 2026-09-20 18:04 模板提示视口与单开关修订

用户“有”确认前版提示可见；生产17:54:29.775→32.189、35.231→37.652等notice/hidden约2.4秒成对，不能据此外推全部期间不抢焦点。随后用户截图明确显示“当前模板：Agent”外横灰条/双滚动条，原因是scene-overlay继承html/body/#app的min-width:940px而宿主420px。用户要求模板页可关闭该提示，覆盖先前无设置范围，仅增加mappingNoticeEnabled单布尔值。

修订使用独立透明且min-width:0的浮层根视口，卡片自然换行、按DOM完整内容高度请求宿主适配；不以省略号或藏滚动条掩盖裁字。原菜单使用自身有界列表，开关不隐藏菜单。配置事务只写提示偏好，默认true；关闭独立事件立即隐藏并清计时器，既有模板、绑定、按住状态及配置代次不变。Vue17项、Rust4项及真实SettingsStore保存→重新打开→恢复1项通过；尺寸用例曾抓出Vue响应式对象比较导致不请求高度，已修并复验通过。完整日志留target/dev/rc003-three-key/mapping-notice-switch-*，仅支撑本次修订/待实机，完成后按引用清理；没有新来源实验或HID修改。当前候选构建中，尚未将修订可视/持久化标passed，持有断连未armed、睡眠继续用户暂缓。


18:06:22.3000915+08最终包SHA256 `42b8e28e7fc289eb797b3654e1f2dbdd7f36fdd33b6a2cc2a98b6f961d4c6fc2`，同既有target/release/bundle/nsis/无线麦 SayAll_0.2.5_x64-setup.exe，build-mapping-notice-switch.log exit0。旧App25704/Helper35956正常退出，run5b851d9b564d489180408865a02176dc清理/卸载/分离/terminal0；安装exit0、配置安装前后哈希不变。新App41364创建18:07:07.6462711，Explorer父11928、TokenElevation=false、安装载荷仅标准NSIS标记差异；安装AppSHA256 `22c304f2875929a9b76ed39c9125b549efe143ac02295d08c103a1e1a9ea91c1`，两个Helper载荷未变。

真实模板页公开UI已操作提示关闭→实际文件false→恢复开启true；排除该新增字段后全部配置JSON相同，原模板/绑定及本任务临时配置保留。首张布局截图受采集进程DPI虚拟化裁错，不能作为完整布局passed；程序化切前台纠正截图被自动审批拒绝，未执行、不绕过。改为PrintWindow仅捕获本应用自身绘制面，无前台/窗口位置修改，会话86339等待用户任意时刻自然切窗。原Helper恢复启动按钮动作及补持续授权/固定哈希的一次正规重提均被自动审批拒绝（旧授权转述未被接受），故当前Helper0；已向用户说明增强五键暂不执行，原生按策略放行，基础语音独立。此为审批状态，非来源或签名技术失败；不通过其他通道启动。仍待截图实见，未执行持有断连/睡眠。


18:15后续：用户当前明确“确认，启动原限定 Helper”，同一正式审批动作获准；启动前保护发现Helper已存在，故没有再次点击。只读核Helper37596创建18:13:32.7205010，固定安装路径/载荷SHA989b87a0…、TokenElevation=true，run8963e2fb57a74bdc8166aa470e85bd08；唯一目标宿主/pipe peer_verified及configuration6 mask31 accepted=true。主App41364仍同创建时点、Explorer普通权限和已核最终载荷。当前source=pending_per_request，没有本代真实精确报告/all-up证据，不能称映射ready。用户此答是Helper授权，不当成自然切窗截图完成；只读捕获会话86339继续独立等待，不改配置、不自动按键。


18:13:10.7443455+08只读PrintWindow会话86339已自然捕获并exit0；后续查看525×85实图，圆角卡片无外横灰条/横竖滚动条，“当前窗口未配置映射”全字完整。该图只来自本应用绘制面、不含其他窗口工作内容，已保留为[mapping-notice-20260920.png](mapping-notice-20260920.png)，供本次布局修正实证；原DPI坐标错误截图不作证据。本项可视自验证passed，不冒称用户已回复完成、全部期间焦点无变化或菜单本体实机通过。模板页单开关实际关闭/保存/恢复开启及其余配置不变已有实测，当前用户要求的UI修订完成；其他按键目标不因此完成。临时构建/测试日志仅支持本修改及后续整体验收，最终可按本记录回收；唯一恢复配置件不动。

### 2026-09-20 18:30 持有断连与公共列表增长竞态

用户明确“允许，现在做这一次蓝牙断连测试”后，一次性控制器会话5706基线18:28:41.808、run8963e2fb57a74bdc8166aa470e85bd08，固定App41364/Helper37596/Notepad31748代次、原On单一无线电；仅在Notepad真实音量＋持有及重复映射后Off一次。初始真实目标先为音量＋18:30:28.657，ready=false，29.660全释放ready=true；没有把未出现的请求中返回键说成已执行。31.201新DOWN physical/suppressed/mapped=2，31.201/.560/.670/.778共4次map_fire及成功注入；31.718控制器请求Off，31.804确认实际Off。31.793映射cancel_release=1、ReleaseHardware引用释放held0；31.799最后在途注入完成并交付VolumeUp UP，之后零map_fire。没有前台先切换取消，持有取消/引用清理这一段passed。

控制器34.220实际恢复原On、attempt1、exit0；18:36只读再次确认唯一Bluetooth radio On。该一次Off许可已用尽，未重复arm/Off。32.321公共节点已消失status0x0d；36.424 PDO属性重新成功type1/bytes34，36.432公共绑定status0x1a/exact_matches0/currentfalse。官方头明确0x1a为CR_BUFFER_SMALL，说明Size与List间设备树增长，不能解读成真实完整枚举下零匹配。旧生产只响应通知读取一次，暂态失败发present=false后无重试；37.907 BLE已连接、Notepad mask31配置回执正常但没有增强报告/映射。用户反馈“三个按键好像失效”，因此恢复配置动作failed；方向键原生正常不作为增强就绪。没有script_error或用户UI开关改变配置的证据。

最小修订仅native/hid-host-helper/main.c：CR_BUFFER_SMALL重新取长度最多3次；仍变化时在同一既有设备通知链250ms重试，期间present=false/零接管，完整唯一且current重新证明后仅一次present=true。移除、重复PDO、对象替换继续拒绝；stop/hostgone停止重试。既有lifecycle.test.c直接编译调用生产函数，10项控制/列表竞态与拒绝用例passed、exit0（不附加进程/不调用设备API），包括连续两批列表增长后无需新通知自行恢复、停止撤销及其他节点不误认。Native新载荷SHA256 b6044e9baa48e9b562f51f056e9651a4d8f430e0a6723e6dcc8284873aba1336，构建exit0；整包正在构建，恢复实机未通过。控制器脚本/result及本次失败脱敏日志留target/dev/rc003-three-key支撑独有失败和复验，问题关闭后按LOGGING核引用回收，原配置恢复件不删；没有新的通用探针或重复来源实验。

18:47:12.4191424+08来源恢复修订包0.2.5完成，SHA256 `0700b858214c256ffa1c485f62ba6c3dd291f641adc197dda0118e290465a605`，绝对路径仍`D:\BaiduSyncdisk\01_Code\GIT\remote-mic-app-windows\target\release\bundle\nsis\无线麦 SayAll_0.2.5_x64-setup.exe`，source-refresh-build.log exit0。旧App41364正常清理、Helper37596/run8963…的script_cleanup/script_unload/session_detach/terminal均code0；安装exit0且button-mappings.json哈希不变。新App16928创建18:47:45.2609729、真实Explorer11928父进程、TokenElevation=false，实际安装路径`D:\Program Files\无线麦 SayAll\sayall-windows-app.exe`，App SHA256 `1e83b548fc2e81373b32a5cdb89b556de46b331649f8a93d7ce20cd42a5ad402`，与构建仅NSIS标准标记差异；固定新Native Helper载荷匹配b6044e9b…，显式按钮启动Helper24744创建18:48:17.3236765、TokenElevation=true、run d77b51b08035447cb1af64415a937c2b。

新代descriptor/签名/固定宿主/pipe门禁通过，18:48:17.419公共PDO exact_matches1/selected_function/current=true；18:48:28.078取得精确9字节物理0报告，初期rawReleased未知，不将其误称就绪。18:49:08后的真实配置barrier确认raw_release_known/raw_released=true；18:49:12.017 mask31 configuration2回执与configuration_reuse ready/all_up=true、physical/suppressed/mapped0，之后未绑定窗口mask0仍保存all_up；不需为前台切换重复初始化。只读公开桌面确认原Notepad31748同创建代次、验收窗口可见，临时模板和绑定仍启用、提示开关true。一次性验收脚本仅更新固定新代次，并允许既有已验证configuration_reuse全释放作为准备，真正触发仍须当前Notepad中的精确物理持有/所有权及两次成功映射；尚未armed或再次访问Radio.SetState，需新的当前一次Off影响许可。新版恢复实机仍pending，睡眠保持用户暂缓；不重跑其余已过且不受影响组。

### 2026-09-20 18:55 修订版持有断连恢复实机与收尾审计

被测当前包0700b858…、App16928/Helper24744/run d77b51b08035447cb1af64415a937c2b，用户重新明确“允许，复验这一次蓝牙断连”。一次性会话80595在18:53:45.143 armed，最终exit0、仅一次Off。真实持有从18:55:30.688 VolumeUp DOWN开始，physical/suppressed/mapped=2，30.691/31.054/31.162三次注入成功；31.147请求Off时当前Notepad且映射仍持有，31.214实际Off。31.242 cancel_release1/ReleaseHardware held0、31.243引擎UP，33.566第一次恢复原On成功；没有新的Off、没有Helper/App重启或UAC恢复操作。

31.441旧公共来源present=false；34.608全新完整枚举status0/exact_matches1/selected_function/current=true并present=true。本次没有CR_BUFFER_SMALL，不能称硬件重现了重试分支；该分支由上述10项生产函数用例覆盖。恢复generation17在39.479先收到真正request物理全释放/ready=true、raw_released=true，之后55.857才有新的VolumeUp DOWN，56.099UP；56.277–.424及56.519–.697另两次音量＋均各新DOWN/UP/单次注入。返回59.294–.461才出现，单次B/注入ok；不把请求顺序冒充实际“首返回”。取消至55.857之间map_fire为0，因此没有旧持有复活或无DOWN重复；最终held_keys/waiting_release0、脚本/注入异常0。用户明确“正常，同时看到了过程中蓝牙断开”，支持本组可观察恢复、停止和后续键盘结果；没有采集私人输入文本。持有断连取消、引用释放、唯一current重建及新代有效释放后的首个真实DOWN接管passed。

原验收审计：13:43五键各单次/TV与Home原生抑制、13:49快速三键与按配置连发/UP停止、14:00真实282.311秒闲置首按、实体Home与遥控Home及Shift+TV并用、14:07真实U+0060/U+007E独立键盘输入、17:05无持有重连首目标/三键单次、17:17音量OS无变化/零voice-lock及语音和弦/输入三角色实际切换恢复、17:24持有正常退出与完整撤钩，均有既有实证。后续UI仅摘要显示与开关，Native本次仅公共列表暂态恢复，不改变这些输入解析/抑制/映射/语音函数；不重跑无影响组，不把之前版本的全部字节称与当前相同。来源未知始终放行、当前run也有source_mismatch/native=passthrough记录，B来源实证与后续当前节点重建不接受共同宿主推断。

仍缺两项直接实机证据：实际反引号/Shift+反引号与遥控映射持有重叠（早期同时组实际用了单/双引号，真`/~补测未同时按遥控）；Windows睡眠/唤醒（用户PC有其他工作，明确暂缓）。因此暂保留独立Notepad BUDTH模板以完成最小无蓝牙补组，不标目标完成；RC001本轮无硬件，deferred。原配置恢复件核对无task模板/原Notepad绑定；当前只有一个task模板及其一个Notepad绑定，用户其他新增和mappingNoticeEnabled=true不动。之后恢复仅移除这两个自有条目，不拿旧整份备份覆盖。

本次唯一控制器结果留target/dev/rc003-three-key/source-refresh-held-result.log，旧失败held-disconnect-result.log与同目录构建/生产函数测试结果为故障对照、原位提炼于本节。控制器已自然结束，无长期监听或待执行Off；脚本在待验收完成后按绝对路径/引用核对移回收站，原配置恢复件和当前唯一交付包保留。没有Git写操作、强杀、睡眠/重启、安全设置变更或新增来源探针。

### 2026-09-20 19:01 最后键盘重叠补测与配置恢复

仍为0700b858候选、App16928/Helper24744。用户对指定“遥控音量＋真实持有期间，物理Esc下/数字1左的反引号及Shift+该键，随后释放”的补组明确“与设计一致都是正常的”。19:01:29.135–45.448精确来源VolumeUp DOWN/UP，physical/suppressed/mapped2→0，期间148次U映射与148次注入ok、其他按键映射0；最后一次45.355，UP以后零重复。最终held_keys/waiting_release0、脚本/注入异常0，另7次未知/其他来源明确native=passthrough。实际`/~在重叠期间正确出现及释放停止采用用户对本组明确问题的确认，不冒称本轮取得新UIA Unicode正文。结合14:07已直接读取的U+0060/U+007E及现有精确设备边界，本缺口passed。

截至该组，原RC003必需矩阵中所有非睡眠项均已有实际证据；唯一必需未执行项为用户明确暂缓的Windows睡眠/唤醒与按键释放恢复，不自动触发，不标整个目标完成。RC001本轮无实机，deferred。菜单多功能与Agent新绑定未获最终行为确认，没有实施或写入计划。

收尾仅移除本轮独立验收配置。首次正常托盘菜单在400ms内未就绪，未发退出/未写配置；同一入口唯一延长等待重试成功。App16928正常退出，run d77b51…的script_cleanup/script_unload/session_detach/terminal均code0。读取退出后最新JSON并核task模板仍为原五键BUDTH、绑定只对应Notepad；准备原子写时PowerShell空备份参数不适配，原文件未改变。核原配置哈希仍一致后完成替换：恰好删除模板1、Notepad绑定1，所有其他字段和数组元素逐项相同，用户新增及mappingNoticeEnabled=true完整保留；未用旧整份备份覆盖。受限mapping-before-final-restore.bin保留退出后的最新恢复依据，不公开私人配置。

真实Explorer11928恢复App41808，创建19:06:03.5297879、0.2.5/TokenElevation=false，安装路径仍D:\Program Files\无线麦 SayAll\sayall-windows-app.exe，实际SHA1e83b548…与当前包载荷仅NSIS标准标记差异。沿用户已明确的原限定Helper授权由现有按钮恢复Helper28700，创建19:06:45.6556816、路径及b6044e9b…固定载荷核对、TokenElevation=true，run c9845dd7edcc42f3a119f5eb46396531；descriptor/catalog/公共PDO exact1/current及bound门禁通过。此为新运行实例启动核验，不将pending_per_request外推新的物理验收。当前原配置恢复完成，Notepad不再被临时BUDTH接管；提示开关保持开启，不隐式启用Agent绑定或菜单模式。安装包最终复核SHA0700b858214c256ffa1c485f62ba6c3dd291f641adc197dda0118e290465a605未变，无额外构建。

一次性断连控制器会话已结束且两次各自许可均用尽；已过期的固定PID脚本held-disconnect-acceptance.ps1按本次明确路径核验后移回收站，避免留下可误执行的验收广播动作。旧失败held-disconnect-result.log、新通过source-refresh-held-result.log、当前构建/定向测试结果与受限恢复件保留用于本轮故障对照和未完成的睡眠验收，正式脱敏事实已在本文件可独立读取；睡眠闭合后再按引用清理，不清历史或其他任务产物。当前唯一交付包保留，无Git写入、强杀、自动睡眠/重启或新Off。

### 2026-09-20 21:13 睡眠验收恢复准备

用户明确“可以测试睡眠”，仅授权其手动真实睡眠，不授权自动睡眠/无线电/重启。原App41808/Helper28700载荷仍为0700b858包，临时模板/绑定已清空，旧Notepad已关闭；自动正常托盘菜单两次未取得后停止操作并由用户确认“已退出”。一次核验实际App/Helper均0，21:12:27 App overall passed/failed_stages0/138ms，Helper run c9845dd7edcc42f3a119f5eb46396531清理/卸载/分离/terminal均0。

从退出后最新配置仅追加原自有BUDTH模板1和Notepad绑定1，移除这两项后与写前JSON逐项一致；未回写旧整份配置。写前完整受限恢复依据为target/dev/rc003-three-key/private-config/mapping-before-sleep-acceptance.bin。创建同任务目录空白RC003-sleep-acceptance.txt，Explorer11928普通启动App43976（21:13:21.4467019、0.2.5/TokenElevation=false，实际安装路径和1e83b548…载荷核对），Notepad45756（21:13:21.6076170）。显式原按钮启动Helper28148（21:13:42.3101598、固定安装路径/b6044e9b…载荷、TokenElevation=true），run b0a86e0168754f28bd729bdbf6001529；描述符/catalog/公共PDO唯一current/pipe/config门禁通过，无额外构建或测试。

21:14:19.645取得新代精确9字节physical0报告，但ready/all_up仍false，RawInput配置屏障已确认释放；这是新实例初始接管准备，不能把source成功冒充ready。已请求一次合并人工组：睡前在专用正文英文/CapsOff返回按放一次初始化→键盘EndEnter→全部释放后手动Windows睡眠→非遥控唤醒→等连接、点击正文→不插任何遥控准备键，直接返回/音量＋/音量−/TV/Home各一次（budth），分别观察音量原生现象。无短截止，失败不补按。完成后核Windows Kernel-Power 42/107或ModernStandby506/507及Power-Troubleshooter1的实际睡眠/恢复事实，与产品代次/首目标/配对清理关联。当前仅准备完毕，睡眠未通过；本组只直接覆盖全释放状态的睡眠恢复，不外推映射持有/语音输入锁定在睡眠中的释放。其他非睡眠已过证据保持不重跑。

### 2026-09-20 21:18 空闲 Modern Standby 实测与剩余边界

同一0700b858包、App43976/Helper28148/run b0a86e…，Windows Kernel-Power 506于21:18:02.6358605进入Modern Standby、507于21:18:13.7543432退出，间隔11.118秒；未观察到本组42/107或Power-Troubleshooter1，不称S3或已证明深度驻留。本组没有新ReleaseHardware/来源对象代次，沿用仍有效的精确PDO来源，不能声称执行了来源重建。21:18:27.419 Notepad配置7/mask31回执与configuration_reuse确认ready/all_up、physical/suppressed/mapped均0。

唤醒后第一个实际目标为Home（21:18:38.437 DOWN、38.442 H注入、38.685 UP），不是返回；随后返回40.572/40.814、音量＋41.693/41.902、音量−42.485/42.713、TV43.327/43.558、Home46.881/47.079各成对，分别成功注入B/U/D/T/H。目标报告均9字节且来源精确；DOWN目标usage抑制与映射一致，UP归零，没有把初次Home当作未计入的准备动作。用户反馈字符与预期一致且系统音量未变化；日志实际H+BUDTH与用户总体正常观察分别保留。空闲睡眠恢复、首个真实目标Home及随后五键映射/配对passed；不外推映射持有或语音/输入锁定在睡眠中的取消。

用户另述睡前最初TXT没有按预期切模板，先将SayAll前置再点TXT后出现测试模板提示。21:16:59至21:17:01三次返回报告已收到且增强ready，但当时最后已发布的是WeChat语义绑定，未执行临时BUDTH；21:17:20.262才发布Generic/direct/profile_active，随后返回注入正常。现有日志无该失败时实际HWND/前台事件入队事实；代码已有启动和配置后刷新，实际身份取自GetForegroundWindow及可执行文件，不按TXT标题判定。不能据此在“聊天TXT预览/实际前台未变”和“事件未到达/处理迟到”之间归因；已请用户澄清是否独立记事本正文有光标，不要求重现，未改模板逻辑。

剩余必需睡眠验收分为映射持有取消，以及语音快捷键/输入锁定持有取消。一次性held-sleep-acceptance.ps1仅作为本机可审阅验收编排，固定本组App/Helper/Notepad创建时间、run与配置指纹；Mapped/Voice互斥，前者等精确来源真实音量＋持有并至少两次U注入，后者等真实capture三角色确认与chord DOWN。只调用一次公开SetSuspendState(false,false,false)，不改前台、radio或系统设置；命名互斥与CreateNew尝试标记防重入/重复调用。PowerShell AST零错、嵌入C#编译通过，未执行或arm，等待用户对具体自动睡眠影响的当前许可。该脚本留至这两项实证归档后回收，不作为生产能力或实机通过证据。

### 2026-09-21 持有睡眠失败与目标更新

被测安装仍为0700b858包。一次性Mapped控制器53599记录10:14:16.6852253请求SetSuspendState(false,false,false)，触发前精确来源physical/owned2且至少两次U注入；单次尝试标记已创建，未记录API返回或控制器完成。10:14:17.330应用发布suspended/disconnected，17.341取消UP1、引擎VolumeUp=false；最后应用日志17.425，没有完整退出终态，旧Helper未见本次ReleaseHardware/完整撤钩终态。该组failed，不能以映射取消UP局部成功当作持有睡眠恢复通过；Voice从未arm。

System事件：10:17:54.3307786 Kernel-General12为新启动；10:17:56.9748463 Kernel-Power41记录BugcheckCode159、参数1=3、SleepInProgress5、非长按电源；10:18:07.4314955 WER-SystemErrorReporting1001独立记录0x9F/3，同期EventLog6008/6005。没有本组计划重启1074或正常42/107配对；10:18:22/31的506/507位于新启动之后，不能当作此前请求成功恢复。官方0x9F/3只支持“设备阻塞电源IRP过久”，当前没有责任驱动栈，不能排除本Helper或归咎硬件。dump元数据读取权限拒绝，常见本机调试器入口未找到；这不证明dump不存在或所有读取均不可能。本轮不安装工具、下载符号或复现电源操作。

10:22新App26876、10:23新Helper29188与旧固定实例不同。旧53599已不存在，既有取消事件和互斥不存在；保留控制器脚本已加入无条件拒绝执行，全部未执行睡眠/休眠/关机/重启许可撤销。原脚本bcbeeffc…、唯一请求结果与attempt标记保留故障对照，不保留可继续arm的入口。

用户已更新目标：本轮交付为异常只读判定、增强启动恢复/真实状态、程序自动与菜单完整模板切换、真实applied短提示及正常安装/受影响验收；旧持有/语音睡眠缺项只保留风险，不作为本轮必须继续电源测试的门槛。增强及菜单开关默认关闭，增强仍需UAC且取消不自动重试。菜单短按完整模板、长按保留既有语义调节；auto开启时手动选择跨同程序窗口保持、真实程序改变后auto接管，auto关闭时当前运行内持续手动选择；单一提示开关不重配mapper。

实现自验证：scene_control定向19/19、Vue模板/组件/overlay22/22、SettingsStore重开持久化1/1、pnpm build均exit0。证明状态机、checkbox IPC与保存重开，不替代UAC/实体菜单/实际提示验收。当前仅源码候选，尚未安装本轮新包。日志复用target/dev/rc003-three-key/productization-*.log，保留至该候选验收收尾；异常唯一原日志/控制器结果为未解决故障证据，不按普通中间产物删除。无Git写操作或新来源实验。

本轮最终NSIS在2026-09-21 11:14:08.6168937 +08:00完成，56371 exit0（前一完整编译85596亦exit0，最后仅将纯手动模式反馈文字纳入增量）。唯一当前候选target/release/bundle/nsis/无线麦 SayAll_0.2.5_x64-setup.exe，26,353,144字节，SHA256 d5cb68f03912d18b559db7b0003ebd4cdc398dfdf464d8ba8ffec96091b185e8；release App SHA94f20e43c8c39559077e8ab75eff3784aab5d9edd79aed903ae8291b885e4e1d，原native Helper b6044e9b…未变。最终前端产物已包含关闭自动切换但保留手动选择的准确反馈。所有源码冻结后才取该包身份，不把前一中间包当最终候选。

安装前固定App26876/Helper29188仍为原创建代次。既有正常托盘入口单次调用未取得唯一tray_icon_app，因此没有发退出命令、没有安装或更改配置；主控已请求用户托盘正常退出，待明确回复后核完整清理再部署。不增加自动化探路、不强杀。当前最新只读配置仍有自有临时模板及绑定各1、跟随/提示true，新menu/auto-restore字段尚不存在（默认false）；用户其他新增保留，未以旧备份覆盖。

### 2026-09-21 产品化候选安装与原生设置自验

用户确认“已退出”后一次核App26876/Helper29188均0：11:15:40.836应用overall passed/failed_stages0/121ms；原run 2a49a8…的script_cleanup、script_unload、session_detach、terminal均code0。管理员覆盖安装d5cb68f0最终包exit0，两份用户配置安装前后SHA逐字节不变。已安装App SHA d8c5a11f0984cf883c5c1bd1ccd31a84909c08f1586bf840a67094f6086139f3与release载荷94f20e43…仅3字节NSS/UNK标准标记差异；native Helper仍b6044e9b…完全一致。

第一次受限Shell调用在取得Explorer桌面时AccessDenied，未启动应用；依既有明确普通启动授权通过正式桌面访问审批后，真实Explorer于11:25:12.8096547启动App9696，安装路径正确、0.2.5、TokenElevation=false。默认opt-in字段不存在→false，Helper0，无自动启动请求。该事实只验证默认关闭路径，不代替选中后的UAC取消/重开恢复。

公开UI Automation仅操作本应用页面，不激活或移动窗口：模板页“完整按键模板自动切换”“菜单键选择完整模板”“模板切换提示”均为原生CheckBox。逐个真实Toggle→读取实际保存配置→Toggle恢复，跟随true/菜单false/提示true全部恢复，其余JSON逐项不变。驱动页“启动时恢复三键增强”实际Off；未代用户勾选。新menu=false因真实设置保存而显式写入，不能将规范化后的文件hash变化误称其他设置丢失。11:31:42.940后台RC003 ready/connected=true；11:33:43基线交主控请求用户实际勾选并取消一次UAC。增强取消/成功、应用重开和实体菜单/提示等仍待实际反馈，不以这些控件自验替代。

### 2026-09-21 增强启动恢复实机与菜单对照准备

后续16:34实际菜单组failed：16:34:11.989 Menu/Single Handled，方向事件持续更新菜单；16:34:21.142与16:35:17.403 Ok/Single均Blocked/native_delivered=true，无template_selection requested，实际Back/VolumeUp/VolumeDown仍B/U/D，没有M。根因为菜单没有选中项滚动，以及无焦点窗口无法承接已原生交付的确认；不能直接绕过来源保护后让Enter发送给原聊天窗口。

修订仅完整模板交互窗口取得并核实实际foreground HWND，原生DOM输入与已抑制远端输入分开，真实UP/all-held-empty后才选中和关闭；失焦取消不抢回，配置取消和正常退出先排空本菜单原生持有。toast仍无焦点、鼠标穿透。没有HID/WDF/来源协议修改。scene_control 21/21、SceneOverlay 10/10、Tauri check exit0；实际映射引擎Keyboard与GateEdge两条确认路径均无Enter注入、释放后Back注入M，配置/失焦/退出取消与旧长Menu均通过。Edge隔离无界面渲染实际SFC 40项，scrollTop2275、最后Template39完整处于列表可视区，截图已人工视觉确认文字与底部说明完整；初次测试页Vue重复导入导致空壳，修同一导入后通过，不当产品故障。该证据不是RC003真机确认通过。过程文件menu-focus-*.log、menu-scroll-check.html/log/png及隔离浏览器目录留至本修订验收后按LOGGING回收，唯一旧失败日志继续保留。修订尚未安装。

17:19:47.6238738修订NSIS完成（26354686字节，SHA256 3c96471d7b45e972f090d0190d192f906fd4677b898dfe89be0c6a42da33c23a），构建6918 exit0，包含冻结后前端。17:17:38旧App42608正常overall passed/failed_stages0/164ms，原Helper41904四清理阶段全0；管理员覆盖安装exit0。真实Explorer启动App28804（17:21:17.9808113、普通）和产品opt-in自动Helper11944（17:21:19.4829853、提权），两者均为原安装目录；App已安装SHA e8881b126178b074cf6c0b70c1ffe2ae599cbbd3996d491ecd1c48da407ab857，release SHA 36705fe2d6ea6427d6fca2f4b99bba9da4848db80358ecf110b35f39f91ebb0f，native Helper仍b6044e9baa48e9b562f51f056e9651a4d8f430e0a6723e6dcc8284873aba1336。安装前后settings/mapping SHA分别DF8B145B…/3B65A0A7…逐字节相同，用户restore/follow/notice=true、临时menu=true保留，无配置改写。

新run736897a6b178403bb01dbce6618c7010的公开来源exact_matches1/current=true，17:22:40.406真实目标报告all_up/ready，mask31/raw_released=true。17:25:19.894准备同一自有B测试文档（长度7、CR2、末尾budbb），确认标题前星号是未保存标识，未访问私人正文；交主控End/Enter/s→Menu选MUDTH并核滚动→确认持约1秒释放→三键预期同一行smud。无需新Helper初始化。该组未获用户完成前不读按键结果，不将安装或浏览器布局通过当真实焦点/确认通过。

用户后续明确“反复试验几次没问题”，但首次主窗被带起、要求上下循环。一次统一核得17:26:59/17:27:57/17:28:09三次菜单真实foreground HWND均核实，三次Confirm在真实UP后选择并成功恢复目标，之后M/U/D及额外M/H逐次注入成功；受限正文为CR+shudmudmh，用户多次编辑，不能称严格原行smud或换行数量保持。Confirm实际约82/54/24ms，未执行1秒长持，仍待精确复验。实际滚动/短确认选择和MUDTH动作通过，不外推纯手动跨程序或真实双窗口。

主窗和菜单当前owner均0、主窗可见但非前台，历史没有主窗Z序记录；不能否定用户观察或宣称owner已证根因。固定Tao源码确认set_focus可能Alt回退，产品原close顺序先hide后restore；修订为非激活show后一次公开前台请求、真实UP后先restore成功再hide。恢复失败且仍菜单前台则保持可见错误提示，用户失焦不抢回；追加仅main/menu/other角色日志。仅Template上下首尾循环，空项/单项/首Up末Down/连续重复及旧菜单夹限边界通过；场景22/22、浮层10/10、真实生产restore_before_hide事务1/1均exit0。HID/WDF与配置未改，候选待新进程首次及长持确认验收。menu-cycle-*.log保留至该候选收尾；3c96471d旧包只作未解决首次主窗现象对照，不额外保存普通中间包。

用户实际勾选后报告“没有出现弹窗，直接勾选之后就好了”。设置restore_hid_enhancement=true已落盘；15:03:25.804仅一次restore_requested，15:03:26.4436198新Helper44020由App9696创建。App TokenElevation=false、Helper=true，原Helper已退出没有复用；run2618538…公共PDO唯一current与15:04:06.222真实全释放/ready均成立，公开UI实际显示“已就绪：三键增强已接管”。当前公开策略EnableLUA=1、ConsentPromptBehaviorAdmin=0、PromptOnSecureDesktop=0；[微软策略定义](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-gpsb/341747f5-6b5d-4d30-85fc-fa1cc04038d4)明确Admin=0为无需提示提权。没有修改这些值；UAC取消实机路径未发生，当前主机策略下deferred，不将未见弹窗当取消成功。

15:08:03 App9696正常退出overall passed/130ms，Helper全清理code0。真实Explorer重开App43156（15:09:27.7516242、普通），由产品读取opt-in后自动请求一次Helper42360（15:09:28.8979655、提权）。实际两个进程完整映像路径及App d8c5a11f…/native b6044e9b…已核，设置文件hash保持；没有另点启动按钮。新会话8bb7fbbc…公共来源唯一current/握手通过，尚无真实报告不外推新ready。选中后保存、应用正常重开自动启动及权限分离passed；没有Windows重启/电源操作。

为实体菜单验证实际切换，在第二次正常退出与8bb7fbbc…完整code0清理后，仅从最新配置克隆自有BUDTH为“RC003 菜单验收 MUDTH”（返回M，其余UDTH相同）并临时menu=true；新增模板1、关联0，移除新增模板并恢复menu=false后JSON逐项等于写前，原跟随/提示/增强opt-in true保留。原子替换第一次遇PowerShell null参数错误，旧配置未变；核对后用相同File.Replace真实null完成，没有重放退出或覆盖旧整份备份。写前受限恢复件mapping-before-productization-menu.bin仅供最终差量核对。

普通App42608（15:17:17.3896834）与自动Helper41904（15:17:18.7624349）已恢复，恰一次启动请求；run332d2abc918d4f4386ce3ef40b994cd5来源唯一current/握手通过但仍pending_per_request。两个自有空白A/B文档被本机Notepad开成同一窗口标签，实际只有B标题顶层HWND4331352/PID29304，不算双窗口准备或通过，不继续猜启动参数。15:18:59.797基线将“新Helper初始化一次→Menu选择MUDTH→mud→同模板重复选择不重复toast→聊天/Notepad切换恢复BUDTH→b及实体`/~”交主控；尚未用户完成，不提前判按键、自动优先级或提示passed。真正双窗口项保留待验，不以标签页替代。

### 2026-09-21 17:52 首次菜单与循环修订安装

92728构建exit0，最终NSIS于17:47:08.0565906完成，SHA256 ac9ee2253025ad893901ff7bf2ce724a0079ba64c4250c47d1eeee1c2d84ac9b。旧App28804于17:43:49.979正常退出overall failed_stages0/141ms，Helper11944原run736897a6…清理、卸载、分离、terminal全code0；管理员覆盖既有安装目录exit0，两份用户配置hash逐字节未变。真实Explorer启动App29796（17:51:59.4580933、普通）及产品opt-in自动Helper43936（17:52:01.0498157、提权）；实际安装App SHA fcd44f645ef919dcb914bd8ffe7949a8db2fd57af8e7e059230dc06f2a37f710，与release仅3字节UNK/NSS标准标记差异，native Helper仍b6044e9baa48e9b562f51f056e9651a4d8f430e0a6723e6dcc8284873aba1336。

主窗口经既有WM_CLOSE正常隐藏到托盘，IsWindowVisible=false，本进程未打开完整模板菜单。新run d74d7f65915e4ab8a1207867da6f00c8公共来源exact_matches1/current=true、descriptor/catalog/pipe/config通过，17:52:01.957 bound仍pending_per_request，无真实state，不以启动门禁冒充ready。自有B测试Notepad保持原实例。交主控准备一次返回按放初始化（不计验收），随后用户新行q→首次Menu→Up末MUDTH/Down首/Up末→确认持约1秒并观察菜单保持、UP关闭恢复→仅返回一次预期原行qm；无初始化Menu、无重复Vol±矩阵。待用户反馈统一核新角色日志、循环、真实持时/恢复与正文，不提前读取本组结果。
### 2026-09-21 Agent连续方向失效定位与修订

用户说明微信及Codex编辑框均存在“前一两次方向有效、之后停止”，且早于当前安装。当前App29796未有新的Left路由，不能将缺日志当无按键；已有最近失败段位于App28804。17:36:05.855–07.428 Codex七次Left均DOWN/UP成对：首项native_delivered=true原生交付，后六项captured且semantic_action_unavailable，无动作执行。17:36:02.185独立focus查询为Content，02.993能力查询却为Input；产品将前者区域与后者动作拼接，Left选FocusApplicationList而非MoveCursorLeft。没有菜单打开/释放残留证据。

微信17:35连续Left为Unknown→Disabled，并非同一错配；当前17:58微信仍Unknown、available3/blocked23，公开证据不足以称编辑区已识别。原识别配置汇总所有区域方向，Unknown仍捕获后禁用，造成原生第一项后停止。组合修订仅scene_control：用同一CapabilitySnapshot内focus与actions、丢弃过期代次观察；无交互面板且Unknown时不发布四方向语义捕获，由既有原生事件交付，不伪造Input、不补注入、不改LL/HID来源或其他未知动作保护。打开交互面板仍接管方向。此边界不宣称微信全部语义动作可用。

四项定向真实产品逻辑测试exit0：实际Keyboard首原生交付无重复语义动作，之后六个GateEdge触发正确Left；交替Left/Right、持有跨Input/Unknown排空UP、Unknown七次零补注入、菜单开关/回Input；旧观察不能覆盖新配置/目标；既有前台持有与语音取消；原生/捕获菜单真实映射路由。首轮夹具漏有效内建模板与订阅后发布，修同一夹具前置后通过，不作为产品失败。fmt exit0。保留semantic-*.log至本修订收尾，不新增诊断框架/来源实验；两应用连续方向及首次菜单qm仍待新候选实际验证。
18:11:17.2674411组合NSIS完成，95501 exit0，26350618字节，SHA256 03384859d7d3cd24a3df8998cfbbdb99e5c8bb1564f04d6a00eda67db50d8568。App29796/Helper43936于18:08:52正常退出，overall failed_stages0/149ms、原run d74d7f…cleanup/unload/detach/terminal全0。安装器exit0，实际既有安装目录App新SHA b30d182ec670918b5fdc4752d1552faa124e7c65a3654aa180ea84afd9a34eda，与release仅UNK/NSS标准3字节差异；native Helper仍b6044e9b…未变。settings/mappings安装前后SHA DF8B145B…/3B65A0A7…保持，未改任何绑定/偏好。

真实Explorer启动App38604（18:12:29.4125755、TokenElevation=false）和产品自动Helper40592（18:12:30.8007948、true），两者固定安装路径已核。main通过正常WM_CLOSE隐藏，visible=false，完整模板菜单本进程未打开。run b6e4653f0b5544ee8eccc53128e767f7公共来源unique/current、descriptor/pipe通过，18:12:31.707仍pending_per_request，不冒称逐报告ready。方向验证不依赖增强五键初始化；原绑定微信preset-chat、Codexpreset-agent且follow=true。

18:13:20.269基线交主控两应用独立空白未发送输入：abcdefghijkl末尾→遥控Left按放6次→键盘X应abcdefXghijkl→Right按放3次→键盘Y应abcdefXghiYjkl。不读私人正文、不新建程序绑定、不打开Menu，未收到完成前不提前读取本组日志。当前仅安装准备通过，两程序方向、首次菜单循环/长持确认及其余产品化验收仍待实际确认。
### 2026-09-21 Agent光标动作延迟与定向短路径

03384859包App38604/Helper40592的本组现有日志确认三条不同路径：TXT direct未映射方向、微信Unknown方向均原生交付，gate吞计数不增且无语义队列。Codex Input在18:26:48.936/49.086/49.236/49.416/49.581收到五个captured Left，DOWN→Handled均0–1ms；实际Performed依次49.717/50.446/51.201/51.947/52.733，累计约781/1360/1965/2531/3152ms，多数UP早已完成。因此不是长按/释放阈值，现有日志不能再拆出UIA与SendInput单独耗时，不把完整字符组记passed。

确定代码路径每个MoveCursor调用全probe_evidence，包含root.FindAll Descendants及列表/发送/滚动/缩放等全能力，之后才执行公开单键快捷键。修订仅四个MoveCursor：每次取当前焦点及原目标祖先链，核真实键盘焦点、可见/启用、编辑祖先、modal与归属；SendInput前再次比较当前焦点及原窗口token/场景取消代次。没有跨按键动作许可缓存、计数合并/丢弃、阈值修改或来源放宽，其他语义动作保留原全查询。结构日志限定cursor queue_ms、初始目标/代次、focus_ms、最终focus/guard、inject_ms与结果，无文本/窗口标题/路径。

4项定向测试通过：现有输入焦点能力边界；生产短路径事务28个有效方向逐个执行、Unknown/Content/IME/modal与最终guard失效均零注入、注入失败如实失败；真实scene worker在检查中取消后当前和排队旧动作零执行；既有实际引擎连续方向/Unknown原生/菜单相邻状态用例。fmt与cargo check exit0。此为逻辑/编译证据，实际Codex控件guard是否满足及冷/闲置后首用速度仍待新包真机。过程cursor-latency-*.log与03384859旧包只作本延迟对照，验收收尾后按引用回收；不新增通用探针或电源操作。

18:44:02最终0.2.5包SHA256 `56a00bd1a37cd49b64219601b6b56d27aa7566bae7c0cbe305de26fa1dd8053a`，26,352,748字节，release/NSIS会话90449 exit0。18:45:55通过既有真实托盘退出命令正常关闭App38604/Helper40592，app failed_stages=0、Helper cleanup/unload/detach/terminal均code0。管理员覆盖原安装目录exit0，安装主程序SHA256 `c8113b1ac50e5881abd0637f31c0cb4c67fa64ba17ea487407260216bb11ede9`，与release仅NSIS UNK→NSS三个字节差异；固定原生Helper仍`b6044e9baa48e9b562f51f056e9651a4d8f430e0a6723e6dcc8284873aba1336`。

真实Explorer普通启动App44260（18:47:26.7637541），用户opt-in自动启动提权Helper39204（18:47:28.0497670），run `0e0c7ece379641b2bb8247d03d46f5a1`。主窗口正常隐藏、首次交互Menu仍未打开，配置两个文件哈希与安装前完全相同。BLE已连接；Helper bound为pending_per_request，不能称来源ready。新实例没有方向热身或注入，性能验收须分别记录第一物理键（可能原生交付）及第一个真正semantic cursor perform，不把首个原生动作耗时当短路径优化证据。冷/闲置首用与连续字符计数仍待用户实际操作。

### 2026-09-22 统一直接映射前检查

用户明确不要第三方界面扫描，要求所有模板仅映射固定键或组合键；本轮先检查，不转换配置或移除入口。仍为56a00bd1/App44260。一次既有日志核对：09:55:52.407 Codex Input VolumeUp路由，53.226完成，约819ms；09:56:02.377 Backspace路由，03.109完成，约732ms。连续音量动作在同一语义worker积压，代码仍调用全probe_evidence后注入；现有日志不再细分这两次的UIA/注入耗时。09:55:51.496 MoveCursorRight经新短路径，51.517完成（focus17ms/guard1ms/inject2ms/total21ms），只证明该执行记录，用户未确认旧冷态完整字符组，不能记整体passed。

实际绑定：TXT的BUDTH为direct，Back/Volume＋/Volume−/TV/Home分别B/U/D/T/H，方向与确认未映射保留原生；微信preset-chat、Codex preset-agent为语义。两者输入区四方向为同名箭头、Back为Backspace、短确认为Shift+Enter、长确认为UIA发送、Home/TV分别寻找列表/输入框，当前volume模式覆盖音量键为系统Volume±；不同区域仍换义。菜单开启时短Menu由本应用模板面板接管，长Menu仍旧调节面板，不能与第三方扫描混为一谈。用户另有一个自定义语义副本，其输入区缺少四方向/TV/Home定义，不能套内置补齐。仅记录差异，不提交用户完整配置或私人绑定路径。

收敛方案是复用既有ButtonMappings与成对映射执行器，保留公开程序身份选择和本应用菜单导航/确认/焦点事务，退出第三方区域分类、能力扫描与语义动作队列。确认长按发送、聚焦列表/输入框、旧长Menu调节均没有跨应用固定键等价，待用户决定置空或指定键；暂未授权擅定这些转换值。未操作设备、未新探针/来源实验、未改代码或用户配置；全部电源/无线电禁止边界保持。

### 2026-09-22 固定映射与程序默认纵切（待部署）

用户随后明确授权删除全部第三方区域变义/扫描；此前“只检查、取值待定”的停止点已被取代。所有内置与用户模板统一ButtonMappings，普通键直接进入原映射执行器，程序监测只读取公开进程身份。自有Menu每次checkbox=false，临时选择同程序换窗保持、外部程序改变清除；checkbox=true固定打开菜单时程序，SettingsStore最新配置锁内合并单关联，成功才显示已更新，失败保留临时。不再启动区域能力扫描或语义动作工作线程，语音/HID/WDF未改。

定向自动验证：前端58项（目录、设置/关联、按键编辑、菜单控件/短提示），vue-tsc；Rust程序/菜单8项、生产目录JSON契约3项、真实映射引擎42次DOWN立即执行/UP不重复1项、配置事务21项，均通过。发现并修正菜单新代checkbox DOM未重置、普通模板编辑优先读取最新模板数据、导入v3安全校验仍留v2条件。保存失败仅默认事务不更新，临时选择仍有效。过程unified-direct-*.log复用target/dev目录，保留至本候选实机/收尾；已过时区域编辑器及专属面板6文件移入系统回收站。未改运行配置或设备。

转换将正常退出后读取最新配置，精确字节备份仅本机受限保存，写前复核未变化；现有4个直接模板均仅快捷键，无OpenApp项。用户语义副本按自身Input键转换，不套内置填方向；无固定等价长确认发送/Home/TV/旧长Menu置未配置。尚未安装或实机，旧source/voice等证据仅复用未改范围，旧冷态组不记passed；禁止一切电源/无线电操作。

10:54:26.1136467最终0.2.5 NSIS：SHA256 `b7187d9a4d2581956461abc26c4eaa840a27e68b37b846043edf50d410d20314`，26,257,895字节，80478 exit0；前端生产构建/fmt exit0。旧App44260于10:52:50真实托盘命令1001正常退出，overall failed_stages0/126ms，旧Helper39204/run0e0c7e…cleanup/unload/session_detach/terminal均code0，两实例0后才转换与安装。

一次转换脚本仅本机target/dev/rc003-three-key/convert-fixed-template-config.py，未打包/不自动运行。最新旧配置SHA3b65a0a7…写前复核未变，精确原字节保存在受限private-config/before-fixed-templates-20260922.bin（不入提交，保留至用户确认转换与回退需要关闭）；新SHA5c1f36ea…，仅转换原1个语义副本并合并4个直接模板及6条关联，所有不相关字段逐项相等。副本Input原缺项不填；原Send/菜单/调节三项置未配置，保留原volume系统键模式。settings SHA df8b145b…完全未变，auto增强true、follow/menu/notice true均原值。

管理员覆盖原D盘安装目录exit0。安装App SHA `92678e26a4e4b6440594dbb46dad288731570a5cbd40c1c7921a9d65ed99ff25`，与release `11b0f32f6766ec2d18603dedc9d3ea0e4ff2945155c67fe56d91c0a996c206ec`仅UNK→NSS三字节；component Helper `c2491a8f28ce50a592ad1780bda161eb165409bf12e8bdab765ce455fd604099`，固定HID Helper仍b6044e9b…未变。真实Explorer普通启动App5536（10:56:47.3624362、TokenElevation=false），产品原opt-in一次restore_requested后Helper49756（10:56:48.8232794、TokenElevation=true），实际路径均既有安装目录。

run e82cf627efa7483886bd8b9325d1934f公开唯一功能PDO/current、描述符/签名/管道通过；当前bound仍pending_per_request，无真实物理报告/全释放，不能标ready。启动程序监测记录identity=public_process/ui_query=false；没有旧区域能力扫描/语义队列入口。主窗口通过正常WM_CLOSE隐藏，首次Menu保留。独立Notepad旧进程无可见主窗，一次Explorer打开自有文档未得到可见窗口证明，没有继续争抢前台或更改正文；首组建议用户当前Codex空白编辑区先一次返回仅初始化，随后固定Left/Back动作实测。尚未取得新契约实机结果，不运行旧冷态组或任何电源/无线电。

### 2026-09-22 模板切换规则局部布局与反馈

本轮用户仅调整本块复选框布局，不改变默认值、开关含义、持久化、Menu行为或模板映射。旧TemplatesPage以justify-between将复选框放右侧并重复名称；修订为三个原生label，两列18px/minmax(0,1fr)，名称/常驻解释同文字列，整行点击、focus-visible保留，允许文字换行。10项既有模板页测试通过。第一次headless Edge退出0但无DOM/截图，未视为可视通过，未提权重放或扩大工具排障。

11:24:52 fa4076f2包正常安装后，App39720普通/Helper43068提权、实际路径既有D盘安装目录。通过本应用公开UIA进入模板页，不激活/移动窗口；三个checkbox屏幕left均827，说明全可见。PrintWindow只保存本应用卡片裁图，无第三方内容。用户随后截图确认此布局，同时报告点击开关导致页面上移下移：run()保存开始清status、成功插入顶部operation-message，是确定原因，不是用户模板异常。

只给三个开关新增窄saveRule路径：成功只应用已持久化配置，不清写全局status/error；失败在规则块固定2.8em反馈区显示，失败时checkbox保持旧持久值，重试成功后清错误但不改变反馈区几何。模板创建/重命名等原反馈不改。11项模板页测试passed，覆盖成功状态不变、真实异步保存失败/重试、持久勾选值与局部反馈节点不替换；未重跑后端/HID。被测TemplatesPage.vue SHA256 fe7af20ca830276a58da4d515d01ff74077f3922b393f3f73b40b11e8a5894bb，测试文件25e527fc51c979bfa3953b06df263c0b54335c04d23800175eec5d0f4477c498。

11:33:13旧App39720经已验证真实托盘1001正常退出，overall passed/failed_stages0/151ms；Helper43068/run ca9024b2bd894c0f80818fe391c8bf73 的cleanup/unload/session_detach/terminal均code0。用户这期间实际操作后配置SHA变为ef88f857f7a130ef88153292d7ed429c6f6db7ad9451e9524c20e25e3aa9b2c3，作为最新基线保留，不回写旧值；settings仍df8b145b…。全部电源/无线电禁止；旧未回复的固定按键实测组暂停、不记passed。

用户在ec164ed5（11:35:41最终包；App47560/Helper44612）继续报告规则块、刷新/添加按钮闪烁。确认是新版，根因为三开关仍置共享saving，使busy传播到整页disabled，再命中button:disabled opacity .55与mode-row:has(input:disabled) opacity .65。固定顶部高度只能解决位移，不能解决本次淡出，前述稳定性仍failed。

最终窄修订只将三个字段分别pending；不触发共享saving、不改其他控件disabled；本项aria-busy/aria-disabled表达等待，同项原生change在任何early return前同步恢复持久值，重复点击/Space不会留下假勾选。成功回包只合并该字段，不把完整旧配置覆盖其他字段；每字段失败独立保留，重试成功清该项。13项现有模板页定向验证passed，明确覆盖pending刷新/添加/其他checkbox未禁用与节点不替换、重复仅一次请求、两个字段逆序回包、失败恢复；不把jsdom结果冒充真实中间帧视觉通过。最终SFC SHA edd112ec7670bd7249de433cb83e34ef27d7f67cfcf14367091cc9c50ebfcc73，测试5397387ccaf10c2145045e3ec5087ee1aeb1f49b9dcedd05225b80b2beb0a67a。

11:42:04原47560/44612再次经托盘1001正常退出，app整体passed/0失败/130ms，run5e7c5772… Helper cleanup/unload/session_detach/terminal code0。保持最新配置ef88f857…及settings df8b145b…；无按键、电源、无线电操作。

11:44:17.6333139最终0.2.5包SHA256 `1466c7a27677f762aa67389e4b37da94b57a88a22e18feaea197ff1739954ccd`，26,264,924字节，66574 exit0，包含13项验证后的最终SFC。正常管理员覆盖既有D盘目录exit0，最新配置ef88f857…及settings df8b145b…精确字节均未变化。安装App SHA `12df92b3fe6c69083e3ae14ac002b2930896dec81fb8ad59eae87ba658336fca`；component c2491a8f…与固定HID b6044e9b…不变。真实Explorer启动App34632（11:45:34.0605320，TokenElevation=false）、自动Helper48612（11:45:35.3317086，TokenElevation=true），两者实际路径核为既有安装目录。

经授权拟仅将提示开关往返一次自验证并差量保护用户修改；一次公开UIA检查未取得本应用模板导航，因此在任何开关操作前停止。没有切换/恢复配置，没有抢前台或换检查路线。最终真实pending中间帧无闪烁仍待用户自然操作观察，不能记passed；此前用户截图及本应用PrintWindow仅证明左列布局/文字完整，非最终pending行为。过程日志template-rules-layout-tests/release及最小历史布局裁图保留至当前UI验收关闭；失效的本地headless页面/空输出仅属未成功的验证载体，不作产品证据，Vite已正常中断。未重跑后端/HID、未执行旧待回按键组、电源或无线电；无Git写入。
安装载荷补核完成：release SHA `122b167da11ddcd9e357483078d34839b057e773415a5602f12fd4093f8d1e00` 与上述installed SHA等长，仅NSIS构建标记UNK→NSS三字节不同，其他字节全部一致。

### 2026-09-22 模板页排序与独立折叠

用户新增局部要求，顺序调整为模板切换规则→完整按键模板→程序关联。复用项目ConnectionPage已有原生details/summary模式，无依赖/通用组件/新设置：两个列表首次open，互相独立，summary只含原生箭头、标题与实际catalog/关联数量；全部操作按钮在内容内，折叠内容不卸载组件，编辑对话框与草稿仍由原组件管理。刷新和单字段开关保存不重建details、不重新写open；页面生命周期内保留选择，不修改持久配置。

TemplatesPage 15项与ApplicationTemplateBindings 12项passed；新增用例实际summary click切换open，证明默认/独立性、刷新/开关保存后仍折叠、组件实例ID与DOM节点保持、添加/编辑不误折叠和草稿保留。首轮只因Vue测试包装代理Object.is比较失真失败（实际uid相同），改核真实组件uid后通过；原关联12项未重复。原生Enter/Space由浏览器实现，未将jsdom鼠标结果外推为安装版键盘/可视验收。上轮用户尚未确认最终闪烁结果，旧按键实机也未回，继续pending。

14:47:01已确认原App34632/Helper48612同代，真实托盘1001正常退出；app整体passed/failed_stages0/143ms，run cd706b57… cleanup/unload/session_detach/terminal均code0。配置ef88f857…与settings df8b145b…保留；无设备、电源或无线电操作。最终前端/增量包构建进行，后端/HID未重测。

14:50:32.2910878最终0.2.5包SHA256 `ab9aa895f620fe4d8138fe5883a86899f94f9d7f6385dea972cb535b4d834f45`，26,263,205字节，44092 exit0；前端类型/生产构建及Rust增量编译通过。安装exit0，原配置ef88f857…/settings df8b145b…字节未变。Explorer新App50016（14:51:34.6071935，TokenElevation=false）/原opt-in Helper17304（14:51:36.1757882，TokenElevation=true）实际路径均既有安装目录。installed App SHA `92175edb7bfe5daf4754570220cbf1d7bc5ff6f17d1f87556915f242b4cd2af9`，与release `32cdc49f91bd885ef434dfa6640c0fc91584e15ffb196511adf5a3365f4fb165`等长，仅UNK→NSS三字节标记不同；component/HID Helper分别仍c2491a8f…/b6044e9b…未改。

本轮源身份：TemplatesPage.vue `8f6fa580e945f873ae6d5ad15d1950a50eb93dfb0a755e39d8a6709d52ce9633`，ApplicationTemplateBindings.vue `a77673281b4c1f0fda90ceea0931f47e6557f81fada6517237115c7ba88c2982`，styles.css `09b3e1a66d0a090d6b8195494d947d263fb13f5e59a91b81f0655f8d03562bde`。复用前两轮自身UI导航不可访问事实，本轮不再重复UIA获取/换截图路线；实际排序/数量/原生折叠和上轮闪烁统一待用户自然打开页面观察，不把测试或新进程存在当实机passed。未点击任何checkbox、未操作遥控/第三方前台。保留template-sections-tests/recheck/release日志至本UI观察关闭，只有当前交付NSIS，未额外复制包/台账或清理历史产物。

### 2026-09-22 面板内Menu长按切换默认保存意图

用户要求仅完整模板面板已打开时用遥控长按Menu切换checkbox。实现复用现有GestureRecognizer的550ms阈值；仅面板交互recognition增加Long标记，用户ButtonMappings不改。首次打开的Menu DOWN无面板代次，不能同时反转；新的面板内DOWN记录当前代次，Long在当前焦点/目标、仍持有且未消费时只切一次；重复DOWN不重置消费，UP静默，短Menu仍取消。关闭/失焦/配置、断连和退出取消使旧标记无效。鼠标及Tab+空格走同一意图状态；确认全释放才走既有原程序默认持久化，意图变化不重配有效映射、不写配置。

定向验证：`cargo test -p sayall-windows scene_control::tests --lib` 13/13 passed、`pnpm test src/components/SceneOverlay.test.ts` 13/13 passed、`cargo fmt --all -- --check` exit0。测试通过真实recognizer→edge callback→gesture route，覆盖打开那次按键排除、长按一次/保持/重复DOWN/UP静默、连续两次反转、短Menu取消、旧代/目标/焦点/退出、控制和遥控一致以及确认后才保存；Vue验证远端快照更新同一checkbox且不回写。没有重跑HID、注入或语音，不能代替本次实体长按/焦点观察。源SHA256：scene_control.rs `92cd4fd97e1634668f5a3d266f10354b40c894976ab7a0f067fce5260ce33655`；SceneOverlay.vue `0c65e56cb1e66aed1bc3652448e6045c863ae0293f5d42016729c102bedb27d4`；SceneOverlay.test.ts `ba81d6dfc49c268bf58f8020ecd998797e1c98246c0b0e4c95b024a4af2106a1`。

16:22:13旧App50016经真实托盘退出1001正常清理，overall passed/failed_stages0/135ms；Helper17304/run b7b23a7e456547779ab2a16d40e475f8 的script_cleanup/script_unload/session_detach/terminal均code0。当前配置button-mappings SHA ef88f857…、settings df8b145b…保持最新用户字节；本轮无配置写入、无遥控/电源/无线电操作或Git写入。过程日志menu-default-hold-rust/vue/release保留至本次实体验收关闭；只生成当前交付包，不复制台账/历史产物。

最终0.2.5包 `target/release/bundle/nsis/无线麦 SayAll_0.2.5_x64-setup.exe`，2026-09-22T16:25:09.2127282+08:00，26,267,069字节，SHA256 `daa336fb79f158e3702375d731a1322964d1b07297e67bec4507edc9650e3f83`；构建98764/管理员原目录覆盖均exit0。release App `972fbc16a26497693d55a859e48721fabe31d114efc332c14b6202edd12e2ca9`，installed `bc09fcd0c705df29e5e4d53cd3ae0465555a6810571245704ab45c10a530806e`，逐字节比对仅UNK→NSS三字节构建标记差异；HID b6044e9b…、组件c2491a8f…未改。配置button-mappings与settings完整SHA分别 `ef88f857f7a130ef88153292d7ed429c6f6db7ad9451e9524c20e25e3aa9b2c3` / `df8b145b5048b300b5202c2e92b0fb9e7e162d0deae49670a606e10413d2a872`，安装前后字节完全一致。

真实Explorer启动App6896（16:27:33.8530517，TokenElevation=false）及用户opt-in自动恢复Helper51216（16:27:35.7727742，TokenElevation=true），实际映像路径均在既有D盘安装目录。新run `68440e97ba2c465fa284947d64040cda` 已bound，RC003连接/输入上下文applied，但仅source=pending_per_request，当前无真实全释放证据；来源未知拒绝保持原生放行。Menu功能及Back取消实体结果尚待用户，首次非Menu按放如需用于新实例初始化不能计为功能通过。未主动打开面板/截图/抢前台，旧模板页折叠与闪烁及尚未返回固定按键组仍pending，不从本轮26项代码测试外推。

### 2026-09-22 选项记忆：用户替代每次不勾选契约

用户反馈“重开丢勾选”并要求设置均记录。原open_panel显式update_default=false，属于原设计被用户替代，非偶发文件丢失。此前16:32:00.797/16:32:09.873在不同面板代13/14各一次remote_long→true，随后UP long_consumed=true；不能合并成同一面板true→false全组passed。该组还出现foreground_verified=false及target_restored=false，保留为实际失败/待定位，不为本轮偏好修改展开新的窗口调查。

有界设置盘点：模板规则三个开关、增强自动恢复、语音输入设备锁定/目标、主题与预览更新、音频端点/遥控器、按键及语音快捷键已有持久化入口；本轮不重复设备测试。缺口为Menu保存默认意图、按键页“锁定当前按键”（编辑选择锁，非语音输入设备锁）、完整模板与程序关联折叠。编辑目标select、导入勾选/搜索、未保存动作草稿是当前编辑/操作，不改成自动保存；使用说明展开仅临时阅读，不新增设置。

新增MappingConfiguration.menuUpdateDefault及AppSettings.ui_preferences内lockButtonSelection/templatesExpanded/associationsExpanded（首次三项均true）。Menu鼠标/TabSpace/遥控Long走同一偏好事件及既有设置工作线程，SettingsStore锁内加载最新配置只合并字段；不重配mapper/取消长按/更改程序关联。取消只取消模板选择，已接受的偏好仍落盘；确认才改默认关联。保存中阻止同项重复和确认，失败恢复已保存值并显示错误；迟到旧请求不写新代面板，旧IPC失败也不在新面板显示。偏好待保存时正常退出等待已有任务，完成后继续既有清理。完整映射编辑旧快照不拥有该偏好，不能覆盖它。

三个UI偏好复用一个仅限定这三字段的读取/保存模块与SettingsStore单字段接口，无新开关/全页busy/自动保存框架；details首次挂载/加载默认和程序设置open的toggle事件不写回，保存失败恢复旧值并就地说明。字段分别保存，不覆盖其他模板、关联或设置。

验证：场景16项passed（含取消后保存/新controller恢复、失败/过期回包、Long配对及退出同时等待物理UP与偏好落盘）；SettingsStore并发字段合并/保留新模板修改/存储重开/无效写不应用1项passed；受影响前端69项passed（模板15、关联12、按键编辑23、浮层15、UI偏好4），其中一个载体漏写input type=checkbox已纠正；最后新增旧面板IPC错误回包测试先失败后修复，浮层15项复验passed。前端生产构建、Rust宿主check及格式通过。27905中间包未安装，最终包必须包含旧代catch修正。真实Menu跨重开与应用重启保持、三个UI偏好实际恢复仍待验，不外推旧视觉闪烁/按键体验。

16:54:21旧App6896经托盘1001正常退出，overall passed/failed_stages0/130ms；Helper51216/run68440e97…的cleanup/unload/session_detach/terminal均code0。退出前用户配置仍button-mappings ef88f857…、settings df8b145b…，未修改。menu-preference-*最小日志保留至本次验收关闭，唯一当前包不另复制，中间包由最终候选替换；无电源/无线电/强杀/Git操作、无新探针或额外代理。

最终冻结候选构建42266 exit0：0.2.5，`target/release/bundle/nsis/无线麦 SayAll_0.2.5_x64-setup.exe`，2026-09-22T17:06:14.6274170+08:00，26,276,847字节，SHA256 `7d7c2cd6aea48502a47fda020153af8104e6dc2cce97992ebee1559d84b79c3f`。27905/88374均为未安装中间构建；最终包包含旧代IPC错误隔离及pending退出先标记取消/等释放再落盘的修正。最终源scene_control SHA `def700cdc3c1c905a4a8d71d2276827921c1d84339a7eb8ac26fe4a0da209821`，SettingsStore `227e5461a8e489c202ed297e24e794c9cf715b8e5c9d385cd4eb5503bcd81f8e`，Tauri lib `4e5a6f14e418ccba2b28138a538e453727e302f06bd9044b2e31c361ed73307e`，ui-preferences.ts `d612fd7c985f6bcd2732492813d1b1f323813ae11deb4d932697b5472e099425`，SceneOverlay.vue `49f3aef5f20abc6e80f5dd12e8cf83a81f004c7a08b5c62f439ace811c95f90d`。

管理员原目录覆盖exit0，安装App SHA `6474695f1575969d05daaee30e96986cd5f3095d43e3ead51e4f3fe1f20d8f64`，release `2cc820b26a0bc22912efff717359a47bbe54fa5bca0ba4c6ac988f52e8ec949c`，字节比对仅NSIS UNK→NSS三字节标记不同；HID b6044e9b…与组件c2491a8f…不变。Explorer普通App32456（17:08:26.7940848，TokenElevation=false），用户opt-in自动Helper36440（17:08:28.2884424，TokenElevation=true），均核原D盘安装路径。RC003连接/监听输入上下文ready，Menu scene_active=true；run `2be3ef24caa84c3f9021d3cde48405fa` 仍source=pending_per_request，不宣称增强ready。本次菜单组无需增强初始化，不使用返回/音量、不确认模板、不更改默认关联。

安装前后映射文件仍ef88f857…，原settings仍df8b145b…；启动后原设置字段规范化SHA `1a0dad3e55989cf73fcbd98596b9d2b32f76463efe2b2d8ee2c4479fa3359723` 一致。新ui_preferences尚未产生用户写入，首次按既有true默认读取；Menu偏好尚未持久选择，首次false，不能从旧未保存历史猜值。后续用户显式选择按新契约保护。代码验证通过，实际Menu跨重开/应用重启和UI偏好保持仍pending。

### 2026-09-22 按键页语音卡独立行

用户截图显示旧语音卡沿图片宽202px，而普通卡为270–300px，普通首行从y=34.4px开始，与语音y=8–80px共处一行。仅修改ButtonsPage及共享卡片样式：语音同普通cardWidth/72px高度，顶部居中独立行；普通六行在下方剩余640px画布内等距排列，第一行top96px、末行bottom632px。左右按键顺序、图片/热点统一frame、语音实时按下/释放说明及无自定义单/双/长控制保持；连接终点读取新行中心，原窄窗连续缩放不变。

`pnpm test src/pages/ButtonsPage.test.ts` 23项passed，既有几何用例核等宽、每行不重叠且完整位于画布内、连线中心及13卡/36格。`pnpm tauri build --bundles nsis --config target/sayall-input-candidate.json --ci` 会话30601 exit0，含前端生产构建；未重跑无关Rust/HID或新增UI探针。用户截图为修改前事实，新安装布局与窄窗实际视觉仍pending，不从jsdom外推。

唯一最终0.2.5包 `target/release/bundle/nsis/无线麦 SayAll_0.2.5_x64-setup.exe`，2026-09-22T17:27:33.3983862+08:00，26,280,942字节，SHA256 `576b5aebe21776d1bf8d2b3055ed29007cda4f0a18ef04380072f467d896d794`。旧App32456经托盘实际退出1001正常结束，overall passed/failed_stages0/117ms；原Helper36440的cleanup/unload/session_detach/terminal均code0。首次退出前因Get-Process读提权路径为null而守卫拒绝，未发退出；核同PID/创建代后复用原已核路径正常退出，无强杀。

管理员既有目录覆盖exit0，安装App SHA `eaf3f80b5852a273099eee47d76a74b53266cf327b68354d0ec204678cfa6629`，release `480d927432fab07ec33c4f23760883be980aa751f15002a1f62801a53eb42016`，逐字节仅UNK→NSS三字节标记不同；HID b6044e9b…及组件c2491a8f…未改。真实Explorer启动App10384（17:29:58.0459579，TokenElevation=false）与用户opt-in自动Helper51572（17:29:59.3162172，TokenElevation=true），实际映像均核在既有D盘安装目录。进程存在不证明增强source/ready，本轮不做按键验收。

退出前、安装后、启动后最新配置SHA均保持：button-mappings `ef88f857f7a130ef88153292d7ed429c6f6db7ad9451e9524c20e25e3aa9b2c3`、settings `df8b145b5048b300b5202c2e92b0fb9e7e162d0deae49670a606e10413d2a872`；未回写旧备份或改变偏好。voice-card-layout-tests/release日志位于既有target/dev/rc003-three-key，保留至本次视觉验收关闭，仅当前交付包无中间副本；不扩历史清理，无Git、电源、无线电或遥控操作。前一Menu记忆组与固定按键/UI待验状态保留，用户新布局请求不代表这些项目已通过。

### 2026-09-22 晚 Menu 偏好保存抖动与语音恢复组合修复

用户新报只有语音不可用并指出长Menu偏好保存时浮层抖动。语音首次失败/原三角色已恢复但journal仍阻断的证据和零setter契约归 `../capture-input-session-20260916/evidence.md` 同日段，未重查无影响HID来源。当前Menu日志22:33:24.155/28.164/29.710的三次Long意图均各一次，分别6/3/5ms后persisted=true，UP均long_consumed=true；后两次同面板代107 false→true，前次为代106，不把该自然记录归并成此前指定四步全项通过。

确定抖动触发共三处：前端pending/error small在列表上方v-if插拔；每次Snapshot都重算选中滚动；宿主每次面板Snapshot重复set_size/position/show。修为三个反馈同一常驻网格显隐，最大换行空间始终保留（含IPC错误）；pending只保留aria状态和确认屏障，无原生disabled或整板opacity改变；选中项/代次/列表长度未变不重滚；宿主同面板代内容快照只由原scene-event更新Vue，不重复窗口API，Hidden/Toast→Menu、新代、DPI变化仍走原显示事务。未改Long阈值/边沿、来源、模板持久化或焦点恢复语义，旧foreground/restore失败不因此称已验收修复。

输入锁定23项、SceneOverlay16项与宿主呈现事务1项passed，含pending/成功/失败中列表/反馈/checkbox节点不变、不重复滚动，以及新代/DPI重新布局。组合源码冻结后唯一构建会话63466开始；真实语音/Menu视觉仍pending，不复用旧截图冒称修复。未执行电源/无线电/强杀/Git操作或新探针；用户新偏好按最新配置保护，不回写历史快照。

最终构建63466 exit0（含前端生产构建），0.2.5唯一候选 `target/release/bundle/nsis/无线麦 SayAll_0.2.5_x64-setup.exe`，2026-09-22T23:00:11.1922041+08:00，26,280,066字节，SHA256 `39e5b0fb6edf15d4c97a97120525873e89fbd6175dfa1c80e2dea79057c069a6`。此刻尚未退出旧App10384/Helper51572或安装新包；等待用户在当前公开界面完成既有“保留当前选择”零setter操作，再一次核正常记录收尾与配置。过程日志保留供本次故障对照/安装验证，不复制新台账或清理历史唯一证据。

后续用户完成正常记录处理，旧实例两次真实语音恢复证据归capture-input-session同日段。23:05旧App10384正常退出，capture_route/overall passed、failed_stages0；Helper51572/run2a0009bb…全部清理终态code0。39e5b0fb原目录管理员覆盖exit0，installed App SHA `97ae65870896f140850216eca6bd52d4d50fdcc9b72d71d57bc784d5a695b82e`，release `bcca616ecc902c8401b2d348f0f97b7ca0cf1be4ca3e4fd88b0baac1dad7a12f`，仅UNK→NSS三字节标记差异；原HID/组件hash b6044e9b…/c2491a8f…保持。真实Explorer新App59576（23:07:20.8545384，TokenElevation=false）、Helper42676（23:07:22.3794662，true），实际映像均核原D盘安装目录。

用户最新配置（含新Menu偏好true、输入锁定true及三个UI偏好true）退出前/安装后/新启动后字节相同：button-mappings SHA `e1588bf1b8505f3b1ef80db259497b648a12318e25587d709f2065d2d8134018`，settings `032ce5ec920a1b747d6b58d691ddd84ac2b41d9797713e00aed938e83be40956`；不回滚到下午旧hash。23:08:42.289就绪后先语音短句UP+x，再Menu true→Long false→独立Long true→短Menu取消/重开保持，均不需增强初始化、不确认模板或改关联。此刻新包实机尚pending；不重复截图/UIA或按键日志轮询，不将旧Menu/布局尚未回复项标passed。四份当轮定向/构建日志留在既有target/dev目录作故障对照，当前唯一交付包无冗余副本。

### 2026-09-27 用户自行重启后三键无动作：现有配置与接管分层

本轮仅只读取证，未退出/重启应用、操作遥控器或无线电、重跑来源实验、构建或安装。安装载荷仍为上述39e5b0fb包：主程序97ae6587…、HID Helper b6044e9b…、组件Helper c2491a8f…均保持。实际App30788于00:11:06.630启动、TokenElevation=0；固定安装Helper32984于00:11:08.229启动、TokenElevation=1。自动恢复偏好true，日志仅一次restore_requested及peer_verified；用户配置两文件SHA与上述最后记录完全相同。当前无签名或权限不足的产品启动证据。

产品日志本地时间00:11:23.019配置5/mask0，随后返回5次、音量+5次、音量−6次，均有逐请求selected_instance来源、9字节报告和完整physical DOWN/UP，suppressed/mapped均0，未产生三键map_fire。00:11:23.166主窗口获得焦点，00:11:48.487失焦；失败区间profile_active=false，通用actions为空，只有模板Menu入口。自有窗口本身不强制禁用映射，而是保留此前选择；此前外部程序身份未记入历史日志，不能据此指认某个未绑定程序。00:11:48.634配置6/mask7开始ready=true/all_up=true；该run之后尚无三键物理按放，不把就绪当动作通过。00:12:40.868有Ok→Shift+Enter及注入成功，仅支持该键当时执行。

只读公开前台查询确认当前Codex归一身份codex，唯一默认关联preset-agent，不受其版本化安装目录变化影响；随后最新配置14/mask7/ready=true/all_up=true。Agent现有固定动作分别为Backspace/VolumeUp/VolumeDown。此时不需要准备键；仍须在该程序实际输入区完成一次真实动作观察才能关闭本次用户无动作报告，不能把过去BUDTH验收或此处源报告成功替代。新包语音自动恢复分支、Menu稳定反馈及此前未回复组继续pending。

驱动页“启动三键增强”调用现有request_start；已有ACTIVE时返回状态，不另启Helper也不改变模板。页面挂载/刷新getComponentStatus还会检测独立内核候选，两次kernel_catalog_verify拒绝属于该签名检测，不能等同三键Helper失败或推断用户点错按钮。当前状态分支会区分“已就绪：三键增强已接管”和“等待映射：当前窗口未配置增强按键动作”；本轮未另行自动化读取可见页面，不将源码文案当用户已观察。原配置保持，未擅自补通用动作；本轮未产生新过程文件，原运行日志保留用于本次报障对照。

用户随后报告Agent音量±实际有效，并指出SayAll按键页三键无动态。00:24:07.993/24.084两次Back均在mask7、准确来源且suppressed=mapped=1，随后Backspace注入成功，UP成对；不读取第三方正文，实际删除仍待用户光标位置反馈。音量±多次同样抑制/映射/注入/释放完整，结合用户观察可确认本次有效。未配置时无高亮的独立缺陷已确认：客户端仅将mapped位图变为DriverEdge，可信physical只记录日志。前端编辑选择锁true只锁编辑对象，本来不应阻止按下高亮。

最小修复只补Rust客户端观察分发：当前认证连接、配置/序号及报告校验通过后，physical进入独立UI观察并集，不进入执行边沿/场景菜单/手势识别；与既有普通/DriverEdge观察去重。同一物理按住在配置取消时不假释放，监听停止/断连/退出清观察；断连后的旧排队观察拒绝。前端事件与轮询均读取该并集，避免旧RawInput快照清掉增强高亮。mask0仍零抑制/零映射、来源保护及Helper二进制不变，首次可信DOWN无需执行器ready也能观察。按键页常驻说明高亮只代表接收、动作由实际程序模板决定；没有擅改通用配置。

Rust观察/执行隔离与生命周期3项、IPC序列化1项、按键页24项及bridge9项passed。首次UI轮询测试因挂载后才切虚拟时钟未驱动已建interval而失败，调整测试前置后通过；断连测试补实际队列屏障后区分已观察持有与断连前尚未处理请求。构建类型检查拒绝误加到脱敏诊断示例的字段，已限定移除，并一次核对所有运行时/诊断构造；不把这些软件结果当实际高亮通过。00:41:18旧App30788经既有实际托盘退出命令正常退出，overall passed/failed_stages0/120ms；原Helper32984/run91c2ab6b…的cleanup/unload/session_detach/terminal全部code0。用户当前配置哈希留于既有target/dev主题，仅用于本轮安装保护，未回写旧配置。

最终构建86157 exit0，0.2.5候选 `D:\BaiduSyncdisk\01_Code\GIT\remote-mic-app-windows\target\release\bundle\nsis\无线麦 SayAll_0.2.5_x64-setup.exe`，2026-09-27T00:44:30.1736937+08:00，26,293,829字节，SHA256 `5c95f36ad463d164c61ce304252f6f2ec7c14b7f9007d386e13bbe06f9ea204e`。管理员原目录安装exit0；installed App SHA256 `25423c92161d6b6bfbfa990a65656fff3268dae68c6f5dddc64f527d369ea38a`、release `781723e5581d952c96288adc5d25a1c6012195b2f607f0a916c62014f9cb5455`，已核仅NSIS标准UNK→NSS三字节差异。HID Helper仍为b6044e9b…，未改来源/报告/抑制协议。

真实Explorer启动App33900（00:46:33.1357045，TokenElevation=0）、自动恢复启动Helper20952（00:46:35.0582986，TokenElevation=1），两者实际映像均在既有 `D:\Program Files\无线麦 SayAll`。run3de91823590d4970a38831f3ffa05e16在00:46:36.188完成mask0配置回执，bound为pending_per_request，尚无新物理报告，不能表述来源/执行器已就绪。安装前最新button-mappings/settings基线在安装后与启动后完全一致，保留用户本轮新增选择，没有按更早备份回滚。

真实无配置高亮待用户操作：现有程序关联不含Windows资源管理器、Chrome或Edge；先自然进入未绑定程序再回SayAll按键页，分别按放三键，后续按日志确认实际mask0及观察/执行次数。页面编辑“通用”不代表当前实际生效通用；不要求初始化键，不把首份可信DOWN丢弃。旧语音/Menu/布局组仍pending。测试/编译完整日志与配置hash基线复用target/dev/rc003-three-key，仅留本次缺陷对照与唯一交付包，未新建台账或清理无关历史产物。

同日用户明确要求“提交当前状态→清理中间文件→修改按键页四操作固定底栏”。严格按此顺序执行：fetch origin main与cargo fmt --all -- --check通过，原6份暂存工程文档提交为 `d94f6f7713a7eedc388f1bfb48117f2f462810cc`；累计121项源码/测试/必要证据候选提交为 `7d9b068d23f7fe56f686162a194c3e7094ab61fa`，明确实机待验，85fffc84仍为祖先。仅本地提交，无push；个人配置、二进制、原始现场与236份本地产物不入提交且保留。提交门禁发现3处空白，限定修正后diff --check通过，不改变产品逻辑。

提交后仅将target/dev/rc003-three-key中10份已汇总成功构建stdout与7份空检查载体移入系统回收站，共17文件/70,354字节；逐文件核精确边界、非reparse、未受管、无占用及文档无直接引用，原位置剩余0。保留当前包/缓存、当前运行日志、用户配置恢复依据、0x9F及来源/恢复失败唯一材料。未清理其他任务历史。

随后局部修改ButtonsPage/App与样式：只在按键页使主内容为有界高度的flex容器，页面为可滚正文与独立操作行两行grid；四操作复用原处理器，自动换行，监听和锁定留在正文，不使用全局fixed覆盖侧栏或对话框。既有24项按键页与1项导航通过；首轮旧测试查找原footer导致6项失败，只将动作查找定位改为新操作区后25/25通过。未加后端或HID复测，未将DOM测试当真实视觉通过。新UI保持未提交，等待本地包安装与用户实际滚动观察；先前按键/语音/Menu未回复组不因截图自动通过。

底栏最终构建75323 exit0（含前端生产构建），唯一0.2.5包 `target/release/bundle/nsis/无线麦 SayAll_0.2.5_x64-setup.exe`，2026-09-27T01:21:00.2441426+08:00，26,291,469字节，SHA256 `ff8f8f6cdb3a431a1756b04b978bcc166c522deca8c4843b855fbb56ba5fcb99`。01:16:57旧App33900/Helper20952经既有托盘命令1001正常退出，overall passed/failed_stages0/156ms、Helper清理四终态均code0。管理员原目录覆盖exit0；installed App SHA256 `b6dc1237b0dca7c1915e8eca7e01bb1b5128ab237aa2078641e933af2c53cd17`，与release仅标准NSIS标记差异，HID Helper b6044e9b…未变。

真实Explorer新App39028创建01:22:11.1625948、TokenElevation=0，自动Helper37924创建01:22:15.2421179、TokenElevation=1；公开QueryFullProcessImageName确认二者均为既有D盘安装目录对应映像。最新配置在退出前/安装后/新启动后完全一致，不回滚Menu或UI偏好。受影响源码/测试和本段文档保持未提交；没有第二次UI提交、push、设备按键/无线电/电源操作，也未重试旧失败UIA或截图路线。随后用户明确反馈“已经能看到这4个按钮在底部了”，据此仅确认安装版四操作底部可见；滚动全位置、窄窗/遮挡/弹窗、切页与实际保存动作仍未获得对应观察，不外推passed。新过程输出仅fixed-actions-ui.log、fixed-actions-release.log与配置hash基线，复用既有主题目录，保留用于本次交付与待验对照。

### 2026-09-27 Windows 官方上游整合（进行中）

固定输入 `74230bf5f841cac2f099d1c6fd25683dac50131d`，本地 checkpoint `f32ae1b14f93bd56ec82e5c650c48085fd1f329c`，共同基线 `6504010828b12713ce033cb3e231087af6a6482f`。本地 `85fffc84`、`d94f6f7`、`7d9b068` 均为祖先；没有 reset/clean/整配置恢复。

另一个已结束任务仅将 fork main 快进至上游，没有第二套产品提交。共享 `sync_git_repos` 的 SayAll 矩阵项通过 PR #1 / `94dbb8e8750e5d0dbe649b9065e926c8a6e971f7` 停用，其他九项保留；本仓库继续人工 test 分支/PR 门禁。

18 个冲突文件逐段融合；严格来源/边沿配对与普通主程序边界保留。上游仅 BLE 建链吞 F5 不采用，单 RawInput 注册改进保留；安装器接入现有 ExitCleanup，超时中止安装。现有模板直接键语义与草稿明确保存保留，应用库启动目标独立于程序匹配身份，导出清除本机应用库。新上游历史硬件文件已脱敏，未执行其探针。

首轮前端169项为165passed/4failed（新增观察字段及即时保存旧断言），受影响修正后34/34passed。默认并发 Rust 测试曾在系统回收站 Shell 扩展 libapr_tsvn.dll 内发生 access violation；IFileOperation 清理串行后不再复现。全局 KeyGate/组件操作夹具互扰改为 test-only 独占，updater 真实日志落盘用独立测试子进程，未降低生产门控。最终 `scripts/ci-preflight.ps1` exit0、7/7passed：前端169/169，Rust331passed/17ignored（原硬件/故障专项条件），生产前端、普通宿主及 runtime-simulation feature 编译通过；`git diff --check HEAD` exit0。没有执行历史硬件探针或电源/无线电操作。当前无最终合并包/安装/新实机结论。过程日志限 `target/dev/rc003-three-key/upstream-*`，保留本轮首个失败与最终门禁以供对照，完成安装/PR后回收可再生成中间输出；不把未反馈的旧UI高亮/语音/Menu等用例补记 passed。


合并候选已保存为 `729d14dfa60022253a31ef9d4f9e989eab8a99ee`；236份原有未跟踪材料未纳入。生产构建 exit0，0.2.6 NSIS 包 `target/release/bundle/nsis/无线麦 SayAll_0.2.6_x64-setup.exe`，2026-09-27 03:19:58+08:00，26,412,928 bytes，SHA256 `0a15fd2f2c0f5c6b833aae720efab4d71aa44fc1f63c3001b23dcc192fc2ce35`。旧App/Helper通过托盘正常退出，App整体清理128ms、failed_stages0，Helper terminal0；03:16退出前的历史宿主detach事件不补记为本轮主动卸载passed。

管理员覆盖既有D盘安装目录 exit0。安装EXE SHA256 `83538e8a21c37c134b260fd8e771e92bfed23c116f84004d1253c71c08a32376`；与构建EXE只差Tauri打包时 `__TAURI_BUNDLE_TYPE_VAR_UNK`→`...NSS`的3字节，其余全部字节相同。固定Helper SHA256 `2d344265e2ed6d116631fa64dcfb10c0065d2d2d499f8a82655df55d46c92c55`。真实Explorer启动App32936（03:22:44.176652，TokenElevation0）和用户已保存opt-in启动Helper27836（03:22:45.574294，TokenElevation1）；日志source_revision精确为上述合并提交。settings、button-mappings、capture-input-session三文件退出/安装/启动哈希相同，5模板/6关联与用户Menu偏好true保留。

BLE真实连接ready、input_context connected=true、电量公开读取available、journal为空；Helper已握手与bound、配置mask7回执成功，但新run `864b2e7a45b14fb097f895a3cc444684` 仍 `pending_per_request`，不以进程存在宣称增强动作ready。复用现有Notepad→BUDTH关联，建立独立空白 `RC003-acceptance-merge.txt`（PID33472）；03:26:52.608起仅请求语音短句hold/release+x验收，等待用户实际反馈。三键首实际报告、映射/注入、菜单/键盘录入与其他受影响硬件结果仍pending；无电源/无线电操作。此后允许push test并建draft PR运行CI，main合入仍需CI及受影响实机门禁通过。

2026-09-27 CI 跟进：draft PR [#1](https://github.com/luck-gh/remote-mic-app-windows/pull/1)，head `6811bc8b4ff6422bde2992653563a02fde3f2587`。首轮 [run 36266247769](https://github.com/luck-gh/remote-mic-app-windows/actions/runs/36266247769) completed/failure，唯一失败步骤为运行中安装的正常退出测试；此前前端/Rust/仿真/生产构建/安装矩阵/静默安装步骤已通过，artifact 上传未执行。命名退出请求与 supervisor/input_quiesce/raw_input_stop/ble_disconnect/capture_route 清理均 passed，overall 85ms/failed_stages0；失败因测试仍断言已删除的上游 `ble_session_shutdown/platform_shutdown` 日志。修正仅涉及测试契约和文档，要求统一 ExitCleanup 全阶段、从 NSIS 读取21.5s预算，失败收尾不强杀；14项纯断言（含冻结实际日志与缺阶段/失败/超时反例）通过。当前产品包不变、用户运行现场与配置未动；新CI与用户语音组仍 pending，未重建/重装、未合main、未发布。
