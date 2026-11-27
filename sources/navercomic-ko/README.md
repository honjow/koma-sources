# Naver Webtoon（韩国）

Naver 韩国官方条漫平台，既有按星期更新的正式连载，也有创作者投稿的「最佳挑战」与「挑战漫画」。主页为 <https://comic.naver.com>。源 ID 为 `com.naver.comic.ko.koma`，最低 Koma 版本声明为 1.4.1。

## 上游与 Koma 调用对照

固定 Keiyoushi `extensions-source` 提交 `90b6e6a905ca186945b21ec0b0be4ca9135b8e69`，目录 `src/ko/navercomic/`。参考 `NaverComic.kt`、`Dto.kt` 和该目录的 Gradle 入口。宿主通过 BrowseViewModel、SourceMangaDetailRequest、SourceChapterListResolver、SourceChapterPageHydrator 和 ReaderPageSourceAdapter 调用下列操作。

| 上游行为 | Koma 消费与实现 |
|---|---|
| 正式连载的热门和最新使用移动版星期页与 `ALL_READER`、`UPDATE`，按 `.list_toon > [class='item ']` 读取，无下一页 | `get_listings` → `get_manga_list` → `weekday`，按原顺序去重；封面提供公开 Referer |
| 两个 Challenge 来源各有 `VIEW` 和 `UPDATE`，榜单 API 以 `pageInfo.nextPage` 翻页，缺页标记时按本页是否非空判断 | `best_challenge` 与 `challenge` 栏目保留 `VIEW` 榜单，`args.page.cursor` 传回上游下一页数字；保留三种带 mType 的作品身份 |
| Kotlin 按扩展 mType 提交站内搜索；DTO 返回 `searchList` 与下一页 | `search` 调用 `/api/search/webtoon`，以 UTF-8 编码关键词，使用下一页数字；本候选的关键词搜索覆盖正式连载 |
| 详情与完整目录用共享 article API，作品与章节公开链接按 mType 区分 | `get_manga` 用 `titleId` 获取详情，保留作者、简介、封面和状态。 `get_chapters` 逐页消费 `nextPage`，完整保留原始倒序并从真实话号生成稳定 ID |
| 正文先读取 `.wt_viewer img`，然后回退 `.toon_view_lst img.toon_image` 的 `data-src` 或 `src` | `get_pages` 由章节 ID 恢复 mType、作品和话号；解析相对与协议相对地址。 `get_image_request` 返回 Naver Referer 与移动浏览器 UA |

正式连载的 Mihon source ID 为 `1311262507446028482`。宿主每个包只接受一套作品路径规则，本包直接匹配普通 `/webtoon/list?titleId=<id>` 与对应章节路径。早期带 `week` 的链接、绝对移动地址、两个 Challenge 的备份及页级恢复尚待另外验证。两个 Challenge 的 Mihon ID 分别为 `3983114856627782141`、`4487676425355815867`。

相对时间的章节日期留空，明确的 `yy.M.d` 按首尔时区转换。原站的付费或年龄授权仍由原站决定。候选只声明公开浏览和阅读，同站的独立 Challenge 最新榜及关键词搜索不在本批功能范围。联网需分别验证三种详情/阅读路由、分页结束、两种正文 DOM 和 CDN 实际解码；还要检查真机书源列表的明暗图标。

## 本地验证

从最终包解出 WASM 回放了实站星期页和搜索样本。另用构造响应核对两种 Challenge 栏目的详情和阅读路由、跨页终止与重复条目处理、章序、正文选择器回退及图片头。
