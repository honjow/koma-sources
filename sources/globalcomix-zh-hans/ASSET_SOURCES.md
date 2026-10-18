# GlobalComix 图标来源

## 当前隔离候选：官网声明原ICO

- 页面： https://globalcomix.com/ ，2026-10-18一次普通匿名GET，原UA/Referer/Origin，无Cookie/redirect/JS。
- 页面HTTP200；达到本地2097152B上限curl56，整个HTML不完整、不重取。完整head已接收，第58行 `<link rel="shortcut icon" href="https://assets.globalcomix.com/static/images/header/favicon.ico">` 是本资源唯一读取依据，不猜URL。
- 原资源： https://assets.globalcomix.com/static/images/header/favicon.ico ，一次200完整，Content-Type image/x-icon，270398B，SHA256 `d1d64206cae427430b8c7f1a69d44250c7c28578a788997ef224cb0ddc8b9e98`。
- ICO单256×256 RGBA帧，alpha0..255；橙GC笔画、透明外部与孔隙。原bytes原格式作为根icon.ico，无加工/转码/裁剪/加板/阴影/遮罩。解码PNG仅目检副本，非交付素材。
- 原HTTP/HTML/head声明及逐bytes解码证据： `/private/tmp/koma-globalcomix-official-icon-20261018/` 的24绑定。官网来源不替代小槽视觉验收；Root已亲看原帧，standard候选尚未canonical/Native。App圆角显示独立保留。

## 同页对照原件（不采用）

完整head第23行apple-touch-icon明确 https://assets.globalcomix.com/static/images/header/homescreen.png 。一次200完整，180×180 RGB，6827B，SHA256 `75a86787983b828519d35db2d6887167e24fab499840ea6a3930c2bb93a7e496`；原图为深实底橙GC，不用于透明候选。未处理。

## 历史移植图标（保留原历史，不再随新候选打包）

Keiyoushi extensions-source记录commit1319949ca9bc456908196e3ba2d104e94182c5d6，路径src/all/globalcomix/res/mipmap-xhdpi/ic_launcher.png，Git blob4e7dfa6e15fb4e6d821d017d1e7963bca940a247；实际旧96PNG Git内容地址与记录一致，SHA256 `80d46eea7f8514f2f82b7abf7a3ba2ffe55fcdf21f1717cf4ba10270ad99b686`，3914B。旧010/011 ZIP图标原bytes均一致，Koma打包零加工。官网归属/更早加工史UNKNOWN，不再用“Official”概述替代原站证据。深圆角板/亮边与当前透明原件分开保留，不改写历史包。
