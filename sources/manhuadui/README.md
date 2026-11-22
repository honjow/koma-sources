# 优酷漫画 / Manhuadui 0.1.4

稳定源身份com.manhuadui.koma；目录与标准构建名manhuadui，显示名优酷漫画，不创建youku别名。完整业务继承b994ad0380df8f7da7e185c98dc093a233502883（前序ce96f8e）；当前SDK保持canonical原字节。源码/Cargo版本均0.1.4；三语言metadata继承原012。

仅声明search/listings/mangaList；detail/chapters/pages/imageRequest不可用，不用未接受正文实现声称可读。保原热门栏目、operation、对象cursor/legacy page、真实next控件终态与有界固定诊断，401/403明确permission_denied。不更换原UA/站点，不绕过挑战或重试已拒路由。NSFW=true继承原包与sourceInfo，不声称所有单作品评级已知。

标准入口 bash build.sh --source manhuadui，minAppVersion0.1.0与原012 index相同。原0.1.2补齐必需官网原PNG；本次icon.png为下述独立标识高清还原，出处见ASSET_SOURCES.md，App圆角及槽位保持。标准构建使用当前SDK/正式CLI/既有flags，新WASM与旧Native647a0332不同，不能冒旧二进制复现或当前Native通过。

历史197具名012只接受一拳超人搜索两作品/真实封面/终态与不清数据cold身份；默认列表当次403保持，详情正文不接受。本次current SDK离线重建适配仅info/listings0HTTP和两原HTML输出，非现站可用证明，page2原体缺失不补造。原回执：Koma .hermes-artifacts/20261005-production/src010-manhuadui-20261010/diagnostic-012-native-acceptance-receipt.json。上述为原0.1.2业务边界；当前0.1.3图标的设备结果见下段。

当前沿用独立蓝圆标识的512px高清还原资源，业务与能力边界见上文。它不是官网高清原件；完整身份依据、重绘差异及待验边界见ASSET_SOURCES.md。上文原PNG/旧包/历史原图描述保留为历史，不作为当前资源未处理或内容已通过的结论。

0.1.3在237 VDE-AL00展开1320×2120经普通系统选择器更新成功，Root亲看管理及Browse两处独立蓝圆标识清楚，无可见彩边或长影；45源ID顺序、原启用和索引保持。原图在Koma user-flow-audit/device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership/ykmh013-manager237-001及ykmh013-browse237-002。只接受这两处图标，未访问该源列表或正文，原403与内容能力边界保持。

搜索先解码请求中的 JSON 转义，再按既有站点请求规则编码用户输入；默认、空查询与分页规则保持原行为。

## 0.1.6 候选：WebView 请求桥接

按用户反馈普通 WebView 可访问、普通网络请求被拒的现象，声明宿主已有的同源 GET WebView fetch 桥接；最低 Koma 1.4.2。当前只开放列表和搜索，详情、章节与阅读能力未开放。候选尚未设备验证；是否能在用户网络环境下实际读取列表和搜索、封面能否显示，以及需要的会话行为均未确认，不能视为修复完成或发布。

2026-11-12：237 读回 0.1.6 已安装。热门列表及封面可见，但未排除缓存；搜索真实关键词失败，日志显示桥接超时，原生请求返回 520/403。同设备打开并刷新 WebView，均显示 Cloudflare 520。用户此前报告 WebView 正常，该差异按本次具体时段保留；候选未通过、未发布。

## 0.1.7 候选

原生热门请求取得带 Cloudflare 挑战标记的 403。当前源依宿主的
`accessSignal=challenge_required` 显示验证入口，并声明精确的官网验证 origin；
普通 401／403 仍报站点拒绝。已授权的同源 GET 继续走现有 WebView 桥接。
本地响应证据不代表验证后热门或搜索已经可用。

237 普通浏览已弹出验证页，验证后原请求网页出现实际漫画列表。候选复用现有页面回流，
以解析器所需的漫画卡片判断浏览网页已可用，再交给当前待处理请求。
237 日志证实验证后的网页跳到另一个 origin，原精确请求不能接收该文档。
热门和搜索从头请求官网移动站，沿用核验过的路径和查询参数。作品身份与对外地址继续使用原规则。
237 实际热门卡片用 `li.list-comic a.txtA` 与 `a.ImgA`，搜索用 `div.itemBox .itemTxt > a.title`。
原选择器无法读出作品，本次补上两种结构，保留旧页面解析和作品身份。

