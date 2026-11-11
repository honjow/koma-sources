# TerraHistoricus

当前源是 0.1.8，带稳定 ID `com.terrahistoricus.koma` 和既有作品、集和页身份。规范 Cargo 和标准源构建注册存在。源保持匿名公开 API 访问；首页、筛选、设置和凭据仍不受支援。本地化显示名和内容语言是单独合同。真实用量仍未知。

当宿主声明 `source_public_cover_user_agent_v1`，0.1.8 把既有 Referer 和 User-Agent 与公开 `web.hycdn.cn/comic/pic/` 封面一起传递。App 可在浏览、最近更新、搜索和详情直接抓取同一 URL。其他封面 URL 和遗留消费者仍遵循既有图像请求解析器，包括单独的签名阅读页解析。当前官方目录有 22 部 Terra 和 6 部 Talos-II 作品。

0.1.6 业务后继更正了完整数组终止、直接 JSON 成员/空白/转义处理、集编号和未知章节语言。0.1.7 后继只改变版本和包身份资产。当前未改的站点声明 512px 资产及其来源在 [ASSET_SOURCES.md](ASSET_SOURCES.md) 描述；旧的黑色合成 favicon 是历史素材。

原 0.1.6 证据覆盖具名 21 项目录和作品 7746、一章、58 中第一真实页和返回。封面仍明显像素化。它不确立全页内容、新下载、冷续读或整源。原 0.1.7 证据覆盖普通索引更新和已安装/Browse 身份行；不添加新内容、下载或冷验收。两条记录都不是这次恢复审计的新结果。

## 历史 0.1.1 记录

完整原 `662b02d2` README 原样跟随。其候选版本、较早资产声明、设备跟进和 `native OPEN` 措辞描述那个阶段。它们不替代当前源、资产记录、具名证据边界或当前人工指令。

```text
# TerraHistoricus 0.1.1 候选

中文官方 Terra Historicus / 泰拉记事社 源。保留 com.terrahistoricus.koma、cid、cid/episode 和 page:cid:episode:n 身份。更正实际 get_listings 响应 ABI、当前对象游标和遗留数字/字符串页解析、热门/最新的两主题终端 null 语义，以及独立 HTTP status/statusCode 分类。既有静态首页区块缺 ID/条目且筛选/设置为空，故那些能力为 false。无 App/SDK/Reader 变化、登录、账号、挑战处理或发布。

本地化名：zh-Hans 泰拉记事社，zh-Hant 泰拉記事社，英文规范 TerraHistoricus。真实用户用量率未知。既有图标/整站评级保留；不从元数据推断流行度声称。

主 Keiyoushi 参考：https://github.com/keiyoushi/extensions-source/blob/5b95ccc4b6a96fb9a4a28cead59255b0af99276e/src/zh/terrahistoricus/src/eu/kanade/tachiyomi/extension/zh/terrahistoricus/TerraHistoricus.kt。当前官方公开 API 提供观察到的合同；这扩展既有 Koma 源且不复制任何 Kotlin 代码。

实际具名宿主路由：热门主题页 21 和 5 作品，最新主题 4 和 4；搜索 大将军 找到 cid7746 循途漫录：大将军，出击！；完整目录 1 集7746/8623；全部58 页描述符；第一原公开 JPEG1534154B 正常抓取。既有旧 Terra上篇 native Reader 路径未重做。这个宿主路由从不替代 native 验收。Root 必须用197 普通订阅/Browse/搜索/详情/当前章真实阅读器/下载/不清数据冷路由；native OPEN。

调查的两个替代方案未实现：Manhuadb 不在当前 zh 目录；Creativecomic 需要超出当前 get_image_request ABI 的加密图像响应转换。两个结论都不是整产品阻塞。
```

## 237 现装0.1.7具名第二页补验（2026-10-21）

沿普通Browse现装0.1.7→21本列表→7746《循途漫录：大将军，出击！》详情，实际一章7746/8623，原继续阅读为1/58。普通进入亲看完整彩色封面1/58，再普通右侧翻页亲看中文黑白叙事正文2/58；随后普通左侧返回1/58并自然Back，详情仍为同章第1/58页。会话读回comicId=com.terrahistoricus.koma:7746、chapterId=7746/8623、pageId=page:7746:8623:1、pageIndex=0、totalPages=58、completed=false。没有新增下载、收藏、登录、改变模式或编辑持久状态。

只接受现装包上述原具名章节首两页和返回；未核下载/缓存来源，不将本轮称作无缓存在线首取，也不扩大为全58页、cold、其他章节或整源通过。原封面低清、图标小槽及其他既有未闭项保持。原Browse末端已恢复并亲看，50项完整registry与进入前逐项相同；原OverrideTimeout10000ms/AWAKE已读回，租约84519278已释放，全部回执哈希关闭。

原件在Koma/.hvigor/outputs/user-flow-audit/device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership/：terra017-{entry,detail,first}237-002、terra017-page2-237-002、terra017-progress-restored237-002与terra017-session-restored237-002。当前源码、版本与资源未改，未构建或更新包。
