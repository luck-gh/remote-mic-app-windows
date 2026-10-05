# Windows 安装器优雅退出验证（应用运行中执行安装/卸载）

初版：2026-09-16；当前安装前卸载/覆盖流程：2026-10-04。

## 验证目标

确认"应用正在运行"时执行安装或卸载**不会强杀应用**，而是：

1. 安装器先请求应用退出（会话内命名事件 `Local\SayAll-GracefulExit`）；
2. 应用收到请求后**自己**关闭 BLE 会话并等待 `ble_session_cleanup` 落盘；
3. 应用退出后核验按键组件真实清理终态，确认后才覆盖文件；当前定制模板已移除
   `CheckIfAppIsRunning` 的强杀调用，退出或清理未确认就中止安装。

双击发现已有安装时，提供“安装前卸载”和“请勿卸载”：前者安全退出、卸载后自动继续安装，
后者覆盖；两者保留原安装目录与配置。静默 `/S` 与被动 `/P /UPDATE` 默认覆盖。
安装器使用当前用户最高可用权限，权限预检必须早于退出和卸载；主程序仍由 `RunAsUser`
以普通权限启动。独立卸载器仍保留。Helper 每阶段等待 45 秒；原助手结束后至多启动一次包内
`--cleanup-only`，新阶段重新计时。该预算覆盖共用 30 秒的连接/HELLO 等待和正常响应，
不是持键释放上限；超时仍为 `pending` / `unconfirmed` 时保留后台清理者和回执，
停止替换，不强杀。完成新 payload 写入后只把安装目录内两个旧 Helper 与
`SayAllInput` 的 INF/SYS/CAT 五个明示文件送入回收站，不清除配置、未知文件或已安装驱动。

历史背景（2026-09-16）：当时 Tauri 的 NSIS 默认模板在 `Section Install` / `Section Uninstall` 中把钩子**之后**
紧跟 `CheckIfAppIsRunning`（`nsis_tauri_utils::KillProcessCurrentUser`，直接
`TerminateProcess`）。应用在持有活动 BLE GATT 会话时被强杀会留下未关闭的会话，使系统
蓝牙栈进入僵死态，此后所有 WinRT 入口返回 `0x80070008`，应用内全部自愈手段与睡眠都无效，
**只能重启电脑**。详见
[../Bugs/2026-09-16-ble-stack-resource-exhaustion-recovery-ineffective.md](../Bugs/2026-09-16-ble-stack-resource-exhaustion-recovery-ineffective.md)。

## 历史：默认模板强杀与"是否关闭应用"弹窗

本节保留历史故障证据；当前覆盖模板不再调用这些强杀分支。

来源：本仓库构建产物（`target/release/nsis/x64/utils.nsh` + `SimpChinese.nsh`，构建时生成，
可用作 CLI 版本的 ground truth）。`CheckIfAppIsRunning` 的实际分支：

```nsis
nsis_tauri_utils::FindProcessCurrentUser "${executableName}"   ; $R0 = 0 表示有实例在跑
  IfSilent kill_...                        ; 静默安装（/S）：不提示，直接强杀
  ${IfThen} $PassiveMode != 1 ${|} MessageBox MB_OKCANCEL ... ${|}
    kill_...:   KillProcessCurrentUser + Sleep 500   ; 交互点"确定" → 强杀
    cancel_...: Abort $R1                            ; 交互点"取消" → 安装中止
```

弹窗文案：`{{product_name}} 正在运行！$\n点击确定以终止运行。`

由此得到三条结论：

1. **弹窗是 Tauri 模板的固定行为，不是本应用加的。** 它只在"实例在跑且本钩子没能
   让它退出"时出现。
2. **交互安装时用户本来就有一次安全选择**：点"取消"= `Abort`，什么都不装、不碰应用；
   只有点"确定"才会强杀。静默（`/S`）与被动模式没有这个选择，直接强杀。
3. 本钩子若成功让应用自行退出，后续检测落空，**连弹窗都不会出现**。

### 2026-09-16 更正：上面"0.2.10 起不会再有弹窗"的结论是错的

用户反馈"装新包时还是弹窗问我是否关闭无线麦"，回查发现**钩子从来没生效过**，
里面有三个叠加的 bug，每一个都足以让它静默失效。均已实测定位并修复：

