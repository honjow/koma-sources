# MangaDex 0.1.13

Independent source `org.mangadex.koma`, API 0.2, language en. Existing `mdx:` manga IDs, `ch:` chapter identities, original official icon and [asset attribution](ASSET_SOURCES.md) remain. Ordinary public search/list/detail/Home, exact chapter restoration and existing filters remain; no external provider body is followed.

Settings retain `language` (default en), `dataSaver` (default false) and `defaultContentRating` (default suggestive). Explicit request filters retain their existing precedence. `thumbnailQuality_en` follows fixed Keiyoushi 949dea252eb8e867b7b40834a26e13088764ff55: original, medium `.512.jpg`, low `.256.jpg`. Nonempty Host value `original` encodes the upstream empty suffix. Missing/unknown quality leaves original URLs unchanged. The selected published suffix is consumed by search, list, detail and all Home sections; existing missing-cover handling is unchanged.

Chapter IDs, title fallbacks and original directory order remain. Null/missing/empty/nonstring chapter numbers remain unknown; actual zero and fractional numbers retain their values. No dates are inferred. The existing mdxc1 pagination state preserves full external-only terminal detection; empty catalogues, hosted/external mixed results, pagination errors and unsupported external-only results remain distinct.

Existing data-saver pages use the actual selected API filename array and corresponding data/data-saver path, retaining page index IDs. No account/token/Cookie flow, statistics side effect, new source/host grant, SDK or App style is introduced. Standard build: `bash build.sh --source mangadex`. Runtime/crate/package versions align at 0.1.13.
