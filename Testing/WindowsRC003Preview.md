# Windows RC001 / RC003 Preview 测试手册

## 适用范围

- 仓库：本 Windows fork；官方上游为 `GetSayAll/remote-mic-app-windows`
- 平台：Windows 10 1809+ / Windows 11 x64
- 硬件 A：小米蓝牙遥控器 2 / RC001
- 硬件 B：小米蓝牙遥控器 2 Pro / RC003
- 当前状态：WinRT BLE / ATVV / PCM / WASAPI、设备名与 2A24 型号识别、端点持久化、退避重连、睡眠恢复、Raw Input、本机使用统计和 Windows 10 1809 双层版本门禁代码路径已实现；Windows、RC001、RC003、VB-CABLE 真机验收均为 deferred

## 测试前准备

1. 记录 Commit、版本、Windows 版本和电脑蓝牙适配器。
2. 分别准备真实 RC001 和 RC003，并确认两者都可以在 Windows 蓝牙设置中完成配对。不得用其中一种型号的结果代替另一种。
3. 安装待测构建。
4. 若系统尚未安装 VB-CABLE，在 SayAll 可见安装结束提示中选择打开 VB-Audio 官方页面；下载 Pack45，解压后以管理员身份运行对应架构的 Setup，并重启 Windows。SayAll 不复制、下载或执行第三方驱动包。
5. 准备记事本和至少一个真实语音输入应用。
6. 保存测试开始前的应用日志。

## 用例零：Windows 版本门禁

1. 在低于 Windows 10 1809（build 17763）的隔离虚拟机运行 NSIS 安装器。
2. 确认安装器显示最低版本提示并退出，应用文件、快捷方式和卸载条目均未创建；WebView2 bootstrapper 是否已发生动作需单独记录。
3. 将已解压的应用 exe 直接复制到同一虚拟机并运行。
4. 在 Windows 10 1809 x64 和当前 Windows 11 x64 分别安装并启动同一候选。

预期：低于 build 17763 时，安装器在复制应用文件前以退出码 1633 失败关闭；直接运行 exe 时在创建 WebView、BLE、WASAPI 或 Raw Input 资源前显示原生中英文提示并退出。Windows 10 1809 和更高版本不触发版本拒绝，继续进入正常安装与启动流程。

失败判定：只在文档声明最低版本；低版本仍复制应用文件或启动后台平台线程；直接运行 exe 崩溃、静默退出或进入主界面；Windows 10 1809 被误拒绝；把 CI 编译或纯逻辑测试表述为已完成上述虚拟机/真机验收。

## 用例一：首次连接

1. 只配对 RC001，启动无线麦并打开“连接”。
2. 点击扫描，选择 RC001 并连接。
3. 确认界面显示“小米蓝牙遥控器 2（RC001）”。
4. 断开并移除 RC001 配对，改为只配对 RC003，重复扫描和连接。
5. 确认界面显示“小米蓝牙遥控器 2 Pro（RC003）”。
6. 对于名称只能归入白名单、不能确定型号的设备，确认连接时尝试读取标准 Model Number（2A24）；读取失败时显示“型号待设备确认”，但不因此阻断 ATVV。

预期：只显示名称在批准白名单内的 RC001/RC003 候选；零个或多个候选时不猜测。型号来自精确设备名或标准 2A24，不使用两者共用的 HID VID/PID 猜测。页面依次反映连接、特征发现、能力确认和就绪状态；只有收到有效 16 kHz ATVV 能力后才显示“BLE / ATVV 已就绪”。

失败判定：RC001 被标成 RC003，或 RC003 被标成 RC001；仅凭共用 HID VID/PID 判定型号；2A24 不可读时拒绝原本可用的 ATVV；仅进程运行或发现设备名称就显示 ATVV 就绪；把 ATVV 就绪显示成系统麦克风可用；连接失败后需要清配置或重装。

## 用例二：首次语音会话

1. 确认连接页显示“BLE / ATVV 已就绪”。
2. 按住当前型号的语音键并说一整句。
3. 观察页面进入“正在接收遥控器语音”，确认解码采样数增加。
4. 释放语音键。
5. 分别使用 RC001 和 RC003 重复步骤 1–4，两者都必须在第一次会话成功。

预期：第一次会话即完成一组 `STREAM_START → AUDIO → STREAM_STOP`；会话代次只增加一次；解码采样数增加；释放后回到 ATVV 就绪。WASAPI 完成前不要求文本框收到语音。

失败判定：第二次或第三次才开始收到采样、重复启动、释放后仍显示流式接收、旧连接的断开或通知结束了新连接。

## 用例三：极速和连续会话

1. 连续执行 20 次快速按下/释放。
2. 连续执行 20 次 2～5 秒语音。
3. 在上一段文字刚完成时立即开始下一段。

预期：每次按下和释放严格配对，没有卡键、重复音频、跨会话尾音，旧连接通知和上一代停止事件不能结束新连接或新会话。

## 用例四：断开和恢复

1. 建立 ATVV 就绪连接后，让遥控器断电或离开蓝牙范围，记录重连阶段和每次尝试间隔。
2. 保持设备不可用至少 70 秒，确认间隔按约 2、4、8、16、30、30 秒封顶，而不是忙循环。
3. 让遥控器恢复可用，确认一次成功能力协商后重连计数归零，第一次语音即可使用。
4. 点击“断开”，保持应用运行至少 35 秒，确认本次运行不再自动重连；重新选择设备后恢复自动重连。
5. ATVV 就绪时让 Windows 睡眠再唤醒，确认页面先显示睡眠释放，再重新经历连接、发现、能力确认。
6. 关闭再打开 Windows 蓝牙，重复步骤 1～3。
7. 应用退出再启动，确认自动恢复上次明确选择的遥控器及其已识别型号；若首次失败，继续按退避策略重试。
8. 分别使用 RC001 和 RC003 完整执行本用例。

预期：每次失败或睡眠都先释放通知、GATT service、device、ATVV pipeline 和活动音频；旧会话通知不会改变新会话；主动断开不会在本次运行反弹连接；恢复后第一次语音正常，不要求清配置。

失败判定：无间隔忙重试、清理失败后覆盖式创建新代次、主动断开后自动反弹、睡眠后仍显示旧会话就绪、恢复后需要第二次或第三次语音才成功。

## 用例五：音频端点

1. 不安装 VB-CABLE，启动 SayAll，确认连接页自动检测但不会伪造 CABLE Input，并显示“需要安装 VB-CABLE”、官方下载和重新检测入口。
2. 点击“打开官方下载页”，确认由系统默认浏览器打开 `https://vb-audio.com/Cable/`，没有在 WebView 内替换 SayAll 页面。
3. 从官方页面安装 Pack45，但先不重启；点击“重新检测”，分别记录端点是否出现，不把偶然出现视为无需重启。
4. 重启 Windows 后启动 SayAll，确认自动检测到唯一的 CABLE Input；若此前没有保存其他音频端点，应自动选择并显示“WASAPI 已就绪”。
5. 如果此前明确保存过其他输出端点，确认自动检测不会覆盖已有选择，再手动选择 CABLE Input。
6. 按下语音键完成一次语音会话。
7. 在 CABLE Output 侧录制或监听，确认首尾完整且没有重复。
8. 语音期间尝试切换端点，然后在空闲时切换。
9. 退出并重新启动无线麦，确认仍恢复同一个 endpoint ID 和名称，且没有改用系统默认输出。
10. 禁用或移除当前端点后重新启动，再次开始语音。
11. 让同一个 endpoint ID 的显示名称发生变化后重新启动；如果驱动无法稳定复现该状态，记录为 deferred，不得用另一个 ID 替代。

预期：缺少或未选择端点时语音失败关闭但 BLE 连接保持；语音期间禁止切换；正确端点时 16 kHz PCM 进入 CABLE Input，并可从 CABLE Output 收到；`STREAM_STOP` 后等待 WASAPI padding 清零再结束会话；重启只恢复稳定 ID 与原名称都一致的端点；端点缺失或名称变化时保留原选择用于提示，但 WASAPI 不进入就绪，也不退回系统默认输出。应用不读取或修改 Windows 默认输入、输出。

