# 既有 KMDF 候选差距核对（2026-09-19）

这是同一三键任务的只读现状核对，不是新一轮内核审查，也没有安装或重新构建驱动。

## 可复用的精确证据

`kernel-source-recheck.json` 对9/14 maintenance-review-hashes.json的12项逐文件 SHA256 核对全部相同。包含 driver.c、protocol.h、input_state.c、C测试、包身份、Rust通道/包维护、组件Helper及构建锁定入口。内核/ABI/维护源没有混入本轮语音与capture修改。

`kernel-package-recheck.json` 的 SYS/INF/CAT 与9/14原包完全同字节：SYS fe080c487b9c0e5e270c558d69e945a46ba3e8e4c1c3934fd9238d0475b9d10d；INF 8cebec2a9cdb724935a20360ca09568f089e10039ce6ed570f898af5503e3321；CAT b3356686203169b794db38b4e9b0caa2fb3ad27076c154dca34e85b97714768f。原KMDF编译/INF和CAT生成、C状态机、指定catalog内核签名拒绝及Helper exit50证据可复用。9/14Windows152测试只作为当日整库测试，不能将当前混合工作树所有源码自动外推为该152项相同SHA。

## 三键最小闭包已存在的部分

- INF只匹配研究过的RC003精确revision，不含RC001；运行时HidP验证121字节Report1及3个u16槽位，不凭VID/PID直接接管未知格式。
- raw PDO与所选HID父节点精确绑定；ABI3、普通用户客户端、显式CLAIM、租约/关闭/取消；全释放观察后准备、队列及序号检查、快速DOWN/UP不合并、重复去重。
- 只清除已声明映射的Back/Vol+/Vol− usage，其他键和ReportID透传；DriverEdge进入已有映射执行器并退出时释放。未知格式/契约失配fail-closed。
- 独立显式Helper固定包hash+指定CAT成员+内核策略验签；未签名包在系统安装API前拒绝；维护独占、失败回滚/恢复、需重启报告已有候选。

## 仍未完成且不能由代码/签名替代的部分

1. 当前用户没有明确可用Microsoft签名渠道，当前外部前置不可用。不能加载本SYS，也不能用测试签名、安全设置变更绕过。未授权采购、申请、上传或发布。
2. 三键实体usage、实际WDF过滤位置/读完成、每键报告→设备通道→抑制→映射尚未在已加载内核上执行。HidP合成格式验证不证明物理键会到达该过滤层；9/15已按键的用户态失败不能转算kernel通过。
3. 有签名后仍须同机最小逐键、冷首用/重复/组合、断连/睡眠、按住退出、Verifier及真实安装/卸载/升级/需重启边界。微软签名可能改变SYS，应以最终微软返回候选重锁常量、重构建Helper/app，而非拒绝所有未来签名文件。
4. RC001无当前硬件/描述符报告/INF绑定，仍硬件deferred，不能复用RC003结论。
5. TV/Home与所有共享VK的实体键盘并用仍是独立未完成实现：ABI mask只有三键；旧常驻/四秒LL来源推断尚未撤除。不能扩大未证usage、仅删LL导致原生+映射双响应，也不能把三键filter签名当作这项已经实现。

本轮没有发现需在签名前修复的三键最小代码闭包缺口，因而不为进度新增泛化接口/重编同SHA。现成可审目录为drivers/SayAllInput、crates/sayall-windows/src/input_driver*.rs、target/input-driver；历史三键应用包artifacts/three-key-input-20260914/无线麦 SayAll_0.2.5_x64-setup.exe（0d437e5f…）仅供身份/历史证据，不回退当前已验证capture功能，也不提供可安装内核。当前安装应用保持不变。
