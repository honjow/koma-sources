# Buon Dua 0.1.0

Independent no_std Rust port of the complete Keiyoushi BuonDua.kt at
de743aed846da53e00c45ab26ee0e77f7bb4355b (versionCode 13 / lib1.6 / Apache-2.0).
Provider https://buondua.com, sourceId com.buondua.koma, reference NSFW/all language.

Popular and latest use the exact 20-item offset requests. Non-empty text search
wins over Tag ID; empty search/browse can use the original Tag ID route. Details
retain the title's page-suffix removal, article info, original download link
descriptions, public archive-code text and tags. No download link is fetched.
Details and URL search use the same article's official Open Graph cover when
its owner matches the requested gallery and the image is on a provider host.
Missing or foreign metadata leaves the remaining detail available.
Default pref_split_pages=true lists webpage chapters newest-first, Page N down
to Page 1; false returns one Gallery and concatenates every real webpage's images
in ascending webpage/original DOM order. Repeated image URLs keep their positions.
A failed webpage fails the whole merge; no partial success or invented page.

Host adaptations: independent Koma chapter IDs explicitly record webpage/gallery
mode, preserving precise existing chapter IDs across settings changes. The
upstream's optional related-work function has no Koma ABI operation. Whole-gallery
reads are sequential because the Host has bounded calls; content order is
unchanged. The date preserves the reference's unzoned local date/time, without
adding a timezone. Text/header filter schema uses supported Host declarations.
Standard image transport is sufficient: actual CDN original bytes matched with
no Referer or Cookie. No image-processing/header/auth feature is claimed.

Requests to buondua.com share the reference's 10 requests per second Host
limit. The rule does not include i2.buondua.com or other CDN hosts. Network
operations require the specific source_rate_limit_buondua_v1 capability before
performing HTTP; the older generic rate feature is not a substitute. The
production package requires Koma 1.2.0 or newer, whose Host provides this rule.

Settings identity contract: when the Host declares source_chapter_exclusions_v1,
a complete split catalogue explicitly excludes its Gallery ID; a complete merged
catalogue explicitly excludes its verified Page 1..N IDs. This hides alternate
rows through the existing Host contract without deleting progress/downloads.
An old exact CID still resolves its own encoded mode. Before publishing those
exclusions, the advertised Page 1/N and real pagination End must agree; malformed
pagination fails rather than publishing a partial catalogue.
