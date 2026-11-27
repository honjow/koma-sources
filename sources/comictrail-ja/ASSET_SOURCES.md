# 图标原件与实物检查

- 官网：[https://comic-trail.com](https://comic-trail.com/)。站标 SVG 是高清制作原件，官网正式方形图标仅核对站点身份、配色与原有画面。
- SVG 原件：[https://cdn.comic-trail.com/images/logo.svg](https://cdn.comic-trail.com/images/logo.svg)；SHA-256 `f866c8e447ce40191dec507eb9a8e3875015043c13a871e9fdc4a3e44fc77972`。
- 官方方形参考：[https://cdn.comic-trail.com/images/android-icon-192.png](https://cdn.comic-trail.com/images/android-icon-192.png)；SHA-256 `51003991d0e6d2f9c5031bba2135ceac1f48414199e8139c23f67dbdfe3ec5cd`。
- 制作：官网完整 SVG 原比例栅格化，在官方方形图标自带的白色画面中居中，保留全部字形、comic 字样和色块。移除原参考 PNG 的圆角透明蒙版，恢复完整白色四角，不添加阴影或额外底板。 `assets/icon.svg` 保留可复现矢量稿，先栅格化为 2048×2048，再以 Lanczos 缩至 512×512 PNG。未放大低清 PNG、热链外站素材或拉伸字形。
- 矢量稿 SHA-256 `d01172cf746b047fbbab64faa7728b928244fa2ff3f07d1b3858bc852d88c95c`。
- `icon.png`：512×512，SHA-256 `8a3adb6ad198cd52c2d32ee41d11d9acbd727329ec0cf6725cef6bc4bcac2dc3`。

已逐一实际打开官网原件、处理稿和最终 .koma 提取的 icon.png，核对身份、完整正方形边界、字形、原比例、留白、浅深背景和锐利度；提取后字节与源图一致。最终拼图位于 `/Users/honjow/.gemini/antigravity/brain/cecc3fcb-d070-43ba-8d77-3593835b7ffc/scratch/wave11_pkg_icons_review.png`。

用户禁止触碰 Koma，故 App 书源列表真实呈现未验；这里只记录主机和包内素材验收，不宣称完成 App 验收。官方标识、配色与商标权利归原站权利人；源代码 LICENSE 不授予商标权。
