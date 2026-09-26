# 当前主要能力与验证边界

产品方向与计划导航见 [PLAN.md](PLAN.md)。本页只列实现与成功验证均可定位的主要能力，不承诺未覆盖的硬件或第三方应用行为。

## 验证依据

以下能力采用 [Windows CI run 34374972029](https://github.com/GetSayAll/remote-mic-app-windows/actions/runs/34374972029) 的成功执行证据。该运行绑定源码提交 `6504010828b12713ce033cb3e231087af6a6482f`，于 2026-09-09 UTC 触发，`verify` 及下表对应步骤均为 `success`；2026-09-10 建档时已核对该 SHA 与本地代码基线一致，本次仅追加基线文档。

执行环境为工作流的 `windows-latest`。验证命令与行为由 [Windows CI 定义](../.github/workflows/windows-ci.yml) 及其调用脚本拥有；未执行或被忽略的测试不计入通过范围。后续实现变化时，必须重新核对证据适用性，不能仅凭这次 CI 成功沿用能力结论。

| 能力及可证明范围 | 实现与验证定义 | 成功证据及限制 |
| --- | --- | --- |
| ATVV 语音管线：控制事件、ADPCM 解码、分片累积、连续会话和中断后清理 | [管线](../crates/sayall-core/src/pipeline.rs)、[解码器](../crates/sayall-core/src/adpcm.rs)、[RC001 场景回放](../crates/sayall-core/tests/rc001_scenario_replay.rs) | CI `Test Rust workspace`，执行 `cargo test --workspace`；证明夹具及纯逻辑行为，不代表固件、无线传输或实际音质 |
| 设置界面与宿主交互：页面导航、仿真连接/音频状态、映射编辑和快捷键计划、深色/系统外观设置闭环 | [Vue 入口](../src/main.ts)、[IPC 桥](../src/lib/bridge.ts)、[宿主](../src-tauri/src/lib.rs)、[页面用例](../src/runtime-simulation.ts)、[仿真脚本](../scripts/test-windows-runtime-simulation.ps1) | CI `Test frontend`（`pnpm test`）、`Build frontend`（`pnpm build`）及 `Test Windows Tauri WebView and IPC runtime simulation` 通过；WebView/IPC 实际运行，BLE、WASAPI、Raw Input 和 SendInput 使用仿真后端，不证明真实设备或注入送达 |
| 诊断日志与摘要：默认日志初始化、结构化元数据和前端摘要展示 | [宿主诊断](../src-tauri/src/diagnostics.rs)、[平台日志](../crates/sayall-windows/src/ble.rs)、[日志集成测试](../crates/sayall-windows/tests/diagnostic_log.rs)、[摘要仿真](../src/runtime-simulation.ts) | CI Rust 测试与 WebView/IPC 仿真通过；证明测试覆盖的写入和展示，不代表已复现所有真实故障、完成日志隐私全量审计或验证系统剪贴板回读；使用规则见 [LOGGING.md](../LOGGING.md) |
| NSIS 当前用户安装生命周期：安装与启动存活、升级身份与数据保留、降级拒绝、卸载保留数据 | [安装钩子](../src-tauri/windows/installer-hooks.nsh)、[生命周期矩阵](../scripts/test-windows-install-lifecycle-matrix.ps1)、[静默安装用例](../scripts/test-windows-silent-install.ps1) | CI `Test install upgrade downgrade and uninstall matrix` 与 `Test silent current-user install and uninstall` 通过；使用 CI 生成的前驱版本夹具，启动只检查进程存活，不等同于历史发布包迁移、可见安装界面、双系统真机矩阵或遥控器功能验收 |

## 硬件与交付边界

RC001、RC003、真实音频回环、闲置后首次注入、睡眠/断连恢复与第三方听写的当前版本验收，未由上述 CI 证明。已有现场实验保留在原调查和测试文档中，本页不将历史通过改记为当前版本通过；相关用例和剩余验收沿 [PLAN.md](PLAN.md) 的真源导航读取。

CI 另已通过生产构建排除 runtime-simulation 的检查。仿真通过不放宽基础路径的权限、第三方边界或发布门槛；安装包、签名与发布资格仍按 [RELEASING.md](../RELEASING.md) 判断。
