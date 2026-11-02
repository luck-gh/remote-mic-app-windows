# TV 设备专属键盘过滤器：离线观察候选

本目录是来源隔离可行性阶段的独立 KMDF 1.15 x64 候选，预期部署位置是精确 RC003 Keyboard collection 的设备 UpperFilter、kbdhid 上方。它不是旧 `SayAllInput` 三键 HID LowerFilter 的升级或兼容层，也不进入当前 App/Helper 安装包。当前只有源码、离线测试与未签名 SYS，没有 INF、CAT、安装入口、私有用户态通道或捕获所有权；全部输入始终原样透传，实体反引号故障仍未修复。

## 当前契约

- CONNECT 保存本设备的上层 `CONNECT_DATA`，替换为本设备回调；重复连接明确拒绝。发送或下层完成失败时恢复原连接缓冲和状态，完成原请求。其他内部请求使用 `SEND_AND_FORGET` 透明转发，成功后由下层完成原 IRP，失败时本过滤器完成；CONNECT 保留自有 completion，不混用两种归属。DISCONNECT 按微软示例的未实现契约返回 `STATUS_NOT_IMPLEMENTED`。
- `TvServiceCallback` 将原上层设备、原 packet 范围、原 `InputDataConsumed` 指针交给原回调，不修改 packet、扫描码、flags、UnitId、ExtraInformation 或消费结果。返回后只观察上层实际消费的 prefix；未消费后缀由下层重试，不提前记边沿。越界消费结果记录 `TvConsumptionRejected`，保留上层返回值，取消不可信的观察 hold。
- 来源来自单个 `WDFDEVICE` 上下文；UnitId 和扫描码都不是全局设备身份。未来绑定须锁定真实目标 collection，不得注册 class filter；本轮没有安装程序，也没有实际绑定、层位或共存证据。
- 仅在设备硬件 key 同时存在显式 `REG_DWORD`：`ObserveCandidateMakeCode`、`ObserveCandidateFlags` 时启用候选匹配。初始化时仅 `KEY_READ` 读取一次，没有写注册表或热配置代码。MakeCode 为非零 16 位值且不为 overrun `0xff`；flags 只接受普通 `0`、E0 `2`、E1 `4`，BREAK 单独匹配。
- 两值均不存在记 `TvProfileAbsent`；单值缺失、类型错误、读取失败或值非法记 `TvProfileRejected` 及 NTSTATUS，拒绝观察、继续原生透传。没有 scan29 默认值，测试 `0x29` 是合成 fixture，不是 RC003 TV 实测事实；外部 profile 也不能单独证明该候选就是 TV。
- 匹配后观察 MAKE/BREAK、重复 MAKE 和孤立 BREAK。D0 退出/移除取消观察 hold，不合成原生键或映射 UP；重复停止幂等，移除后迟到 D0Entry 不恢复观察。这里的取消不是 App 已释放映射的证明。

## 回调与诊断

设备局部 WDF 自旋锁只保护状态/计数。上层 callback 和 ETW 写入都在锁外；DISPATCH_LEVEL 不分配、等用户态、读注册表或写文件。源码、上下文、栈快照及静态 TraceLogging 元数据均不放分页段。WDF 负责对象与下层目标的移除寿命；没有私有线程、定时器或用户请求队列。

开始回调时保存观察 generation/epoch，结束时若 D0 换代或回调重叠，只统计消费结果，明确记 `TvObservationSkipped`，不凭完成顺序制造边沿；重叠会取消旧观察 hold。这是明确的观察缺口，不代表硬件松开或捕获成功，实际验证须检查 `skipped/overlapped`。离线重入测试不能验证真实 CPU 调度、内核锁或 WDF 移除保证。

TraceLogging provider：`WirelessMic.TvKeyboard`，GUID `{20b52f98-976a-4d84-9ac6-55d9af7c70c2}`，事件 `Observer`，schema 1。字段含源码指纹、临时设备 ordinal、sequence、stage/NTSTATUS、配置/运行/移除状态、generation，候选 down/up/repeat/orphan_up/cancelled_hold，以及 offered/consumed/partial/zero_consumed/invalid_consumed/skipped/overlapped 累积计数。ordinal 仅本次加载有效；sequence 在锁内递增，以区分锁外日志的到达顺序。stage 数值见 `trace.h`。

不记录原始键码/flags、任意按键内容、UnitId、设备身份/路径、蓝牙地址、个人路径或凭据。无候选配置时不逐批输出普通输入事件。provider 在 DriverEntry 注册，创建失败或 unload 配对注销。ETW 可丢事件，未来真实采集须核对 session 的 EventsLost；计数和 API 返回不能替代用户可见结果。

## 构建与离线验证

```powershell
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/build-tv-keyboard-driver.ps1
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/build-tv-keyboard-driver.ps1 -TestsOnly
```

只复用已有 MSVC、SDK `10.0.26100.0`、WDK NuGet `10.0.26100.6584`，不下载依赖。目标 Windows 10 1809 x64 / KMDF 1.15，实际最低系统兼容性未验证。真实 `driver.c/observer.c/trace.c` 用 `/W4 /WX /kernel /GS /guard:cf` 编译链接 SYS，不签名或安装。

