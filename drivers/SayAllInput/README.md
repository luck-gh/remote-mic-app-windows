# 三键输入增强开发候选

本目录是 RC001/RC003 返回、音量加、音量减任务的本地源码候选。当前仅完成用户态逻辑测试、KMDF x64 编译与 INF/CAT 静态检查；没有 Microsoft 签名，也没有内核加载或任一型号的实体三键验收。本地应用候选附带开发驱动包和独立 Helper，现有组件入口因 Microsoft 内核目录验签失败拒绝安装、修复与卸载动作。未签名文件不会交给系统安装 API。不能将这些成果表述为三键已修好。

## 输入契约

使用 KMDF 1.15 HID lower filter，在下层 READ 完成后处理报告。只对精确 INF 硬件版本及运行时 HidP 验证通过的报告启用私有 raw PDO 通道；普通用户主程序通过所选 HID 的 PnP 父节点匹配私有接口，再进行 ABI 3 握手和显式 CLAIM。无全局 F13–F15 替代键，也不注入第三方进程。普通 HID、语音与 ATVV 基础路径保持独立。

2026-09-14 本机 RC003 的公开 HidP 描述符解析成功。`HidP_SetUsages` 的合成结果表明：Report ID 1、长度 121，三枚 16 位 usage 在字节 1/3/5，后部为零填充。usage F1/80/81 分别为候选返回/音量加/音量减；这里只证明声明的报告格式，不证明用户实际按键发送这些 usage。非零高字节、rollover、未知长度或填充会拒绝接管；其他 report ID 透传。INF 仅采用已研究的 RC003 精确 revision，RC001 没有猜测绑定。

## 生命周期与权限

- QUERY 不启用捕获。CLAIM 只接管当前有映射的三键位；等待真实全释放后才接管，记录已吞 DOWN 的归属。
- 队列上限 64 个状态快照；每个独占客户端最多一个未完成 READ。同一文件对象和请求会话才可续租/读取，关闭标志与所有权在同一锁内处理。释放、关闭、租约过期、溢出、报告拒绝、电源转换均有 ABI 原因码。
- 取消后保留已吞且仍按住的物理位，直到释放，避免把旧 hold 变成系统新 DOWN；重新接管等待全释放。用户态取消清理已交付映射边沿，序号断档或 ABI 错误立即退出通道。
- raw PDO ACL 允许本机交互用户、SYSTEM 和管理员，拒绝远程交互登录。应用仅在活动控制台会话 CLAIM；同一时刻一个客户端，不抢占其他持有者。ACL 不认证应用二进制，获准的本机进程仍可能主动占用该专用三键接口；不授予其他键盘或任意内存/设备控制权限。
- Control 队列明确 PASSIVE_LEVEL，父 HID 完成路径和状态仍使用非分页内存与自旋锁。父过滤队列沿用 WDF 的非电源管理默认值，不新增未经实证的停止时序。

目标最低 Windows 10 1809 / 17763 x64。使用官方下级系统池零初始化支持及 NonPagedPoolNx，不因 WDK 版本抬高应用契约。INF 为保持 1809 使用设备专属 LowerFilters append；InfVerif 会提示新版推荐 DDInstall.Filters。17763 实装和 HVCI/Verifier 仍待测。

## 构建与交付边界

运行 `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/build-input-driver.ps1`。依赖现有 MSVC/SDK 26100 与仓库 `target/wdk-nuget/10.0.26100.6584` 下的官方 Microsoft.Windows.WDK.x64 NuGet；锁定包 SHA256 为 `c393d03dfb640b5c92f546b32f6770ef68cd3aaf691956e7d66d8e2c28a1b55e`，NuGet author/repository/timestamp 校验通过。依赖、开发二进制与日志不构成可再分发签名包。

本地证据在 `artifacts/three-key-input-20260914/`，状态机与内核构建日志在 `target/input-driver/`。Inf2Cat 成功只生成内容目录；必须使用 `signtool verify /kp /v /c SayAllInput.cat SayAllInput.sys` 验证正式 Microsoft 内核签名及内容绑定。当前执行结果为签名缺失，不能安装。

Microsoft 硬件计划账号、组织/EV 条件、签名提交和发行许可仍是外部条件。Attestation 仅用于微软定义的测试交付，产品认证须按 HLK/WHCP 要求另行完成。不得安装上游测试签名二进制，不关闭 Secure Boot/内存完整性，不开启 TESTSIGNING。

