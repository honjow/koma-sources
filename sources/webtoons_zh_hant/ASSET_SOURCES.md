# WEBTOON source identity icon

The official Traditional Chinese homepage at https://www.webtoons.com/zh-hant/
uses `.header_left .btn_logo:after`. Its current stylesheet declares
https://webtoons-static.pstatic.net/image/static/pc/sprite/sp_webtoon_svg_557ff460.svg
as the logo sprite, with an 80 by 80 viewport at x=1688, y=969.

`icon-original.svg` extracts the exact `ico_logo_header` path and preserves
its geometry and #00DC64 color. Its viewport reproduces that website window.
`icon.png` directly rasterizes this vector to a transparent 192 by 192 PNG.
No launcher plate, shadow, rounded mask, or new brand artwork is added.
The app owns display dimensions and rounded clipping.

This is website-published WEBTOON brand artwork used only for source identity.
The implementation license does not grant ownership of the brand or its art.
The public evidence receipts and exact path binding are preserved in the
local icon-only candidate artifacts; no account or user data is included.
