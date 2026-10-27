This independent Rust/WASM source adapts endpoint and page-structure behavior
documented by the Keiyoushi `extensions-source` Webtoons extension:

- Upstream repository: https://github.com/keiyoushi/extensions-source
- Pinned upstream commit: e40e1b78f46baf5ef9dc68cb980f82c66766a211
- Reference files: `src/all/webtoons/src/eu/kanade/tachiyomi/extension/all/webtoons/Webtoons.kt`,
  `Dto.kt`, and `src/all/webtoons/build.gradle.kts`
- Upstream repository license: Apache-2.0 (`LICENSE`)

The Rust implementation is independently written and does not include upstream
Kotlin code or assets. The upstream license applies to upstream code, not to
WEBTOON content, marks, endpoints, or third-party comics. No comics are included.
The separately obtained provider identity image is documented in ASSET_SOURCES.md.

The optional quality, sequential-numbering and creator-note settings also use
Keiyoushi tree 949dea252eb8e867b7b40834a26e13088764ff55 as a behavior reference
(Webtoons.kt and the shared TextInterceptor helper), under Apache-2.0. The Rust
source emits plain text descriptors for the existing host renderer and includes
no upstream Kotlin renderer code.
