# CosplayTele

独立 no_std Rust/WASM 移植 Keiyoushi `de743aed846da53e00c45ab26ee0e77f7bb4355b`（versionCode 1，lib 1.6，Apache-2.0）的 CosplayTele.kt、Dto.kt 和 Filters.kt。提供方：https://cosplaytele.com 。保留参考的 NSFW/全语言分类；这是摄影画廊源。

默认 Latest 列表、Popular API（包括其首页偏移 20）、文本搜索、作品/分类/标签 URL 搜索、分类选择、懒加载分类获取、详情/标签/日期、单 Gallery 章节和每个原 `.gallery-item img[src]` 位置都保留。当前动态分类按原标签映射合同覆盖默认；重复桌面/移动块不创建额外选项。重复图像 URL 保留独立原序号和 ID。不重写、过滤、裁剪或从广告提升任何图像。

宿主适配：WordPress Popular API title.rendered 由固定参考原样赋值。本移植按其 HTML 实体/标记解码为 Koma 纯文本标题，遵循 Koma 源运行时 ABI（源拥有 HTML 实体解码并发布普通 Unicode）。全部 20 条捕获的热门记录在证据中同时保留原始 rendered 和解码标题；它们与该参考赋值不是逐字节标题等价。Mihon `ede052af170e2ef2949dac2acd91adfb58043a96` SMangaImpl 存普通 lateinit title，SManga.toDomainManga 直接复制 title；这些模型路径没有观察到等价清理。后来 UI 渲染未确立，故本移植不断言 Mihon 已做相同转换。

Koma 没有相关作品操作；那个可选参考操作不能在此暴露。筛选头/获取字段依赖宿主声明。日期保留参考的日历日而不发明时区。参考没有自定义账号、图像头或解密合同，故使用标准 HTTP/Reader 图像传输。Home、设置、凭据和 image_request 能力为 false。

ID 保留原站点路径：cosplaytele:/robin-2/、cosplaytele-read:/robin-2/、cosplaytele-page:/robin-2/:0。漫画/章节所有权必须在联网前匹配。作品 URL 搜索接受相同固定提供方 origin。不引入任意域抓取或镜像。

失败区分传输、401 登录、403 拒绝、404 缺失资源、429 限流、5xx、畸形 HTML 和无图像。404 不断言下架。无自动认证、拒绝后重试或虚构回退页。

本地验证 2026-11-05：五个列表/搜索/分类层（57 个不同发现路径）、55 个完整作品样本、55 个单章目录和全部 3204 个原图像位置匹配独立保存的网站解析。12 个不同画廊原件解码；10 个目视确认为实际照片/拼贴，不是广告、文字页或空白。Latest 和 Robin 正文也经真实 CLI/WASM HTTP 运行。冻结回放单独检查热门顺序、完整页表、动态分类、所有者不匹配、重复位置、401/403/404/429/500/传输错误、无图像和后续恢复。

本地分母不证明每个画廊都工作。在设备 237（PuraX）上，Root 通过普通导入、URL 搜索、详情和 Gallery 阅读验证了 Robin 1/62 和 62/62 以及 Evanescia 1/99 和 99/99。每次事务后恢复原数据和设置。详细证据：Koma `.hvigor/outputs/source-coverage-20261105/new-source-ports/`。公开分发由源发布管线跟踪。

评审更正：opt-in SDK source_protocol 在任何 HTTP 前校验直接请求字段、重复身份和 JSON 字符串转义。带引号/Unicode 搜索由既有 SDK 解码器解码后 URL 编码。HTTP 拒绝状态先于宿主传输完成；状态 200 带 ok=false 仍是网络错误。确认的正常文章模板无直接画廊图像时用 chapter_unavailable；未知/畸形模板保持 parse_error。功能检查只读实际顶层 hostFeatures 数组。V1 候选证据保持不可变；这些更正单独冻结为 V2。


封面补全（用户报告 2026-11-06）：参考的详情解析器和本移植的旧作品 URL 卡片没有缩略图。现在两者共享一个可选解析器，用于文章自身声明的 `link[rel=alternate][type=application/json]` WordPress 文章端点，采用 `wp:featuredmedia` 嵌入关系。返回的文章 id 必须匹配该声明，link/slug 必须匹配作品，`featured_media` 必须等于所选嵌入媒体 id。不使用 gallery-first、相关作品或任意嵌套 `source_url` 回退。`wp:attached-to` 是上传初始父级的证据，不是门禁：一篇文章可以复用另一篇文章的图像作为其选择封面。原资源 URL 保留。辅助元数据拒绝/传输/schema/所有者失败保留先前可用文章而无错误封面；取消仍传播。主 HTML 失败仍是失败。章节和全部页 URL/ID 不变。跨既有层和 12–126 照片相册的四个真实声明元数据/图像用例已检查；55/3204 冻结章节/页响应与 V2 完全相等。V3 单独导入到设备 237；Root 目视确认两个具名详情的真实特色封面，保留其单章目录和原书签状态。测试数据、偏好和电源已恢复。
