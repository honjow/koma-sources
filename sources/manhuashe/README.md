# 漫画社 public adapter

Contract reference: Keiyoushi extensions-source `src/zh/manhuashe/.../Manhuashe.kt`, Apache-2.0, revision `10f06eb833af5990bc5470c84c5e93edc159ad4e`. https://github.com/keiyoushi/extensions-source/blob/10f06eb833af5990bc5470c84c5e93edc159ad4e/LICENSE

Stable source id and old canonical name remain unchanged. Manga `manga:<original path>`, chapter `chapter:<original manga path>:<original chapter path>` and page `page:<zero-based index>` identities remain compatible. The primary's explicit catalogue reversal is preserved. Through 0.1.5 chapterNumber was null; index represents only that original order.

Upstream MIXED is conservatively mapped to current nsfw=true/contentRating=nsfw, not a statement that every title is adult. Individual work ratings and actual usage are unknown. No auth, cookie bootstrap, mirror fallback, arbitrary JS, image bytes decrypt or automatic retry. Missing/ambiguous/oversized parsing inputs fail. Normal public HTML contract only; build and host gates do not establish physical acceptance.

Current production entry is `bash build.sh --source manhuashe` using the current canonical SDK and formal CLI. The historical package_candidate.py remains in the original owner only; it is not imported into canonical. Its stale registry correction is represented by this source's nsfw=true row. Minimum App version remains 0.1.0. The original official homepage-declared JPEG is now retained unprocessed as icon.jpg; see ASSET_SOURCES.md. This closes only the package resource prerequisite; small-slot visual suitability remains unaccepted. No publication.

0.1.2 adds only the primary fixed public image-request Referer/Origin descriptor. It preserves the input HTTP(S) URL and sets no User-Agent/cookie. Descriptor success does not prove image download/rendering or fix the observed 197 failure.


Historical 197 0.1.2 accepted only the named 一人之下/767 坦率 17-page route, P1/P3, single download and no-clear cold/Library continuation. That historical native success does not certify the new standard WASM or current website. Original five retained-response results and image descriptors survive; their three raw HTML fixture bodies currently do not, so current-SDK HTTP/DOM equivalence is unknown. This integration changes no parser, ID or public image headers. App display clipping remains independent and unchanged.


Official 150×50 JPEG horizontal logo is source-owned artwork, not code licensed by this adapter. No crop, resize, recoloring, backing removal, shadow/mask, redraw or format conversion is performed. The URL ends .png and the server labels image/png, but the actual original bytes decode as JPEG/RGB with no alpha; package name icon.jpg follows the actual format. App display clipping is separate and unchanged.

## 237 现装 0.1.2 具名在线观察（2026-10-21）

237 VDE-AL00 展开竖屏1320×2120，普通 Browse 行实际显示 0.1.2；不能把197的资源交付或本地产物文件名当作237版本。普通热门返回20本，《一人之下》详情及目录显示800/800章。原767《坦率》行已有下载，本次没有重开它作为在线证据，而打开未下载的766《反杀》。首帧仍为图片解码中，随后只读观察亲看真实中文正文1/17；普通右侧下一页后亲看不同正文2/17。

自然 Back 后原读档记录绑定 comicId=`com.manhuashe.koma:manga:/comic_13871.html`、chapterId=`chapter:/comic_13871.html:/chapter_13871_1342899.html`、pageId=`page:1`、pageIndex=1、totalPages=17、completed=false。未直接修改持久状态、未收藏或新增下载。原Browse末端完整画面已亲看，50项源registry与进入前完整相同，原10000ms/AWAKE读回。只接受这个现装版本的具名列表、目录、首两页与自然返回；没有核验安装包字节身份，不外推当前源码WASM、其他章节、下载、冷启动或全源。

原件沿 Koma `.hvigor/outputs/user-flow-audit/device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership/` 的 `manhuashe012-popular-terminal237-001`、`manhuashe012-yirenzhixia-chapters237-001`、`manhuashe012-yirenzhixia766-{first-ready237-001,page2-237-001,natural-return237-001,session-restored237-001}`。最初定位回执误名为manhuashe013-browse237-001，实际界面明确012，保留原名且不据此升级版本。一个日志命令被参数校验阻止，后一次-x/-z组合返回多命令错误，均不算有效日志或产品故障。

Root 已亲审本条首两页原图及精确记录；结论仅限上述具名路径，原未闭边界保持。


## 0.1.4 详情来源网页（2026-10-21）