失败判定：自动使用默认扬声器、只按名称匹配导致误选、端点 ID 或名称变化后仍自动恢复、队列无限增长、停止时直接丢弃尾音、端点失败后仍显示系统语音可用。

## 用例六：可靠按键

1. 在“按键”页面点击“启动监听”。
2. 确认页面只匹配一个 RC001/RC003 共用设备族的 Raw Input 路径；零个或多个匹配时记录错误并停止，不得自动选择第一个。
3. 逐个测试 Windows 公共 API 能稳定收到的按键：按下、释放、持续按住、系统重复、快速连续。
4. 对同一实体按键核对原始事件数和语义边沿数，确认 Keyboard 与 HID 同时报告时不会产生两次语义按下。
5. 按住普通按键时点击“停止监听”，再重新启动监听。
6. 按住并释放语音键，确认普通按键最近边沿和语义边沿计数不因语音键变化。
7. 分别使用 RC001 和 RC003 完整执行步骤 1–6，单独记录两种型号的 Keyboard、HID-only 或双路径报告形态。

预期：监听使用隐藏 message-only window 和 `RIDEV_INPUTSINK`；每个事件都按启动时唯一选中的完整设备路径过滤；一次实体动作只产生一次语义按下和一次语义释放；重复 key-down 不重复触发；停止时释放仍处于按下状态的普通键；线程未能停止时页面必须显示失败，不能伪装为已关闭；普通键盘不被映射；语音键不进入普通动作路径。

失败判定：零匹配仍显示就绪、多个匹配时取第一个、其他键盘改变最近按键、一次按住产生多次语义按下、Keyboard/HID 双路径重复触发、停止后残留按下状态、语音键进入普通按键统计或动作路径。

## 用例七：安装升级

1. 安装旧候选并修改配置。
2. 安装新候选。
3. 检查安装条目、配置、映射、统计和日志。
4. 卸载应用。

预期：升级只保留一个安装条目并保留用户数据；卸载范围与界面说明一致。

## 用例八：映射保存与 SendInput

1. 在“按键”页面选择一个普通按键，并从页面内按钮网格选择快捷键预设。
2. 点击“保存映射”，退出并重新启动应用，确认映射从 `button-mappings.json` 恢复。
3. 在记事本中点击“测试当前快捷键”，分别测试单键、`Ctrl+C` / `Ctrl+V` 和 `Win+D`。
4. 测试左右修饰键、方向键、Home/Delete 等扩展键，并核对按下顺序和逆序释放。
5. 通过自动化故障注入模拟 SendInput 只提交部分事件以及调用后抛出未知错误。

预期：一个快捷键的全部按下和逆序释放通过一次 SendInput 批量调用提交；部分提交只释放可能已按下或尚未释放的键；未知交付状态逐键 best-effort 释放全部可能按下的键并报告失败；保存后无需重启即可用于显式测试。真实 RC001/RC003 边沿当前都不会自动执行映射。

失败判定：逐键分别调用 SendInput、按键释放顺序与按下相同、部分失败后修饰键卡住、无效或超过四键的映射被保存、测试失败仍显示成功、未完成真机来源确认就自动把 Raw Input 边沿注入前台应用。

## 用例九：NSIS Preview 安装包

1. 从对应 Windows CI Run 下载名称包含精确 source commit 的 `sayall-windows-unsigned-preview-*` artifact。
2. 确认 artifact 只包含一个 `*-setup.exe`、`SHA256SUMS.txt` 和 `build-metadata.json`。
3. 重新计算安装器 SHA-256，并与两份记录逐字匹配。
4. 确认 metadata 中 productName、identifier、publisher、版本和 source commit 与仓库一致，且 `signatureStatus` 为 `NotSigned`、`distributionStatus` 为 `unsigned-ci-preview-not-for-public-release`。
5. 确认 Windows CI 的 `Test silent current-user install and uninstall` 步骤通过：`/S` 安装后只有一个 HKCU 卸载条目、安装目录位于当前用户 `LOCALAPPDATA`、开始菜单只有一个入口，已安装进程持续运行 8 秒后由测试关闭。
6. 确认同一步骤使用注册表中的真实卸载命令执行 `/S` 卸载，程序目录、开始菜单和卸载条目均移除；位于 Tauri `app_config_dir` 的测试标记仍保留，证明默认静默卸载不会删除当前用户设置目录。
7. 在未安装 VB-CABLE 的隔离 Windows 测试用户中手动运行可见安装器，确认安装完成后提示 VB-Audio、Donationware、管理员权限和必须重启；选择“是”只打开官方页面，选择“否”仍能完成 SayAll 安装。
8. 安装 VB-CABLE 后重新运行 SayAll 安装器，确认检测到 `VBAudioVACMME` 服务后不再显示缺失提示；不得仅通过伪造普通用户注册表值冒充驱动安装。
9. 确认 `/S` 静默安装、升级和 CI 不弹消息框、不打开浏览器；继续检查可见语言选择、安装界面、开始菜单启动、退出、再次启动和卸载界面。

预期：CI 从干净提交构建唯一 NSIS 包；校验脚本拒绝错误产品身份、多个安装器、异常小文件、签名状态不符合当前 CI 边界或摘要不一致。静默生命周期必须证明当前用户安装、使用 Tauri updater 兼容的 `/S /UPDATE` 升级路径在等待旧卸载器退出并经过有界稳定窗口后收敛、单一卸载条目、单一开始菜单入口、进程不立即崩溃、程序文件完整卸载和设置目录默认保留。SayAll 安装器本身无需管理员权限；用户另行启动的 VB-CABLE 官方驱动安装需要 UAC。安装器不允许用旧版本覆盖新版本。

失败判定：artifact 无法绑定精确 commit、存在多个候选安装器、SHA-256 不一致、CI 包意外带未知签名、静默安装非零退出、安装到当前用户目录之外、卸载条目或开始菜单入口不唯一、应用进程立即退出、静默卸载残留程序文件或误删设置标记、安装器要求全局管理员权限、把未签名 artifact 当作公开发布包。

当前自动化状态：第 2 项生命周期矩阵此前失败在降级用例：Tauri 2.11.1 的 `/S` 静默模式不会执行负责设置版本比较结果的重装页，`/S /P` 仍属于 silent，因而内置 `allowDowngrades=false` 检查均未阻止 predecessor。现已在既有 preinstall hook 中直接读取本产品 `DisplayVersion` 并以 Tauri SemVer 比较器拒绝旧版本，静默拒绝约定退出码为 1638；Windows CI 已通过，详见下方 Run 记录。该结果仍不替代真机验收。

