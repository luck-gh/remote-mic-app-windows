# 个人版检查更新错误指向上游公共版

- 发现日期：2026-10-15。
- 状态：本地修复、上游整合、本地包覆盖安装与真实个人源检查通过；远端签名发布链路与硬件完整验收仍 deferred。
- 影响范围：个人版 0.8.6 的稳定和预览更新；不限定遥控器型号。
- 功能点：更新来源、下载安装、发布清单与签名身份。
- 现象：个人版提示公共版 1.0.0，说明为“无线麦 SayAll Windows 版”。
- 复现条件：个人版以原内置端点检查更新，GetSayAll 已发布更高版本。
- 正常预期：个人版只检查 `luck-gh/remote-mic-app-windows`；无个人发行版时无更新，个人源失败时报告失败；公共版清单/资产/签名不能变成个人版可安装更新。

## 已确认根因与安装现场

稳定端点与预览 Atom/tag/manifest/路径白名单仍全部硬编码 GetSayAll；清单生成脚本默认仓库也是 GetSayAll，个人 Release 工作流未覆盖参数。公钥沿用上游，因而只修改一个检查地址不能隔离信任。2026-10-15 主机只读核对：安装注册显示“无线麦”0.8.6，未发现运行的主程序，个人仓库 Release 和 Secret 名称列表均为空。本轮没有证据证明公共版已完成安装，不能把提示更新等同已升级。

## 修复与边界

- 稳定与预览发现、清单资产下载地址均限定个人仓库，检查与安装各校验一次来源。生产构建忽略开发端点覆盖；仅 debug 构建允许本机环回端点验证。
- 检查开始清除旧待装包；偏好保存前后均使旧检查失效，迟到结果不得覆盖新代次。
- 个人预览 feed 无发行版的 404 与稳定无清单均返回无更新；网络或其他状态错误保持失败。没有上游 fallback。
- 用户明确授权在仓库外建立独立个人 minisign 密钥；只将公钥写入配置。私钥不输出、不入库、不上传。实际签名校验复用已锁定的 `minisign-verify 0.2.5` 与 `base64 0.22`，与本机 Tauri updater 2.11.0 相同。Release 生成清单前必须对实际安装包验签，不能仅判断 Secret 非空。
- 本地打包默认使用个人密钥并验证配置公钥；仍须本地身份/载荷门禁，不授予发布权限。未来发布需要用户备份私钥并另行配置自己的 GitHub Secret；本轮不操作 Secret、不发布。
- Gitee 尚无用户提供的实际仓库，仍未接入，不以 GitHub 修复代表双源完成。

## 上游整合身份与取舍

原 HEAD `8b9dd31460f1b3b4c6859b2216de61ede0966206`；最新上游 `05c37cf5f740db7f34cc2d9a4270dca7bd2c2e6d`，共同基线 `3ec66566ceabf82915dd1b9ff9a3b10ec163ca26`。上游新增107个提交，个人分支独有33个提交。沿现有分支 `merge --no-commit --no-ff`，保留未提交的原报障文档与 artifacts；不创建额外 worktree，不提交、推送或发布。

接入独立的设备行令牌/中文名称识别、Raw Input 多设备消歧、连接参数观察与成对退订、只读输入法诊断及选择后失焦仍在自身的重新布防。设备令牌只用于内存/UI对齐，不记录身份；Raw Input 选择仅在本进程传递，不写未被个人 Helper 使用的跨进程身份文件。

共享宿主语音合成、计划任务、强制结束 Helper/宿主、自动在线换代、常驻原生键盘抑制与本地既有契约冲突，保留个人 Helper/桥接/正常停止实现；不引入依赖 synth 的“无帧即捕获失效”判据。个人按键/模板页面、音频严格配对、向导 skip/退出/转写证据及布局保持。上游 voice-attempt 两项新测试重复覆盖撕裂快照；个人已有3项测试同时约束启动混合帧、排空混合帧及排空截止时间，整合中保留其实现与测试。首次416项包含这两个导入用例；其后414项没有删除个人契约测试，审查新增隐私/radio边界2项后最终为416项。

独立审查发现上游初始 Raw Input 歧义分支会在未知 BLE 目标时盲选第一支；已将启动、到达与审计统一为纯选择器：未知目标多候选等待；已知地址先过滤，同地址多实例可保留既有绑定；目标缺失不回退其他遥控器。歧义或改绑先释放旧按住状态。相关测试实际先红后绿，共享 Raw Input 日志不再包含路径，物理键观察不再输出真实键码。

审查后吸收上游 10 月 14 日删除自动无线电 Off/On 与 PnP 重启的修复，继续个人既有普通退避/重连、重复 arrival 不清退避契约。依据为本地 DEBUGGING/9月16日 A/B 证据及上游蓝牙鼠标受影响记录；没有进行新的电源或无线电实验。AGENTS 同步零介入目标的具体手段边界，正常退出失败 blocked、禁止强杀不弱化。

