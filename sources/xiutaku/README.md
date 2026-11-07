# Xiutaku

书源 ID `com.xiutaku.koma`，多语言，标记为成人内容。参考 [Keiyoushi Xiutaku](https://github.com/keiyoushi/extensions-source/blob/b647c92fd14801fda23e866d5a7da7d5fdd7ec92/src/all/xiutaku/src/eu/kanade/tachiyomi/extension/all/xiutaku/Xiutaku.kt)，扩展 1.6 (versionCode 2)，上游源 ID `6520733892888572506`。

| 行为 | 参考与实现 |
| --- | --- |
| 热门与最新 | `getPopularManga`、`getLatestUpdates` 对应 Koma `get_manga_list`。热门走 `/hot`，最新走根路径，实际 Next 链决定 20 条步进。 |
| 搜索与直达 | `getSearchMangaList` 对应关键词分页；支持本站相册、热门及标签等列表 URL。 |
| 详情与目录 | `fetchMangaUpdate` 对应 `get_manga` 与 `get_chapters`。保留简介、标签和 UTC 发布日期。站点分页显示的末页为目录上限。从末页到首页列出章节，正文按页号读取。 |
| 正文 | `getPageList` 对应 `get_pages`，仅使用站点图集正文容器的原始顺序和图片地址，按本相册校验页号。 |

公开路径无需登录。参考客户端对主站统一限制每秒 10 次。当前宿主没有 Xiutaku 专用的限流规则，源每次操作只请求一页 HTML，不批量预拉。
