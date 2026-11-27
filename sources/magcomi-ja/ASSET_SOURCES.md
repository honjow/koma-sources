# 图标原件与实物检查

- 官网：[https://magcomi.com](https://magcomi.com/)。站标 SVG 是高清制作原件，官网正式方形图标仅核对站点身份、配色与原有画面。
- SVG 原件：[https://magcomi.com/_next/static/media/logo_magcomi2.034d192f.svg](https://magcomi.com/_next/static/media/logo_magcomi2.034d192f.svg)；SHA-256 `76f2fafcf055ed60b7a64ebe44a9424e41e03424f2fb2e1dadeeda102ff8f48e`。
- 官方方形参考：[https://magcomi.com/_next/static/images/apple-touch-icon-180.fdb159f8.png](https://magcomi.com/_next/static/images/apple-touch-icon-180.fdb159f8.png)；SHA-256 `39cc1e677d874d75fb02fc8e765341119a358e89b8a15eea6a39932dc82f48b6`。
- 制作：取官网 SVG 中完整独立圆形 M 标识的三条原始路径，保持图形比例；颜色采用官方方形标识已有的白色／品红。官网原有品红画面填满完整正方形，不添加阴影、圆角或额外底板。 `assets/icon.svg` 保留可复现矢量稿，先栅格化为 2048×2048，再以 Lanczos 缩至 512×512 PNG。未放大低清 PNG、热链外站素材或拉伸字形。
- 矢量稿 SHA-256 `6aa4bbb9db44a1d25d50b8024f879ff006428f9e95def70d979c0fa83842024a`。
- `icon.png`：512×512，SHA-256 `3a0671db4fe9f555efb47b9d71fdf0a3b2053d6ef5f73dc4be10c4c98797a6ee`。

已逐一实际打开官网原件、处理稿和最终 .koma 提取的 icon.png，核对身份、完整正方形边界、字形、原比例、留白、浅深背景和锐利度；提取后字节与源图一致。最终拼图位于 `/Users/honjow/.gemini/antigravity/brain/cecc3fcb-d070-43ba-8d77-3593835b7ffc/scratch/wave11_pkg_icons_review.png`。

用户禁止触碰 Koma，故 App 书源列表真实呈现未验；这里只记录主机和包内素材验收，不宣称完成 App 验收。官方标识、配色与商标权利归原站权利人；源代码 LICENSE 不授予商标权。
