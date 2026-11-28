# 官方图标来源与验收

采集日期 2026-11-28。官网 https://namicomi.com/it 的 img[alt=NamiComi] 内嵌 data:image/svg+xml，独立抓取并解码原件；viewBox 0 0 2000 2000，保留蓝色双弧和透明镂空。

保存的官方矢量原件/独立提取稿为 assets/icon.svg，SHA-256 `174263901610820502049a384355bc1ff5b2f710095490e1401caf108df87a6c`。官网原件与 HTML/CSS/精灵取证保留在 dist/wave-16/evidence/assets/。

先打开官方原件预览核对品牌，再使用 CairoSVG 渲染2048×2048，Pillow LANCZOS 缩至512×512 RGBA；icon.png SHA-256 `b6dc49a97b999f965843245379ae26d9b9b28a96fa841eb14b2f052d42144a11`。未添加外框、底板、圆角蒙版或阴影，未改变原形、颜色或裁断边界。文字镂空、弧形和气泡属于官方图案。

依次实际打开官方原件、处理稿和最终 .koma 包内提取实物，核对真实身份、画布、边界及留白；包内PNG和源成品逐字节一致。128/64/32像素浅底 #F4F4F5 与深底 #18181B 拼图在 `/Users/honjow/.gemini/antigravity/brain/cecc3fcb-d070-43ba-8d77-3593835b7ffc/scratch/wave16_pkg_icons_review.png`。尺寸与哈希只辅助实图核查。

本轮不操作设备，App书源列表呈现留待后续真机QA；拼图不代表真机验收。官方品牌和素材权利归各平台，代码许可证不转授其所有权。