| # | bug | 证据 | 后果 |
|---|-----|------|------|
| 1 | `FindProcessCurrentUser` 传的是**全路径** `"$INSTDIR\xxx.exe"` | 探针：裸名 → `0`（在跑），全路径 → `1`（不在跑）。见 `artifacts/nsis-probe/sayall-findproc-probe2-result.txt` | 永远判定"没有在跑"，整段等待逻辑被跳过 |
| 2 | 返回值判断**写反**：`${If} $R8 != 0` 才补等 | 探针：返回 `0` = 进程在跑，`1` = 不在跑。见 `sayall-findproc-probe-result.txt` | 进程已退出时白等 6.5s，进程还在（BLE 收尾要 5s）时反而不等 |
| 3 | `System::Call` 输出用了 `.r8` 却判断 `$R8` | 探针：`.R8` → `$R8`（成功 `916`，不存在 `0`）；`.r8` → `$8`。见 `sayall-probe3-result.txt` | `$R8` 恒为空，而空值 `!= 0` 在 NSIS 里为**真** → "事件存在"分支恒真，旧版检测从未触发 |

三者叠加的净效果：**无论运行的是新版本还是旧版本，钩子都会白等 6.5 秒，然后
必然落到 Tauri 的强杀弹窗。**这正是用户看到的现象，与"是不是旧版本"无关。

### 修复后的三条路径（均已在 2026-09-16 实测）

| 路径 | 行为 | 实测结果 |
|------|------|----------|
| 没有实例在跑（含应用内更新路径） | 零等待直接继续 | `passed`，耗时 1s（旧实现在这里也要白等 6.5s） |
| 有实例且能打开事件 | 置位 → **轮询**到进程消失（预算 20s，覆盖应用侧 5s BLE 收尾）→ 继续安装 | `passed`：替身进程打印 `signalled, exiting` 后自行退出，安装器未中止、未强杀，耗时 2s |
| 有实例但打不开事件（无监听的旧版） | **不杀也不装**：立即 `Abort` + 引导走应用内更新 | `passed`：退出码 1639，耗时 1s，进程**存活**（未被杀） |

第三条的取舍：旧版无法被请求退出，强杀会残留未关闭的 GATT 会话把系统蓝牙栈楔死
（只能重启 Windows），装下去又会被旧进程占住链路。因此选择中止并引导用户
在应用内点"检查更新"——旧版的 `on_before_exit` 会**先断开 BLE 再拉起安装器**
（`tauri-plugin-updater` 的 `install_inner`：`on_before_exit` 在 `ShellExecuteW`
之前执行，见插件源码 `updater.rs:843→862→876`），既不重启电脑，也不需要用户
手动退出应用。

### 现场记录（2026-09-16 09:05 左右，0.2.9 → 0.2.10）

**升级 `@tauri-apps/cli` 时必须复核**：tauri `dev` 分支已把该宏改为走 Restart Manager
（`RSTRTMGR::RmShutdown` + `RmForceShutdown`，交互取消同样是 `Abort`）。两种实现都不
请求应用自行退出，本钩子对两者都成立；但升级 CLI 后需重新查看生成的 `utils.nsh`，
并重跑契约测试与端到端测试。

## 自动化步骤

普通预检和 Windows CI `verify` 运行以下隔离检查，不安装产品、不控制产品进程、
不写真实 stop 或 receipt；默认路径测试只在 `target/dev/unified-input` 创建小型 fixture：

```powershell
node --test scripts/test-windows-overlay-install.cjs
powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -File scripts/test-windows-capture-cleanup.ps1
powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -File scripts/test-windows-retired-files.ps1
```

