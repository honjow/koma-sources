# MYCOMIC 0.1.30

独立公开HTML适配 `com.mycomic.koma`，API0.2，主页 `https://mycomic.com`。中文内容、源级nsfw，单作品分级和用量未知；官网繁中与简中页面标题带“MYCOMIC - 我的漫畫／我的漫画”副文案，但按用户 2026-11-10 要求显示名只保留大写品牌 MYCOMIC，不再把页面副标题当作品牌。现红色 C 是此前保留身份的重绘图，不冒称官网原图，图标字节不变。popular、latest、search、筛选、排行、详情、目录、页描述与图像请求已实现；home/credentials不开放。不登录、不改Cookie、UA、网络或权限。

## 首选语言

`pref_key_lang`（設置首選語言）提供繁體中文与简体中文，默认 `cn` 保留Koma原简体路径；固定Keiyoushi `949dea252eb8e867b7b40834a26e13088764ff55` 默认是空前缀繁体，这里按保当前默认适配。ABI选项不允许空值，`original`表示参考空前缀，也兼容已有空值及zh-hant；cn/未配置保持 `/cn/comics`，显式original使用 `/comics`。

列表、搜索、分页与详情/目录的共同manga_doc均消费所选前缀。Koma作品ID一直为无语言前缀的slug，因此详情与目录从当前设置派生路径，保持原作品身份；不主动重写已入库标题。参考Dto章地址始终 `/chapters/<id>`，正文请求保此无前缀路线，不按语言设置擅加cn；图像URL和请求头保持原输出。原正整数字符串章ID、页ID `page:<chapterID>:<index>`、目录顺序及读态不变；App沿已采用设置摘要合同按需重新取页，不清图片缓存。

## 解析与可用性

目录完整解析实际owner中的strict JSON chapters数组，保title/原序/notes；仅从原章名明确的「第N话／第N話」提取话数，卷号、裸数字、无编号番外和未知日期不猜。分页next仅接受当前选定语言下实际下一页号。原狭窄ASCII slug规则、HTTPS图片、非2xx与容量/截断守卫保持；未知HTML或缺图明确失败，不把部分解析冒完整目录。

具名公开请求曾返回 HTTP 403，当前站点内容链可用性未确认；不对受拒请求反复重试。红 C 是保留原身份的重绘图，不是官网原图。

## 实现来源

独立Rust适配依据Apache-2.0 Keiyoushi `10f06eb833af5990bc5470c84c5e93edc159ad4e` 的 `src/zh/mycomic`；本次语言项核固定 `949dea252eb8e867b7b40834a26e13088764ff55` 的MyComic.kt/Dto.kt。许可见NOTICE/LICENSE，不复制Kotlin实现或图像。

## 参考筛选与宿主调用链

固定参考 949dea252eb8e867b7b40834a26e13088764ff55：独立热门按 -views、最新按 -update；查询默认最新上架（无 sort），提供最近更新、最高人气、日/週/月/历史排行，另含地区、类型、受众、年份、进度。四种排行走 /rank、忽略搜索词且不翻页；普通查询及分类走 /comics，按站点 next 续页。参考无 home 或账户功能。

Koma 仅在宿主声明 source_filter_search_without_query_v1 时提供 search 作用域筛选并声明 useSearchForFilters；筛选以 search 执行，允许空查询。热门和最新 get_manga_list 不消费筛选，保持原 sort；显式最新上架使用内部值 new 并省略 sort，普通查询未选择排序亦省略 sort。旧宿主保留原无筛选和非空搜索行为。排行仅消费排序和其他筛选，保留原站无封面数据，不伪造封面；排行续页请求在网络前拒绝。cn 与 original 前缀沿当前设置，不改变已有默认和 ID。筛选项按源参考原值发送，未知值拒绝。

本轮未发布候选仅通过合成 HTML 的实际 WASM 请求、解析及旧响应/403对照；原站先前返回403，真实筛选、排行 HTML 与 Native 普通路径均未验证，不能据此声称站点恢复。

## 2026-11-10 Cloudflare 403 与网页验证

197（ALN-AL80，误标为 237）实际回归：0.1.7 的 `get_manga_list` 为 `permission_denied`/HTTP403，页面只有“源浏览失败”，不会弹出网页验证。0.1.14 用过 `chrome145` 浏览器指纹，0.1.15 回到默认 NetworkKit 与 SDK Firefox UA，0.1.16 保留默认 NetworkKit 且不覆盖 UA，同时保留 `webSessionOrigins=["https://mycomic.com"]` 并接入 `source_http_challenge_cause_v1`：遇到宿主 `accessSignal=challenge_required` 或原始响应中的 Cloudflare hard-block 标记时返回 `challenge_required` 和 `challengeRequestId`。

197 结果：源页现在显示“站点要求网页验证，当前请求未通过（HTTP 403）”及“网页验证 / 登录”；点开后出现网页验证半模态，WebView 可完整呈现 MYCOMIC 列表页并显示“完成并重试”。但无论 `chrome145`、默认 NetworkKit + Firefox UA，还是默认 NetworkKit + 不覆盖 UA，完成并重试后的原 GET 都仍返回 403 `challenge_required`；日志均为 `session=attached cookieState=present` 而 `clearance_cookie state=missing`，WebView 会话没有带回 `cf_clearance`。因此“能不能过 CF”的差异不在 `httpProfiles` 或 UA：包子和漫画160能自动回流，是因为它们的站点实际给了可复用的 clearance；MYCOMIC 这个挑战只让 WebView 显示页面，没有给出该 cookie。本候选只修复“应用页不弹验证入口”的具名 UI 缺陷，不宣称 MYCOMIC 已可正常浏览，也不发布为可用修复。

