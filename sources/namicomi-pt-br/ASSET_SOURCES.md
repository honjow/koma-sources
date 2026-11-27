# 官方图标来源与制作验收

采集日期 2026-11-28。官方巴葡语首页 https://namicomi.com/pt-br 的 img[alt=NamiComi] 内嵌 data:image/svg+xml 原件；独立抓取页面并解码 SVG，完整字节保存在 assets/icon.svg。原件 viewBox 0 0 2000 2000，保留官方双弧、蓝色和透明镂空。

assets/icon.svg SHA-256：`174263901610820502049a384355bc1ff5b2f710095490e1401caf108df87a6c`。原始官网 HTML/CSS/完整精灵和原件预览保留在 dist/wave-15/evidence/assets/。

先打开官方原件的512预览，核对真实品牌，再用 CairoSVG 渲染2048×2048，通过 Pillow LANCZOS 缩至512×512 RGBA。icon.png SHA-256：`b6dc49a97b999f965843245379ae26d9b9b28a96fa841eb14b2f052d42144a11`。未添加背景板、外框、圆角蒙版、阴影或改色，未变形或裁断品牌轮廓。透明文字/弧形/气泡属于官方图案本身。

依次核查官方原件、处理稿及最终 .koma 内提取实物；包内PNG与源成品逐字节一致。128/64/32像素浅底 #F4F4F5 和深底 #18181B 拼图：`/Users/honjow/.gemini/antigravity/brain/cecc3fcb-d070-43ba-8d77-3593835b7ffc/scratch/wave15_pkg_icons_review.png`。原形、边界、透明留白及缩略可辨性由实图核查，尺寸和哈希只作辅助。

本轮不操作设备，App书源列表真实呈现留待后续真机QA；拼图不代表真机验收。品牌及素材权利归官方平台，代码许可证不转授商标或图形所有权。
