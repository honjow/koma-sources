# ZaiManhua source identity asset

The packaged `icon.png` is a 512 x 512 render of the source-referenced cubic
reconstruction in `artwork/identity.svg`. This is a derived redraw made after
the user's low-resolution icon feedback and explicit request for 2D redrawing;
it is not an official SVG or a recovered high-resolution master.

Reference: the existing official favicon conversion (64 x 64 PNG SHA-256
`c6bd82f469c6a94dda39d41de4c967116d9723bb44c30ea867d1ba980d95009e`).
The current official mobile HTML again explicitly declares
`https://m.zaimanhua.com/assets/favicon-BL04EiLw.ico`; the 2026-10-18 response
is an ICO with only a 64 x 64 frame, SHA-256
`6efa6b120a48e291e3b453c2e2f1ea759bbe5752963e812e94972272e77e59c6`.
The separately declared header wordmark was inspected and not substituted.

The reconstruction retains the original three colors and silhouette. Cubic
curves regularize subpixel raster contour noise; no new backing, shadow,
font, text, extra symbol or mask was added. The source's ambiguous subpixel
edge geometry is reconstructed, not claimed pixel-identical. Original colors
are RGB (173,199,125), (77,142,118), (12,90,100). SVG renders through sharp to
a transparent PNG; packaged PNG SHA-256:
`698c361d3bbca4f9ca352f599c4dfa6745b35523e25deabe64eb20f30a0d2530`.

Use this asset only for ZaiManhua source identity; never as a manga cover or another source's brand.

Device 237: ordinary 0.1.4 local icon-only update and source-management list
appearance reviewed. Current installed archive/manifest/WASM/icon hashes
match the candidate, while manifest and WASM bytes retain the previous
standard package. Library, Session and Queue bytes remain unchanged; other
43 source objects keep all fields. Ordinary import moves the updated source
to the tail. Original library, PID, power and owned-file cleanup verified.
No App dimensions, radius, layout or interpolation change. Native proof:
`/private/tmp/koma-icon-current-audit-20261018/hd-update/zaimanhua-native-receipt.json`.
Other consumers, dark mode, cold start and source-content quality are outside
this finite icon acceptance.
