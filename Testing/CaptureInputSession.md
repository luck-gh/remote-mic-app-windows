# 会话期间临时Capture输入锁定

用户于2026-09-16明确授权“允许受限使用，并接受外部改选时让出控制”。仅此功能在当前用户Voice DOWN准备中使用IPolicyConfig；主程序保持普通权限，不注入第三方程序。

选择CABLE Output作为capture目标，既有render仍CABLE Input；默认关闭。切换三角色读回后才注入目标快捷键，源UP取消迟到启动。BLE owner先释放快捷键，再结束音频/路由；不能在释放排队时抢先恢复其他麦克风。

Console/Multimedia可能联动，写前持久记录before/desired/mask，仅接受组内向目标收敛；第三值和组外改变使本次整体让出。normal原两角色分裂时写前明确拒绝，以免无法精确恢复。崩溃后提供可见恢复/保留，不自动覆盖。没有CAS或通知actor，同值用户操作无法识别；第三方显式设备不保证跟随默认。

## 验证

- 2026-09-19用户真实确认RC003非目标默认输入的冷首按正常，长CABLE Output名称下拉框不再越界；这是用户验收，不补造过期观察器记录。
- 修复前3次role0 setter后被自身external_change中止，PCM为0；组模型覆盖切换、恢复、写前/部分完成崩溃及外部改变。
- 提交视图自身通过Windows Tauri cargo check、Vue类型检查/构建、19项Windows路由、3项宿主事务、5项页面测试；不依赖未提交driver/HID旁路/模板文件。
- 原精确WASAPI端点重建与配对取消是必要生命周期依赖；此前真实静音恢复已验证，但20秒队列溢出仍未解决，不称本提交修好长期吞吐。
- 耳机/手选、快按/重复/闲置组合、断连/睡眠/退出、crash恢复的完整真机矩阵仍待验证；RC001缺本轮硬件deferred。60秒问题按用户要求暂停。

来源与受限API边界见[ATTRIBUTION](../ATTRIBUTION.md)，剩余验收归[TODO](../TODO.md)。本地详细证据在artifacts/capture-input-session-20260916和capture-input-commit-20260919，不提交私有配置、设备身份或语音。

源码复现以提交 `85fffc84d41c3bdaf822e49ab115e1d922c75e42` 为准。2026-09-19 核对提交候选目录的 235 个受管文件与该提交内容一致（允许 CRLF/LF 换行差异），其余 13 个文件均为构建或测试生成物；按用户确认范围清理 `candidate/`、临时 diff、索引和提交准备文件。原目录继续保留 `commit-result.json`、源码清单与验证日志；`capture-input-session-20260916/` 的安装包、失败现场及首按修复证据完整保留。
