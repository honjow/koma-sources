# CosplayTele

Independent no_std Rust/WASM port of Keiyoushi CosplayTele.kt, Dto.kt and
Filters.kt at de743aed846da53e00c45ab26ee0e77f7bb4355b (versionCode 1, lib 1.6,
Apache-2.0). Provider: https://cosplaytele.com . Reference NSFW/all-language
classification is retained; this is a photographic gallery source.

The default Latest list, Popular API (including its first-page offset 20),
text search, work/category/tag URL search, category selection, lazy category
fetch, details/tags/date, single Gallery chapter and every original
.gallery-item img[src] position are preserved. Current dynamic categories
override defaults by the original label-map contract; repeated desktop/mobile
blocks do not create extra options. Duplicate image URLs keep separate original
ordinals and IDs. No image is rewritten, filtered, cropped or promoted from an ad.

Host adaptations: WordPress Popular API title.rendered is assigned raw by the
fixed reference. This port decodes its HTML entities/markup into Koma plain-text
titles, following Koma's source-runtime ABI (sources own HTML-entity decoding and
publish ordinary Unicode). All 20 captured popular records retain both raw
rendered and decoded titles in the evidence; they are NOT byte-for-byte title
equivalence with that reference assignment. Mihon ede052af170e2ef2949dac2acd91adfb58043a96
SMangaImpl stores a plain lateinit title and SManga.toDomainManga copies title
directly; no equivalent cleaning was observed in these model paths. Later UI
rendering has not been established, so this port does not assert Mihon already
does the same transformation.

Koma has no related-work operation; that optional reference
operation cannot be exposed here. Filter header/fetch fields depend on host
declarations. Date retains the reference's calendar day without inventing a
timezone. Standard HTTP/Reader image transport is used because the reference has
no custom account, image-header or decryption contract. Home, settings,
credentials and image_request capabilities are false.

IDs keep the original site path: cosplaytele:/robin-2/,
cosplaytele-read:/robin-2/, cosplaytele-page:/robin-2/:0. Manga/chapter ownership
must match before networking. Work URL search accepts the same fixed provider
origins. No arbitrary-domain fetching or mirror is introduced.

Failures distinguish transport, 401 login, 403 refusal, 404 missing resource,
429 rate limit, 5xx, malformed HTML and no images. A 404 does not assert takedown.
No automatic auth, retries after denial or fabricated fallback pages.

Local verification 2026-11-05: five list/search/category strata (57 distinct
discovered paths), 55 complete work samples, 55 one-chapter directories and all
3204 original image positions matched independent saved website parsing.
Twelve different galleries' originals decoded; ten visually confirmed as actual
photographs/montage, not ads, text pages or blanks. Latest and Robin body also
ran through real CLI/WASM HTTP. Frozen replay separately checks popular order,
full page tables, dynamic categories, owner mismatch, duplicate positions,
401/403/404/429/500/transport errors, no-images and subsequent recovery.

These are local results, not proof every gallery works or 237 has passed.
Detailed evidence: Koma .hvigor/outputs/source-coverage-20261105/new-source-ports/.
No package/index has been published by this task.

Review corrections: opt-in SDK source_protocol validates direct request fields,
duplicate identities and JSON string escapes before any HTTP. Quoted/Unicode
search is decoded by the existing SDK decoder, then URL-encoded. HTTP refusal
status precedes Host transport completion; status 200 with ok=false remains a
network error. A confirmed normal article template without direct gallery images
uses chapter_unavailable; an unknown/malformed template remains parse_error.
Feature checks read only the actual top-level hostFeatures array. V1 candidate
evidence remains immutable; these corrections are frozen separately as V2.


Cover completion (user report 2026-11-06): the reference's detail parser and this
port's old work-URL card had no thumbnail. Both now share an optional resolver
for the article's own declared `link[rel=alternate][type=application/json]`
WordPress post endpoint, using the adopted `wp:featuredmedia` embed relation.
The returned post id must match that declaration, link/slug must match the work,
and `featured_media` must equal the selected embedded media id. No gallery-first,
related-work or arbitrary nested `source_url` fallback is used. `wp:attached-to`
is evidence of the upload's initial parent, not a gate: a post can reuse another
post's image as its selected cover. The original resource URL is preserved.
Auxiliary metadata denial/transport/schema/owner failures leave the previously
available article intact without a wrong cover; cancellation still propagates.
Primary HTML failures remain failures. Chapters and all page URLs/IDs are
unchanged. Four real declared metadata/image cases across existing strata and
12–126-photo albums were checked; 55/3204 frozen chapter/page responses are
exactly equal to V2. V3 is frozen separately and awaits Root's cover UI check.
