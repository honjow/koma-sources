# 官方图标来源与验收

2026-11-28 独立抓取官网 https://namicomi.com/de，提取 `img[alt="NamiComi"]` 内嵌 `data:image/svg+xml` 官方品牌原件。原件为 viewBox 0 0 2000 2000 的蓝色双弧，独立保存于 assets/icon.svg，SHA-256 `174263901610820502049a384355bc1ff5b2f710095490e1401caf108df87a6c`。官网HTML、解码SVG、官方原件预览及网络收据保存在 dist/wave-17/evidence/preflight/de/。

先实际打开官方原件核对身份、透明区域和完整边界，再以 CairoSVG 渲染2048×2048，Pillow RGBA LANCZOS 缩至512×512。成品 icon.png SHA-256 `b6dc49a97b999f965843245379ae26d9b9b28a96fa841eb14b2f052d42144a11`。保留官方原形与蓝色色彩，无裁切、额外底板、外框、圆角蒙版或阴影；双弧轮廓属于原图案。

验收按官方原件 → 处理稿 → 最终可安装包内实物依次实际打开核查。包内提取PNG需与源成品逐字节一致；使用Python/Pillow生成128/64/32像素浅底 #F4F4F5、深底 #18181B 拼图，路径 `/Users/honjow/.gemini/antigravity/brain/cecc3fcb-d070-43ba-8d77-3593835b7ffc/scratch/wave17_pkg_icons_review.png`。尺寸和哈希仅辅助实图核查，最终审图证据写入verification.json。

本轮不操作设备，App书源列表中的真实呈现等待后续真机QA；离线拼图不代表真机验收。图案及品牌权利归NamiComi，代码Apache-2.0许可证不转授品牌或素材所有权。