上游问题记录只作来源，按[固定提交的原始记录](https://github.com/GetSayAll/remote-mic-app-windows/tree/05c37cf5f740db7f34cc2d9a4270dca7bd2c2e6d/Bugs)查阅，不在本地保留重复副本；其 passed 不外推为个人版通过。上游手册中被本地淘汰的计划任务/共享报告路径不恢复，避免形成相互冲突的操作真源。

## 验证

- `pnpm test`：passed，416/416；新更新源契约3项、审查安全边界2项先红后绿。
- `pnpm build`：passed，类型检查与 Vite 生产输出。
- `cargo fmt --all -- --check`：passed。`cargo test --workspace`：最终 passed 739项（738主套件+1独立诊断子进程），21 ignored；硬件相关 ignored 不算通过。审查前746/23，删除过时系统 radio/PnP 后少7项执行及2项忽略；新增来源选择测试与删除重复 IME 测试各一项抵消，不删除有效个人契约。
- `cargo check --workspace` 与 `cargo check -p sayall-windows-app --features runtime-simulation`：passed。
- 个人密钥签署本地无敏感内容夹具，再用配置公钥实际验签：passed；非空错误口令被拒后改为密钥实际空口令，不修改密钥。
- 错上游公钥与篡改内容两种签名负例实际被拒：passed。
- 首次真实 WebView 仿真被旧安装实例单实例保护拒绝：failed，无报告；正常退出旧实例后隔离仿真18步骤 passed。仿真随后仅修改其 mock 的平台来源/radio 实现，最终生产构建及相关单测通过；该仿真不代表物理设备验收。安装器契约5项、清理状态69项及构建身份夹具 passed。
- 最终本地包 `target/release/bundle/nsis/无线麦_1.0.0_x64-setup.exe`，12,915,240 B，SHA-256 `3ad7fc17a25762187ebb1c266aa1de0abf30dbdd952b6ccd6c78f99583ba1877`。配置公钥对真实 setup 的 updater 签名验证 passed；Authenticode 为 NotSigned，属本地测试包。生产嵌入6个资源，runtimeSimulation=false，内容指纹 `4487283408be7d7c703c2fcd9be074b5003c1c1f57ae3129c5b8f0048d4014c9`，dirty/local/included、sourceCommit=null，不能将原 HEAD 当完整源码。
- 首次 NSIS 下载工具跨卷移动失败（os17）：failed；仅使用 Tauri 官方 `bundle.useLocalToolsDir=true` 本地 overlay 复用已编译生产载荷完成 bundle，没有改全局配置或新建构建目录。原0.8.6正常退出留下清理未确认，现有 cleanup-only Helper 实际完成且门禁 settled 后才安装，无强杀。1.0.0安装后四项载荷核验 passed，主 EXE 与 staging 全字节一致（0差异），SHA-256 `0139067d7a4912129ae9123c0d3ad57d4dbbb156667d411339ab1be62b508955`；注册/文件版本1.0.0，五份既有配置哈希全部保持。
- 安装版真实主 WebView 的稳定/预览 IPC 均返回 currentVersion=1.0.0、available=false；运行日志分别记录 `source=personal_github channel=stable|preview`，稳定 release_not_found、预览无候选，没有公共源回退。无待装更新调用安装被拒“请先检查更新，再下载安装”。原预览偏好false已恢复；截图只证明个人版品牌，不作最终偏好证据。
- 诊断实例主PID64200普通权限，完成后正常退出（overall passed、failed_stages=0、856ms），清理门禁 settled。真实 Explorer 日常启动最终主PID64252/子PID75624均 TokenElevation=0，Helper78564独立提权；窗口“无线麦 · 个人版”，运行内容指纹匹配包，144ms初始IPC。进程级诊断端口9465已无监听，未留下环境/更新源覆盖。
- 个人线上签名下载/自动安装因个人仓库无 Release/Secret 仍 deferred；本轮未配置外部 Secret、未发布。RC001、RC003、ARM64与第三方工具完整矩阵 deferred，不能由编译、仿真或启动外推。Gitee待用户实际仓库。

本机最小证据位于 `target/dev/personal-updater/`：`package/build-metadata.json`、`install-result.json`、`installed-update-results.json`、品牌截图、`runtime-evidence.log`、清理终态与18步仿真报告用于本候选复核，至候选被替代且无未解决争议时回收；自动化日志保留至本次审阅完成。一次性脚本、模拟状态与签名夹具按回收站规则整理，不处理既有 artifacts。私钥只在用户授权的本机仓库外位置保存，须由用户安全备份；丢失将使当前已装客户端无法接受新密钥签发的更新。

## 隐私检查

记录不含私钥、密码、Token、用户数据、蓝牙身份、语音内容或个人绝对路径。新增输入观察保留计数和热键过滤，不把12个真实键码序列写入日志。公钥为可公开信息。
