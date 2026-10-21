# 老夫子繁中・授權關聯站 0.1.0

提供 Oldmasterq.com 网页当次实际最新25条独立漫画，每条一个作品、一个章节、一张完整原图。不利用TOTAL_STRIPS=2518/Random整数公式造全目录，不抓随机/旧档案、不裁六格为六页，不把漫画图打包分发。

站点原About声明其为OMQ ZMedia批准、独立运营并拥有展示许可的关联站，由作者孙女Ginger Wong维护；古典条漫Alfonso Wong创作、少量新作Ronny Cuxun。源名称标获授权关联站，不冒充总部。逐条详情没有独立作者字段，authors/artists为空（未知），不把全部作品归Alfonso；status/contentRating/usage unknown。实现作者Koma与漫画作者分开。

Source ID `com.oldmasterq.zh-hant.koma`，0.1.0/API0.2，最低App0.1.1。当前原25目录2518..2494。Work ID=`omq:zh-hant:strip:<真实ID>`；CID=`omqread:zh-hant:strip:<真实ID>`；PageID=`omqpage:zh-hant:strip:<真实ID>:0`。ID只从实际href取得，Chinese title只从title-ch取得，原DOM顺序保留。每chapterNumber/volumeNumber null、readingGroup本作品MID、continuationEligible=false、pageCount1。Strip编号不是章号，前后条回环2518→1不是同书自动章。仅当前最新25成员范围；未来条目离开这个目录时无法在本最小范围继续读取，不宣称全档案/历史持久可用。

## 实际HTML合同

目录 https://www.oldmasterq.com/comics/ 的 `.comics-feature-item`为1，`.comics-thumbnail-list-item`为24，源读这25项，不截取后补ID或标题。最新首图src，其他24图data-src；先presence-qualified `[data-src]`选择实际节点，缺失才以`[src]:not([data-src])`取得原src。skeleton路径不属于允许的strips目录而失败，不把占位图当封面。实际作者留未知。纯本地最新25标题过滤搜索，无远端搜索/分页假承诺，后续cursor返回空。

每条独立 `/comics/<id>/`真实详情请求，原og:url canonical为 `https://oldmasterq.com/comics/<id>/`，只作声明字段核对，不换域发请求；原og:title必须以`Strip #<id> <目录中文标题> `开头。正文只选`article.comics-strip-container > figure.comic-image-container > img[data-src]`，要求一个article/一个正文img。前后条thumb/箭头/随机/广告/Shopify图均在该范围外。data-src原完整URI属于唯一官网声明R2媒体host和`/strips/`直接文件，接受原jpg/png/gif干净文件名，不接受query/子目录/traversal/其他host/协议/广告。实际loader dataset.src→image.src且USE_OPT=false，源不跑JS、不拼文件名/改协议/CDN，不造Token/签名。

get_manga/get_chapters/get_pages及正文计划：1次当前目录GET验证当前成员，再1次同ID详情GET核正文（31354+28138=59492B为当前2518保留metadata）。25封面计划仅1目录GET逐原URI/MID验证。请求有mangaUrl时严格等同这个work的原www详情完整URL；有mangaId时必须对应该work。封面可无pageId或正确MID，空串/错MID拒绝。正文必须正确独立PageID且ordinal0/URL等同当前详情原URI。同一条封面与正文原URI可能相同，本例正是相同；正确正文PageID仍合法，不伪造一个不存在的独立封面路径。未知/未在最新25中的ID、错CID/MID/ordinal、错误URL与结构均fail closed；未成功解析不从硬编码表回退。

原PNG来自head明确apple-touch-icon https://pub-ea4ffe6130c34b07a02c682abfde808a.r2.dev/images/favicons/apple-touch-icon.png ，3677B/RGB180×180，SHA9ad0fe72aa164850572cbeece5093a9ef0849c575f8ab20192e863b13439c306。源icon.png与原bytes严格相同，实际validator通过；黑底/白色衣服角色形状为素材固有，不是我们加底。未裁/转码/缩放/重画/生成/遮罩/抠黑，不借其他品牌。Native可辨性未验。

## 运行证据与界限

原合同32文件绑定 `/private/tmp/koma-oldmasterq-zh-hant-public-contract-20261014/readonly-binding.json` 保持。#2518人為財死原JPEG294401B/RGB800×1313完整6格已查看，SHAf1d17b44e104bbc83f532e3020caf9dd430135b2dcb7829c41a0beebdc8e2f7b。实现阶段没有新网络请求；其它24详情/原图实体未取得、未解码或视觉验收，25封面计划并不证明这些实体/正文成功。

62生产WASM操作：35真实保留HTTP正向操作（25目录项目+25封面计划+#2518详情/一章/一页/正文计划）、19合成错误请求、8明确单列合成异常HTML反例（missing data-src/错owner/跨host/子目录/重复ID/缺目录原图/骨架冒正文/多正文）。成功响应全为原bytes，不造真实响应。现App原html_host.cpp/.h、1057原Lexbor文件与静态lib绑定，macOS Wasmtime22+真实C++HtmlHost/Lexbor对本原HTML实际解析，全部operation关闭descriptor0、无缺失Attr调用。完整ABI gate通过。不是197/WAMR接受，也不是真实下载或网络性能证明。

保护全部旧冻结源码包、主源WIP、SDK/App/Kit与共享runner；不设备/IDE/签名/提交/推送/发版。不重试BNS失败JS、Overwatch PDF、探測機522、任何旧挑战，不复测創漫冻结包。Root审全部Rust/README/diff/原HTML与Host/ABI/PNG及冻结后自行串行普通197链；其他24正文和全档案/Native阅读下载cold与缓存成本unknown。


## 237 0.1.0 普通交付与详情限制（2026-10-21）

沿现有公开关联站分支接续，不重做原目录/正文合同。普通网页读取当前 /comics/ 仍显示 #2518《人為財死》及2517..2494；这只提供当前目录线索。原主仓 offline/locked 标准参数构建后，真实 WASM SourceInfo 为0.1.0，使用原180px官方 apple-touch 图生成三成员正式包，在237经系统 picker 普通导入成功。管理和 Browse 两消费者原图实际显示标识及0.1.0；未改素材、业务、App容器或顺序。

Native普通列表实际显示“最新25條獨立漫畫 / 25本”及原具名首项。进入 #2518 后实际显示“源详情加载失败”，未进入阅读；标签范围 hilog 原记录14:49:31.484 HTTP403、14:49:31.570 manga-detail source detail load failed reason=source_detail。此前列表加载过程中14:48:58.301/.623也有HTTP403，故不声明全部25封面成功。最初事后流式hilog为空，不能作无错误证明；后续只读hilog -x标签转储取得以上原码，没有重新请求内容。具体服务端拒绝原因未知，未修改解析、重试该路径、变更客户端/身份或绕过挑战；正文、下载与cold仍未验。

原件在 Koma/.hvigor/outputs/user-flow-audit/device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership/：oldmasterq010-import237-001、oldmasterq010-manager237-001、oldmasterq010-browse237-001、oldmasterq010-list237-001、oldmasterq010-detail2518-237-001、oldmasterq010-detail-failure237-001；HTTP原码在 oldmasterq010-detail-logs237-001/run-metadata.json。oldmasterq010-session-restored237-001 记录普通返回原Browse、reader与Root原004逐字节相同、原51项sourceId/enabled顺序逐项保持，正常新增本源后52项；仅自传包删除并断言不存在，AWAKE/OverrideTimeout=10000ms读回后释放aa51056e。未清数据/缓存，全部设备回执关闭哈希。
