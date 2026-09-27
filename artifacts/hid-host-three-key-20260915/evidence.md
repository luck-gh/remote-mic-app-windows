# RC003 三键宿主旁路开发证据

这是 2026-09-15 限定三键只读实验，不是功能验收。旧应用包安装证据仍归 `../three-key-input-20260914/evidence.md`；本目录没有新应用包或生产增强交付。未更改用户配置、报告内容或安全设置，未安装未签名驱动，未终止任何宿主或应用。

## 可复现开发检查

- 官方 Frida core 17.15.3 devkit archive SHA256 `52d4b60d0fb9f9e69f03c652d50d5f3f22c9c967b3d23ff26d33d3c9039bd2d3`，只解包未执行自解压 EXE。固定源码提交 `e0ec2bee624b6ea11b72d5cc74639132449789c2`。
- `official-link-inputs.json` 是实施者从已核 archive 新目录解包所得 header/lib 哈希与实际构建输入一致证据。独立审查核对固定成员锁与输出绑定，未独立重做这次解包，不把实施者证据写为独立通过。
- Rust 三键解析/边沿隔离测试 3 passed；JS 匹配归因、pending 不读输出、严格报告解析及停止撤钩测试 3 passed。日志 `parser-tests.log`、`probe-js-tests.log`。合成报告不证明物理 RC003 键实际报告。
- 最后诊断增量构建 exit 0，`probe-build-diagnostic.log`。C SHA256 `81489572041a96a2e0486f464b615aa03acd213b085f52d9562babbf4e05c666`；EXE `6198e43c7cd4092b6d6bb8665378bc9e9d0a92ad3964924be02a3fd5428be661`；运行脚本 `dd917908e8426a24be0a72997d979525d1bb94b50d6b94d6279ff2e5d21c9d83`。本增量只增加公开宿主元数据查询、固定阶段/隐私错误分类/计时和相应运行 pin；原只读 JS、目标身份、来源锁和有界清理保持原审查范围。不能把早期独立审查扩大到增量全部新字节。

## 实际主机检查与首次失败

- 10:40 第一次 inspect exit 3：实际 HostPid 注册表类型 QWORD，初始实现仅接受 DWORD。修正为精确 DWORD/QWORD 范围解码，不放宽设备选择；原记录 `inspect-registry-type-failed.json`。
- 10:43 第二次 inspect exit 3：7 次宿主 OpenProcess 失败，原顺序在发现完成后才启用 SeDebug。随后将显式 UAC Helper 的 SeDebug 前移并记录实际启用结果；原记录 `inspect-privilege-order-failed.json`。主程序不提权。
- 10:45 inspect exit 0，唯一 RC003 指定 revision、精确系统 WUDFHost 签名和创建时点验证成功。
- 10:47:18–20 首次 attach **failed**，exit 1、core 错误 code 4；初版未记录 domain，不能仅凭数字认定根因。没有 hook_ready、没有报告、没有让用户执行按键序列。manager close exit 0，DLL 物理驻留未证明。原始匿名事件 `capture-first-failure.json`。
- 同时被动 RawInput/LL 对照进程正常结束；由于没有用户物理序列，其空结果不能用于证明三键失效或 Windows 不消费。
- 10:47–48 Defender 唯一事件 2010 为云安全智能更新，没有对应检测/阻断事件；CodeIntegrity 无事件；Application 无相应 Error/崩溃事件。这里只说明未观察到事件，不证明安全软件绝对未参与。`windows-event-summary.json`。
- 11:00 只读 inspect exit 0：同一宿主创建时点，LocalService/session 0，非 AppContainer、非 restricted，保护级别 NONE，动态代码及签名限制 flags 0；模块快照 31 个、Frida 命名模块 0。不能凭名称快照证明全部注入痕迹清空。TOKEN_QUERY 成功，TOKEN_DUPLICATE 被拒 5，因此未伪称实际宿主 AccessCheck 成功。`inspect-host-metadata.json`。
- 宿主及普通用户 SayAll 实例仍存在且 Responding 为 true。未通过关闭宿主作为恢复手段。
- 调用者 Temp 的必要读取主体 ACE 不存在，但首次 agent 临时目录已清理，实际 agent 路径/ACL 尚未观察，不能以父目录推论直接宣称失败根因。`temp-acl-summary.json`。

