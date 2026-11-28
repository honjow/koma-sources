# GlobalComix 官方图标与验收

本轮独立下载官网原件 https://assets.globalcomix.com/static/images/web-ui/GCLogo-57062f.svg ，HTTP200；未以旧包图标充当原件。官方SVG保存在assets/icon.svg：20×22，viewBox 0 0 20 22，SHA-256 `0928f81f54764e42ff4deb9ed708db202a94fccf1bcf72f2a9424985f29cc4e4`。保留官网亮黄绿色路径、字形与透明镂空，不修改任何旧源素材。

CairoSVG按默认xMidYMid meet等比居中栅格化到2048×2048透明方形，再由Pillow LANCZOS缩为512×512 RGBA。未拉伸原形、未添加阴影、底板、外框或圆角蒙版。成品icon.png：51351字节，SHA-256 `ba0f376472a42d62500a6ef467979fd3fd77e60c1086119da5e4fe1c9075398b`。

核查链：dist/wave-23/evidence/official-icon.svg → official-preview.png（600×660官方矢量栅格预览）→ raster-2048.png → processed-icon.png → 最终.koma提取icon.png。主代理实际打开原件预览、处理稿和包内实物，在/tmp/wave23_icons_review.png中对照官方预览、#F4F4F5浅底、#18181B深底；512方形边界完整且镂空透明，包内PNG与成品逐字节核对。

App列表真实呈现未代替用户真机QA，本轮不操作设备。标识与品牌权利属于GlobalComix权利人；Apache-2.0仅适用于源码移植。
