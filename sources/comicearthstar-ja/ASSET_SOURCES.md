# 官方图标与实物检查

- 官网：https://comic-earthstar.com/。2026-11-28 实际读取官网 header、CSS 与 touch icon 声明，确认品牌。
- 高清 SVG 原件：[https://comic-earthstar.com/_next/static/media/header_logo.e0922828.svg](https://comic-earthstar.com/_next/static/media/header_logo.e0922828.svg)，SHA-256 `71578c9b69d090280525ae46d9c0935e402ea7331c9af200c60c673896c7a18e`。保存于 `assets/original-logo.svg`；使用真实矢量路径栅格化，不放大低清 PNG。
- 官方方形参考：[https://comic-earthstar.com/_next/static/images/apple-touch-icon-180.fb85a02b.png](https://comic-earthstar.com/_next/static/images/apple-touch-icon-180.fb85a02b.png)，180×180，SHA-256 `0ae2edea5c0ff345bf49405b100fe9a1c868fa06ef7c77beac9e9220c8178334`。仅供身份和方形画面核对，不作为放大制作原件。
- 制作：从官方完整 SVG 中分离前 8 条 path：蓝色球体和 7 枚白星，保留全部星形、官方径向渐变和原比例；与官网 touch icon 的独立球体标识核对一致。SVG 其余字标不属于官方方形 touch icon 的画面，故未纳入方形图标。球体的高光属于官方原画，不是新增阴影。可复现稿 `assets/icon.svg`，SHA-256 `72c39028fea2d0f1d4142351bf9379ecc9bda497d952a2233a213568af1dbca3`。CairoSVG 输出 2048×2048，再以 Lanczos 降采样至 512×512；没有添加底板、圆角蒙版、阴影或拉伸图形。
- 成品 `icon.png`：512×512 RGBA，SHA-256 `fef496bba4ae9b66c562c95ac2c83b145426372dbf485d3361dcbe372f9cd7da`。

已实际打开官网 SVG 栅格化预览、官方方形参考与处理稿，核对身份、完整边界、原形和配色。随后从最终 `.koma` 提取实际 `icon.png`，与源码成品逐字节比对一致；实际打开浅／深背景拼图，核对 280、128、64、32 像素下的形状、边界、留白与识别度，未发现额外阴影、底板或圆角蒙版。审图路径：`/Users/honjow/.gemini/antigravity/brain/cecc3fcb-d070-43ba-8d77-3593835b7ffc/scratch/wave13_pkg_icons_review.png`。

未在 Koma App 书源列表检查，本轮 App 实际呈现待验。官方标识、原画及商标归原站权利人，源代码 LICENSE 不授予商标权。
