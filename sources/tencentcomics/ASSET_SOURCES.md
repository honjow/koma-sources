## 当前审查中的图标资源（2026-10-17）

- 官方素材URL：https://gtimgcdn.ac.qq.com/media/images/ac_logo.png
- 页面声明/归属链：https://ac.qq.com/ 既有公开作品详情 img.header-pic-logo（alt=腾讯动漫官方网站）；原页面保留在 /private/tmp/koma-source-icon-next3-20261017/provenance/tencent-parent.original
- 实际原件：`/private/tmp/koma-source-icon-next3-20261017/tencent/asset.original`
- 实际打包资源SHA256：`fa0cdf9db0e7568f18f0ce1b9ac24c640d003306404d122d70211648bb5db56c`
- 选择理由与边界：原173×95透明角色与正式字标，替换移植图的外板/阴影；32vp适用仍待Native。
- 已审候选：`/private/tmp/koma-source-icon-next3-20261017/candidates/tencent/tencent-0.1.2-icon-r2.koma`。历史旧素材/包均保留，来源不以临时路径代替官方URL。
- 标准构建携带本资源与原metadata，但既有标准编译flags可改变WASM字节；不保证重编WASM与冻结包字节相同。旧WASM仅封包等价不构成完整可复现编译。


## 方形官网原素材（2026-10-17，源码采纳后 Native 待验）

- 正式原资源URL：https://gtimgcdn.ac.qq.com/h5_hd/downloadForPc/images/logo.3e14a6cd.png?v=daa0cc89ec0fd3e0
- 声明/归属链：原官网作品详情明确手机版入口 https://m.ac.qq.com/event/downloadForPc/index.html ，新保留的客户端下载页 img.logo 原始声明；PNG120×120，透明外角和橙色为官方固有像素，未加工。同品牌角色方形替代当前横标，并非字标逐像素保持。
- 原件SHA256：`fb4885dc78a1cec40a9f418e416389e965e5afec32a1e7e2c49f7440a15c38e8`；`icon.png` 与该原件逐bytes相同，没有缩图/裁切/加底/阴影/遮罩。
- 历史横标与旧移植图来源全部保留在上文及原冻结包；App显示圆角独立保留。
- 仅资源/出处差异，parser/Cargo/SDK/metadata不改；旧WASM封包不是重新编译。Native32vp适用与普通更新尚未验收。
