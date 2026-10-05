# TV 再按未取消系统任务列表

- 发现日期：2026-09-27
- 状态：2313101f修订版两模式再次TV取消实测仍failed；用户明确要求停止排查。未resolved，完整机制未闭合。
- 影响范围：Windows 0.2.6包a3586eed、RC003，Applications与Desktops。
- 现象：用户能进入两种列表，再按TV不能取消；打开观察不等于导航/确认/首用通过。
- 复现条件：内置Agent的TV短按Ctrl+Alt+Tab或长按Win+Tab后，再按TV。
- 正常预期：打开按压已完整释放后，下一可信执行TV新DOWN即请求Escape；该次重复DOWN、Long和UP全部消费。原始长按不自取消，外部切走/确认后不向其他窗口补Esc。
- 证据：2026-09-27 14:40:04.232Z等Applications、14:40:24.791Z及14:41:36.938Z Desktops均launch=ok后0–1ms closed reason=foreground_changed；随后存在TV执行DOWN/UP，无navigation/Escape请求。不是已证明Esc无效。
- 根因边界：模式被现有前台身份判定立即清掉已证实；原日志缺拒绝窗口类别，无法区别本机Shell窗口类型、进程归属或迟到观察。一次当前前台只读核对已为非Shell/other，不拿当前窗口替历史。原Shell PID+两类窗口守卫不放宽。
- 修正：SceneController仅在执行边沿订阅中用真实新TV DOWN取消，复用原guarded Escape；消费标记保留到下一次新DOWN，UP不触发重新打开。UI-only物理观察不驱动执行。增加本动作前台检查日志：同Shell、有限系统公开class、原/目标相等关系、观察与模式代次及拒绝错误；不读取标题/控件/内容/路径。
- 验证：先运行第二TV DOWN回归failed（只有启动、未在DOWN取消），最小实现后相关7项passed，覆盖两模式、前台观察先后、原长按重复、第二次按住/重复/Long/UP、外部切走及原生确认迟到。工作区349 passed/17 ignored、fmt/workspace check/simulation check passed；前端179项与真实IPC14步未变证据复用。诊断包及实际两模式取消待本候选收口；不能称完整根因已修复。
- 隐私检查：长期记录无设备身份、个人路径、语音内容或凭据。过程材料复用target/dev/rc003-three-key/tv-cancel-*，首失败和最终结果保留到验收闭合。

## 诊断候选交付

2026-09-27 22:59:28本地0.2.6包SHA256 `52047c89b14db9d94f1fd88f254d07b2cfa83f7ad01ded62bfd53298e25f1eac`，构建41503 exit0；旧App47868/Helper41732正常退出，overall272ms/failed_stages0，Helper清理/卸载/分离/terminal0。管理员原目录覆盖exit0，Explorer启动App49276普通权限、Helper41760提权，载荷核对通过，三份最新用户配置字节相同。

23:01:22.738基线，run3923634ce0f44aa6b854b1dfa73b2c35已握手及mask31最新回执，仍pending_per_request，不称动作ready。既有Codex→无用户覆盖内置Agent有效，无隔离配置写入。请求每种模式各一次打开→再次TV取消；不插准备键、不反复失败动作、不向第三方发送消息。此刻本机系统类拒绝原因及完整取消结果仍pending。

## 23:15 诊断组与有限握手修复

52047c89同一App49276/run3923634ce0f44aa6b854b1dfa73b2c35：23:15:53.845 Applications、23:16:02.667 Desktops launch=ok，随后1ms观察均为 `shell_owned=true public_class=ForegroundStaging accepted_class=false target_known=false origin_same=false`；模式立刻以foreground_changed清除。后续TV执行DOWN/UP仍收到，但没有tv_cancel_down或Escape调用。用户确认两个界面均无法再次TV取消：failed；证据不支持“系统拒绝Escape”。

