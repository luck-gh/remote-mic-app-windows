# RC003 future-open 元数据证据（2026-09-19）

当前诊断已完成一次120秒元数据观察及正常清理；没有观察到可建立来源的新打开，用户态 future-open 路径到此停止。既有应用、配置和设备保持不变；三键与键盘共存仍未修复。

- actual JS VM 用例：11 passed。覆盖既有/非选定/前缀/PDO拒绝、future Win32/NT精确别名、失败/pending、嵌套去重、close失败撤销、duplicate全部失效、在途open被epoch拒绝、RootDirectory精确拼接/未知拒绝、容量及stop清理。任何IOCTL buffer访问会让fixture报错。
- C官方固定devkit构建 passed；同构建入口透明线程观察器测试结果见thread-tests.log。真实宿主运行结果单独记录，不以编译代替。
- 白名单由CM_Enumerate_Classes(INTERFACE)及CM_Get_Device_Interface_ListW按已验证selected devnode精确列举。只用公开符号名及字面NT别名，不接受其共享对象目标。接口0/错误/超界在attach前拒。
- 本版只读开/关/复制元数据和目标IOCTL代码/返回状态，不读任何输入输出或IO_STATUS_BLOCK。同步、pending、失败均无报告内容读取。
- NtClose onEnter撤对应绑定；任意NtDuplicateObject onEnter清全部；open跨mutation epoch拒。该保守失效可能漏采，不是生产捕获生命周期。
- 匿名字段：interfaces/tracked，open_calls/exact/unknown/open_failed/open_pending/raced/opened/nested/capacity，close/revoked/cleared_close/cleared_dup_all/duplicate，query_failed/name_rejected，ioctl_known/ioctl_unknown/synchronous/pending/failed。没有接口名、句柄数值或设备标识。
- 拟定一次自然状态120秒观察，无需用户按键/断连/重开。外壳最多等待190秒，超时不强杀；native有stop ACK和原有bounded unload/detach，模块物理卸载仍unknown。没有future精确open或只被保守失效清空须分别报告，不能转为要求用户反复按键。
- 附加前已打开句柄不可证明，未来打开也不保证发生，因此本诊断本身不能作为冷首用生产解决方案。不得据其成功声明三键映射或实体键盘共存passed。

来源与构建身份见freeze.json / probe-build-identity.json；9/15的6396合法PDO mismatch仍是历史失败证据，本版已移除该错误匹配和payload分支。

## 实际执行与初始化修正

18:56 首次 inspect：选定宿主身份校验通过，首次调用接口 JsonBuilder 前缺少官方 core devkit 的 frida_init，探针自身以 0xC0000005 退出。Windows Application Error 1000 的 module=unknown/offset=0；未创建 Session、未 attach。该路径是本轮新增枚举在 inspect 中首次使用 GLib，旧 inspect 没有该调用。已将唯一 frida_init 移到身份门后、首次 JsonBuilder 之前，与官方 devkit example 的初始化顺序一致；不新增注入动作。先前失败原始事件保留 inspect-before-initialization-fix.json。

18:59 最终冻结 C a6dc23bc…、EXE 5add917c…、run 75dd622e…，JS/test 不变；5/5 hash 经原 reviewer 增量复核，无阻断。C 重编 passed。随后 inspect exit0，199 个注册接口类中得到所选 devnode 的2个公开接口；同宿主身份全部门禁通过。完整匿名事件见 inspect-result.json。

本轮不存在已知自然重开触发，原600秒是为等待用户动作的历史窗口，因此改为120秒只观察当前自然状态。窗口零打开不能证明设备永不重开，也不能成为要求用户等待/重连的产品方案。

## 唯一自然状态观察终态

19:00:53.157 hook_ready 至19:02:53.543 stopped：2个所选公开接口；open_calls/exact/opened/ioctl_known均0，ioctl_unknown=550，tracked=0；close=1、duplicate=1，但cleared_close/cleared_dup_all/raced均0。不是保守失效器清除了已建立关联，而是本窗口根本没有future open。没有读取报告内容，不代表不存在三键或未来永不重开。此结果不能满足现有通道、冷/闲置首用的生产归属要求，因此停止此用户态路径，不要求用户重复按键、重连或等待，不开payload。

stop ACK、script_unload=0、session_detach=0、manager_close=0、worker exit=0、native exit=0。session_detached code=1是通知原因，与显式detach API结果0不同。模块物理卸载仍unknown，不声称完全移除。19:04只读核探针实例0；原安装应用PID10324仍在原路径且响应，EXE SHA256 d7b2f1d88900630c31a95756ced124bbeeb1c60bd58f4ad080117ad265e776a4；精确目标宿主仍存活。见capture-result.json、post-run-state.json。

原reviewer独立核5项冻结及实测终态一致，无新增安全阻断；只证明该受控元数据实验与正常清理，不证明三键支持。未新打应用包、未安装驱动、未改安全设置、未强杀、未提交本轮诊断。RC003三键和键盘共存仍未完成；RC001无硬件。后续原KMDF候选须独立核本地实现/签名外部条件，不能自动采购/提交微软或安装未签名候选。
