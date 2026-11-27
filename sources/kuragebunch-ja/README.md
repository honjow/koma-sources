# くらげバンチ

新潮社官方漫画站，刊载《极主夫道》《働かないふたり》等作品，并设有短篇及 KANATA 专栏。 版本 0.1.0，最低 Koma 1.5.5。栏目、分类、设置和错误信息提供完整五语本地化。匿名阅读范围遵循原站，付费和下架话仍受限制。图标见 [ASSET_SOURCES.md](ASSET_SOURCES.md)。

## 移植前对照与行为边界

固定上游 [KurageBunch.kt](https://github.com/keiyoushi/extensions-source/tree/c09b174459c037c62292e4e7a929a0cd59430bc4/src/ja/kuragebunch) 与同版 [GigaViewer](https://github.com/keiyoushi/extensions-source/tree/c09b174459c037c62292e4e7a929a0cd59430bc4/lib-multisrc/gigaviewer)。2026-11-28 对照官网目录、GraphQL 节点及桌面 viewer；沿现有 SDK 导出 → request.args → 共享实现复核，宿主调用合同以本仓 docs/WRITING_A_SOURCE.md、SDK 与开发宿主为依据，本次未访问 Koma 仓库。

| 行为 | 上游／原站 | Koma 调用与消费 |
| --- | --- | --- |
| 热门、最新 | seriesListIds 的 SerialGroup；按 likeCount 或 latestEpisode.publishedAt 排序，未提供点赞时保留站内顺序。 | get_manga_list → listing → gigaviewer_browse::collection；按作品 ID 去重。热门表示合集点赞排序，不冒充官网独立首页或独立排行榜。 |
| 分类、短篇 | getFilterOptions 的 SerialGroup／Genre 节点，本次逐项访问实测。 | get_filters → filters；listing 和空关键词 search 消费 filters.collection。合集每 80 条返回游标，以 args.page.cursor 续页并核对前项身份。 |
| 关键词搜索 | POST /graphql，searchSeries(keyword)。非空词忽略合集筛选，与上游保持一致。 | search → gigaviewer_browse::search；非空词一批返回，无继续游标；空词查所选合集。 |
| 作品详情 | series(databaseId) 的标题、作者、简介、封面及首话／首卷链接。 | get_manga → gigaviewer::Site::detail；保留系列身份，格式标签 Manga。 |
| 完整章节 | readableProducts 的话／卷总数，GET /api/viewer/pagination_readable_products，以原站倒序 offset 续页。 | typed get_chapters → Site::chapters；话、卷分组，校验锚点和总数，不把残缺目录当成功。锁定／下架保留 locked:true、🔒 标识；隐藏开关各自默认关闭。 |
| 阅读页和图片 | 桌面 script#episode-json，isPublic、pageStructure.choJuGiga=baku，正文 main 页。 | get_pages → Site::pages；锁定返回 chapter_locked。get_image_request 冷查询同一 viewer 核实授权及页身份，协商 source_required_image_decode_wasm_v1 后返回 baku 像素阶段；无能力返回 host_feature_required。 |
| 登录、设置 | 上游仅匿名目录与可读内容；两项隐藏设置。 | get_settings → settings；不获取账号、不扩展原站权限，无独立首页。 |

实测封面来自 `cdn-img.kuragebunch.com` 或 `cdn-scissors.gigaviewer.com` 的缩图，正文来自 `cdn-img.kuragebunch.com/public/page/`；官网静态标识使用官网公布的资源地址，不作为正文页。所有图片保留原站 Referer。

直接引用 [gigaviewer.rs](../gigaviewer.rs)、[gigaviewer_browse.rs](../gigaviewer_browse.rs) 与现有 [gigaviewer-tiles](../../image-processors/gigaviewer-tiles)。现有 crate 的准确名称是 `koma_gigaviewer_tile_processor`（单数 tile）；metadata 沿用该名称，未新建或改名处理器，也不修改已有锁文件条目。还原保持原尺寸和不足一块的右／下边缘。

实测《働かないふたり》第一回的 readingDirection 为 `ttb`，共享解析同时保留 `rtl/ltr/ttb`；正文原序不变，不把纵向作品伪装成横向。该兼容分支有最终 WASM 的协议边界回归。

身份、默认可见性、公开权限、正文原序和合集筛选只对空词生效是保留边界；更改需要实站证据或明确用户决定。

## 验证

包内最终 WASM 的在线调用、完整章节与三张正文下载／瓦片还原结果写入 `dist/wave-10/verification.json`。本次不操作 Koma，App 书源列表及真机普通阅读路径保留待验，不能由宿主模拟验收替代。

离线协议回归（Python 需要 `wasmtime`、`beautifulsoup4`）：

```bash
python3 sources/kuragebunch-ja/tests/test_viewer.py --package dist/wave-10/sources/kuragebunch-ja/kuragebunch-ja-0.1.0.koma
```