最小修复仅保留本代启动中、未确认最终目标的同Shell精确ForegroundStaging；它不成为可注入窗口。最终目标继续限定既有公开类、Shell归属与当前HWND。第二次真实TV DOWN只登记一次取消意图，重复DOWN/Long/UP消费；最终目标确认后才提交受保护Escape。旧窗口代观察不覆盖较新目标，无关Shell/外部窗口仍取消。未确认目标最多10秒，届时仅重新读取当前公开身份一次并失败关闭、记录target_unconfirmed_timeout，绝不因超时将未知窗口判为目标。已确认界面不设用户操作截止。

阶段窗口回归先RED（模式被提前清除），修后10项task_switch GREEN；两模式阶段→最终、迟到观察、第二TV整次消费、无关窗口、超时零注入均覆盖。工作区352 passed/17 ignored，fmt/workspace check/runtime-simulation check均exit0。前端179与真实WebView/IPC14同内容证据复用；本次不修改UI、来源协议、手势阈值或WDF生命周期。

这证明具体生命周期缺陷已修且软件门禁通过，不证明本机一定产生另一最终窗口，也不证明两模式真实取消成功。若复验持续ForegroundStaging，将以有界终态给出下一证据，不盲目放宽窗口类。过程仅保留tv-staging-*及唯一tv-cancel-classification-failure.log对照，实机闭合后按LOGGING回收可重生成中间物。


2026-09-27 有限阶段修复候选交付：唯一生产构建64077 exit0，含前端生产构建；0.2.6安装包 `target/release/bundle/nsis/无线麦 SayAll_0.2.6_x64-setup.exe`，2026-09-27T23:38:11.7807534+08:00，26,430,208bytes，SHA256 `2313101f2bdd6e838426451b18c261cee1feb8de254c1a9a549f911d245f1eb1`。旧App49276/Helper41760由既有托盘入口正常退出，客户端clean=true/exit0/142ms，应用overall265ms/failed_stages0，两个原实例均消失。管理员原安装目录覆盖exit0；真实Explorer启动App48704（23:39:40.9396659，TokenElevation0）、Helper41632（23:39:43.0542025，TokenElevation1）。

安装App SHA256 `8bfbe22164fde5f711685a01426e2b44b30d3e177800297d617d997102478338`与release只差NSIS标记；Helper `e64e1c95ce5bb014784fa7fc996494bd898fc84830a6939cce9cda465508f3d1`与本次固定构建载荷精确相等。settings/button-mappings/capture-input-session与正常退出前最新hash全部相同，未回写旧配置。run `70db0902319a46c9bb2beccc53bafa7a` 已peer_verified、configuration1/mask31 accepted=true、raw_released=true；23:39:44.276原生bound仍source=pending_per_request，未收到报告不能称增强ready。Codex→preset-agent且无用户覆盖，保持当前TV默认，旧Notepad BUDTH不作本组入口。

新包两模式再次TV取消待用户实际观察，不加初始化键、不自动按键；首开失败停止该段，取消失败只用实体Escape退出一次，不反复试。若过渡类没有最终窗口，10秒终态将明确拒绝而非向未知窗口发Escape。所有既有其他pending未外推。仅保留本轮tv-staging红/绿/门禁/生产日志、包元数据/安装配置hash及上次唯一分类故障对照，用途为同候选复验，闭合后按LOGGING清理可再生材料；未清历史、无Git写入/电源/无线电/强杀。

## 用户停止排查

用户对2313101f修订版反馈“依然是无法取消的状态，不过这个也还好，不纠结这个问题了”。按本次两模式复验记Applications与Desktops再次TV取消均failed；先前ForegroundStaging提前清除是已证缺陷，但修改后仍不能取消，完整失败机制未闭合。依用户指令停止此Bug调查、修复与复验，不再读取新运行日志、不尝试接口或设备操作，也不以软件测试通过标记resolved。保留现有候选代码与全部未提交状态，不回滚或删除TV功能。上一轮已NEEDS_USER，无本Bug在途工具或等待控制器；未扫描系统、未执行Git写入。其它任务与主PR不因本次停止而取消，本轮不自行推进它们。
