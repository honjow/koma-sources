# WEBTOON (Traditional Chinese) 0.1.1

Independent source `com.webtoons.zh-hant.koma`, content language `zh-Hant`.
Canonical name remains WEBTOON (Traditional Chinese); localized display names are optional metadata.
English `com.webtoons.koma` and Simplified-Chinese Dongman are separate sources; no aliases, migration or shared-history claim.

Current Keiyoushi protobuf identity is 2959982438613576472, extension all.webtoons 1.6.2,
fixed primary revision 10f06eb833af5990bc5470c84c5e93edc159ad4e (Apache-2.0).
Upstream labels the extension SAFE: packaged nsfw=false/sourceInfo safe describes that source-wide label,
not a classification or acceptance of every work. Actual usage is unknown.

Public contract: four finite popular ranking buckets trending/popular/originals/canvas, desktop detail,
mobile episode JSON and ordinary viewer `#_imageList` images. Original href/title/query URLs are preserved.
Manga identity includes language, webtoon/canvas type, actual title_no and actual list path.
Chapter identity retains the exact viewerLink (including encoded Chinese path and episode_no);
page identity includes the actual title, episode and page index. Explicit Chinese title numbers only;
unknown chapter numbers stay null. Primary catalogue order is explicitly reversed, with index as order only.
Complete catalogue requires nextCursor zero/null and at most 2048 records; no truncated-success fallback.
All-types search is limited to a terminal first page; an advertised next page is explicitly unsupported,
never returned with a fake terminal cursor. Search has not had live acceptance.

No latest/day scheduling, filters/settings, MotionToon, synthetic author notes, query quality mutation,
login/paid/account access, ageGatePass/needGDPR cookies, automatic retry, challenge/mirror behavior.
Requests use the existing SDK Firefox121 and fixed public Referer/Origin; mobile catalogue has no Origin.
Image descriptors serialize only literal ASCII space as %20 for transport, preserving all other UTF8 and escapes.
Stored cover/page URI, IDs and public desktop headers remain unchanged, with no cookies.
197 observed five Chinese-filename covers cached but literal post sum.png failed with URL-format code2300003.
0.1.1 transport serialization still awaits native acceptance; frozen0.1.0 evidence is preserved.
HTML and output limits, UTF8/JSON/URL checks reject malformed, unknown, oversized or gated structures.
Logging contains only fixed stage/status/error values, never URLs/titles/queries/body/headers/account values.

Local source-owned packaging (public build/validator stay bound and unchanged):
`python3 sources/webtoons_zh_hant/package_candidate.py --runner /absolute/path/to/local-offline-host`
The wrapper uses paired Rustup stable compiler/cargo in offline mode, reads actual WASM info, validates metadata,
and creates a deterministic two-file .koma plus selected-only index with the exact package SHA.
No icon: the retained accepted English WEBTOON archive contains no icon; none is fetched or invented.

Retained live evidence: one trending GET (30 titles), title5486 detail, its JSON catalogue (154 episodes),
and the earliest ordinary 第1話 新手教學(1) viewer HTML (128 original image addresses).
The first image is the genuine site warning image, retained in place. No image was requested.
These bodies support offline production WASM replay only, not native/image/reading/download acceptance.
197 ordinary acceptance belongs to the root agent after exact source/package review.
