# MAGCOMI

MAG Garden 官方漫画站，刊载《轮回的花瓣》《转生贵族的异世界冒险录》等连载，以及短篇和漫画奖入选作品。 版本 0.1.0，最低 Koma 1.5.5。栏目、筛选、设置与错误信息提供简中、繁中、英语、日语和韩语；作品正文保留原文。

## 移植前的源码对照

固定对照 [Keiyoushi magcomi](https://github.com/keiyoushi/extensions-source/tree/c09b174459c037c62292e4e7a929a0cd59430bc4/src/ja/magcomi) 与同版 [GigaViewer](https://github.com/keiyoushi/extensions-source/tree/c09b174459c037c62292e4e7a929a0cd59430bc4/lib-multisrc/gigaviewer)，实际读取其入口、请求、分页、设置和图像拦截链。2026-11-28 逐项访问本站 HTML、GraphQL、完整产品目录及桌面 viewer。宿主合同沿本仓 SDK 导出 → `request.args` → 开发宿主核对，未访问 Koma 仓库。

| 行为 | 上游／实站依据 | Koma 调用与消费 |
| --- | --- | --- |
| 热门、最新 | 固定 `seriesListIds` 合集去重，按 `likeCount`／`latestEpisode.publishedAt` 降序；缺少点赞保留站内顺序。 | `get_manga_list → listing → gigaviewer_browse::collection/page`；热门表示合集点赞排序，不声称官网有独立排行榜。 |
| 合集、筛选 | `getFilterOptions` 的 SerialGroup／Genre；实测逐项记录于验证文件。 | `get_filters → filters.collection`；热门、最新与空关键词搜索消费所选合集。每页最多 80 条，以 `args.page.cursor` 中的 offset 和上一作品身份核对续页；集合为空则正常结束。 |
| 搜索 | POST `/graphql` 的 `searchSeries(keyword)`；非空词优先于合集，与上游一致。 | `search → browse::search → Site::search`；非空词不分页，空词按所选合集分页。不把热门首屏包装成独立首页。 |
| 详情 | `series(databaseId)` 的标题、作者、简介、封面和首话／首卷链接。 | `get_manga → Site::detail`；作品为稳定系列 ID，格式标签 Manga。 |
| 章节 | `readableProducts` 总数与 `/api/viewer/pagination_readable_products` 倒序目录。话与卷独立分组。 | typed `get_chapters → Site::chapters`；跨页保留原身份与稳定阅读位置，核对总数和前项锚点，拒绝残缺目录；付费／下架分别保留设置和 locked 状态。 |
| 锁定、设置 | `can_read`／`unavailable`，隐藏付费话、隐藏下架话默认均为 false。 | `get_settings → browse::settings`；锁定项显示 🔒，阅读返回 `chapter_locked`。不获取账号或扩展站方权限。 |
| 阅读、图像还原 | 桌面 `script#episode-json` 的 `isPublic`、`pageStructure.choJuGiga=baku` 与 `type=main` 原序正文。 | `get_pages → Site::pages`；`get_image_request` 冷查同一 viewer，核实授权和页身份，协商 `source_required_image_decode_wasm_v1` 后返回 baku 像素阶段；缺能力返回 `host_feature_required`。 |
| 图片／封面 | 实站封面和正文主机见下文；请求保留原站 Referer。 | `get_image_request` 同时处理封面与正文，封面不附瓦片阶段；不读取响应头。 |

实测 `cover_hosts` 为 `cdn-img.magcomi.com`，另支持站方 `cdn-scissors.gigaviewer.com` 缩图；`page_host` 为 `cdn-img.magcomi.com`，正文仅接受 `/public/page/`。

直接引用 [gigaviewer.rs](../gigaviewer.rs)、[gigaviewer_browse.rs](../gigaviewer_browse.rs) 和既有 [gigaviewer-tiles](../../image-processors/gigaviewer-tiles)，crate 准确名称为 `koma_gigaviewer_tile_processor`（单数 tile）。metadata 声明 `requiredImageDecodes: ["wasm_image_v1"]`。没有新增处理器，也不更改已有源的语义。

MAGCOMI 默认合集是周二、周五连载；独立“连载中”栏目使用固定合集，不声明支持额外合集筛选，热门、最新及空词搜索仍支持所选合集。短篇包括读切、今日十页和七页连载预告，漫画奖包括奖项与选集；完结和休刊合并为一个合集。另有奇幻、科幻动作、男／女性向异世界等 16 个原站题材筛选。

系列身份、匿名公开权限、完整话／卷分组、默认隐藏开关和正文原序是本源行为边界；改变须有原站证据或用户明确决定。公共协议边界以共享实现注释和 [制作规范](../../docs/WRITING_A_SOURCE.md#移植前的源码对照) 为唯一依据。

## 验证与素材

最终可安装包内 WASM 的实站调用、完整章节、锁定映射与三张正文的包内处理器实际执行结果记录于 `dist/wave-11/verification.json`。输入图先完整解码 RGBA，再通过从 source.wasm 提取的处理器调用 `koma_image_process`；独立 baku 像素参考逐字节比对，核对尺寸、右／下残边及视觉连贯性。

图标原件和包内实物检查见 [ASSET_SOURCES.md](ASSET_SOURCES.md)。本任务禁止触碰 Koma，App 书源列表与真机普通阅读路径未验；主机实站验收不替代 App 验收。