Helper 已实现固定包内容校验、Windows 指定 catalog 的 DRIVER_ACTION_VERIFY、精确 DriverStore 识别、安装/修复/卸载、先前精确包失败恢复和新增包失败回滚。运行前独立重复验包，并建立全机维护互斥；驱动 MAINTENANCE 请求仅允许管理员，在同锁下确认无捕获所有者、已观察有效全释放报告，再阻止新 CLAIM，文件关闭自动解除。尚未观察报告或仍按住则拒绝，可在普通按键按下/释放产生有效报告后重试。Windows API 返回需重启会保留并报告，不自行重启；实际成功/升级/回滚/卸载及 Driver Verifier 仍 deferred。独立 UAC Helper 承担系统变更，主程序保持普通权限。

## 未完成的目标环境验证

包身份由 `scripts/lock-input-driver-package.ps1` 从最终待分发包重新生成到编译期常量。正式流程必须先验证 INF/SYS 对指定 CAT 的 `/kp` 签名；Microsoft 签名改变 SYS 时应从签名后的精确文件更新身份再构建主程序和 Helper，不接受用户编辑的运行时 manifest。开发参数只允许生成未签名测试常量，永不跳过运行时内核策略校验。`scripts/build-local-input-candidate.ps1` 先构建 Helper，再把其精确 hash 编译进主程序，并打入固定四个资源；只有已锁定且受系统内核签名信任的三文件包才显示维护动作。

ABI 3 的 QUERY 额外暴露三键 physical/swallowed 位及 observed_report，均为脱敏状态。维护请求与普通 CLAIM 分离，不把被动检测当作捕获授权；原始设备的 ShareAccess 不能单独作为维护互斥证明，所以驱动内另有文件级 reservation。新连接/未知报告不会默认“全释放”。维护源码入口为 `crates/sayall-windows/src/input_driver_package.rs`，公开 API 为 SetupGetInfDriverStoreLocationW、DiInstallDriverW、DiUninstallDriverW，均使用固定包且 flags=0；不会强制替换其他驱动、直接修改注册表中的其他 filters 或自行重启。

RC001 与 RC003 的每个三键分别需要真实报告→Windows 交付→设备通道→系统抑制→映射执行证据，覆盖普通权限、正常安全设置、冷启动、闲置首按、快按/重复/连续组合、断连、睡眠恢复、按住退出/进程异常以及驱动卸载。还需 Driver Verifier 下的取消/关闭/并行 IOCTL/移除压力和安装生命周期。过滤器尚未加载，内核报告采集仍未执行；2026-09-15 用户已确认的实体操作属于失败的用户态旁路证据，不能转算驱动验收。纯 C 合成状态机通过不证明 WDF 调度或设备时序安全。

固定参考、许可和公开 API 依据见 [ATTRIBUTION](../../ATTRIBUTION.md)，型号验收真源见 [专项手册](../../Testing/StructuredTemplatesAndDrivers.md)。

Helper 在加载器阶段限定 System32 依赖（DEPENDENTLOADFLAG=0x800，Windows 10 RS1 起支持），本地构建使用静态 CRT；不会在 main 前从用户可写安装目录寻找 VCRUNTIME。固定包路径解析为 canonical 后持有全部祖先目录及成员的 deny-write/delete 句柄，供后续 WinTrust 和 Windows 安装 API 使用。维护失败且 Windows 要求重启使用失败状态与 restartRequired 同时表达；恢复/回滚不是原操作成功。独立审查与静态 PE 检查不能替代带签名内核的系统实测。

本轮最终本地包和逐型号逐键矩阵见 [交付证据](../../artifacts/three-key-input-20260914/evidence.md)。2026-09-14 的托盘自动退出阻塞已于 2026-09-15 用户确认退出后解除：冻结包覆盖安装 exit 0，Explorer 普通权限启动、实际载荷与配置哈希核验 passed。最终 Release Helper 的签名拒绝路径已于开发阶段实际执行，exit 50；驱动仍未签名、未加载，应用安装不代表三键或内核验收。

2026-09-19 只读复核：核心12项源码与上述维护冻结全部同SHA，SYS/INF/CAT也同字节；复用既有开发验证，不重跑同SHA构建。用户目前没有明确可用的Microsoft签名渠道，此外部门槛不可用；仍不采购、申请、上传或安装未签名候选。三键最小源码闭包已存在，但实际过滤层、物理报告、抑制/映射、WDF和维护验收尚缺。TV/Home及全部共享VK键盘共存不在mask7范围，仍有独立实现与实证缺口。详细差距见[9/19核对](../../artifacts/hid-host-future-open-20260919/kernel-gap.md)。
