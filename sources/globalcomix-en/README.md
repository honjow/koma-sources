# GlobalComix 英文

源 ID `com.globalcomix.en.koma`，面向出版社和独立创作者的英文漫画，包括 DC 的《Absolute Batman》。源级分级沿用 GlobalComix 中文源的 `nsfw`，正文只接受原站明确开放的免费页。

## 上游与宿主调用对照

对照 Keiyoushi 固定提交 `90b6e6a905ca186945b21ec0b0be4ca9135b8e69` 的 `GlobalComix.kt`、`GlobalComixConstants.kt` 和 DTO。Koma 请求参数见[书源运行时 ABI](https://github.com/honjow/Koma/blob/master/docs/source-runtime-abi.md)。

| 行为 | 上游及 Koma 路径 |
| --- | --- |
| 热门、最新、搜索 | `simpleQuery` → `get_manga_list` / `search` → `list`。使用 `lang_id[]=en`，热门沿站点默认排序，最新用 `recent`，关键词用 `relevance`。消费 `args.page.cursor`，按服务端页数停止。 |
| 详情和作品身份 | 上游列表保存站点数字 ID，详情用 `/read/{slug}`。`get_manga` → `detail` 用官方 `/comics/{id}` 直查同一实体，验证作品 ID 和唯一英文语言记录，保留站点声明的同作品链接。Mihon 数字 ID 直接映射为 `gc:en:{id}`，避免标题和 slug 变化影响关联。 |
| 目录及评论 | `fetchChapterList` → `get_chapters` → `chapters`，使用 `/comics/{id}/releases?lang_id=en&all=true`。目录保留站点顺序及原续读分段校验。Mihon 的 release UUID 对应 `gcch:en:{comic}:{release}:{uuid}` 中的 UUID。原站章节评论走 `get_comments` → `/v1/releases/{id}/comments`。 |
| 正文 | `getPageList` → `get_pages` → `pages`，使用 `/readV2/{uuid}`。校验 release 和每页的归属、英文语言、免费状态和 CDN；`gcp:en:` 下的页 ID 不随画质改变。 |
| 设置 | `dataSaver_en` 默认关闭，启用后同页使用 mobile URL。协商 `source_chapter_exclusions_v1` 后提供默认开启的 `showLockedChapters_en`，仅控制目录可见性。旧宿主只显示免费目录。 |
| 筛选、首页、账号 | 上游无独立筛选或首页。沿既有 Koma 源的公开 API 路径，无账号设置或付费阅读。 |

栏目、设置及源简介提供简中、繁中、英、日、韩文案。旧宿主使用英文基准。以 `./build.sh --source globalcomix-en` 构建。素材与归属见 [ASSET_SOURCES.md](ASSET_SOURCES.md) 和 [NOTICE.md](NOTICE.md)。