CI `installer` job 完成真实 NSIS 构建后，另跑 `test-windows-retired-files.ps1 -RealRecycle`
核对自建文件实际进入回收站，及 `node scripts/test-windows-overlay-compile.cjs` 编译微型
payload 的定制模板（生成物不执行）。`test-windows-installer-write-log.ps1` 另运行自有微型
NSIS fixture：锁定自建文件时必须返回非零并记录失败组件，释放锁后必须实际覆盖、
以本轮 completed 替换旧日志；日志写失败不得影响 NSIS error flag、error level 或寄存器。
所有 fixture 日志限定在任务目录，不写生产安装结果日志。2026-10-04 晚最终源码对应结果：模板 4 项、清理协调器
63 项及真实回收 13 项 `passed`，NSIS 实际编译 `passed`。新增
`test-windows-reinstall-flow.ps1` 实际执行生产宏，覆盖直接覆盖、卸载续装、非目录拒绝、
属性查询异常拒绝、子卸载失败与新目录六种场景，全部 `passed`；失败场景核对旧载荷保持且
没有继续写入。新目录测试使用每次不存在的子目录；非法路径通过普通变量传入，避免 NSIS
自动过滤 `$INSTDIR` 中非法字符而形成假测试。上述检查不代替下面的真实升级验收。

真实安装的本次结果保存在 `%LOCALAPPDATA%\SayAll\installer-result.log`，只有 start、failed、
completed 和固定组件名；失败时先看 app/helper/gadget/license/uninstaller 等具体阶段。
原首次 1603 未被独立 fixture 复现，不能仅凭同包重试成功断定原因；调查记录归
[前台输入与退出回归](../Bugs/2026-10-03-foreground-input-regression.md)。

```powershell
# 仅在干净、隔离的 Windows runner 执行真实安装/卸载矩阵；
# 不要为了运行该脚本先卸载用户正在使用的安装。
pnpm tauri build --bundles nsis
./scripts/test-windows-installer-graceful-exit.ps1
```

脚本覆盖两个场景，任一失败即抛错：

| 场景 | 步骤 | 判据 |
| --- | --- | --- |
| 安装覆盖运行中的应用 | `/S` 安装 → 启动应用 → 等 `reason=listening` → `/S /UPDATE` 覆盖安装 | 新增日志含 `reason=installer_requested_exit`、`app_shutdown` 的 supervisor/input/RawInput/BLE/capture 全阶段 passed 及 overall passed/failed_stages=0；进程在 NSIS 实际等待预算（当前 **21.5s**）内自行消失；超时安装中止 |
| 卸载运行中的应用 | 启动应用 → 等 `reason=listening` → 静默卸载 | 同上，走 `NSIS_HOOK_PREUNINSTALL` 路径 |

脚本自身收尾只请求正常退出；仍存活则保留进程和安装并报告失败，不强杀、不继续卸载。

手工复核（等价判据，安装过程中随时可看）：

```bash
LOG="%LOCALAPPDATA%\SayAll\Logs\sayall-diagnostic.log"
rg "app_exit |app_shutdown " "$LOG"
# 期望顺序：reason=listening → reason=installer_requested_exit
#          → app_shutdown 各资源阶段 passed
#          → app_shutdown stage=overall phase=completed terminal_result=passed failed_stages=0
```

## 2026-09-27 整合候选的 CI 契约修正

