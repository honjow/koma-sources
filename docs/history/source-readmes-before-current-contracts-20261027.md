# 书源 README 历史存档

本档仅保全修订前文字及历史证据，不是当前合同；不从默认阅读链引用。

## dm5

# DM5 0.1.20

当前生产源码完整继承已验0.1.17 owner（包含015完整业务和016展示标题修复），使用仓库标准入口 `bash build.sh --source dm5`；保当前SDK、原icon和ASSET_SOURCES，不使用历史borrowed build或重封旧WASM冒充编译。

完整继承包括：原卡片父容器及uk关联封面、既有正文签名/身份/顺序及失败传播、正式目录ABI与真实pager cursor、Home listingLink、latest首页别名、合并014正文容量继承、015卡片/章节数组结束与写失败整次拒绝。固定元数据、业务过滤和既有未改行为均保原owner，不夹带新parser或素材变化。

已验交付：0.1.15包3e6e65642725f913faafc79f5cdc6bc41d0e4b6697209c44fba6cd75008e79bf，WASM12e506346eae312a0a14d61390983f9231c4a2596eaecc5bda072887319f2908。历史015回执记import1792151324288，2026-10-17只读实物核其archive-only目录为1792151324283；q06历史四实体证据独立保留；q08热门70、q09原虫笼1章16页/已下载P2及原书架进度有限接受。原P2复用下载，不证明新get_pages/全源/新网络/全章图片；本次017隔离标准WASM需独立绑定和Root审核，不自动继承旧Native。

既有Host证据保真实与受控分层：真实原热门/最新70和虫笼目录原体；101卡/601章、payload跨界和257标题为受控输入。016已审DisplayChapterTitle只完整展示标题；身份、页数和章卷仍由原legacy256字段派生，late-pageCount身份边界仍OPEN。旧unintegrated-chapter-title-streaming实验未集成，不能因交付整合改变章ID。

历史README标题0.1.9及后追加010/011/013/014记录是演进依据，原件保存在独立owner；0.1.9不是当前最新owner。原015最终仅整次错误传播/完整性范围及原get_pages精确保留依据见 `/private/tmp/koma-dm5-v0115-list-completeness-20261016/{REPORT.md,preserved-algorithms.json,verification.json}`；Native边界见 `/private/tmp/koma-dm5-v0115-native237-20261016/native-receipt.json`。

图标仍取本目录ASSET_SOURCES所列官方AppIcon原bytes；新标准manifest由当前打包器写必需icon字段，App显示圆角裁剪独立保留。无新网络/设备、推送或发布。

当前017完整继承owner为 `/private/tmp/Koma-sources-dm5-chapter-language-combined-20261016`，lib SHA0360fa660d0598f620c7ae4a95d445d36a5ed56029ec07a7ad3f9c2bba61a4ec。仅在完整015基础增加已审016 display标题流（不改旧ID/pageCount/章卷）和017省略无上游依据的chapter.language；不从源目录zh推断章节正文语言。原库已有持久语言不能由源省略自动撤销。

017原交付包543ed8d732f940e91c9c0ec5b9ad3eaf4a682a6189c2f798e07f91a0e4f973f0/WASMe93b8d9d75855a19c5fa31ede037f839055b52ad4a4c229b02affe72993a8e5c。Root124绑定Host审与237batch具名目录/已下载虫笼P2回执在 `/private/tmp/koma-four-source-chapter-language-20261016/ROOT_REVIEW.json` 和 `/private/tmp/koma-source-quality-batch-native237-20261016/native-receipt.json`；2026-10-17 live唯一DM5解包manifest版本0.1.17及四实体精确复核在 `/private/tmp/koma-dm50115-standard-native237-20261017/m01d/run-metadata.json`。原v017-cover目录是0.1.7，不是本017；旧DM5019标签是0.1.9，不据文件名推版本。

2026-10-26用户纠正：018首页不再仅返回两颗listingLink。保留热门/最新的既有ID、完整分页和菜单，沿同一真实目录请求/卡片解析各提供12项首页预览，错误仍整次传播，不伪装空成功；不改详情、目录、正文和既有记录。中文显示名按https://www.dm5.cn/标题“动漫屋”补齐简繁localizedNames，英文/源ID保持。尚待当前包普通入口Native验证，旧017的具名验收不外推本次首页。

