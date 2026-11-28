# Playmate Hunter 官网图标与素材验收

- 原件：https://cdn.pmatehunter.com/wp-content/themes/masonry/images/favicons/apple-touch-icon.png
- 独立直连下载：HTTP 200，180×180 RGBA PNG；SHA-256 `1b815c3476bc972808502fb6cbba1825e933f1aa639a0ca4ace556dcbe61bf6a`。
- 官网名称和link rel=apple-touch-icon及固定上游站点身份相互核准；原件在dist/wave-32/evidence/probe/playmatehunter/official.icon。
- 原件自带圆形或圆角透明蒙版。以已核实的画面底色RGB(0, 0, 0)恢复完整方形边界，去掉透明蒙版的圆弧抗锯齿边缘；官方中心图案、文字和自身抗锯齿保持原值，无缩放、无阴影、无额外白边或底板。成品保持原生180×180 RGBA PNG，不用低清拉伸伪造高清。
- 最终icon.png SHA-256 `c7fc76f563562d00cd7f5caf8796c90995ce0c586a9dcd30ce964b2c80fc39f4`。主代理先打开官网原件，再打开处理稿，最后解包.koma逐字节比对并在浅底#F4F4F5/深底#18181B以及64/32px下实图复核。证据：scratch/wave32_pkg_icons_review.png；逐项结论见wave32_sol_report.md。
- 代码许可证不转授站点图标或商标权利，品牌资产归原权利人。未操作设备，App列表呈现尚待用户真机QA，不以Pillow或拼图代替设备验收。
