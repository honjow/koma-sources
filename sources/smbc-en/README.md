# Saturday Morning Breakfast Comics

源 ID `com.smbc.en.koma`。扎克·韦纳史密斯的英文短篇网漫，题材包括科学、哲学和日常生活。有些漫画另有红按钮彩蛋。

## 上游与宿主调用对照

对照 Keiyoushi 固定提交 [90b6e6a](https://github.com/keiyoushi/extensions-source/tree/90b6e6a905ca186945b21ec0b0be4ca9135b8e69/src/en/saturdaymorningbreakfastcomics) 的 `SaturdayMorningBreakfastComics.kt` 及本仓 `xkcd-zh`、`xkcd-en` 归档分组。请求与响应遵循 [Koma 源运行时 ABI](https://github.com/honjow/Koma/blob/master/docs/source-runtime-abi.md)。

| 行为 | 上游及 Koma 路径 |
| --- | --- |
| 栏目与组织方式 | 上游 `getPopularManga` 返回一本 `/comic/archive`。Koma 的 `get_manga_list` → `listing` 默认保留单本，并提供按年和按年月。后两种按原站日期划分，每次 10 组。各分组保留独立作品 ID 和进度，旧作品 ID 始终按自己的分组打开。没有搜索、最新榜、静态筛选或账号。 |
| 详情与归档 | 上游 `fetchMangaUpdate` → `get_manga`、`get_chapters`。封面使用官网标志，分组详情优先使用该组最近漫画图片。归档读取 `select[name=comic]` 的漫画选项，拆分首次出现的日期分隔符并保留完整作品标题。上游站点可能用 HTTP 500 返回完整归档或阅读 HTML，源只对这两类内容结构接受该响应。 |
| 章序和分页 | 各章的阅读位置是其在官网完整归档中的原始位置。对 `args.page.cursor` 回传的末章标识向更早的条目继续，直至 `hasMore: false`。每页最多 1,000 章并受 440 KiB 输出预算限制，预算含转义上限与响应附加字段。官网末尾追加新漫画不会移动既有阅读位置。 |
| 阅读 | 上游 `getPageList` → `get_pages` → `read_viewer`。依次输出 `#cc-comic`、协商成功时的 `title` 悬停提示、存在时的 `#aftercomic` 彩蛋图。无提示或彩蛋时跳过相应页，主图和彩蛋 ID 保持稳定。 |
| Mihon | 英文扩展 `3035799230093482569` 的 `/comic/archive` 映射到单本，章节用 `/comic/…` 的 slug。可选文字页会改变页序，因此未声明相同页序。 |

栏目和组织方式设置提供简中、繁中、英、日、韩文案。图标出处见 [ASSET_SOURCES.md](ASSET_SOURCES.md)。