## 0.1.19 章節排序設定

按固定 Keiyoushi 949dea252eb8e867b7b40834a26e13088764ff55 的 Dm5 實作補入 sortChapter，預設 false 保留原網站序列；僅使用者明確開啟時按 URL 章節數字由新到舊排列。同號保原順序，章節 ID、title、pageCount、chapterNumber、volumeNumber、readingGroup、continuationEligible 不變。設定從每次請求 settings 讀取。章末吐槽頁在 0.1.20 依下述宿主能力協作；清 Cookie 依賴 Android CookieManager，不冒作普通設定。

## 0.1.20 章末吐槽文字頁

僅頂層 hostFeatures 含 text_page_image_v1 時顯示 chapterComments（預設 false），消費時同時要求能力與使用者啟用。從原 viewer 保存 DM5_PAGEPCOUNT、DM5_TIEBATOPICID；原圖完整取得後逐頁請求網站 pagerdata.ashx 的 pageindex/pagesize/tid/cid/t=9 合同。每個回應陣列按 Poster、PostContent 產生「poster: content」段落，空陣列為「没有吐槽」，輸出固定 image.kind=text、template=dm5-chapter-comments。原圖身份和順序保留。JSON Unicode 與 HTML 實體在源側解碼，保正文標籤字面量和 CR；復用既有 HTML5 Host 表並關閉 descriptor，不加依賴庫或 Host import。損壞、缺 metadata、容量或請求失敗整次拒絕。Host 只按固定參考尺寸本地繪 PNG；缺能力時舊 App 保原 URL 頁且不請求評論。普通 CookieManager 清理仍非可儲存源設定，不冒作開關。

## komiic

# Komiic 0.1.7

Based on accepted014 commit71c0f35, not the old unadopted WIP. Cargo/runtime/package version align from 0.1.7. Only get_listings now emits the existing popular/Popular navigation DTO without HTTP; all other014 business bytes remain.

Old page:index identities, kid from original imageURI, returnedkid equality, official img.komiic.com boundary and originalcover Accept/Referer are preserved. No new ticket/image/site/Native request for this candidate. Historical197 ONEPIECE1194/12P read/download/cold belongs to014 only;015 is not freshlyNative accepted.

Use current canonical build.sh --source komiic and SDK with originalicon.png/ASSET_SOURCES provenance. No styles/icons changed. Homepage upstreampopular body and pagination remain unverified; static DTO and productionApp first-call evidence must not be treated as upstreamcontent success.


## 0.1.6 homepage repair (2026-10-27)

The user reported a blank Home and an ordinary WebView action without an address.
The old Home returned a fixed empty popular section although the exact 0.1.5 WASM
popular operation successfully returned 30 real upstream manga cards. Home now
uses that same GraphQL request and card parser, limited to a 12-card preview with
its existing popular listing destination. Full-list pagination, search, chapter
identity/order and the accepted image-ticket chain remain unchanged.

The package declares the official `https://komiic.com/` homepage for ordinary
navigation only. This does not grant verification/login origins. The existing
180px icon is byte-for-byte identical to the current official touch icon;
no asset was replaced. The website's other favicon is only 16px.

The current Keiyoushi source also offers chapter/book filtering and a mirror
choice. The 0.1.7 chapter preference is described below; other variants are not silently treated as selectable mirrors.
Current build/real Home/Native acceptance must be recorded separately below.


Local validation: canonical `build.sh --source komiic` completed and the actual
0.1.6 WASM `get_home` made one ordinary POST to the original API, returning the
12 real preview cards (including 腦洞學生會 and 魔都精兵的奴隸). This proves
current host/API behavior only; device Home/WebView acceptance remains pending.

## 0.1.7 章卷顯示設定

按固定 Keiyoushi 949dea252eb8e867b7b40834a26e13088764ff55 的 setupPreferenceScreen 補入 CHAPTER_FILTER：all（同時顯示卷和章節）、chapter（僅章節）、book（僅卷），預設 all。get_chapters 每次從 settings 讀取；只明確選 chapter/book 時過濾同型別項，原相對順序、ID、序列欄位與文字保持。無設定或未知值沿原全量；沒有套用參考排序來改既定預設。沒有新增鏡像、自由域名或帳戶權限。Cargo/runtime/package 同步 0.1.7。

