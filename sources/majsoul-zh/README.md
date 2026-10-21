# 雀魂官方漫画

独立 source `com.catfood.majsoul.zhhant.koma`，版本 `0.1.1`，语言 `zh-Hant`，最低 Koma `0.1.1`；使用现有 Host/信封 ABI v1。这里仅适配官方“漫畫壁紙”栏目中的一个漫画合集和其 150 条真实漫画记录，不将 150 话当成 150 本书，不支持另列的 80 条壁纸。

官方身份链为猫粮工作室 / Catfood Studio 的 TapTap 开发者页 `https://www.taptap.cn/developer/56654` 的官网字段 → `https://www.maj-soul.com/` → HTML 声明主脚本 → `GalleryList.json` 的 comic 分类。早先候选 `https://www.majsoul.com/` 的 404 为另一域名的单路径证据，不重试。Steam 商店 HTTP 200 实际是地区错误页，未用于官方身份确认。

合集名称采用“雀魂官方漫画”；代表封面选用目录中第一话原图，是适配选择，原站没有单独声明漫画合集 cover 字段。每话标题保留原 `subTitle` 和 `title`，仅按原 client 的 `replace("\\n", "\n")` 做一次 literal 换行转换。原 `title` 独立保留用于合同校验。章号仅从显式 `第…話` 解析，完整 1–150；原 JSON 63/64 话相邻位置倒置保留于 `JSON_ORDER`，读取顺序按显式章号升序。原官网图库 `timestamp` 升序后 reverse 是展示顺序，未用作章号。timestamp 1/2 不变成 epoch 发布日期；作者为空、状态与内容评级 unknown、卷号/发布日期/更新时间 null。

运行时 browse/search/detail/chapters/pages 每次操作只 GET 一次已取得的完整原目录 URL（含其普通 `v` 缓存参数），核对 150 条原 title/subTitle/link/timestamp 和原 JSON 次序后才产生数据。固定 v 来自本次公开原请求，不制造 clock/random/token，也不依赖跨操作内存缓存；服务器和中间缓存的鲜度 unknown。上游增加、删除、重排、改名、换链接或改排序字段均 fail closed，需重审源包更新。壁纸记录不参与漫画校验或输出。

`get_pages` 每章只返回原目录中一个原图描述符，单图保持全部四格、原顺序和 URI，不分格、不拼接、不猜文件名或第二页。`get_image_request` 零 HTTP，仅从不可变原 URI 许可表返回请求。严格检验真实 envelope sourceId、MID/章节 pageId、可选 mangaId/mangaUrl 和精确 HTTPS URI；正确 MID 只允许第一话代表封面，正确第一话 pageId 也能解析同一 URI，二者不混淆。未携 pageId、空 sourceId、错作品/错页/外域/增删 query、路径遍历和壁纸均拒绝。图片 Referer 是实际官网 origin 首页，未引入凭据。

实现沿用本项目已有 source-local guarded exports，维持 SDK 与公共 ABI 不变；它检查 guest 内存范围/1 MiB 边界、取消、args 对象及操作/sourceId。每个回放操作创建新的 Module。原 SDK 的导出宏不提供此信封身份/范围组合守卫，未为本源扩大或改动 SDK。

唯一已取得正文是第一话原 PNG：900×1200，繁中麻将四格笑话；未见成人内容。其余 149 图仅有官方原 URI 和单图消费合同，没有实体可用性、逐页年龄、下载或 Native 验收结论。全站使用率和逐话评级 unknown。官网透明 PNG logo 原 bytes 保留，未加底板、阴影、遮罩或替换形状。

原 HTTP / JSON / JS 精确 byte slices / 首话与 logo：`/private/tmp/koma-majsoul-zh-contract-20261014/`。完整生成依据/源 diff/生产包与 WASM 回放：`/private/tmp/koma-majsoul-zh-implementation-20261014/`。Host 回放不是 Harmony Native 验收；未推送、发布或操作设备。

## 2026-10-21 Native 限定观察（237）

现装 0.1.1，经普通 Browse → 唯一合集 → 继续阅读第一話（`majsoulread:zh-Hant:official:ch-1`），完整原图四格同时可见、页码 1/1；未拆图、未把相邻章当第二页。自然 Back 回详情仍显示第一話第 1/1 页，真实 `reader-sessions.v1.json` 对应 pageId `majsoulpage:zh-Hant:official:ch-1:0`、pageIndex 0、totalPages 1。只接受该一话完整画面与普通返回，不外推其他149图、下载、cold 或目录鲜度。

原件：Koma `.hvigor/outputs/user-flow-audit/device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership/majsoul011-{first-complete,natural-return,session-restored}237-001/`。已返回原 Browse 末端，50 项 sourceId/enabled 顺序逐项保持，AWAKE/原 OverrideTimeout 10000ms 读回；哈希关闭。额外旧 `reader-progress.v1.json` 导出不存在的失败保留于 natural-return 回执，未计成功；真实会话读回在 session-restored。执行者与 Root 均已亲看上述完整原图和自然返回，结论仅限本条路径。
