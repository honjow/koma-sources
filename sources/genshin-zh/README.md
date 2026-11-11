# 原神官方漫画 · 0.1.1

面向 https://ys.mihoyo.com/main/manga 发布的中文漫画的独立 Koma 源。此源不在当前 Keiyoushi 中文目录中；不复用其他提供方身份。稳定包 ID：`com.mihoyo.genshin.zh.koma`；书 ID：`genshin:zh:official-manga`；章节 ID 用官方 `iInfoId`；页 ID 用章节身份加原零基数组位置。

站点发布的 `3738d5b.js` 声明 content_v2_user app 43、key 16471662a82d418a、channel 725。其 `56dc5b3.js` 漫画组件声明 getChapterTotal pageSize 1000，并按数组顺序消费 `sExt` key `725_0` 作页、`725_1[0].url` 作封面。只用普通公开 GET。不实现 cookie、登录、挑战处理、任意 JavaScript、统计 POST、付费章节解锁或图像转换。宿主 HTTP `status` 和源 `retcode` 必须分别为 200 和 0。JSON 本地解析；没有 HtmlHost 属性查找或依赖其缺失属性语义。源暴露 native alloc/free 并对请求做边界检查。

当前完整列表有 15 章，包括官方标题/ID 对 118835「第一话 添酒」；它不列第六章或序章。此源保留发布列表，包括其缺口和标题，不添加预测章节。`iTotal` 必须匹配完整接收列表；截断响应产生错误。浏览/搜索返回这一部漫画；搜索匹配其本地中/英标题，不是虚构的远程搜索 API。章节顺序遵循官方导航对 API 列表的反转。显式中文编号前缀提供 chapterNumber；未知或额外章节标题得 null。状态、作者、单内容评级和真实源用量未知。

冻结在线证据：完整 15 项列表、首章详情带 53 个原页 URL、一张真实 800×1131 JPEG 首页。其他 52 个图像体和其他章节图像体未下载或解码。保留响应的生产 WASM 回放匹配全部 15 个章节 ID/标题/页计数和全部 53 个页 ID/索引/URL；负 ID/宿主/游标和 ABI 边界单独检查。这是宿主证据，**不是真机验收**。Root 必须审查最终源和包，然后在选定设备上执行普通安装/浏览/搜索/详情/阅读/下载/冷路径。本交付不含设备操作、App/SDK/运行器树编辑、推送或发布。

## 官方图标来源

抓取的官方目录 HTML 声明 `rel=icon href=/favicon.ico`。该端点返回 302 带官方 Location `https://ys.mihoyo.com/main/favicon.ico`。ICO 文件含十个原条目。`icon.png` 是条目索引 3 的**未修改 PNG 字节切片**，256×256，偏移 7518 长度 87373。不施加光栅解码/重编码、裁剪、背景、遮罩、阴影或添加美术。其原派蒙 /「6th」/ miHoYo 设计保留。

原 ICO SHA256：
`2410be3117373b4f5e82f8f38f4711ee997210bd602deadf62af4d9db515f72f`

打包 PNG SHA256：
`42d4debfa65eb66f8625dc3ffa89813f71ef3638c5d271b510cb51016c2e638e`

头部字标被检查并作为研究材料保留，然后在此候选中用实际官方 favicon 替换后冻结。在应用既有图标槽中的可见性/尺寸保持未验证。

元数据 R2 保留 R1 冻结并只更正章节顺序和标题派生 chapterNumber。单本书当前无 `url` 字段；其既有完整源链接保留在 `links`。未添加其他字段。Native 验收保持未验证。

## 0.1.1 继续元数据

官方 `getChapterTotal` 以反转 API 顺序返回那部中文漫画的完整章节列表；`getDetail` 把相同列表赋给 `chapterList`。对这些已验证同作品章节，每章现在发出 `readingGroup: "genshin:zh:official-manga"` 和 `continuationEligible: true`。这启用既有应用继续门禁而不改变它。所有原章节 ID、标题派生编号（包括缺失的第六章）、标题、页数组、URL、图标和能力保持不变。原安装的 0.1.0 metadataR2 冻结保留。这个候选只有保留响应 WASM 证据；native 边界导航未验证。

## 0.1.2 页数组终端处理

当前标准入口：`bash build.sh --source genshin-zh`。只有 page_count 和 get_pages 通过本地 Result 迭代器聚合源声明的 725_0 图像数组。无效消费元素返回 parse_error 而不是假装前一张图像是完整章节。不添加新大小或 URL 规则。目录 iTotal 检查和封面首匹配/未读尾行为保持不变。这是已审 0.1.1 元数据的后继；device237 registry 仍报告 0.1.0，故未来更新也添加那个已审 readingGroup/continuationEligible 元数据。这里不含部署。

保留的 256px 官方图标字节保持不变；manifest 添加当前必需本地图标字段。美术适用性保持单独。


## 237 现装 0.1.2 第十六话首两页（2026-10-21）

本轮 registry 实读现装 0.1.2、enabled=true，取代上方旧「237 仍 0.1.0」的当时状态；没有更新包或修改元数据。普通 15/15 章目录选择无现存章节记录的《第十六话 添酒》（genshinch:zh:118850），未重开已有第一话样本。Root 亲看 1/21 与 2/21 两完整彩色剧情原图，安柏出窗、跃空与蒲公英可辨；两页主要是动作画面及中文拟声，不扩大为大段对白、全 21 页、下载、cold 或无缓存在线首取验收。

普通 Back 后真实进度为 genshinpage:zh:118850:1、pageIndex1、totalPages21、completed=false。源执行者在返回后因 502/429 中断，Root 沿同一 manifest 及 770dcddc 租约接管恢复；最终 Browse 末端与 10000ms/AWAKE 亲看读回，租约已释放。52 项 registry 与 reader 偏好 XML 相对本轮 baseline 内容相同，原 17 sourceHistory/60 chapter 对象保持，仅新增本章记录；原 47 progress 中仅本书自然从原第一话变为本章 P2，其他书对象保持。没有编辑持久记录、清数据/缓存、改变模式或修源。

原件在 Koma/.hvigor/outputs/user-flow-audit/device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership/ 的 genshin012-{baseline,public-list,detail,chapter-seek,ch16-first,ch16-second,ch16-natural-return,session-restored}237-001；哈希关闭。只接受上述具名正文加载、翻页与自然返回边界。
