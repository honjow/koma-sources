# DOTA2 官方中文漫画 0.1.1

仅官网 https://www.dota2.com.cn/comics/ 明确发布的《我们是英雄了吗  Are We Heroes Yet? - 秘密店铺之传说》。使用率 unknown；源和逐作品正式分级 unknown，样本为普通中文奇幻漫画。

实际目录列出官方第1..6章，href comics.htm?=0,8,16,24,32,38。该 query 被公开 /js/cartoon.js 解释为全书初始页偏移；前后翻页使用同一个 cartoonJson.cartoonJson1 数组及其完整长度，并无六个独立页数组或章末边界。源提供一个完整阅读单元，原作品标题作为阅读单元标题，章号/卷号 null，readingGroup dota2:zh:are-we-heroes-yet，continuationEligible false；不从偏移猜六章页切片，不重复六份全文、不伪造连续章。48 原 URI 逐条从公开脚本实际数组解析，保留原顺序，其他五个未使用数组排除。get_pages（及取目录页数的 get_chapters）仍通过三次真实 HTTP 核对目录、viewer 身份、实际控制器及数组，并与包内已核官方48 URI 原字节有序许可表逐项比对；替换或重排返回 official_contract_changed，绝不从许可表补造正文响应。get_image_request 不再重复发起元数据 HTTP，仅对该许可表的原 URI 和实际位置执行独立 owner/规范 PageID 校验；URL/Referer 不依赖实时凭据。不运行网页 JS，不猜 API。

同一书、单阅读单元和 PageID 绑定，严格 HTTPS 官方 host、真实 part_1/page_两位数_cn.jpg 路径及现存数组成员；query、额外路径、跨 host/work/page 拒绝。mangaUrl 独立检查。cover 仅官网目录原 img1.jpg 缩略图，未把它当正文。书架更新能力/图像加载与下载均待 Root 在真实 App 验收。

图标来自官网 head 声明 /favicon.ico (SHA256 1d262a4844cca735a0cfe2570b7a362cb7b1b4006f6dab8e3f9daf027a9227f2)，直接提取 ICO 第10条已存在的256×256 PNG 数据，74612字节、SHA256 2bb8d047ce0d30301c6d0bb43d1cd3382fdedeed01f784659f691722f8d14167。PNG 原字节不解码重编码、不裁切缩放、不加背景/阴影/遮罩，图案与已有透明保留。原 ICO 和提取依据一并冻结。

官方目录、第一viewer、脚本、原图标以及第一实际中文正文 GET 原字节/头/SHA 均在 /private/tmp/koma-dota2-zh-readonly-20261014 保留。正文只取第1页一次，其他47图片实体未请求/未解码。生产 WASM 保留响应回放不能替代 Native host/设备、首图加载、48图下载或全48解码验收。无登录、cookies、挑战绕过、付费或 DRM。

依赖原 SDK，使用已审两宏及显式范围校验 ABI 导出约定，保护 SDK/runner/App/Kit/主树 WIP；未提交、推送、发布。

R2 冻结时仍为未安装的0.1.0，R1全部冻结实体保留。该R2相对R1仅本文件与 lib.rs 改变。许可表逐条来自保留原 cartoon.js (3723B，SHA256 d8039a387faa7dc63afc6b50823fc031ee478e7eb8cf3ba99fe2d614e98c88a6) 的实际 cartoonJson1 数组；不从文件名或页号生成 URI。保留同一单作品、完整48原页、原顺序与所有旧成功输出，以及18原 fail-closed 请求边界。该表是可信包内已核官方合同快照，不是服务器签名、授权 token 或可写运行时缓存。当前 Native 每调用新建/销毁 WASM 实例，不能复用 get_pages 的 guest static 状态；本 R2 不改 App/SDK/ABI。

明确的行为边界：get_image_request 不再每图实时核对官网目录/viewer；get_pages 仍做真实读取，而上游48项重排以前可能被接受，R2显式拒绝，必须审核新的官网合同后再更新源。图像请求计划不证明实体已下载/解码；首图以外47实体、Native读取/下载仍未验收。未增加网络或设备操作。

R3 source-only 0.1.1 封面正确性修复：R2 已由 Root 普通安装为0.1.0，首次 Browse 的封面占位 FAIL 保留。实际 App 封面链将 RemoteMangaID `dota2:zh:are-we-heroes-yet` 传作 args.pageId，R2 将任何带 pageId 的封面错误归入正文验证，拒绝原 `img1.jpg`（invalid_request，0HTTP）。0.1.1 仅对原 COVER URL 接受没有 PageID 或 PageID精确为同一 MID；其他 MID、正文页ID用于封面、作品MID用于正文仍拒绝。mangaUrl 独立校验仍仅允许原 BASE；URL/Referer、原页许可表/顺序/标题/章号null/eligiblefalse、图标、ABI不变，无 UI/App/SDK 改动。

R1/R2全部冻结保留。原75请求/输出与18拒绝保持（sourceInfo仅版本步进）；新增实际App封面MID正向及错MID/正文ID/错误mangaUrl负向通过生产WASM保留回放，不代表Native封面已恢复。未重新抓取远程原图或其他47正文实体、未设备操作；Root后续普通更新并验收。
