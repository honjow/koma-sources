# 官方图标来源与核查

官方原件URL：https://namicomi.com/pl（官网SVG/favicon，NamiComi为首页img[alt=NamiComi]内嵌正式SVG）。原件viewBox `0 0 2000 2000`，原始文件 `dist/wave-20/evidence/assets/namicomi-pl-original.svg`，SHA-256 `174263901610820502049a384355bc1ff5b2f710095490e1401caf108df87a6c`。

权利归NamiComi。继承已验收sources/namicomi-ja/ASSET_SOURCES.md记载的官网原件；本次独立抓取本语种首页复核，SVG SHA与原件174263901610820502049a384355bc1ff5b2f710095490e1401caf108df87a6c一致。完整官网HTML及下载收据保留于dist/wave-20/evidence。

制作：独立抓取本语种官网HTML内嵌SVG（data:image/svg+xml百分号编码），解码后与已验收NamiComi官方原件逐字节一致；保留2000×2000 viewBox、蓝色双弧路径及透明区域，无裁切、额外底板、蒙版或阴影。 CairoSVG按2048×2048栅格化，再用Pillow RGBA LANCZOS缩至512×512。成品完整四角，保留官方主图形与配色，不生成假标识，不以封面替代，不拉伸低清原图。制作稿assets/icon.svg SHA-256 `174263901610820502049a384355bc1ff5b2f710095490e1401caf108df87a6c`，成品icon.png SHA-256 `b6dc49a97b999f965843245379ae26d9b9b28a96fa841eb14b2f052d42144a11`。

验收链：实际打开官网原件、制作稿，再从最终可安装.koma解出icon.png核查同一字节；以128/64/32像素在#F4F4F5和#18181B双底审看身份、字形、边界与透明度。复核拼图：`/Users/honjow/.gemini/antigravity/brain/cecc3fcb-d070-43ba-8d77-3593835b7ffc/scratch/wave20_pkg_icons_review.png`。尺寸/哈希只辅助实图核查，实际审图结果记入dist/wave-20/verification.json。未操作设备；App书源列表真实呈现待后续真机QA，不以拼图代替真机验收。
