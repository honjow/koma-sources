# 官方标识来源与制作核查

版权与正式品牌归属：**少年画報社 / Shonengahosha**。已直接核对官网首页页脚版权；原始 HTML 与 SHA-256 见 dist/wave-19/evidence/intake.json。

- 官方 apple-touch-icon：https://comic-y-ours.com/images/apple-touch-icon-180.png
- 原件尺寸：180×180；SHA-256：`44e36b986022f7c7e82f0fa0fbb50ba1399e1bd12884ff3c0bc99ddf7925052d`。
- 优先使用的官方高清 SVG：https://comic-y-ours.com/（首页内嵌 SVG 或官网 CSS 引用的正式 SVG）；原始 viewBox：`0 0 220 80`。
- 原始 SVG 保存于 `dist/wave-19/evidence/comicyours-official-logo.svg`；SHA-256：`81dfb4e6ea71047e3ddbd70b8b928fc293ab177ade33f962f8819036a65c697d`。
- 制作：完整保留官方 SVG 路径与字形。Seasons 的 CSS 颜色变量取官方 touch 图标的粉色；其它保留原填色。只裁去矢量视口本身的空边，不裁字、不改长宽比；完整字标在官方原有白色正方形画面内居中，宽480像素，两侧各16像素防止抗锯齿切边。字标本来是横向排版，上下留白属于保持原形后的必然比例，不添加外框或额外底板。使用 CairoSVG 2048像素栅格化后以 Pillow LANCZOS 得到512×512，四角完整不透明，无圆角蒙版和阴影。
- 制作稿 `assets/icon.svg` SHA-256：`1bd8333730579adb9864d870466f1a50827e5e17d22afae34cec49a0fd3a90aa`；成品 `icon.png` SHA-256：`dff61282eda03c2425f8cf7c779e23e77d5f38af2170909ca2151674c1c964e5`。
- 已实际打开官方 touch 原件、官方 SVG 渲染图与制作稿；最终 .koma 内实际 icon.png 需再提取核对同一哈希，并在128/64/32像素浅底 #F4F4F5、深底 #18181B 拼图审看。复核图路径：`/Users/honjow/.gemini/antigravity/brain/cecc3fcb-d070-43ba-8d77-3593835b7ffc/scratch/wave19_pkg_icons_review.png`。
- 本轮仅主机候选包制作与实网验证，不操作设备；App 书源列表实际呈现待后续真机 QA，不以尺寸/哈希替代该项验收。
