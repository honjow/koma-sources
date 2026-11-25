# 香香腐宅

## 站点简介

香香腐宅的成人向漫画书库，设有独立的 BL 区。站上有分话连载与完结作品，VIP 章节需要站点账号和相应权限。

Koma `com.boylove.koma`，Mihon 来源 `1471112097704477289`。行为移植自 [Keiyoushi 固定版本](https://github.com/keiyoushi/extensions-source/tree/8947c372cd87f546a0b80cb0e4b0a5a8add5ef88/src/zh/boylove) 与其 [Apache-2.0 许可](https://github.com/keiyoushi/extensions-source/blob/8947c372cd87f546a0b80cb0e4b0a5a8add5ef88/LICENSE)。

| 原有行为 | Koma 调用与消费 |
| --- | --- |
| 热门 `getpage` 和每日更新 `getDailyUpdate.html` 从第 0 页开始。热门读 `lastPage`，更新在一页少于 10 条时结束。 | `get_listings` 提供热门和最近更新，`list` 读取 `args.page.cursor`，转换成站点页码。更新栏目不接收目录筛选。 |
| 有关键词走 `searchk`，空词筛选走 `cate/tp`。保留状态、清水/有肉、地区、动态标签和 VIP 过滤。 | `get_filters` 首次返回静态选项，打开时通过 `args.fetchFilterData` 读官网分类页。当前两个域名的桌面网页使用 `ul[data-str=tag] a.cate-option[data-value]`，仍兼容旧页面的标签容器。地区与标签按上游顺序用 `+` 组合，标签本身按 URL 路径编码。 |
| 详情用原题搜索并匹配作品 ID。目录由 `chapter_list` 返回并反转。 | Koma 详情和备份恢复只保证传入作品 ID，`details` 先从原站作品页取得原题，再按 ID 匹配上游同一搜索。SDK typed chapters 保留 `/home/book/capter/id/<id>`、标题和 UTC 时间。 |
| Reader 优先解析延迟载入容器，另有普通图片及旧章 ID 内置图列分支。页内出现原站分段数时对图片横条带还原。 | `chapter_pages` 保留各分支与原图顺序。`get_image_request` 仅在带分段标记时输出像素处理参数，网络请求移除本地标记。普通图透传。随包处理器 `koma_reversed_strips_processor` 保留小图逆序和大图原位分段分支。 |
| 两个镜像和 WebView 登录。VIP 仍由原站账户等级判定。 | `preferredMirror` 同时决定文档、API、相对封面域及普通官网入口。实际两个站的 HTML 分别声明自己的 `img.<站点>`，绝对图片地址保持站点实际下发值。旧阅读相对地址继续用上游共用 CDN。网页登录限两个明确 origin。 |

本机开发宿主已跑通热门、最新第二页、筛选、搜索、详情、117 章目录和公开章 76 页。两站同一封面均解码成功，备用站同一公开章实际下发自己的图片域名。该章正常页图未带分段标记。197 已从热门筛选加载两个域名的「香香汉化」标签，并核对两个域名筛选后均返回作品。官网列出的「三角/多角关系」在当前上游分类 API（含官网脚本的 `mt=0` 写法）返回 HTTP 200 内的业务 500，源保留明确错误。197 已在普通 App 路径验证公开阅读、真实 13 段乱序首图、43/43 下载及冷启动后的离线重开。原站只返回 VIP 提示和跳转时，读取按明确锁章处理。作品目录中相应 isvip 字段可能为 0。登录后能读取会员章仍待实际 VIP 账号验证。集成需要登记两个 crate，源登记最低 Koma 版本 1.5.5。图标依据见 [ASSET_SOURCES.md](ASSET_SOURCES.md)。
