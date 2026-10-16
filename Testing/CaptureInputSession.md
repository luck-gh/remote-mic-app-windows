# 会话期间临时Capture输入锁定

用户于2026-09-16明确授权“允许受限使用，并接受外部改选时让出控制”。仅此功能在当前用户Voice DOWN准备中使用IPolicyConfig；主程序保持普通权限，不注入第三方程序。

常规页面选择 CABLE Output 等目标麦克风，临时切换开关默认关闭；对应声音写入端按同线身份自动处理，高级诊断保留手动选择。切换三角色读回后才注入目标快捷键，源UP取消迟到启动。BLE owner先释放快捷键，再结束音频/路由；不能在释放排队时抢先恢复其他麦克风。

Console/Multimedia可能联动，写前持久记录before/desired/mask，仅接受组内向目标收敛；第三值和组外改变使本次整体让出。normal原两角色分裂时写前明确拒绝，以免无法精确恢复。崩溃后提供可见恢复/保留，不自动覆盖。没有CAS或通知actor，同值用户操作无法识别；第三方显式设备不保证跟随默认。

## 验证

### 2026-10-12 两端恢复事务与失败回滚

旧 Capture 缺失且新线只有显式 16Ch 可证明时，两个独立选择入口互相要求旧另一端可用，形成选择循环。当前连接页将用户明确选择但自动配对失败的麦克风保留为“尚未保存”草稿，高级诊断“选择并保存两端”以一次 `set_capture_input` 事务提交 Capture 和显式 writer；服务端重新枚举精确身份并严格核对同一适配器，跨线、缺失或歧义仍拒绝。未提交草稿不冒充运行态，不修改临时默认输入切换授权。

准备或保存失败时恢复已提交配置身份，旧设备缺失不再阻断恢复；写入端保持原精确身份和实际 Failed 状态。恢复 Capture 只恢复配置，不依赖硬件读回，也不宣称旧设备重新可用；enabled 保留，但 readiness 标为 unverified，后续语音准备仍走原设备验证。活动语音、取消、超时、迟到及同配置锁门禁保留。worker 不可用或前序请求仍阻塞时的回滚失败必须单独记录，不把 `configuration_uncertain` 解释为已经恢复。