已完成：Windows CI Run [`33637195089`](https://github.com/GetSayAll/remote-mic-app-windows/actions/runs/33637195089) 通过全部生命周期步骤。`0.0.1` predecessor 以 `/S` 安装，`0.1.0` 以 `/S /UPDATE` 升级；随后 predecessor `/S` 被 preinstall SemVer 门禁拒绝并返回 1638，安装版本仍为 `0.1.0`，主程序、设置、映射、统计和单一安装身份均未变化；最终静默卸载移除程序、快捷方式和卸载条目并保留用户数据。该 Run 仍不替代可见安装界面、SmartScreen、Windows 10 1809 / Windows 11 真机或历史二进制兼容性验收。第 3 项 Authenticode 演练可在此矩阵通过后开始。

## 用例十：隐私安全诊断摘要

1. 依次在未连接、连接中、ATVV 就绪、语音流式、排空、断开和失败状态打开“权限”页面。
2. 点击“生成摘要”，确认页面内 JSON 的阶段、能力、代次和计数与同一时刻的连接、音频、Raw Input 和 SendInput 状态一致。
3. 搜索摘要，确认不存在真实设备 ID、蓝牙地址、完整 HID 路径、遥控器名称、音频端点 ID/名称、错误原文、窗口标题、语音内容或用户文本。
4. 点击“复制摘要”，粘贴到本地记事本，确认内容与页面内可见 JSON 逐字一致。
5. 在 980 × 700 最小窗口和 1120 × 800 默认窗口重复生成，确认权限状态、按钮、摘要和滚动均可访问，没有横向裁切。

预期：摘要由当前运行快照即时生成，不读取其他 App 或系统私有数据；敏感字段在 Rust 诊断结构生成阶段即被排除。复制失败必须显示错误，不能显示成功或静默上传。

失败判定：摘要包含任何设备或用户身份、路径、端点名称或错误原文；状态与当前快照不一致；浏览器预览被描述为 Windows 可用；未点击时自动读取或复制；剪贴板写入失败仍提示成功；页面出现横向溢出或中文小于 12pt。

## 用例十一：本机使用统计

1. 记录测试前“统计”页面今日、本周和全部的按键次数、语音次数与语音时长。
2. 启动 Raw Input，执行 10 次普通按键完整按下/释放；包含持续按住产生的系统重复，以及 Keyboard/HID 双路径同时报告的按键。
3. 按住并释放语音键 3 次，确认语音键不增加普通按键次数。
4. 完成 2 次包含真实音频、且 WASAPI 排空成功的语音会话；另制造 1 次端点失败或断连中断的语音会话。
5. 保持统计页打开，确认数据在约 1 秒内更新；依次切换今日、本周和全部，并核对最近 7 天对应本机日期。
6. 退出并重新启动应用，确认统计不丢失、不重复累计；跨本机自然日后重复一次普通按键和一次完整语音。
7. 从上一候选版本升级后再次启动，确认旧设置、按键映射和已有统计仍保留。

预期：每个去重后的普通按键语义按下只增加 1 次；系统重复、对应释放、语音键和 Keyboard/HID 双路径不重复累计。只有 `STREAM_STOP` 后 WASAPI 排空及 ATVV drain 均成功的会话增加语音次数；语音时长按该会话 16 kHz 已解码采样数折算。每日数据按本机日期保存，页面提供今日、本周、全部与最近 7 天汇总；退出前最后增量落盘，重启和升级不重置或重复。`usage_statistics` 统计字段不得包含设备 ID、蓝牙地址、HID 路径、遥控器/按键名称、端点身份、语音内容、识别文字或第三方 App 上下文；应用原有的用户明确选择设备设置按既有配置边界保存。

失败判定：一次实体按键增加多次、语音键进入普通按键统计、中断会话被记为完成、页面只显示进程内计数而重启归零、升级丢失或重复统计、日期跨天错误、统计文件含任何设备或用户内容、统计写入阻塞 BLE/Raw Input 回调而造成语音或按键响应退化。

## 用例十二：Windows Tauri/WebView 运行时仿真

1. 以 `runtime-simulation` Cargo feature 和 `VITE_SAYALL_RUNTIME_SIMULATION=1` 构建专用测试程序；普通构建不得包含仿真前端入口或仿真专用 Tauri command。
2. 在 `windows-latest` 启动该程序，并设置唯一的运行报告路径；不得向真实桌面发送 SendInput，也不得尝试扫描真实 BLE、音频或 HID 设备。
3. 由实际 Windows WebView JavaScript 依次通过 Tauri IPC 读取运行快照、首次检测并自动选择唯一仿真 CABLE Input、渲染 RC001/RC003 扫描结果、连接 RC001、启动 Raw Input、保存并显式测试 Ctrl+C 映射。
4. 依次打开驱动、按键、模板、权限、设置和连接页面；在权限页生成诊断摘要，确认平台明确标记为 `windows-ci-simulation`；在设置页确认顶部（应用标识 / 当前版本 / 检查更新）与“通用”“问题反馈”分组标题，切换“应用图标”并确认 IPC 与持久化往返，再点击“官网”“GitHub”两个入口。页面无成功提示只验证交互，IPC 成功只证明系统打开请求已提交；实际浏览器结果须在用例十四核对。被 opener capability 拒绝（`Not allowed to open url`）判失败，runner 上没有可用默认浏览器则如实记为 deferred。
5. 通过测试专用 command 驱动纯 Rust ATVV 管线完成首次 `STREAM_START → 40 + 80 AUDIO → STREAM_STOP → DRAIN`，确认得到 240 个采样、generation 为 1、连接和音频均回到 ready。
6. 停止 Raw Input 并断开，确认最终快照为 `rawInput.phase = stopped` 和 `connection.phase = disconnected`；程序写入报告并自行以成功退出码结束。

预期：真实 Windows WebView、Tauri invoke 和 Rust command 边界完成闭环；六个侧栏页面均能挂载；RC001/RC003 和 IPC camelCase 数据可被 Vue 消费；测试专用 SendInput 只记录批次和事件数，不向桌面注入；普通生产构建经字符串检查不包含仿真平台名称或仿真 command。入口存在不代表本轮已执行，结果按实际报告记录。

失败判定：只调用 Rust 单元测试或浏览器预览而没有启动 Windows Tauri WebView；普通构建能启用仿真；测试误调用真实 BLE/WASAPI/Raw Input/SendInput；WebView 进程存活但没有完成报告；把仿真 240 个采样、就绪状态或页面渲染表述为 RC001/RC003 真机通过。

## 用例十三：NSIS 安装生命周期矩阵

1. 使用配置覆盖构建版本低于当前版本的同源码 NSIS predecessor fixture，将其移出当前 bundle 目录，再按普通生产配置构建当前 NSIS。
2. 静默安装 predecessor，确认只有一个 HKCU 卸载条目和一个开始菜单入口，安装位置位于当前用户目录。
3. 在本应用配置目录写入有效的 `settings.json`、`button-mappings.json` 和保留标记；设置夹具包含遥控器选择、音频端点、增益和每日使用统计，并记录两个 JSON 的 SHA-256。
4. 静默安装当前版本，确认版本升级、安装位置不变、卸载条目和快捷方式仍各一个，两个 JSON 的 SHA-256 逐字节不变；启动升级后程序并确认持续运行 8 秒。
5. 以 `/S` 再次运行 predecessor 安装器，确认退出码为 1638，且当前版本、当前主程序 SHA-256、设置、映射、统计和单一安装身份均未被替换。
6. 通过当前卸载条目的真实静默命令卸载，确认程序目录、快捷方式和卸载条目移除，而设置、映射和保留标记继续存在。

预期：同一产品身份只存在一个当前用户安装；覆盖升级不改变本应用用户数据；较低版本安装器在复制文件前由 preinstall SemVer 门禁以退出码 1638 拒绝，不能把当前版本降级；最终卸载边界与现有约定一致。

失败判定：使用当前版本直接覆盖当前版本冒充升级；升级后出现两个安装条目或快捷方式；设置、映射或统计被重写；降级后版本或主程序发生变化；卸载残留程序身份或误删用户数据；把同源码 predecessor fixture 表述为真实历史版本兼容性验收。

## 用例十四：设置页结构、外部入口与应用图标

1. 打开侧栏最后一项，确认名称是“设置”、图标是齿轮，页面标题为“设置”；顶部一行为应用图标 + “无线麦 SayAll” + “让语音触手可及”，其右为“当前版本”与版本号（应与当前实际安装包版本一致）、“检查更新…”按钮、“检查预览版更新”开关，再往下是分隔线与更新状态。
2. 确认页面里没有说明性小字：按钮旁不再有“默认关闭。预览版包含新功能……”、“外观”下不再有“该选择会在重启后保持。”或“跟随 Windows 的应用颜色模式。”。
3. 确认随后是“通用”（外观 / 启动行为 / 应用图标）与“问题反馈”（官网 / GitHub）两个分组，且诊断摘要与日志目录仍只在“权限”页。
4. 点击“官网”，确认系统默认浏览器打开 `https://sayall.app/?from=win`（地址必须带 `?from=win`）；返回应用后页面**不显示**任何成功提示。
5. 点击“GitHub”，确认系统默认浏览器打开 `https://github.com/GetSayAll/remote-mic-app-windows`，同样不显示成功提示。
6. 通过测试替身覆盖打开失败，确认页面就地显示原因而不是静默无反应；自然出现无默认浏览器等真实失败时另记结果，不为本用例修改系统关联或安全策略。错误若为 `Not allowed to open url`，说明 opener capability 白名单与 `bridge.ts` 的常量不一致，属配置回归而非环境限制。
7. 在“应用图标”里选“几何鸭”，确认三处同时换成几何鸭：本页顶部标识、窗口与任务栏（含 Alt-Tab）图标、通知区域（托盘）图标；切回“默认”确认三处一起还原。日志应出现 `app_icon action=apply ... applied=faceted-duck` 与 `target=window` / `target=tray` 两条结果。
8. 重启应用，确认应用图标与外观、启动行为一样被保持。

预期（2026-10-02 吸收的上游设置页契约，本机结果另记）：顶部模块集中应用标识、版本与检查更新；“通用”“问题反馈”分组卡片与 Mac 新设置页同构，Windows 没有的模块不出现；外部入口只打开 capabilities 白名单内的固定 URL，成功不打扰、失败给原因；应用图标切换对齐 Mac `AppIconController`（稳定语义 ID、未知 ID 回落默认、切换落结构化日志），窗口/任务栏与托盘一起换，安装包与快捷方式图标不随切换变化。

失败判定：WebView 自身跳转，或入口可打开白名单外的任意 URL；点击失败但页面无任何提示；正常安装上出现 `Not allowed to open url`；官网地址缺少 `?from=win`；成功后又出现“已在系统默认浏览器打开…”这类提示；诊断摘要入口重回设置页；侧栏底部仍显示“预览版”或版本号错误；新装没有默认选中“几何鸭”；旧配置升级后仍停在水彩鸭（迁移没有生效）；安装目录里的 exe 或安装包图标不是几何鸭（构建图标回归）；切换应用图标后窗口/任务栏、托盘或快捷方式有一处没换、失败时单选按钮停在未保存的位置、或出现 Mac 菜单栏的单色状态图标（Windows 没有该图标）；把浏览器预览或仿真结果表述为 Windows 真机通过。

## 用例十五：语音增益

1. 打开“连接”，确认“语音设备”卡片里 CABLE 端点下方有“增益”滑块与读数（初始为 `0 dB`），以及说明“0 dB 保持原始音量；数值越大声音越响，也会放大环境噪声。建议先从 6–12 dB 开始。”。
2. 选一种判定口径（按实际使用方式，2026-10-04 Andy 口径）：遥控器语音通常**直接进输入法、操作人听不到声音**，因此默认用 ① **转写文字**——同一句话分别在 0 dB 与 24 dB 下各说一次，比较识别结果并记录；若音频可监听（例如系统直接回放 CABLE Output）才用 ② 听感；需要客观数字时用 ③ 回录电平（`crates/sayall-windows/examples/cable_level_probe.rs` 逐秒输出 CABLE Output 的 RMS/峰值，两段的能量 RMS 比值应显著大于 1；24 dB 理论 ×15.85，若削波会小于理论值但必须明显 >1）。
3. 把滑块拖到 12 dB，确认松手后读数变为 `12 dB`；日志出现 `audio_settings feature=gain action=save phase=completed terminal_result=passed gain_db=12`。
4. 再按一次语音键，用同一音量说同一句话。日志应出现 `audio_gain action=apply phase=completed terminal_result=passed generation=<n> gain_db=12`（每会话一条；会话内再拖滑块会按新值补一条）——这是“该轮会话生效增益”的直接证据，不依赖时间反推；配合步骤 2 的三种判据之一给出结论。
5. 把滑块拖到 0 dB 复测一次；再拖到 24 dB 确认应用不报错（若音频可监听，此时应明显更响/可能削波）。
6. 完全退出并重新打开无线麦，确认读数仍是上一次的值；日志出现 `action=restore ... gain_db=<值>`。
7. 在语音会话进行中拖动滑块（松手生效），确认下一句语音按新增益处理，且会话不中断、按键按下/释放仍成对。
8. 分别用 RC001 与 RC003 执行步骤 2–4 与步骤 7，单独记录两种型号结果（RC003 已于 2026-10-04 完成，见“当前自动化记录”；RC001 仍 deferred）。

预期：滑块拖动过程只更新读数，松手才落盘；生效值以保存返回值为准（越界被钳制到 0–24 dB）；增益在解码后、写入 CABLE 播放端前生效，改动下一批音频立即生效；重启后恢复；会话生命周期、断连与睡眠恢复不受影响。

失败判定：拖动过程中每个刻度都写一次设置文件；保存失败仍显示新值或静默失败；重启后回到 0 dB；增益未生效（回录电平两段比值 ≈1×、或转写结果与 0 dB 无差别且无其他解释）或整段音频被错误静音/清零；只验证了编译、单测或仿真就宣称真机通过。

## 日志收集

日志不得包含语音内容、识别文字、真实蓝牙地址、完整 HID 路径、窗口标题或个人文档路径。报告应包含 Commit、版本、时间段、Windows 版本和失败步骤。

## 验证边界

- Mac 自动化：前端构建、Rust 核心测试、格式化和静态检查。
- Windows CI：Windows 编译、自动化测试、测试专用 Tauri/WebView/IPC 仿真、当前用户静默安装/启动存活/静默卸载和设置目录保留边界；不包含可见安装界面、SmartScreen、Windows 10 1809、真实硬件或第三方应用验收。
- 用户/维护者：RC001 与 RC003 分别的型号识别、蓝牙、音频、Raw Input、输入法、睡眠和安装升级真机验收。

## 当前自动化记录

### 2026-10-09 本 fork 合并候选

源码身份：本地 `50f21e005b4f1bf3455268eb013d58259196d6a2` 合并上游 `e225bb4c9791dbfefa37c8fb197bc06e95fca428`（0.8.5），本段随合并提交保存；原有个人版与返回键诊断工作在合并提交之后恢复，不包含在下列基础合并结果中。

- **passed（Windows x64 自动化）**：`cargo test --workspace`：core 33、场景回放 4、Windows 平台 473、平台集成 3、宿主 152 项，21 项硬件/交互测试 ignored。`cargo fmt --all -- --check`、`cargo check --workspace`、宿主 `--features runtime-simulation` 编译通过。
- **passed（Helper 与安装契约）**：Helper release 单元测试 56 passed / 3 ignored；Agent 78、清理 69、受限回收 8、安装覆盖宏/正常退出契约 5 项通过。前端完整 353 项测试及生产构建通过。
- **回归证据**：焦点取消后的排队请求重开、退出后新请求进入两个用例均先 failed，增加代次/退出门后焦点套件 43 passed；按键宿主测试覆盖 DOWN 超时放行后迟到决定不能吞 UP，以及退出/重启跨边沿配对。保留模板实际执行、音频同线配对和失败回滚测试。
- **deferred**：本合并候选的 RC001/RC003 实体按键与物理键盘共存、冷态首用/连续语音/默认麦克风恢复、真实应用焦点、断连/睡眠/持键退出、ARM64 主机及完整安装/卸载矩阵、WebView 真实崩溃恢复。未把单元测试、编译或上游历史实验当作这些场景通过。
- 正常执行前曾出现 sandbox 工具权限/loopback 限制；改在当前 Windows 主机执行后通过。旧测试 PID 占用导致 Helper debug 链接失败，最终 release 测试通过；用户仅授权结束该测试进程后已核其退出，正式应用未被强杀。
- 临时日志复用 `target/dev/upstream-sync/` 和 Helper 核对所用 `target/dev/upstream-merge/`；留到候选验证完成或被替代后回收，不复制源码、缓存或安装包留档。

### 同日恢复原有未提交改动后的验证

本地合并提交为 `9f4ead50bcd429ef9bfe448cec54aeede936bf7f`，父提交分别为原工作基线 `50f21e0` 和上游 `e225bb4`。恢复合并前的个人版品牌与 RC003 返回键诊断补丁后，继续保持未提交；新增向导也沿用个人版显示，固定图标不恢复旧图标选择。原文件存在/删除状态全部保持，个人 SVG 母版 SHA256 仍为 `40cc8382d2e16ef060abac300d5f3fb9bfaf48ece841939d2eeac3151d9d238e`。

最终 **passed**：前端 348 项、类型检查与 Vite 构建；Rust 工作区 660 项（core 32、场景 4、Windows 478、集成 3、宿主 143），21 ignored；fmt、普通与 runtime-simulation 编译检查。测试数变化来自个人版移除已退役图标/反馈流程及恢复诊断回归，不是跳过失败测试。映射诊断与桥接定向 40/25 项通过；真实合成快捷方式的受限回收及 5 项安装脚本契约通过。日志为 `target/dev/upstream-sync/*restored*.log` 与 `target/dev/upstream-merge/*restored*.log`，按本节留存条件收尾。未运行真实安装器、未改变现有安装实例，前述实体按键、真实收音、冷态/睡眠、ARM64 等边界仍 **deferred**。

安装器当前钩子的安装/卸载段汇编为 **deferred**：复用已有 `target/.tauri/NSIS/makensis.exe`，第一次受 PowerShell 原生命令启动器输出编码错误阻止；第二次显式 stdout/stderr 重定向仍报系统 DLL relocation，编译器未启动，未生成或执行安装包。钩子 SHA256 `2e2c4d2c11cc083d4e88975aeb9c15b245d4d45c7f1db724d44e3bd0712af22c`；摘要 `target/dev/upstream-sync/installer-hooks-makensis-summary.log`，本次 4 份探针文件已回收。此环境失败不算 NSIS 语法或生命周期通过，也不影响已通过的 Rust/前端验证；真实出包前须补此项。

### 同日按用户要求覆盖安装并打开（11:28）

用户明确要求“安装并打开”后，以 `9f4ead5` 加已恢复的未提交个人版/诊断工作构建本地 x64 生产包；正常 Windows 构建环境中 `build-local-input-candidate.ps1` 完成 NSIS 出包，替代前述未启动编译器的 deferred 汇编结果，不据此推断那次隔离工具失败的具体责任。

- **passed**：`target/release/bundle/nsis/无线麦_0.8.5_x64-setup.exe`，SHA256 `55d3cf83c192cbaad9cdf106923bbe936e0068da7cf6436adba37c7adb177352`；GUI/版本/许可证与四项载荷校验通过，10 个前端资源均实际嵌入。
- **passed（实际安装）**：原 PID 3976 通过正常退出事件结束；管理员覆盖既有安装目录，注册版本读回 0.8.5，原 JSON 配置在安装前后哈希保持，四项资源逐件 SHA 对齐。主程序与构建产物的差异严格仅为 `__TAURI_BUNDLE_TYPE_VAR_UNK` → `NSS` 的三个字节，其余字节完全一致；已安装 EXE SHA256 `707880d02f6dc40edfe7f4030e3fe73af5c93de7f72b1514d28fe50fc1312f7c`。
- **passed（实际启动）**：由 Explorer 普通令牌启动 PID 10168，TokenElevation=false；一个主窗口及一个明确 `--sayall-key-host` 子进程，主窗口“无线麦 · 个人版”可见、Responding=true、三项导航标签实际存在。初始 IPC 和按键页监听/轮询均有完成日志，短时观察无该进程的 WebView process_failed。
- 初次临时验收脚本错误要求 EXE 字节完全一致，并把同名按键宿主算作第二主实例；逐项只读核对后按已有 Tauri 标记和新进程职责完成验证，没有重复安装或强杀。既有未提交源码继续保持未提交；未推送、发布或新增 worktree。
- 2026-10-10 交接核对：本机 `build-metadata.json` 的 `sourceCommit` 为 `null`，没有形成单一精确提交的源码身份。上述候选为 `9f4ead5` 加当时未提交工作，不能用版本号、HEAD 或空元数据声称可完整重建。下一轮先核对实际载荷及工作区；沿用本页包/EXE 指纹区分现场，构建时按 LOGGING 核对来源字段，不新增整仓库快照。
- 原始摘要为 `target/dev/upstream-sync/` 中的 `local-install-build.log`、`local-install-bundle-check.log`、`local-install-embedded-check.log`、`local-install-result.json` 与 `local-startup-result.json`，留到候选被替代或验收闭合后回收。ARM64 不在本次包中；实体遥控按键、实际收音及完整卸载矩阵仍 deferred，不把可见启动外推这些功能。

### 2026-10-10 三轮启动调查后合并 0.8.6、本地安装与启动

先保护当前及轮转日志，再按本次开机时间核对三轮主程序、Helper、按键宿主、配置与捕获代次。事实与未知点原位归 [前台输入 Bug](../Bugs/2026-10-03-foreground-input-regression.md#2026-10-10-用户报告同次开机的三轮启动部分定位未闭合)；第三轮正常观察不算故障修复。设备宿主始终为同一个 11972，没有因本轮操作被强杀、重启或更换；未操作无线电。

最新上游仍为 `3ec66566ceabf82915dd1b9ff9a3b10ec163ca26`（0.8.6）。本地必要合并提交 `af710b757897cb0567b63c63a7ceb40f62426652` 保留双方父历史，按 [ATTRIBUTION](../ATTRIBUTION.md#2026-10-10-上游-086-评估与合并) 适配焦点恢复、迟到回调与隐私日志。原有 80 项文件存在状态及 52 个已修改文件内容与本轮保护 stash 核对一致（文本只允许换行差异，个人品牌配置仅另改版本号）；全部保持未提交/未暂存，旧恢复 stash 未再次 apply。本轮新增 reset/实际映射与连发诊断也保持未提交，不包含根因未确认的按键修复。

- **passed（自动化）**：`cargo test --workspace` 669 passed（core 32、场景 4、Windows 487、集成 3、宿主 143），22 ignored；不重复计入宿主测试产生的隔离子进程一次执行。`cargo fmt --all -- --check`、`cargo check --workspace`、`cargo check -p sayall-windows-app --features runtime-simulation`；`pnpm test` 348 passed，`pnpm build` 类型检查和生产构建；安装覆盖契约 5 项通过。正常 Windows 环境执行，sandbox 下 IPC 失败/挂起的中间测试未作为产品失败或通过证据。
- **passed（本地生产包）**：既定 `scripts/build-local-input-candidate.ps1` 构建 x64 NSIS，包 `target/release/bundle/nsis/无线麦_0.8.6_x64-setup.exe`，SHA256 `e4d20971632f5bbdaa3b4a1ee43da8bdc81498f477466d182802abcad8753873`；版本/许可证/GUI、四项载荷与十项实际嵌入的前端资源通过。`build_channel=local`，内嵌 revision 为合并提交，但包包含原有未提交内容和本轮诊断；`build-metadata.json` 仍因本地没有 GITHUB_SHA 而 sourceCommit=null，不能把内嵌 HEAD 当作完整源码身份。
- **passed（正常退出及实际覆盖安装）**：原主程序 23708 在 12:10:16 请求正常退出，日志 overall passed、failed_stages=0、728ms；进程观察 913ms 后确认主程序/按键宿主已结束，原 Helper 46672 随后单独只读确认已退出。管理员仅覆盖原安装目录一次，版本读回 0.8.6；settings、button-mappings、capture-input-session、onboarding 四份 JSON 安装前后哈希保持。实际四项载荷逐件 SHA 对齐；主 EXE 与构建产物逐字节核对，仅允许既有 NSIS bundle 标记差异。安装 EXE SHA256 `307a5ebe5720a2b6154e254df91133d3830daf968420331a36ee50b8778147f3`。
- **passed（普通权限启动与诊断副作用）**：首次普通候选 18960 的父进程不能确认，先通过正常退出结束，日志 799ms、failed_stages=0，Helper 39648 的 reset 从 helper_count=1/receipt_settled=false 到 0/true/complete 有实际记录。没有重复安装；改用已有真实 Explorer 窗口的自动化入口于 12:13:44 启动主程序 32448、按键宿主 49576，父进程为 Explorer，双方 TokenElevation=false，个人版窗口实际可见且响应，配置仍保持。启动 reset 记录完整匹配判据并 complete；映射 revision=3 的 execution_mask=0、recognition_mask=256、repeat_single/long_mask=0 与当前配置一致，捕获所有权也确认 256。12:14:17 日志记录正常 hide_to_tray，后续只读 UIA 仅见事件窗口，不能把这个查询扩大解释为导航页面检查通过。
- **passed（同日 13:45 按用户要求核对最新版并退出）**：再次核对当前主程序 32448/按键宿主 49576 来自既有安装目录，版本 0.8.6；安装 EXE、包指纹及四项实际载荷与上述交付一致，没有比 EXE 构建更新的功能源码，复用已完成的打包/安装，不重复构建、恢复或安装。13:45:37 请求应用正常退出，主程序日志 overall passed、failed_stages=0、799ms，增强释放 passed、592ms；984ms 后主程序、按键宿主与 Helper 20032 均已退出，无强杀，未重新打开。安装版保留为当前候选，下次启动仍使用该 0.8.6。最小当前会话日志复用 `target/dev/foreground-input/current-086-session.log`，留到对应未闭合异常完成或候选被替代后回收；不扩大按键/收音验收结论。
- **deferred**：实体键盘实际键位/扫描码及异常复现、第二轮确切模板/前台程序、在同一实际模板下显式关闭/开启连发的实体对照、输入法真实首按/收音和焦点恢复、冷态/闲置后首用、断连/睡眠/持键退出、分别 RC001 与 RC003 的硬件回归、ARM64、完整升级/卸载矩阵。未运行新候选的完整 GUI runtime simulation；其编译及现有隔离 IPC/生命周期测试不替代硬件验收。

安装验证脚本最初因现有载荷校验脚本的默认 StagedDirectory 求值为空而中止；显式传入路径只补校验，未再次安装。临时启动脚本用本地 Shell.Application 的首次普通启动未确认 Explorer 父进程，按上述正常退出后改真实 Explorer 自动化；不把中间校验失败改写成产品故障。当前只创建必要合并提交，未自动提交新增诊断、未推送、PR、合入 main 或发布。

### 2026-10-10 晚共享宿主来源修复、本地安装与旧代次拒绝

### 2026-10-12 默认长按连续单击修订

用户明确要求未配置长按时默认连续单击，替代此前缺省关闭规则；不是重复迁移或把旧关闭配置误诊为引擎丢拍。模型新增独立关闭值，缺省自动仅作用于原有可重复按键和动作，长按动作优先；编辑器显示自动/关闭/重复单击/重复长按。菜单与任务识别占位明确关闭，原捕获来源、语音、时序和成对释放路径保持。完整规则与分项结果归 [连发手册](WindowsButtonGestureRepeat.md#2026-10-12-用户调整无长按动作时默认连续单击)。

- **内容身份**：合并基线仍为 `af710b757897cb0567b63c63a7ceb40f62426652`，保留原有及本次未提交工作。0.8.6 本地包 SHA256 `5fd60befb83f5889d7b7024cad5c662b886470bff614d37e62a77c5f51e62dd3`，12,853,490 字节；安装主程序 SHA256 `fe177026a96603d4932ca64192ec544b4b3c5aefd8132a64ab4e6ad6c57ba502`，20,740,096 字节。Helper 保持 `f9ede074f96b56c99e2468ca7137af6c45168c557934d6f4186fe7971ef9cce2`。`sourceCommit=null`，运行 `build_channel=unknown`，不能以版本/HEAD 代替完整源码身份；未签名、未发布。
- **passed（自动化）**：默认无字段行为先红；菜单、任务和合并占位三项在新默认下先红后绿。`pnpm test` 360 项/28 文件通过，内置 IPC 夹具仅同步三个 schema 版本后相关 62 项/2 文件复验通过；`cargo test --workspace` 全过（Windows 500 passed / 22 ignored、Host 145 passed / 1 ignored，另有核心与集成测试），`cargo fmt --all -- --check`、`cargo check --workspace`、`cargo check -p sayall-windows-app --features runtime-simulation`、生产 `pnpm build` 和 NSIS 构建通过。初次完整检查只剩内置夹具 schema 旧值失败，修正后重跑通过，不修改模板动作或弱化断言。
- **passed（部署）**：11:56:35.104–.605 原 App 30736 / Helper 42984 正常退出，overall 500ms、failed_stages=0，Helper cleanup passed / module_unloaded=false。覆盖安装四份配置字节保持，四项载荷一致，安装主程序与构建仅三个 NSIS 标记字节差。11:57:35.289 新 App 46728 / 按键宿主 40748 由 Explorer 12848 普通权限启动，父子 TokenElevation=0；Helper 25884 与设备宿主 12192 中相同 `2026-10-10.request-source` 鉴权成功。11:57:36.247 同线音频启动 passed。无强杀、重启电脑或无线电操作。
- **passed（当前 PC / RC003 实际默认连发）**：关闭自动跟随时，重开回到空通用配置属既有会话契约；用户按提示于 11:58:13–15 用菜单选回 Agent，revision 9 的有效单击和自动掩码均为 6385，长按/明确关闭掩码为零。11:58:16.213–34.410 记录七趟连续单击：左 6 拍、右 6/9 拍（100ms 间隔），返回 3/11/5/10 拍（50ms 间隔），共 50 续拍、不含首拍，全部以真实 release 终止。用户确认“连发、松开停止、菜单和键盘都正常”。各趟实际持有时长并非都达到请求的 2 秒，不虚记；本次启动首用与连续操作通过，不外推长闲置或完整双型号。
- **deferred**：新包 Up/Down/音量逐键、明确关闭/重复长按模式的完整实体矩阵、RC001、长闲置、断连/睡眠/持键退出等；对应自动化与已测当前场景分别记录。新包语音同线启动可读回，但未将前包四次语音会话移作新包端到端通过。原 Code 10、UI 音频两端选择/回滚和历史三轮未知点不因本轮连发通过关闭。

本次必要输出仍复用 `target/dev/foreground-input/hold-default-*`，含原状态、先红后绿、完整检查、配置哈希与唯一生命周期日志；当前包仍在既定 `target/release/bundle/nsis/`。上一语音正常包 `voice-confirmed-a3ec3155-setup.exe` 仅作本次默认规则差异对照，关闭剩余模式验收或候选被替代后回收；旧格式无法理解新显式关闭，不能把对照包直接安装当成安全降级。没有源码副本、额外 worktree、自动提交、推送或发布。

收尾回收本次空格式检查输出和已被完整前端检查覆盖的重复定向通过日志；先红证据、最终检查与实体现场保留，未动原始应用日志、用户配置、历史材料或构建缓存。

### 上游历史记录（不代表本 fork 当前合并候选通过）

以下新导入的上游实验只适用于各段明确的提交与环境；原始材料可追溯到本次固定上游提交。未在本 fork 重做的场景继续 deferred。


2026-10-04 在 Andy 的 Windows 真机上完成语音增益 RC003 对照（操作人执行；遥控器语音直接进输入法、操作人听不到音频，判据 = 回录电平 + 日志 + 转写结果）：

- 本地测试包 `无线麦 SayAll_0.5.0_gain-521cc4b_x64-setup.exe`（SHA-256 `a766c5eb86d732aaa8546538aee7ead5e42e396b0141fe0bdc724463f7f5ffb3`）安装后，同一支 RC003 在 24 dB 与 0 dB 各完成一段真实会话（07:03:40–50Z / 07:04:03–15Z，各约 10–12 秒，共 156,480 / 187,680 个采样）；
- CABLE Output 回录：24 dB 段能量 RMS 2671.8、峰值 16143；0 dB 段能量 RMS 124.1、峰值 1476；能量 RMS 比值 **21.53×**。24 dB 理论倍数为 15.85×，余量来自两遍人声自身约 1.36× 的音量差；若增益未生效，两段只会相差约 1.4×，不可能到 21.5×。24 dB 段峰值 16,143 未削波；
- 日志时间线：`action=restore ... gain_db=24` → `audio_session generation=1`（24 dB 段）→ `feature=gain action=save ... gain_db=0` → `audio_session generation=2`（0 dB 段），两段 `finish terminal_result=passed`、`queued_samples=0`；同日 06:47 连续 10 次会话（含 24 dB 两次）全部 passed，覆盖冷态首用与连续会话；
- 合成信号回环（`gain_effect_probe`，走产品 `process_pcm` 路径）：12 dB → 3.98×、24 dB → 15.85×，与 dB 定义一致；
- 证据目录：`artifacts/local-test/2026-10-04-audio-gain/`（`real-session-level-compare.txt`、`gain-probe-loopback.txt`、`build-metadata.json`）。
- **仍 deferred**：RC001 同链路；睡眠/断连恢复后的增益复测；含每会话 `audio_gain action=apply` 日志（提交 `3332e1c`）的安装包复测与转写文字 A/B。

2026-09-02 在 `windows-latest` 完成 Tauri/WebView/IPC 运行时仿真：

- Windows CI Run [`33611750471`](https://github.com/GetSayAll/remote-mic-app-windows/actions/runs/33611750471) 使用 Tauri CLI 构建仅测试 feature 程序，并在真实 Windows WebView 中完成 11 个结构化步骤；
- WebView JavaScript 通过真实 Tauri IPC 读取仿真快照，渲染 RC001/RC003 扫描结果，连接 RC001、选择仿真 CABLE Input、启动 Raw Input、保存映射并由非注入式 SendInput 记录器验证 Ctrl+C 四事件；
- 五个侧栏页面均完成导航和挂载，权限页生成平台标记为 `windows-ci-simulation` 的去标识化诊断摘要，统计 IPC 返回最近七天结构；
- 首次 `STREAM_START → 40 + 80 AUDIO → STREAM_STOP → DRAIN` 通过纯 Rust ATVV 管线得到 240 个采样和 generation 1，随后 Raw Input 与连接状态均完成释放；
- 同一 Run 重新构建普通 NSIS，二进制和前端资源均确认不含仿真平台名或仿真 command，并继续通过静默安装/卸载回归；
- Run [`33610428178`](https://github.com/GetSayAll/remote-mic-app-windows/actions/runs/33610428178) 首次暴露直接 `cargo build` 生成开发协议程序会等待 Vite dev server；改为 `tauri build --no-bundle --features runtime-simulation` 后修复；
- 该结果不访问真实 WinRT BLE、WASAPI、Raw Input 或桌面 SendInput，不代表 RC001/RC003、音质、VB-CABLE 或真实按键已经通过。

2026-09-02 在 `windows-latest` 完成 NSIS 静默安装与卸载生命周期验证：

- Windows CI Run [`33593663937`](https://github.com/GetSayAll/remote-mic-app-windows/actions/runs/33593663937) 从提交 `5f01fa41cfbd15b967a760e490b9343260e768a6` 构建 `无线麦 SayAll_0.1.0_x64-setup.exe`，SHA-256 为 `a9fa2bfbfd519189629abc3ed91d5a811752081452d93c475c62129dd1e16e44`；
- `/S` 安装以普通 Runner 用户完成，唯一卸载条目位于 HKCU，安装目录为 `%LOCALAPPDATA%\无线麦 SayAll`，开始菜单文件夹内只有一个 `无线麦 SayAll.lnk`；
- 安装后的主进程持续运行 8 秒，未立即崩溃；测试随后关闭进程，并使用卸载注册表中的真实命令执行 `/S` 卸载；
- 卸载后程序目录、开始菜单文件夹和卸载条目均移除，`%APPDATA%\app.getsayall.remote-mic.windows` 中的测试标记保留，确认默认静默卸载不删除 Tauri `app_config_dir`；
- Run [`33591214805`](https://github.com/GetSayAll/remote-mic-app-windows/actions/runs/33591214805) 在安装前暴露空注册表结果的 PowerShell 数组边界，Run [`33592013119`](https://github.com/GetSayAll/remote-mic-app-windows/actions/runs/33592013119) 暴露带引号 `InstallLocation`，Run [`33592799733`](https://github.com/GetSayAll/remote-mic-app-windows/actions/runs/33592799733) 暴露非交互 Runner 无法通过 WScript 读取快捷方式目标；三项均按实际 Tauri NSIS 注册表契约修正；
- 该结果不证明可见安装/卸载界面、语言选择、SmartScreen、Windows 10 1809、Windows 11 真实桌面、签名发布包、RC001/RC003 或第三方应用已经通过。

2026-09-02 在 Apple Silicon Mac 上完成 Rust ↔ TypeScript IPC 契约快照验证：

- 同一份 JSON 夹具覆盖完整 `PlatformSnapshot`、嵌套连接/音频/Raw Input 快照，以及 RC001、RC003、unknown 三种 `PairedRemote`；
- Rust 测试从真实结构序列化并与夹具结构级全等比较，同时拒绝任何 snake_case IPC 字段；
- TypeScript 测试使用前端接口装载同一夹具，核对枚举白名单、精确字段集合、`remoteModel` 和 `isSupportedCandidate` 的 camelCase 边界；
- 该结果只证明仓库两端静态类型和序列化契约一致，不证明 Tauri command、Windows WebView、WinRT 或真机运行时数据已经通过。

2026-09-02 在 Apple Silicon Mac 上完成 RC001 协议场景回放：

- 从 `GetSayAll/hardware-simulation@65248499cac7da3ad46cd0c11dca1478f7733255` 的 RC001 短语音场景提取纯 ATVV JSON 夹具，保留 `STREAM_START`、40 + 80 字节音频拆包和 `STREAM_STOP`；
- 夹具回放确认无主动 `MIC_OPEN`，两段音频恰好组成一个 120 字节帧并解码为 240 个采样，停止时没有遗留半帧；
- 同一夹具扩展覆盖 20 次极速按下/释放空会话、20 次完整语音会话，以及首段 40 字节半帧后中断、下一会话仍首次完整解码；
- 该结果只证明纯 Rust ATVV/帧累积/ADPCM/会话状态机在确定输入下的行为，不证明真实 RC001 固件、WinRT 通知顺序或实际音质。

2026-09-02 在 Apple Silicon Mac 上完成 RC001 支持实现级验证：

- 对照 macOS `b233a88cc4457b00413dda6b37ec8b4af12c5121` 的设备档案、标准 2A24 型号识别和 RC001 首次短语音路径；
- Rust 单元测试覆盖 RC001/RC003 名称、`RC001` / `RC003` Model Number、通用名称不猜测型号、两种型号高半字节优先 ADPCM，以及 RC001 无需主动 `MIC_OPEN` 的 `STREAM_START → AUDIO → STREAM_STOP`；
- `sayall-core` 25 项、macOS `sayall-windows` 24 项、Tauri Host 6 项测试通过，`cargo check --workspace` 和 `x86_64-pc-windows-msvc` 平台层交叉静态检查通过；
- Vue 4 个测试文件共 9 项测试、TypeScript 类型检查、Vite 生产构建和 `tauri build --no-bundle` 通过；
- 以上结果只证明型号字段、WinRT API 类型、共用 ATVV 管线和界面可编译且自动化通过。真实 Windows 上的 RC001 与 RC003 设备名/2A24 值、首次语音、按键报告、睡眠恢复和 VB-CABLE 回环均为 deferred。

2026-09-01 在 Apple Silicon Mac 上完成：

- Vue 导航、连接阶段映射、诊断格式和权限页复制组件测试、类型检查和 Vite 生产构建；
- 900 × 620 与 1080 × 720 浏览器界面检查，全部五个侧栏入口可打开；权限页无横向溢出，诊断摘要可在页面内生成和滚动；
- 浏览器控制台零错误；
- `sayall-core` 22 项测试；macOS `sayall-windows` 20 项平台边界、退避、Raw Input 和 SendInput 纯逻辑测试；Tauri 设置与诊断 4 项测试；Windows CI 额外运行 WASAPI generation、有界队列、持久端点身份、重连调度、电源回调转发、Raw Input 纯逻辑测试和完整 Tauri Host 编译；
- 诊断隐私测试使用设备名称、蓝牙地址、端点身份、用户路径、HID 路径和后端错误作为敏感夹具，序列化摘要均未包含这些值；浏览器生成摘要也未出现 `remoteName`、`selectedEndpointId`、`selectedEndpointName` 或 `lastError` 字段；
- Raw Input 自动化覆盖两种 Windows 设备路径、零/多匹配 fail-closed、6/7/9 字节 HID 报告、RAWHID 批量拆分、重复 key-down、Keyboard/HID 双来源并集、停止释放和语音键隔离；隐藏窗口和真实事件形态仍需 Windows/RC001/RC003 分别验收；
- 纯 Rust 首次 `CAPS → STREAM_START → AUDIO → STREAM_STOP → DRAIN`、同步帧、8 kHz 拒绝和流外音频测试；
- macOS 全 workspace `cargo test`、`cargo check` 和格式化检查；
- `x86_64-pc-windows-msvc` 目标下 WinRT BLE/GATT/ATVV/WASAPI/Raw Input 平台层交叉静态检查。
- Windows CI Run [`33516627294`](https://github.com/GetSayAll/remote-mic-app-windows/actions/runs/33516627294) 通过；该结果证明 Windows 代码和 Tauri Host 可编译、自动化可运行，不证明隐藏窗口收到真实 RC001 或 RC003 报告。
- Windows CI Run [`33522534257`](https://github.com/GetSayAll/remote-mic-app-windows/actions/runs/33522534257) 通过并生成 artifact `sayall-windows-unsigned-preview-e11e460f15287468c01225a8ab86b302d59f09d4`；下载后确认只有一个 4,565,134 字节 NSIS 安装器、`SHA256SUMS.txt` 和 `build-metadata.json`，UTF-8 中文文件名可由 macOS `shasum -a 256 -c` 校验，安装器实算 SHA-256 与两份记录一致为 `e1cccf73c8b20487874eadb12eb9823635f52a0d7664c07fe6b8928532d246d6`，metadata 标记 `NotSigned` 和 `unsigned-ci-preview-not-for-public-release`。
- main Windows CI Run [`33525406955`](https://github.com/GetSayAll/remote-mic-app-windows/actions/runs/33525406955) 从 merge commit `94923dea303a66604b61ca2572fd13d9d48a9d60` 生成同边界 artifact 并通过下载后 SHA-256、metadata、PE/NSIS 文件类型和 UTF-8 校验文件复验。
- main Windows CI Run [`33529876579`](https://github.com/GetSayAll/remote-mic-app-windows/actions/runs/33529876579) 从诊断功能 merge commit `f8a65d33c31283c3db9e7964d9a9a36bc1e9f6c3` 成功完成前端、Rust、Windows Host、NSIS 和 artifact 上传；下载后确认唯一安装器为 4,561,376 字节，SHA-256 为 `cfddc29da4ed40d543039c545a0ab23993165243b95bf4b7335d98ff5ed848b3`，metadata 来源提交、`NotSigned` 和 `unsigned-ci-preview-not-for-public-release` 均一致。

2026-09-02 在 Apple Silicon Mac 上完成本机使用统计实现级验证：

- `sayall-core` 覆盖每日批量累计、今日/周/总汇总、饱和计数、非法持久时长归一化和旧 `settings.json` schema 迁移；
- Windows 平台计数覆盖普通按键按下次数、完整语音会话次数和 16 kHz 采样累计；语音只在成功排空后提交统计，中断路径清除未完成采样；
- Tauri 统计层覆盖进程计数增量、采样到秒换算、周一起始的本周汇总和最近 7 天日期桶；文件写入由独立后台线程执行，不进入 BLE、WASAPI 或 Raw Input 回调；
- Vue 组件覆盖今日、本周、全部切换、时长格式、最近 7 天数据和零数据空状态；900 × 620 与 1080 × 720 浏览器检查无横向溢出，新增统计界面最终字号不小于 16px，控制台零 warning/error；
- `x86_64-pc-windows-msvc` 目标下 `sayall-windows` 平台层静态检查通过；完整 Tauri Host 交叉检查在 Mac 上仍受缺少 `llvm-rc` 限制，交由 `windows-latest` CI 验证；
- Windows/RC001/RC003 真实按键计数、真实语音计数、跨自然日、退出落盘和升级保留均为 deferred，必须对两种型号执行用例十一后才能称为真机通过。

2026-09-02 在 Apple Silicon Mac 上完成 Windows 10 1809 版本门禁实现级验证：

- 纯 Rust 版本比较覆盖 Windows 10 build 17762 拒绝、17763 边界接受、更新 Windows 10 build 和未来主版本接受；
- `sayall-windows` 的 `x86_64-pc-windows-msvc` 静态检查通过，确认 `RtlGetVersion` 封装与原生 `MessageBoxW` 代码可编译；
- Tauri 配置解析、macOS workspace 测试/检查、前端 8 项测试、生产构建和 `tauri build --no-bundle` 通过；
- NSIS hook 的实际编译交由 Windows CI；低版本安装提示、退出码 1633、直接运行 exe、Windows 10 1809 接受和 Windows 11 接受仍为 deferred，必须执行用例零后才能称为运行验收通过。

以上结果只证明 Tauri Windows NSIS 未签名 Preview 可在干净 Windows Runner 生成且产物身份、摘要和来源元数据一致；不能证明安装、升级、卸载、WinRT 运行时、真实 RC001/RC003、WASAPI、Raw Input 或 SendInput 已经通过，这些用例仍为 deferred。
macOS 上对完整 Tauri Host 的 Windows 目标交叉检查需要 `llvm-rc`，本机未提供该 Windows 资源编译器，因此完整 Host 由 `windows-latest` CI 验证。

诊断复制已通过前端组件测试确认向 Clipboard API 写入完整可见摘要；应用内浏览器的剪贴板回读与页面 Clipboard API 不共享同一自动化通道，因此不能把浏览器提示当作真实 Windows WebView 剪贴板验收。Windows 安装包中的生成、复制和记事本逐字比对仍为 deferred。


## 会话临时麦克风输入（2026-09-16 本地候选）

这项验证只覆盖用户明确允许的受限 IPolicyConfig 会话功能，默认关闭；不能外推三键修复、20 秒内部队列溢出或 RC001。未文档化 setter 没有 CAS，通知没有变更者；检测到外部变化后整次让出，不宣称消除全部读写间竞态。

1. 安装前后保存 SayAll 配置哈希、普通用户实例与固定包载荷证据；不回滚用户的 Windows 默认设备选择。记录 Capture 的 Console/Multimedia/Communications 三角色基线。
2. 用户在“连接与语音”读取输入设备，选择 CABLE Output 并显式开启“遥控器说话时临时锁定麦克风输入”。原写入端点仍 CABLE Input；不自动选择目标，不因音量键或闲置改变默认设备。
3. 先在目标软件按住 3–5 秒并松开。日志核实三个角色实际读回目标后才发快捷键 DOWN，源音频提交/排空，快捷键 UP 先于默认角色恢复。公开会话观察器核 WeType 实际 active CABLE Output；用户确认目标收到遥控器声音。API 成功、路由选择或静音帧数均不能代替最终收音。
4. 分别验证快速释放（准备中取消不得迟到 DOWN）、连续会话、真实闲置首用、开关关闭、设备 missing/replaced、耳机插拔或系统手选。检测到任一角色偏离预期后全部角色保持当时现状，不争抢、不再恢复其他角色。会话进行中设置变更须提示松开后操作。
5. 断连、睡眠、正常退出均先走 BLE owner 的快捷键 UP，再做路由恢复。记录未能恢复时的具体错误与 journal 状态。异常退出恢复只在隔离验证环境制造：下次启动必须显示“恢复/保留当前选择”，未点击不得静默改设备；即使未改过的角色发生外部变化，也不能恢复其余角色。
6. IPolicyConfig 不能保证目标软件覆盖其显式指定的麦克风。将三角色默认确认、WASAPI 实际供音、WeType 采集端点和用户可见结果分别记录 passed/failed/deferred；有硬件但未操作的项标未执行，不虚构 deferred 原因。

本候选真实验收证据归 `334c372:artifacts/capture-input-session-20260916/evidence.md`。状态以该记录为准。
