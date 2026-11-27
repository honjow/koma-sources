# Dynasty Scans

英文漫画阅读站，提供系列、章节、选集、杂志刊物和同人志。上游分级为 `MIXED`，原站公开目录含 `NSFW` 标签，订阅注册按 `nsfw:true` 标记。最低 Koma 版本为 0.1.0。栏目、筛选、设置及提示提供简体中文、繁体中文、英语、日语和韩语。图标来源见 [ASSET_SOURCES.md](ASSET_SOURCES.md)。

基线为 Keiyoushi [Dynasty.kt、Filters.kt、Dto.kt、构建分级及 assets](https://github.com/keiyoushi/extensions-source/tree/c09b174459c037c62292e4e7a929a0cd59430bc4/src/en/dynasty)。

| 上游 | Koma ABI 与源内实现 |
|---|---|
| `getPopularManga`、`getLatestUpdates` | `get_manga_list` 的 `popular` 取官网首页七日热门，`latest` 取 `/chapters/added.json` 并按公开关联标签聚合系列。附加 `chapters` 直接呈现近期发布章节。最新列表保留已有数据中的作者、题材和更新标题。 |
| 官网 Directory | `series`、`anthologies`、`issues`、`doujins` 各自访问原站同名路径，`rel=next` 转为 Koma `args.page.cursor`。静态封面表复用上游 assets，详情优先原站公开封面。 |
| `getSearchMangaList`、`fetchTagId` | `search` 使用 `/search` 和 `classes[]`，支持原站的五种类型、自动/相关度/标题/收录/发布日期，标签包含排除走 `with[]`、`without[]`。作者、翻译组、角色配对通过 `/tags/suggest` 精确匹配。标签三态按宿主特性启用，英文原站标签原样返回。站内作品或章节链接可直接搜索。 |
| `fetchMangaUpdate`、`parseChapterList` | `get_manga` 与 `get_chapters` 读取站点的 `/<type>/<slug>.json`。保留章节标题、卷分隔、日期和翻译组。系列按原目录倒序展示并提供阅读位置，选集和刊物沿用原站编排。同人志聚合常包含独立故事，目录保留原站最近发布顺序，逐条打开。 |
| `CHAPTER_FETCH_LIMITS`、`getPageList` | 默认取原站目录前 2 页，可以设置 5 页、10 页或全部。Koma 目录每批限制 80 条，站点长目录同时透传站点分页。独立话和正文都读取 `/chapters/<slug>.json`，依次发出原站已公开的图片 URL。 |

原站限速为每秒一次，随包向支持请求限速的 Koma 声明同样的窗口。Mihon 的相关作品入口未移植。旧版 Koma 使用基础筛选；支持三态标签与列表摘要的版本会显示对应功能。
