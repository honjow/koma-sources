# 官方图标来源与制作验收

采集日期 2026-11-28。官方 德语首页 https://www.webtoons.com/de/ 的当前样式表 https://www.webtoons.com/static/bundle/linewebtoon-b165d7c5b4ac57099614.css 引用精灵图 https://webtoons-static.pstatic.net/image/static/pc/sprite/sp_webtoon_svg_54232f64.svg。完整精灵 SHA-256 `89c34228492d3d8cdbf551395da980e4c05712e6d8ca9d4b6d957df56db24c99`；独立抽取 ico_logo_header 的确切 path，与本平台已验收矢量逐条相等。保留官方 #00DC64 气泡和透明文字。

assets/icon.svg SHA-256：`fe2ae734cc72f05ce570a4a3852c741300089d27d6ed33060c1bf30fa9b349d9`。原始官网 HTML/CSS/完整精灵和原件预览保留在 dist/wave-15/evidence/assets/。

先打开官方原件的512预览，核对真实品牌，再用 CairoSVG 渲染2048×2048，通过 Pillow LANCZOS 缩至512×512 RGBA。icon.png SHA-256：`986524cc4aeddf8915affd081ffd11a29ddc74eecdd93ae83d7a29bf2b98528d`。未添加背景板、外框、圆角蒙版、阴影或改色，未变形或裁断品牌轮廓。透明文字/弧形/气泡属于官方图案本身。

依次核查官方原件、处理稿及最终 .koma 内提取实物；包内PNG与源成品逐字节一致。128/64/32像素浅底 #F4F4F5 和深底 #18181B 拼图：`/Users/honjow/.gemini/antigravity/brain/cecc3fcb-d070-43ba-8d77-3593835b7ffc/scratch/wave15_pkg_icons_review.png`。原形、边界、透明留白及缩略可辨性由实图核查，尺寸和哈希只作辅助。

本轮不操作设备，App书源列表真实呈现留待后续真机QA；拼图不代表真机验收。品牌及素材权利归官方平台，代码许可证不转授商标或图形所有权。
