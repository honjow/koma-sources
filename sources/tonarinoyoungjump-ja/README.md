# となりのヤングジャンプ

集英社日文漫画站，提供连载、短篇和客串刊载。最低 Koma 版本为 1.5.5。栏目、作品分类、设置及运行提示提供简体中文、繁体中文、英语、日语和韩语。图标来源见 [ASSET_SOURCES.md](ASSET_SOURCES.md)。

对照版本为 Keiyoushi [TonariNoYoungJump.kt](https://github.com/keiyoushi/extensions-source/tree/c09b174459c037c62292e4e7a929a0cd59430bc4/src/ja/tonarinoyoungjump) 和 [GigaViewer](https://github.com/keiyoushi/extensions-source/tree/c09b174459c037c62292e4e7a929a0cd59430bc4/lib-multisrc/gigaviewer)。

| 上游 | Koma ABI 与源内实现 |
|---|---|
| `seriesListIds`、`getFilterOptions` | `get_listings` 暴露热门、更新、短篇和客串刊载。`get_filters` 提供连载、短篇、客串及九个原站题材。源内 `listing` 以和上游相同的 SerialGroup／Genre 节点发 GraphQL，按点赞、最近章节时间或原站顺序去重。网页入口有 `/series`、`/series/oneshot`、`/series/ynjn` 与 `/series/trial`。 |
| `getSearchMangaList`、`fetchSeries` | 有关键词时共用 `searchSeries`，无关键词时按分类取合集。`get_manga` 从 `series(databaseId)` 返回详情和原站入口。合集每 80 条一个 Koma 游标，并核对继续前一条的身份。 |
| `fetchReadableProducts` 和两个可见性设置 | `get_chapters` 分话、卷调用 `/api/viewer/pagination_readable_products`。目录用宿主返回的 `args.page.cursor` 和锚点续页，锁定与下架各自可隐藏，默认保留。付费或已下架的章节以 `🔒 ` 开头并返回 `locked:true`，下架项保留独立状态，用于 Koma 1.6.3 的列表呈现。 |
| `getPageList`、`ImageInterceptor` | `get_pages` 读取桌面 viewer 的 `script#episode-json`，检查系列、产品和匿名公开权限，锁定话返回 `chapter_locked`。保留原序和 RTL，滤掉非正文页。`get_image_request` 回查同一 viewer，对 `baku` 启用随包去扰，对普通封面返回 Referer。 |

[共享协议与阅读实现](../gigaviewer.rs)由 Jump+、邻座 Young Jump 和 Comic Days 使用。[4×4 处理器](../../image-processors/gigaviewer-tiles/src/lib.rs)保留尺寸和不满一块的右、下边缘。读取范围为匿名公开正文。