## manhuagui

# Manhuagui 0.1.7

Source-only ordinary public HTML adapter for https://www.manhuagui.com, based on Keiyoushi extensions-source commit 10f06eb833af5990bc5470c84c5e93edc159ad4e (Apache-2.0). LZ-string algorithm attribution: Keiyoushi LZString (Apache-2.0), with an independent algorithm check against pieroxy/lz-string 1.5.0 (reference file licensed WTFPL v2); bounded source-local implementation.

Stable source ID com.manhuagui.koma; manga IDs remain comic/<digits>, canonical absolute chapter IDs and page:<chapterURL>:<index> remain unchanged. Catalogue uses actual section order with ul groups reversed within each section; unknown chapter numbers and dates stay null. Public work ratings are unknown; source-wide nsfw conservatively reflects upstream mixed content. Usage is unknown.

Popular/latest paging, text search, detail, public catalogue, fixed Base64-LZ/radix62 dictionary-decoded page metadata and ordinary image requests are implemented. Optional home/filters/credentials/deeplink are not implemented. Rate-limit preferences require the explicit host feature below. Fixed initial Chrome 140, Accept-Language, Referer and Origin match the reviewed parent contract. No cookies, hidden VIEWSTATE decoding, adult access, login-check/vote side effects, mirror/UA switching, JavaScript evaluation or image-byte decryption. Unknown packed forms, malformed/truncated data and capacity overflow fail explicitly.

Older hosts without source_rate_limit_v1 receive no rate-limit settings; the host-owned scheduler contract is below. HTTP compression parity (br/gzip/zstd) is unproven. This candidate has local code/build gates only; no current source-site or physical-device chain has been accepted. Old tree connection/SSL timeouts are separate transport history, not HTTP rejection evidence.

Source version is 0.1.6; minimum App version remains 0.1.0 (registry fourth field). These are independent. The previous candidate incorrectly required App 0.1.1; root observed native install_blocked/app_version_unsupported on App0.1.0 at17:04:49 and17:05:52, with no package request. Root owns that physical failure evidence. No App check/version/SDK change is needed.

0.1.2 adds finite top-level failed-host-envelope classification only when no valid numeric HTTP status is present. Root observed 0.1.1 canonical popular timeout at17:14:58→17:15:13, status0 and no HTTP response/body. Network failures and native timeout now remain network_error/timeout with fixed privacy-safe messages. Existing numeric status/statusCode exchanges, including ok:false, retain HTTP2xx/403/429 behavior; no retry or request contract change.

## Official desktop identity (2026-10-20)

0.1.3 adds only the site's explicitly distributed desktop shortcut icon and a delivery version bump; source requests, parsing, capabilities, source ID, language and minimum App version remain unchanged. The user requested clear, officially recognized identity resources while preserving App icon slots, rounding and list order.

The readable official homepage https://www.manhuagui.com/ links “设为桌面图标” to https://www.manhuagui.com/support/shortcut.ashx. That successful response names `URL=https://www.manhuagui.com/?desktop`, `IconFile=https://www.manhuagui.com/support/naruto.ico` and `IconIndex=0`. It also contains a second Windows SHELL32 IconFile field, so no claim is made about which field Windows ultimately chooses. The linked original ICO returned 200 and contains 128/64/48/32/16px frames (99678 bytes). `icon.ico` preserves that complete original; no crop, resampling, redraw, added background, shadow or mask. Root visually reviewed the clear Naruto portrait; it is the site's distributed shortcut identity, not an exclusive Manhuagui character trademark. The brown frame and highlights belong to the original artwork.

This supersedes only the missing-resource status. The earlier 318-byte 16px conventional favicon remains rejected because its brand binding was not established; it has not been restored. A current Root fetch of the shortcut timed out, distinct from the earlier successful original response. 0.1.3 was imported through the ordinary system file picker on device197 (ALN-AL80, portrait1260x2720); Root reviewed both source-manager and Browse screenshots. The original portrait is legible at the existing slot size and the approved rounded corners remain; its row stays between SixMH and Manga1234. This accepts those two icon consumers only, not the source reading chain or all source icon quality.