`get_manga` 的实际 `detail` 输出新增 `kind=source` 链接，复用同一 `fetch(mp)` 请求的 `BASE + mp`，其中mp由原网页href身份经manga_path验证。例如已有237正文成功的《一人之下》对应 https://www.311s.com/comic_13871.html；不从App复合ID猜造网址，不填站首页。原244d63b业务及5965937的013图标交付保持，版本013→014只供普通更新；请求、身份、章节、正文、图标和App不动。

按现源Firefox121/Referer/Origin合同对该准确作品页单次Host读取200/175915B，未请求已403首页或处理挑战。原件在Koma `.hvigor/outputs/user-flow-audit/manhuashe014-detail-fixture.html` 与 `manhuashe014-detail-fixture-receipt.json`。013基线WASM逐字节等原正式013包。013/014实际WASM在同一真实HTML上离线详情均成功，除新增links外全部manga字段深比较相同、唯一HTTP请求合同深比较相同；输出为同目录manhuashe013-link-baseline-offline.json和manhuashe014-link-offline.json。离线适配器仅用禁网惰性Chromium DOM查询，不代表生产Lexbor/scraper或Native等价。单源offline/locked构建及真实WASM info014通过，正式dist/sources/manhuashe/manhuashe-0.1.4.koma三成员读回，原013 manifest核心字段保持，补入旧013包漏打的本源既有localizedNames（zh-Hans漫画社、zh-Hant漫畫社），icon.jpg原bytes保持；未运行会清空dist的全量脚本。普通实机结果见下，仅限具名路径，不由Host或离线输出替代。


237 VDE-AL00（展开竖屏1320×2120），现装Root指定16289728B/22:29:30 Debug，普通本地选择器012→014更新后重导已补既有中文名的29732B包；服务54包、Browse54已启用可见。当前行实际仍显示“漫画社 (Manhuashe)”v0.1.4，不把manifest含中文名冒运行名称已切换，未改App/源名。《一人之下》两网页菜单可见；仅执行WebView，等待加载完成后显示真实封面、标题、评分9.9与人气171740。自然Back回详情800/800章，原“766反杀・第2/17页”保持，再返回源列表/Browse。未开正文/外部浏览器、未刷新/登录或遇站点拒绝，不扩全源接受；原图标小槽问题仍OPEN。最终10000ms/AWAKE、本人唯一传包不存在均读回，租约释放。

原图在Koma `.hvigor/outputs/user-flow-audit/device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership/manhuashe014-webview237-004/` 的 `{browse014,both-web-actions,webview-workpage,detail-returned,browse-restored}.png`；最终恢复回执为 `manhuashe014-webview-postflight237-004/run-metadata.json`。`detail-before.png`仍在加载，不作终态；服务54包原图在 `manhuashe014-webview237-002/services54.png`，54启用原布局在 `manhuashe014-webview-postflight237-003/layout.json`。

同版本metadata局部刷新边界：上述重导后旧Browse行仍为raw名称；随后237不清数据冷启（PID8283→stop后空→37211）后，Browse实际恢复“漫画社”v0.1.4，54个已启用保持。仅检查列表，未进入源页或请求站点；这不改变前述网页通过结论，也不证明同进程同版本行会立即刷新。原图/回执沿同一category-membership分区的`manhuashe014-name-cold237-001/{enabled54.png,run-metadata.json}`、`manhuashe014-name-cold237-003/browse-name.png`和`manhuashe014-name-cold-postflight237-003/run-metadata.json`。最终10000ms/AWAKE并释放租约；自动化的不存在scroll调用与重复浏览文本断言失败原件保留，不计产品故障。

## 0.1.5 user-approved icon derivative

The user explicitly approved this resource treatment from source-icons-four-proposals.png on2026-10-22. Uses the approved744×612RGBA yellow stylized漫/chocolate-brown speech-bubble derivative, closing the former right-hand connection smoothly and omitting画社/English. identity-reconstruction.svg preserves the smooth geometric input. The complete official150×50JPEG is preserved as official-header-logo-original.jpg; it is not an official independent original or exact glyph restoration. This explicit selection supersedes the earlier no-extraction/connected-wordmark hold. No additional background plate, shadow or external mask is added. App30vp/R7 and source order remain unchanged. Only source resource delivery versions are aligned to0.1.5; provider requests, identities, chapters/pages, localized metadata and other business behavior are unchanged. Offline/locked single-source build and actual WASM info0.1.5 passed; the52592B standard package has exactly manifest.json/source.wasm/icon.png, complete registered rating/minAppVersion/localized metadata and the approved candidate bytes. Icon validation and diff checks passed without hashes; dist/index is unchanged. Root completed the ordinary237 picker 0.1.4→0.1.5 update and reviewed the complete light-theme manager/Browse images: the approved identity is complete and clear. Evidence: Koma `.hvigor/outputs/user-flow-audit/device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership/three-icons-package237-001/{manager-manhuashe015,browse-manhuashe015}.png`. This accepts only these two light-theme icon consumers; it does not establish dark-theme rendering or source content. Final registry readback: 59→60 entries, only H-Comic newly added; the original relative order and all unrelated entries remain unchanged. Manhuashe/MH160 changes are limited to their expected package references and versions. Original system theme/Browse group and10000ms/AWAKE/window208 were restored, only the three uploaded installer files were removed and confirmed absent, and the lease was released. Receipts: `three-icons-final-restored237-001` and `three-icons-package237-001/browse-original-restored.png`. This does not establish source-content or reading acceptance.


