# WeType ConsentStore 历史记录修复测试手册

## 测试对象

- 修复 PR：[#55](https://github.com/GetSayAll/remote-mic-app-windows/pull/55)
- Source commit：`6b4d8b64d0711b4e1eda21b3fa81da2653c665bd`
- 本地测试包：`artifacts/windows-preview/SayAll-Windows-0.2.5-pr55-6b4d8b6-x64-setup.exe`
- SHA-256：`37dfadeb78bbe87a34d27712ec59c4e49b51ff75525412efc97b5954ec0cffc4`
- 适用环境：Windows 10 1809+ / Windows 11 x64、WeType、RC003；RC001 需单独记录

本包仅供本地验收。它没有 Windows Authenticode 签名，可能触发 SmartScreen；
本地构建使用的一次性 updater 签名不属于公开更新信任链，不得上传或公开分发。

## 测试目标

验证系统存在一条或多条 WeType 麦克风访问历史时，SayAll 能识别当前正在录音或
本次按键后新开始的录音，不会在用户持续按住语音键期间误触发输入法恢复、释放快捷键
或重复注入快捷键。

## 安装前准备

1. 记录 Windows 版本、WeType 版本、遥控器型号和当前 SayAll 版本。
2. 确认 WeType 的语音快捷键与 SayAll“按住说话快捷键”一致。
3. 保存现有设置和按键映射；升级后应保持不变。
4. 在系统托盘中选择 SayAll“退出”，确认应用正常退出。不要用任务管理器强杀正式安装版。
5. 在 PowerShell 中校验安装包：

   ```powershell
   Get-FileHash -Algorithm SHA256 -LiteralPath ".\artifacts\windows-preview\SayAll-Windows-0.2.5-pr55-6b4d8b6-x64-setup.exe"
   ```

6. 运行安装包完成当前用户覆盖安装，然后从开始菜单启动 SayAll。

## 用例一：升级与启动

1. 打开连接、音频端点、按住说话快捷键和按键映射页面。
2. 对照安装前记录，确认配置没有被重置。
3. 连接遥控器并等待 BLE / ATVV 就绪。

预期：只存在一个 SayAll 安装身份；程序正常启动，原设置、映射和音频端点选择保留。

## 用例二：持续按住不被错误恢复

1. 在可输入文本的窗口中激活 WeType。
2. 按住 RC003 语音键并连续说话 15 秒，然后释放。
3. 重复 5 次，每次间隔至少 10 秒。

预期：每次只启动一次听写；持续按住期间不停止、不重新开始、不重复注入快捷键；
只在物理释放后结束。SayAll 的音频会话和快捷键 DOWN/UP 必须严格成对。

## 用例三：闲置后首用

1. 保持 SayAll、WeType 和遥控器连接状态，至少 5 分钟不使用语音键。
2. 闲置后的第一次按住持续说话 15 秒，然后释放。
3. 再连续执行两次普通语音会话。

预期：闲置后第一次即成功，期间不触发错误恢复；后续会话不受上一会话影响。

## 用例四：快速连续会话

1. 连续完成 20 次约 1 秒的按下、说话、释放。
2. 再完成 3 次 10～15 秒持续按住。

预期：没有粘键、漏释放、跨会话重试或旧检查线程干扰新会话；每次听写只对应一个
物理按下/释放周期。

## 用例五：多历史记录边界

如果当前 Windows 自然存在多个 WeType 版本或安装路径留下的麦克风历史，直接执行
上述用例并记录结果。不得为了制造场景而编辑 ConsentStore 注册表、WeType 私有配置
或内部数据库。没有自然多记录环境时，本项记为 `deferred`，由自动化回归覆盖枚举顺序、
活动记录、已完成新记录、未变化记录以及观测缺失/回退五类判定。

## 日志判据

默认诊断日志位于 `%LOCALAPPDATA%\SayAll\Logs\sayall-diagnostic.log`。健康会话通常包含（`evidence=mic` = ConsentStore 开麦观测；`evidence=marker` = 微信输入法自注入的 0xFC 存活标记，见 Bugs\2026-09-30-wetype-246-consentstore-probe-blind.md；`mic=` 单独记录开麦判据自身结果，用于识别"盲判"）：

```text
wetype_check armed attempt=0 ... marker_baseline=<n>
wetype_check reacted=true attempt=0 ... evidence=mic mic=observed marker_extra=0x0
wetype_check reacted=true attempt=0 ... evidence=marker mic=observed marker_extra=0x57545950
wetype_check reacted=true attempt=0 ... evidence=marker mic=not_observed marker_extra=0x57545950   ← 盲判被门禁拦住（正常）
```

持续按住且 WeType 已开始录音时，不应出现同一会话的：

```text
wetype_revive result=...
chord_retry result=ok ...
```

若观测不可用，允许出现 `reason=observation_unavailable`；若已有正面存活证据，允许出现
`wetype_check skipped_retry reason=wetype_alive evidence=marker`（或 `chord_retry skipped reason=wetype_alive`）
——**两者都不得因此释放并重注入当前和弦**。

提交日志前应删除任何意外出现的个人路径或用户内容；不要提交语音、转写正文、设备
身份、蓝牙地址、HID 路径或音频端点身份。

## 结果记录模板

```text
Source commit: 6b4d8b64d0711b4e1eda21b3fa81da2653c665bd
Windows / WeType:
Remote: RC001 | RC003
升级与配置保留: passed | failed
持续按住 5 次: passed | failed
闲置 5 分钟后首用: passed | failed
快速会话 20 次: passed | failed
自然多历史记录环境: passed | failed | deferred
失败发生时间:
对应日志片段:
```

只有实际执行并观察满足预期才记录 `passed`；当前未提供真实硬件或自然多历史记录环境
的项目必须记录为 `deferred`。

## 历史结果：2026-09-10/11 本地 6504010 修复包

本节对应本机 RC003 / 遥控器2 Pro、微信输入法和0.2.5本地修复包，独立于上方 PR #55 包身份。保留当时观察与结果，不把它们外推为当前版本、RC001或完整用例矩阵通过。

### 原故障与原因证据

- 用户报告持续按住约5秒后听写停止，但 SayAll 仍显示连接。已安装程序启动日志源码标识为 `54ab159aedf27254f2d9237f53a6e1a11297e0ba`；当时仅观察到一个 SayAll 主进程、未观察到 Axonkey 主进程，不能据此证明所有卸载残留已清除。
- 当前登录用户的 Windows 公开麦克风访问历史有11条 WeType 匹配记录，首条不是最新。只输出布尔结果的比对确认旧日志反复使用的 baseline 恰为首条，并有更新记录存在。
- 2026-09-10 17:50:15.962（UTC+8）开始的会话，于17:50:16.663触发恢复判定，17:50:18.890和17:50:22.819重试快捷键，证明应用在一次持续会话中主动重试；日志不能单独确定用户界面停止的精确时刻。
- 旧源码只取首条匹配历史记录。固定版本修复聚合记录、识别活动录音、注入前取基线，并在观测不明时跳过恢复。

### 精确构建与本地材料

- Source commit：`6504010828b12713ce033cb3e231087af6a6482f`；使用该提交源码归档，未纳入当时工作区其他未提交修改。归档 SHA-256 为 `476258314c776be9347409481e60f1084b4e86f000ce8fa5bbf5b6ea5f89bd7d`。
- 版本0.2.5，通道 `local`，release tag 为 `none`。临时 Tauri 配置只关闭 updater 发布资产生成，复用项目 target 中 NSIS 工具缓存；应用功能源码及更新公钥不变，没有公开上传或发布该包。
- 当时本地安装包为 `SayAll-Windows-0.2.5-wetype-fix-6504010-x64-setup.exe`；SHA-256 `6dbccecec965e3126b28c2bf27472af335ec8ca7b814945626d62dde3ed0b499`，签名状态 `NotSigned`，仅本地测试。安装后的主 EXE SHA-256 为 `ecef5515afa6308f2406287c197fc9621b00852e6a913457cf88236ff4c8f7f0`。该已结束对照的工作目录现已回收，保留此历史身份，不保证能重建相同二进制。
- 前端构建、Windows release 编译和 NSIS 打包 `passed`。`cargo test --release --locked -p sayall-windows --lib wetype_revive -- --include-ignored --nocapture` 为6项 `passed`，包括本机11条公开历史记录的只读测试。
- GUI PE 子系统、x64架构、源码标识、修复日志标记和 NSIS bundle 标记核对 `passed`。第一次打包已写入 NSIS 标记，第二次仅打包报告未找到未替换占位符；最终成品含 `NSS`、不含 `UNK`，不能把第二次进程失败解释为最终成品未完成。
- 2026-10-05静态归档核对：旧 `source.zip` / `source/` 的215项源码与上述提交仅在规范换行后相符，另13项为生成物；不宣称整目录或原始字节与提交完全相同。有效源码身份改用精确提交，原本地诊断文档的有效事实已收归本节。随后按用户要求进一步撤去不必要的本地过程目录：该问题已有源码修复与两次真实观察，旧包、逐轮日志和过期设置备份不再用于当前恢复，随目录回收；保留上述原失败、安装身份、计数及未验边界。当前应用配置未被改写。

### 覆盖安装失败与获准安装例外

2026-09-11用户授权备份设置、正常退出、原目录覆盖安装及启动验证，并通过托盘退出旧进程。两份设置 JSON 的备份一致性 `passed`。普通用户以 `/S /UPDATE` 向原安装目录执行已核哈希的包，安装器退出码0，但文件核验 `failed`：原程序仍含旧源码标识，新程序未写入，不能以退出码宣称安装成功。结束时旧版完整保留且已退出，无安装器/SayAll残留进程，设置与备份完全一致。

无写入的句柄检查发现原程序文件拒绝 ReadWrite 访问（`UnauthorizedAccessException`、HRESULT `80070005`）；普通用户令牌未提权，文件 Owner 非当前用户。该次未改 ACL 或提权安装。用户随后明确授权一次 UAC 安装例外，同一已核安装器经 `RunAs` 执行、退出码0；原目录实际写入前述 `ecef5515…8f7f0` 新 EXE，设置与备份逐字节哈希一致，原安装身份与0.2.5版本保持。

主程序由确认未提权的启动进程启动；前端初始化、首次 IPC、原音频端点恢复均记录 `passed`。更新检查出现 `manifest_or_network_failure`，不影响该次本地覆盖安装，但更新检查没有记为 `passed`。安装提权不代表主程序提权。

### 两次 RC003 持续听写与剩余边界

- 用户确认两次均能持续听写，只在松开语音键后结束。
- 会话一：2026-09-11 09:43:25.098—09:43:39.175（UTC+8），快捷键 DOWN/UP 间隔 **14,077 ms**。
- 会话二：09:43:54.377—09:44:06.712（UTC+8），快捷键 DOWN/UP 间隔 **12,335 ms**。
- 两次均在 `attempt=0` 检测到微信输入法响应；共两组成对 DOWN/UP，`chord_retry` / `wetype_revive` 均为0，音频 finish 完成。原持续按住中途停止问题在这两次实际场景中未复现，用户可见结果与日志一致，限定这两次结果为 `passed`。
- 实际14.077秒和12.335秒均超过原约5秒中断阈值，**不记为严格15秒用例或持续按住5次通过**。闲置至少5分钟后的首次使用仍待验；RC001及其他未执行场景不外推，也不借历史 CI/硬件结果宣称本机新包其他场景通过。