## 2026-10-21：0.1.3 具名 Host / 237 内容后继

使用现有 `koma_manhuagui_source.wasm`（无网络 info 读回 0.1.3，且与既有 `manhuagui-0.1.3.koma` 内 source.wasm 逐字节相等，未重建）按原合同唯一一次 Host 热门请求 `GET https://www.manhuagui.com/list/view_p1.html`，返回 HTTP 200 / html_ready，42 个真实条目、nextCursor=2、hasMore=true。原件在 Koma `.hvigor/outputs/user-flow-audit/manhuagui013-current-popular-host.json` 及同名 `.stderr.log`。Host 成功不能替代 Native；原 0.1.1 timeout/status0 历史不撤销，不归因为已修复。

237 初始未安装本源；按原授权普通系统文件选择器导入现有已审 0.1.3，成为准确第 50 项 `com.manhuagui.koma`，原 49 项 sourceId/enabled 整段前缀完全保持。普通 Browse 一次热门入口实际 HTTP 200，显示 42 本；首屏《鬼灭之刃》封面仍占位，一拳超人、ONE PIECE、斗破苍穹、斗罗大陆、百炼成神五张实际封面可见，不称全封面通过。进入首卡《鬼灭之刃》comic/19430，详情及 235/235 章目录均 HTTP 200。普通“开始阅读”按当前目录进入第 23 卷（不是第 1 卷），准确 chapterId=`https://www.manhuagui.com/comic/19430/585116.html`；get_pages HTTP 200 / payload_ready，真实显示 1/230 卷封，经两次原翻页动作显示 2/230 扉页及 3/230 人物介绍/前情内容，原图已亲看。只接受该版本、设备及具名路径/三页，未验整卷读完、其它章、分页、下载或冷启动；封面占位原因未知，未重试图片或换 UA/域。

普通 Back 三层回到 Browse 末端；本源进入前 reader 对象为空，本次正常阅读保存同一第 23 卷 pageIndex=2 / totalPages=230、completed=false、columnMode=odd_left，并新增相应源作品元数据，未注入或直接删除持久状态。仅删除本人传入的 manhuagui-0.1.3.koma，Download 读回已无此包；最终原 49 项前缀仍完全相同，仅末尾新增本源 enabled=true。OverrideTimeout 恢复 10000ms 且读回 AWAKE，租约已释放，197 未操作、哈希关闭。原图/原码在 Koma `.hvigor/outputs/user-flow-audit/device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership/` 的 `manhuagui013-{manager-open,import,manager,browse,popular,detail,reader}237-001`、`manhuagui013-page{2,3}-237-001`、`manhuagui013-session-restored237-001`。本轮无 App/源实现、图标、能力或请求合同改动。

Root 后继 237 封面诊断（同日 07:27–07:29）：保持现装包、缓存、请求合同和界面，普通从 Browse 再进该源。热门仍为42本，首屏六张（含《鬼灭之刃》）现在均显示真实封面；Root 亲看 `manhuagui-cover237-terminal-001/screen.png`。当前有界原日志只捕获列表 HTTP200，未取得首次占位的 HTTP/解码失败码，因此原反例保留为未复现、原因未定，不能由单次Host200认定App缺陷或已修复。未修改任何加载代码/素材/源包，未清缓存、未换UA、未新增外部请求测试。返回原Browse末端，读回AWAKE/OverrideTimeout10000ms并释放Root短租约。证据沿同一设备分区的 `manhuagui-cover237-{wake,home,ready,terminal,session-restored}-001`。

## 0.1.4 optical identity framing (2026-10-21)

