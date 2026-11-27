# コミックDAYS

讲谈社漫画服务，提供连载、排行、免费活动和杂志分话。最低 Koma 版本为 1.5.5。栏目、作品分类、设置及运行提示提供简体中文、繁体中文、英语、日语和韩语。图标来源见 [ASSET_SOURCES.md](ASSET_SOURCES.md)。

对照版本为 Keiyoushi [ComicDays.kt](https://github.com/keiyoushi/extensions-source/tree/c09b174459c037c62292e4e7a929a0cd59430bc4/src/ja/comicdays) 和 [GigaViewer](https://github.com/keiyoushi/extensions-source/tree/c09b174459c037c62292e4e7a929a0cd59430bc4/lib-multisrc/gigaviewer)。排行、限时免费和杂志分话另对照[官网](https://comic-days.com/)的公开页面和客户端 GraphQL。

| 上游或官网 | Koma ABI 与源内实现 |
|---|---|
| `seriesListIds`、`getFilterOptions` | `get_manga_list` 的连载、最近更新和 `get_filters` 的 29 个题材沿用 Keiyoushi 的 8 个连载分组及 Genre 节点。连载按原站数组、最近更新按最近发布排序。 |
| `/ranking`、`/#days-original`、`/magazine` | `listing` 读取官网数据字段 `seriesRanking(ALL)`、`seriesRanking(ONESHOT_AND_NEWCOMER)`、`comicdaysFreeSeriesCampaignList`、`comicdaysMagazineGroup`。杂志分话按当前刊物里的作品去重，进入目录仍使用真实系列 ID。官网排行使用 Next.js 页面，旧网页入口 `/series`、`/oneshot` 及 `/magazine` 的布局不同，直接查询对应数据可以避免下载整页。合集每 80 条用 `args.page.cursor` 续页。 |
| `searchSeries`、`series` | `search` 有关键词时查作品，无关键词时查分类。`get_manga` 返回系列详情并保留原站第一话或首卷链接。 |
| `fetchReadableProducts` | `get_chapters` 用共享目录代码分话、卷续页，保留锁定、下架和公开语义。付费或下架话同时返回 `locked:true` 和显式 `🔒 ` 标题前缀。付费和下架隐藏设置默认关闭。 |
| `getPageList`、`ImageInterceptor` | `get_pages` 检查匿名公开权限和正文结构，锁定话返回 `chapter_locked`。`get_image_request` 保留原图 CDN、Referer 和页序，实际 `baku` 模式交给随包处理器。 |

[共享协议与阅读实现](../gigaviewer.rs)使用官网桌面 viewer。[4×4 处理器](../../image-processors/gigaviewer-tiles/src/lib.rs)保留尺寸和不满一块的右、下边缘。杂志入口覆盖公开杂志分话目录，会员刊物的购买及订阅由原站处理。
