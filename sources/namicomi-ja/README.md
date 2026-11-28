# NamiComi（日语）

源 ID `com.namicomi.ja.koma`，语言 `ja`。NamiComi 日语分区，收录《異世界令嬢が恋に落ちたのは元婚約者のおじい様でした》等创作者与发行方发布的日语章节。仅提供原站匿名可读内容。

| 行为 | 固定上游 → 宿主 → 本源消费位置 |
| --- | --- |
| 语言 | 固定上游 lang=ja，extLang 不触发特殊映射而保持 ja；SOURCE_INFO 与 typed Chapter.language 均为 ja。API availableTranslatedLanguages[]=ja / translatedLanguages[]=ja。 |
| 列表、搜索和分页 | getPopularManga/getLatestUpdates/getSearchMangaList → get_manga_list/search → list。热门浏览量降序，最新及关键词默认发布时间降序，20项一页，args.page.cursor 保留offset，核对total与每页实际数量。浏览可读性默认all，非空搜索默认available，两个作用域独立。 |
| 筛选 | Filters.kt → get_filters → filter_schema/apply_filters。可读性、分级、状态、排序、包含/排除标签与and/or；fetchFilterData 加动态及三态能力协商后读 /title/tags。 |
| 详情 | get_manga → detail → /title/{id}，标题和简介 ja → en → 首个站方值，公开链接 /ja/title/。这是本语言的显式回退；不改变旧源。 |
| 目录、组与权限 | get_chapters → chapters，卷章双降序，每批200，核对总量、组织、作品与 translatedLanguage=ja，保留卷号、小数章、分组与稳定阅读顺序。/gating/check 只采用匿名返回的逐章授权，不绕过锁定。 |
| 正文和图片 | get_pages 再复核gating，/images/chapter/{chapterId}?newQualities=true 的 source/low 对象数组读取 filename，只接受官方 uploads.namicomi.com，附原站 Referer。 |
| 设置 | thumbnailQuality_ja 默认original，可选.512.jpg/.256.jpg；dataSaver_ja 默认关；showLockedChapters_ja 默认关且需目录排除能力。只改变图片质量或目录显示，不改变授权与页身份。 |
| 评论与登录 | get_comments 仅读同章，保留分页和剧透过滤；保持匿名访问，不移植上游账号凭据回退、购买或会话写入。 |

独立作品、章、页前缀 `namicomi:ja:`、`namicomich:ja:`、`namicomipage:ja:`。Mihon ID `6873060443067341608`，实际计算键 `namicomi/ja/1`。ExtensionPlugin.kt 对 name 小写，按 MD5 前8字节大端序取正63位；work 裸ID和chapter稳定末段保持原契约。

## 上游与 Koma 宿主调用链

固定上游 [Keiyoushi c09b174459c037c62292e4e7a929a0cd59430bc4](https://github.com/keiyoushi/extensions-source/tree/c09b174459c037c62292e4e7a929a0cd59430bc4/src/all)，实际对照平台 .kt、Filters.kt、build.gradle.kts 和 ExtensionPlugin.kt。宿主合同沿仓内 `sdk/src/runtime.rs`、`sdk/src/source_protocol.rs`、`sdk/src/chapters.rs` 与 `tools/koma-source-dev/src/host.rs` 只读复核；源导出入口接受版本化 args、settings、hostFeatures、environment，游标从 args.page.cursor 读取。所有既有源及 tools 保持不变，不创建分支或工作树。

五语（简中、繁中、英、日、韩）覆盖名称、简介、栏目、筛选、设置、错误和作者后记提示；作品和站方动态标签保留站点内容。固定 source_info 基准简介等于 metadata 的英文 source.description。无独立首页；没有把热门首屏声明为首页。没有读取 HTTP 响应头；不假设响应头只能是字符串。

版本 0.1.0，最低 Koma 1.5.5，safe。候选包、原始联网请求与响应、图像 HTTP/MIME/像素尺寸/SHA-256 及五语结果在 dist/wave-17/verification.json 和 evidence 中。图标见 [ASSET_SOURCES.md](ASSET_SOURCES.md)。本轮不操作设备，不发布；App 书源列表图标呈现等待后续真机 QA。

## 本语言分区实网对照

2026-11-28 独立探测官网 /ja、热门/最新/关键词及免费可读条件。原站 hasAvailableChapters=true、availableTranslatedLanguages[]=ja 返回 63 部作品；代表作《異世界令嬢が恋に落ちたのは元婚約者のおじい様でした》，API 目录 3 章，样本公开话 44 页，初探前3张原图全部HTTP 200并成功解码。最终包内 source.wasm 的重新联网验证以 dist/wave-17/verification.json 为准。

本次只新增语言分身，不抽取或修改既有实现。按固定上游 build.gradle.kts 声明，lang 与 extLang 均为 ja，Mihon计算键精确为 `namicomi/ja/1`。完整继承意大利语源的显式请求校验、匿名gating、游标和章节分组；不移植账号登录、付费权限回退。

## 当前 API 与固定上游的差异

固定上游 Filters.kt 含标题字母升序/降序。但2026-11-28原站 /title/search 对 order[title]=asc 和 desc 均返回HTTP 400，错误 input_validation_error_additional_prop 明确说明 order 不允许 title 字段。其余8个字段的升/降序共16项均实测HTTP 200。本次三个新源只提供这16个真实可用排序，拒绝未知值，不以无效选项交付；所有既有源完全不变。完整请求与响应证据见 dist/wave-17/evidence/sort-contract.json。
