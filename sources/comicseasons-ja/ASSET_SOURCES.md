# 官方标识来源与制作核查

版权与正式品牌归属：**文藝春秋 / Bungeishunju Ltd.**。已直接核对官网首页页脚版权；原始 HTML 与 SHA-256 见 dist/wave-19/evidence/intake.json。

- 官方 apple-touch-icon：https://comic-seasons.com/_next/static/images/apple-touch-icon-180.2905b264.png
- 原件尺寸：180×180；SHA-256：`6a9b8f90a82d543cb9468440b54efb8f70b8bfbd26549c8f7a8ca4e58ceebea8`。
- 优先使用的官方高清 SVG：https://comic-seasons.com/（首页内嵌 SVG 或官网 CSS 引用的正式 SVG）；原始 viewBox：`0 0 286 50`。
- 原始 SVG 保存于 `dist/wave-19/evidence/comicseasons-official-logo.svg`；SHA-256：`1e6d032522bb65a3d1ac7f1ec5a170362e00913bcdebfd8589da520907204901`。
- 制作：完整保留官方 SVG 路径与字形。Seasons 的 CSS 颜色变量取官方 touch 图标的粉色；其它保留原填色。只裁去矢量视口本身的空边，不裁字、不改长宽比；完整字标在官方原有白色正方形画面内居中，宽480像素，两侧各16像素防止抗锯齿切边。字标本来是横向排版，上下留白属于保持原形后的必然比例，不添加外框或额外底板。使用 CairoSVG 2048像素栅格化后以 Pillow LANCZOS 得到512×512，四角完整不透明，无圆角蒙版和阴影。
- 制作稿 `assets/icon.svg` SHA-256：`e9d64bccc823ba397bfca72dbb3f8c3cc59acf97d0edba5aec0a99defbe26a89`；成品 `icon.png` SHA-256：`cdcf4cc564e29f77ce020d3df70dec1986a23d8111fc9998c43fa0b290845f00`。
- 已实际打开官方 touch 原件、官方 SVG 渲染图与制作稿；最终 .koma 内实际 icon.png 需再提取核对同一哈希，并在128/64/32像素浅底 #F4F4F5、深底 #18181B 拼图审看。复核图路径：`/Users/honjow/.gemini/antigravity/brain/cecc3fcb-d070-43ba-8d77-3593835b7ffc/scratch/wave19_pkg_icons_review.png`。
- 本轮仅主机候选包制作与实网验证，不操作设备；App 书源列表实际呈现待后续真机 QA，不以尺寸/哈希替代该项验收。
