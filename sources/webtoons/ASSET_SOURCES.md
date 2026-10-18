# WEBTOON source identity icon

The official Traditional Chinese homepage at https://www.webtoons.com/zh-hant/
uses `.header_left .btn_logo:after`. Its current stylesheet declares
https://webtoons-static.pstatic.net/image/static/pc/sprite/sp_webtoon_svg_557ff460.svg
as the logo sprite, with an 80 by 80 viewport at x=1688, y=969.

`icon-original.svg` extracts the exact `ico_logo_header` path and preserves
its geometry and #00DC64 color. Its viewport reproduces that website window.
`icon.png` directly rasterizes this vector to a transparent 192 by 192 PNG.
No launcher plate, shadow, rounded mask, or new brand artwork is added.
The App controls display dimensions; it must not add a mask.

This is website-published WEBTOON brand artwork used only for source identity.
The implementation license does not grant ownership of the brand or its art.
The public evidence receipts and exact path binding are preserved in the
local icon-only candidate artifacts; no account or user data is included.

## 当前审查中的图标资源（2026-10-17）

- 官方素材URL：https://webtoons-static.pstatic.net/image/static/pc/sprite/sp_webtoon_svg_557ff460.svg
- 页面声明/归属链：https://www.webtoons.com/zh-hant/ stylesheet .header_left .btn_logo:after引用官方sprite，80×80 viewport x=1688,y=969，原ico_logo_header path提取保持几何与#00DC64，再直接栅格成透明192PNG；当前仅保原SVG提取件与历史整sprite回执，完整sprite原体缺失，不能冒本轮独立重建链
- 实际原件：`/Users/honjow/git/Koma-sources-webtoons-zh-hant-20261010/sources/webtoons_zh_hant/icon.png`
- 实际打包资源SHA256：`7746c706aae41368182ce6621e1e3dd00d51a609f8d81d45722ef2a7ec93f39b`
- 选择理由与边界：同WEBTOON官方品牌，不借繁中WASM/manifest能力；替换英文移植外圆板/阴影，32vp仍未验。
- 已审候选：`/private/tmp/koma-webtoon013-icon-original-20261017/candidates/webtoons/webtoons-0.1.3-icon-r2.koma`。历史旧素材/包均保留，来源不以临时路径代替官方URL。
- 标准构建携带本资源与原metadata，但既有标准编译flags可改变WASM字节；不保证重编WASM与冻结包字节相同。旧WASM仅封包等价不构成完整可复现编译。
