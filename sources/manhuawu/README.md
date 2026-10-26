# 漫画屋 public HTML adapter

Keiyoushi Manhuawu -> MCCMSWeb -> KeiSource, fixed revision 10f06eb833af5990bc5470c84c5e93edc159ad4e, Apache-2.0. Stable source ID and old canonical name retained. Upstream SAFE; each work unclassified, usage unknown.

New public cards use existing fallback mh:<actual slug>:0. Existing mh:<slug>:<numeric PICID> requests echo the exact input mangaId and resolve current HTML by slug. No aliases, migration, merge or claim of historical nonzero-PICID end-to-end identity compatibility. Chapter ch:<original full href> keeps /index.php exactly; page p:<index> is the existing identity. Catalogue follows primary explicit reverse. Only explicit 第N话/話/回/章 title leaves provide chapterNumber; order indices never become numbers.

No source-managed auth/cookies, challenge bypass, mirror, JS/image decryption or automatic retries. Version0.1.5 can request the App’s existing origin-scoped website verification when the Host explicitly identifies a challenge. Missing/invalid/oversized parsing fails explicitly. Fixed existing SDK Firefox121 UA, no Referer/Origin, including image descriptor. Primary Firefox150 is documented but not adopted by probing. Images were not fetched; local body/WASM gates do not establish 197 acceptance.

Build: normal paired Rust toolchain ./build.sh --source manhuawu. minAppVersion0.1.0 and nsfwfalse reflect fixed upstream SAFE only. No publication.

## 当前0.1.2标准后继

完整保留已审011 HTML/中文身份/旧PICID兼容与严格错误合同；012只修相同pager的a.on/a.end/a.next职责，next==end不能用来判终态。current==terminal且next自指或缺失才终止，current<terminal时next必须是current+1；共享listing/search保同路由/顺序，不猜页数。

012原Root仅Host有限接受：真实首屏30项与原请求/输出不变；倒数第二/终态/search均明确受控，不冒实站。page49唯一公开请求403原件保留，禁止重试/换客户端；新012未Native，旧011/010具名正文下载cold不扩到012。生产入口 `bash build.sh --source manhuawu`，当前SDK/必需icon字段，不用旧封包冒标准编译。原icon/ASSET_SOURCES/Apache许可保留，App显示圆角独立保持，无新素材处理。当前未部署。

## 0.1.3 webpage-link candidate

`get_manga` now declares one `kind=source` link using the same BASE + detail path as its existing GET. For the existing `mh:doupocangkong:0` (斗破苍穹) entity, one ordinary anonymous GET to `https://www.mhua5.com/index.php/comic/doupocangkong` returned HTTP200/318057B and the named work HTML, without redirect. No canonical/og:url was present; this is the successful original detail request URL. Identity/PICID handling, requests, chapters, images, permissions, icon and metadata remain unchanged.

Actual 0.1.2 and 0.1.3 WASM replayed this same saved HTML with network blocked: both succeeded, every non-links manga field and the sole HTTP request were deeply equal. Offline locked release build passed; the locally assembled 85453B package contains exactly manifest/source.wasm/icon.png, with complete existing localizedNames and unchanged icon bytes. The full-dist cleanup/index-summary build path was not executed after its automatic approval rejection. The separate page49 and current popular-page403 remain stopped and do not count as this detail request failing. Evidence: `Koma/.hvigor/outputs/user-flow-audit/manhuawu013-{detail-once.html,detail-once.json,baseline-offline.json,candidate-offline.json,link-compare-offline.json,build.log}`. No reading, downloads or chapter refresh was run.

237 ordinary picker update installed 0.1.3 and kept 54 enabled sources. The existing 斗破苍穹 was reached safely through Library-only global search; detail kept 670/670 chapters and Start reading, but its menu still showed only three Tracker actions, without either webpage action. WebView/return acceptance therefore remains OPEN; no popular request, reading or download was performed. That 237 run retained no detail-time get_manga/get_chapters evidence. Root subsequently traced the Library-only search route: it opened the local snapshot instead of the source-detail route. The App now shares the existing shelf route; 197 then entered source loading but displayed a source-detail failure. Its precise failure code remains unknown and was not retried; neither result counts as webpage acceptance. Query was cleared, the original All sources filter restored, Browse54 and 10000ms/AWAKE restored, own transfer removed and leases released. Raw evidence under `Koma/.hvigor/outputs/user-flow-audit/device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership/`: `manhuawu013-menu237-001-gate/{detail013.png,webpage-menu.png}`, `manhuawu013-version237-001-gate/{manhuawu-installed.png,browse54.png}`, `manhuawu013-session237-001-{gate,restored}`, and `manhuawu013-bounded-log237-003/run-metadata.json`.

