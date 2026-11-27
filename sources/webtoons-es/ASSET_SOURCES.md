# 官方图标来源与验收

采集日期：2026-11-28。官方原件入口：https://webtoons-static.pstatic.net/image/static/pc/sprite/sp_webtoon_svg_54232f64.svg

西语首页最新 stylesheet `https://www.webtoons.com/static/bundle/linewebtoon-b165d7c5b4ac57099614.css` 的 `.header_left .btn_logo:after` 引用此精灵图，80×80 窗口 x=716 y=1867。精灵 `ico_logo_header` 的外框 x=712 y=1863，viewBox=-4 -4 88 88，窗口对应无额外边距的 0 0 80 80。逐条 path 与 assets/icon.svg 完全一致。

完整官网精灵 SHA-256：`89c34228492d3d8cdbf551395da980e4c05712e6d8ca9d4b6d957df56db24c99`。

仓库保存的官方矢量原件 / 独立提取稿：`assets/icon.svg`。SHA-256：`fe2ae734cc72f05ce570a4a3852c741300089d27d6ed33060c1bf30fa9b349d9`。

从当前官方精灵独立抽取 logo path，保留 #00DC64、字形、气泡轮廓与内置透明文字。没有裁断图案。 CairoSVG 先按 2048×2048 栅格化，再用 Pillow LANCZOS 缩至 512×512 RGBA PNG；未做低清放大、变形拉伸、色彩替换、底板、阴影或圆角蒙版。图案自带的弧线或气泡是品牌原形，不是方形画布的圆角裁切。

成品 `icon.png`：512×512，RGBA，49533 字节；SHA-256：`986524cc4aeddf8915affd081ffd11a29ddc74eecdd93ae83d7a29bf2b98528d`。

## 实图核查证据

已按制作规范依次打开官方矢量栅格化预览、最终处理稿和最终 .koma 内提取的 icon.png；核对字形、颜色、原图边界、透明镂空与完整正方形画布。包内 PNG 与源成品逐字节一致。浅底 #F4F4F5 / 深底 #18181B 同时检查 128、64、32 像素缩略图，边角未蒙版，未添加阴影或底板，身份可辨认。

原件预览和官网 HTML/CSS 原始取证在 `dist/wave-14/evidence/assets/`；包内实物在 `dist/wave-14/evidence/webtoons-es/icon.png`。三源拼图：`/Users/honjow/.gemini/antigravity/brain/cecc3fcb-d070-43ba-8d77-3593835b7ffc/scratch/wave14_pkg_icons_review.png`。

本轮未授权设备操作，App 书源列表的真实呈现保留待真机验收；宿主拼图不代替 App 验收。品牌与官方素材权利属于原平台，代码许可证不转授商标或图形所有权。