2026-11-19 0.1.7 237 已具名验证热门首屏、《一拳超人》搜索和验证页自动关闭，作品出现在应用原生搜索页。


## 0.1.8 候选：恢复完整目录及公开免费页表

本轮只改该源，保留 com.manhuadui.koma、ykmh:<slug>、ykmhch:<path> 和 page:<index>。
0.1.7 的详情/目录/页表导出一直存在，但 capabilities 关闭；这属于源侧能力缺口，
不能归类为 Cloudflare 或登录阻碍。0.1.8 恢复这三项操作及 imageRequest。
热门、搜索、分页、分级和现有素材保持。

上游固定为 keiyoushi/extensions-source 5d54f0d02e2a7dbdda08e823de444f7527cc883c：
YKMH.kt（blob 6264bf25f9b93dbe01e0ddce8be7b0fae2fd7f24）继承 SinMH.kt
（blob acfff55809180c72296fd7ece8e06efda5832b3b）。上游详情/目录/章页走移动域，
各 .chapter-body 独立，YKMH 保留每组网站数组顺序；图片来自 chapterImages/chapterPath，
配置中的 resHost.domain 只参与相对地址。

| 行为 | Koma 已有调用与本源消费 | 本轮依据及边界 |
| --- | --- | --- |
| 热门、搜索、分页 | get_manga_list/search → 原 parse_list | 沿用 0.1.7 已接受移动站入口和真实 next；本轮未重新验设备分页 |
| 详情 | get_manga → 原桌面选择器，兼容上游 #comicName/#Cover/简介 | 五部真实桌面 HTTP 原件已回放；缺作品标题报错，不能用 slug 冒充成功 |
| 完整目录 | get_chapters → chapter_catalog / SDK GroupedChapters | 六个真实分区、130 条完整；各组独立位置，无号与番外保留；不按标题、编号去重或重排 |
| 原始与渲染文档 | 既有 webFetchBridge/renderedDocumentFallback | 桌面原始 HTML asc，渲染后活动按钮 desc；移动 chapter-body 实测为新到旧，UL 仍标 asc，不以 UL ID 识别桌面。按各模板真实合同还原正向数组，宿主显示排序独立；详见末节反例 |
| 替代组 | 原目录 chapter-list-1、chapter-list-7 | 三日月与流星同名不同 ID 的两组完整保留；第41话的真实 next 指向第01话，遵守站点实际导航，不能数值排序 |
| 正文页表 | get_pages → 原 chapterImages/chapterPath 合同 | 先核父作品版权下架，再核 chapterCanRead=1 且 chapterPrice=0；绝对 http/https 解码后原样返回，不依赖配置；相对图才取真实 config.js domain |
| 权限和容量 | 版权提示/权限/数组完整性 | 下架拒绝阅读残留旧 ID；付费、不可读、未知权限拒绝；容量或解析错误整项失败，不截断后声称完整 |
| 原点 | 包内 web-session-origins.json | 保留 m，新增精确 www；主页不扩大会话权限。详情/目录/阅读按上游和官网 phone.js 从 m 请求，公开作品链接保留 www；桌面原件仅做解析兼容回放，未冒充移动域真实响应 |
| 登录、筛选、设置 | 原 capabilities | 本源没有账户登录或筛选设置能力，不新增登录、凭据或付费能力 |

本轮使用原仓现成 dev runner 与其已有证据回放器。五部公开 HTTP 200 目录原片段是
Cat Loss Syndrome、三日月与流星、死亡游戏bad end、只属于你的单马尾、开始吧！秘密恋爱。
旧发布版第一组 ID、标题、原 null 数值全部保留；新增恢复原先遗漏的第二组 37 条。
Cat Loss Syndrome 序章真实可读、免费，8 张绝对 http 图片地址逐项保持，未请求 config。
相对图片分支用真实 config.js 配合派生输入验证；不可读、付费、重复 ID 和版权拒绝
也为明确标注的派生边界验证，不冒充真实受限页面。