[CI run 36266247769](https://github.com/luck-gh/remote-mic-app-windows/actions/runs/36266247769) 的覆盖安装实际收到正常退出请求，统一 `ExitCleanup` 的全部资源阶段 passed，整体 85ms、failed_stages=0；原验收脚本却仍要求已移除的上游双清理器日志，因而失败。脚本现使用当前完整清理标记，拒绝缺阶段或任何清理失败，并从 NSIS 定义读取等待预算；没有放宽为只看进程消失。

纯断言验证 14 项 passed（冻结 CI 实际日志、逐个缺失标记、失败阶段、未退出及预算边界、无强杀收尾）；未在用户主机执行安装/卸载测试。新一轮 GitHub CI 与受影响硬件验收仍待完成；旧 CI 未执行到卸载场景，不能将其记为 passed。产品二进制未变化，沿用已安装候选。

## 2026-10-04 当前候选现场结果

`56fec037` 本地包的“安装前卸载”和“请勿卸载”两条可见向导均从真实 Explorer `open`
入口通过；安装器实际同用户提升权限，应用自行完整退出后分别卸载续装或覆盖，三份配置
字节保持，完成页启动的应用实际为普通权限。最终再经 Explorer 重开并核对安装载荷和可见
主窗口。精确包身份、正常退出耗时、日志片段、验收脚本修正及未覆盖范围归
[同主题 Bug](../Bugs/2026-10-03-foreground-input-regression.md#2026-10-04-晚普通双击安装的权限失败)。
本轮未执行新的 CI 或独立完整卸载；不能把卸载续装路径外推为所有卸载选项通过。

## 历史现场结果

| 项目 | 结果 | 证据边界 |
| --- | --- | --- |
| 安装器钩子语法（`makensis` 汇编） | passed | 0.2.10 构建产出安装包，含本钩子 |
| NSIS `System::Call` 宽字符串 + 指针句柄在**运行时**有效 | passed | 独立探针（`Local\SayAll-Probe-GracefulExit`，对生产零影响）：PowerShell 侧事件被置位、`CloseHandle` 分支进入 |
| `FindProcessCurrentUser` 返回值语义（0=在跑 / 1=不在跑） | passed | 探针 `sayall-findproc-probe-result.txt` |
| 该函数**只按进程名匹配**，传全路径恒返回 1 | passed | 探针 `sayall-findproc-probe2-result.txt` |
| `System::Call` 输出寄存器大小写语义（`.R8`→`$R8`，`.r8`→`$8`） | passed | 探针 `sayall-probe3-result.txt` |
| 安装器钩子在**安装段与卸载段**都能汇编（标签解析） | passed | `artifacts/nsis-probe/sayall-hook-compile-test.nsi` 用 `makensis` 编译 |
| 三条路径的运行时行为（无进程 / 有监听 / 无监听） | passed | 见上表；替身 `crates/sayall-windows/examples/installer_exit_mock.rs` |
| 安装器契约测试（事件名一致、两道钩子都请求退出、宽限 > 应用预算、钩子无强杀、裸进程名、寄存器一致、轮询等待） | passed | `cargo test -p sayall-windows-app --lib`，6 项 |
| 退出收尾只执行一次（不产生误导性 failed 日志） | passed | `claim_exit_shutdown` 单测 |
| 应用侧命名事件（创建/等待/置位/无事件时报错） | passed | `cargo test -p sayall-windows --lib graceful_exit`，4 项 |
| 应用运行中执行安装 | deferred | 需真机（会改动机器安装状态），CI 步骤 `Test installer graceful exit while the app is running` 覆盖 |
| 应用运行中执行卸载 | deferred | 同上 |
| 真机"升级期间正在使用的遥控器语音链路" | deferred | 需 RC001/RC003 实机；本脚本的 CI 版本无蓝牙硬件，会话清理是空操作 |
| 从 0.2.9（无监听）升级到 0.2.10 的现场 | passed | 弹窗出现（旧实例不会响应）；该次日志无 `installer_requested_exit`；新实例 2 秒后落 `listening`、2.84s 连上遥控器，此后 3 小时零 `windows_resource_exhausted`——**这是运气，不是保证**，不能据此认为强杀可接受 |
| "有监听的新版本 → 更新版本"不再出现弹窗 | passed | 2026-09-16 用 `installer_exit_mock` 替身实测：应用自行退出，安装器未中止、未强杀 |

## 边界

- CI 机器无蓝牙硬件，因此 CI 只断言**退出机制**，不断言"真实 GATT 会话在升级中被正确关闭"。
- 当前本机两条可见安装界面已验；SmartScreen、Windows 10 1809、代码签名、真正标准账户和交互 UAC 取消未覆盖。
- 应用侧宽限（`GRACEFUL_EXIT_TIMEOUT` = 5s）必须始终小于安装器轮询预算
  （`SETTLE 1500ms + 轮询上限 20000ms`）；契约测试守着这个不等式，改动任一侧都要重跑它。
- **改 NSIS 钩子后必须真跑一次 `makensis`**：`System::Call` 的类型串、输出寄存器名、
  标签解析都是**运行时/汇编期**才检查的，编译通过不等于行为正确——本页记录的三个
  bug 全都是"看起来对、跑起来静默失效"。最小复现脚手架见
  `artifacts/nsis-probe/sayall-hook-compile-test.nsi`。
- 事件名改动即破坏兼容：`crates/sayall-windows/src/graceful_exit.rs` 与
  `src-tauri/windows/installer-hooks.nsh` 必须逐字一致（已由契约测试守住）。
