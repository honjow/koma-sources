# NamiComi 中文源 README 历史快照

仅用于明确历史查证，不是当前合同，不从默认阅读链引用。

## namicomi-zh-hans

# NamiComi — Simplified Chinese independent Koma source0.1.1

Stable ID `com.namicomi.zh-hans.koma`; canonical `NamiComi`, lang `zh-Hans`, API0.2. Only formal entity1163192659786040070. Traditional Chinese is implemented independently as com.namicomi.zh-hant.koma; it is not aliased, migrated or overwritten by this source. Optional localized names identify this language variant; usage UNKNOWN. Extension declares SAFE; each title retains actual safe/restricted/mature/unknown rating independently.

Fixed Keiyoushi primary1319949ca9bc456908196e3ba2d104e94182c5d6: ordinary public api.namicomi.com JSON, original uploads.namicomi.com covers/pages. Existing Firefox121 UA + published Referer/Origin. No Cookie/Authorization/token, account, registration, premium purchase, browser/JS/challenge bypass, mirror/UA switch or image decrypt. Source-local raw POST preserves existing SDK ABI; SDK unchanged.

Popular/latest select available `zh-hans` chapters; search explicitly sends published `hasAvailableChapters=true` and `availableTranslatedLanguages[]=zh-hans`, normalizes whitespace. Actual lowercase language-key descriptions take precedence over English fallback. Published source default first cover is preserved (may be English asset); no fabricated cover. Authors, status, tags and per-title rating use actual entity data.

Catalogue returns complete server-paginated language-matching records in original order, validating totals, IDs/relationships and each200-entry anonymous gating/check chunk. Only current true entries are exposed, matching upstream showLocked=false default. Missing/nonboolean access results fail; false chapters stay hidden. Pages perform a fresh anonymous check and stop on401/402 or map=false without auth retry, then return original source-quality array in exact order. No fabricated chapter numbering, dates, free flag or medium/low substitution. IDs: `namicomi:zh-hans:<title>`, `namicomich:zh-hans:<title>:<chapter>`, `namicomipage:zh-hans:<title>:<chapter>:<zero index>`.

Strict complete UTF8/JSON (including Unicode pairs), HTTP2xx, declared pagination and safe API/CDN identities. Final payload plus framing below native1MiB; any catalogue/body/output capacity overflow fails, never silently truncates. This pure JSON source creates no HTML descriptors.

Retained actual normal responses: two Simplified Chinese titles; 我是赛拉 detail; one Chinese chapter URFD7dhU, no volume/chapter number; anonymous map=true; twenty original source pages. Root actual first PNG2044565B SHA078a8bc4... decoded1200×1680; this doesn't validate other nineteen images or all native routes. Production WASM retained-only replay and limited error gates are separate from native acceptance. Popular, latest and the exact normalized search now have their own retained actual responses and production WASM replay; Historical 0.1.0 has finite 197 acceptance for the named 我是赛拉 route, first two pages and 20/20 chapter download/persistence; the ordinary search failure remains unknown and unaccepted. The 0.1.1 update separately accepted icon/version and no-clear process persistence. These historical results do not accept a new current-SDK build or the current App rendering/mask contract.

Build/package after canonical integration: `bash build.sh --source namicomi-zh-hans` using the current standard source repository. The original package_candidate.py, borrowed builder/validator and retained runner paths are historical evidence, not production entry points restored with this provider. Runtime version is 0.1.1; the original crate package version 0.1.0 is retained. No new build, website or Native acceptance follows from this documentation correction.

## 2026-10-21 named search follow-up

Current canonical WASM source_info reads com.namicomi.zh-hans.koma / 0.1.1. One actual production dev-host invocation of search with query `我是赛拉` and page1 returned the unique `namicomi:zh-hans:5eKytbrB`, author Everybnuy, and terminal pagination. The retained original search-once HTTP200 receipt used `我是 赛拉` (with one internal space); these queries are explicitly distinct. The existing source normalizes whitespace, percent-encodes the title and requests the published available-Simplified-Chinese filters. No source or request-contract change was made. The SDK response suffix statically labels hostHints.network=false; it does not negate the actual CLI HTTP callback/ureq transport and must not be read as no-network evidence.

On device237 VDE-AL00, unfolded portrait1320x2120, the already installed independent Simplified Chinese package read back 0.1.1. Its ordinary source search for `我是 赛拉` completed with one visible 我是赛拉 card, decoded cover and 没有更多了. This observes only this current named search consumer. The exact historical197 failed query/raw error was not located, so its cause remains unknown; no source fix or general search recovery is claimed. The previously accepted reading/download paths were not rerun. Original evidence: Koma/.hvigor/outputs/user-flow-audit/namicomi-hans-search-current-host.{json,stderr.log} and device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership/namicomi-hans-search-{baseline237-001,query237-001,result237-001,restored237-001}/. No new package, source version, icon, App style or persistent preference change.

## namicomi-zh-hant

# NamiComi — Traditional Chinese independent Koma source 0.1.1

Stable ID com.namicomi.zh-hant.koma; canonical NamiComi; API 0.2; content language zh-Hant. Only formal entity 7859611418350123856; no alias, migration or overwrite of Simplified source. Usage unknown. Actual title ratings safe/restricted/mature/unknown are independent of extension SAFE metadata.

Fixed published primary 1319949ca9bc456908196e3ba2d104e94182c5d6. Canonical public JSON, exact original Ref/Origin/Firefox121. Popular/latest availableTranslatedLanguages[]=zh-hant; search explicit hasAvailableChapters=true and same language. Normalized zh-hant title/description key preferred, then English/first fallback. Published default first cover retained even if another locale. Authors/status/tags/rating actual.

Catalogue validates complete limit200 pagination, IDs, zh-hant language and actual title relationships; null volume/chapter numbers remain null. Anonymous gating/check raw JSON chunks <=200 actual IDs, strict per-ID boolean; false hidden per published default, missing/nonbool/error fail. Every get_pages makes fresh single-chapter gate. 401/402/map false stop; no login/authHeaders/token/account/payment fallback. Default data.source originals preserve index order, exact uploads.namicomi.com host, safe ID/hash/filename. No data saver/medium/low or alternate hosts.

Strict complete UTF8/JSON including pairs and four JSON whitespace bytes; host error envelope currently folds network_error as the separately frozen Simplified package does. Source capacity fails explicitly; response+framing<1MiB. Pure JSON creates no HTML descriptors. SDK unchanged.

Retained actual Traditional popular meta20/0/23 contains first20 only (17safe/3mature); remaining3 not fetched. First actual SAFE73hLs8EN title 愛上我前未婚夫的祖父 has actual three chapters3/2/1 with null volume, 20/15/44 pages and anonymous map true. Chapter1 EiiT84c6 has retained44source pages. Production retained-body offline evidence is separate from device acceptance; Historical 0.1.0 has finite 197 acceptance for the named 愛上我前未婚夫的祖父 Chapter 1 route, first two pages and 44/44 chapter download/persistence. The 0.1.1 update separately accepted icon/version and no-clear process persistence. Other titles/pagination and a new current-SDK build remain unaccepted; the current App rendering/mask contract remains OPEN. No live site HTTP/device done by this owner.

Build/package after canonical integration: `bash build.sh --source namicomi-zh-hant` using the current standard source repository. The original package_candidate.py, borrowed builder/validator and retained runner paths are historical evidence, not production entry points restored with this provider. Runtime version is 0.1.1; the original crate package version 0.1.0 is retained. No new build, website or Native acceptance follows from this documentation correction.