## 尚未通过的边界

11:06 第二次有界诊断附加再次 **failed**：local_device 成功 31ms，attach 返回 `domain=frida code=4 category=agent_load_or_early_exit elapsed_ms=2125`，精确对应官方 core 远程 worker 在握手前退出后内部取消的分支；不能进一步区分 LoadLibrary 或 agent 早退。manager close 0，无 hook_ready、没有物理按键序列。记录 `capture-second-failure.json`。实际 ACL 观察器因 finally 中对空 subscription 执行 Remove-Job 失败，未保存实际 agent ACL，故此项 **failed**。修复后只用本地空白文件 fixture 自测观察器 1 项 passed，文件明确 `syntheticFixture=true`；`agent-temp-acl-observation.json` 与 `agent-temp-observer-selftest.json` 目前都是该合成证据，绝非真实 agent 权限。

随后新增仅在本探针静态链接内观察 CreateRemoteThread 的诊断：透明转发原参数、返回值与 LastError，仅对已持有的同一内核进程对象复制一个最小查询/同步线程句柄。使用 Wait(0) 与 GetExitCodeThread 区分尚在运行和真实返回 259，不读写线程内存、不挂宿主新钩子。无注入的本进程 CreateThread 假后端 9 项 passed，构建 exit 0；冻结清单 `worker-observer-review-hashes.json`，该阶段等待有界独立复核，当时尚未据此运行宿主实验。该诊断不会作为生产通用注入功能保留。

同日新增当前必做范围：实体键盘与遥控器 TV/Home 共存。源码已证明 selected_path 过滤只存在于 Raw Input 路径，而全局 LL 使用 persistent+online 或四秒 armed 推断来源；TV/Home 之外，共享方向/Enter 也经过此来源推断。现阶段只有调查和明确撤销旧取舍，尚未改生产输入逻辑或宣称修复。验收真源仍为 TODO 与专项手册。

11:19:48 线程观测候选通过该 7 文件独立定向复核后执行一次实验，attach 仍 failed；这次真实 worker `matched=1 observed=1 duplicate_error=0 exited=1 wait=0 query_error=0 exit_code=5`，固定官方 worker 实现证明此非零来自 LoadLibrary 的拒绝访问，而非成功进入 agent 后返回 0。core attach 耗时 1468ms、相同分类；manager close 0。没有 hook_ready、没有物理序列。记录 `capture-worker-access-denied.json`。此次实际 ACL 观察器成功收集 3 个官方资源文件创建事件，文件和上两层 ACL 均可读，未见 LocalService allow ACE；这不是宿主实际 AccessCheck 成功/失败的替代。当前 `agent-temp-acl-observation.json` 已被这次**实际**观察覆盖；合成 fixture 证据只保留在 `agent-temp-observer-selftest.json`。对应三类 Windows 事件通道同窗口未观察到 warning/error；不把未观察到事件表述为绝对无安全软件参与。SayAll 与精确宿主仍在响应。

后续私有 ProgramData runtime prepare 候选编译 passed，ACL 结构纯内存测试 6 passed；尚未 attach。实际 prepare **failed**，创建目录数 0：普通 linked token 的系统卷根访问检查获 `0x1f01ff`，只读 ACL 确认该机器已有卷根 Everyone FullControl。这与 ProgramData 允许创建子项的正常权限不同；候选按严格祖先替换门禁拒绝，不擅自更改全盘权限或降低门禁。当时冻结 6 文件清单 `runtime-prepare-review-hashes.json` 等有界评估，原拒绝记录已分别另存并在下文指明。没有声称私有目录生成文件继承或宿主 RX 已通过。

