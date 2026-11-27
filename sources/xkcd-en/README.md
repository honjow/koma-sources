# xkcd 英文

源 ID `com.xkcd.en.koma`。收录兰德尔·门罗的英文原作，可按单本、年份或年月浏览。互动作品保留在目录，可在官网打开。

## 上游与宿主调用对照

对照 Keiyoushi 固定提交 `90b6e6a905ca186945b21ec0b0be4ca9135b8e69` 的 `Xkcd.kt` 英文分支。Koma 请求参数见[书源运行时 ABI](https://github.com/honjow/Koma/blob/master/docs/source-runtime-abi.md)。

| 行为 | 上游及 Koma 路径 |
| --- | --- |
| 组织与浏览 | `makeGroupedManga` → `get_manga_list`。默认 `SINGLE`，另有 `BY_YEAR` 和 `BY_YEAR_MONTH`。按原站日期分组，从新到旧，每次 10 组，按 `args.page.cursor` 续页。列表使用官方站点标志，分组详情优先取该组最近漫画图片，缺图回到站点标志。单本入口不需要预取归档。 |
| 归档、详情和目录 | `chapterListParse` → `get_chapters` → `archive`，读取 `/archive/` 的 `#middleContainer > a`、数字路径、作品名和发布日期，保持原站倒序。分组和章身份以请求 ID 为准，换组织方式后旧作品仍可打开。Mihon 的 `SINGLE`、年份、年月映射到 `xkcd:en:{group}`；站内数字章节路径对应 `xkcdch:en:{group}:{number}` 的数字段。 |
| 阅读 | `getPageList` → `get_pages` → `read_viewer`，访问官网数字路径并解析 `#comic > img`，优先用 `srcset` 首项，回退 `src`。缺少静态图片或图片后存在交互标记时返回 `interactive comic unsupported`。 |
| 文字页 | 中文 Koma 源只输出图片页。英文源用同一份官网 HTML 中的 `alt` 和 `title` 作为普通文本。宿主声明 `text_page_image_v1` 时，以 `webtoon-author-notes` 模板在原图后追加一页；无有效文案时只输出原图。两类宿主使用相同的第 1 页身份和 URL。最低 App 版本沿中文源的 `0.1.0`。 |
| 其他入口 | 上游仅有归档入口和组织设置，无搜索、最新、筛选或账号。沿 Koma 请求周期读取原站归档，未引入上游的应用内一小时缓存。 |

### 章节分页

宿主回传 `data.page.nextCursor` 到 `args.page.cursor`，直到 `hasMore: false` 才完成目录。英文源最多每次输出 1,000 章，并按 JSON 转义后的最坏字节数预留空间，以适应 1 MiB 宿主上限。游标是上一页最后一章的 ID，官网在归档首部插入新条目也不会使后续页跳过或重复已有章节。每页按该作品完整归档计算阅读位置，续页不重置章序。跨页拼接后的作品、章节和页 ID 继续用于原有续读与 Mihon 匹配。

栏目、组织设置及说明提供简中、繁中、英、日、韩文案；漫画标题和附注保留官网原文。以 `./build.sh --source xkcd-en` 构建。图标沿用作者官方账号头像，出处见 [ASSET_SOURCES.md](ASSET_SOURCES.md)。
