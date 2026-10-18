# 菇菇官方四格漫畫 · 繁中 0.1.0

Source ID `com.beeworks.namepara.zh-hant.koma`, API0.2, minAppVersion0.1.1. One official Traditional Chinese collection, MID `namepara:zh-hant:four-panel`, https://namepara.com/zh/info/comic/ . Source usage and per-work content rating are unknown; manifest nsfw=false. This official Chinese source is not claimed to be an entity in Keiyoushi. Current index.pb SHA e26c3c6bf077e4bbdaa389fdb7f14020ec8297a45aec412b6934ac10e537dce3 is unchanged from the retained formal103 disposition set.

MangaToon (Limited) Chinese entity5676233752760414531 is an existing retained canonical404/home-shell branch; neither failed URL nor old homepage was requested again. Girls' Frontline's official Chinese site exposed an illustration gallery; one exact first linked original was inspected and no comic source was implemented from that gallery.

## Real archive, identity and continuation boundaries

The official archive declares four pages. The source fetches page1, reads its actual `e-pagenation-indicator`, requires every later index_N.html href to have been explicitly declared on that first page, then fetches each declared page once (cap32). It checks canonical category owner, Chinese title and each page's current/total indicator. It includes only actual comic article cards with that category chip and exact `/zh/info/comic/<six digits>_<six digits>.html` identity, excluding the pinned APP/news item.

Four raw pages contain35 comic cards and34 unique article URLs. Two cards both point at `251107_100000` with the same displayed publish date but different thumbnails; source keeps the first actual identity and does not invent another article. Duplicate identity with conflicting date fails closed. Chapter order stays the first-occurrence website archive order, without sorting or inferred numbering.

Each chapter's visible label is the exact website publish date; it is a date label, not a claimed story subtitle. Textual story titles and chapter/volume numbers are absent. Therefore chapterNumber=null, volumeNumber=null, readingGroup=`namepara:zh-hant:four-panel`, continuationEligible=false and pageCount=null. No chapter numbers are inferred from date, image filename or list position. The existing App's manual chapter selection can consume those real source identities/order; actual manual actions remain for Root. Natural page-boundary advancement and automatic reading's adjacent-chapter sequence are not promised by this source contract, and their global gates are not relaxed.

## Actual original pages and image requests

One get_pages call first verifies article membership against the complete archive, then GETs that exact original article. It requires body-article-comic and canonical og:url equal to the actual requested article URI. It reads only the original img src declarations within `#js-article-content.p-article-content.comic`, keeping DOM order and full original URLs. In28 retained actual details this is two pages: first the story title graphic, then the complete four-panel graphic. The source does not crop, merge, resize, convert formats or replace original images with thumbnails.

The article og:image is only a preview (first retained preview is640x452). It is never used as a reader page. Actual first-page title graphic is1173x352 and full four-panel PNG1173x3487; both declared original binaries were fetched once and visually inspected. The source accepts the PNG and JPG suffixes actually observed in real article bodies; one title graphic is JPEG. The first PNG-only implementation's source/WASM/failure replay remain preserved separately.

Image URI contract is HTTPS namepara.com/zh/info/img/<ASCII alphanumeric,underscore,hyphen basename>.png|jpg, without foreign host, query, extra directory or traversal; the exact official touch icon is separately accepted for cover/identity. Supplied mangaUrl must equal the official collection URI; supplied pages mangaId must equal MID. Network-free image_request preserves the emitted URI and ordinary Referer, and does not independently prove cryptographic article ownership of an arbitrary caller-supplied permitted filename. The original article's canonical owner and body mapping establish that relationship in get_pages.

HTML is parsed with bounded literal-template checks and escaped JSON output, without evaluating website JavaScript, HtmlHost optional attributes, login, cookies, signature fabrication, challenge handling, paid content or DRM. Template/identity/category/pagination/URI failures return explicit errors. IDs preserve all original digits exactly as URI identity, without interpreting them as dates or chapter numbers.

## Original official icon and finite evidence

icon.png is the unmodified official head-declared apple-touch-icon-precomposed PNG, https://namepara.com/assets/img/apple-touch-icon.png . Original114x114,6797 bytes,SHA b90b74ad1cd3f1bc3f48442137b522615987a5fa0c7a075a2f98ccbcf6daa532. The yellow backdrop and mushroom drawing are inherent official image pixels. No backing, shadow, mask, redraw, crop, resize or reencoding was added. The package validator accepts the original asset; Native icon appearance remains unaccepted.

Evidence root `/private/tmp/koma-namepara-zh-hant-readonly-20261014` retains all4 actual archive pages and28 original detail HTML bodies with headers/SHA, actual34 identity/date order, the raw declared icon, first title/full original PNGs, preview-role distinction and isolated retained-only host. Production WASM106 operations verify full34 source-order metadata, all28 actual detail mappings/56 original page descriptors/56 image plans,14 malformed input rejections and ABI range bounds. It performs no network and fabricates no HTTP response. The isolated runner is not the Native host.

One remaining declared article `251020_100000` had TLS handshake UNEXPECTED_EOF before any HTTP response; cause/permission/challenge status is unknown. It was not retried and no other client/domain/identity was used. The remaining5 article bodies were not requested after that interruption. These6 bodies have no fabricated replay material and are not claimed validated. Other27 retained articles' image binaries were not fetched; only the first title/full images and official icon have direct binary inspection. Native browse/body/manual chapter selection, natural progression, download and cold persistence are all for Root's named197 acceptance.

No App, ReaderKit, SDK, main runner or main source WIP changes; all earlier candidate source/packages remain frozen. No commit, push, release or device action.
