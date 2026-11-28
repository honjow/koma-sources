# 官方图标来源与验收

采集日期 2026-11-28。官网 https://www.webtoons.com/id/ 当前样式表 https://www.webtoons.com/static/bundle/linewebtoon-b165d7c5b4ac57099614.css 引用官方精灵 https://webtoons-static.pstatic.net/image/static/pc/sprite/sp_webtoon_svg_54232f64.svg，完整精灵 SHA-256 `89c34228492d3d8cdbf551395da980e4c05712e6d8ca9d4b6d957df56db24c99`。独立抽取 ico_logo_header 的准确路径，与当前官方原件逐条核对相等，保留 #00DC64 气泡和透明字形。

保存的官方矢量原件/独立提取稿为 assets/icon.svg，SHA-256 `fe2ae734cc72f05ce570a4a3852c741300089d27d6ed33060c1bf30fa9b349d9`。官网原件与 HTML/CSS/精灵取证保留在 dist/wave-16/evidence/assets/。

先打开官方原件预览核对品牌，再使用 CairoSVG 渲染2048×2048，Pillow LANCZOS 缩至512×512 RGBA；icon.png SHA-256 `986524cc4aeddf8915affd081ffd11a29ddc74eecdd93ae83d7a29bf2b98528d`。未添加外框、底板、圆角蒙版或阴影，未改变原形、颜色或裁断边界。文字镂空、弧形和气泡属于官方图案。

依次实际打开官方原件、处理稿和最终 .koma 包内提取实物，核对真实身份、画布、边界及留白；包内PNG和源成品逐字节一致。128/64/32像素浅底 #F4F4F5 与深底 #18181B 拼图在 `/Users/honjow/.gemini/antigravity/brain/cecc3fcb-d070-43ba-8d77-3593835b7ffc/scratch/wave16_pkg_icons_review.png`。尺寸与哈希只辅助实图核查。

本轮不操作设备，App书源列表呈现留待后续真机QA；拼图不代表真机验收。官方品牌和素材权利归各平台，代码许可证不转授其所有权。
