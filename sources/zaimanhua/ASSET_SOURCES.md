# ZaiManhua package assets

`icon.png` is the original 64 x 64 PNG conversion of the official mobile
favicon. SHA-256: `c6bd82f469c6a94dda39d41de4c967116d9723bb44c30ea867d1ba980d95009e`.
Source: `https://m.zaimanhua.com/assets/favicon-BL04EiLw.ico`.
It is low resolution; no claim of high-resolution restoration is made.

## Rejected redraw and recovery

The 512px redraw added in a47543d and delivered locally with version 0.1.5
was rejected after the user's 2026-10-18 jagged-curve feedback. Its hundreds
of short cubic segments retained raster sampling/blur artifacts instead of
recovering the intended smooth geometry. The earlier visual-acceptance
claim is withdrawn; package-hash and data-preservation evidence does not
establish acceptable artwork quality. The defective SVG is removed from
the source tree and remains available in Git history and local evidence.

This correction restores only the original icon. Source ID, manifest,
WASM behavior and current version remain unchanged. No new backing,
shadow, mask, App dimensions, layout, corner radius or interpolation.
The separate MangaBZ and ComicABC official originals are unaffected.

A fresh geometry candidate must distinguish true contours from sampling
uncertainty and be reviewed at native and enlarged sizes before adoption.
The temporary hand-adjusted draft was also rejected and never packaged.
Investigation and retained candidates:
`/private/tmp/koma-icon-geometry-correction-20261018/REPORT.md`.
Device237 ordinary local import restored these original bytes at0.1.5.
All four installed entities match; current manifest/WASM are unchanged.
Library, Session and Queue hashes are unchanged, and other43source objects
keep all fields and relative order; ordinary import moves ZaiManhua to the
tail. Original library/PID/power and owned-file cleanup were verified.
Recovery receipt: `/private/tmp/koma-icon-geometry-correction-20261018/native-receipt.json`.
This is a recovery of identity fidelity;64px clarity remains unresolved.
