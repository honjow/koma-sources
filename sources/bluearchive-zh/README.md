# 蔚蓝档案官方漫画（中文）

源 ID `com.yostar.bluearchive.zh.koma`，版本 `0.1.1`，API `0.2`，最低 Koma `0.1.1`。一个公开中文四格合集；用量和内容分级为 `unknown`，manifest `nsfw=false`。无认证、cookie、JavaScript 执行、付费内容或挑战处理。

## 实际公开合同

官方页面：https://www.bluearchive-cn.com/official 。其声明的 `index.0773252b.js` 发布 GET `/api/comic/list?pageIndex=<n>&pageNum=<n>&type=<type>` 和 `/api/comic/detail?chapters=<n>&type=<type>`。声明的 `Home.5b98ce15.js` 移动实现调用 `getComicList(1,999)` 和 `getComic(c.value.rows[0].chapters,1)`。源精确使用这个完整列表合同和 type=1 详情合同，不是虚构的浏览源流。

实际列表响应有 count=15、15 个唯一 ID、type=1；源保持网站顺序 15 到 1 和每个原标题。两部第六话的两个标题保持不变。章节编号使用观看者从 `comic.chapters` 显式的全局 NO 显示，而不是从标题或原始 ID 推导本地编号。精确发布代码：`"NO."+j(e.value.comic?e.value.comic.chapters>10?e.value.comic.chapters:"0"+e.value.comic.chapters:"")`；图像渲染使用 `src:e.value.comic?e.value.comic.content:""`。发布的原脚本和字节偏移/片段独立保留。

元数据 R2 额外为这个已验证的单合集序列声明 `readingGroup=bluearchive:zh:four-panel` 和 `continuationEligible=true`。实际 15 个详情响应形成从第 1 章（pre=-1，next=2）到第 15 章（pre=14，next=-1）的完整前/后链；官方观看器显示相同全局 NO 并跟随那些发布链接。Root 在 FGO 普通使用中识别了被省略的源继续元数据。这个候选保持 0.1.0，因为 R1 未安装，并保留所有 R1 ID、编号、标题、内容 URL、图标、能力和其他合同。不改变 App/Reader 门禁。

详情必须以 `comic.id == comic.chapters`、type=1、非空标题和已批准原 `content` URL 识别所请求的列出章节。每个当前章节有一张完整原漫画图像。源发出一個描述符带不变内容 URL。无拆分、裁剪、图像转换或缩略图替换。不完整计数、重复、未知章节和不受支持类型失败关闭；未来超过发布 999 抓取大小的目录若不完整则拒绝。

图像请求保持原 CDN URL 和 Referer=官方页面。允许 URL 是实际 favicon 或 `https://webcnstatic.yostar.net/ba_cn_web/prod/upload/comic/<文件名>.jpeg|jpg|png`，文件名限制为字母数字、下划线和连字符。外部宿主、额外子目录、穿越、查询参数和其他扩展被拒绝。提供的 mangaUrl 必须匹配合集的官方路由；提供的 get_pages mangaId 必须匹配合集。不透明 CDN 文件名在无网络的图像请求操作中不能独立证明章节归属；归属由 get_pages 对照实际详情响应建立。缺失 mangaUrl 仍支援标准封面请求。不引入凭据或额外头。

## 官方资产与保留验证

`icon.png` 是原官方页面声明的 favicon，https://webcnstatic.yostar.net/ba_cn_web/prod/web/favicon.png 。原 512x512 PNG，309338 字节，SHA256 `61810acce252780ddf7686f37159af4812b1c30d314038fc154c48f5565a423b`。无重绘、裁剪、重压缩、添加底板、遮罩或阴影。任何既有背景都是官方资产的一部分。

证据：`/private/tmp/koma-bluearchive-zh-readonly-20261014`。完整原始列表、全部 15 个实际详情 JSON 体带响应头/回执、原声明脚本和第 15 章原 JPEG 都保留。每个 URL 用有界普通匿名 GET 抓取，无重定向/重试/客户端切换。全部 15 个原描述符和全部 15 个图像请求计划对照最终生产 WASM 用保留的精确响应回放；`/private/tmp/koma-bluearchive-metadata-r2-20261014` 中的 R1 到 R2 比较只允许两个新增章节字段，所有其他输出相同；无效请求边界和 ABI 已检查。仅保留宿主不执行网络且不是 Harmony Native 宿主。

只抓取并目视检查了第 15 章原 JPEG（1224x1877，1590276 字节，SHA256 `23a62bd756277c29c3bb3d907cc44bec19bd7e7739c427004f44b636999ea232`）。其他 14 个二进制未抓取。描述符成功不是图像体验证。不声称 Native 安装、Reader 路由、下载或冷启动验收；Root 拥有那些检查。无 App、ReaderKit、SDK、主运行器、其他源或主树 WIP 变化；无提交/推送/发布。

## 2026-10-21 Native 有限观察（237）

安装 0.1.0，普通 Browse → 唯一合集（15/15 章）→ 官方第 15 章「第二部第六话：蕴含强大力量的装备！」。一张完整原四格图像（包括版权页脚）以 1/1 可见。自然 Back 返回合集；其行变为已读且按钮只显示「继续阅读」，因此不声称那里可见章节/页文本。实际 `reader-sessions.v1.json` 绑定 `bluearchive:zh:four-panel` / `bach:zh:four-panel:15` / `bapage:zh:four-panel:15:0`，pageIndex 0，totalPages 1，completed true。无拆分、下一章、下载、冷启动或其他 14 个图像验收。

原件：Koma `.hvigor/outputs/user-flow-audit/device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership/bluearchive010-{chapter15-complete,natural-return,session-restored}237-001/`。原 Browse 末端和 AWAKE/10000ms 恢复；全部 50 个 sourceId/启用项保持顺序。哈希关闭。执行者和 Root 都检查了完整原图像和自然返回；验收限于这条路由。上面先前的无 Native 说明描述更早的仅宿主运行。

## 0.1.1 源编年史

包为不变官方列表输出声明 `chapterOrder: newest_first`。这完成用户采用的 Mihon 最早未读合同；它不排序、反转、重新编号或丢弃章节。既有进度和显式章节选择保持优先。

2026-10-26 普通公开列表返回相同 15 行，顺序 15 到 1。全部 15 个官方详情响应各读一次，无重试、重定向或图像请求。它们的 `preComicChapters`/`nextComicChapters` 链接形成从 1（pre=-1）到 15（next=-1）的完整互逆链，恰好是列表顺序的反向。方向跟随这些链接，而不是从 ID 或标题推断编年史。原始响应保留在 Koma `.hvigor/outputs/user-flow-audit/bluearchive-current-{catalogue,chain}-20261026.json`。

当前 0.1.0 WASM 对照那个真实保留列表回放并以相同顺序发出原 15 个章节对象。0.1.1 中只有包声明和源版本变化；API 请求、解析器、ID、名称、分组/继续合同、页、资产、图标和 App 呈现保持不变。构建、打包、生产选择器回放和普通 App 交付在 Koma CURRENT_STATUS 中保持独立证明边界。

标准单源构建产生 325253 字节的三成员 0.1.1 包。执行其实际 WASM 对照保留官方列表产生与 0.1.0 相同的完整章节响应和 HTTP 请求；图标字节不变。当前 App 生产 `firstReaderChapter` 选择官方第 1 章，若 1 已读则第 2 章，全部已读时返回无目标，不修改列表。这是真实 WASM/生产选择器回放，不是 Native 首读或正文验收。结果：Koma `.hvigor/outputs/user-flow-audit/bluearchive011-chronology-replay-20261026.json`。
