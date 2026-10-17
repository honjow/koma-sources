# BH3 package assets

- `icon.png` is the public Honkai Impact 3 header logo published by the
  official BH3 site stylesheet at
  `https://act-webstatic.mihoyo.com/new_static_v2/image/benghuai3logo.png`.
- Retrieved on 2026-08-14; SHA-256:
  `ab08ecfa0cac17825df4563c928e0155e8ca52f9e9eab358d28307399d4568ad`.
- It is packaged only as the identity icon for this BH3 source. Do not use it
  as a manga cover or as a substitute for another source's brand asset.

## 当前审查中的图标资源（2026-10-17）

- 官方素材URL：https://act-webstatic.mihoyo.com/new_static_v2/image/benghuai3logo.png
- 页面声明/归属链：原BH3官网样式声明header logo；canonical ASSET_SOURCES.md保原URL。20261017对该精确URL正常一次GET200，与原canonical SHA相同。旧012移植launcher的归属依据另保留 /private/tmp/koma-source-icon-next3-20261017/provenance/bh3-primary-icon-binding.json
- 实际原件：`/private/tmp/koma-source-icon-next3-20261017/bh3/asset.original`
- 实际打包资源SHA256：`ab08ecfa0cac17825df4563c928e0155e8ca52f9e9eab358d28307399d4568ad`
- 选择理由与边界：透明官方143×96字标，替换旧移植图灰色圆板/阴影；小槽可辨性仍待Native。
- 已审候选：`/private/tmp/koma-source-icon-next3-20261017/candidates/bh3/bh3-0.1.2-icon-r2.koma`。历史旧素材/包均保留，来源不以临时路径代替官方URL。
- 标准构建携带本资源与原metadata，但既有标准编译flags可改变WASM字节；不保证重编WASM与冻结包字节相同。旧WASM仅封包等价不构成完整可复现编译。


## 方形官网原素材（2026-10-17，源码采纳后 Native 待验）

- 正式原资源URL：https://bh3.mihoyo.com/favicon.ico
- 声明/归属链：漫画官网 https://comic.bh3.com/ footer http://www.bh3.com ，正常301精确到 https://www.bh3.com/ ，head favicon原始声明。实际PNG119×119，URL/MIME标ICO但未转码，按真实PNG打包；琪亚娜/miHoYo完整游戏画面，非旧泛mi/当前字标像素保持。
- 原件SHA256：`010b7376dd307a4bc7fe09c9d8f9bae58be27c48cf5ecbd1d2bfc785610097f0`；`icon.png` 与该原件逐bytes相同，没有缩图/裁切/加底/阴影/遮罩。
- 历史横标与旧移植图来源全部保留在上文及原冻结包；App显示圆角独立保留。
- 仅资源/出处差异，parser/Cargo/SDK/metadata不改；旧WASM封包不是重新编译。Native32vp适用与普通更新尚未验收。
