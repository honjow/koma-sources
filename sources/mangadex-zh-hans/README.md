# MangaDex (Simplified Chinese)

Independent org.mangadex.zh-hans.koma0.1.0, API0.2. Language zh-Hans, public API zh. English source and IDs untouched. Usage unknown.

关注较多 is all-time followedCount descending, not primary30-day popularity or usage ranking. Listing/search explicitly require available Chinese chapters and safe/suggestive ratings. Search/title query is ordinary public GET. Details retain actual per-book ratings. Catalogue loops all100-item feed pages, checks UUID ownership/language/pagination/duplicates/total and fails wholly on missing data/capacity. External/unavailable/zero-page chapters are omitted; nullable chapter/volume numbers are not ordinal guesses.

Pages use original ordered chapter.data files. get_pages checks the complete owned zh feed before at-home; stable pageID carries actual chapterUUID/index. Each imageRequest makes a fresh normal at-homeGET, resolving the current indexed original URL; this replaces the primary5min source-clock/cache with one explicit descriptorGET per image. No account tokens/cookies/statistics POST/cache/retry; no App/SDK change. This increases public descriptor traffic; the named chapter download below is accepted, while general traffic/limit behavior remains unproven.

Allowed image HTTPS hosts: uploads.mangadex.org or descriptor-provided actual subdomain of mangadex.network. Full JSON/UTF8,1MiB response/input/output bounds; errors never silently return partial chapters. Official original icon retains its published deep-color background. The finite named native acceptance below does not close whole-source acceptance.


## Finite device acceptance — 2026-10-12

On device 197, ordinary subscription installation of 0.1.0 matched the frozen package, WASM, manifest and official icon hashes. The all-time followed listing displayed 20 actual entries with covers; the ordinary `Love Bullet` search displayed two entries with covers. The named book `cb1b1da5-9436-49f5-8a6b-a1689fb86d91` displayed its complete 21-item Chinese directory.

For chapter `a6daac4d-f4bd-4102-84f3-f8b78cf95fc6` (DTO title `爱上恋爱`), ordinary reading displayed original pages 1/33 and 2/33 with matching HUD and saved index 1. Ordinary download completed 33/33; all 33 downloaded PNG entities were decoded and their sizes, content hashes and manifest integrity matched, totalling 38,829,379 image bytes. After a process-death cold start without clearing data (PID 32723 → 44881), seven state files were byte-identical and ordinary Library Continue returned to the same page 2. Original Library/queue/registry objects and order, other Reader collections, three subscriptions and Reader preferences were preserved; only the temporary test index was removed through the GUI.

Authoritative receipt: `/Users/honjow/git/Koma-production-20261010/.hermes-artifacts/20261005-production/mangadex-zh-hans-20261012/native040-finite-acceptance.json`, SHA256 `fc9d9274a173369dde22ea11d99e369320eae61e789950cd5995e1b53de07267`. Installed package SHA256 `1ead24f19f3d77fb93228767da8177d2dbde99663cf7d40c0104bf7c3e3ce6f4`; WASM `b259046f6e2b1838ae92c4762a195858b8666dae5e141308806e46d2841abb7a`.

Usage remains unknown. Other listing/search pages, books and chapters remain OPEN. This does not claim all 33 pages were visually reviewed, an uncached transition, airplane-mode offline operation, a traditional-Chinese package, whole-source acceptance or completion of Koma's first production version.
