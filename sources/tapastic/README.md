# Tapas

Independent Rust/WASM port of `en.tapastic` v1.6.6, pinned to Keiyoushi
`fc7a827769f612dfa6dccb51e819f5b4eecaeab5`. The reference is Apache-2.0 and
uses the public English Tapas comics catalog. Official origin is tapas.io.
The reference is MIXED and there is no age classification for every work.

| Koma operation | Reference behavior |
|---|---|
| `get_listings`, `get_manga_list` | Popular (ranking) and Latest (genre, newest episode) use the same COMIC category, subtab, order, 25 entries and zero-based site page index. Pagination uses the official `last` field. |
| `search` | Original comic search HTML, artwork, synopsis and pagination. Exact Tapas series links resolve numeric IDs or the series ID published in page metadata. |
| `get_manga` | Original series info title, cover, synopsis, publisher credits, authors, genres and schedule status. |
| `get_chapters` | Fetches the full newest-first episode listing with its original pagination and one timestamp per operation. Keeps original scene number, publication time, paywall and scheduling state. Settings-based exclusions use the existing Koma chapter contract. |
| `get_pages`, `get_image_request` | Exact episode ownership and original page order, including original repeated positions. Preserves the original image Referer and UA. The optional author notes use the existing local text-page renderer. A confirmed lock keeps the paid status; a novel or absent body has a separate outcome. |
| `get_settings` | The four reference options keep their effective defaults: show paywalled episodes, show scheduled episodes, and show author's notes on, prefix the number off. The episode's ID stays stable when its displayed title changes. |
| `get_authentication` | The real site’s `/account/signin` opens in Koma’s explicit source web session. The extension’s hard-coded birthday is not used. Public requests remain anonymous and account and age checks remain with Tapas. This source does not purchase or unlock episodes. |

Koma 1.2.0 supplies the clock, per-source web authentication, local text pages,
and the chapter-exclusion contract. The text template uses plain Unicode notes
with the reference heading. An account's unlocked content depends on Tapas
authorization. This checkout used real anonymous free and locked pages;
no personal sign-in was submitted.
