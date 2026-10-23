# Manhuagui 0.1.6

Source-only ordinary public HTML adapter for https://www.manhuagui.com, based on Keiyoushi extensions-source commit 10f06eb833af5990bc5470c84c5e93edc159ad4e (Apache-2.0). LZ-string algorithm attribution: Keiyoushi LZString (Apache-2.0), with an independent algorithm check against pieroxy/lz-string 1.5.0 (reference file licensed WTFPL v2); bounded source-local implementation.

Stable source ID com.manhuagui.koma; manga IDs remain comic/<digits>, canonical absolute chapter IDs and page:<chapterURL>:<index> remain unchanged. Catalogue uses actual section order with ul groups reversed within each section; unknown chapter numbers and dates stay null. Public work ratings are unknown; source-wide nsfw conservatively reflects upstream mixed content. Usage is unknown.

Popular/latest paging, text search, detail, public catalogue, fixed Base64-LZ/radix62 dictionary-decoded page metadata and ordinary image requests are implemented. Optional home/filters/settings/credentials/deeplink are not implemented. Fixed initial Chrome 140, Accept-Language, Referer and Origin match the reviewed parent contract. No cookies, hidden VIEWSTATE decoding, adult access, login-check/vote side effects, mirror/UA switching, JavaScript evaluation or image-byte decryption. Unknown packed forms, malformed/truncated data and capacity overflow fail explicitly.

The existing ABI cannot express upstream cross-call main-site 10 requests/10 seconds or image 4/second limits; no equivalent limiter is claimed. HTTP compression parity (br/gzip/zstd) is unproven. This candidate has local code/build gates only; no current source-site or physical-device chain has been accepted. Old tree connection/SSL timeouts are separate transport history, not HTTP rejection evidence.

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