The user explicitly requested replacing the inset book-shaped 0.1.3 identity because its apparent size and corner relationship looked inconsistent in the source list, and approved the full-bleed portrait candidate. Current `icon.png` is a 512px export of an image-generated reconstruction based on the site's distributed Naruto shortcut portrait above. It fills the square without the old book spine, bevel or transparent frame, allowing the App's existing rounded slot to define its boundary. It is a derived portrait, not an official high-resolution original or a pixel-identical crop; fine drawing details differ. No external background tile, mask, shadow or pre-rounded corners were added. The source behavior, requests, capabilities, language and minimum App version are unchanged; the version bump delivers the replacement asset through ordinary source updates. Device237 (VDE-AL00, unfolded1320×2120) ordinary local import completed as an update to the existing 50th source. Root reviewed both source-manager and Browse screenshots: the full-bleed portrait has the same App-defined rounded boundary as adjacent square icons. All50 source IDs/enabled states retain their exact order, and only this version becomes0.1.4. The sole uploaded014 package copy was removed afterwards; the original10000ms timeout was restored and read back AWAKE. Evidence: Koma `.hvigor/outputs/user-flow-audit/device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership/manhuagui014-{import,manager-visible,browse-restored}237-001`. This accepts only the two icon consumers; no source reading/download/cold result is implied.


## 0.1.5 详情来源网页（2026-10-21）

`get_manga` 新增 `links:[{kind:"source",url:<作品页>}]`，地址复用原 `manga_doc` 的 `BASE + /comic/ + 已验证原网页编号 + /`。例如《鬼灭之刃》为 https://www.manhuagui.com/comic/19430/；不使用站首页或猜造 App 内部 ID 对应网址。除交付版本014→015外，请求、身份、章节、阅读和图标保持。

Host 按原源声明头单次读取该页200/53615B；014基线WASM与原正式包相同，014/015实际WASM在同一真实HTML上离线详情成功，除新增links外所有manga字段、唯一请求合同深比较相同。离线适配器使用禁网惰性Chromium DOM查询，不代表生产Lexbor/scraper或Native等价。单源offline/locked构建、真实WASM info015及图标门禁通过；正式包 `dist/sources/manhuagui/manhuagui-0.1.5.koma` 为372036B/三成员，沿014manifest仅版本变化，图标原bytes保持。

237（VDE-AL00，展开竖屏1320×2120）普通本地选择器014→015更新成功；随后覆盖安装Root指定16289728B/2026-10-21 22:29:30签名Debug，未清数据。《鬼灭之刃》235/235章详情两网页菜单可见；仅执行WebView，终态显示真实封面、作者、9.5评分和卷章列表及导航控件。原003白屏转圈只证明标题到达，未作为内容通过；004等加载结束后才接受终态。自然Back仍第23卷3/230页，再返回Browse；服务页54个源包，新名称序中漫画1234→漫画160→漫画柜015→漫画人可见。未切换启用/索引、未导出私有registry，不冒全54条字段逐项比对；未执行外部浏览器、正文、下载或全源矩阵。最终10000ms/AWAKE读回、本人上传包不存在，租约释放。Root已亲看004五张原图，结论仅限上述路径。

Host原件在Koma `.hvigor/outputs/user-flow-audit/` 的 `manhuagui015-detail-fixture.html`、`manhuagui015-detail-fixture-receipt.json`、`manhuagui014-link-baseline-offline.json`、`manhuagui015-link-offline.json`。237原件在同目录 `device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership/`：`manhuagui015-import237-002/import-success.png`、`manhuagui015-app-update237-001/run-metadata.json`、`manhuagui015-webview237-004/{both-web-actions,webview-workpage,detail-returned,services54-newapp,browse-restored}.png`；最终恢复回执 `manhuagui015-webview-postflight237-004/run-metadata.json`。


## 0.1.6 声明目录分组与卷/话metadata（2026-10-23，本地候选）

修改依据是原成功公开详情 `https://www.manhuagui.com/comic/19430/` 的53615B HTML及200回执，现存于Koma `.hvigor/outputs/user-flow-audit/manhuagui015-detail-fixture{.html,-receipt.json}`。原页明确说明单行本卷包含杂志连载话，并以h4分出单行本/单话/番外篇；两后组均复用chapter-list-1，不能拿DOM id当阅读组。网站“开始阅读”明确指向 `/comic/19430/219425.html` 第01回，而不是数组首项第23卷。

只消费同一目录owner的直接h4/list声明：单行本明确“第N卷”单位标签只填volumeNumber字符串；单话明确以“第N回/话”单位标签开头的title只填chapterNumber，后续题名原样保留。各组有独立readingGroup，只有明确数字标签才continuationEligible=true；番外/未知分组或歧义标签不猜主序、false且保留所有手动目录。日期仍未知，不将整书“最近更新2021-08-21”复制给每章。原稳定ID、全部title/index、section顺序、每section逆UL的原输出、get_pages/HTTP/网页字段/图标和最低App版本保持；不新增首章API、不用数组位置冒充章号、不全表reverse。

