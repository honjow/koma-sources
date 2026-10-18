# 漫画社 public adapter

Contract reference: Keiyoushi extensions-source `src/zh/manhuashe/.../Manhuashe.kt`, Apache-2.0, revision `10f06eb833af5990bc5470c84c5e93edc159ad4e`. https://github.com/keiyoushi/extensions-source/blob/10f06eb833af5990bc5470c84c5e93edc159ad4e/LICENSE

Stable source id and old canonical name remain unchanged. Manga `manga:<original path>`, chapter `chapter:<original manga path>:<original chapter path>` and page `page:<zero-based index>` identities remain compatible. The primary's explicit catalogue reversal is preserved; chapterNumber is null, index only represents that order.

Upstream MIXED is conservatively mapped to current nsfw=true/contentRating=nsfw, not a statement that every title is adult. Individual work ratings and actual usage are unknown. No auth, cookie bootstrap, mirror fallback, arbitrary JS, image bytes decrypt or automatic retry. Missing/ambiguous/oversized parsing inputs fail. Normal public HTML contract only; build and host gates do not establish physical acceptance.

Current production entry is `bash build.sh --source manhuashe` using the current canonical SDK and formal CLI. The historical package_candidate.py remains in the original owner only; it is not imported into canonical. Its stale registry correction is represented by this source's nsfw=true row. Minimum App version remains 0.1.0. The original official homepage-declared JPEG is now retained unprocessed as icon.jpg; see ASSET_SOURCES.md. This closes only the package resource prerequisite; small-slot visual suitability remains unaccepted. No publication.

0.1.2 adds only the primary fixed public image-request Referer/Origin descriptor. It preserves the input HTTP(S) URL and sets no User-Agent/cookie. Descriptor success does not prove image download/rendering or fix the observed 197 failure.


Historical 197 0.1.2 accepted only the named 一人之下/767 坦率 17-page route, P1/P3, single download and no-clear cold/Library continuation. That historical native success does not certify the new standard WASM or current website. Original five retained-response results and image descriptors survive; their three raw HTML fixture bodies currently do not, so current-SDK HTTP/DOM equivalence is unknown. This integration changes no parser, ID or public image headers. App display clipping remains independent and unchanged.


Official 150×50 JPEG horizontal logo is source-owned artwork, not code licensed by this adapter. No crop, resize, recoloring, backing removal, shadow/mask, redraw or format conversion is performed. The URL ends .png and the server labels image/png, but the actual original bytes decode as JPEG/RGB with no alpha; package name icon.jpg follows the actual format. App display clipping is separate and unchanged.
