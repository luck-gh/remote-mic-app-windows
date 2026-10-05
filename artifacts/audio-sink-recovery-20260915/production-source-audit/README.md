# 精确生产源回归审计（2026-09-15）

本目录保存2026-09-15生产回归对照的档案源码，不是产品、默认 CI 入口或可交付修复。七份具名源码按原始字节保存，并以本目录 `.gitattributes` 的具名 `-text` 规则保持冻结 SHA；完整日志、原始机器清单、配置备份和二进制仍仅本地保留。旧、新 audio 源当时按 Git 保留 blob 导出，逐字节 SHA 匹配被安装包构建前冻结清单；没有将实验源误当已安装内容，当时已恢复工作区 audio.rs 为新包原始 25EE 源。此历史状态不代表当前工作区仍与该快照相同。

- 旧 audio：blob `43c319e36816157c02a58dbb826136fb1caeea80`，`old-production-audio.rs`，SHA `7fdac64f87bd1697fb2d4c003a0a9a6e118130d9433401eaa0b5f8f585987f95`。
- 新 audio：blob `6b2e8ee92d0ab764625fd4bdb9f69602d78f0152`，`installed-recovery-audio.rs`，SHA `25ee719c468e4483683e98ae7a2718d988be58e58ed692a6c91508fbe775a1fe`。
- 旧 BLE：blob `c945af81a1d987bef572a7f0e21784cf1115aa5d`，`old-production-ble.rs`，SHA `65d1befee17dc88e5b9b01a786c62aa22b3859117bc473057eaa6f86cd475f48`。
- 新 BLE：`installed-recovery-ble.rs`，SHA `b531653df620c2a3c2342256413ee5fb5751beec57544734b79d1da437716aa3`。

精确 diff：audio 的 16k/480 prebuffer/32k queue/5ms polling、WASAPI shared 默认周期、静音检查及稳态写入一致；新稳态增加计数和 Instant，失败才格式化汇总。BLE 持续音频批次逻辑一致，变化在启动准备/取消 guard 与按键顺序。不能仅据静态审计否定用户“安装前可超过一分钟”的生产回归线索。

## 实际试验

`source-ab-manifest.json` 对应历史 `write-only-harness.rs` 和旧、新原始模块。固定同一 optimized harness，old→new 顺序，墙钟补齐 16k silence，最多 60s/版，快照每秒一次；源模块未修改。18:08:26 结束，old 56.014s / new 35.004s 均 worker queue overflow，正常 interrupt/drop，exit 1，配置 hash 一致。该试验没有显式采集方，只是辅助证据。

`consumer-manifest.json` 对应最终 consumer harness。18:05 空闲快照：Capture Console/Multimedia/Communications 全非 CABLE Output；CABLE Output active 0，WeType inactive 1，不能推断按住时的私有选端点。完整 consumer 实验用公开属性验证唯一 CABLE Output 与选定 CABLE Input 的非空 ContainerId 相等；在独立 MTA 线程按 native GetMixFormat 48k/2ch/32bit 打开 capture。Start 成功后启动 writer，GetBuffer 数据指针完全不解引用，每包 Release，只统计帧/flags，不保存音频或修改 default。

18:17:35 完整 consumer 对照结束：old 30.008s failed（fed 480138，submitted 435851）；new 35.005s failed（fed 560094，submitted 515191；失败瞬间 accepted 543996、queued 28805）。fed 包含异步尚未接受的消息，不能冒充 accepted。失败后 queue 被清零，终态 0 不能解读为正常 drain。两端 consumer 实际取得帧：old 1363808@48k=28.412s/wall30.034s，discontinuities73；new 1605024@48k=33.438s/wall35.023s，discontinuities82。Capture discontinuity 可能丢帧，不能仅凭帧时长短于墙钟断言设备 clock 变慢。两版 Stop/drop 完成，runner exit1，配置一致。

结论：无 consumer 不是已证唯一原因；精确 audio 新版本没有表现出独有失败。两版隔离失败仍不等同 RC003→微信输入法真实拓扑，不能否定生产回归或宣称修复。9月15日当时已停止调参，受控旧包物理对照尚未执行；9月16日后续四轮旧/新完整包实测见 [60秒停止证据](../sixty-second-20260916/evidence.md)。60秒停止研究已按用户要求暂停，不将本段历史步骤作为当前待办。

