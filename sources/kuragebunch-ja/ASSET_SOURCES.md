# 图标原件与制作验收

- [官网原件](https://cdn.kuragebunch.com/images/kuragebunch_logo.svg)。官网 CSS 指定的 SVG，viewBox 0 0 348 54.58；黑色 st0 和黄绿色 st1 分别是「くらげ」「バンチ」完整原始路径。官网 Android 图标用于核对方形两行结构。
- 原件 SHA-256：`831a2899b4fa481faf27958251515a6983a8857f477d754d45855be9d80d1cab`。
- [官方方形参考](https://cdn.kuragebunch.com/images/android-icon-192.png)，原件 SHA-256 `fe7bcb52bc8a6dd3c15dad400f87bf99aae5ab2f36f9ea8ffb65f8f380182e6b`。仅核对身份、配色和官方两行结构，未用于低清简单拉伸。
- 制作：按原 SVG 的两组字形分别高清栅格化，以官方方形图标的上下排列重建 512×512。保持字形、长宽比例与原有配色，恢复完整白色方角；不沿用原 Android 图标的圆角、额外轮廓和边缘杂线，不拉伸 192px PNG。
- 最终 `icon.png`：512×512 PNG；SHA-256 `a86f33298af82c70024befc4eede09f79efab414cb3be4d60d3fb9c0c369fc3d`。

## 实物检查

已实际打开官网原件、处理稿，核对真实站点身份、原始字形、配色、完整方形边界、留白，以及浅深背景下的可辨认性。已从最终可安装包提取实际 icon.png，确认与源图逐字节一致，并实际打开浅深底拼图复核。包内检查结果记录在 `dist/wave-10/verification.json`；拼图位于用户指定的 `scratch/wave10_pkg_icons_review.png`。

本次用户明确禁止触碰 Koma，未运行 App 书源列表或真机路径；App 呈现保持待验。这是候选包的主机素材检查，不声明完成 App 素材验收。官方商标及图案权利归原站权利人所有，源代码 LICENSE 不授予商标使用权。
