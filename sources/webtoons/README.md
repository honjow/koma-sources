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

章节标题中的 Unicode 转义遵循 [SDK 公共文本解码合同](../../sdk/README.md)，合法表情不使整份章节响应失效。

## 独立列表候选

固定参考 `949dea252eb8e867b7b40834a26e13088764ff55` 的热门依次请求 trending、popular、originals、canvas 四个榜单，最新按宿主本地星期请求 `/en/originals/{weekday}?sortOrder=UPDATE` 并单页结束；旧宿主没有时钟特性时仅显示热门。2026-11-06 四个榜单官网均 HTTP 200，实际 WASM 与 HTML 桥各解析 30 项，第四页终止；同日星期五最新 162 项、无下一页。热门与最新为独立栏目。支持空词筛选的宿主显示参考的 ALL、Originals、Canvas 搜索分类；旧宿主不显示新筛选。英文官网 love 查询的两类前两页各30项、空关键词 Originals 为空并终止；Canvas 第10页下一按钮指向11，空格关键词 high school 下一页链接也已用实际 WASM 回放。默认 ALL 与正式源在同一原页逐字段一致。既有目录对照回放通过；普通实机和正式发布未验。
