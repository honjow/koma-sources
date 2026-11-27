# 官方图标与实物检查

- 官网：[https://comic-zenon.com](https://comic-zenon.com/)。已实际读取当前 header／CSS，核对其实际使用的品牌资源。
- 高清 SVG 原件：[https://cdn.comic-zenon.com/images/logo-zenon.svg](https://cdn.comic-zenon.com/images/logo-zenon.svg)；SHA-256 `b81e09af484df30fe32bd85c4b14eb6c230f2e1714379942f00847c545466150`。
- 官方方形身份参考：[https://comic-zenon.com/images/apple-touch-icon-180.png](https://comic-zenon.com/images/apple-touch-icon-180.png)，180×180；SHA-256 `97415030c2f41d51168e03431c12384a6fe5a022eebbd8255835bf15d8595bab`。仅作身份、配色和原画面核对，不作放大制作原件。
- 制作：从官网完整 SVG 中分离官方方形图标使用的完整「ゼ」字形，包括主字形与两个浊点，原始路径和珊瑚红色不变。移除参考 PNG 上缘透明缺口，恢复其原有白色画面的完整正方形四角；不放大该 PNG。 可复现矢量稿为 `assets/icon.svg`，SHA-256 `fcffbd6049da6e79552109a2f0d117e66207c05d28565a03e737716c60ab305c`。先栅格化为 2048×2048，再以 Lanczos 输出 512×512 PNG；不拉伸字形，不加阴影、圆角蒙版或额外底板。
- `icon.png`：512×512，SHA-256 `cea97478749413f770f68123d6713836eedf9790d7c99334b9d19060f75b9eba`。

已依次实际打开官网原件、处理稿和最终可安装包提取图，对照完整身份、正方形边界、笔画、填色、留白及浅深背景的清晰度。最终拼图：`/Users/honjow/.gemini/antigravity/brain/cecc3fcb-d070-43ba-8d77-3593835b7ffc/scratch/wave12_pkg_icons_review.png`；提取字节与源图一致。

未运行 Koma App 书源列表，App 真实呈现待验；此处仅为主机与包内实物检查。官方标识和商标归原站权利人，源代码 LICENSE 不授予商标权。
