# 60 秒停止核查与旧/新完整包实测（2026-09-16）

## 当前结论

用户明确新包两次、旧包测试都在持续按住时约60秒自动停止。四个长会话均先收到遥控器ATVV `[00 02]`，23次MIC_EXTEND、没有本机MIC_CLOSE或queue overflow，随后热键UP和正常排空。协议公开参考的0x02为HTT release类别，不能等同用户实际松手；也不能仅据该报文证明RC003不可突破的硬件上限。

这与9/15新包19秒内部PCM queue overflow是不同停止链路。旧/新同时复现排除了“仅本次恢复补丁才出现60秒停止”的结论；先前持续吞吐失败仍保留，不因今天未溢出而宣称消失。

## 新包固定身份

新安装包SHA `fbc792f8fd356870508bbba1362a495fa4b96c6997a7f4b8deb8ae35c89bfe03`。9/16读取安装EXE SHA `0edbe8c5daf7697b09cb1bf9213a5b87dff781e1df5dc9b4cc0d0603b03c747c`，与9/15安装后记录逐字节一致。新实例PID28380/session1，10:07:00.013启动，WMI核安装路径匹配；不是昨日PID71260。

- gen1 10:07:47.719→10:08:47.734，60.015s，UP 10:08:47.818，正常排空831360samples（51.96秒PCM）；尾队列380。首写延迟约182ms不能解释约8秒PCM差额，既有日志没有源包数/首末回调，无法追溯原因。
- gen3 10:09:43.216→10:10:43.215，59.999s，UP 10:10:43.304，正常排空950880samples（59.43秒PCM）；尾队列682。
- 两次均23条续期写日志，最后距STOP2252/2265ms，无关闭命令/解码同步/失败事件。语音键独立物理UP时间未被现有日志记录；`source_release`原标签只是收到ControlSTOP后的代码命名，不能当人体松手证据。
- 昨日observer在18:55正常结束，完全没有覆盖今日两次hold。今日事后默认Console/Multimedia=CABLE Output，Communications=other，不能倒填历史WeType私选。

## 旧包真实对照

旧包SHA `0d437e5feabe19428cf06a4e3614f24e1f152b8241439c915575a70cdd19fd0f`，用户正常退出后Get-Process/WMI双门禁0；管理员覆盖安装exit0，实际app SHA `e8375887be4f003fbaacc64eccd8206861866a5be86c4b89c38ab278a4dd4cf7`，原始载荷与安装载荷仅Tauri UNK→NSS 3字节offset13493186；Helper/driver资源逐文件匹配，未装驱动。10:28:39验收PID30728/session1/ordinaryExplorer/non-elevated/响应和配置hash一致。配置三文件存在状态和私有备份已保存，未改默认Capture或WeType配置。

- 长1：10:32:06.584→10:33:06.567，59.983s，WeType公开Capture session active在other端点；audio正常finish957360samples/queue0。
- 长2：10:33:39.132→10:34:39.131，59.999s，observer10:33:40.159确认WeType active于CABLE Output，并在10:34:40.061变inactive；正常finish同样957360samples=59.835秒PCM/queue0。
- 两长会话均23次MIC_EXTEND，末次距STOP2354/2370ms，关闭命令0，失败事件0，UP比remote STOP晚85/86ms。中间还有1.020s、末尾6.751s短会话，分别正常排空13680/105600samples；不混入长会话统计。
- observer观察到测试中默认Console/Multimedia由other变CABLEOutput；本任务没有调用setter，公开回调不提供actor，不能猜测谁改变。
- 旧包只用于受控对照，会回退sink自动重建与释放/生命周期guard。用户再次正常退出后，已完成固定fbc新包恢复，双进程门禁0，绝不强杀。

具体机器证据：`session-timeline.json`（新）、`old/session-timeline.json`、`old/capture-route-changes.json`、`old/delivery-status.json`、`old/installed-runtime-evidence.json`。

## 诊断开发候选（不属于上述已安装包）

`diagnostic-manifest.json`：audio5edfb3…、ble6a958…；正常finish聚合accepted/submitted/首末提交，BLE各代只累加callback时点/字节/解码/队列迟到和原始停止原因。3项定向测试passed、fmtcheck exit0，独立只读review无P0/P1。无每包日志、无新COM/参数/时序变更。协议音频无session-id，host callback归属不能绝对识别新START后才到的旧音频；详见diagnostic-source/review.md。

未打包/安装该诊断候选，未宣称BUG修复、未Git提交。三键与实体键盘并用仍未完成，RC001没有本轮实机证据。


## 恢复与收口（2026-09-16 10:39）

恢复包fbc792原固定产物安装exit0；`restore/delivery-status.json`和`restore/installed-runtime-evidence.json`证明实际EXE回到0edbe8…747c，Tauri仅UNK→NSS三字节offset13509554，Helper/所有资源匹配，配置hash不变。当前PID30468/session1/ordinaryExplorer/tokenElevated=false/响应passed，10:38:42 RC003 Ready，原选输出端点restore56ms passed。没有改Windows默认Capture/WeType配置，保留用户自己的改选，没有装驱动或启用新诊断代码。

只读observer收到固定停止标记，10:39:38.497正常退出exit0，elapsed620331ms，进程数0；没有遗留采集进程。机器证据见 `old/capture-route-watch-result.json`。其停止标记保留在ignored证据，下次确需同observer应显式处理该标记，不假设旧观察仍在运行。

最终限定：当前这台RC003/当前固件配置在新、旧包各两次持续按住时约60s自动停止，且旧包有一轮公开确认WeType实际在CABLE Output采集。远端先发HTT释放类别STOP，设备侧固件/协议行为是高可信推断；没有小米官方60秒规格或精确固件实现证据，Windows GATT写成功也不等于遥控器实际采纳EXTEND，不能称硬件不可延长的固定上限。9/15约20s队列溢出独立未解决。新增输入锁定方案仍等待用户明确私有API边界，未提交Git；诊断3tests/reviewpassed只是开发证据，不是BUG修复验收。
