# Kiutaku

书源 ID `com.kiutaku.koma`，多语言，标记为成人内容。参考 [Keiyoushi Kiutaku](https://github.com/keiyoushi/extensions-source/blob/b647c92fd14801fda23e866d5a7da7d5fdd7ec92/src/all/kiutaku/src/eu/kanade/tachiyomi/extension/all/kiutaku/Kiutaku.kt)，扩展 1.6 (versionCode 1)，上游源 ID `3040035304874076216`。

| 行为 | 参考与实现 |
| --- | --- |
| 热门与最新 | `getPopularManga`、`getLatestUpdates` 对应 Koma `get_manga_list`。热门走 `/hot`，最新走根路径，实际 Next 链决定 20 条步进。 |
| 搜索与直达 | `getSearchMangaList` 对应关键词分页；本站相册 URL 直达相册。 |
| 详情与目录 | `fetchMangaUpdate` 对应 `get_manga` 与 `get_chapters`。保留标签和网站实际列出的页链接，原参考不提供发布日期。从末页到首页列出章节，正文按页号读取。 |
| 相关图集 | `fetchRelatedMangaList` 对应 `get_manga` 的 `manga.relatedManga`，在详情 HTML 中按原顺序读取 `.bottom-articles .items-row`。声明 `source_related_manga_v1` 的宿主通过详情菜单展示，同源作品走普通详情入口。 |
| 正文 | `getPageList` 对应 `get_pages`，仅使用站点图集正文容器的原始顺序和图片地址，按本相册校验页号。 |

公开路径无需登录。没有声明相关图集能力的旧宿主仍可正常浏览和阅读。图片请求和封面均按参考省略 Referer。
