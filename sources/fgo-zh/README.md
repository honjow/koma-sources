# FGO国服官方漫画 · 0.1.2 数组元素/终态错误传播

当前标准生产入口：仓库根 `bash build.sh --source fgo-zh`。当前012在源局部对象数组读取中区分合法终态与无效元素，覆盖NUXT单data对象、目录comList、封面meta查找、正文comList、banner单图行与样式数组。无效元素在被读取时整次parse_error；保留封面og:image首匹配提前返回与非章节业务组件过滤。下方011章节语义及历史原件/官方素材出处继续适用。

同工具链011/012对照在已保存真实HTML上44组完整输出与HTTP callback一致，含15章/15原页URI和图片请求计划；受控插入null用于否定部分成功，不代表官网实际出现null。当前图标仍为下方官网声明300×300 PNG，原bytes未变。manifest按现行必需图标合同新增icon字段，旧字段除version保持。当前SDK与历史冻结构建有区别，已核实际rustc和flags相同，不声称生产WASM旧bytes复现或Native通过；设备更新须另行验收。

## 0.1.1 历史实现与来源记录

# FGO国服官方漫画 · 0.1.1 source-only continuation candidate

Official Chinese public first-series catalogue at
https://game.bilibili.com/fgo/manga01 . Independent source ID
`com.bilibili.fgo.zh.koma`, book `fgo:zh:manga01`; episode numbers and chapter
IDs come only from the catalogue's actual manga01_comic episode links.
This candidate covers the first series (15 actual links), not manga02 or
later official-account posts. Real usage and per-work ratings are unknown;
contentRating remains unknown and no adult catalogue is queried.

The returned HTML publishes literal window.__NUXT__ JSON, parsed as data
without executing scripts. The unique BiliBanner publishes 15 single-image
subComponentsList rows; each component ID has a declared background-image
URI in the same Nuxt metaInfo stylesheet. Official BiliUser loader publishes
script_id vue_e800a3b6620266002a7832f50cbae7c4075109e3. That exact published
script selects the same banner by episode query via swiper.slideTo(n-1).
The official default reader and catalogue-linked chapter3 page actually
returned the same complete 80871-byte HTML (SHA e5d849c543a43db8c4e75bc73f878448ddf97c229330297bdf8363402d391828).
Thus every get_pages operation normally fetches the public default reader
containing all 15 pages after checking catalogue membership, then chooses the
published episode's actual array member. It does not manufacture HTTP
responses for individual episodes or predict CDN names.

Source emits complete 15-chapter catalogue order/actual numbers, one original
page per current chapter template, and stable page IDs based on series and
actual episode. Every chapter additionally declares readingGroup=`fgo:zh:manga01`
and continuationEligible=true: the actual first-series catalogue and published
episode-to-slide selector establish one trustworthy numbered reading sequence.
This fixes the omitted source continuation contract without relaxing App/Reader
gates or including another series. IDs, chapter numbers, titles, original URIs,
page counts, image plans, capabilities and icon are unchanged from 0.1.0.
Unknown/malformed owner, selector ID, banner ID, row count,
component shape, stylesheet mapping, image URI or capacity fails the operation.
CDN URLs accept only the observed official i0.hdslb.com/bfs/game 40-hex
JPG/PNG paths. No login, cookie, challenge, payment, DRM, arbitrary JS,
image conversion or App/SDK changes. get_image_request returns the unchanged
source-emitted URI and ordinary catalogue Referer without metadata/image HTTP;
it validates CDN syntax, not independent cryptographic book ownership of
an arbitrary caller-supplied CDN hash. Catalogue/get_pages retain the real
provider membership/mapping checks.

All 15 descriptor mappings are compared against the real complete template
in retained-only production-WASM replay. Browse, local single-title search,
detail and chapter operations also replay actual published material; invalid
cursor/work/episode/CDN/hash/path rejects and ABI bounds are verified.
No HtmlHost attribute behavior is used. Only chapter3's original JPEG was
fetched and visually inspected (1157×1637, 1040964 B,
SHA fdff0fdceb3cdf5c18a26a6f8809722f38156ec53b4976ddebdc0928aa376c26).
For 0.1.0, Root has finite chapter3 Native body/download1/cold/History evidence.
Other14 image bodies and cross-chapter continuation were not accepted. Current
0.1.1 candidate has retained-only WASM comparison evidence; Native ordinary
update, natural chapter-end advancement, return and cold remain for Root.
Host evidence is not Native acceptance.

## Asset provenance

icon.png is the unchanged 300×300 PNG explicitly declared by the official
comic pages' og:image/shareImage, at
https://i0.hdslb.com/bfs/game/3631d85916ba65e587a342c466c89f099819c95f.png .
It is a site-declared official share image, not a claim that it is the site's
favicon (the page separately declares bilibili's favicon). Original SHA256
bae11a4963c35e401548ace641c64eee3523aaa529147db75eeaf6c6f3a3c1a0.
No crop, mask, backing, shadow, drawing, resize or reencoding was applied.
Root must inspect its actual appearance in the existing source-icon slot.


## 237 现装0.1.2第4→5章正文与自然续章（2026-10-21）

本轮registry实读 `com.bilibili.fgo.zh.koma` manifestVersion=0.1.2、enabled=true。普通Browse列表为1本《从漫画了解！Fate/Grand Order》，详情实际15/15章，每章1页。原reader已有第1章1/1记录，另有上方旧0.1.0第3章Native证据，因此没有重开这两章；从真实目录选择无现存章记录的第4章。

第4章P1/1实际显示彩色中文正文，“要让从者成为伙伴就必须通过召唤”“召唤又分友情点召唤和圣晶石召唤两种”可读。章末普通右侧下一步直接切换下一张P1/1，可读“FGO的卖点之一是其特有的指令战斗系统”；自然Back后真实reader确认 `fgoch:zh:manga01:5` / `fgopage:zh:manga01:5:0`、pageIndex0、totalPages1、completed=true，第4/5章记录各自然新增。这里只接受当前012这两章各一页及第4→5章普通续章/返回；不称单章P2、全部15章、下载、cold或无缓存在线首取通过。

原件在Koma/.hvigor/outputs/user-flow-audit/device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership/：fgo012-baseline237-001、fgo012-public-list237-001、fgo012-manga01-detail237-001、fgo012-chapter4-body237-001、fgo012-chapter4-next237-001、fgo012-natural-return237-001、fgo012-session-restored237-001。最终原Browse末端恢复，52项registry及reader偏好XML与本轮基线全bytes同；原17条sourceHistory和58条chapter对象全保持（含第1章），其他书46条progress对象全保持，只有本书当前progress从原第1章自然更新至第5章，两条新chapter记录保留。恢复回执实读AWAKE/OverrideTimeout10000ms，c49ac476已释放；哈希关闭，无清数据/缓存、模式/全局偏好更改、源/App/Kit业务改动、登录或下载。
