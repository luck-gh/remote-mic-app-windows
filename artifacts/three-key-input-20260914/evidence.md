# 三键开发候选与最终交付证据

2026-09-14：源码/构建候选已完成，三键用户目标尚未验收。全部操作均为本地；未暂存、提交、发布、安装内核驱动、调整系统签名或安全配置。

## 本地包

- `无线麦 SayAll_0.2.5_x64-setup.exe` SHA256 `0d437e5feabe19428cf06a4e3614f24e1f152b8241439c915575a70cdd19fd0f`。
- 最终 Release Helper SHA256 `fcc1343498c3ef7a6973eee7579eb207ba44073dbb743ed2bb984c37fd297d45`；PE DependentLoadFlags 0800，无 VCRUNTIME/MSVCP 动态 CRT。
- 开发 SYS SHA256 `fe080c487b9c0e5e270c558d69e945a46ba3e8e4c1c3934fd9238d0475b9d10d`，没有 Microsoft 内核签名；包内安装门禁拒绝。
- 应用包构建和 `scripts/verify-windows-bundle.ps1` 均 exit 0。精确文件见 `build-evidence.json`。

## 实际验证

| 范围 | 结果 | 证据 |
|---|---|---|
| Windows lib | exit 0；152 passed、5 ignored | windows-lib-final.log |
| 维护定向 | exit 0；6 passed | maintenance-final-tests.log |
| 组件 UI | exit 0；5 passed | component-maintenance-ui.log |
| 纯 C 合成状态机 | exit 0 | state-abi3-final-tests.log |
| KMDF 编译 | exit 0 | driver-abi3-build.log |
| INF / CAT 生成 | exit 0；INF warning 1384 | inf-verification.log、catalog-abi3.log |
| 内核签名 | exit 1：No signature found | kernel-signature-final.log |
| cargo fmt | exit 0 | fmt-final.log |
| 最终 Release Helper 实际 CLI | exit 50；SignatureInvalid；固定 Event Log 终态存在；调用者日志路径未使用 | helper-release-audit-evidence.json |
| 独立源码/Helper PE 审查 | 21:27:41+08，12/12 hash；无 actionable finding | maintenance-review.json |

Helper 源码含限定包的 catalog 成员与内核信任校验、真实维护 API、精确旧包恢复/新包回滚、需重启失败状态和终态日志。测试验证共享句柄释放后再后验、旧包仍在不再次安装、失败与重启并存、缺失/篡改/签名拒绝；没有执行真实签名包的安装、升级、卸载或恢复。

## 2026-09-14 安装阻塞（历史）

当日两次有界托盘定位后的右键均未得到可确认弹出菜单，Quit 未调用。源码没有其他明确外部退出 CLI/IPC；主窗口关闭只隐藏。当日没有执行安装器，也没有强杀，旧实例 PID 81844 保留。该历史证据见 `delivery-status-20260914-blocked.json` 和两份 normal-exit 日志；后续应用安装结果如下。

## 2026-09-15 安装与启动 passed

用户明确确认“已退出”后，09:25:10+08 只读核对 SayAll 实例数为 0，冻结包 SHA256 完全匹配。没有重新构建或重试托盘操作。安装前再次确认无实例，使用 RunAs 仅提权安装器，覆盖既有目录；安装器 exit 0。通过真实 Explorer desktop COM 启动主程序。

09:27:31+08 核验唯一新实例 PID 75452、Session 1、安装路径匹配、所有者与 Explorer 相同、TokenElevation=false。应用自己的顶层窗口通过 WM_NULL / SendMessageTimeout 有界响应检查。安装 EXE 与冻结原载荷逐字节比较仅偏移 13493186 的三字节 `UNK→NSS` 不同；Helper、INF、SYS、CAT 的 SHA256 均完全匹配。两配置文件在安装前、安装后、启动后哈希一致，未读取内容或操作配置界面。

执行 `target/install-three-key-candidate.ps1` 和 `target/verify-three-key-install.ps1` 均 exit 0；日志见 `install-20260915.log`、`verify-install-20260915.log`。结构化证据见 `delivery-status-20260915.json`、`installed-runtime-evidence-20260915.json`、`install-boundaries-20260915.json`。只读确认 SayAllInput 系统驱动数为 0；未执行驱动安装、系统安全设置变更、重启或发布。此结果仅解除应用安装阻塞，不是三键实体或内核验收。

## 分型号、分键边界

| 型号 | 按键 | 实际报告 → Windows 交付 → 捕获/归因 → 系统抑制 → 映射执行 |
|---|---|---|
| RC001 | 返回 | 全部 deferred；没有实机报告或 INF 绑定 |
| RC001 | 音量加 | 全部 deferred；没有实机报告或 INF 绑定 |
| RC001 | 音量减 | 全部 deferred；没有实机报告或 INF 绑定 |
| RC003 | 返回 | 全部 deferred；物理序列未确认、驱动未签名未加载 |
| RC003 | 音量加 | 全部 deferred；物理序列未确认、驱动未签名未加载 |
| RC003 | 音量减 | 全部 deferred；物理序列未确认、驱动未签名未加载 |

RC003 HidP 描述符合成证明 Report 1 为 121 字节、三个 u16 槽位 1/3/5，不能当实际按键报告。11:42:45Z 至 11:47:45Z 的空物理采集窗口没有用户确认，不能判失败。当前 INF 仅精确 RC003 revision，不能宣称双型号支持。

正常安全配置、Windows 1809、Verifier、冷/闲置首用、组合、断连、睡眠、异常退出与驱动卸载均待真实目标环境验收。签名账号、组织/EV、微软提交与产品认证是外部条件；未代购、申请或发布。应用安装闭环已于 2026-09-15 完成，无需额外构建或重复已冻结测试；上述内核与硬件验收仍 deferred。

实现、固定参考、验证真源分别见 [驱动说明](../../drivers/SayAllInput/README.md)、[ATTRIBUTION](../../ATTRIBUTION.md)、[专项](../../Testing/StructuredTemplatesAndDrivers.md)、[TODO](../../TODO.md)。
