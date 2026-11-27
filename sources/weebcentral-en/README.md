# Weeb Central

英文日漫、韩漫和华语漫画，包括《王者天下》等长篇连载。图标记录见 [ASSET_SOURCES.md](ASSET_SOURCES.md)。

移植基线为 [Keiyoushi Weeb Central @ 90b6e6a](https://github.com/keiyoushi/extensions-source/tree/90b6e6a905ca186945b21ec0b0be4ca9135b8e69/src/en/weebcentral)。

| 上游行为 | Koma 操作和源内入口 |
|---|---|
| `getPopularManga`、`getLatestUpdates`、`getSearchMangaList` 使用 `search/data` | `get_manga_list` 和 `search` 共用 `listing`。沿用 32 本一批和上游搜索标点清理。用 HTMX 按钮的真实 offset 产生 `data.page.nextCursor`，下一次读取 `args.page.cursor`。 |
| `Filters.kt` 和 `setupPreferenceScreen` | 保留排序、方向、官方译本、动画、成人、作者、状态、类型、题材的请求字段。状态和类型多选，题材以 Koma 的 `id` / `!id` 映射重复的 included/excluded 参数。简化标题默认关闭。 |
| `getMangaDetails` | `get_manga` 取 canonical、标题、封面、作者、题材、类型、状态、别名、简介、相关作品和追踪链接。关系和站点正文保留原文。 |
| `getChapterList` | `get_chapters` 从全目录保持原站新到旧顺序并用 ULID 区分章节。输出压缩在 960 KiB 缓冲内，超预算时用上一条稳定章 ID 继续。阅读位置 按目录位置计算。 |
| `getPageList` 和 `getImageRequest` | `get_pages` 使用 `reading_style=long_strip` 及原始页序。图片请求保留原始 URI，图片类型由实际字节解码。目录文档采用普通桌面 UA，站内频率按上游 1 次 / 2 秒。 |

Koma 调用分别来自发现页、详情目录 resolver、页表 hydrator 和图片描述解析器。相关作品和追踪站点在详情简介内链接，上游独立推荐列表及账号交互未纳入本源。
