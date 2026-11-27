# 图标原件与制作验收

- [官网原件](https://comic-action.com/)。官网首页 header 内嵌 SVG，viewBox 0 0 220 37.97；SVG 原始字形与路径作为高清制作原件。另以官网 180×180 Apple Touch Icon 核对站点身份、两行结构和米色原有填色。
- 原件 SHA-256：`dddb66f0ad02e65162c02246b57a0e3e5770bdecd207f64428f5a5dc9abe98ee`。
- [官方方形参考](https://comic-action.com/_next/static/images/apple-touch-icon-180.b58bf412.png)，原件 SHA-256 `4dcddb850535251fdbd1e1c96a39492e8312a680ed7337b0dfe61e26d4098763`。仅核对身份、配色和官方两行结构，未用于低清简单拉伸。
- SVG 从上述首页 header 提取，恢复 HTML 解析后的 viewBox 属性大小写，原始字形路径保持不变；下载页面快照 SHA-256 `a88e2b4edfa6d9846c36b67af10a52d4a34da2b8c3090bac26228fb2084c6247`，提取 SVG SHA 与下一项一致。
- 制作：SVG 按 2200px 宽栅格化，将完整的标语/web 字形组与アクション字形组按官方方形标识的上下结构排放；保持各组原比例，没有拉伸字形、替换字体、使用外站素材或放大低清 PNG。512×512 米色来自官网方形图标原有画面，无新增阴影、额外底板或圆角蒙版。
- 最终 `icon.png`：512×512 PNG；SHA-256 `45dbf1a90b1c8fdbcafce8a1fb604a374ede2235bd1383da82c6bd127f4ea785`。

## 实物检查

已实际打开官网原件、处理稿，核对真实站点身份、原始字形、配色、完整方形边界、留白，以及浅深背景下的可辨认性。已从最终可安装包提取实际 icon.png，确认与源图逐字节一致，并实际打开浅深底拼图复核。包内检查结果记录在 `dist/wave-10/verification.json`；拼图位于用户指定的 `scratch/wave10_pkg_icons_review.png`。

本次用户明确禁止触碰 Koma，未运行 App 书源列表或真机路径；App 呈现保持待验。这是候选包的主机素材检查，不声明完成 App 素材验收。官方商标及图案权利归原站权利人所有，源代码 LICENSE 不授予商标使用权。
