# 官方图标来源与像素处理

官网原主JS `download-popup` ios href：`https://pvp.qq.com/zlkdatasys/mct/d/play.shtml?device=ios`。原页面apple-itunes-app id989673964、MCTCFG.iosURL明确`https://itunes.apple.com/cn/app/id989673964`（getEagleCfg可override；没有运行其远程JS）。沿实际301 Location至Apple王者荣耀产品页，主app-icon square/aspect1的exact400w JPEG srcset：

`https://is1-ssl.mzstatic.com/image/thumb/PurpleSource211/v4/af/ea/af/afeaaf72-519b-9797-e0f8-807e79b393d9/Placeholder.mill/400x400bb-75.jpg`

原JPEG `/private/tmp/koma-honorofkings-zh-app-icon-chain-20261014/declared-primary-square-icon.original`，38384B/400×400 RGB，SHA `e5e3b880164bed4187a1df9baeadf3e76d35e601fc8e1cbe7d484419ea31ce4e`。仅按用户授权用确定性PNG编码满足源包格式，全部480000解码RGB bytes相同；回执`icon-pixel-equality.json`保Pillow版本及两文件/RGB SHA。没有改变像素、尺寸、圆角、白角、透明度，未加背板/阴影/遮罩或重绘。PNG仍RGB，无alpha。

原官网CSS logo.0e6d22cf.png是197×53横向纯白透明字标，白底无对比边界保留，未选入包。Apple其它推荐游戏图标及平台favicon未借用；横向1200×630 socialcard未取，未猜PNGthumbnail路径。

## 当前审查中的图标资源（2026-10-17）

- 官方素材URL：https://is1-ssl.mzstatic.com/image/thumb/PurpleSource211/v4/af/ea/af/afeaaf72-519b-9797-e0f8-807e79b393d9/Placeholder.mill/400x400bb-75.webp
- 页面声明/归属链：下方既有官网download-popup → pvp.qq.com/zlkdatasys/mct/d/play.shtml?device=ios → itunes.apple.com/cn/app/id989673964 → 实际301 Location Apple产品页；相同primary app-icon square/aspect1块明确source type=image/webp 400w URL，不猜格式路径；原声明 /private/tmp/koma-magireco-hok-icon-suitability-r3-20261017/provenance/ 保留
- 实际原件：`/private/tmp/koma-magireco-hok-icon-suitability-r3-20261017/hok-declared400-webp/body.original`
- 实际打包资源SHA256：`a46f4868e6b12f886d0e8b93b146d12ce6a0e3c9240b44efab48afaf56cb8f7a`
- 选择理由与边界：保原400×400 WebP，透明四角、角色/5v5完整，避免旧JPEG白角；原标自身圆角仍存在，不声称所有圆角移除；App显示裁剪独立保留。
- 已审候选：`/private/tmp/koma-magireco-hok-icon-suitability-r3-20261017/candidates/honorofkings-zh/honorofkings-zh-0.1.0-icon-r2.koma`。历史旧素材/包均保留，来源不以临时路径代替官方URL。
- 标准构建携带本资源与原metadata，但既有标准编译flags可改变WASM字节；不保证重编WASM与冻结包字节相同。旧WASM仅封包等价不构成完整可复现编译。