## 0.1.6 explicit chapter numbers

Only titles explicitly prefixed with `第N话` or `N <chapter title>` now provide chapterNumber. Announcements and other unnumbered titles remain null; this does not infer dates, seasons, volumes or numbering from index. Original chapter IDs, titles, catalogue reversal/order, index, groups, requests and reading continuation boundaries remain unchanged. MH160 is not changed.

The actual Rust helper passed all 800 titles from the retained successful `manhuashe014-detail-fixture.html`: 786 explicitly numbered entries and 14 announcement/activity entries left unnumbered, plus nine negative cases covering seasons, volumes, extras, dates and overflow. Single-source offline locked WASM build passed; these checks do not substitute for ordinary App update, sort or first-reading acceptance, which remain pending.


## 0.1.7 delimiter-free chapter labels

The 237 ordinary update to 0.1.6 exposed a real counterexample: 大奉打更人 has 672 entries including `669阴物`, `624“鸡”不可失`, and numeric-only `583`/`571`. These explicit leading labels now provide chapterNumber too; the original spaced form including `721 TA` remains accepted. Season/volume/date prefixes, announcements, leave notices, events and extras remain unnumbered. IDs, catalogue reversal/order, index, groups and all requests are unchanged; no MH160 change.

The retained successful 800-title catalogue plus this same exact work’s single successful production-Host result (`manhuashe016-dafeng-chapters-host.json`, HTTP200) passed all 1472 actual-title cases: 786/800 and 671/672 numbered, respectively; 13 negative cases passed. The initial stricter candidate rejected the real spaced `721 TA` and was corrected before acceptance. Ordinary 0.1.6 first reading had no prior history for comic_13836 and selected `第1话 001牢狱之灾` (chapter_13836_17244), showing the complete P1/24 cover and saving P1/24 on natural return; 0.1.6 numeric sorting failed on the unnumbered newer labels, and is not marked passed. 0.1.7 ordinary update/sort verification remains pending.


237 ordinary picker 0.1.6→0.1.7 update succeeded, with Browse and registry both reading 0.1.7. In the same 672-entry work, descending now begins 669/668/667; ascending shows the unnumbered 请假条 followed by chapters 1/2/3. This accepts the corrected numbered metadata and numbered ordering only: the current App’s treatment of unknown chapter numbers remains an observed issue, not a full sorting pass. Evidence is Koma’s existing237 `category-membership/manhuashe017-number237-{desc,asc-oldconsumer}-001/{screen.png,layout.json}`; the earlier first-reading and natural-return evidence is `manhuashe016-number237-{firstread,firstreturn}-001`.

`manhuashe017-number237-final-restored-001` retains the normal Browse anchor, 63 source records in the original relative order, and only this source’s version/package references changed. All previous progress records are unchanged; only comic_13836’s natural P1/24 progress was added. The temporary order override was reset through the ordinary control; AWAKE/10000ms and absence of both uploaded installers and both scratch files were read back before lease release. Source image/identity, App UI and MH160 were not modified by this source delivery.

## 0.1.9 显式镜像设置

固定参考 949dea252eb8e867b7b40834a26e13088764ff55 的镜像选择首项为 `https://www.311s.com`，另可选 `https://www.m206.com`。`preferredMirror` 默认原地址，未设置或未知值沿旧请求；仅用户显式选择备用地址时，列表、搜索、详情、章节、正文及图片请求 Referer/Origin 使用该地址。不自动试探或切换镜像；旧路径形式的漫画和章节 ID 保持。两站是否对每个作品均同一身份不能推定。

备用站实际返回的搜索第一页无 `/1` 链接；仅将精确匹配查询路径且无页号的链接识别为第一页，其他分页仍校验精确查询和页序。现有用户身份、图标和默认域不变。仅真实站点/WASM与具名旧作品测试，普通 Native 设置持久和设备阅读仍需独立验证。
