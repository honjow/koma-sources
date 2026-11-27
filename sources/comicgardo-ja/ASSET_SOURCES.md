# 官方图标与实物检查

- 官网：[https://comic-gardo.com](https://comic-gardo.com/)。已实际读取当前 header／CSS，核对其实际使用的品牌资源。
- 高清 SVG 原件：[https://comic-gardo.com/_next/static/media/gardo_white_blue.5d5b5cb2.svg](https://comic-gardo.com/_next/static/media/gardo_white_blue.5d5b5cb2.svg)；SHA-256 `39589734a0bea92b812e5c8d605a164a96889fe6b0bb97d11ad1c0cc0d8785e3`。
- 官方方形身份参考：[https://comic-gardo.com/images/apple-touch-icon-180.png](https://comic-gardo.com/images/apple-touch-icon-180.png)，180×180；SHA-256 `59aab9611251bcff17e7b45206b3aa9e90351936d498c44733a15c8d62a4cdfd`。仅作身份、配色和原画面核对，不作放大制作原件。
- 制作：从官网当前 header 的完整 SVG 中分离独立蓝色 G 标识，保留全部笔触、比例和原蓝色；以官网方形图标核对同一 G 标识及其原有白色画面。SVG 中其他站模板的 header_logo 不用于本源；已通过实图和当前 header 选择器核对身份。 可复现矢量稿为 `assets/icon.svg`，SHA-256 `1b37787d9a5bc91bba7f146ec8bb6173ca23e54abe3a6db9e72af1fcedd0664e`。先栅格化为 2048×2048，再以 Lanczos 输出 512×512 PNG；不拉伸字形，不加阴影、圆角蒙版或额外底板。
- `icon.png`：512×512，SHA-256 `43b11c1efda9897c3b46667204c01524c2ce0ad4596f21c322416a6754b73946`。

已依次实际打开官网原件、处理稿和最终可安装包提取图，对照完整身份、正方形边界、笔画、填色、留白及浅深背景的清晰度。最终拼图：`/Users/honjow/.gemini/antigravity/brain/cecc3fcb-d070-43ba-8d77-3593835b7ffc/scratch/wave12_pkg_icons_review.png`；提取字节与源图一致。

未运行 Koma App 书源列表，App 真实呈现待验；此处仅为主机与包内实物检查。官方标识和商标归原站权利人，源代码 LICENSE 不授予商标权。
