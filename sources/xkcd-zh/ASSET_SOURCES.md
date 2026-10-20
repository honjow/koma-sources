# xkcd official account identity resource

Version 0.1.4 uses the complete 1000×1000 WebP avatar currently declared by
Randall Munroe's official xkcd.com Bluesky account. Provenance observed 2026-10-20:

- https://xkcd.com/ directly links https://store.xkcd.com/ in #xkcdNews.
- The store Organization.sameAs explicitly declares https://bsky.app/profile/xkcd.com.
- That profile and its anonymous public profile response agree on handle xkcd.com,
  displayName Randall Munroe and did:plc:cz73r7iyiqn26upot4jtjdhk.
- The public profile declares the exact avatar URI:
  https://cdn.bsky.app/img/avatar/plain/did:plc:cz73r7iyiqn26upot4jtjdhk/bafkreibandvfr3i2qpwbuzfwcw26px2wfh2vkrgtbr2neiof4muzca2h2i
- The original response is HTTP200, 12440 bytes, WebP, 1000×1000.

This is the author's official account avatar, not a claimed original publication
logo or vector. icon.webp retains the complete source image and its original
white canvas without cropping, tracing, recoloring, shadow or added backing/mask.
The separately found official shop header logo is a 400×356 transparent black
XKCD/five-character mark; it is not the active asset because dark slots need
contrast without inventing a background for that file.

This replaces the displayed derived two-person asset from 0.1.3. The prior
extension PNG, officially declared favicon and derived SVG remain historical
source material; the rejected direct-ICO/imagegen candidates are not restored.
App slot size, rounded presentation, ordering and all provider behavior remain.
Brand/art rights remain with their holders; no affiliation is asserted.
