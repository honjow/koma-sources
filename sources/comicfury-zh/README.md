# Comic Fury (Chinese) source

Stable identity `com.comicfury.zh.koma`, version `0.1.5`, language `zh`, canonical origin `https://comicfury.com`. MIXED remains conservatively mapped to `nsfw=true`; this does not label every comic as adult. Source behavior follows the retained Keiyoushi ComicFury implementation and the fixed settings reference `949dea252eb8e867b7b40834a26e13088764ff55`.

## Public behavior

Popular, search and latest retain the published four flag filters set to zero and Chinese language. The explicit official empty-search marker returns an empty successful result; an unexplained empty card list remains an error. Catalogues retain primary order reversal, chapter identities and explicit group episode numbers; covers/extras keep null chapter numbers.

Reader images belong only to the exact first `div.is--comic-page` matching the requested chapter. Preloaded neighbouring chapters are excluded. A server response for another comic remains an owner error. The existing image URLs, IDs and order are unchanged with no settings.

## Settings

`showAuthorsNotes` defaults to `false`. It is shown and consumed only when the host explicitly advertises `text_page_image_v1`; older hosts return no setting and receive only original URL pages even if a saved value is true. When enabled, each author-note box under the current comic appends one text page after the original images. Reader comments and notes from other comics are excluded.

The source converts note HTML to plain Unicode: explicit BRs retain each line break, blocks provide the minimum paragraph gap, and the existing HTML parser decodes entities once. Text pages use the existing `webtoon-author-notes` template, whose layout matches the adopted text interceptor; the host prepares a local PNG through the original Reader image/cache pipeline. No synthetic network URL is emitted.

## Retained boundaries and delivery

No warning-confirmation POST, cookies, login, custom-host layout fallback, mirror selection or account configuration is added. Official SVG default-cover URLs remain genuine URLs; support is owned by the host. The complete official 192px icon and its original green field remain unchanged.

Build with `bash build.sh --source comicfury-zh`. Build and parser checks do not establish Native image rendering or full provider availability. Code and asset attribution remain in `NOTICE.md` and `ASSET_SOURCES.md`.
