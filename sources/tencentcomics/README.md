# 腾讯动漫 0.1.4

Public Chinese catalogues, title search, details and ordered chapters. Pages are
limited to chapters currently marked free in the public catalogue and whose page
data explicitly says canRead=true. A chapter with the provider's pay marker
and no free marker returns `paid_chapter`. Missing or conflicting access marks
return `permission_denied`. The gate returns before requesting any non-free
reader HTML. No login, purchase, DRM, challenge handling, JavaScript execution,
or account headers are implemented.

Nonce parsing permits only concatenated alphanumeric string literals and bounded
numeric constants with up to three decimal places: unary !/~/+/-; arithmetic;
comparisons; conditional expressions; Math.pow with exponent 0..8; Math.round; charCodeAt() on a bounded ASCII string literal.
A whole nonce atom may also use substring(nonnegative constant) on a bounded
decimal string literal; its entire result is converted to an integer. String
addition and other string operations remain unsupported. The final nonce atom
must be an integer. All values, input length, nesting, steps, noise tokens,
base64 data, handles and page count have fixed limits. The exact public viewer
nonce atom `1 - !!document.children` folds to zero because the [DOM Standard
defines `children` as an HTMLCollection](https://dom.spec.whatwg.org/#dom-parentnode-children),
which is truthy regardless of length. No browser property is accessed. Other
global expressions, arbitrary calls, noninteger values, overflow and unknown
encodings fail explicitly. This expression was observed in the public free
chapter `537057:3` of 《铳火》.

IDs use canonical comic/chapter/picture IDs, never temporary image URLs. The
known image domain is HTTPS manhua.acimg.cn; Referer/User-Agent are public constants.
Desktop catalogue pagination follows the actual next link; mobile search uses the
observed ten-result contract. Localized display names live in metadata.json and
remain separate from source identity and content language. Attribution: NOTICE.

Source/build/host checks do not replace the named Koma native-device acceptance.

Historical 0.1.1 named Koma acceptance on 197 (2026-10-10): ordinary subscription update and
physical package/module digests matched; default listing shows 12 covers;
一人之下 search and 834/834 chapters; 1.姐姐1 download 17/17, actual first
image retained after no-clear cold start; ordinary picker from chapter 1 opens
2.姐姐2 with actual panels and 1/17. Existing library/progress/download objects
and original preferences were preserved. This accepts only the named public/free
path. Earlier uncaptured 0.1.0 Reader failure remains unexplained. Unknown nonce
grammars still fail explicitly; no whole-site availability claim. Current host
search 一人 traverses 10 results then 2 distinct results and terminates; its
second native search page remains unverified.

Current 0.1.2 chapter semantics use explicit numeric labels with the reviewed separators to populate chapterNumber; unknown labels remain unknown and are not numbered by index or chapter ID. Original chapter identity/order and paid/locked identity are retained; paid content is not requested. The exact reviewed owner, retained 835-record contract/output comparison and source diff are preserved in `/private/tmp/koma-tencent012-chapter-semantics-20261017/` (Root review included). This canonical integration preserves that parser and adopts the current shared SDK/standard flags; the historical 0.1.1 acceptance above is not acceptance of newly compiled canonical WASM. Current icon identity and its Native visual limits are described in ASSET_SOURCES.md.

目录支持参考排序、属性、进度和标签筛选；未选择排序时分别保留热门与最新原路由，标题搜索不消费这些筛选。旧宿主不展示新筛选。
