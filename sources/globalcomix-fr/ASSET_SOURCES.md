# 官方图标来源与验收

采集日期：2026-11-28。官方原件入口：https://assets.globalcomix.com/static/images/web-ui/GCLogo-57062f.svg

官网 https://globalcomix.com/ 的 `img#pulse-icon` 直接引用此 SVG。当前官方亮黄绿色 GC 图形，与旧版橙色 favicon 不同；旧源图标未变。

仓库保存的官方矢量原件 / 独立提取稿：`assets/icon.svg`。SHA-256：`0928f81f54764e42ff4deb9ed708db202a94fccf1bcf72f2a9424985f29cc4e4`。

原始 SVG viewBox 0 0 20 22，按 SVG 默认 xMidYMid meet 等比放入正方形透明画布，保留官方路径、镂空与亮黄绿色。 CairoSVG 先按 2048×2048 栅格化，再用 Pillow LANCZOS 缩至 512×512 RGBA PNG；未做低清放大、变形拉伸、色彩替换、底板、阴影或圆角蒙版。图案自带的弧线或气泡是品牌原形，不是方形画布的圆角裁切。

成品 `icon.png`：512×512，RGBA，51351 字节；SHA-256：`ba0f376472a42d62500a6ef467979fd3fd77e60c1086119da5e4fe1c9075398b`。

## 实图核查证据

已按制作规范依次打开官方矢量栅格化预览、最终处理稿和最终 .koma 内提取的 icon.png；核对字形、颜色、原图边界、透明镂空与完整正方形画布。包内 PNG 与源成品逐字节一致。浅底 #F4F4F5 / 深底 #18181B 同时检查 128、64、32 像素缩略图，边角未蒙版，未添加阴影或底板，身份可辨认。

原件预览和官网 HTML/CSS 原始取证在 `dist/wave-14/evidence/assets/`；包内实物在 `dist/wave-14/evidence/globalcomix-fr/icon.png`。三源拼图：`/Users/honjow/.gemini/antigravity/brain/cecc3fcb-d070-43ba-8d77-3593835b7ffc/scratch/wave14_pkg_icons_review.png`。

本轮未授权设备操作，App 书源列表的真实呈现保留待真机验收；宿主拼图不代替 App 验收。品牌与官方素材权利属于原平台，代码许可证不转授商标或图形所有权。
