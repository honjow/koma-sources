# Joymii Hub 官网图标素材与验收

- 官网原件URL：https://cdn.joymiihub.com/wp-content/themes/masonry/images/favicons/apple-touch-icon.png
- 本次独立直连下载HTTP200，180×180 RGBA PNG，原件SHA256 `4ccbd94adeef6859bf0f4aba72a8c5b7c23637a302015c28d492bc45686b9990`；官网link rel=apple-touch-icon与地址对应，固定上游身份和官网标题共同核准。原始字节在dist/wave-33/evidence/probe/joymiihub/official.icon。
- 制作：原件为带透明圆形蒙版的官方文字标识，以图案自带且实际核准的均匀RGB(36,37,39)填色恢复完整方形边界，去除圆弧蒙版边缘；中心官方字形和抗锯齿像素保持原值，不重绘、不放大、不加底板或阴影。
- 成品180×180 RGBA icon.png，SHA256 `4b8935f51ee29ce99e6a3f8aa7561565b80c0c7d0738f740610e84602b305b32`。保留原生分辨率，不用低清拉伸制造高清。
- 主代理先打开官方原件，再打开处理稿，最后从最终.koma解包并核对PNG字节，在浅底#F4F4F5、深底#18181B及64/32px实图检查身份、边界和留白；证据scratch/wave33_pkg_icons_review.png。最终图标与制作目录原件分别留证，不以哈希或解码替代实图检查。
- 品牌、商标及图标权利归原权利人；代码许可证不转授素材权利。未操作设备，App书源列表真实呈现尚待用户真机QA。
