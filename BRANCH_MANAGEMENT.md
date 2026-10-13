# 分支与提交管理策略（Windows 版）

参照 macOS 原版 `BRANCH_MANAGEMENT.md` 裁剪为 Windows 仓库当前适用的最小集；
发布管线不变量（相当于 macOS `release-main` 部分）待 Windows 发布流程建立时
再按原版补入。

## main 不变量

- 开始已获授权的分支、提交、PR 或同步操作前 `git fetch origin main`；功能分支必须从**最新的
  `origin/main`** 创建，不基于过时的本地 main。
  普通文档维护不以 fetch 为前置条件；上游贡献分支按 AGENTS 的 `upstream/main` 基线规则执行。
- main 工作区只用于同步已合入的远端主线：**不直接开发、不保存临时改动、
  不直接 push**（包括文档改动）——所有变更一律经 PR 合入。
- **该规则已由分支保护变成强制（2026-09-21）**：main 要求必需检查 `gate`
  通过，且 `enforce_admins` 开启——管理员同样不能直接 push、也不能在 CI
  未通过时合并。实测直推被拒：`Required status check "gate" is expected.`。
  紧急绕过方式：临时 `gh api --method DELETE
  repos/<org>/<repo>/branches/main/protection`，处理完立即按原配置恢复。
- PR 的目标分支只能是远端 main；合入后再次 fetch，确认本地 main 与
  `origin/main` 精确一致。
- 不使用 force-push、广泛 reset，或把未验收内容以整支旧分支覆盖主线；
  冲突逐文件核对解决。

## 工作项提交纪律

- 提交、推送、合并与发布分别遵循用户明确授权；功能完成或会话结束不自动授权提交。
  获准提交后，每个独立工作项保存一个检查点；未获授权时保留正常工作区差异，
  不用源码副本或新 worktree 代替 Git 历史。检查点记录实际验证和剩余失败，不能代替验收。
- **提交前必须 `cargo fmt --check` 通过（2026-09-05 教训）**：格式漂移曾致
  CI verify 连续 10 次失败（run #64-#73），阻塞 PR #19 合入近一天才发现。
  发现漂移先 `cargo fmt` 并作为独立 style 提交；CI 全量流水线约 19 分钟，
  格式问题拖到 CI 才暴露反馈太慢。
- **push 前跑 `scripts/ci-preflight.ps1`（2026-09-05 新增）**：本地镜像 CI
  verify 的 7 个快速步骤（前端依赖/测试/构建 + fmt/Rust 测试/check +
  `cargo check -p sayall-windows-app --features runtime-simulation`），
  约 1-2 分钟；通过即等价于 CI 的这些步骤必过。发布前加 `-Full` 再追加
  runtime-simulation 的完整构建。第 7 步是 2026-09-21 补入的：此前默认只跑
  前 6 步，而 `cargo check --workspace` 不带 feature、覆盖不到 feature-gated
  的 simulation 模块，#98 的 E0063 就是因此在本机全绿的情况下进的 main。
- **PR 与 push 的路径过滤不再相同（2026-09-21）**：`push` 到 main 仍按
  `paths-ignore` 跳过纯文档改动；`pull_request` 不再按路径跳过整个 workflow，
  改由 `changes` job 判定、`verify` 按需运行、`gate` 始终给出结论。原因是
  被 `paths-ignore` 跳过的 workflow **不会创建任何 check**，而分支保护要求
  `gate`，纯文档 PR 会永久卡在等待状态；现在这类 PR 由 `gate` 秒过。
- **PR 与安装器生命周期校验分层（2026-09-27）**：代码 PR 的 `verify` 只运行
  前端、Rust、Tauri runtime simulation 等快速正确性检查；NSIS 构建以及安装、
  升级、降级、卸载矩阵移到 main push 或人工 `workflow_dispatch` 的 `installer`
  job。`verify`、`installer` 分别设置 30、45 分钟硬超时，同一 PR 的旧提交会被新
  提交取消。必需检查仍为 `gate`，因此不改变分支保护配置。
- **CI 出结果前不得合并（2026-09-21 教训）**：#98 在自己的 CI 判定失败前
  5 分 35 秒被合入 main，导致 main 连红三个提交、后续所有代码 PR 都会在同一
  步骤失败。`gate` 必需检查已在机制上堵住这条路径——不依赖自律。
- 每个独立工作项一个 commit，只包含该工作项的内容；交付时报告完整 SHA、
  Push 状态与验证命令（引用 macOS 原版"worktree、提交和清理"不变量）。
- 工作必须中途暂停或移交时：在现有真源记录当前状态与剩余事项；只有已获提交授权时，
  才在功能分支保存检查点并注明未完成状态。
- 上游贡献按功能与性能说明用户可判断的行为变化，按合理依赖拆成少量 PR；保留真实作者
  和贡献提交的可追溯性，不擅自改作者、压平或重写历史。仓库可保留作者的合并方式需核对；
  无法保证上游采用时如实说明，不承诺替上游决定合并策略。

## 与现有规范的关系

- 提交前必须满足 `AGENTS.md` 的自验证规范（真机证明生效、逻辑完备、
  最小化修改）——commit 是验证完成的落点，不是绕过验证的通道。
- `AGENTS.md` 运维与自愈节继续适用（部署不强杀应用、破坏性操作先验证
  目标等）。