2026-10-22 237 follow-up: Library source filter 漫画屋 → the sole 斗破苍穹/670-chapter card used the ordinary shelf source-detail route. Synchronous PID40941 logs recorded get_manga HTTP200/operation_ok at 03:06:19 and get_chapters HTTP200/operation_ok at 03:06:20; the real cover, 知音漫客 任翔, 670/670 catalogue and both webpage actions appeared. The subsequent ordinary WebView action displayed the same work's real site title, matching cover, author, synopsis and chapter grid. An initial loading white screen and a remaining green loading indicator are preserved; this does not claim all subresources finished loading. Natural Back returned to native 670/670 and Start reading. The earlier 197 detail failure cause remains unknown. No refresh, popular/page49, chapter reading, download or external browser action was performed. Both sessions restored All sources, last-read shelf and Browse55; library/reader/full registry JSON deeply equal before/after, 10000ms/AWAKE read back, leases43d11d2a/a9b42707 released; no package transfer/install. Raw evidence under Koma category-membership/: manhuawu013-sync-detail237-001/{detail.png,detail-hilog.txt,webpage-menu.png}; manhuawu013-webview237-001-gate/{webview-initial.png,webview-current.png,native-return.png,shelf-restored.png,browse-restored.png}; corresponding *-restored/ directories contain comparisons and power receipts.

## 0.1.4 HTTP错误信息

保留书源实际收到的HTTP状态码，区分401、403、404、429及5xx；网络或解析错误不伪造状态。请求、目录解析、阅读身份、图标及会话权限保持。197当前列表实际收到403，具体拒绝原因仍待调查；网页首页可打开不代表列表或正文已恢复。


## 0.1.5 标准网页挑战信号

仅在Host明确提供 `accessSignal=challenge_required` 且HTTP状态为401/403/503时，返回现有挑战错误并保留状态；普通403及其他错误不变，不猜HTML关键词。仅声明 `https://www.mhua5.com`，沿宿主既有受限验证及失败地址回流，不新增请求、Cookie读取或自动重试。

197普通官网分类曾自然出现安全验证并自行通过；这不证明Native423的403正文或响应头。当前候选还需支持该标准信号的App构建与普通Native验收，不能以离线回放宣布列表/正文已恢复。历史page49请求停止边界保持。

## 0.1.6 明确源目录时间方向

本次人类在核对Mihon默认源码后采用“按源序选最早未读”，取代尚未采用的改源DOM顺序候选。`metadata.json`声明`chapterOrder: newest_first`，通过标准打包进入manifest；这是原输出时间方向，不是展示排序。保留MCCMSWeb参考的整体反转、全部原chapterId、原话数（含unknown）、请求、权限和样式；不新增标题解析或伪造小数。宿主已有进度/显式章选择优先，未声明源不得猜方向。

原始证据在Koma现有user-flow-audit/manhuawu-yuanzun-exact-contract-228：准确SDK WindowsFirefox121单次详情200/552175B，1270章DOM首为第1上228928→第1下228919；发布015WASM回放仍输出最新在前。斗破旧成功HTML670章以同WASM回放：默认最小已知号误选131回，Mihon源序合同则选原01/196437。固定参考10f06eb的MCCMSWeb明确getDescendingChapters/asReversed；Mihon宿主d001e6f默认SOURCE+DESC、降序取最后未读。该声明只适用于本源，不把Mihon标题编号推断或其他源的方向自动移植。

0.1.6标准单源构建完成，87192B三成员包已通过237普通系统文件选择器更新，80源中仅本源版本/交付路径及chapterOrder变化；App普通冷恢复仍保留声明，旧79源、书库和原阅读记录完整保持。最终App8588931B的书架来源筛选→现有斗破详情670/670→开始阅读，实际首图1/36，独立249读回chapter196437/page0/36，原61项进度及原顺序逐项保持；没有清历史造首次。元尊1270章的生产WASM回放与宿主真实选择器已证第1话上228928，普通237源入口248却自然触发网页挑战，尚未到该书实机首次阅读，不将其或整个站点记为通过。后继App8589011B仅按当前人类要求收紧验证半模态底栏，不改变本源请求/目录。范围提交、精确CI与实际公开发布结果仍分开记录于Koma CURRENT_STATUS。

## 0.1.7 搜索第一页的规范分页地址

2026-10-26 普通统一搜索“斗破苍穹”自然收到 HTTP200 后报 parse_error；同关键词、SDK 原 UA、无 Cookie 的一次桌面响应有6条漫画，分页 current/end/next 都指向 `/index.php/search/斗破苍穹`，省略 `/1`。原0.1.6实际WASM回放该HTML明确返回 `invalid search pagination`；此为独立解析反例，不解释旧元尊页表网络超时或证明同份Native响应。

只把当前查询的规范搜索base地址认作第一页；显式数字页和原查询身份、当前/末页/下一页关系校验保持。沿固定MCCMSWeb参考的末页停止意图，不放宽其他查询或无效页；请求URL、UA、会话权限、漫画/章节身份、目录方向和样式不变。回放及后续普通升级/搜索的实际验收仍以Koma CURRENT_STATUS原漫画屋项为准，不以构建代替。
