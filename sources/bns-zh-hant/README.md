# 劍靈官方繁中漫畫 0.1.1 候选

沿现有 NCSOFT 台服劍靈源，消费官网无尘篇 `library_data.js` 对应的漫画分类原始行。当前保留原件有《悲運的徒弟，無塵》、《慟哭的星 第1集》、《慟哭的星 第2集》三个独立阅读单元；不是三个新源，也不把人物分组当作一本连续漫画。目录保官网行序 comic3、comic2、comic，不按文件名或标题排序。小说和史诗任务保持过滤；其它人物组、显式身份/路由覆盖和全站内容没有验收。

旧作品 `bns:zh-hant:musung:comic`、章节 `bnsread:zh-hant:musung:comic`、PageID `bnspage:zh-hant:musung:comic:0..2` 与既有字段保持。新单元使用同一身份格式的 comic2/comic3，各仅一个完整阅读单元，chapterNumber/volumeNumber null、continuationEligible false，不猜跨故事关系。标题核官网 h2 与对应目录行一致，创作者取详情文/图角色节点；旧书原空作者字段不补造。新两单元原文李次銑/圖朴鍾成。

详情 currentArr、zh-TW、漫画类别与所选单元须一致。完整消费 `#view .viewArea > .page > img` DOM顺序；data-src 存在优先，缺失才读无 data-src 的 src，spacer 排除。当前三单元为3/23/35图。第二集文件名28P后直接31P，绝不补图。删除固定3图与256字节URI槽；复用现有1MiB正文/HTTP ABI预算保存目录原体和请求URI，不增加页数阈值。任何后段缺图、属性失败或输出容量不足使整次失败，不发前半份成功。Text/Attr填满输出预算时明确报容量不足，避免误认完整值。

所有消费者共用单元归属：列表/搜索/详情/章节/get_pages/image_request。正文计划必须匹配所选单元、ordinal和当前详情完整DOM中的原URI，封面MID与正文PID互不混用，错单元URL拒绝；不改原HTTP scheme、host或添加凭据。元数据HTTP只保实际原UA，删除旧版固定指向另一阅读单元的Referer；所有可信公开原站取样均无Referer成功，未从此推导官网强制头要求。正文imageRequest仍空headers。

计数 SelectAll 使用零输出容量，生产App C++仅为写入的槽分配descriptor，所以不取得子句柄；Text/Attr所选节点在读成功或失败后均Close；详情根句柄统一在所有分支Close。同体生产WASM/实际App C++ HtmlHost与Lexbor离线回放记录 OperationEnd，全部0存活句柄。

请求成本是本候选显式行为差异：为每次解析使用当次官网目录身份，列表main+data+3detail共5GET，单作品详情/章节/get_pages/正文plan均main+data+所选detail共3GET，封面plan仍main+data共2GET。旧正文plan/get_pages原仅1GET，详情原仅2GET；没有实现跨operation缓存或批量下载优化。Native延迟未验收。

原封面实体仅旧书 thumb_comic.jpg 已保留8552字节JPEG。新两封面URL来自官网footer公式，后续已各一次普通GET200并完整解码目检，均为250×142真实JPEG；原件保留在前候选covers/comic2、comic3。不借首图冒充封面。两新正文首JPEG已各一次读取并实际目检，其他页只核声明/顺序，不能称所有图片可用。

图标保持原PNG原bytes，不加工素材，不改变App显示圆角。候选包manifest遵守当前必需icon资源合同，新增 icon 字段，旧身份字段保持；version和description准确反映本候选。原010包、原回放和原owner WIP保留。

本最终候选在独立tree使用当前canonical SDK原字节、同一已安装rustup stable工具链离线编译。生产编译使用原owner build.sh的 -reference-types/--strip-all flags，并保留无该flags的开发对照；二者不能混称WASM字节复现。未修改SDK/App/canonical，未部署237，未重跑已接受正文/下载/cold。原owner→当前SDK的48份完整输出/HTTP回调/DOM轨迹兼容回放已另保留；最终去Referer的48对请求另逐实际头比较，响应/DOM轨迹保持。实际canonical集成和Native仍须Root审后分别验证。