真实网络与回放分开：Edge 桌面公开页正常，原生 dev runner 同一目录仍 HTTP403；
最初移动 UA 的 m 域浏览器停在自动安全验证。官网 phone.js 确认 Android 会从 www 跳 m。
后续237普通官网回流后，原候选取得 Cat 详情及7/7目录，手动序章首图与末图已观察；
普通开始阅读却进入完结章，不能把已接通内容算作章序通过。新增精确 origin 也不等于
所有请求均已接通，不将候选称为普通路径整体恢复通过或可直接公开发布。
本次已取得237源实际消费的移动 Cat 目录，详见末节；其余作品移动模板及真实付费/权限页
仍未验。桌面用图片描述符真实请求并解码序章第0／7图，均为 JPEG（1280×12642／1280×11393）；
新章序修正包的237开始阅读与上下章仍未验。任务浏览器工具两次没有遵守有界 timeout；
最终恢复临时 UA 的结果未确认，详见证据，不归因于人类用户。

证据：Koma .hvigor/outputs/published-source-order-audit-20261123/youku-investigation/
下的 provider-capture-manifest.json、candidate-validation.json、candidate-native-chapters.*、
progress.json、browser-errors.json。初次候选当时未提交、未推送、未发布；此前五文件后由主代理保护提交1d6155b5。本次移动章序修正仍未提交或发布。


### 0.1.8 候选图片请求头补审

固定上游同一提交的 `core/src/main/kotlin/keiyoushi/source/KeiSource.kt`
（blob 5372ca0363544fee6f127d66c39853e1ab58a9d0）199–201 行在最终
headersBuilder 设置 `Referer: $baseUrl/`，517–525 行图片 GET 使用这份 headers，
并标记 Fetch.IMAGE。SinMH.kt 43 行仅覆盖系统 HTTP UA，YKMH.kt 9–11 行未更改图片请求。
首个候选 imageRequest=false 遗漏了父类消费链，父代理审查指出后在同一版本补齐。
本源 get_image_request 现在返回原样绝对 URL、`Referer: https://www.ykmh.net/`
和既有 SDK 普通请求 UA；该 UA 是 WASM 宿主适配，并非逐字模拟 Android JVM 的
System.getProperty(http.agent)，也不改变验证页 UA。沿用宿主现有图片描述符调用，
不新增 App/SDK 功能，不携带 Cookie、账号或凭据。


### 0.1.8 移动目录真实反例与修正

237 普通详情/完整目录取得 Cat Loss Syndrome 的 7 条，旧候选普通开始阅读却进入
第6话（完结），页1/14、章1/7；手动序章8图已看到首图/末图，序章下一章按钮禁用。
这是真实章序反例，旧候选不算阅读顺序验收通过。

临时源 host::log 包的 checked hilog 记录了实际消费的移动 `.chapter-body clearfix`：
内部仍复用 `ul#chapter-list-1[data-sort=asc]`，没有 `.chapter-sort.active`，实际7行
从完结至序章。按 UL ID 判断为桌面模板并信任该 asc，导致旧包把完结写成位置0。
完整 hex0..17 还原了2750字节公开目录（至真实 UL 闭合，外层闭合未采集），中文标题
与全部业务路径完整；旧包内 WASM 对同原件已重现相同错误，无联网。

修正按容器区分模板：移动 `.chapter-body` 沿当前实测网站数组合同，将新到旧数组转为
SDK 正向位置；不读取该模板已证陈旧的 UL data-sort。桌面 `.zj_list` 继续以活动排序
控件为当前状态，原始文档无控件时沿其 data-sort。两种模板如有明确活动控件则优先
使用当前状态。UL ID 只保留为原分组身份，不再充当模板识别。没有按标题、章节号或
URL 数字猜方向，全部无号章节和同名不同ID保留。临时诊断模块及调用已删除。

实证范围是237实际消费的移动默认7章及原有五部桌面原件；公开移动排序JS和用户切换
移动网页排序后的输入未取得，不冒充已验。新修正候选的237普通开始阅读与序章下一章
仍需设备owner独立复核，源侧回放不能替代普通旅程。所有旧候选、诊断包及失败原件保留。
证据在本轮 youku-investigation/mobile-order/actual-mobile/counterexample-witness.json，
原始 checked hilog 位于 native-host-validation 的38-diagnostic-full-bounded-public-log。
