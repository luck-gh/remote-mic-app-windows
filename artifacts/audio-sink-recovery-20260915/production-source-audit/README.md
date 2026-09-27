# 精确生产源回归审计（2026-09-15）

本目录是 ignored 本地证据，不是产品、默认 CI 入口或可交付修复。旧、新 audio 源按 Git 保留 blob 导出，逐字节 SHA 匹配被安装包构建前冻结清单；没有将当前实验源误当已安装内容。已恢复工作区 audio.rs 为新包原始 25EE 源。

- 旧 audio：blob `43c319e36816157c02a58dbb826136fb1caeea80`，`old-production-audio.rs`，SHA `7fdac64f87bd1697fb2d4c003a0a9a6e118130d9433401eaa0b5f8f585987f95`。
- 新 audio：blob `6b2e8ee92d0ab764625fd4bdb9f69602d78f0152`，`installed-recovery-audio.rs`，SHA `25ee719c468e4483683e98ae7a2718d988be58e58ed692a6c91508fbe775a1fe`。
- 旧 BLE：blob `c945af81a1d987bef572a7f0e21784cf1115aa5d`，`old-production-ble.rs`，SHA `65d1befee17dc88e5b9b01a786c62aa22b3859117bc473057eaa6f86cd475f48`。
- 新 BLE：`installed-recovery-ble.rs`，SHA `b531653df620c2a3c2342256413ee5fb5751beec57544734b79d1da437716aa3`。

精确 diff：audio 的 16k/480 prebuffer/32k queue/5ms polling、WASAPI shared 默认周期、静音检查及稳态写入一致；新稳态增加计数和 Instant，失败才格式化汇总。BLE 持续音频批次逻辑一致，变化在启动准备/取消 guard 与按键顺序。不能仅据静态审计否定用户“安装前可超过一分钟”的生产回归线索。

## 实际试验

`source-ab-manifest.json` 对应历史 `write-only-harness.rs` 和旧、新原始模块。固定同一 optimized harness，old→new 顺序，墙钟补齐 16k silence，最多 60s/版，快照每秒一次；源模块未修改。18:08:26 结束，old 56.014s / new 35.004s 均 worker queue overflow，正常 interrupt/drop，exit 1，配置 hash 一致。该试验没有显式采集方，只是辅助证据。

`consumer-manifest.json` 对应最终 consumer harness。18:05 空闲快照：Capture Console/Multimedia/Communications 全非 CABLE Output；CABLE Output active 0，WeType inactive 1，不能推断按住时的私有选端点。完整 consumer 实验用公开属性验证唯一 CABLE Output 与选定 CABLE Input 的非空 ContainerId 相等；在独立 MTA 线程按 native GetMixFormat 48k/2ch/32bit 打开 capture。Start 成功后启动 writer，GetBuffer 数据指针完全不解引用，每包 Release，只统计帧/flags，不保存音频或修改 default。

18:17:35 完整 consumer 对照结束：old 30.008s failed（fed 480138，submitted 435851）；new 35.005s failed（fed 560094，submitted 515191；失败瞬间 accepted 543996、queued 28805）。fed 包含异步尚未接受的消息，不能冒充 accepted。失败后 queue 被清零，终态 0 不能解读为正常 drain。两端 consumer 实际取得帧：old 1363808@48k=28.412s/wall30.034s，discontinuities73；new 1605024@48k=33.438s/wall35.023s，discontinuities82。Capture discontinuity 可能丢帧，不能仅凭帧时长短于墙钟断言设备 clock 变慢。两版 Stop/drop 完成，runner exit1，配置一致。

结论：无 consumer 不是已证唯一原因；精确 audio 新版本没有表现出独有失败。两版隔离失败仍不等同 RC003→微信输入法真实拓扑，不能否定生产回归或宣称修复。停止调参，下一受控旧包物理对照尚未执行。

来源：微软 [Capturing a Stream](https://learn.microsoft.com/en-us/windows/win32/coreaudio/capturing-a-stream)、[Device Properties](https://learn.microsoft.com/en-us/windows/win32/coreaudio/device-properties)、[PKEY_DeviceInterface_FriendlyName](https://learn.microsoft.com/en-us/windows/win32/coreaudio/pkey-deviceinterface-friendlyname)；现有 wasapi 0.24 实现与 VB-CABLE 官方手册。没有复制第三方音频数据。
