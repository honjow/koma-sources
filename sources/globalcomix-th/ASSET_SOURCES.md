# GlobalComix 官网素材与验收

本轮重新获取官网原件：https://assets.globalcomix.com/static/images/web-ui/GCLogo-57062f.svg （HTTP200）。assets/icon.svg为20×22、viewBox 0 0 20 22，SHA-256 `0928f81f54764e42ff4deb9ed708db202a94fccf1bcf72f2a9424985f29cc4e4`；未拿旧源PNG当作矢量原件。

CairoSVG按默认xMidYMid meet等比居中放入2048×2048透明方形，Pillow LANCZOS缩至512×512 RGBA。保留正式标识的路径、镂空及亮黄绿色，不添加阴影、底板、外框或圆角蒙版，不拉伸、不重绘、不更换色彩。icon.png为51351字节，SHA-256 `ba0f376472a42d62500a6ef467979fd3fd77e60c1086119da5e4fe1c9075398b`。

核查链：dist/wave-26/evidence/official-icon.svg → official-preview.png（600×660）→ raster-2048.png → processed-icon.png → 最终.koma包中解出的icon.png。主代理实际打开官方预览、处理稿及最终包内实物，在浅底#F4F4F5和深底#18181B核对512方形边界、颜色、镂空和缩略尺寸。拼图保存为dist/wave-26/evidence/wave26_pkg_icons_review.png，并嵌入scratch/wave26_sol_report.md。

品牌权利属于GlobalComix相应权利人；代码Apache-2.0不转授商标或标识权利。本轮未操作设备，App书源列表实际呈现保留待用户真机QA。
