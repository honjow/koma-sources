# 紳士漫畫 (WNACG)

Independent no_std Rust/WASM port of the complete public WNACG.kt, Filters.kt,
Preferences.kt and mirror behavior at Keiyoushi
de743aed846da53e00c45ab26ee0e77f7bb4355b (extension v6, lib1.6, Apache-2.0).
The source name, zh language, NSFW classification, default buzz/site mirrors and
the exact original desktop User-Agent are retained. Related-works and
ONLY_FETCH_ONCE are not exposed by the current Koma ABI.

Anonymous popular/latest, keyword search, tag/category/ranking/sort filters,
supported gallery/reader/collection URL import, metadata, single-gallery and
complete paginated collection chapters, and every original gallery-image match
are implemented. Collections keep the first occurrence of each data-chid and
reverse the complete original catalogue. Repeated image URLs keep original
positions and separate IDs. Single-gallery date and collection dates retain
Asia/Taipei midnight. No images are filtered, cropped or unscrambled. At the first request of each
Host process, including a cached-page image request, mirror discovery follows
the original BaseUrlFetchInterceptor. A request built for the exact previous
mirror host moves to the current mirror with its path/query unchanged; external
CDNs are never guessed or rewritten.
Source-side owner validation covers PageID, optional manga/chapter/URL owners
and provably different CID partitions in /data/<prefix>/<two-digits>/ paths.
Opaque CDN paths are accepted without guessing their host or ownership.

Host adaptations:
- Stateless calls use an opaque next-source-page cursor for blacklist skipping,
  replacing Mihon's mutable generation/offset object. Query/settings lifecycle
  remains Host-owned. Comma/newline keywords, case-insensitive matching, search
  toggle and default maximum five scanned pages retain the reference behavior.
- html_text lacks a br-node callback. Genuine br tags become literal text
  newlines before HTML parsing; ordinary node text separately normalizes
  whitespace, while descriptions keep whole-text line breaks.
- The original process-scoped mirror refresh uses source_private_session_process_v1
  and CAS source_private_session_replace_v1. One Host process tag triggers one
  ordinary public domain-list discovery. Failure/empty discovery retains the
  previous mirror list for that process, with no TTL, alternate URL or retry.
  Versioned private storage is install-scoped; this differs from upstream's
  SharedPreferences lifetime and is documented rather than treated as identical.
- Mirror option values are opaque URL tokens plus an automatic-first alias.
  Visible options remain the actual mirror hosts. The default follows first;
  an explicit nonfirst pick follows its URL; disappearing or becoming-first
  picks become automatic. This preserves the reference's selection semantics
  without writing public settings from WASM. Settings/login metadata only read
  ready cache, or immediately abort an empty owner; they never discover mirrors,
  call process_scope, commit a value, or send HTTP.
- Extra categories replace the saved menu snapshot after each actual list.
  This uses the same private CAS owner; merge-only settingObservations cannot
  represent logout/removal semantics.
- Text scan-count consumption follows Kotlin's optional plus, BMP decimal
  digits, leading zeros, Int range and invalid/nonpositive fallback five.
  The reference also rejects invalid values before saving. get_settings requires
  source_setting_positive_integer_v1 and declares its existing positive_integer
  validator with the original error message. Ordinary reads keep the reference
  getter fallback for legacy invalid saved values; they do not rely on form
  validation as a substitute. Host implementation exists; Native acceptance is
  tracked separately by Root.

All real network entry points require source_private_session_v1,
source_private_session_process_v1, source_private_session_replace_v1 and
source_rate_limit_wnacg_v1 before RPC/HTTP. The rate owner shares 50/60000ms for
all actual hosts matching ^(?:www\.)?wn(?:acg\w*|\d+)\.; unrelated image CDNs
are excluded. This regular expression never grants Cookie or login permission.
The web authentication descriptor supplies the exact desktop UA and actual
selected mirror /users-login.html. Only explicitly declared origins may get a
Host web session; a newly discovered undeclared mirror fails that grant.
authenticated:false means no source-verifiable account state; no Cookie
inspection or speculative login request is used.

404 and missing h2 keep the reference's ambiguity: content may not exist or may
require login. Neither proves takedown, payment or authentication. Transport,
403 refusal, 429, 5xx, known empty album and malformed responses stay separate.
A failed collection page never publishes a partial catalogue. No fake account,
access-restriction bypass, or retry after refusal is introduced.

Local verification 2026-11-06: a fixed 60-work stratified plan produced 59
complete original website samples and one preserved RemoteDisconnected failure.
The actual WASM matches five list strata and all 59 details, 92 original
chapters and 5807 original image positions. Twelve distinct works' middle
originals were downloaded, decoded and visually reviewed (11 comic pages,
one photograph); no ads or blanks counted. Separate actual-WASM state/ownership/
transport recovery uses explicit local Host test doubles. Local replay does not establish all-site health. Production requires Koma
1.2.0 or later for the Host capabilities described above; Native acceptance and
public distribution must be verified separately.

Review correction: internal V1 used uploadTime, which the actual Host chapter parser did not consume. V2 uses the existing dateUpload ABI. The original V1 WASM loses all 92 expected dates through SourceModels; the V2 WASM preserves all 92 against the unchanged original website oracle. V1 remains immutable failure evidence, and no Host parser was changed.

The production workspace retains this source’s original size-optimized release profile; packaging rejects a WASM over the Host’s 1 MiB limit.

封面请求沿上游桌面 User-Agent 和当前镜像 Referer；宿主以作品 ID 作为 pageId 时，书源只在本地校验已观察到的 WN 缩略图 CDN 与路径，并在给出作品网址时核对作品 ID，不为每张封面重复请求详情。合集封面路径可能属于子相册，不能据此假设它等于合集 ID。正文图片仍使用原章节身份校验。
