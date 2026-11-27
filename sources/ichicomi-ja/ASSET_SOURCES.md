# 官方图标与实物检查

- 官网：https://ichicomi.com/。2026-11-28 实际读取官网 header、CSS 与 touch icon 声明，确认品牌。
- 高清 SVG 原件：[https://ichicomi.com/_next/static/images/ic_logo.53994845.svg](https://ichicomi.com/_next/static/images/ic_logo.53994845.svg)，SHA-256 `f9d8399f6f8a2266a5cc0bff8170f548240fc4771851fe18f80c3d1c9cad782a`。保存于 `assets/original-logo.svg`；使用真实矢量路径栅格化，不放大低清 PNG。
- 官方方形参考：[https://ichicomi.com/images/apple-touch-icon-180.png](https://ichicomi.com/images/apple-touch-icon-180.png)，180×180，SHA-256 `e4ea3ef29894bd4728480df269adf29ad14fdc4952d7991fc8b202d59597ad40`。仅供身份和方形画面核对，不作为放大制作原件。
- 官网独立标识参考：[https://ichicomi.com/images/favicon.ico](https://ichicomi.com/images/favicon.ico)，含 16/32/64 像素规格，SHA-256 `23d52d4b833fa6339e118e2de74021707e44f32eeafdff56d0b2ae31892bb9b5`；保存于 `assets/official-favicon-reference.ico`。已实际打开并确认是蓝色“迅”轮廓和橙色十字。仅供形状核对，不直接放大 ICO。
- 制作：初稿完整横向字标在小尺寸下难以识别，已弃用。终稿从官网高清 SVG 的第一个 g 中分离第 15、16、18、19、20 条 path（从 0 计数），组成官网 favicon 实际使用的独立“迅＋”标识，保留蓝橙配色、原笔画和十字比例，方形透明画布中等比居中。没有重新绘制、截掉字形或重新排字。可复现稿 `assets/icon.svg`，SHA-256 `be9cba51a68acab7d30c73200314b0edbe5f693097f67d25f7e8880d7f99dbb3`。CairoSVG 输出 2048×2048，再以 Lanczos 降采样至 512×512；没有添加底板、圆角蒙版、阴影或拉伸图形。
- 成品 `icon.png`：512×512 RGBA，SHA-256 `048d2194648f1e2ecd8b8fc2b8a66777049b11f73a2e6b8f923825178c2a278c`。

已实际打开官网 SVG 栅格化预览、官方方形参考与处理稿，核对身份、完整边界、原形和配色。随后从最终 `.koma` 提取实际 `icon.png`，与源码成品逐字节比对一致；实际打开浅／深背景拼图，核对 280、128、64、32 像素下的形状、边界、留白与识别度，未发现额外阴影、底板或圆角蒙版。审图路径：`/Users/honjow/.gemini/antigravity/brain/cecc3fcb-d070-43ba-8d77-3593835b7ffc/scratch/wave13_pkg_icons_review.png`。

未在 Koma App 书源列表检查，本轮 App 实际呈现待验。官方标识、原画及商标归原站权利人，源代码 LICENSE 不授予商标权。