输出复用 `target/dev/tv-keyboard-filter/`。成功构建的 `source-identity.json` 保存 C/H/测试/脚本的相对路径、hash、组合源码指纹与实际 SYS hash；指纹同时编译进 ETW。文档不属于二进制输入，失败构建不刷新成功 SYS 的身份记录。

| 入口 | 实际边界 |
| --- | --- |
| `tests/observer_test.c` + 生产 `observer.c` | profile、设备局部匹配、MAKE/BREAK/重复/孤立 UP、E0/E1、持键停止/重复启停 |
| `tests/driver_test.c` + 同一生产 `driver.c/observer.c` | 用最小 NT/WDF 替身测试 CONNECT 保存/回滚/重复/取消和自有 completion；普通转发同步/异步成功或取消的下层完成、发送失败及恰好一次完成；原 packet/指针/消费值、混合/部分/零消费与重试、另一设备同码/Shift、坏 profile、D0/移除与重入/迟到 |
| 真实 WDK/PE 检查 | 公开头、调用类型、内核链接与产物；不代表 WDF runtime、内核 ETW 或真机行为 |

本轮实跑与精确身份归 [原位 Bug](../../Bugs/2026-10-03-foreground-input-regression.md#2026-10-22-已选择签名过滤器路线的离线观察候选)，来源归 [ATTRIBUTION](../../ATTRIBUTION.md#2026-10-22-设备专属键盘观察候选)。当前 SYS、测试 EXE、日志和中间输出用于冻结候选复核；候选被替代且独有证据已归档后按回收站规则清理，不另存源码包。

## 后续门槛

### 只读双设备候选观察入口

`crates/sayall-windows/examples/tv_raw_input_probe.rs` 是独立普通用户诊断程序, 不加载本驱动、不注入按键、不修改全按键支持。它通过 Windows 给出的 `RAWINPUTHEADER.hDevice`、接口属性与 CM devnode 查询关联来源, 再按型号硬件 ID 选唯一候选。输出 `model_candidate` 不是已验证的用户所选遥控器; 相同型号多个候选会拒绝启动。匿名 ordinal 只在该次进程有效, 不跨两次观察比较 ordinal。

只在用户已准备好安全空白文本框后运行, 每步最多30秒且正常按放一对即结束。当前 Agent 可能在 Raw Input 之前改写/抑制报告, 因此需用户显式正常关闭全按键支持并确认释放, 不能由工具偷偷关闭或把没有事件解释为设备未发键。第一步只按遥控 TV 一次并松开, 第二步只按实体键盘反引号一次并松开, 不输入普通文字或密码:

```powershell
pwsh -NoProfile -File target/dev/tv-keyboard-source/g2-source-step.ps1 -Step tv
pwsh -NoProfile -File target/dev/tv-keyboard-source/g2-source-step.ps1 -Step backtick
```

构建入口为 `cargo build -p sayall-windows --example tv_raw_input_probe --offline --locked`, 离线验证为同目标 `cargo test` 及 `--selftest`。本地入口固定本轮源码/EXE hash并禁止覆盖证据, 不自动执行两个步骤。入口先建立唯一命名event, 工具在注册/readback/来源表就绪后通过公开SetEvent通知; 观察到该信号且子进程仍存活才提示用户按键。命名event不携带设备或输入信息, 不依赖监听期间stdout; 失败沿原注销收尾。原30秒预算不变, 提示截止采用进程启动加30秒的保守界限。Ctrl+C、截止、设备变化和错误均沿原注销路径收尾; 注册后才建立来源表, 设备变化立即停止下一次读取, 诊断摘要在注销后输出。注册/卸载的真实副作用仍须现场核对, 离线 closure 测试只验证调用顺序/失败传播。

每步同时核 `finished.outcome`、候选 MAKE/BREAK、来源类别、取消/设备变化与 `cleanup_errors`, 不凭 exit 0 判通过。TV步记录该型号候选的首个键, 它是否为TV仍依赖用户当次动作; 反引号步仅保留目标用例候选, 不记录其他键码/文字。该观察不证明过滤器实际附着、清键前内核归属、抑制、TV到Tab映射或热插拔全时序安全; 不将候选观察扩为内核来源授权。

签名渠道/成本、精确设备 INF、安装/加载和跨机验证须另行授权；保持 Secure Boot/HVCI/签名强制，不启用 TESTSIGNING。本轮不修改当前 App/Helper/Agent、任务、binding 或设备状态。

获准加载后，须用 profile、匿名上下文、实际按键、已消费候选边沿和 ETW 丢失计数验证只读观察。TV 的真实 MakeCode/flags、准确层位、其他键盘并发、跨 Windows build/非开发 PC 都未证明。匹配计数不会自动发现任意 TV 键码，也不是系统抑制验证。

未来捕获还需有经验证的事件消费者和取消/失联/溢出的成对归属及模板映射；先取得旧 Agent stop ACK、确认释放并排除 TV 后再交接。本目录没有这些通道，不做无消费者的吞键。TV 动作仍由用户模板决定，不在驱动硬编码 Tab。RC001 必须独立验收。