## 2026-10-21 237现装0.1.1具名Native观察

设备为VDE-AL00展开态、竖屏1320×2120。未更新源包或App；Browse原生行明确显示本源v0.1.1，普通进入后为三本原顺序及各自封面。本次只选《慟哭的星 第1集》，详情显示李次銑 / 朴鍾成、章节1/1及23页，既有继续阅读为第2/23页。普通继续阅读实际到2/23，再用普通前页回1/23、后页回2/23；原图均显示真实繁中彩色正文：第一页雷雨中追兵与“快點找”“在過今晚之前一定要找到”，第二页逃跑者与“我要培養力量／我要讓自己變強”。当前长条原图在此阅读模式呈居中窄栏；本次未修改模式或样式，不将能解码等同全部屏幕/模式体验通过。

普通Back后同本详情继续阅读仍明确显示《慟哭的星 第1集》与第2/23页。原源合同将该标题绑定comic2、章节`bnsread:zh-hant:musung:comic2`及`bnspage:zh-hant:musung:comic2:0..22`；这是源合同与可见标题/页位的绑定，不是本轮持久文件身份的新实读。私有registry、reader-sessions和preferences导出被自动审批拒绝后未执行、未换方法；本轮没有新文件层进度或全库/偏好不变证明。未重跑旧《悲運的徒弟，無塵》，《慟哭的星 第2集》、其余21页、下载和cold不外推为通过。

原图及回执沿Koma原目录`.hvigor/outputs/user-flow-audit/device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership/`保留：`bns011-browse-visible237-001`、`bns011-list-ready237-001`、`bns011-comic2-detail-ready237-001`、`bns011-comic2-first237-001`、`bns011-comic2-second237-001`、`bns011-comic2-natural-return237-001`。`bns011-session-restored237-001`实际恢复原Browse末端（漫画人／老夫子尾项），读回AWAKE及OverrideTimeout=10000ms；原租约18e67332已正常release。全部新回执关闭artifact hash，未清数据、注入、改偏好、改App容器或样式。Root已亲审两页正文、自然返回及恢复原图；仅接受此具名路径，不外推全源。


### 同日后继：《慟哭的星 第2集》

同一现装0.1.1，重新实时解析237并取得a513360f独占租约，完整wake门禁读回AWAKE/86400000ms及Koma窗口1320×2120。普通详情显示《慟哭的星 第2集》、李次銑 / 朴鍾成、章节1/1及35页，既有继续阅读为2/35。普通进入2/35、前页到1/35、后页回2/35，均实际显示彩色正文；第一页为男子与持刀女子场景，第二页可见“竟然想拿刀刺我／妳知道這是什麼意思嗎”等繁中文字。普通Back后详情仍具名显示本集第2/35页。此单元按原源合同为comic3，章节`bnsread:zh-hant:musung:comic3`；不将合同身份冒作本轮私有持久文件实读。只观察首两页，不外推其余33页、下载、cold或完整阅读体验。未重试前述私有文件导出，也未改偏好、包或产品代码。

导航中的一次偏差保留：从Browse末端连续复用三滚动后，原坐标实际进入包子漫画并显示source_runtime失败；未点重试、不据此判BNS失败。普通Back后以当前可见行重新定位BNS，才接上述第2集路径。错误原件`bns011-comic3-list237-001`和恢复原件`bns011-comic3-browse-recovery237-001`均保留。

具名原图/回执仍在上段同一category-membership原目录：`bns011-comic3-detail-ready237-001`、`bns011-comic3-first237-001`、`bns011-comic3-second237-001`、`bns011-comic3-natural-return237-001`。`bns011-comic3-session-restored237-001`已恢复原Browse末端，读回AWAKE/OverrideTimeout=10000ms后正常release a513360f；全部回执hash关闭。Root已亲审两页正文、自然返回和恢复原图；仅接受此具名路径，不外推全源通过。
