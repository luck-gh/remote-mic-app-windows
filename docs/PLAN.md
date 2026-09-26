# 项目方向与计划入口

## 目标与当前阶段

无线麦 SayAll Windows 版面向小米蓝牙遥控器 2 / RC001 和 2 Pro / RC003，提供按住说话与按键映射。产品范围、普通用户权限、公开 API、语音生命周期和平台分层约束由根目录 [AGENTS.md](../AGENTS.md) 定义；架构选择见 [ADR 0001](decisions/0001-tauri-rust-windows-only.md)。

当前处于产品化与分场景验收阶段。代码已形成 Vue 页面 → Tauri IPC → Windows 平台层 → 纯 Rust 语音管线的纵向路径；当前可证明的行为及其验证范围见 [FEATURES.md](FEATURES.md)。代码存在、自动化通过与双型号真机通过分别判断。

## 计划与决策真源

| 内容 | 权威位置与读取范围 |
| --- | --- |
| 后续工作、优先级、依赖和剩余验收 | [TODO.md](../TODO.md)；此处不复制清单或维护第二套完成状态 |
| 长期架构、阶段边界和来源策略 | [Windows 路线图](architecture/windows-tauri-roadmap.md)；参考实现归属见 [ATTRIBUTION.md](../ATTRIBUTION.md) |
| 默认注入轨与可选 Helper 的隔离边界 | [ADR 0002](decisions/0002-dual-track-injection-optional-helper.md) 的决策部分；历史归因的适用限制见下节 |
| 已确认专项方案及其验收条件 | [应用打磨](plan/2026-09-05-windows-app-polish.md)、[深色模式](plan/2026-09-08-windows-dark-mode.md)；保留专项归属，不把其历史步骤重新登记为新计划 |
| Windows 与双型号验收定义 | [Preview 测试手册](../Testing/WindowsRC003Preview.md)、[WeType 历史记录修复测试手册](../Testing/WindowsWeTypeConsentHistory.md)；用例和结果模板本身不证明已执行 |
| 安装、配置与交付 | [安装与配置指南](installation-and-configuration.md)、[发布规范](../RELEASING.md)、[分支规范](../BRANCH_MANAGEMENT.md)；发布授权边界以根级规则为准 |

没有新增后续计划或重排既有工作。下一项实施及其验收条件沿用 TODO 与对应专项真源；记录计划不构成实施或发布授权。

## 影响判断的证据边界

- [README](../README.md) 的型号适配总述、TODO 中不同时期的验收描述和 Preview 手册的整体验收状态并不一致。不能把其中任一总述当作当前全部场景的结果；型号、版本与场景须绑定直接证据，当前基线只采用 FEATURES 中明确限定范围的验证。
- ADR 0002 的历史背景将 WeType 不响应归因为注入过滤，但后续 [零间隔注入调查](../Bugs/2026-09-04-wetype-zero-gap-injection.md) 已用受控实验区分注入时序与 F5 干扰。不能用该旧背景推断微信输入法必须依赖驱动；这不变更 ADR 的可选 Helper 隔离决策，也不外推豆包的行为。
- 根级规则中的“生产常驻日志系统列入路线图”是旧状态描述；当前实现与自动化证据见 FEATURES，日志行为规范归 [LOGGING.md](../LOGGING.md)。旧状态不作为新建立日志系统的计划依据。

上述差异保留在各原文，未据此更改产品目标、任务状态或专项方案；不确定的真机覆盖保持未确认。
