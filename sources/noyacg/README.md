# NoyAcg 0.1.3

保留独立源身份 `com.noyacg.koma`、现有作品/章/页ID、能力、JSON解析、参数、顺序与分页语义。0.1.3仅将已过时的beta.noyteam.online主机及活动表单Referer换成当前官网noymanga.com，并使get_listings返回正式popular/latest导航元数据，不再把漫画列表冒作导航。当前完整官方角色图及App圆角/几何保持。

## 当前公开合同与限定修复

2026-10-21正常匿名公开读取的父链为https://noymanga.com/ → /assets/index-BfXnMy5n.js → rankings-T13-2B3F.js → rankings-CYXAJkYb.js。排行模块明确/readLeaderboard及page/type参数；form-BwbhdKB8.js明确POST、URLSearchParams与application/x-www-form-urlencoded；client-NJP-QThl.js明确baseURL=/api。当前准确排行请求为POST https://noymanga.com/api/readLeaderboard。原源自097c343保留的旧beta主机在237本次012普通入口实际HTTP405/client_error；没有把405冒成403或WASM初始化失败。

get_listings现在仅返回id=popular/name=热门/kind=popular与id=latest/name=最新/kind=latest，无HTTP回调。它们匹配原get_manga_list的popular→day、latest→booklist分支；未新增榜类、改月份token或改变分页。实际当前WASM无网导出与App导航消费者已核；这不证明上游内容可用。当前官网详情/reader另声明v4路由，旧源b1详情/章端点未请求，兼容性未知，未猜补迁移。

## 当前Host与237原生边界

本机ureq对当前域准确排行请求唯一一次返回TLS init UnexpectedEof，未得到HTTP状态或正文；没有重试、改TLS/UA或换客户端。该macOS传输失败与237结果分别保留，不互相替代。原SDK信封中的hostHints.network=false是固定输出后缀，不能据其否认实际HTTP回调。

237（VDE-AL00，展开竖屏1320×2120）普通系统文件选择器更新0.1.3成功；管理/Browse两消费者显示同一完整官方角色图与0.1.3，原49项sourceId/enabled顺序逐项相同。随后唯一一次普通热门入口：get_listings无网成功，get_manga_list于06:18:42返回HTTP200，接着原生安全日志为reason=unknown/error=auth_required。源既有逻辑仅在响应包含精确status=login标记时发该码；当前停在明确账户边界，不登录、不发Cookie/凭据、不绕路或再请求内容。没有取得作品、详情、章节或正文，不能称内容修复/可用或整源通过。

原始证据留在Koma/.hvigor/outputs/user-flow-audit：noy-public-contract-*静态父链、noy013-current-rank-host.{json,stderr.log}、noy013-formal-listings-host.{json,stderr.log}，以及device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership中的noy012-diagnostic-single-entry237-001、noy013-{import237-001,manager237-001,browse237-001,popular-entry237-001,popular-auth237-001,session-restored237-001}。本轮只清理自己传入的013包，并恢复原10000ms/读回后释放租约；源包保留已安装013供后续具名调查。未改App样式、源启用/索引或用户书架/下载数据。
