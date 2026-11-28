# 官方品牌素材与验收

2026-11-28 从官网 https://feelweb.jp/ 的 link[rel=apple-touch-icon][sizes="180x180"] 直接提取方形正式标识。

- 官方原件 URL：https://cdn.feelweb.jp/images/apple-touch-icon-180.png?1795764624
- 原件像素：180×180；SHA-256 `dc0f2a3a1f6262257ca10349478be3c5a47ce67956337c19c7032f8b4f2b34bf`。默认57像素版本和favicon只作核对，未直接拉伸生产。
- 官网当前SVG字标：https://cdn.feelweb.jp/images/logo.svg；SHA-256 `9f5e27bab62020505c8a9892d6195371ffb28ed28a2f426753a87aad96ea56ea`，用于核对正式品牌身份与字形。方形标识的字形、排版及原有画面配色以官方180像素图为准，不将横向字标另排成假图标。
- 制作：从原件抗锯齿边缘恢复连续轮廓，再用potracer 0.0.4拟合闭合Bézier路径，turdsize=0、alphamax=1、opttolerance=.01。保留全部字形孔洞、位置和#ffffff底色／#000000图案，不新增或裁掉品牌笔画。直接二值描摹曾出现多边形边缘，实图核查后已淘汰；当前矢量稿是连续边缘恢复稿。
- 可复现处理稿 assets/icon.svg：SHA-256 `6dd344239442fae25ca44b73a601e394c236f4e4a0fe92a697cca7807f1e9f07`。CairoSVG先渲染2048×2048，再由Pillow LANCZOS输出512×512 RGBA icon.png，SHA-256 `6309f01ddd53ce5070fd84ff6ccd94074533916afe347e6fdefaebf465661891`。原尺寸轮廓掩模IoU为0.987140，仅辅助核对，不替代审图。

已实际打开官网原件和处理稿，核对原形、文字、完整方形边界及留白；原件的整面品牌填色保留，未增加底板、白边、外框、阴影或圆角蒙版。最终必须从.koma解包提取icon.png，与本文件字节一致，再打开浅底 #F4F4F5 和深底 #18181B 的128/64/32拼图复核。路径：`/Users/honjow/.gemini/antigravity/brain/cecc3fcb-d070-43ba-8d77-3593835b7ffc/scratch/wave18_pkg_icons_review.png`。

官网HTML、原PNG、SVG字标、制作脚本与前后核对证据保留在dist/wave-18/evidence/assets/；原件SHA见其中receipts。App书源列表的真实呈现待后续真机QA，本轮不操作设备。品牌及商标权归原站权利人，代码LICENSE不授予素材商标权。