- **passed（红绿与独立审查）**：Windows 宿主隔离测试先复现旧 Capture＋保存失败后运行仍新 Capture，以及显式两端请求被 `render_missing` 拒绝，各 0 passed / 1 failed；页面先 1 failed / 41 skipped，再通过。审查另发现恢复过程仍同步读取旧设备名称，真实控制器线程注入 `REQUEST_BOUND+100ms` 阻塞，实际返回 `route_request_timeout`（0 passed / 1 failed，1.61 秒）；去掉恢复中的硬件读回后转绿。新增真实 worker 的取消/已过期恢复零变更回归；9 项 Windows 控制器及相关 capture_input 共 26 项、宿主事务 14 项、页面＋IPC 56 项通过。缺失目标测试使用虚构端点，未改变现用默认设备或音频通道；不是用 mock 冒充真实失效硬件。
- **passed（组合自动化）**：全部前端 363 项 / 28 文件；`vue-tsc --noEmit`＋`vite build --outDir dist-build` 对应 `pnpm build` 的同一检查和构建，已安装 `vitest run` 对应 `pnpm test`，绕过包管理器联网 bootstrap，无依赖安装或设置修改。`cargo fmt --all -- --check`、`cargo test --workspace`（core 32＋4、Windows 503＋3 集成 / 22 ignored、Host 150 / 1 ignored）、workspace check 和 runtime-simulation check 通过。身份 build.rs 初次格式检查失败仅由原作者格式化后通过；不全量重写用户无关代码。
- **deferred（失效现场 UI）**：目前正常通道是用户已选择的同线 Capture＋16Ch，不为验收改成无效生产配置。消失设备的 UI 全流程恢复、保存失败的实际硬件窗口及 RC001/ARM64/完整断连睡眠矩阵仍未执行；上述软件和真实 controller 隔离结果各按其边界报告。旧 Code 10、标准 pin 识别问题及历史三轮/17:56 未知原因仍未关闭，捕获安全换代与租约补验按用户要求暂缓。
- **本轮回退保护**：新包构建前核对现用 NSIS 为 SHA-256 `5fd60befb83f5889d7b7024cad5c662b886470bff614d37e62a77c5f51e62dd3`（12,853,490 字节）；既有过程目录没有同 hash 包，故仅保留一份 `target/dev/foreground-input/audio-recovery-baseline-5fd60bef-setup.exe`。其用途是组合交付失败时回退已知正常的安装载荷；dirty 源码不能仅按 HEAD 重建该包。新包实际安装、运行身份与功能验收闭合后按 LOGGING 评估回收，不复制源码或覆盖用户当前配置。
- **passed（本轮已安装通道）**：组合包 `228f6221` / EXE `1ec6e08b` 已正常退出后覆盖安装、四配置保持并普通启动；五字段实际运行身份与当轮 manifest/载荷一致，完整身份归 [组合 Preview](WindowsRC003Preview.md#2026-10-12-音频两端恢复与构建身份组合交付)。连接页实际读回目标下拉、paired、无未保存草稿。用户确认闲置首次与连续语音/反馈正常；generation3–5提交193200 / 73440 / 39600 samples、结束队列0、三角色确认与释放恢复 original_mask=7。未执行无效配置或消失硬件场景的生产故障注入。菜单首次失败后恢复另归同主题 Bug，不将音频通过扩为全部功能验收。

以下为此前候选的历史现场与验收，不将其未修复描述替代本节当前软件结论；本轮包、安装与实际运行身份以组合部署记录为准。

- 2026-10-04/05 设备选择简化：旧 capture/render 两端独立保存且 UI 会猜选唯一输出，不能证明声音进入目标麦克风。新增同拓扑适配器 PnP 身份配对，保留已有有效同线输入（包括显式 16ch），无目标时保留原显式写入配置。设备缺失、多义、身份变动或跨线不显示 paired，不选扬声器/耳机/另一条线。关闭默认麦克风切换不关闭声音通道；缺设备时仍能关闭切换。页面提供明确重新连接入口，失败保留实际状态。
- 两端配置在原 capture 配置锁内准备并一次保存；磁盘失败、sink 返回 Ok 但实际 Failed、首次配置失败分别回滚旧两端或清空未配置写入端。Voice DOWN 在该配置/回滚窗口被同锁拒绝，不增加语音按键等待或枚举；启动配对失败清空错误写入端。原角色 lease、外部改选让出、恢复 journal 与快捷键生命周期不改变。
- 2026-10-12 缺失端点边界：本机只读枚举确认已保存的旧 Capture 为 `NOTPRESENT`；新 Capture 与同线 16Ch Render 的严格配对只在显式指定该写入端时通过，未指定时因另一写入端无法证明为标准 pin 而返回 `RenderMissing`，不自动代选 16Ch。代码核对确认现有两端 UI 存在选择循环：先选新 Capture，`set_capture_input` 在保存前仍用旧写入配置配对，遇到 `RenderMissing`；先在高级诊断选 16Ch，`select_audio_endpoint` 先与已失效的旧 Capture 配对，遇到 `CaptureMissing`。重新连接仍使用原目标；启用输入锁定时清空目标会被拒绝，不能以关闭锁定或改选其他麦克风绕过。此为只读现场与代码路径结论，未执行受控 UI 失败复现；两端选择流程修复及通过该 UI 的恢复验收 `deferred`。下述获准离线更新配置后的恢复不证明 UI 循环已修复。
- 同一缺失场景的回滚边界仍未知：配置事务先应用新 Capture，再准备写入端及保存；后两步失败时重新配置旧 Capture，但旧目标已不可用时该回滚也可能在端点校验失败，运行态与持久化状态是否保持一致尚未实测。既有事务测试未覆盖旧目标缺失的真实控制器失败，不扩大上述已验证回滚结论；该失败注入、状态读回与恢复验证 `deferred`。该阶段只读记录边界，后续仅按用户明确许可更新端点选择，没有关闭输入锁定或扩展为事务重构。
- 2026-10-12 11:13–11:16 本地交付 `passed`：旧 App 10780 / Helper 29896 于 11:13:22.950–11:13:23.635 正常退出，日志终态 `overall=passed`、684ms；11:14 覆盖安装时四份配置字节不变。包 SHA-256 `a3ec31552954347c6b3dbc3b4df426f92ea50fdacc2ccf9a5fcdc9d9dac94b4c`（12,848,921 字节），安装主程序 SHA-256 `10f62ab982b9e962d22ce5a98029807f6d92df4180a86505fc4843d8c69cb297`，Helper 保持 `f9ede074` 前缀的既有载荷；`sourceRevision=af710b7+dirty`、`sourceCommit=null`、`build_channel=local`，不能以 HEAD 代替完整源码身份。用户明确同意已核实的新同线通道后，在主程序与 Helper 已退出时仅替换自身设置中的四个 capture/render ID/name 字段；其余设置语义保持、`enabled=true`，模板、向导与恢复 journal 保持。11:16:18 新 App 30736 / 按键宿主 43264 / Helper 42984 启动，主进程由 Explorer 12848 以普通权限启动（token elevated=0）；新 Agent `2026-10-10.request-source` 仍绑定设备宿主 12192，rearm 成功，音频启动严格 `same_adapter` 配对 `passed`。
- 同包 11:19:00–11:19:50 实体 RC003 四次语音会话 `passed`：generation 1–4 分别提交 23040 / 41280 / 35760 / 40560 samples，结束时队列均为 0；四次均先有 `default_roles_confirmed` 再开始语音，释放后四次 `original_mask=7` 恢复，最终 journal 为 null。前后端语音按下/释放各四次成对；自身 UIA 实际观察到按下 badge 的 received 状态及顶部反馈，最后“已收到语音键，语音已结束。”与“按住说话”可见，用户明确确认“按键反馈和语音输入都正常”。首会话在启动约 161 秒后发生，仅证明本机此次首用与连续会话，不外推所有输入法或更长闲置冷态。
- 该候选将“收到语音键”观察与音频准备成功分离：独立 `voiceButtonPressed` 由已过连接代次过滤的 ATVV START/STOP 更新，STOP 在管线 Idle 早退前发布，音频准备失败不抹去已收到按下的事实；断连、失效和退出清除观察，不改变 ATVV、输入切换和快捷键的控制职责。前端与 Rust 定向回归先 red 后 green，前端定向 78 项、Rust 观察契约 4 项及完整前端 355 项 `passed`；Rust 工作区测试（含 Windows 493 passed / 22 ignored、Host 143 passed / 1 ignored）、fmt、workspace check、runtime-simulation check、`pnpm build` 与 NSIS 构建均 `passed`。四项 Rust 测试是控制器/管线组合，不是真实 BLE worker 跨代或失败注入集成验收；新包受控音频失败通道的实际 UI 尚未复现，仍 `deferred`。
- 仍未闭合：旧声卡 Code 10 根因、另一写入端不能识别为标准 pin 的原因、上述 UI 循环和旧目标失效回滚，以及 RC001、ARM64、完整断连/睡眠恢复、所有输入法/更长闲置首用和显式开启连发的矩阵。当前模板返回及四方向未配置连发，继续保留用户选择。日志复用 `target/dev/foreground-input/audio-feedback-lifecycle.log` 等既有目录，长期结论在本页与 [Preview](WindowsRC003Preview.md)、[同主题 Bug](../Bugs/2026-10-03-foreground-input-regression.md) 原位维护；本轮未提交、推送或发布。
- 2026-10-04/05 当轮自动化 `passed`：配对 11 项、音频 25 项（2 原有硬件 ignored）、宿主配置事务及全部工作区、前端 255 项。先 red 复现未准备写入端即保存、配对歧义及配置中语音启动窗口，再 green。当时 Windows 生产 resolver **只读** `passed`：本机一个 Capture、两个 Render 同适配器，选择标准 pin；未打开音频流、未设置默认麦克风、未采集语音。来源与型号边界见 ATTRIBUTION。
- 2026-10-04/05 候选当时的实体 RC003 收音、按住切换/松开恢复、冷首用及外部改选未验收；RC001、多条实体虚拟线与缺失/重现硬件场景 `deferred`。10 月 12 日同页新增实测只覆盖其对应新包与场景，不回填旧候选端到端通过。
- 2026-10-05 00:28 已覆盖安装 `908f6ea0` 候选（完整身份见模板专项），三配置保持。新进程启动日志 `audio_route action=pair result=passed reason=same_adapter saved_writer_retained=true`、`action=startup result=passed`，页面实际显示 `paired`；证明恢复原有正确写入端且界面读回一致，不证明麦克风已收到实际语音。安装后没有程序化改选默认麦克风或用户端点设置。
- 2026-10-05 10:43 布局跟进：用户确认连接状态在上、语音设备并入同一卡片下方横向展开。传输状态、目标麦克风、临时切换说明分列，窄窗自动纵排；下方语音输入设置保持独立，高级诊断仍折叠。仅调整模板结构/样式及方位文案，所有设备操作、禁用条件、恢复提示和事件处理保持。新增归属测试先 red（旧布局 1 failed/40 passed），调整后完整前端 256 passed；生产前端/NSIS 构建、隔离检查 passed。本地包 SHA-256 `e7105ec45b6ce2eade82f1806c2332833f09341adeed288cee62e52546fdf29a`（13,029,355 字节）正常覆盖安装，三配置字节保持，Explorer 普通主程序/独立提权 Helper 及载荷核对 passed。实际宽窗截图确认三列和上下归属，980 逻辑像素宽窗截图确认纵排、目标输入及开关不越界，之后恢复窗口原大小。此轮未重新执行实体录音/切换恢复，不扩大原硬件验收结论。`target/dev/template-audio-ui/connection.png` 原位替换；`connection-narrow.png`、`connection-layout-*` 日志和更新的源码/部署指纹用于本主题验收，闭合或替代后按 LOGGING 回收；未提交、推送或发布。

- 2026-09-19用户真实确认RC003非目标默认输入的冷首按正常，长CABLE Output名称下拉框不再越界；这是用户验收，不补造过期观察器记录。
- 修复前3次role0 setter后被自身external_change中止，PCM为0；组模型覆盖切换、恢复、写前/部分完成崩溃及外部改变。
- 9月22日历史故障：原端点短暂不可用时，恢复在任何 setter 之前失败并保留 generation8 journal，后续请求因 `recovery_required` 中止且0音频包。用户通过旧产品“保留当前选择”执行零 setter 清理后，两次真实语音分别1.694秒/102包、2.340秒/146包，快捷键成对，原三角色恢复通过。该观察证明手动处理后的旧现场恢复；新增自动零 setter 对账分支当时只有23项路由测试通过，未重现端点消失做真机验证，不能混称自动恢复或完整矩阵通过。完整时序保存在 `334c372:artifacts/capture-input-session-20260916/evidence.md`。
- 提交视图自身通过Windows Tauri cargo check、Vue类型检查/构建、19项Windows路由、3项宿主事务、5项页面测试；不依赖未提交driver/HID旁路/模板文件。
- 原精确WASAPI端点重建与配对取消是必要生命周期依赖；此前真实静音恢复已验证，但20秒队列溢出仍未解决，不称本提交修好长期吞吐。
- 耳机/手选、快按/重复/闲置组合、断连/睡眠/退出、crash恢复的完整真机矩阵仍待验证；RC001缺本轮硬件deferred。60秒问题按用户要求暂停。

来源与受限API边界见[ATTRIBUTION](../ATTRIBUTION.md)，剩余验收归[TODO](../TODO.md)。历史证据由 `334c372:artifacts/capture-input-session-20260916/evidence.md` 及同提交的 `first-use-fix/evidence.md` 保存，可用 `git show <提交>:<路径>` 读取。已修复旧错误的中间源码、旧安装包与配置备份不再具有当前恢复用途，回收工作树过程目录；不声称所有中间源码都能由最终 `85fffc84` 逐字重建，不提交私有配置、设备身份或语音。

旧音频调查仅在原 `artifacts/audio-sink-recovery-20260915/` 保留19秒内部队列溢出的唯一故障安装包和 `physical-voice-observation.json`，供尚未闭合的生产故障对照；归因闭合或复现用途明确取消后再回收。其他逐轮日志、源码及60秒实验目录退役，必要历史正文与七份精确档案源均可从 `334c372` 的原路径读取；这不把60秒远端停止认定为硬件定时上限，也不恢复已暂停调查。上述历史整理未改动当时语音路径，历史失败不能因整理记为通过。

上述历史候选的源码复现以提交 `85fffc84d41c3bdaf822e49ab115e1d922c75e42` 为准，不代表本页 2026-10-12 的 dirty 候选。2026-09-19 核对提交候选目录的 235 个受管文件与该提交内容一致（允许 CRLF/LF 换行差异），其余 13 个文件均为构建或测试生成物；当时已清理 `candidate/`、临时 diff、索引和提交准备文件。2026-10-05 再核对，候选 tree 与21个记录的 blob 均等于该提交；早期18项清单中的 `lib.rs`、`button_mapping.rs` SHA不符，早期测试日志不能代表最终提交完整验证。提交准备目录已无独有实现，按本次授权整体回收；真实验收的有效结论与未验边界以本页及固定历史证据为准，不因清理改判。
