# Identity asset

The official WNACG pages embed the actual `紳士漫畫` logo as an inline PNG
inside their home-link `.logo` element. The original 130×40 PNG has SHA-256
`469ad5cafd24f59611ac05f9bb1cbfb916ca6004758a07b84fc7e1e798c761c4`.
The official `https://www.wnacg.buzz/themes/weitu/images/style.css?=23.11.26`
sets its header background to `#334348`. The packaged 130×130 square preserves
the original text pixels and antialiasing, crops only transparent outer pixels,
and centers it on the official header color; no rounded corners, border, shadow,
recoloring or redrawing. SHA-256
`bfcf0eb2213d7316af55a6cedbb7855c930456774337018acd5e756a4f7dd14c`.
The earlier unmodified 16px website favicon was a generic camera and is not used
as the source identity.

## 0.1.6 候选：官网原字标分两行

官网原始 130×40 字标的四个字之间存在透明间隙；在第二、三字间裁分，将原像素等比例放大并居中排成两行，保留官网头部背景色 `#334348`，没有重绘字形。官网原图 SHA-256 `469ad5cafd24f59611ac05f9bb1cbfb916ca6004758a07b84fc7e1e798c761c4`，候选 SHA-256 `85ef45ce54a40f40737d81d8aaf61ac69449440dcd6fc94c22169408f3233cf5`。浅、深背景离线对照见 Koma `.hvigor/outputs/source-icon-audit-20261112/candidates/icon-light-dark-review.png`；候选图标已从最终包中读回，并在 237 浅色书源管理列表核对版本和显示；深色设备列表及浏览首页未验，未发布。