版本015→016仅本parser及交付版本。原015正式包与016在同一真实HTML、当前生产CppHtmlHost/Lexbor上离线回放均235唯一href；全部ID/title/index/数组顺序、get_manga完整输出及HTTP请求保持。016给23卷/206单话/6番外分组；当前实际parseMangaChapterFromSourceItem/firstReaderChapter/continuationChapterIds消费后首章是上述219425，卷1→23、单话1→206互不串组，番外各仅自身。另三份受控原HTML派生只验证未知h4/非单位标签前置年份/真实零号边界，绝不冒当前上游或Native接受。单源offline/locked release编译通过，未联网、改App/SDK或部署。

本地回放与精确diff原件 `/private/tmp/koma-manhuagui016-metadata-20261023/`。这只证明该原HTML的源与当前纯消费函数；016有限普通更新结果见下一段，既有收藏metadata正常落库、首次阅读/分组续章硬件仍未证。当天237现装015热门首批唯一请求15秒平台2300028/network_error/http0失败保持，未触页2、未重试；016不改网络合同，不作为该超时或分页修复。

237有限普通验收（2026-10-23）：

- **结果**：同一系统本地picker将015更新为016，原生显示“导入成功”；安装实体manifest为com.manhuagui.koma/0.1.6且仍启用。原样本是sourceHistory中的comic/19430而非收藏；只打开该项第23卷585116、第3/230页，未触同名其他源或新增收藏。
- **实际失败**：get_chapters为15150ms network_error/http0，随后App自动get_pages为15058ms network_error/http0，原生终态“页面目录加载失败”。没有HTTP/HTML，不能归因为CF或parser；未点击重试。
- **恢复与边界**：普通Back后同项仍第23卷3/230页，再恢复原History首屏，10000ms/AWAKE读回，本次自传385962B包及capture scratch已清、租约释放。本次不作为235目录、metadata落库或正文Native通过；237仍旧App，不代表Root仅197安装的metadata持久化候选已验。完整会话文件后置导出被自动审批拒绝且未执行，进度保持以同实体普通历史UI前后为证。
- **证据**：原件在Koma `.hvigor/outputs/user-flow-audit/device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership/manhuagui016-native237-`各run：`picker-select016-012`、`history-scroll-016`（安装manifest接收）、`history-target-anchor-022`、`resume-original19430-023`、`resume-diagnosis-025`、`failure-preserve-026`、`return-original-history-027`、`restore-history-top-028`、`restored-cleanup-029`。精确scope及回执集中于 `/private/tmp/koma-manhuagui016-metadata-20261023/`。

## 0.1.7 源級請求額度設定

固定 Keiyoushi 949dea252eb8e867b7b40834a26e13088764ff55 的兩條滑動窗口：mainSiteRatelimitPreference 選 1..10，預設 10，每 10000ms 用於精確 www.manhuagui.com；imgCDNRatelimitPreference 選 1..10，預設 4，每 1000ms 由 i.hamreus.com 與 cf.hamreus.com 共用一份額度。生產 BASE 固定 https://www.manhuagui.com，沒有 baseUrl 設定；正文目前輸出 i.hamreus.com。僅頂層 hostFeatures 含 source_rate_limit_v1 時公開兩項。Host 於 HTTP 與圖片快取 miss 共用同一調度 owner，快取 hit 不耗額度，不把設定冒作 HTTP header。R18、帳戶與 hidden VIEWSTATE 能力未因此改變。

## webtoons_zh_hant

# WEBTOON (Traditional Chinese) 0.1.6

Independent source `com.webtoons.zh-hant.koma`, content language `zh-Hant`.
Canonical name remains WEBTOON (Traditional Chinese); localized display names are optional metadata.
English `com.webtoons.koma` and Simplified-Chinese Dongman are separate sources; no aliases, migration or shared-history claim.

