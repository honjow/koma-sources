# WEBTOON（印尼语）

源 ID `com.webtoons.id.koma`，语言 `id`。WEBTOON 印尼语分区，收录《The Boxer》等官方连载及 CANVAS 创作者漫画。阅读限原站匿名公开章节。

| 行为 | 固定上游 → 宿主 → 本源消费位置 |
| --- | --- |
| 热门与今日更新 | `getPopularManga` / `getLatestUpdates` → `get_manga_list` → `list`。四榜 trending/popular/originals/canvas 分页；最新使用只读本地 clock 得出的星期名，访问 `/id/originals/{weekday}?sortOrder=UPDATE`。仅在宿主协商 source_private_session_v1 时显示最新；没有账号会话读写。 |
| 筛选与搜索 | `Filters.kt`、原站题材抓取 → `get_filters` / `search` → `browse.rs`。独立现场配置 26 个题材组合，Originals 与 CANVAS 单方栏目有明确提示；星期 monday 到 sunday、complete，排序 MANA/LIKEIT/UPDATE。空词日期优先、再题材；非空词仅 Type 生效。CANVAS 按真实页码续页，协商空词筛选能力才返回 useSearchForFilters。 |
| 详情和身份 | `mangaFromElement` 的站内 list URL → `get_manga` → `detail` / `manga_parts`。新源作品 ID 使用 `manga:webtoons:id:` 加原站完整相对 list URL（含 title_no），浏览和 Mihon 导入生成相同 ID；这是新增语言的独立约定，绝不改变已验收西语及其他旧源。 |
| 目录 | `fetchChapterList` → `get_chapters` → `chapter_list`。移动站 episodes API 用 pageSize=99999，必须终止游标为0/null，不输出截断目录。CANVAS 使用 readingLanguageCode=id；仅允许带此参数的真实英语 canonical viewer，不能混入英语原章。 |
| 正文与图片 | `getPageList` → `get_pages` → `page_list`。只抽取 #_imageList 的正文图，保留原站随章警告页；get_image_request 返回原站 Referer。章 ID `chapter:webtoons:id:{viewerPath}`，页 ID `page:webtoons:id:`，无瓦片处理。 |
| 设置和编号 | 独立键 `useMaxQuality_id, useSequentialNumbering_id, showAuthorsNotes_id`，均默认关。最大画质只去压缩参数；明确的 Episode/EP/ตอนที่ 等标题数字用于章号，不能推断时保留空；顺序编号只在用户开启后使用完整目录顺序。作者后记只在文本页能力协商后追加。没有移植当前上游 Season/Bonus 名称推断。 |
| 评论 | 原站社区接口只读 chapter 评论，语言头 INDONESIAN，校验章归属，保留游标，排除盲评、删除、未揭示剧透。 |
| 访问停止条件 | 限匿名公开内容。401/403/付费/年龄或同意页停止，不自动提交同意、不用账号或挑战回放。 |

Mihon source ID `8749627068478740298` 使用固定上游 build.gradle.kts 的印尼语显式覆盖值：该值因历史显示名调整而保留，不能用普通 webtoons.com/id/1 哈希替代。work/chapter 在 metadata.identity.mihon 声明独立前缀和原站相对路径规则，普通浏览与导入身份一致。未声明未经完整顺序核对的 samePageOrder。

原站题材与星期路由快照见 evidence/preflight.json；印尼语 Originals 题材为：drama, fantasy, romantic_fantasy, comedy, action, slice_of_life, romance, thriller, horror, web_novel, local。

## 上游与 Koma 宿主调用链

固定上游 [Keiyoushi c09b174459c037c62292e4e7a929a0cd59430bc4](https://github.com/keiyoushi/extensions-source/tree/c09b174459c037c62292e4e7a929a0cd59430bc4/src/all)，实际对照平台 .kt、Filters.kt、build.gradle.kts 和 ExtensionPlugin.kt。宿主合同沿仓内 `sdk/src/runtime.rs`、`sdk/src/source_protocol.rs`、`sdk/src/chapters.rs` 与 `tools/koma-source-dev/src/host.rs` 只读复核；源导出入口接受版本化 args、settings、hostFeatures、environment，游标从 args.page.cursor 读取。所有既有源及 tools 保持不变，不创建分支或工作树。

五语（简中、繁中、英、日、韩）覆盖名称、简介、栏目、筛选、设置、错误和作者后记提示；作品和站方动态标签保留站点内容。固定 source_info 基准简介等于 metadata 的英文 source.description。无独立首页；没有把热门首屏声明为首页。没有读取 HTTP 响应头；不假设响应头只能是字符串。

版本 0.1.0，最低 Koma 1.5.5，safe。候选包、原始联网请求与响应、图像 HTTP/MIME/像素尺寸/SHA-256 及五语结果在 dist/wave-16/verification.json 和 evidence 中。图标见 [ASSET_SOURCES.md](ASSET_SOURCES.md)。本轮不操作设备，不发布；App 书源列表图标呈现等待后续真机 QA。
