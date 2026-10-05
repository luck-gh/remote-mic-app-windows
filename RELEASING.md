# 无线麦 SayAll Windows 发布流程

本流程将参考仓库的发布不变量改写为 Windows 版本，适用于公开 Preview、Stable 及应用内更新资产。

## 发布不变量

- 默认交付物是本地测试包。先在本机生成测试安装器，报告其路径和校验值，并完成与改动风险相称的本地安装、升级、启动和功能验证。
- 本地验证通过后必须停在“可发布”状态；不得自行创建发布 Tag、GitHub Release、发布草稿或上传发布资产。只有用户在收到验证结果后明确要求“发布预览版”，才获得本次发布授权。“继续”“做完”“合入 main”等指令本身不构成发布授权。
- 发布源必须是已合入远端 `main` 的精确 SHA；开始前 `git fetch origin main`，发布 worktree 必须干净且与该 SHA 一致。
- `main` 不直接开发或 push，版本、Release Notes、脚本和文档均通过普通 PR 合入。
- Preview 的 CI 未签名 NSIS artifact 只能用于受限验收，不能宣称为公开可信安装包；公开分发需同时满足 Authenticode（若流程已启用）、updater minisign 签名、SHA-256 和来源元数据要求。
- Tag、Release Notes、资产和 `latest.json` 建立后视为不可变。内容变化回到普通 PR，使用新版本/Build；不得覆盖旧 Tag 或资产。

## 版本号唯一来源（2026-09-30 收敛）

- 应用版本号**只写在** `src-tauri/tauri.conf.json` 的 `version`。安装包文件名、exe 的
  版本资源、关于页显示、更新器比较、诊断日志的 `app_version` 全部由它派生。
- 改版本号 = 改这一行。`Cargo.lock` 里的 crate 版本是占位 `0.0.0`，不随应用版本变化，
  因此发布只需一个文件的一行 diff。
- 不要在两处维护版本号：`Cargo.toml` 的 `[workspace.package] version` 是内部 crate 的
  占位值，`package.json` 不再带 `version`。把版本号写回这两处既不会改变产物，又会让
  安装包/关于页与仓库里的数字重新漂移。
- 守卫：`src-tauri` 的单元测试 `app_version_comes_from_tauri_config` 核对运行期
  `package_info().version` 与 config 一致（删掉 config 的 `version` 会同时丢掉 exe 的
  版本资源并让测试失败）；`generate-updater-manifest.ps1` 继续强制 Tag 与 config 版本一致。

## 本地安装与升级

- 双击安装包发现已有安装时，提供“安装前卸载”和“请勿卸载”：前者正常退出应用、用当前包生成的卸载器移除产品文件后自动继续安装，后者直接覆盖。两者均保留原安装目录、设置、按键映射和启动偏好。静默 `/S` 与被动更新 `/P /UPDATE` 默认覆盖；系统“已安装的应用”及独立 `uninstall.exe` 仍提供卸载。
- 当前用户安装采用 `RequestExecutionLevel highest`：管理员的同账户提升令牌可写受保护安装目录，真正的标准账户仍使用普通权限，不通过其他管理员身份改写 HKCU。安装前检查目标目录或最近已存在父目录的写入权限；权限不足时在请求退出和卸载之前停止。完成页及 `/R` 仍通过 `RunAsUser` 启动普通权限主程序。
- 替换文件前先请求应用正常退出，再确认按键 Helper 的清理终态。每个 Helper 清理阶段等待上限为 45 秒；原助手结束后至多启动一次包内 `--cleanup-only` 恢复并重新计时。`pending` / `unconfirmed` 超时只停止本次文件替换，保留清理记录和仍在等待真实释放的助手，不强杀、不伪造成功；45 秒不是物理按键必然释放的保证。
- 新 payload 写入成功后，仅回收安装目录内的 `sayall-hid-host-helper.exe`、`sayall-component-helper.exe` 和 `SayAllInput/SayAllInput.inf`、`.sys`、`.cat`。未知文件、用户配置和已安装驱动不在清理范围；回收失败不降级为永久删除。
- “安装前卸载”使用当前包生成的临时卸载器和 `/UPDATE` 保留状态，只将明示的旧产品载荷送入回收站；子卸载失败或主程序仍存在时停止，不继续写入新程序。不能调用旧目录内可能过时的卸载器，也不能用卸载的退出码代替文件状态核验。
- `ci-preflight.ps1` 和 Windows CI `verify` 运行隔离的模板、清理状态机及路径规则测试；只有 `installer` job 在完成 NSIS 构建后执行自建文件的真实回收与 compile-only 检查。真实升级和退出验收见 [安装器正常退出验证](Testing/WindowsInstallerGracefulExit.md)。

## Preview

1. 先完成本地测试包的构建和验证，并向用户报告结果；没有用户随后给出的明确预览发布指令时，到此停止。
2. 获得明确发布授权后，从最新 `origin/main` 建立发布分支，确认版本、Build 和说明已冻结。
3. 运行 `scripts/ci-preflight.ps1`；需要深度检查时运行 `-Full`。CI 必须记录 source SHA、构建通道和 artifact digest。
4. 运行 `scripts/verify-windows-bundle.ps1`，确认 NSIS、应用和元数据状态；未签名候选明确标记为 `unsigned-ci-preview-not-for-public-release`。
5. 在 Windows 主机按 [Testing/WindowsReleaseBranchLifecycle.md](Testing/WindowsReleaseBranchLifecycle.md) 完成安装、升级、卸载、启动和设置保留验证；按 [Testing/WindowsRC003Preview.md](Testing/WindowsRC003Preview.md) 执行硬件与第三方语音边界。
6. PR 描述分别列出自动化、安装器、真实 RC001/RC003、VB-CABLE 和第三方输入法结果；不可把 Mac 构建或模拟器结果写成 Windows 真机通过。

## Stable 与 updater 资产

- 正式 Release workflow 必须从精确 Tag/Commit 构建，缺少 `TAURI_SIGNING_PRIVATE_KEY` 或 Authenticode 发布凭据时 fail closed；CI 临时 updater key 只允许测试构建。
- `scripts/generate-updater-manifest.ps1` 生成 `latest.json`、ASCII 安装器名、`.sig` 和 `SHA256SUMS.txt`。清单中的签名是 `.sig` 内容，不是路径；下载地址必须为 HTTPS。
- 发布前打印并核对待上传资产清单、大小和 SHA-256；删除或覆盖远端资产不属于正常重试流程。
- 应用内更新退出前必须显式断开 BLE、释放键态和停止音频；不能依赖 `Drop`，也不能强杀正在连接的旧进程。

## 失败与重试

- Runner、GitHub、网络或签名服务失败且尚无公开身份：在同一 SHA、版本、Build 和 artifact 身份上重试，不新建 rerun 分支、不升版本、不重签已成功字节。
- Release 已创建但验证失败：先只读核对 Tag、资产、digest 和 notes，只补缺失验证；发现字节或来源不一致立即停止并保留现场。
- 任何认证、权限、5xx、超时或无法判断的远端结果均 fail closed。

## 发布报告

报告 source SHA、Tag、版本/Build、workflow Run、artifact ID/digest、安装器签名状态、资产 SHA-256、安装生命周期矩阵、真实硬件/第三方工具结果及 `passed`、`failed`、`deferred` 边界。不得输出证书、私钥、密码、Token 或用户数据。
