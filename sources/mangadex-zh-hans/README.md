# MangaDex — Simplified Chinese 0.1.1

Independent source `org.mangadex.zh-hans.koma`, API 0.2, content language zh-Hans, provider API language `zh`. Other language-source IDs remain independent; no alias or migration. Provider language metadata does not guarantee the writing system of every image. Implementation/license and original official icon attribution remain in NOTICE/LICENSE; icon bytes/background/mask are unchanged.

Listing 关注较多 is all-time followedCount, not 30-day popularity or usage. Search/list require available provider-language chapters and safe/suggestive titles. Original localized title/alt-title/summary fallbacks and actual per-title ratings remain. Complete owned feed pagination, UUID/language/total/duplicate checks and whole-operation bounds remain; external, unavailable and zero-page chapters stay omitted. Null/fraction/zero chapter and volume metadata are not replaced with ordinal guesses.

`thumbnailQuality_zh` follows fixed Keiyoushi 949dea252eb8e867b7b40834a26e13088764ff55: original (default Host value `original`, encoding upstream empty suffix), medium `.512.jpg`, low `.256.jpg`. List/search/detail append only the selected published suffix to the actual cover filename. Missing/unknown settings preserve original covers.

`dataSaverV5_zh` defaults false. Only explicit true selects actual `chapter.dataSaver` filenames and `/data-saver/`; otherwise original ordered `chapter.data` and `/data/` remain. get_pages still validates the complete owned feed and declared page count; stable page IDs carry the original chapter UUID/index. Every image request still makes its existing fresh normal at-home GET and resolves the selected quality by index. Both selected and original file sets retain safe filename, capacity and duplicate guards; no filename is guessed.

Image HTTPS hosts remain uploads.mangadex.org or the actual descriptor-provided mangadex.network subdomain. Cover requests remain bound to the same manga UUID. No tokens, Cookie handling, statistics POST, auth/retry fallback, new host grants or App/SDK/style changes. Standard build: `bash build.sh --source mangadex-zh-hans`. Runtime/crate/package versions align at 0.1.1.
