# 首按与 capture 下拉框修复：2026-09-16

状态：已安装 9a7048 候选真实验收 failed；本目录修复源码/测试已通过，尚未安装或真机通过。9a 包及源码继续原位保留，不能覆盖原始失败证据。

## 真实失败证据

原实例 PID39548：17:49:28 generation2、17:50:16 generation5、17:50:26 generation7 均在 set_role_0 成功后约10–15ms被自身 prepare=external_change 中止，decoded/submitted=0，未发目标快捷键 DOWN。随后第二次 generation3/6/8 只需 set_role_2 即确认并正常供音；gen8 36.944秒/588480样本正常排空。这直接证明首按失败发生于本应用路由准备门禁，不能归因第三方就绪或加固定sleep。原日志见上级 first-use-failure-production.log。

旧观察器17:46:24正常达到30分钟上限后退出，而用户会话17:48以后开始，故没有该区间WeType实际capture证据，不补推历史路由。

## 修复与软件验证

- General={Console,Multimedia}、Communications独立。每次setter前持久化transition(mask,before,desired)；组内仅允许before→desired，组外变化或第三值整体让出。apply/restore共用此规则，已确认值才重算changed。crash先验证在途envelope、持久确认actual后才能恢复。无CAS与同值外改不可判因果边界仍在。
- 初始normal两角色分裂时写前拒绝，界面明确说明无法保证精确恢复；不猜配或重试争抢。
- native新增匿名baseline/before_set/after_set/restored角色掩码。目标匹配只证明default，不等于WeType真实采集。
- CSS只约束capture容器/按钮行min-width与select width/max-width，不重设界面。
- Windows路由/取消/恢复19项 passed；Vue页面5项 passed。source-manifest.json四文件独立审查一致，无P0/P1。完整日志 routing-tests.log / ui-tests.log。
- 首次Edge headless实际生成profile但无截图/DOM输出，视觉验证未通过；不重试该不明环境。待新包安装后复用现有真实窗口截图入口检查长select边界。

## 待实机

固定非target基线首按：仅0/1组收敛、Comm独立→hotkey DOWN→WeType实际CABLE Output及用户听写→hotkey UP→精确恢复。包括立即重复/闲置首用/外改/快按等既有矩阵，未完成不标passed。不开启后台setter实验，不改用户默认或映射，不提交Git。独立20秒overflow仍unresolved、60秒研究暂停。

## 修复候选安装：2026-09-16 18:07–18:09

release/NSIS 构建 exit0（2m22s），4/4冻结源码一致。最终包 SHA256 `970e6c4a544b1812d280596606c60ec8403c080e47811c5d03c71e9065e5e652`；原始app `21828edad72253e2f6e8dd015b06cd1852dbc494e9dbf0f8467ee08e54a9f2a9`。完整载荷见 build-evidence.json。

旧PID39548在18:00:48正常shutdown全部passed（130ms），18:06两种进程查询均0、journal存在但null，未清除/恢复用户默认。管理员覆盖原目录installer exit0；真实Explorer普通用户新PID10324/session1/tokenElevated=false、响应正常、单实例。实际安装app SHA256 `d7b2f1d88900630c31a95756ced124bbeeb1c60bd58f4ad080117ad265e776a4`；逐字节仅偏移13787202的UNK→NSS三字节差异，Helper/INF/SYS/CAT一致。保存配置安装/启动前后哈希相同，用户已开启capture_input并已选target均保留，idle setter次数0。SayAllInput服务0，未装驱动。

原render恢复和RC003 ready通过。只读observer普通用户launcher3052/session1于18:08:34启动，现有固定1800s上限，至18:38:34，支持正常stop marker，不读音频。当前baseline三role为CableOutput，必须由用户先改非target再做首按，不能用暖态通过掩盖。实际窗口截图得到锁屏而非应用画面（installed-window.png），因此未标视觉passed，不尝试解锁；待用户回桌面后核验。

安装已passed，首按/恢复/第三方收音与实际长select布局仍pending；无Git提交。配置摘要读取使用真实schema的capture_input字段，未输出ID。


## 2026-09-19 用户验收

用户明确反馈非target首按锁定与UI越界两处“已修复，很OK”。仅这两项按用户反馈记passed；不补造过期observer记录，其他矩阵pending、RC001 deferred、20秒溢出未结项。
