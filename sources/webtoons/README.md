# English WEBTOON / CANVAS 0.1.6

Public English search, details, episode catalogues and reader pages. Existing IDs, API catalogue order, image-page order and default output are retained. Requests keep the existing `locale=en` cookie; this source does not accept age/consent or introduce account settings.

## Settings

All settings default to `false` and are read from each request snapshot.

- `showAuthorsNotes`: append the current episode's creator note using the host's `webtoon-author-notes` text-image template. This setting is shown and consumed only with `text_page_image_v1`. HTML entities are decoded by the source DOM once; text whitespace follows Jsoup 1.22.2. Existing locked/MotionToon guards apply before notes.
- `useMaxQuality`: remove only a complete `type=q90` query pair from normalized image URLs. Other parameters, fragments, page IDs and indices remain unchanged.
- `useSequentialNumbering`: request the complete catalogue with the reference's `pageSize=99999`, assign one-based numbers in API order and reject an incomplete response. With the setting off, the original cursor pagination and official-title numbering remain unchanged. The reference reverses its final list; this source preserves its established API order rather than changing the directory contract.

The source keeps its existing response limits and explicit errors. It does not advertise unsupported listings or bypass upstream access restrictions. Text descriptions are never sent to an external image service.

## Implementation source

The independent Rust implementation and provider-asset attribution are documented in NOTICE.md and ASSET_SOURCES.md. The settings behavior additionally follows Keiyoushi `extensions-source` tree `949dea252eb8e867b7b40834a26e13088764ff55`, `src/all/webtoons/.../Webtoons.kt`, with only the host capability and existing directory-order adaptations above.

搜索先解码请求中的 JSON 转义，再按既有站点请求规则编码用户输入；默认、空查询与分页规则保持原行为。
