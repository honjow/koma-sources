# Comic Fury (Chinese) source

Stable identity `com.comicfury.zh.koma`, version `0.1.6`, language `zh`, canonical origin `https://comicfury.com`. MIXED remains conservatively mapped to `nsfw=true`; this does not label every comic as adult. Source behavior follows the retained Keiyoushi ComicFury implementation and the fixed settings reference `949dea252eb8e867b7b40834a26e13088764ff55`.

## Public behavior

Popular, search and latest retain the published four content flag defaults at zero and Chinese language. The reference tags, sorting, last-updated, completed and four content filters are exposed through the existing host filter contract; explicit selections are forwarded to the official search form. Sort has no universal schema default: absent selection retains search=0, popular=1 and latest=2, so host default injection cannot change the adopted listing behavior. The explicit official empty-search marker returns an empty successful result; an unexplained empty card list remains an error. Catalogues retain primary order reversal, chapter identities and explicit group episode numbers; covers/extras keep null chapter numbers.

Reader images belong only to the exact first `div.is--comic-page` matching the requested chapter. Preloaded neighbouring chapters are excluded. A server response for another comic remains an owner error. The existing image URLs, IDs and order are unchanged with no settings.

## Settings

`showAuthorsNotes` defaults to `false`. It is shown and consumed only when the host explicitly advertises `text_page_image_v1`; older hosts return no setting and receive only original URL pages even if a saved value is true. When enabled, each author-note box under the current comic appends one text page after the original images. Reader comments and notes from other comics are excluded.

The source converts note HTML to plain Unicode: explicit BRs retain each line break, blocks provide the minimum paragraph gap, and the existing HTML parser decodes entities once. Text pages use the existing `webtoon-author-notes` template, whose layout matches the adopted text interceptor; the host prepares a local PNG through the original Reader image/cache pipeline. No synthetic network URL is emitted.

## Retained boundaries and delivery

No warning-confirmation POST, cookies, login, custom-host layout fallback, mirror selection or account configuration is added. Official SVG default-cover URLs remain genuine URLs; support is owned by the host. The complete official 192px icon and its original green field remain unchanged.

Build with `bash build.sh --source comicfury-zh`. Build and parser checks do not establish Native image rendering or full provider availability. Code and asset attribution remain in `NOTICE.md` and `ASSET_SOURCES.md`.

## Filter contract (0.1.6)

The fixed reference `949dea252eb8e867b7b40834a26e13088764ff55` defines tags, sort, last update, completed, violence, nudity, language and sexual-content selections. The source now exposes these through `get_filters` and consumes `args.filters` for search and listings. To preserve the already accepted Koma results, all four content flags default to `0` even though the reference defaults to `2`; changing them requires an explicit selection. Last-update defaults to `0`, completed false maps to the site's `completed=1`, and tags are UTF-8 URL encoded after the reference comma-space normalization. Unknown choices or malformed values fail before HTTP. No account, cookies, warning confirmation or UI style changes are introduced.

Official same-header Chinese popular requests returned 12 cards with 0/0/0/0 and 19 with 2/2/2/2. Production candidate and original C++ Host replay of retained real HTML verified old versus new exact request URL and data equality for popular, latest and search with host-injected defaults; explicit choices and invalid inputs were checked separately, with live candidate popular calls returning the respective 12 and 19 cards. These are source/desktop results, not Native filter persistence or acceptance.
