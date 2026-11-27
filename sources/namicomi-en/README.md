# NamiComi 英文

源 ID `com.namicomi.en.koma`。收录创作者发表的原创漫画英文版章节，正文限原站匿名可读内容。

## 上游与宿主调用对照

对照 Keiyoushi 固定提交 `90b6e6a905ca186945b21ec0b0be4ca9135b8e69` 的 `NamiComi.kt`、`Filters.kt` 和 DTO。Koma 请求参数见[书源运行时 ABI](https://github.com/honjow/Koma/blob/master/docs/source-runtime-abi.md)。

| 行为 | 上游及 Koma 路径 |
| --- | --- |
| 热门、最新、搜索 | `getPopularManga` / `getLatestUpdates` / `getSearchMangaList` → `get_manga_list` / `search` → `list`。`/title/search` 使用英文 `availableTranslatedLanguages[]`，按每页 20 项的游标和站点 `total` 翻页。热门默认按浏览量倒序，最新及搜索默认按发布日期倒序。沿已采用参考分开保留浏览与非空关键词搜索的可读性默认值。 |
| 筛选 | `getFilterList` / `fetchFilterData` → `get_filters` → `filter_schema`。`apply_filters` 保留可读性、分级、连载状态、排序、标签 ID 和 and/or。协商动态筛选与三态能力，并收到 `fetchFilterData` 时从 `/title/tags` 取得英文站点选项。栏目、设置、筛选分组和静态选项按界面语言翻译。 |
| 详情和身份 | `fetchMangaUpdate` → `get_manga` → `detail`，从 `/title/{id}` 校验作品归属；英文标题优先，缺失时使用站点提供的首个标题。Mihon 裸作品 ID 映射为 `namicomi:en:{id}`，裸章 ID 与 `namicomich:en:{title}:{chapter}` 的末段匹配。 |
| 目录、正文、评论 | `getChapterList` → `get_chapters` 按完整卷／章双降序分页，使用 `translatedLanguages[]=en` 并校验每章 `translatedLanguage`。匿名 gating 决定可见性和阅读权限。页表先复核 gating，再消费 `/images/chapter/{id}?newQualities=true` 和指定 CDN。`get_comments` 读取原站同章评论并保留分页、归属及剧透过滤。 |
| 设置 | `thumbnailQuality_en` 默认 `original`，另可选 `.512.jpg` 和 `.256.jpg`。`dataSaver_en` 默认关闭，启用后用站点低画质数组，`namicomipage:en:` 页 ID 保持一致。宿主协商目录排除能力后可打开默认关闭的 `showLockedChapters_en`。 |
| 账号与首页 | 沿 Koma 中文源的匿名访问边界。上游凭据回退与独立首页均不在此源范围。 |

简中、繁中、英、日、韩文案随包提供，旧宿主使用英文基准。以 `./build.sh --source namicomi-en` 构建。图标与归属见 [ASSET_SOURCES.md](ASSET_SOURCES.md) 和 [NOTICE](NOTICE)。
