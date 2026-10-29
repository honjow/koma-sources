# MangaDex — Simplified Chinese 0.1.10

Independent source `org.mangadex.zh-hans.koma`, API 0.2, content language zh-Hans, provider API language `zh`. Other language-source IDs remain independent; no alias or migration. Provider language metadata does not guarantee the writing system of every image. Implementation/license and original official icon attribution remain in NOTICE/LICENSE; icon bytes/background/mask are unchanged.

Listing 关注较多 is all-time followedCount, not 30-day popularity or usage. Search/list require available provider-language chapters and default to safe/suggestive titles. Original localized title/alt-title/summary fallbacks and actual per-title ratings remain. Complete owned feed pagination, UUID/language/total/duplicate checks and whole-operation bounds remain; external, unavailable and zero-page chapters stay omitted. Null/fraction/zero chapter and volume metadata are not replaced with ordinal guesses.

`thumbnailQuality_zh` follows fixed Keiyoushi 949dea252eb8e867b7b40834a26e13088764ff55: original (default Host value `original`, encoding upstream empty suffix), medium `.512.jpg`, low `.256.jpg`. List/search/detail append only the selected published suffix to the actual cover filename. Missing/unknown settings preserve original covers.

`dataSaverV5_zh` defaults false. Only explicit true selects actual `chapter.dataSaver` filenames and `/data-saver/`; otherwise original ordered `chapter.data` and `/data/` remain. get_pages still validates the complete owned feed and declared page count; stable page IDs carry the original chapter UUID/index. Every image request still makes its existing fresh normal at-home GET and resolves the selected quality by index. Both selected and original file sets retain safe filename, capacity and duplicate guards; no filename is guessed.

Image HTTPS hosts remain uploads.mangadex.org or the actual descriptor-provided mangadex.network subdomain. Cover requests remain bound to the same manga UUID. No tokens, Cookie handling, statistics POST, auth/retry fallback, new host grants or App/style changes. Standard build: `bash build.sh --source mangadex-zh-hans`. Runtime/crate/package versions align at 0.1.10.

`altTitlesInDesc_zh` defaults false and is consumed only by manga detail descriptions. Explicit true appends the fixed reference `Alternative titles:` block: extension-language or original-language romanized maps, exactly one nonempty value per map, provider order, no sorting/deduplication. The shared SDK plain-text helper decodes JSON Unicode and HTML entities once while preserving literal markup; the original base description is retained. Missing/false leaves the prior description bytes, title fallback and all other fields unchanged. Title priority and final-chapter text have independent settings below.

`usePort443_zh`（Use HTTPS port 443 only）默认false。沿固定Keiyoushi949dea的实际消费者，只在显式开启时给at-home服务器查询追加 `?forcePort443=true`；默认请求原样。图片URL、页ID/序和校验仍取真实响应，不向URL硬写端口，不改其它API、封面、认证、网络、超时或重试。中文图片请求复用同一descriptor/settings合同。

`tryUsingFirstVolumeCover_zh` defaults false. Explicit true follows the fixed reference bulk `/cover` query ordered by volume, selecting the first cover related to each returned manga with its original-language locale. Search/list/detail consume it before applying the existing cover-quality suffix. Missing locale, no matching cover, unsafe filename or optional HTTP/parse failure keeps the original cover. Default requests and cover URLs remain unchanged; no chapter/page/identity or Library data is rewritten. Existing library covers require their ordinary explicit refresh. The shared provider implementation is `sources/mangadex/first_volume_cover.rs`.

`originalLanguage_zh` is the fixed original-publication-language multiselect (Japanese/Chinese/Korean), empty by default. It filters public list/search requests only; Chinese includes both `zh` and `zh-hk`. It does not select translated chapter language, rewrite IDs or Library titles, or filter exact detail/chapter/image restoration. Missing/empty selection preserves previous requests and output.

`contentRating_zh` follows the fixed four-option content-rating multiselect; missing settings default to `["safe","suggestive"]`, preserving previous list/search requests and guards. Explicit choices supply only those API query values and accept only their known returned ratings. Explicit empty selection omits this API filter, as in the reference; unknown/missing rating data is still rejected. Exact details, complete chapter feeds (including their existing all-rating query), pages, image permissions, IDs and existing Library data are unaffected. No account/login or permission bypass is added.

## 完结卷章说明

`finalChapterInDesc_zh`（Final chapter in description）默认关闭，保当前Koma无配置的简介；固定参考默认开启，这里保留既定缺省输出。显式开启仅在详情末尾按固定MangaDexHelper拼接非空 `lastVolume`/`lastChapter` 为 `Final chapter:\nVol.x Ch.y`，在已有简介/别名后隔一空行，并按原实体表解码一次。沿上游原字符串，不猜最大章、不要求数字、不剥正文标签。缺少两值不造段落，非法字段或容量不足明确失败。列表、HTTP、标题、ID、目录/页顺序和已存Library不主动变化。

## 标题优先顺序

`preferExtensionLangTitle_zh`（Use alternative titles）默认true，保当前Koma原标题优先链与原JSON输出，固定参考也是true但既有链的细节不据此重写。显式关闭沿固定MangaDexHelper：扩展语种primary值仍优先，缺失才用primary首value，再回扩展语种单值alt、英文单值alt，并实体解码一次。列表、搜索、详情及已有Home共用；不排序或去重标题，不主动重写Library名称、ID、目录及页面。显式关闭时按参考区分空字符串与缺失值，不因字符串为空继续回退；默认和开启仍保原空标题处理，容量不足明确失败。

UUID blacklist: `blockedGroups_zh` and `blockedUploader_zh` are comma-separated UUID text settings exposed only when the host declares both `source_setting_uuid_list_v1` and `source_chapter_exclusions_v1`. Both default empty; empty or older hosts preserve existing requests and results. The host validates the fields before saving; the source also rejects invalid values rather than adding unchecked query parameters. The normal chapter feed applies both exclusions, while exact chapter recovery and page ownership checks bypass them. This source has no Latest or Home section; its ordinary manga search and followed listing retain their existing requests.

Status filtering supports the fixed reference Ongoing, Completed, Hiatus and Cancelled multiselect. Explicit values are sent as status[]; no selection preserves the original request, available chapter and language constraints, rating, settings, and ordering. Older hosts see no new filter. Other reference filters remain unimplemented.