来源：微软 [Capturing a Stream](https://learn.microsoft.com/en-us/windows/win32/coreaudio/capturing-a-stream)、[Device Properties](https://learn.microsoft.com/en-us/windows/win32/coreaudio/device-properties)、[PKEY_DeviceInterface_FriendlyName](https://learn.microsoft.com/en-us/windows/win32/coreaudio/pkey-deviceinterface-friendlyname)；现有 wasapi 0.24 实现与 VB-CABLE 官方手册。没有复制第三方音频数据。

## 档案字节与复核边界（2026-10-05）

七份源码不改写、不格式化；`-text` 只作用于下表具名文件，保证暂存及后续 checkout 不因 `core.autocrlf` 改变原始字节。下表 SHA-256 与长度按原始文件计算，不能用规范换行后的内容冒充冻结身份。

| 源码 | 字节数 | SHA-256 |
| --- | ---: | --- |
| `audio-ab-harness.rs` | 17335 | `e213a6dade9a0e4885608e548d1a07cfc1abd4ceaee426dd78e59741b1d0ed1f` |
| `write-only-harness.rs` | 8448 | `e8ceee6c1c273cdf038ab87d3b10e96d0ddb9ec9dd448a0d68976189110d069a` |
| `capture-route-watch.rs` | 4900 | `eed2374b617e532e839b14330bfc8ce53b649756bb74e28b7af1d741e9b89b98` |
| `old-production-audio.rs` | 57039 | `7fdac64f87bd1697fb2d4c003a0a9a6e118130d9433401eaa0b5f8f585987f95` |
| `installed-recovery-audio.rs` | 79760 | `25ee719c468e4483683e98ae7a2718d988be58e58ed692a6c91508fbe775a1fe` |
| `old-production-ble.rs` | 99207 | `65d1befee17dc88e5b9b01a786c62aa22b3859117bc473057eaa6f86cd475f48` |
| `installed-recovery-ble.rs` | 107458 | `b531653df620c2a3c2342256413ee5fb5751beec57544734b79d1da437716aa3` |

`audio-ab-harness.rs` 与 `write-only-harness.rs` 均通过相对 `#[path]` 包含两份精确 audio 模块，并引用当时 `sayall_windows` 的音频类型、wasapi 0.24 和 windows 0.62；harness 自带局部 `ble::gatt_note` 输出适配。consumer 版本增加公开 Capture 帧计数与 ContainerId 校验，音频数据指针不解引用。`capture-route-watch.rs` 只读公开默认角色与 Capture session 分类，具有1800秒上限、连续错误停止和显式停止标记。两份 BLE 源只供时序静态比较，不被 harness 包含。

档案复核已确认七份文件、暂存 blob 与 Git checkout 过滤后字节的 SHA/长度一致，两份相对模块均存在，九份重复源与下表正式 Git 来源逐字节相同；检出字节在内存中核验，没有新增源码副本目录。原始 CRLF 文件的空白检查按 `cr-at-eol` 识别其行尾，源码字节不变。此次没有运行任何 harness、观察器（包括 `--once`）或设备实验，也没有编译档案目标。七源保存不等于独立构建闭环、当前 API 兼容或当前功能 `passed`，不把旧模块接回产品或默认 CI。

`source-ab-manifest.json` 的6条和 `consumer-manifest.json` 的2条原始记录已静态核对 SHA/长度全部一致；这些机器清单、完整日志和停止标记继续本地保留，不随源码归档提交。源码未发现真实设备地址、个人路径、配置原文、凭据或音频数据；audio 中全零 `bthenum-device` 测试值是合成身份。

## 重复快照的正式源码指针

以下九份 head/index/candidate 副本与正式提交 `ff7cb8625f46a2712b942c40c585af303314fd2e` 的对应 tree blob 原始字节一致，可以通过这些指针复核，不依赖 Codex checkpoint 或 stash 引用。这里的 head/index 是当时审计标签；`old-production-audio-candidate.rs` 实际是 head 副本，不能替代前述真正 old 生产源。

| 重复副本 | 正式提交与路径 |
| --- | --- |
| `head-audio.rs`、`index-audio.rs`、`old-production-audio-candidate.rs` | `ff7cb8625f46a2712b942c40c585af303314fd2e:crates/sayall-windows/src/audio.rs` |
| `head-ble.rs`、`index-ble.rs` | `ff7cb8625f46a2712b942c40c585af303314fd2e:crates/sayall-windows/src/ble.rs` |
| `head-lib.rs`、`index-lib.rs` | `ff7cb8625f46a2712b942c40c585af303314fd2e:crates/sayall-windows/src/lib.rs` |
| `head-power.rs`、`index-power.rs` | `ff7cb8625f46a2712b942c40c585af303314fd2e:crates/sayall-windows/src/power.rs` |

`head-to-current-audio.diff` 中的 `head-audio` 标签按上表解析，`current` 是当时取证候选，不是今天的工作区；原始 diff、`head-hashes.json` / `index-hashes.json` 仍本地保留，不改写其历史记录。19秒内部溢出仍未关闭，lead/调度失败实验保留最小复现资料，60秒远端停止调查仍暂停。保留现场与交付对照直到对应调查明确关闭或其复核用途被明确取消；不能因本次归档或后续单次无溢出而扩大成功结论。
