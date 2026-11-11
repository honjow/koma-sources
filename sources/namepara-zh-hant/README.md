# 菇菇官方四格漫畫 · 繁中 0.1.2

源 ID `com.beeworks.namepara.zh-hant.koma`，API0.2，minAppVersion0.1.1。一个官方繁中合集，MID `namepara:zh-hant:four-panel`，https://namepara.com/zh/info/comic/ 。源用量和单作品内容分级未知；manifest nsfw=false。这个官方中文源不声称是 Keiyoushi 的实体。当前 index.pb SHA `e26c3c6bf077e4bbdaa389fdb7f14020ec8297a45aec412b6934ac10e537dce3` 与保留的正式103处置集不变。

MangaToon (Limited) 中文实体 `5676233752760414531` 是既有保留的规范404/首页壳分支；两个失败 URL 和旧首页都没有再次请求。少女前线官方中文站暴露了插画画廊；检查了一个精确首个链接原件，没有从那个画廊实现漫画源。

## 真实归档、身份与继续边界

官方归档声明四页。源抓取第 1 页，读取其实际 `e-pagenation-indicator`，要求每个后续 index_N.html href 都已在第一页显式声明，然后每个声明页抓取一次（上限32）。源使用既有 `source_http_batch_v1` 宿主特性以最多三页一组请求独立后续页。第一个请求仍必须建立归档，响应页按原顺序校验和消费。没有这个特性的宿主保留既有独立请求。失败的组从不隐式重试或作为部分归档使用。它校验规范分类所有者、中文标题和每页的当前/总数指示器。只包含带那个分类芯片和精确 `/zh/info/comic/<六位>_<六位>.html` 身份的实际漫画文章卡片，排除固定的 APP/新闻项。

四个原始页含 35 张漫画卡片和 34 个唯一文章 URL。两张卡片都指向 `251107_100000`，显示发布日期相同但缩略图不同；源保留第一个实际身份，不虚构另一篇文章。重复身份且日期冲突则失败关闭。章节顺序保持首次出现的网站归档顺序，不排序也不推断编号。

每章的可见标签是精确的网站发布日期；它是日期标签，不是声称的故事副标题。文字故事标题和章节/卷编号缺失。因此 chapterNumber=null、volumeNumber=null、readingGroup=`namepara:zh-hant:four-panel`、continuationEligible=false、pageCount=null。不从日期、图像文件名或列表位置推断章节编号。既有 App 的手动章节选择可以消费那些真实源身份/顺序；实际手动操作仍留给 Root。自然页边界推进和自动阅读的相邻章节序列不由本源合同承诺，其全局门禁不放松。

## 实际原页与图像请求

一次 get_pages 调用先对照完整归档校验文章成员资格，然后 GET 那个精确原文章。它要求 body-article-comic 和规范 og:url 等于实际请求的文章 URI。它只读取 `#js-article-content.p-article-content.comic` 内的原 img src 声明，保持 DOM 顺序和完整原 URL。28 个保留的实际详情中这是两页：先是故事标题图，然后完整四格图。源不裁剪、合并、缩放、转换格式或用缩略图替换原图像。

文章 og:image 只是预览（首个保留预览是 640x452）。从不作为阅读器页使用。实际第一页标题图是 1173x352，完整四格 PNG 是 1173x3487；两个声明的原二进制各抓取一次并目视检查。源接受真实文章体中实际观察到的 PNG 和 JPG 后缀；一个标题图是 JPEG。首个仅 PNG 实现的源/WASM/失败回放单独保留。

图像 URI 合同是 HTTPS namepara.com/zh/info/img/<ASCII 字母数字、下划线、连字符基名>.png|jpg，无外部宿主、查询、额外目录或穿越；精确官方 touch 图标单独验收用于封面/身份。提供的 mangaUrl 必须等于官方合集 URI；提供的 pages mangaId 必须等于 MID。无网络的 image_request 保留发出的 URI 和普通 Referer，不独立证明任意调用方提供的允许文件名的加密文章归属。原文章的规范所有者和正文映射在 get_pages 中建立那个关系。

HTML 用有界字面模板检查和转义 JSON 输出解析，不求值网站 JavaScript、HtmlHost 可选属性、登录、cookie、签名虚构、挑战处理、付费内容或 DRM。模板/身份/分类/分页/URI 失败返回显式错误。ID 精确保留所有原始数字作为 URI 身份，不解释为日期或章节编号。

## 原官方图标与有限证据

icon.png 是未修改的官方头部声明 apple-touch-icon-precomposed PNG，https://namepara.com/assets/img/apple-touch-icon.png 。原 114x114，6797 字节，SHA `b90b74ad1cd3f1bc3f48442137b522615987a5fa0c7a075a2f98ccbcf6daa532`。黄色背景和蘑菇画是固有官方图像像素。没有添加底板、阴影、遮罩、重绘、裁剪、缩放或重编码。包校验器接受原资产；Native 图标外观保持未验收。

证据根 `/private/tmp/koma-namepara-zh-hant-readonly-20261014` 保留全部 4 个实际归档页和 28 个原详情 HTML 体带头/SHA、实际 34 个身份/日期顺序、原始声明图标、首个标题/完整原 PNG、预览角色区分和隔离的仅保留宿主。生产 WASM 106 次操作验证完整 34 源顺序元数据、全部 28 个实际详情映射/56 个原页描述符/56 个图像计划、14 个畸形输入拒绝和 ABI 范围边界。它不执行网络也不虚构 HTTP 响应。隔离运行器不是 Native 宿主。

一个剩余声明文章 `251020_100000` 在任何 HTTP 响应前 TLS 握手 UNEXPECTED_EOF；原因/权限/挑战状态未知。未重试也没有使用其他客户端/域名/身份。其余 5 个文章体在那个中断后未请求。这 6 个体没有虚构回放材料也不声称已验证。其他 27 个保留文章的图像二进制未抓取；只有首个标题/完整图像和官方图标有直接二进制检查。Native 浏览/正文/手动章节选择、自然推进、下载和冷持久都留给 Root 的具名 197 验收。

无 App、ReaderKit、SDK、主运行器或主源 WIP 变化；所有更早候选源/包保持冻结。无提交、推送、发布或设备操作。
