# XCOMIC Chinese (com.xcomic.zh.koma)

Independent zh entity, v0.1.0 / API0.2 / canonical XCOMIC. MIXED source package is nsfw; each book uses actual title content_rating_id, fallback comic contentRating, unknown when absent. Browse lacks rating and remains unknown. Usage unknown.

Anonymous published GraphQL queries only on canonical xcomic.me/query/. No login, tokens, mutation, view counter, mirror, JS or image transform. Popular/latest/search preserve published 12-card paging and zh translation filter. Work ID stays title identity; all actual comic_ids are resolved individually, regardless of inaccurate declared total_comics. Only exact zh/public/normal editions contribute chapters. All chapter pages require complete next/total/accumulated-count checks; duplicate IDs, no progress, changed totals or capacity failures fail the whole operation. Same chapter numbers across editions retain distinct work+edition+chapter identities and stable ties. Pages validate complete title ID membership, current target edition identity/public/normal/zh and complete target catalogue to verify chapter ownership, then preserve original direct image URL array order.

Existing SDK remains byte-identical. Source-local ABI exports validate real linear-memory range, checked ptr+len and <1MiB input before SDK helper dereference; independent request buffer avoids aliasing output. Complete JSON/UTF8, host returned length and body/output capacities are bounded. No HTML descriptors are used. Details and catalogue operations with large edition counts make many normal metadata queries; pages avoid unrelated edition I/O and native deadline acceptance remains OPEN.

Read-only actual contract: source title xdj9b/native 错撩 has 39 actual edition IDs vs declared37. All39 actual probe bodies retained; one zh edition mkzqa8/public/normal, complete 2 chapters. Chapter1 5xbbxdl has145 originalURL entries. First3 actual WebP images fetched; images2/3 visually show simplified Chinese dialogue. This proves only named images;142 remaining image bytes, other titles, latest/search network calls and native acceptance remain unproven. Production WASM retained-body replay is separate from live/device evidence.

Materials and exact receipts: /private/tmp/koma-xcomic-readonly-20261011. Build wrapper is local-only and pins borrowed packaging compatibility bytes. No publication or deployment authorization is implied.


## Source-tree recovery boundary

The original implementation and its dated stage notes above are preserved. This recovery uses the current canonical SDK and does not add artwork, standard-package registration or new HTTP/native acceptance. Original named runtime evidence remains separately bound to its original version and package.

## 2026-10-21 official identity follow-up

The canonical public homepage `https://xcomic.me/` was requested once through the available web reader and returned `403 Forbidden`. This request yielded no successful parent HTML or official image declaration. No retry, alternate client/domain, challenge handling or artwork substitution followed. The missing official resource remains OPEN; this homepage result does not classify the independent GraphQL endpoint, the Traditional source, or any reading path as failed. No source code, asset, version or package changed.
