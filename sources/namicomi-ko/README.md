# NamiComi（韩语）

源 ID `com.namicomi.ko.koma`，语言 `ko`。NamiComi 韩语分区，收录《전 약혼자의 할아버지와 사랑에 빠져도 되나요?!》等创作者与发行方发布的韩语章节。仅提供原站匿名可读内容。

| 行为 | 固定上游 → 宿主 → 本源消费位置 |
| --- | --- |
| 语言 | 固定上游 lang=ko，extLang 不触发特殊映射而保持 ko；SOURCE_INFO 与 typed Chapter.language 均为 ko。API availableTranslatedLanguages[]=ko / translatedLanguages[]=ko。 |
| 列表、搜索和分页 | getPopularManga/getLatestUpdates/getSearchMangaList → get_manga_list/search → list。热门浏览量降序，最新及关键词默认发布时间降序，20项一页，args.page.cursor 保留offset，核对total与每页实际数量。浏览可读性默认all，非空搜索默认available，两个作用域独立。 |
| 筛选 | Filters.kt → get_filters → filter_schema/apply_filters。可读性、分级、状态、排序、包含/排除标签与and/or；fetchFilterData 加动态及三态能力协商后读 /title/tags。 |
| 详情 | get_manga → detail → /title/{id}，标题和简介 ko → en → 首个站方值，公开链接 /ko/title/。这是本语言的显式回退；不改变旧源。 |
| 目录、组与权限 | get_chapters → chapters，卷章双降序，每批200，核对总量、组织、作品与 translatedLanguage=ko，保留卷号、小数章、分组与稳定阅读顺序。/gating/check 只采用匿名返回的逐章授权，不绕过锁定。 |
| 正文和图片 | get_pages 再复核gating，/images/chapter/{chapterId}?newQualities=true 的 source/low 对象数组读取 filename，只接受官方 uploads.namicomi.com，附原站 Referer。 |
| 设置 | thumbnailQuality_ko 默认original，可选.512.jpg/.256.jpg；dataSaver_ko 默认关；showLockedChapters_ko 默认关且需目录排除能力。只改变图片质量或目录显示，不改变授权与页身份。 |
| 评论与登录 | get_comments 仅读同章，保留分页和剧透过滤；保持匿名访问，不移植上游账号凭据回退、购买或会话写入。 |

独立作品、章、页前缀 `namicomi:ko:`、`namicomich:ko:`、`namicomipage:ko:`。Mihon ID `1524915323009957887`，实际计算键 `namicomi/ko/1`。ExtensionPlugin.kt 对 name 小写，按 MD5 前8字节大端序取正63位；work 裸ID和chapter稳定末段保持原契约。

## 上游与 Koma 宿主调用链

固定上游 [Keiyoushi c09b174459c037c62292e4e7a929a0cd59430bc4](https://github.com/keiyoushi/extensions-source/tree/c09b174459c037c62292e4e7a929a0cd59430bc4/src/all)，实际对照平台 .kt、Filters.kt、build.gradle.kts 和 ExtensionPlugin.kt。宿主合同沿仓内 `sdk/src/runtime.rs`、`sdk/src/source_protocol.rs`、`sdk/src/chapters.rs` 与 `tools/koma-source-dev/src/host.rs` 只读复核；源导出入口接受版本化 args、settings、hostFeatures、environment，游标从 args.page.cursor 读取。所有既有源及 tools 保持不变，不创建分支或工作树。

五语（简中、繁中、英、日、韩）覆盖名称、简介、栏目、筛选、设置、错误和作者后记提示；作品和站方动态标签保留站点内容。固定 source_info 基准简介等于 metadata 的英文 source.description。无独立首页；没有把热门首屏声明为首页。没有读取 HTTP 响应头；不假设响应头只能是字符串。

版本 0.1.0，最低 Koma 1.5.5，safe。候选包、原始联网请求与响应、图像 HTTP/MIME/像素尺寸/SHA-256 及五语结果在 dist/wave-20/verification.json 和 evidence 中。图标见 [ASSET_SOURCES.md](ASSET_SOURCES.md)。本轮不操作设备，不发布；App 书源列表图标呈现等待后续真机 QA。

## 本语言分区实网对照

2026-11-28 独立探测官网 /ko、热门/最新/关键词及免费可读条件。原站 hasAvailableChapters=true、availableTranslatedLanguages[]=ko 返回 23 部作品；代表作《전 약혼자의 할아버지와 사랑에 빠져도 되나요?!》，API 目录 3 章，样本公开话 44 页，初探前3张原图全部HTTP 200并成功解码。最终包内 source.wasm 的重新联网验证以 dist/wave-20/verification.json 为准。

本次只新增语言分身，不抽取或修改既有实现。按固定上游 build.gradle.kts 声明，lang 与 extLang 均为 ko，Mihon计算键精确为 `namicomi/ko/1`。完整继承意大利语源的显式请求校验、匿名gating、游标和章节分组；不移植账号登录、付费权限回退。

## 当前 API 与固定上游的差异

固定上游 Filters.kt 含标题字母升序/降序。但2026-11-28原站 /title/search 对 order[title]=asc 和 desc 均返回HTTP 400，错误 input_validation_error_additional_prop 明确说明 order 不允许 title 字段。其余8个字段的升/降序共16项均实测HTTP 200。本次三个新源只提供这16个真实可用排序，拒绝未知值，不以无效选项交付；所有既有源完全不变。完整请求与响应证据见 dist/wave-20/evidence/sort-contract.json。

## 本源复核证据

固定上游 src/all/namicomi/build.gradle.kts 的 name=NamiComi、lang=ko、baseUrl=https://namicomi.com；NamiComi.kt 的 extLang 对 ko 无特殊映射。精确身份键 `namicomi/ko/1` 的 MD5 前8字节大端正63位为 `1524915323009957887`。本源版本0.1.0，minAppVersion=1.5.5。

本次重新读取 sources/namicomi-ja、namicomi-fr、namicomi-de 及宿主 tools/koma-source-dev/src/host.rs、sdk/src/source_protocol.rs、chapters.rs；只新增本语种副本，不修改既有实现。前缀 namicomi:ko: / namicomich:ko: / namicomipage:ko:，设置 thumbnailQuality_ko / dataSaver_ko / showLockedChapters_ko 均独立。API 使用 availableTranslatedLanguages[]=ko / translatedLanguages[]=ko，标题与简介 ko → en → 站方首个值，Chapter.language=Some(b"ko")。

初探热门匿名样本《전 약혼자의 할아버지와 사랑에 빠져도 되나요?!》，作品 `73hLs8EN`，章节 `Z5MNxCT6`，共3章、样本44页；gating/check 对该章明确 true，前3张官方原图均HTTP200且完整解码。原始请求与响应在 dist/wave-20/evidence/preflight-receipts.json。正式验收必须使用最终 .koma 包内 source.wasm，实际执行 info、五语列表/筛选/设置/错误、热门/最新、筛选生效、关键词、详情、目录、pages 和 get_image_request，再下载解码前3页；最终结果见 dist/wave-20/verification.json。
