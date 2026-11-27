# Ichijin Plus / 一迅プラス

一迅社官方漫画站，汇集 Comic REX、comic HOWL、comic POOL、Comic 百合姬等编辑部作品，按厂牌和题材分类。阅读范围以原站公开话为准。

`com.ichicomi.ja.koma`，日文，版本 0.1.0，最低 Koma 1.5.5。栏目、筛选、设置和错误提供简中、繁中、英语、日语和韩语；作品标题、作者及正文保留原文。

## 移植前的源码对照

固定对照 [Keiyoushi ichicomi](https://github.com/keiyoushi/extensions-source/tree/c09b174459c037c62292e4e7a929a0cd59430bc4/src/ja/ichicomi) 的 Kotlin 入口与 Gradle 声明，以及同版 [GigaViewer](https://github.com/keiyoushi/extensions-source/tree/c09b174459c037c62292e4e7a929a0cd59430bc4/lib-multisrc/gigaviewer) 的 `GigaViewer.kt`、`Dto.kt`、`Queries.kt` 和 `ImageInterceptor.kt`。实现前实际读取首页、`/series`、所有固定合集 GraphQL 节点、作品目录与 viewer。宿主调用形状以本仓 `SKILL.md`、`sdk/src` 以及 `tools/koma-source-dev/src/host.rs::run_operation` 为依据；没有访问 Koma 仓库或设备。

| 行为 | 上游实现与默认值 | Koma 调用及消费 |
| --- | --- | --- |
| 热门、最新 | `getPopularManga/getLatestUpdates → fetchSeriesList` 合集并集去重，按点赞／最新话日期排序。无点赞时原序。 | `args.listingId → listing → browse::collection`。不是官网按日访问量榜单；没有独立首页。 |
| 合集、筛选 | `getFilterOptions` 固定 SerialGroup / Genre；仅空词搜索消费上游分类。 | `get_filters → browse::filters` 暴露 34 项；空词 `search` 消费 `args.filters.collection`。热门／最新也支持该筛选；固定专栏只读对应原站合集，不声明筛选。 |
| 分页 | 上游先完整读取合集；搜索关键词结果单批返回。 | 合集 `browse::page` 每批 80 部，读取 `args.page.cursor` 的 offset 和上一部 ID，变化报 `catalog_changed`；终点 `hasMore=false`。原站完整合集没有因分页被截断。 |
| 关键词 | POST `/graphql` 的 `searchSeries(keyword)`；非空词忽略分类。 | `search → browse::search → Site::search`，消费 `args.query`；非空词不接受续页 cursor，避免重复。 |
| 详情与身份 | `series(databaseId)`，首话或首卷链接、简介、作者与封面。 | `args.mangaId → Site::detail`；作品 `ichicomi:<seriesId>`，话 `ichicomi-ch:<seriesId>:episode:<productId>`；卷使用 `volume`。 |
| 完整目录 | `readableProducts.totalCount`，`/api/viewer/pagination_readable_products?type=episode/volume&aggregate_id=…&sort_order=desc&offset=…` 直到完整。 | typed `get_chapters → Site::chapters`，按总数和页锚点检查重复、残缺与变化；话／卷独立分组，保持提供方顺序、阅读位置和稳定 ID。 |
| 付费、下架 | 默认 `purchase_info.can_read/unavailable`；上游本站没有覆盖 `isUnavailable()`。 | 使用共享默认 `chapters`，**不启用 Zenon 缺失 status 的专属策略**。保留 `locked`、锁标与下架提示；viewer 再核验 `readableProduct.isPublic`，未公开时 `chapter_locked`，不读取付费预览代替正文。 |
| 隐藏设置 | `hide_locked` / `hide_unavailable` 默认 false。 | `get_settings → browse::settings`，协商 `source_chapter_exclusions_v1` 后保留手动阅读边界；不改变原站授权。 |
| 阅读与还原 | `script#episode-json[data-value]` 中仅 `type=main`；`choJuGiga=baku` 执行 4×4 瓦片转置，边缘余数保留。 | `get_pages → Site::pages`；消费原站 `readingDirection`。`get_image_request` 对正文声明 `wasm_image_v1`，实际处理器来自包内 `koma_gigaviewer_tile_processor`；无宿主能力时报错。 |
| 图片、请求 | 正文 `cdn-img.ichicomi.com`，封面兼容同站 CDN 和 `cdn-scissors.gigaviewer.com`。 | 页图严格验证本站 `/public/page/`；封面不套瓦片还原；图片附原站 Referer。不读取 HTTP 响应头，没有 String/Array 差异路径。 |
| 登录与设置 | 此上游没有账号／镜像设置。 | 不声明 credentials，不引入登录或绕过付费。 |

当前官网已有 LOVEBITES，而固定上游未列出该编辑部。补充官网 `/series/love-bites` 所列作品的 `serialGroups` 实查得到的连载中 `2550912965796689223` 与完结 `2550912965796689234`，其余上游栏目均保留；echo、ぱれっと等归档栏目即使没有连载也不删除。品牌名保留官方写法。

共享 `sources/gigaviewer.rs`、`sources/gigaviewer_browse.rs` 和处理器均保持字节不变。此次只加入新源的站点配置，不改变已有源的身份、默认值或运行行为。

## 验证与边界

候选包：`dist/wave-13/sources/ichicomi-ja/ichicomi-ja-0.1.0.koma`。以包内 `source.wasm` 通过 `koma-source-dev` 验证五语栏目／筛选／设置／错误、全部合集及列表分页、关键词、完整目录、公开话与锁定话、封面和多张正文图；包内嵌入的处理器执行真实 RGBA 瓦片还原，并与独立像素参考核对。逐请求记录、实际尺寸和 SHA-256 写入 `dist/wave-13/verification.json`。图标原件与工艺见 [ASSET_SOURCES.md](ASSET_SOURCES.md)。

此轮不操作 Koma App；主机实网验证不等同真机书源列表与普通阅读路径验收。公开范围与目录会随原站更新变化。
