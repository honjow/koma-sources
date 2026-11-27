# AComics 俄语

源 ID `com.acomics.ru.koma`。俄语独立网漫平台，刊登原创连载和译作，设有编辑推荐与新人作品专区。

## 上游与宿主调用对照

对照 Keiyoushi 固定提交 [90b6e6a](https://github.com/keiyoushi/extensions-source/tree/90b6e6a905ca186945b21ec0b0be4ca9135b8e69/src/ru/acomics) 的 `AComics.kt` 和 `Filters.kt`。请求与响应遵循 [Koma 源运行时 ABI](https://github.com/honjow/Koma/blob/master/docs/source-runtime-abi.md)。

| 行为 | 上游及 Koma 路径 |
| --- | --- |
| 榜单、搜索和筛选 | `makeCatalogRequest` → `get_manga_list`、`search` → `browse_url`。榜单默认为订阅人数或更新时间。在主目录、新人作品和编辑推荐中支持题材、内容分级、作品类型、连载状态、订阅状态和最少章节数。已知筛选值按原站当前表单提供五语标签。关键词走 `/search?keyword=…`，至少三字，目录筛选只在无关键词时应用。根据 `a.infinite-scroll` 决定是否继续，每页使用原站的 10 条步长。 |
| 详情和章节 | `mangaDetailsParse`、`chapterListParse` → `get_manga`、`get_chapters`。使用 `/~slug/about` 解析作品资讯和官网发布期数，再按期数倒序输出。超过 400 章时由末章标识向前续页；阅读位置直接对应官网期数。 |
| 阅读 | `getPageList` → `get_pages` → `pages`，从 `/~slug/期数` 的阅读区取 `img.issue` 原图，封面和正文走同一个 `get_image_request`。 |
| 访问范围 | 保持匿名请求。默认分级与上游一致，涵盖 NR 至 R。上游附加的成人确认 Cookie 未带入，站点提示或无法匿名读取的作品继续遵守原站要求。没有登录、独立首页或源设置。订阅状态筛选按原站匿名上下文执行。 |
| Mihon | 俄语扩展的作品地址为 `/~slug/about`。当前源包的 Mihon 路径规则无法声明 `/~` 前缀，本包未声明导入映射。 |

匿名探测覆盖 `/comics`、`/~zpihr/about`、`/~zpihr/466`、目录封面和期数原图，均为 HTTP 200，且封面和正文通过图片解码。图标出处见 [ASSET_SOURCES.md](ASSET_SOURCES.md)。
