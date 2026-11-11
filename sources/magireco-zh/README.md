# 魔法纪录官方漫画（中文第一季）

源 ID `com.bilibili.magireco.zh.koma`，版本 `0.1.1`，API `0.2`，minAppVersion `0.1.1`。公开官方合集 `magireco:zh:comic1`，只有 https://game.bilibili.com/magireco/comic1/ 。官方第二季漫画页显式链接这个第一季。目前没有源分发/实现这个实体；全新 Keiyoushi index SHA `e26c3c6bf077e4bbdaa389fdb7f14020ec8297a45aec412b6934ac10e537dce3` 与保留的 103 中文实体处置集不变。这是单独验证的官方公开源，不是声称的 Keiyoushi 条目。实际用量和单作品内容分级未知；manifest nsfw=false。第 1 章实际图像是普通中文四格故事内容。

## 真实合同与元数据

第一季原 HTML 含字面 window.__NUXT__ JSON，带站点 ID `magireco`、游戏 ID 101644 和 h5-url `https://game.bilibili.com/magireco/h5/comic1`。h5-url 的斜杠转义在严格路由比较前解码。数据解析不求值 JavaScript 也不做 HtmlHost 操作。

其唯一 BiliBanner `autoid-cmv5LAK9` 声明 46 行有序单例 BiliImage。有 46 个真实 BiliExtra 导航控件，每个有唯一 `option.slideIndex` 从 1 到 46，`output.target` 匹配那个 banner，`emitter=click`、`listener=slideTo`、`data=["option","slideIndex"]`。发布的事件总线 formatEvent 对该组件解析那个属性路径。声明的 BiliBanner 组件实现 `slideTo:function(i){this.mySwiper&&this.mySwiper.slideToLoop(i-1)}`。因此显式导航编号选择对应原图像行。源拒绝重复/零/缺失选择器、另一目标、改动数据路径、非单例行、另一 banner、不完整导航或所有者路由。编号来自发布的 slideIndex，不是虚构副标题或仅数组章节猜测。

每章发出显式编号、稳定 ID `magirecoch:zh:comic1:<number>`、readingGroup=`magireco:zh:comic1`、continuationEligible=true、语言 zh 和 pageCount=1。章节标题未知并发出空字符串；第1话点击之类分析字符串不当作书副标题。App 可以从真实编号显示其既有回退。不虚构标题、缺失集数或其他季。已验证的同组继续合同不改变 App/Reader 门禁。

一次页操作做一次普通 GET 实际合集模板，校验所有者/导航成员资格，选择精确所选图像组件，然后从同一发布 Nuxt metaInfo 样式表解析其原 background-image 声明。它发出稳定页 ID、索引 0 和嵌套 image.url。封面使用页的实际 og:image URI。无合成逐集响应、预测 CDN 文件名、缩略图替换、裁剪或格式转换。

图像计划保留 CDN URI 和普通 Referer=第一季合集。允许合同恰好是 HTTPS i0.hdslb.com/bfs/game-static/<40 位十六进制>.png|jpg，无查询、子目录或外部宿主。提供的 mangaUrl 必须是同一官方合集路由，提供的 get_pages mangaId 必须匹配源的合集。无网络的 get_image_request 校验那个 URI 语法，不独立证明任意调用方哈希的加密章节归属；get_pages 从真实模板建立提供方映射。无登录、身份、挑战处理、付费、DRM、凭据或额外 HTTP 请求。

## 历史 R2 官方资产与有限证据

R2 包图标是从官方漫画页声明的 300×300 JPEG 分享/游戏身份图像 `https://i0.hdslb.com/bfs/game-static/c62be58d4a38e7fa801662c41ff5a5069798de58.jpg` 制作的格式兼容 PNG。实际头部 og:image/shareImage 标签和 Nuxt metaInfo.meta 都显式声明这同一图像。原 JPEG 是 56574 字节，SHA256 `95ba38c908b1c45f5b85b945deefdcfc1e18ae5d0dd6fafa986f0a1ab68ae2a6`；它连同响应头和声明链保留在 `/private/tmp/koma-magireco-official-icon-review-20261014` 和 `/private/tmp/koma-magireco-icon-r2-20261014`。原 R1 909×241 漫画标题 PNG 和包独立保持冻结。

当前包校验器要求 icon.png 的实际 PNG 字节。因此授权的 R2 兼容切片把原 JPEG 解码为 RGB 并以相同 300×300 尺寸编码无损 RGB PNG，不裁剪、旋转、缩放、重绘、去白角、加 alpha、底板、遮罩或阴影。全部 90000 个解码 RGB 像素精确匹配；像素字节 SHA256 和确定性重复 PNG 编码记录在 pixel-equality-receipt.json。原图像的圆角美术、白角和右下角出版商标记作为原 RGB 内容保留。PNG 是格式转换产物，不声称原 JPEG 字节同一性。源逻辑、版本 0.1.0、生产 WASM、46 章节元数据/页映射、描述、能力和所有请求与冻结 R1 保持相同。宿主像素比较不验收 Native 图标外观。

证据根 `/private/tmp/koma-magireco-zh-readonly-20261014` 保留完整原页体/头/SHA 回执、实际 46 个导航和图像映射、声明组件/事件总线原件和精确片段。113 次生产 WASM 操作比较全部 46 个原页描述符/全部 46 个图像计划和真实封面计划，验证元数据和 13 个畸形请求拒绝加 ABI 边界。仅保留运行器独立、有精确 URL 映射、不执行网络且不是 Native 宿主。失败的初始转义路由比较输出单独保留。

只抓取并目视检查了第 1 章原 PNG；其他 45 个图像二进制和封面二进制未抓取。原 R1 标题 PNG 和原声明 R2 JPEG 各抓取一次并目视检查；R2 PNG 从那个原 JPEG 确定性编码。Native 安装/浏览/正文/继续/下载/冷读不由这个宿主证据验收；Root 拥有实际 197 验收。无 App/ReaderKit/SDK/主运行器/主源 WIP 或其他冻结候选变化；无提交/推送/发布。

第二季保持排除：实际原 comic2 模板有 86 个图像但导航到 90。那个精确差异保留而不生成 87..90 章节或图像 URL。官方游戏详情链接观察为 HTML 壳；没有从它猜测 API 路由或小图标。

## 0.1.1 选择器路径语义

官方事件总线把 `output.data` 读为有序数组成员。源现在精确比较两个解码字符串 `option`、`slideIndex`，独立于 JSON 空白/转义。原 46 章节和全部 46 页身份/顺序由同工具链 010/011 生产 WASM 回放保留。未知选择器保持整体错误。新标准 manifest 包含必需 `icon.png`；既有官方派生 PNG 字节不变，其白角视觉问题保持 OPEN。不声称新 Native 验收或新站点失败。
