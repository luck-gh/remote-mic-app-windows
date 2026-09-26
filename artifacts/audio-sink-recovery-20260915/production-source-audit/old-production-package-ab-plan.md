# 待执行：今天旧生产包 RC003 对照

此文件仅准备，未执行安装、退出、默认设备修改或配置恢复。必须先收到用户本次正常退出回执，再双重查实例为 0；不得强杀或让安装器强杀。

1. 包已只读重新核验：旧 `artifacts/three-key-input-20260914/无线麦 SayAll_0.2.5_x64-setup.exe` SHA `0d437e5feabe19428cf06a4e3614f24e1f152b8241439c915575a70cdd19fd0f`；可恢复的新 `artifacts/audio-sink-recovery-20260915/无线麦 SayAll_0.2.5_x64-setup.exe` SHA `fbc792f8fd356870508bbba1362a495fa4b96c6997a7f4b8deb8ae35c89bfe03`。无需重编译。
2. 正常退出后，Get-Process + Win32_Process 核所有 SayAll 及安装器为 0；将当前 settings/button-mappings/voice-hold-hotkey 存在项备份到本 ignored 证据的私有子目录，只报告 hash/存在状态，不输出内容。安装前再次门禁为 0。
3. 管理员 `/S /UPDATE /D=D:\Program Files\无线麦 SayAll` 覆盖旧包；不调用增强驱动/安全设置，不重启。有界等待，超时只查状态，不二次运行或强杀。
4. 核旧载荷全量 hash、Tauri 仅 UNK→NSS 三字节已知变化、Helper/资源及配置 hash。真实 Explorer 普通用户启动；新实例/session/token/响应和连接/原选端点 Ready 均实际核验。
5. 持续日志和公开 capture session 观察准备好后，请用户 RC003 在同一微信输入法场景按住 65–75s 再松开；不用短过期采集窗。核每会话边沿、开始/帧/提交/停止、实际 WeType capture 活动端点，记录用户可见录音结果。不得改变默认 Capture 或用户模板来伪造同条件。
6. 无论旧包结果如何，按需要再次请用户正常退出并重复双门禁，可用固定新包恢复。两次安装各自独立 evidence，不覆盖原 9/14 或 9/15 交付证据。配置异常不自动回写旧备份覆盖用户新改选，应先报告差异。

旧包不含永久 sink 恢复修补；若复现 overflow，后续新会话可能无法自愈，因此回到新包的具体路径必须保留。此对照不会使三键或键盘并用功能完成。用户配置始终不因实验修改。
