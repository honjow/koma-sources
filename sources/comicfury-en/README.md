# Comic Fury 英文

源 ID `com.comicfury.en.koma`。收录创作者自行发布的英文网漫。部分作品提供原站作者附言。中文与英文入口使用独立的包、列表语言参数和 Mihon 来源编号。

## 上游与宿主调用对照

对照 Keiyoushi 固定提交 [90b6e6a](https://github.com/keiyoushi/extensions-source/tree/90b6e6a905ca186945b21ec0b0be4ca9135b8e69/src/all/comicfury) 的 `ComicFury.kt`、`Filters.kt`、`ContentWarningInterceptor.kt`，以及本仓 `comicfury-zh` 的正文归属校验和文字页处理。请求与响应遵循 [Koma 源运行时 ABI](https://github.com/honjow/Koma/blob/master/docs/source-runtime-abi.md)。

| 行为 | 上游及 Koma 路径 |
| --- | --- |
| 榜单与搜索 | `getPopularManga`、`getLatestUpdates`、`getSearchMangaList` → `get_manga_list`、`search` → `browse`。固定 `language=en`，搜索、热门、最新的排序分别默认为 0、1、2。标签、完结、更新时间和四项内容尺度都按上游查询参数传递。内容尺度沿英文上游默认为 2，可通过筛选收紧。根据站点的 `div.search-next-page` 决定是否返回下一游标。 |
| 详情与目录 | `getMangaDetails`、`getChapterList` → `get_manga`、`get_chapters` → `detail`、`archive`。保留 `/comicprofile.php?url=…`，按作品的归档分组和网页分页抓取正文条目，反向输出并按原归档顺序计算阅读位置。以末章身份续页。内置归档无条目时尝试同一作品的 `webcomic.ws/archive/comics`。 |
| 正文 | `getPageList` → `get_pages` → `pages`。统一布局读取与章节 ID 一致的第一个漫画块，只取该块内的原图。自定义布局取 `img#comicimage`。正文与封面由 `get_image_request` 返回公开图像请求。 |
| 设置 | 上游只有 `showAuthorsNotes`，默认关闭。宿主声明 `text_page_image_v1` 时展示设置，启用后保留原图顺序并追加 `webtoon-author-notes` 文字页。作品正文和作者附言使用原文。 |
| 访问提示 | 上游会自动提交原站内容警告表单。本源遇到原站确认页时返回受限状态，读者可打开原网页查看提示。没有账号登录或额外的浏览模式设置。 |
| Mihon | 英文扩展 `6251704791239468278` 映射到同一作品档案和标准 `/read/…/comics/…` 章节路径。不同版式及作者文字页可能改动分页，因此未声明相同页序。 |

栏目、设置和静态筛选提供简中、繁中、英、日、韩文案。图标出处见 [ASSET_SOURCES.md](ASSET_SOURCES.md)。