Current Keiyoushi protobuf identity is 2959982438613576472, extension all.webtoons 1.6.2,
fixed primary revision 10f06eb833af5990bc5470c84c5e93edc159ad4e (Apache-2.0).
Upstream labels the extension SAFE: packaged nsfw=false/sourceInfo safe describes that source-wide label,
not a classification or acceptance of every work. Actual usage is unknown.

Public contract: four finite popular ranking buckets trending/popular/originals/canvas, desktop detail,
mobile episode JSON and ordinary viewer `#_imageList` images. Original href/title/query URLs are preserved.
Manga identity includes language, webtoon/canvas type, actual title_no and actual list path.
Chapter identity retains the exact viewerLink (including encoded Chinese path and episode_no);
page identity includes the actual title, episode and page index. Explicit Chinese title numbers only;
unknown chapter numbers stay null. Primary catalogue order is explicitly reversed, with index as order only.
Complete catalogue requires nextCursor zero/null and at most 2048 records; no truncated-success fallback.
All-types search is limited to a terminal first page; an advertised next page is explicitly unsupported,
never returned with a fake terminal cursor. Search has not had live acceptance.

No latest/day scheduling, filters or MotionToon,
login/paid/account access, ageGatePass/needGDPR cookies, automatic retry, challenge/mirror behavior.
Requests use the existing SDK Firefox121 and fixed public Referer/Origin; mobile catalogue has no Origin.
Image descriptors serialize only literal ASCII space as %20 for transport, preserving all other UTF8 and escapes.
Stored cover/page URI, IDs and public desktop headers remain unchanged, with no cookies.
197 observed five Chinese-filename covers cached but literal post sum.png failed with URL-format code2300003.
0.1.1 transport serialization still awaits native acceptance; frozen0.1.0 evidence is preserved.
HTML and output limits, UTF8/JSON/URL checks reject malformed, unknown, oversized or gated structures.
Logging contains only fixed stage/status/error values, never URLs/titles/queries/body/headers/account values.

Local source-owned packaging (public build/validator stay bound and unchanged):
`python3 sources/webtoons_zh_hant/package_candidate.py --runner /absolute/path/to/local-offline-host`
The wrapper uses paired Rustup stable compiler/cargo in offline mode, reads actual WASM info, validates metadata,
and creates a deterministic two-file .koma plus selected-only index with the exact package SHA.
No icon: the retained accepted English WEBTOON archive contains no icon; none is fetched or invented.

Retained live evidence: one trending GET (30 titles), title5486 detail, its JSON catalogue (154 episodes),
and the earliest ordinary 第1話 新手教學(1) viewer HTML (128 original image addresses).
The first image is the genuine site warning image, retained in place. No image was requested.
These bodies support offline production WASM replay only, not native/image/reading/download acceptance.
197 ordinary acceptance belongs to the root agent after exact source/package review.

## 0.1.3 详情来源网页

get_manga的detail仅新增kind=source链接，复用同次doc(manga_url(id))的原manga_url，保留真实语言path及title_no，未从API或App复合ID猜造。d0422d5按原件恢复时详情没有网页字段，未发现有意排除网页链接的记录；原理由未知，本次用户网页入口授权只允许此字段、交付版本及文档。runtime012→013，Cargo/lock旧011→013；旧012图标后继与metadata均保持。

原5486《變成蠻族！遊戲生存戰》作品页https://www.webtoons.com/zh-hant/fantasy/surviving-the-game-as-a-barbarian/list?title_no=5486的单次200/47936B原体，其canonical和og:url均精确等同该地址。只读取已有保全包remaining-materials原件到Koma .hvigor/outputs/user-flow-audit/webtoon-hant013-detail-fixture.html及同名前缀detail-fixture-original-{attempt,receipt}.json；未发新请求，未遇或绕挑战。原Native012是197《武士萬里行》255章、第2季65話P2/106及cold；它与5486原Host合同分开保留，不由此新增013或5486 Native/阅读接受。

