xkcd Chinese independent source, com.xkcd.zh.koma 0.1.2.

Archive order, chapter IDs, metadata and cover stay unchanged. get_pages validates archive membership then reads the chapter HTML and collects every direct #content > img:not([id]) in DOM order. Zero matching ordinary body images returns the existing unsupported error. No ID blacklist or template-name heuristic. The old single-image PageID :0 remains; further body images use their DOM ordinal. All selected URI values pass the existing same-origin strip-image boundary. Metadata preview images do not replace body-role checks.

Real retained 2978 is one normal image and stays identical. Real 1110 is an interactive overlay/map; 1331 requires frequency animation; 1190 requires its frame/canvas player. These three have zero ordinary body matches and return unsupported. A controlled two-image variant proves no first-image truncation; it is not an observed live multi-image comic.

Cost: get_pages adds one HTML GET after the archive GET. get_image_request and cover consumers stay unchanged, with no new network call. Already persisted old page plans/downloads cannot be revoked by this new method; cache refresh and Native acceptance are not claimed. No synthesized alt/title text page, JS execution, tile/image fetch, login, retries or style changes.

Keiyoushi primary revision 1319949ca9bc456908196e3ba2d104e94182c5d6 Xkcd.kt:107/375–405 uses this selector but selectFirst and adds a generated text page. This candidate retains the complete matching body set rather than treating its first item as proof of completeness. Other unknown HTML structures remain unverified.
