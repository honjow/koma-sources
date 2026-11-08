# 绅士会所

Independent Rust/WASM port of complete HentaiClub.kt/Filters.kt at Keiyoushi
de743aed846da53e00c45ab26ee0e77f7bb4355b (extension v1, lib1.6, Apache-2.0).
Name 绅士会所, zh language and original host www.hentaiclub.net are retained.
The reference is MIXED: per-work rating stays unknown; no individual comic is
classified from the website name. There is no account, preference, latest list,
related-work or URL-import implementation in that reference.

Popular/search/tag/category/details/one fixed chapter/all original data-src
positions and original image headers are implemented. Category pagination
deliberately repeats the original category URL; unsupported latest/URL import
remain unsupported. The first content image is the reference's actual cover.
Opaque CDN URLs are not rejected by guessed ownership. PageID and supplied
manga/chapter/URL owners are strictly checked before image request output.

Anonymous network operations require source_rate_limit_hentaiclub_v1 before
HTTP: exact main hostname www.hentaiclub.net shares 2/1000ms, unrelated CDNs do
not. Single fixed-chapter metadata and image-request descriptions need no HTTP.
Jsoup's unquoted substring attribute selector is quoted for the Host scraper;
its meaning is unchanged. Transport/refusal/malformed/known-empty cases remain
separate, with no inferred takedown or payment.

Current real provider evidence is two same-identity TLS EOF failures, no HTTP
body and zero healthy works. Do not publish a healthy or Native claim. Twenty-one
actual-WASM structural/ownership/error/recovery cases use explicitly synthetic
HTML fixtures. The source is not in the production workspace/registry/index,
has no provider icon yet and is not an installable candidate. Provider readiness
and a sufficiently broad real-body sample remain unverified; no further same-URL
retry is being performed.