随后经独立边界评估，唯一卷根使用显式 `volume_root` 模式且校验 `X:\` 形态：仍持有非 reparse 目录对象，只不把该不可重命名的卷根自身 replacement mask 当作产品目录归属判据；ProgramData 以下继续普通 token 权限检查，产品目录继续 owner Administrators、protected 精确三 ACE 和全链 deny-delete。日志保留 `volume_acl_safe_claim=false`，不宣称卷根 ACL 安全，也不声称 share READ 禁止一切属性/EA 修改。实际 prepare **passed**：创建固定 `C:\ProgramData\SayAll\HidHostRuntime\17.15.3` 的三层产品目录，System/Admins Full、LocalService RX；对真实继承空白文件以宿主 TOKEN_QUERY 初始化 Authz，读取/执行允许、写入禁止，ordinary replacement 禁止。该空文件保留、未删除任何共享内容；未修改共享 Temp、全局环境或安全产品。原失败单独保留 `prepare-volume-initial-failed.json`、`prepare-volume-rights-failed.json`，成功记录 `runtime-prepare-passed.json`。

最终 runtime 增量 6/6 hash 经独立定向复核准入（`runtime-prepare-final-review-hashes.json`），使用 EXE `eb461df4627aeadec0403dd91a0048ceda9940a952d29ee80268b0c98ec23c04` 于 **11:42:07.532** 开始首次成功的 120 秒只读窗口。attach 0、create_script 0、load_script 0、hook_ready；11:44:07 附近按时停止，stop ACK、script_unload 0、session_detach 0、最终 worker exited=true/exit_code=0、probe exit 0。该实证支持专用目录解决本次 LoadLibrary 权限失败，但 **不代表三键成功**。最终 selected matched=0/unmatched=1188/reports=0，普通 RawInput/LL 对照也没有按键事件；协调者已请求实体序列，但截至本次收束没有用户确认，结果为物理操作/报告 deferred，不判物理捕获 failed。`capture-runtime-completed.json`、`native-input-comparison-runtime.log`。没有延长窗口、没有连续重开、没有强杀；模块物理卸载仍 unknown。

生产键盘共存逻辑未改，当前缺陷另以三个 opt-in 测试实际复现：在线常驻被当作来源、四秒武装被当作来源、罕见 VK 被当作来源。测试名统一 `keyboard_coexistence`，暂用明确 known-defect ignore 隔离默认稳定入口；显式命令 `cargo test -p sayall-windows --lib keyboard_coexistence -- --ignored --test-threads=1` 的失败不能被忽略包装成通过。实现设备精确抑制后应移除这些 ignore，并用真实键盘/遥控同时验收。没有新生产包、没有覆盖安装或实际配置变更。

11:52 阶段，RC003 返回/音量加/音量减：物理报告、Windows 交付、精确映射、可能抑制及 DOWN/UP 配对全部 deferred。RC001：无当前硬件、无宿主报告证据，且原 KMDF INF 没有 RC001 绑定。现阶段只读探针没有生产 IPC/自动启停/映射集成；冷启动、闲置首按、重复/组合、断连/重连与停用均未真机验收。既有未签名驱动候选仍未加载，不把 UI 映射或源码审查当作三键恢复。

11:52 阶段收束：显式键盘回归命令 exit 101、0 passed/3 failed；默认同筛选 exit 0、0 passed/3 ignored，不能说修复通过。待用户确认“准备好了”后才开启新的实时采集窗口；不使用已经结束的 11:44:07 截止时间，不在缺实体报告时实施生产报告抑制或打包。

## 14:29 首次用户确认的物理序列

14:16:25–14:18:25 的 120 秒窗口正常结束（`capture-user-ready-completed.json`），但用户随后明确回复“未赶上时间”，故该次物理序列未执行，不能判断捕获 failed；`user-ready-physical-response.json` 保存纠正结果。

为避免再次错过，单独将固定采集时长 120→600 秒、外层等待 190→670 秒，来源过滤与停止/清理保持不变；3 文件定向审查准入，构建 exit 0，EXE SHA256 `1a419d0ee59a15e4b91efebd21921c7c3fdf5337cf4891c44cb23fc5059731a3`（`ten-minute-review-hashes.json`）。14:29:14.314 开始至 14:39:14 的只读窗口，用户明确完成“确认校准→返回→音量加→音量减→确认”。普通 RawInput 对照实际记录 14:30:05.526/05.664 与 14:30:12.579/12.625 的选定小米确认键 DOWN/UP，两次阳性对照证明本轮有真实操作；`native-input-ten-minute.log`、`ten-minute-physical-response.json`。

至 14:30:59 宿主侧 matched=0、unmatched=1076、reports=0；当前候选的物理捕获验收 **failed**，不再归因于用户未按或硬件缺失。已知失败边界在报告解析之前：精确 PDO 对象名称归属门没有接受调用，现版将 NtQueryObject 失败/空名称/合法非同名合并，不能断言是哪一种，也不能凭零 pending 证明同步读取。下一版仅增加匿名查询分类，精确同名仍是唯一读取许可，未归属 payload 不读；JS 合成测试 4 passed（`query-classification-js-tests.log`）。当前旧实例继续固定 600 秒，不热换脚本；清理结果待自然结束后补入。实体键盘共存和三键生产映射仍未修复，RC001 仍无硬件/绑定证据。

14:39:14.170 正常到期：stop ACK，script_unload=0、session_detach=0、manager_close=0、worker exited=true/exit_code=0、probe exit=0；确认进程已结束后才构建下一增量。最终 matched=0/unmatched=6351/reports=0，物理捕获 failed 结论不变。完整匿名记录为 ten-minute-capture-completed.json；模块物理卸载 unknown，不作全面残留清空声明。分类增量构建 exit 0，源分类测试 4 passed，构建入口线程测试 9 passed（无远程注入）；固定清单 query-classification-review-hashes.json，下一真实分类实验尚未运行。

14:42:23–14:52:23 分类增量（EXE `205cc36d8af27ba833d666eb2bc291c4e75c3d5d4f6be1f8eb1cd2e8f13a4f07`，最终 5/5 hash 经独立定向复核准入）只观察既有宿主调用，不要求用户重复按键。最终 query_mismatch=6396，query_failed/status/short/empty/shape/prefix/exception 均为 0，matched/reports=0；全部是成功查询后的合法非同名，不能当作三键真实报告。14:52:23 正常 stop ACK、script_unload/session_detach/manager_close 均 0，worker exited=true/exit_code=0、probe exit=0，确认探针进程不存在；模块物理卸载仍 unknown。`query-classification-capture-completed.json` 保存完整匿名事件。

最后有界可行性检查：所选 1812 devnode 的两个注册接口经 QueryDosDevice 解析均等于其 PDO（`selected-interface-object-link.json`）；其直接父节点经公开 PnP 关系确为该蓝牙遥控器而非无线电，有两个接口（`selected-parent-interface-summary.json`），但未取得这些接口与实际宿主句柄的对应证据。系统 HidOverGatt 模块签名 Valid，静态导入 CreateFileW/DeviceIoControl（`system-hid-static-input-boundary.json`）；只观察未来 CreateFile 返回值可建立新句柄生命周期，前提是执行实参确实属于选定设备接口，现有句柄和附加前已发生的打开不能由此恢复。设备自然重开可能触发该入口，但本轮没有重置/断连设备或再 attach；这条未证路径不能承担冷/闲置首用与生产映射。

所选实例实际 INF 是 `hidbthle.inf` / `HidBthLE.NT`，对应 Wdf section 的 UMDF 版本 2.15.0，没有 UmdfHostProcessSharing 禁止池化指令。微软默认可池化，不能把同宿主当作设备唯一性。没有找到已确认的公开 reflector FILE→devnode 闭环；PSS 名称快照不能提供此映射，未为该方向新增探针。当前可解释的按设备输入结构仍是此前 KMDF lower-filter/rawPDO 开发候选，但未 Microsoft 签名、未加载，且原范围仅三键/RC003 指定 revision，不自动涵盖本轮新增 TV/Home 与所有共享 VK；签名也不替代这些尚未实现/验收的功能。

本轮收束：用户 RC003 实体配合已实际完成；当前三键捕获候选 failed、生产三键与实体键盘共存目标未完成。没有新生产包、覆盖安装、配置变更、系统安全设置变更或强杀。RC001 没有当前硬件及 INF 绑定，仍 deferred。未授权购买证书、微软提交或发布，后续路线须先形成具体可审方案，不再自动重复探针。
