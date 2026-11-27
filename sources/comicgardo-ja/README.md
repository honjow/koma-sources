# Comic Gardo / コミックガルド

Overlap 官方漫画站，收录《平凡职业造就世界最强》等轻小说改编漫画，以及选集和完结作品。 版本 0.1.0，最低 Koma 1.5.5。栏目、筛选、设置和错误提供简中、繁中、英语、日语和韩语；作品原文不翻译。

## 移植前的源码对照

固定对照 [Keiyoushi comicgardo](https://github.com/keiyoushi/extensions-source/tree/c09b174459c037c62292e4e7a929a0cd59430bc4/src/ja/comicgardo) 与同版 [GigaViewer](https://github.com/keiyoushi/extensions-source/tree/c09b174459c037c62292e4e7a929a0cd59430bc4/lib-multisrc/gigaviewer) 的 Kotlin 入口、GraphQL、目录续页、设置及 ImageInterceptor。2026-11-28 实际核对本站 HTML、每个合集节点、关键词搜索、产品目录、桌面 viewer、封面与正文图片。宿主调用链以本仓 SDK、`tools/koma-source-dev`、[制作规范](../../docs/WRITING_A_SOURCE.md#移植前的源码对照) 为依据，未访问 Koma 仓库。

| 行为 | 上游及当前官网 | Koma 调用与消费 |
| --- | --- | --- |
| 热门、最新 | `seriesListIds` 并集去重，按 `likeCount` 或 `latestEpisode.publishedAt` 排序。 | `get_manga_list → listing → browse::collection/page`。热门是合集点赞排序，非官网每日访问量排行；没有点赞时保留站内顺序。 |
| 合集、筛选 | `getFilterOptions` 的 SerialGroup／Genre，逐项请求实际节点及标题。 | `get_filters → filters.collection`；热门、最新和空词搜索消费所选合集。固定栏目不声明支持合集筛选。每页最多 80 部，续页读取 `args.page.cursor` 的 offset 与上一部作品 ID，站方排序变动时报 `catalog_changed`。 |
| 搜索 | POST `/graphql` 的 `searchSeries(keyword)`；非空词不应用合集筛选。 | `search → browse::search → Site::search`；非空词单批返回，空词按所选合集分页。不把热门首屏包装为独立首页。 |
| 详情与身份 | `series(databaseId)` 的标题、简介、作者、封面与首话／首卷。 | `get_manga → Site::detail`；保留系列 ID，作品格式为 Manga，封面使用站方真实 URL。 |
| 完整话／卷目录 | `readableProducts` 总数和 `/api/viewer/pagination_readable_products` 倒序 offset 续页。 | typed `get_chapters`；话和卷独立阅读分组，跨页保持原序及阅读位置，核对总数和锚点，拒绝重复 ID 与残缺目录。 |
| 付费、下架与设置 | 上游分别提供“隐藏付费话”“隐藏下架话”，默认均 false。 | `get_settings → browse::settings`；保留 `locked`、🔒 与手动阅读边界，协商 exclusions 后只按对应设置排除。源不接登录或绕过原站权限。 |
| 阅读方向与正文 | 桌面 `script#episode-json` 的 `isPublic`、`pageStructure`、`type=main`。 | `get_pages → Site::pages`；原样保留 provider 的 rtl／ltr／ttb，不改变用户阅读设置，锁定返回 `chapter_locked`。本源代表样例实测为 rtl、baku。 |
| 图片还原与防盗链 | 同站 `cdn-img` 正文，以及站方 `cdn-scissors.gigaviewer.com` 封面派生图。 | `get_image_request → Site::image_request` 冷查 viewer，核对授权、当前页 ID 和 URL；附章节页 Referer。baku 仅在协商 `source_required_image_decode_wasm_v1` 后返回随包像素阶段，否则 `host_feature_required`；封面不附瓦片阶段。 |

实测封面原件和正文主机为 `cdn-img.comic-gardo.com`；正文仅接受 `/public/page/`。直接复用 [gigaviewer.rs](../gigaviewer.rs)、[gigaviewer_browse.rs](../gigaviewer_browse.rs) 和既有 `koma_gigaviewer_tile_processor`（单数 tile）。metadata 声明 `requiredImageDecodes: ["wasm_image_v1"]`，未新建处理器，未读取网络响应头。

默认连载中合集 `10834108156664921090`；选集 `2551460909910482483`；完结 `10834108156664921091`。上游把第二项称为「アンソロジー・読切」，当前实际节点名为「アンソロジー作品」，因此五语显示据实为“选集”，没有伪造单独读切目录。官网另有“本日のランキング”，与上游点赞排序口径不同，本源不冒充该每日榜单。

稳定身份、匿名权限、完整话／卷、默认隐藏设置、原站方向及正文原序是本源行为边界；更改需要真实原站证据或用户明确决定。公共协议以共享实现旁注释和制作规范为依据。

## 验证与素材

最终 .koma 提取的 source.wasm 完成五语导出、全部合集／栏目、搜索、详情、完整目录、锁定话、阅读页和真实图片联调。三张 baku 正文先解码 RGBA，再实际调用从包内 WASM 提取的处理器；核对原尺寸、残边、独立像素参考及目视连贯性。结果、请求和图片 SHA-256 记录于 `dist/wave-12/verification.json`。

图标来源与原件／处理稿／包内实物核对见 [ASSET_SOURCES.md](ASSET_SOURCES.md)。本次未访问 Koma 或操作真机，App 书源列表与普通阅读路径未验；主机联调不替代设备验收。