设备身份校正：`29Q0223919000743` 的型号是 ALN-AL80，即 197；237 是 VDE-AL00（192.168.50.237:12345）。上述早期“237”截图和日志实际来自 197。后续已在真实 237 上复测 0.1.16：普通导入后同样弹出网页验证半模态，WebView 实显 MYCOMIC 列表，但 probe 仍为 403、`clearance_cookie=missing`；验证页内刷新后也未取得 clearance。0.1.16 未发布，197 和 237 均已恢复正式 0.1.7。

证据：Koma `.hvigor/outputs/user-flow-audit/device197__ALN-AL80/not-applicable/portrait-1260x2720/mycomic-cf-20261110/` 的 `mycomic-open-008`（0.1.7普通403）、`mycomic-challenge-037`（验证半模态）、`mycomic-open-048`（0.1.14 chrome145 仍为 challenge）、`mycomic-open-054`（0.1.15 默认传输仍为 challenge）、`mycomic-open-059`（0.1.16 默认传输 + 不覆盖 UA 仍为 challenge）与各 run 的 `hilog.txt`/`screen.png`；这些路径原被误标为 device237，已更正为 device197。

真实 237 证据：Koma `.hvigor/outputs/user-flow-audit/device237__VDE-AL00/baseline/portrait-1320x2120/mycomic-cf-237-20261110/` 的 `mycomic-import-077/registry.json`（0.1.16 导入）、`mycomic-open-082/hilog.txt`（probe 403/clearance missing）、`mycomic-refresh-083/hilog.txt`（刷新后 attempted=true 仍无 clearance）、`mycomic-restore-import-085/registry.json`（已恢复 0.1.7）。

## 0.1.17 宿主文档回放（仅列表页）

0.1.17 新增 `websitePolicy.renderedDocumentFallback: true`，配合宿主 `source_web_document_handoff_v1` 使用：验证半模态页加载完成后，宿主读取当前可见文档，并与原 GET 的源、操作 guard 和请求文本精确绑定；点“完成并重试”时宿主把该文档作为一次性的 200 `bodyText` 交给源解析，不再依赖 MYCOMIC 不会写入的 `cf_clearance`。不新增 WASM ABI，不后台自动开网页，也不改写源自身的 HTTP 行为。

真实 237（VDE-AL00）结果：普通导入 0.1.17 后打开浏览，验证页实显 MYCOMIC 列表；点“完成并重试”后 `document_handoff_reused=used_once`，`get_manga_list` 完成，页面显示“美丽新世界！”“艳母”等真实作品网格，HTTP 403 消失。仅验证列表页；详情、目录、图片与阅读未验证，0.1.17 未发布。

0.1.17 证据：Koma `.hvigor/outputs/user-flow-audit/device237__VDE-AL00/baseline/portrait-1320x2120/mycomic-cf-237-20261110/` 的 `mycomic-handoff-diag-119/hilog.txt`（`document_handoff_reused`、`get_manga_list status=completed`）、`mycomic-handoff-diag-119/screen.png` 与 `mycomic-handoff-diag-119/layout.json`（列表网格）。

## 0.1.18 就绪后自动关闭验证页

0.1.18 把 `renderedDocumentFallback` 从布尔升级为对象：`{"readySelector":"div.grid > div.group"}`。选择器取自本源列表解析的卡片节点。宿主在验证页每次主文档加载完成后检查该选择器：CF 挑战还在时不满足，验证页保持；站点真实列表出现后满足，宿主自动暂存文档、关闭验证半模态并重试原 GET，不再需要手动点“完成并重试”。布尔 `true` 的旧声明仍保留手动完成行为。

该机制由源声明选择器，宿主不识别站点或 Cloudflare 特征；未声明 `readySelector` 的源不会自动关闭。

## 0.1.27 WebView fetch bridge

0.1.27 新增 `webFetchBridge: true`。验证完成后宿主保留一个隐藏的 WebView 会话，详情、章节等 GET 请求在同源页面内用 `fetch` 获取，HTML 回传给源解析。237 实测：列表和封面正常，详情 `web_fetch_bridge status=200 bytes=335475`，`get_pages status=200 bytes=114876`，5 张阅读图片全部 200，阅读页显示 1/60。阅读界面不变；`webSessionAndroidAgent` 仍启用。

## 0.1.29 自动条漫与历史入口

0.1.29 的最低 App 版本为 1.4.2。共享阅读器会在同一次图片下载或缓存读取中取得原图尺寸，首次打开长条图时自动切到连续滚动，并把本书的阅读方式写入偏好；旧阅读页保留同一判断。验证会话的隐藏 WebView 改由应用根持有，从历史或书架直接进入阅读也能继续使用已经通过验证的会话，不再退回原生请求。237 实测：列表封面、详情、目录与阅读正常；原图 720x4200，首次打开判定为条漫；历史直读正常出图。

## 0.1.30 详情作品链接

`get_manga` 增加 `links` 的公开作品页地址，使用当前设置的首选语言路径。宿主因此能显示详情页的网页打开和分享入口；作品 ID、目录、页表、图片请求和最低 App 1.4.2 不变。

## 章节续读

按站点章节数组分组续读，使用已保全目录的最新在前顺序。保留章节 ID 和展示原序。离线回放使用实机缓存的十章数据及人工构造的多分组边界，官网直连仍返回 403。
