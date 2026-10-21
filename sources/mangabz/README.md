# MangaBz 0.1.6 canonical integration

The 0.1.1/0.1.2 paragraphs below are retained historical evidence, not current whole-source acceptance.

Keeps `com.mangabz.koma`, `manga:<id>bz`, `chapter:<id>bz:<cid>:<pageCount>` and page IDs unchanged. Corrects listings response, uses numeric page cursors for real public paging, and removes the silent 600-chapter cap. Unknown/incomplete page lists fail explicitly instead of returning partial success. Home, filters and settings remain unsupported: old static descriptions/empty arrays did not implement those capabilities. No account, JavaScript execution, challenge handling, mirror switching, or release.

Localized names retain the public MangaBz brand in all supplied locales; no invented Chinese brand translation. Public content language remains zh. Actual usage data is unknown.

Reference: Keiyoushi `Mangabz.kt`, source commit `5b95ccc4b6a96fb9a4a28cead59255b0af99276e` (2026-09-30), https://github.com/keiyoushi/extensions-source/blob/5b95ccc4b6a96fb9a4a28cead59255b0af99276e/src/zh/mangabz/src/eu/kanade/tachiyomi/extension/zh/mangabz/Mangabz.kt. This change extends the existing Koma implementation; no upstream Kotlin source copied.

Named actual host evidence: popular pages 1/2 contain 12 distinct items each; latest 12; search 鬼滅 includes manga:73bz; 鬼滅之刃 has 235 public chapters and chapter:73bz:10344:54 resolves all 54 unique original image URLs. Images have not been downloaded or visually accepted here. Root must perform 197 ordinary subscription, browse/search, reader, download and no-data-clear cold continuation; native remains OPEN.

0.1.2 adds only public detail author extraction and synopsis fold-control cleanup. Saved actual 鬼滅之刃 HTML resolves 吾峠呼世晴 and retains all hidden continuation without fold controls. Missing author region emits an empty author array. Normal HTML ASCII whitespace is collapsed inside the synopsis; this does not claim to fix the still-unattributed native literal-backslash rendering. All HTTP, listing/search, catalogue, page and image-request functions are byte-identical to the frozen 0.1.1 source. Native acceptance remains OPEN. The exact 0.1.1 archive/index/WASM/source/review binding is retained under `.hermes-artifacts/mangabz-20261010/accepted-host-0.1.1`.


## Complete reviewed successor

Canonical adopts the complete Root-reviewed0.1.5owner unchanged:0.1.3 card completeness/error propagation;0.1.4 unknown per-chapter language omission;0.1.5 explicit chapter/full-volume ordinals, reading groups and unknown extras. Stable IDs, routes, source order and page metadata preserve the owner contract. Retained detail73 has235 items:205regular,23full volumes,7unknown extras. Existing Koma CURRENT_STATUS/quality-batch receipts carry finite Native acceptance; this integration does not repeat it.

The build preserves current canonical SDK and the existing required16pxicon/ASSET_SOURCES. Source fetch/body decoder does not call SDK decode_json_body_into. Retained full54HTTP image-response originals are missing, so no new complete54-request replay is claimed. Existing real popular/detail bodies and controlled card/ordinal fixtures are replayed offline. Standard compile flags/currentSDK can change the old direct-cargo WASM bytes; the new standard binary is not automatically Native-accepted.

Icon visual clarity remains OPEN. No icon processing, parser redesign, App style, publication, network or device operation is part of this proposal.


## 2026-10-21 device237 named first-two-page observation

Current source and installed version are 0.1.6; commit 8958b46 changed only the source version for the already reviewed identity resource delivery, preserving the 0.1.5 parser. The older 0.1.5 heading and Host-only paragraphs above describe their original evidence boundaries.

On VDE-AL00, unfolded portrait 1320x2120, the ordinary installed-source entry returned 12 popular items. 鬼滅之刃 showed 吾峠呼世晴 and 235/235 chapters. Ordinary 第1話 殘酷（54P） displayed its actual first image at 1/54 and, after one correct next-page action, a traditional-Chinese comic page at 2/54. Natural Back persistence independently bound this observation to comicId=com.mangabz.koma:manga:73bz, chapterId=chapter:73bz:10344:54, pageIndex=1, totalPages=54. Root inspected both original page images. This accepts only those two images and ordinary return for that named version/chapter; it does not accept all 54 pages, downloads, cold continuation or the whole source.

The first left tap was incorrectly named ordinary-next-page in its immutable receipt. It remained at 1/54 because the actual settings were single page, left-to-right, left/right zones and no inversion: left is previous. This is not a page-fetch or Reader defect. The settings were only read, not changed; a single right-side next tap produced the observed 2/54.

Original continuation was 第1卷（190P）, chapter:73bz:271587:190 at pageIndex=1. It was normally reopened at 2/190 and naturally returned; the final reader document and detail both confirm that original continuation. The temporary ascending chapter order was restored to descending, the filter remained all, Browse returned to its original list end, and PowerManager read back AWAKE/OverrideTimeout=10000ms. No source package, index, enabled state, download or cold-start operation was performed. The new named chapter progress remains as the normal result of reading its first two pages; no stored user records were edited or removed.

Raw evidence is under Koma/.hvigor/outputs/user-flow-audit/device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership/: mangabz016-{baseline237-001,detail237-001,single-chapter-locate237-001,first-page237-001,second-page237-001,reader-settings237-001,correct-next237-001,back-progress237-001,original-volume-resume237-001,original-progress-restored237-001,session-restored237-001}. The second-page-named receipt is the unsuccessful previous-page observation described above; correct-next contains the actual second page. Artifact hashes were disabled. The existing 0.1.6 artwork and its separate quality boundary are unchanged.
