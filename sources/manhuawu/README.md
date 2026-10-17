# 漫画屋 public HTML adapter

Keiyoushi Manhuawu -> MCCMSWeb -> KeiSource, fixed revision 10f06eb833af5990bc5470c84c5e93edc159ad4e, Apache-2.0. Stable source ID and old canonical name retained. Upstream SAFE; each work unclassified, usage unknown.

New public cards use existing fallback mh:<actual slug>:0. Existing mh:<slug>:<numeric PICID> requests echo the exact input mangaId and resolve current HTML by slug. No aliases, migration, merge or claim of historical nonzero-PICID end-to-end identity compatibility. Chapter ch:<original full href> keeps /index.php exactly; page p:<index> is the existing identity. Catalogue follows primary explicit reverse. Only explicit 第N话/話/回/章 title leaves provide chapterNumber; order indices never become numbers.

No auth/cookies/challenge bypass/mirror/JS/image decryption or automatic retries. Missing/invalid/oversized parsing fails explicitly. Fixed existing SDK Firefox121 UA, no Referer/Origin, including image descriptor. Primary Firefox150 is documented but not adopted by probing. Images were not fetched; local body/WASM gates do not establish 197 acceptance.

Build: normal paired Rust toolchain ./build.sh --source manhuawu. minAppVersion0.1.0 and nsfwfalse reflect fixed upstream SAFE only. No publication.

## 当前0.1.2标准后继

完整保留已审011 HTML/中文身份/旧PICID兼容与严格错误合同；012只修相同pager的a.on/a.end/a.next职责，next==end不能用来判终态。current==terminal且next自指或缺失才终止，current<terminal时next必须是current+1；共享listing/search保同路由/顺序，不猜页数。

012原Root仅Host有限接受：真实首屏30项与原请求/输出不变；倒数第二/终态/search均明确受控，不冒实站。page49唯一公开请求403原件保留，禁止重试/换客户端；新012未Native，旧011/010具名正文下载cold不扩到012。生产入口 `bash build.sh --source manhuawu`，当前SDK/必需icon字段，不用旧封包冒标准编译。原icon/ASSET_SOURCES/Apache许可保留，App显示圆角独立保持，无新素材处理。当前未部署。