单源offline/locked release构建及真实WASM info013通过；当前源码重编译runtime012与保全原正式012两种实际WASM均用同一5486原HTML和013比较，除新增links外全部manga字段、唯一HTTP请求深相等。禁网惰性Chromium DOM适配只证明该离线输出，不冒生产Host/Native等价；原件沿上述Koma产物分区`webtoon-hant012-{link-baseline,formal-original}-offline.json`与`webtoon-hant013-link{,-formal-compare}-offline.json`。正式`dist/webtoons-zh-hant-0.1.3.koma`29656B三成员，完整保留metadata.json名称；manifest继承012，仅version变更并按当前build.sh合同补入既有icon.png字段（原012含图标成员但未声明该字段）。6252B icon.png与保全原012成员逐字节相等；前文“无icon/两成员”仅是旧011阶段，013不删除012已交付原图标。未改请求、身份、章节/图片/传输合同、App或设备，013普通具名观察见下。

237 VDE-AL00展开竖屏1320×2120，复用现装Debug16289681B/23:30:52，经系统picker012→013普通更新明确成功，服务54包及最终Browse54已启用保持。普通热门一次返回30本，当前首屏未见5486，随后只在本源既有搜索框输入一次准确标题“變成蠻族！遊戲生存戰”，沿真实结果进入5486详情，现155/155章、开始阅读。两网页菜单可见；仅打开WebView，加载完成后亲看准确作品页标题、蓝色banner、作者、简介和章列表，无登录/挑战。自然Back详情仍155/155章及开始阅读，再回搜索结果/Browse；未打开正文、外部浏览器、下载或改变该作读态。旧197《武士萬里行》正文/cold不混入本次网页接受；未导出全库或据此宣称全部阅读位置逐项校验。仅本人传包已清理并读回不存在，最终10000ms/AWAKE并释放租约。

原图与回执沿Koma `.hvigor/outputs/user-flow-audit/device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership/`：`webtoon-hant013-import-list237-001/{import-success,services54}.png`，`webtoon-hant013-source237-002/{browse-unoccluded,source-list-terminal}.png`，`webtoon-hant013-webview237-004/{search-result-unoccluded,detail-before,both-web-actions,webview-workpage,detail-returned,browse-restored54}.png`，收尾`webtoon-hant013-webview-postflight237-004/run-metadata.json`。首次Browse点击落在导航遮挡区而切书架、随后搜索键盘联想导致重复标题断言均已依实际界面纠正，未重导/重搜，原失败产物保留，不作产品故障。

## 0.1.4 圖片品質設定

按固定 Keiyoushi 949dea252eb8e867b7b40834a26e13088764ff55 的 Webtoons 實作補入 useMaxQuality（預設 false）。僅明確開啟時移除章節圖片 URL 完整的 type=q90 查詢對，其餘參數、escape、fragment、頁面 ID 與順序保留；預設 URL 不變。設定從每次請求的 settings 讀取，不引入不存在的 Host import。作者備註需以下 0.1.5 宿主能力協作，舊 Host 不放置無效開關；0.1.5 另補 useSequentialNumbering，預設 false 保原明確數字/null；僅使用者啟用時按原 episodeList 的 index+1 對應反轉後 chapterNumber=count-outputIndex，不改 ID/title/index/排列。

## 0.1.5 作者備註文字頁

僅頂層 hostFeatures 含 text_page_image_v1 時顯示 showAuthorsNotes（預設 false）。使用者啟用後從同次普通 viewer DOM 的 div.creator_note p.author_text 與 div.creator_note .author_name span 取真實文字，非空才在原圖末追加 image.kind=text、template=webtoon-author-notes、heading="Author's Notes from {creator}"、paragraphs=[note]。DOM 已完成實體解碼，Host 只負責固定本地文字轉 PNG，不外發文字。原圖順序、ID、URL 保持；缺能力即使已存 true 仍不輸出 text，缺備註不造空頁。

## 0.1.6 Canvas 繁中實際 viewer 路由

真實繁中 Canvas 榜首 title_no=1127879 的手機目錄 API 200 返回 13 話，其 viewerLink 使用 /en/canvas/.../viewer 且帶 readingLanguageCode=zh-hant。舊版僅允許 /zh-hant/，導致已宣告的 Canvas 作品目錄整次報 invalid language route。現僅加允許同官網的這個 viewer 形式，readingLanguageCode 必須唯一且精確 zh-hant；不允許任意英文/其它語言路由，不改寫網站原章節 ID、query、排列、元資料或既有 /zh-hant/ 作品。
