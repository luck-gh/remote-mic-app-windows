# 已证实经验入口

本页导航到具有实际实验记录、且仍与当前实现相关的项目经验。完整结论、条件与过程保留在各原真源；历史实验证明当时观察到的因果，不证明当前版本已重新完成端到端验收。当前能力证据归 [FEATURES.md](FEATURES.md)，稳定工作规则归 [AGENTS.md](../AGENTS.md)。

## 音频写入成功与下游可听性

- 直接证据：[CABLE Input 静音自愈实证](investigations/evidence/2026-09-07-cable-input-unmute.md) 中的“Windows 真实端点实验”和“SayAll + RC001 现场复验”；包含受控静音、调用产品路径、状态读回及现场观察。
- 复用范围：排查有音频活动却无输出时，按该实验区分端点主静音与应用会话静音，不能以 WASAPI 写入成功代替下游结果。当前 [audio.rs](../crates/sayall-windows/src/audio.rs) 仍分别执行端点和会话检查、解除静音后读回，并在推流期间检查会话状态。
- 限制：原实验覆盖 CABLE 与 RC001，不能外推 RC003；未确定是谁重新设置了静音，也不授权修改其他应用会话或非 CABLE 端点。现有根级自验证和日志规则继续适用。

## 注入形态与额外按键干扰需要分别取证

- 直接证据：[WeType 零间隔注入调查](../Bugs/2026-09-04-wetype-zero-gap-injection.md) 中的交替对照实验及第二层 F5 泄漏记录；分别观察了注入送达、输入法反应和物理按键干扰。
- 复用范围：分析 SendInput 返回成功但听写未启动时，使用该调查的分层证据，避免直接归因为第三方过滤。当前 [按住说话注入](../crates/sayall-windows/src/send_input_windows.rs) 仍调用 [逐事件提交](../crates/sayall-windows/src/send_input.rs)，并使用 [F5 抑制器](../crates/sayall-windows/src/key_suppressor.rs) 处理额外按键。
- 限制：结论限于原实验中的 WeType 版本和配置，不外推其他输入法；时序常量与冷/闲置后验证要求由 [AGENTS.md](../AGENTS.md) 和 [TODO 的性能已知项](../TODO.md) 承载，不从历史热态结果推导新的优化。
