# 漫画社 public adapter

Contract reference: Keiyoushi extensions-source `src/zh/manhuashe/.../Manhuashe.kt`, Apache-2.0, revision `10f06eb833af5990bc5470c84c5e93edc159ad4e`. https://github.com/keiyoushi/extensions-source/blob/10f06eb833af5990bc5470c84c5e93edc159ad4e/LICENSE

Stable source id and old canonical name remain unchanged. Manga `manga:<original path>`, chapter `chapter:<original manga path>:<original chapter path>` and page `page:<zero-based index>` identities remain compatible. The primary's explicit catalogue reversal is preserved; chapterNumber is null, index only represents that order.

Upstream MIXED is conservatively mapped to current nsfw=true/contentRating=nsfw, not a statement that every title is adult. Individual work ratings and actual usage are unknown. No auth, cookie bootstrap, mirror fallback, arbitrary JS, image bytes decrypt or automatic retry. Missing/ambiguous/oversized parsing inputs fail. Normal public HTML contract only; build and host gates do not establish physical acceptance.

Current production entry is `bash build.sh --source manhuashe` using the current canonical SDK and formal CLI. The historical package_candidate.py remains in the original owner only; it is not imported into canonical. Its stale registry correction is represented by this source's nsfw=true row. Minimum App version remains 0.1.0. The original official homepage-declared JPEG is now retained unprocessed as icon.jpg; see ASSET_SOURCES.md. This closes only the package resource prerequisite; small-slot visual suitability remains unaccepted. No publication.

0.1.2 adds only the primary fixed public image-request Referer/Origin descriptor. It preserves the input HTTP(S) URL and sets no User-Agent/cookie. Descriptor success does not prove image download/rendering or fix the observed 197 failure.


Historical 197 0.1.2 accepted only the named 一人之下/767 坦率 17-page route, P1/P3, single download and no-clear cold/Library continuation. That historical native success does not certify the new standard WASM or current website. Original five retained-response results and image descriptors survive; their three raw HTML fixture bodies currently do not, so current-SDK HTTP/DOM equivalence is unknown. This integration changes no parser, ID or public image headers. App display clipping remains independent and unchanged.


Official 150×50 JPEG horizontal logo is source-owned artwork, not code licensed by this adapter. No crop, resize, recoloring, backing removal, shadow/mask, redraw or format conversion is performed. The URL ends .png and the server labels image/png, but the actual original bytes decode as JPEG/RGB with no alpha; package name icon.jpg follows the actual format. App display clipping is separate and unchanged.

## 237 现装 0.1.2 具名在线观察（2026-10-21）

237 VDE-AL00 展开竖屏1320×2120，普通 Browse 行实际显示 0.1.2；不能把197的资源交付或本地产物文件名当作237版本。普通热门返回20本，《一人之下》详情及目录显示800/800章。原767《坦率》行已有下载，本次没有重开它作为在线证据，而打开未下载的766《反杀》。首帧仍为图片解码中，随后只读观察亲看真实中文正文1/17；普通右侧下一页后亲看不同正文2/17。

自然 Back 后原读档记录绑定 comicId=`com.manhuashe.koma:manga:/comic_13871.html`、chapterId=`chapter:/comic_13871.html:/chapter_13871_1342899.html`、pageId=`page:1`、pageIndex=1、totalPages=17、completed=false。未直接修改持久状态、未收藏或新增下载。原Browse末端完整画面已亲看，50项源registry与进入前完整相同，原10000ms/AWAKE读回。只接受这个现装版本的具名列表、目录、首两页与自然返回；没有核验安装包字节身份，不外推当前源码WASM、其他章节、下载、冷启动或全源。

原件沿 Koma `.hvigor/outputs/user-flow-audit/device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership/` 的 `manhuashe012-popular-terminal237-001`、`manhuashe012-yirenzhixia-chapters237-001`、`manhuashe012-yirenzhixia766-{first-ready237-001,page2-237-001,natural-return237-001,session-restored237-001}`。最初定位回执误名为manhuashe013-browse237-001，实际界面明确012，保留原名且不据此升级版本。一个日志命令被参数校验阻止，后一次-x/-z组合返回多命令错误，均不算有效日志或产品故障。

Root 已亲审本条首两页原图及精确记录；结论仅限上述具名路径，原未闭边界保持。
