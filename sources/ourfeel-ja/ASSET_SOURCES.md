# 官方品牌素材与验收

2026-11-28 从官网 https://ourfeel.jp/ 的 link[rel=apple-touch-icon][sizes="180x180"] 直接提取方形正式标识。

- 官方原件 URL：https://ourfeel.jp/_next/static/images/apple-touch-icon-180.24a3137b.png
- 原件像素：180×180；SHA-256 `3b14935272451b4a75cb0f3a83748990c55716ce85fe85b2f198514aca802190`。默认57像素版本和favicon只作核对，未直接拉伸生产。
- 官网当前SVG字标：https://ourfeel.jp/_next/static/media/logo_of.cb1dd2bf.svg；SHA-256 `219e402eb40ff7798a706c1374e814aaf58d6a9c516163356e4175fe1b7569ca`，用于核对正式品牌身份与字形。方形标识的字形、排版及原有画面配色以官方180像素图为准，不将横向字标另排成假图标。
- 制作：从原件抗锯齿边缘恢复连续轮廓，再用potracer 0.0.4拟合闭合Bézier路径，turdsize=0、alphamax=1、opttolerance=.01。保留全部字形孔洞、位置和#f6f0ef底色／#000000图案，不新增或裁掉品牌笔画。直接二值描摹曾出现多边形边缘，实图核查后已淘汰；当前矢量稿是连续边缘恢复稿。
- 可复现处理稿 assets/icon.svg：SHA-256 `23870d095f3320face451ede830f0a2e0a046d5f3b494080b24c6a002ac13032`。CairoSVG先渲染2048×2048，再由Pillow LANCZOS输出512×512 RGBA icon.png，SHA-256 `c72065c737a2c00bea96ada89354cd46ecacf64c7364fd95bc93476484c7a521`。原尺寸轮廓掩模IoU为0.989944，仅辅助核对，不替代审图。

已实际打开官网原件和处理稿，核对原形、文字、完整方形边界及留白；原件的整面品牌填色保留，未增加底板、白边、外框、阴影或圆角蒙版。最终必须从.koma解包提取icon.png，与本文件字节一致，再打开浅底 #F4F4F5 和深底 #18181B 的128/64/32拼图复核。路径：`/Users/honjow/.gemini/antigravity/brain/cecc3fcb-d070-43ba-8d77-3593835b7ffc/scratch/wave18_pkg_icons_review.png`。

官网HTML、原PNG、SVG字标、制作脚本与前后核对证据保留在dist/wave-18/evidence/assets/；原件SHA见其中receipts。App书源列表的真实呈现待后续真机QA，本轮不操作设备。品牌及商标权归原站权利人，代码LICENSE不授予素材商标权。
