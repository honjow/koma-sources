# 原神官方漫画 · 0.1.1

Independent Koma source for the Chinese manga published at
https://ys.mihoyo.com/main/manga . This source is absent from the current
Keiyoushi Chinese catalogue; it does not reuse another provider identity.
Stable package ID: `com.mihoyo.genshin.zh.koma`; book ID:
`genshin:zh:official-manga`; chapter IDs use the official `iInfoId`; page IDs
use chapter identity plus the original zero-based array position.

The site's published `3738d5b.js` declares content_v2_user app 43,
key 16471662a82d418a, channel 725. Its `56dc5b3.js` manga component declares
getChapterTotal pageSize 1000 and consumes `sExt` key `725_0` in array order
for pages and `725_1[0].url` for covers. Only normal public GETs are used.
No cookies, login, challenge handling, arbitrary JavaScript, statistics POST,
paid chapter unlocking or image conversion is implemented. Host HTTP `status`
and source `retcode` must be 200 and 0 respectively. JSON is parsed locally;
there is no HtmlHost attribute lookup or dependency on its missing-attribute
semantics. The source exposes native alloc/free and bounds-checks requests.

Current complete list has 15 chapters, including official title/ID pair
118835 “第一话 添酒”; it does not list chapter six or a prologue. This source
preserves the published list, including its gaps and titles, without adding
predicted chapters. `iTotal` must match the complete received list; a truncated
response produces an error. Browse/search returns this one manga; searches
match its local Chinese/English title, not a fabricated remote search API.
Chapter ordering follows the official navigation's reversal of the API list. Explicit Chinese number prefixes provide chapterNumber; unknown or extra chapter titles yield null. Status, authors, individual
content rating and real source usage are unknown.

Frozen live evidence: complete 15-entry list, first-chapter detail with 53
original page URLs, and one real 800×1131 JPEG first page. The other 52 image
bodies and other chapters' image bodies have not been downloaded or decoded.
Retained-response production-WASM replay matches all 15 chapter IDs/titles/
page counts and all 53 page IDs/indices/URLs; negative IDs/hosts/cursors and
ABI bounds are checked separately. This is host evidence, **not physical-device
acceptance**. Root must review the final source and package, then perform
ordinary installation/browse/search/detail/read/download/cold paths on the
selected device. No device actions, App/SDK/runner-tree edits, push or release
are part of this delivery.

## Official icon provenance

The fetched official catalogue HTML declares `rel=icon href=/favicon.ico`.
That endpoint returns 302 with official Location
`https://ys.mihoyo.com/main/favicon.ico`. The ICO file contains ten original
entries. `icon.png` is the **unchanged PNG byte slice** of entry index 3,
256×256, offset 7518 and length 87373. No raster decoding/reencoding,
cropping, background, mask, shadow or added artwork is applied. Its original
Paimon / “6th” / miHoYo design is preserved.

Original ICO SHA256:
`2410be3117373b4f5e82f8f38f4711ee997210bd602deadf62af4d9db515f72f`

Packaged PNG SHA256:
`42d4debfa65eb66f8625dc3ffa89813f71ef3638c5d271b510cb51016c2e638e`

The header wordmark was inspected and retained as research material, then
replaced in this candidate by the actual official favicon before freezing.
Visibility/size in the application's existing icon slot remain unverified.

Metadata R2 preserves the R1 freeze and corrects only chapter order and title-derived chapterNumber. The single book currently has no `url` field; its existing complete source link remains in `links`. No other field was added. Native acceptance remains unverified.

## 0.1.1 continuation metadata

The official `getChapterTotal` returns the one Chinese manga's complete chapter
list in reverse API order; `getDetail` assigns that same list to `chapterList`.
For these verified same-work chapters, each chapter now emits
`readingGroup: "genshin:zh:official-manga"` and `continuationEligible: true`.
This enables the existing application continuation gate without changing it.
All original chapter IDs, title-derived numbers (including the missing sixth
chapter), titles, page arrays, URLs, icon and capabilities remain unchanged.
The original installed 0.1.0 metadataR2 freeze is preserved. This candidate has
only retained-response WASM evidence; native boundary navigation is unverified.

## 0.1.2 page-array terminal handling

Current standard entry: `bash build.sh --source genshin-zh`. Only page_count
and get_pages aggregate the source-declared 725_0 image array through a local
Result iterator. Invalid consumed elements return parse_error rather than
pretending the preceding image is a complete chapter. No new size or URL rule
is added. Catalogue iTotal checking and cover first-match/unread-tail behavior
remain unchanged. This is the successor to reviewed 0.1.1 metadata; device237
registry still reports 0.1.0, so a future update also adds that already-reviewed
readingGroup/continuationEligible metadata. No deployment is included here.

The preserved 256px official icon bytes remain unchanged; manifest adds the
current mandatory local icon field. Artwork suitability remains separate.


## 237 现装0.1.2第十六话首两页（2026-10-21）

本轮registry实读现装0.1.2、enabled=true，取代上方旧“237仍0.1.0”的当时状态；没有更新包或修改元数据。普通15/15章目录选择无现存章节记录的《第十六话 添酒》（genshinch:zh:118850），未重开已有第一话样本。Root亲看1/21与2/21两完整彩色剧情原图，安柏出窗、跃空与蒲公英可辨；两页主要是动作画面及中文拟声，不扩大为大段对白、全21页、下载、cold或无缓存在线首取验收。

普通Back后真实进度为genshinpage:zh:118850:1、pageIndex1、totalPages21、completed=false。源执行者在返回后因502/429中断，Root沿同一manifest及770dcddc租约接管恢复；最终Browse末端与10000ms/AWAKE亲看读回，租约已释放。52项registry与reader偏好XML相对本轮baseline内容相同，原17sourceHistory/60chapter对象保持，仅新增本章记录；原47progress中仅本书自然从原第一话变为本章P2，其他书对象保持。没有编辑持久记录、清数据/缓存、改变模式或修源。

原件在Koma/.hvigor/outputs/user-flow-audit/device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership/的genshin012-{baseline,public-list,detail,chapter-seek,ch16-first,ch16-second,ch16-natural-return,session-restored}237-001；哈希关闭。只接受上述具名正文加载、翻页与自然返回边界。
