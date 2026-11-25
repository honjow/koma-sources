# 巴卡漫画

## 站点简介

巴卡以中文翻译的韩国条漫为主，有独立的 BL 区，也收录英文版和全年龄作品。部分新章与外传需站点账号。

## 0.1.3 上游搜索同步候选（2026-11-24）

行为基线为 Keiyoushi [8f5540b56ffa38160e3daa63568cfb7b888d612e](https://github.com/keiyoushi/extensions-source/commit/8f5540b56ffa38160e3daa63568cfb7b888d612e)
的 `Madara.kt`、`MadaraBase.kt` 与 `Bakamh.kt`；另核当前 HEAD
`582615a352e38380378cb3f3c3001b0fa7897fe9`，本轮搜索、筛选和详情逻辑相同
（父类仅有 `SMangaUpdate` 导入位置迁移）。

| 上游合同 | Koma 调用与本轮实现 |
| --- | --- |
| 非空词同页执行标题和别名两路，标题结果在前，按作品身份保留首次结果 | `BrowseViewModel.searchSource` → `args.query` / `args.filters` → `browse` / `archive_page`。标题用 `vars[s]`；别名用 `vars[meta_query][alternative][key]=_wp_manga_alternative`、`value=原关键词`、`compare=LIKE`，不带 `vars[s]`。两路保留非漫画排除、作者、画师、年份、状态、排序、成人和题材筛选。`merge_search_pages` 按 `manga:<WordPress ID>` 去重，不按标题或路径别名合并。 |
| 两路各按原始25条判断下一页，任一路异常使整个调用失败 | 现有宿主 HTTP 为同步接口，因此按标题、别名顺序请求；两路都成功且解析完成才交付。403、挑战、限流、取消、传输和解析失败沿原错误返回，不吞错、不返回半路结果。 |
| 空词单路筛选，网址搜索仍走作品分派；热门和更新单路 | `browse` 保持空白词单次 AJAX；作品／章节网址仍经 `series_for_url` → `details`，不进入标题／别名两路。 |
| 第N页上游表单为 `page=N-1`、`vars[paged]=1` | 宿主首屏 `args.page.cursor=null`，续页传上一响应 `nextCursor`（无值才回退数字页）；`request_page_number` 读取该直接字段。同一 cursor 用于两路，向上游发送 `page-1`，任一路满25时才返回下一数字 cursor；不以去重后的总数判断。 |
| 题材全部匹配为 taxonomy `operator=AND` | `filters::search` 修正原 `operation`；题材为空不添加，任一匹配仍省略，作者等 taxonomy 索引顺序保持。 |
| 详情排除与主标题相同及 Updating 的别名；新版简介改用 Markdown | Koma 保留独立 `alternateTitles`，不把别名重复附入 `description`；本轮补同标题去重。现有 `port::Node` / SDK 只有选择器、属性、纯文本读取，没有可复用 DOM→Markdown 处理，因此简介保留原 `p` 段落／无段落时全文；**HTML 强调、链接等 Markdown 转换尚未同步**，不新增源专用宿主能力或不完整 HTML 解析器。 |

237 初次复验时，搜索验证通过后可以正常返回结果，但打开筛选的题材获取仍返回403。日志明确该 GET 走 WebHTTP 子资源 fetch；上游 MadaraBase.fetchFilterData 对同一 `/manga/` 使用 Fetch.DOCUMENT。本轮补 `document_text::catalogue`，复用已存在的 Native DOCUMENT 请求及实际最终地址交付，仅接受同站漫画归档（规范尾斜杠可变），不放宽作品或阅读页所有者校验。题材链接按真实最终地址解析；失败不伪造空选项，不添加站点专用宿主接口。237重新取得包绑定的网页授权后，该Native文档请求实返200且携带当前WebView UA／Cookie。实站归档已变为作品卡片，没有上游 `div.genres` 目录；仅见的12个题材链接全属当页作品标签，不能视为完整选项。对齐Madara在题材目录为空时省略题材控件的语义，保留作者／画师／年份／状态／排序／成人筛选；不把空目录报为整个筛选失败，也不从卡片伪造选项。保留有目录时的动态题材与AND参数。临时公开DOM诊断已移除。最终包e16978b在237读回一致：两路搜索200，同词重复搜索无新验证；Native题材文档200并成功返回固定6项，筛选页实际展示作者、画师、年份、状态、排序和成人内容。

本轮只改源内搜索、复合题材、题材选项读取和同名别名。既有请求 profile、全 client 2次/秒
oncePerRequest、Native IMAGE/CDN/SourceCookies、WebHTTP/BrowserGrant、章节／页身份、
登录及全部设置保持；最低 Koma 仍为1.6.1，无新宿主能力，图标未改。

本轮已通过正式 `build.sh --source bakamh` 构建，最终包实际 WASM 的13项搜索／筛选／分页／失败与取消、9项文档请求边界均通过；真实旧归档仍提取6题材，只有当页卡片标签的页面按上游省略题材控件，原有执行期望未改。第二路返回同时取消的竞态已在源内修正。237普通路径已验0.1.3“迷宫”查询返回4个去重作品，两路POST200；首次安装验证自动收起并恢复原查询，同词重复搜索不再弹验证。Native题材文档带包绑定的授权Cookie及WebView UA，HTTP200，六项固定筛选上下段均已检查。本轮没有新增作品、阅读或下载，不借用0.1.2阅读／下载结果作为0.1.3新验收；未公开发布。
验收重点：别名独有命中、两路交叠与翻页、带全匹配题材的两路查询、空词及作品／章节URL、
任一路HTTP失败、详情别名和阅读图片回归。

### 本轮中间包回放（历史，最终包验收见上）

本执行者已通过 `git diff --check -- sources/bakamh`，以及对 `src/lib.rs`、
`src/filters.rs` 分别执行 `rustfmt --edition 2021 --config skip_children=true --emit stdout`
（输出丢弃、不写回）的语法解析；这不是类型检查、真实逻辑执行或WASM验收。
为避免与根代理正在进行的WNACG构建争用，未运行cargo、build.sh或改写根锁文件。

随后对根代理正式构建的694399字节最终包（SHA256
`a9b86ad4467a35130e5fcd1f37d035e0cd33fda3061b12ddda47c2a2aa17d3a8`）执行真实WASM：
复用既有Baka recorder及C++ HTML bridge，以受控AJAX HTML／HTTP响应验证。
标题与别名交叠保首次、两路同25条仍续页、单路25张重复卡片仍续页、仅标题满页、
仅别名满页、两路均不足25但合计40条不续页、第二路403整次失败、第二路发出前取消、
第二路HTTP失败且取消、空词及空白词单路、Native回退同筛选均通过。
HTTP完成同时取消的边界失败：原包仍返回成功。因此 `archive_page` 补上HTTP返回后、
解析前的取消检查；HTTP本身失败仍保持原错误优先。该修正**尚待重打最终包后重跑**，
不能用修正前12项通过冒充最终包全通过。证据在Koma本轮
`keiyoushi-sync-20261124/bakamh-actual-wasm/results-before-cancel-fix.json`；
未使用网络、设备或替代选择器，未重新构建测试框架，源仓不新增测试文件。

## 0.1.2 普通图片请求

普通 Native CDN 图片显式采用 SDK 共享 Chrome141 Android10/K UA 与有界 HTTPS
IMAGE BrowserHeaders，参考 Mihon c3bf7a78 和 Keiyoushi 8947c37，不采用 CF WebView
UA。按已核实际 origin/site 关系生成 CH、Accept、Mode/Dest/Site 和适用 Storage-Access；
未知 HTTP 协议不发送 H2 Priority，不声明压缩能力，完整章节 Referer 不收窄。

非 BASE 的 Native HTTPS 图像在协商现有 cookie-session 能力后选 SourceCookies，
仅用 Host 精确 origin 的 HTTP Jar，不读取或复制 BASE/WebView 授权。BASE 的原
WebHTTP/BrowserGrant 保持；无该能力保原会话回退，旧 descriptor 不变。全 client
共享2次/秒、oncePerRequest限速，覆盖CDN。现有 SDK 的 challenge/auth/rate/cancel
与 ordinary403 分类、所有权和单次 BodyRef 交付、页/章ID与正文解析保持，无新增重试。

普通 HTTP UA 是共享客户端身份；不把 CF UA、Cookie 授权、TLS profile 混为一项。
本版不含诊断，不新增 App 接口或 C++ 传输能力，最低 Koma 1.6.1。
2026-11-24 已在237实际验证列表封面、两次搜索、《健身教练》104话首/37/73页、
103→104连续跨章、整章73页下载和冷启动本地首尾页读取；未走网络兜底。
原11个下载、完整队列和设置已恢复，保留新包并释租。源 SOURCE_INFO 由 Cargo 版本生成。


完整验收后按用户既有授权恢复书源分发，安装门槛仍为 Koma 1.6.1；宿主是否发布由 App 的独立发布安排控制，不再用开发期 `distribution.available:false` 隐藏已验源。以下旧候选和失败记录是历史依据，当前状态以上述0.1.2具名验收为准，旧产物不重建、不覆盖。

独立源 `com.bakamh.koma`，对应 Mihon 来源 ID `3712194609356981776`，来源分级 NSFW。参考 [Bakamh](https://github.com/keiyoushi/extensions-source/tree/8947c372cd87f546a0b80cb0e4b0a5a8add5ef88/src/zh/bakamh) 和同提交的 [Madara／MadaraBase](https://github.com/keiyoushi/extensions-source/tree/8947c372cd87f546a0b80cb0e4b0a5a8add5ef88/lib-multisrc/madara)，代码许可 Apache-2.0。

| 上游行为 | Koma 入口及实现 |
| --- | --- |
| 热门、更新、搜索走 `POST /wp-admin/admin-ajax.php` 的 `madara_load_more`，查 `wp-manga` 的漫画类条目。每页 25 条，满页才续 | `get_manga_list`／`search` → `browse`、`filters::archive_form`，取 `args.page.cursor` 并换算为上游从 0 起的 `page`。保留上游中文 Accept-Language 和每秒两次请求。 |
| 作者、画师、年份、状态多选、排序、成人内容、题材多选和任一/全部匹配 | `get_filters` → `filters::schema`，题材选项延迟使用 `/manga/` 的真实链接。 `filters::search` 按上游的 taxonomy、meta 和排序形状组 POST。空词与词搜使用同一操作。 |
| 目录卡片的 WordPress 作品数字 ID 和路径提供详情；路径小写化，短链 `?p=` 用作补取。复制同站章节地址时通过其所属作品 ID 打开作品 | `get_manga` → `series`／`details`，保留标题、作者、画师、状态、封面、简介、别名和题材。普通 `search` 支持作品与章节地址，当前书源 ABI 无独立站点链接分派导出。 |
| 相关作品按最多三个题材查询随机列表，过滤泛分类并排除当前作品 | 具备 `source_related_manga_v1` 的宿主可用 `get_manga` 的 `relatedManga`。此请求失效时保留作品详情。 |
| 章节取作品页 `ul.version-chap > li`，以作品身份和章路径末段定位；日期优先采用图标 alt、提示 title、章节日期 | `get_chapters` 用 SDK 分组目录保留站点顺序和章身份。完整单平面 `no-volumn` 目录经身份、文档尾和第一章/最新章控制确认后，按站点最新优先数组赋予 `bakamh-main` 位置；无号也保前后。不完整、嵌套或多组尚未确认时保手动条目，不串独立版本。绝对中文日期和 Madara 相对日期都保留，后者按宿主时钟以 UTC 计算。 |
| 分页阅读切换 `style=list`，处理登录提示及工具警告，读取懒加载图或还原 Madara 的页表 JSON，图像请求以实际阅读文档为 Referer | `reader::pages`、`reader::image_request`。站点下发的页表只产生普通图像 URL，无独立像素处理器。只在锁提示明确要求登录时报告 `auth_required`，保留上游的账号提示。 |
| 上游在站点启用时为详情和阅读文档异步提交浏览计数 | 宿主 HTTP 为同步接口，详情及页表以短超时发尽力请求。目录和详情在 Koma 分开调用，取目录时不重复发送。 |
| 上游 `getHomeUrl` 是热门归档地址，登录通过 WebView | 源没有独立首页或设置。Web 会话仅声明 `https://bakamh.com`，由宿主处理同源登录与授权。 |

## 2026-11-20 原核查记录

以下为原候选时的范围与失败，保留作为历史记录，不代表本轮最终状态。

目前 [官网首页存档](https://web.archive.org/web/20250522060918id_/https://bakamh.com/) 和 [目录存档](https://web.archive.org/web/20250522060918id_/https://bakamh.com/manga/) 可核对品牌、作品卡片和题材路由。站点直连 403 或超时，存档未覆盖真实 AJAX 响应、完整作品目录及正文流程。

当前 WASM 的宿主 runner 验证了静态筛选，动态题材与热门直连均返回 403。 热门和搜索将站点 HTTP 403 保留给宿主显示；WordPress 的 AJAX 返回值 `0` 标记为无效响应。用官网目录存档回放提取了题材和作品卡片，并检查了搜索表单字段。合成样本覆盖 SDK 章 ID、绝对与相对日期、`style=list`、实际阅读页 Referer、页表还原及明确登录锁。真实章节与页图下载待站点可达后核验。

## 0.1.1 Web 上下文 HTTP 候选

保留原 Madara 普通 POST、完整表单/筛选/分页和阅读实现，声明通用 `websitePolicy.webFetchBridge:true`。此次依赖的是宿主 Web 上下文 HTTP 的 method/headers/body 支持，不是新增 Baka 专用 API，也不将已有普通 POST 写成缺失。最低 Koma 锚定 1.6.1；候选待本轮实际桥接宿主和普通真机路径验收，不提前公开。

2026-11-20 的403、GET返回0及旧合成验证仍是原历史证据；本轮真实原件、目录方向和最终包证据另存 Koma 的 source-candidates-20261120/bakamh/web-http-20261123，不以旧结果替代今日可用性。

### 完整单目录的阅读方向

新增真实[作品存档](https://web.archive.org/web/20241103022347id_/https://bakamh.com/manga/%E8%B5%98%E5%A9%BF%E8%A6%81%E9%80%9A%E5%90%83/)《赘婿要通吃》（WordPress ID 5929）的61章完整原件。
页面 `btn-read-first` 文案“观看最新章”、`btn-read-last` 文案“观看第一章”；其实际引用的
[template.js](https://web.archive.org/web/20241203105725id_/https://bakamh.com/wp-content/themes/madara/js/template.js?ver=1.7.3)
分别绑定章节DOM第0项和末项，倒序控制只反转原节点数组。因此以完整原数组赋位置，
不按标题、日期或数字ID重排。末项无号“预告篇”的旧ID仍为 `chapter:5929:c-43`，
位置0；其后是原第一话。61条均保留旧字段和原数组，只新增协商后的组/位置/可续读合同。

自动链仅用于有完整HTML文档尾、确切作品ID、唯一单平面 `ul.main.version-chap.no-volumn`、
第一章/最新章控制，以及全部同作品canonical路径的目录。缺字段、截断、嵌套/多个分区或
混入其它作品/版本时不推断位置；保留原条目、权限/登录提示及旧宿主未协商输出。

真实官网归档目录的题材读取、单目录61章及实际新WASM已回放；旧 `style=list`、
加密页表和明确登录锁沿原受控样本回归，不能称真实正文成功。历史原件不算今日直连或
Native通过。今日普通POST仍实际403，Edge目录访问仍是Cloudflare验证页。
最终候选须在通用WebHTTP bridge的新1.6.1宿主，用普通POST/搜索/筛选/分页/详情/目录、
实际正文图片和下载验收后交付。多分区和未取得的权限反例继续按实物核验，不以全部手动
或受控样本冒充整源完成。保留全部既有表单、选项、日期、图像Referer和阅读还原实现。

### 0.1.1 canonical 文档 GET

237 普通路径已验证同源授权后的热门 POST 及“健身”重复搜索200；旧文档 GET bridge 重试未命中后由带 Cookie/网页 UA 的 Native 返回403，封面直取也返回403，详情、完整目录、正文及下载尚未验收。前段 Mac 与归档记录保留各自的历史边界。

`get_document` 仅在协商 `source_web_http_v1` 后，为精确本站 HTTPS origin 下已有规范 `/manga/.../` 文档显式加入 `webFetch:{version:1}`，支持既有阅读 `style=list`。保留原 Accept、Accept-Language、同源 Referer、超时和解析。无能力、WordPress `/?p=<id>`、缺少末尾斜杠或其它需解析地址仍沿原请求与重定向路径。该选择不改变作品/章节/页身份、筛选、权限、章序或图标，不增加请求。图片 `get_image_request` 继续保留原 Referer；此处不新增图片传输或凭据字段。新版实际目录、图片与下载可用性仍由普通宿主路径验收。

### 0.1.1 已有 BodyRef 图片候选（待普通宿主验收）

图片仅在 `source_http_body_v1`、`source_image_body_v1`、`source_body_v1` 三项均存在时走源内 `get_image_request`：原 URL、章 ID 校验和实际章节 Referer 保留；使用上游公开图片 Accept 与中文 Accept-Language，普通 Host HTTP 默认 BrowserGrant 按精确原点附已有授权 Cookie/UA，不读取或输出凭据，不添加固定 UA、Sec-* 或新 Web 二进制接口。HTTPS 封面在同样协商下省略公开 `cover.headers`，由现有图片 resolver 取得 body，避免静态封面捷径绕过会话；缺能力或旧 HTTP 图片保持原 descriptor。

HTTP 必须成功，空正文、HTML/JSON 错误文档及没有图像签名/图像内容类型的响应拒绝。按宿主 chunk/maxBytes 预算将只读 HTTP BodyRef 分块复制到新 writable BodyRef，再经 SDK `image_result` 交现有阅读/封面/下载缓存；失败关闭本次引用，取消停止复制，不在403后暗中改传输重试。此包仍待237实测，Native 带会话的文档403不作为图片必定失败或成功的结论。GET-only候选另存，不覆盖。

### 0.1.1 R4 同源 WebHTTP 图片候选（须配套新宿主）

R3 在237完成自动CF后 Web POST 为200，同源 Native 图片 BodyRef 即使附已授权 Cookie/真实网页 UA 仍403。R4 仅在原三项 body 能力齐备、图片精确 origin 为 `https://bakamh.com`，且协商 `source_web_http_v1` 时，给同一个已编码的图片 GET/headers/BodyRef request 增加 `webFetch:{version:1}`。不新增站点 API 或能力 flag；非同源或没有 WebHTTP 能力时继续 R3 Native BodyRef，缺 body 能力时继续原 descriptor。

公开请求头、实际章节 Referer、图片验证、分块预算、只读输入复制到 writable BodyRef 的所有权交接均保留，无额外失败重试，不输出 Cookie/UA，也不添加固定 UA 或 Sec-*。这是未公开1.6.1开发合同中的既有 WebHTTP 数据类型扩展候选；旧 ead42 不支持该组合，R4 不可提前交设备安装。须由主维护者配对支持 WebHTTP bodyBytes/bodyRef 的新HAP后普通路径验证，当前不宣称已消除403。原GET和R3候选及失败证据保持冻结。

### 0.1.1 R5 canonical challenge 终止

`series()` 将 canonical 文档的 `challenge_required` 与原有权限/限流/取消错误一样立即返回，保留实际失败及 Host 已观察的同一个 GET，不再落入 `/?p=<id>` 覆盖或混淆句柄。没有 canonical 路径及原有非终止解析/查找回退仍保持数字 ID 解析语义；成功响应、全部业务 ID、图片 R4 接线与预算均不变。此次不改 UA、metadata 或 renderedDocumentFallback，R4包保留冻结，R5仍须配套新宿主验收，不宣称站点403已解。

### 0.1.1 R6 文档 Accept 单变量

R5配f2宿主的首批封面已取得 Web GET200并由主维护者亲审；同轮15:07:14 canonical `get_manga` Web GET仍403、普通forbidden无challenge标记。该结果不是全部详情/正文/下载通过。

固定上游 `8947c37` 中，Bakamh只额外声明中文 Accept-Language；KeiSource配置根 Referer并接入 BrowserHeaders；Madara详情/目录/阅读文档使用 `Fetch.DOCUMENT`。其Chromium文档 Accept包含HTML/XHTML/XML、图像格式、通配类型及signed-exchange，原移植仅 `text/html` 确有窄化。R6只恢复该公开文档 Accept的Chromium基线（与当前图片 Accept同族）；上游Chrome155起按实际UA条件增加的JXL不在此静态基线中，不捏造引擎版本或格式能力。既有语言、Referer、请求/身份、R5 challenge终止和R4图片路径均不改。

这是一个已证移植差异的单变量候选，不预先认定它是403原因。UA/`systemWebAgentOrigins`、metadata和RenderedDOM fallback均保持原样；继续由主维护者用同f2宿主核真实结果。无固定UA、Cookie或Sec-*新增，不启用新API，旧R5候选与原件保留。

### 0.1.1 R7 精确原点系统网页 UA 策略单变量

R6配同f2宿主实测在15:24:31的 canonical `get_manga` Web GET仍403，普通forbidden且无CF/challenge信号；恢复上游文档Accept并未解决该次真实失败。此前封面Web GET200且主维护者已亲审真实图，不据此宣称详情、目录或正文已通过。

R7仅在 `websitePolicy.systemWebAgentOrigins` 声明精确 `https://bakamh.com`，使用宿主既有系统网页UA策略；不扩展到www、其他原点或全局UA，不写固定UA。R6的文档Accept和全部WASM逻辑保留，不增加readySelector、DOM fallback、请求头/Sec-*或新API。授权自动完成分支的后续变更不属于本候选，R7先配同f2按普通授权/请求路径单独验证新UA，不能把旧授权会话的复用当作已切换。当前仍未宣称403解除；旧R6包与失败原件保留。

### 0.1.1 R8 授权就绪候选

R7配f2已证实际系统UA（customUaEqualsNative=true、verificationAndroid=false）；真人CF后automatic=true且POST200，但15:34:34 canonical文档Web GET仍403、普通forbidden无CF信号。这些事实尚未定位阻断原因。R8保留R7原生UA策略、R6 Accept及全部请求/身份/图片逻辑。

现有webSessionReadySelector声明为 `a[href*='/manga/'] img`，仅供授权自动完成的布尔就绪判断，不启用renderedDocumentFallback。既有HTML选择器桥实际消费保留的官网历史目录、作品页及真实CF原件，分别命中12、10、0个节点；历史成功原件不代表当前主页已经通过。此声明必须配父维护者含4a2d94ed仅授权就绪分支的新HAP，不能配旧f2声称已验；普通授权与canonical结果仍待设备owner验证。不新增Host API或Sec-*，不混入DOM正文回放。

### 0.1.1 R9 普通403恢复上游数字入口回退

固定上游8947c37的MadaraBase `supportsPostId` 默认为true，Madara/Bakamh未关闭；Bakamh `fetchMangaUpdate` 仅规范URL大小写后委托父实现。父实现先请求canonical，普通非成功且已有数字post ID时继续 `/?p=<id>`；目录保留数字ID及作品路径。R8以前将普通forbidden列为终止，错误阻止了这一既有入口。R9只把普通forbidden恢复到原数字resolver，不改变resolver的重定向/传输/请求头语义；没有canonical的数字入口与成功路径保持。

`challenge_required` 继续保留实际canonical挑战因果，不落入数字回退；`auth_required`、`rate_limit`、`cancelled` 仍终止。全部作品/章节ID、metadata（含R7原生UA与R8授权就绪selector）、R6 Accept、图片/BodyRef保持；没有新API或DOM fallback，不覆盖已交R8冻结包。受控WASM请求及保留官网原件回放只证明此回退合同与旧成功保持，数字入口今日是否可达仍待配套新HAP实测。

### 0.1.1 R10 显式WebHTTP文档Accept适配单变量

R9真实结果为canonical Web403，随后既有 `/?p=<id>` legacy retry/miss、Native403（body4547），源未通过。数字入口的legacy默认Accept与canonical显式WebHTTP不是同一对照，不能据此前者失败否定后者的单变量核验。

固定上游BrowserHeaders把完整DOCUMENT Accept与 `Dest:document` / `Mode:navigate` 配套；XHR类使用 `Accept:*/*` / `Dest:empty`。实际Web fetch的请求元数据由浏览器控制，不能伪造导航Sec-*。R10仅在已协商 `source_web_http_v1` 的精确canonical文档WebHTTP分支将Accept设为 `*/*`；非Web/native/数字resolver仍保完整DOCUMENT Accept。其它公开头、原生UA、授权selector、普通403数字回退、全部ID/正文/图片/预算保持，无新API或DOM fallback。

这是有待真实验证的传输适配假设，不认定它是403根因。候选可配原236539dff51c或父维护者的新只读观测HAP，保持其余行为变量不变；保留所有旧失败及冻结包。最小旧新实际WASM wire只核此Accept变化与原Native完整头保持，不扩大重跑业务。

### 0.1.1 R11 Native BodyRef原始文档候选

R10实机16:24:25 canonical Web GET仍403，数字入口旧bridge miss后Native403；隐藏观测已证UA/origin匹配、1×1视口、selector=true/CF=false。Accept单变量未解决，R11也不预认定阻断根因。

仅在精确已声明原点的文档GET同时协商source_http_body_v1与source_body_v1时，使用既有Native HTTP BodyRef与默认BrowserGrant，无webFetch、source session或虚构finalURL；Host仍负责真实授权UA/Cookie、网络权限、重定向及状态。新Native文档公共头为固定上游DOCUMENT Accept/语言/根Referer，及navigate/document/same-origin/?1、Upgrade-Insecure-Requests:1、DOCUMENT Priority:u=0, i。不生成私有UA/Cookie、客户端提示或伪造协议；上游Chrome的HTTP/1.1 Priority过滤/底层wire-order不由源伪造。Sec-*绝不进入Browser fetch。缺body能力或非精确原点保原R10路径；正文/封面图片仍原R10 Web BodyRef。

输入BodyRef只读，先取状态、严格原错误及句柄后结束Response借用，再用HTTP_OUT作RPC缓冲。正文直接分块读入既有缓冲，源内最多4MiB、单块最多64KiB且服从宿主更小预算；超限、提前短读、UTF-8不合法或取消均失败。解析HTML后关闭输入，所有成功/失败/HTTP错误都尝试close；清理错误不覆盖原业务错误，Host调用作用域作最终回收。无image_result、writable复制或完整HTML String副本。

普通403仍按R9用原数字ID入口回退；Host challenge/auth/rate/cancel仍终止，不绕CF或登录。原始正文权限、章节顺序、日期、ID、设置与metadata/selector均不变。真实保留HTML回放与受控资源失败只证明源适配，当前Native文档可达性、正文/下载仍待设备owner验证。

### 0.1.1 R12 POST显式WebHTTP选路兼容

通用Host将仅根据envelope.webFetch显式选择WebHTTP，旧普通POST继续Native。Baka仅在协商source_web_http_v1后，对既有浏览/搜索、相关推荐的post()及详情/阅读的view_count()明确增加webFetch:{version:1}；无feature保原wire及失败语义。共享port_support与其它源不改，无额外请求或重试；普通POST15000ms、可选计数1200ms、取消、原表单body和全部业务头保持。计数仍不把响应当目录，也不因失败阻断正文。

R11文档Native BodyRef、正文/封面图片R10 Web BodyRef、全部ID/权限/日期/设置、metadata/selector/图标保持。此为独立本地候选，不覆盖R11产物、不提前交设备、不推送发布，须由父维护者配对Host选路更正。回放复用实际WASM记录器；历史作品真实计数开关为0，计数POST的执行覆盖明确使用受控公开enable/id分支，不冒充当前站点或Native成功。

### 0.1.1 R14 canonical Web BodyRef

R13实机本地解码已见Cloudflare明确block页（Attention Required / Sorry you have been blocked），提示security solution triggered；不是托管挑战，具体触发规则仍未知。首个显式Web POST200且无cancel，去重已有实证。不能据标记或标题猜站点规则。

R14仅在已走既有文档BodyRef、精确原点canonical `/manga/.../` GET且协商source_web_http_v1时显式增加webFetch v1。公共头恢复R10 Web组：Accept:*/*、既有Accept-Language和根Referer；绝不把Native导航Sec-*、Upgrade-Insecure-Requests或Priority送入Browser fetch。数字 `?p` 及缺Webfeature仍原R13 Native DOCUMENT组。读取、错误映射及全部close流程保持R13，不新增请求或Host API。

保R12显式POST、正文/封面图片Web BodyRef、全部metadata/原生UA/selector/IDs/权限/日期/设置/图标，旧R13包及失败原件不覆盖。当前不宣称解除403；详情、完整目录和阅读仍须沿普通宿主路径核验。制作规范locale单句已独立docs提交ea64b812，本轮不再改规范。

### 0.1.1 R15 provider opaque路径编码大小写parity

固定8947c37正常卡片先把href经Jsoup/OkHttp变为encodedPath存memo.path，manga.url保存数字ID；Bakamh对manga.url的小写不改memo。此前Koma manga_path却主动ASCII小写编码slug，遇原始大写转义、Unicode或大写ASCII slug会产生额外路径改写。R15只保留Url正常编码后的原slug大小写，不再把opaque provider路由小写；不是403根因判断。

调用方边界：卡片验证不改原URL；详情/目录/reader文档请求保provider路由；作品ID仍数字、章ID仍数字+原章节叶slug，目录ownership guard、标签/status小写去重不变。旧页ID嵌了已小写的工作slug，因此仅在页ID序列化保原工作slug表示，章节叶slug/query保持；该identity兼容字符串不用于HTTP文档路由，原get_image_request及图片Referer保持。不得以路由更正重新命名用户已有ID；日后改变此身份合同须另有迁移授权。

版本/默认设置/权限、R14文档与图片/显式POST传输、metadata/UA/selector/素材均不改，保所有冻结旧包。

### 当前源码清理边界

临时站点响应取证代码及其专用正文读取、HTML描述符和字段已移除，保留正常文档读取与全部错误/关闭路径。R15的provider路径保真、旧页ID、原错误分类和数字回退、BodyRef/Web/Native路由及权限均不变；该清理提交未实施Native profile或viewport后备；下方Native text候选另行记录。设备owner继续使用已安装的原R15冻结包作独立对照，旧包与证据不覆盖。当前源码继续保持distribution不可用，等待完整验收和正式宿主。

### Native text作品文档与实际最终地址候选（待普通宿主实测）

作品文档在协商source_http_response_metadata_v1后用既有Native text GET、默认BrowserGrant和responseMetadata.finalUrl，按原HTTP状态/accessSignal先分类，再严格读取实际responseUrl；仅精确HTTPS原点的作品路径或原数字resolver可接受，缺失/异常地址不以请求地址冒充。canonical与原/?p入口使用同一作品helper；若已知ID的canonical成功但发生跳转，按固定Madara条件继续原数字resolver。最终标题/postID确认后将实际encodedPath保存为Series的已确认路径，供详情链接、相对资源基址、目录归属及后续作品定位消费，优先于可能陈旧的HTML canonical；能力缺失保原clean路径和HTML字段优先规则。URL搜索的直接reader地址沿原读取；作品/数字地址若实际返回同原点合法reader路径，只有原reading标记与数字postID成立才继续原所属作品查询，reader地址不写入作品canonical。

websitePolicy只在原授权https://bakamh.com声明responseUrlOrigins与pathPrefix:/的chrome131_android，覆盖canonical及原/?p数字入口。Host仍仅对无body文本GET选择该已有实现，不扩大原点或会话授权。它是移动Chromium兼容候选，现有设备证据仅有UA相等布尔，没有授权UA浏览器族/主版本，不能称已验证主版本精确匹配，也不按403结果轮换浏览器族。profile名不写成UA，真实授权UA/Cookie仍由Host持有；不声明新原点、私有session或伪造finalURL。底层profile实体上限1MiB、源正文4MiB与原完整文档/作品/目录守卫均保留。reader文档、图片、POST、权限与chapter/page ID算法不改，旧下载页ID不重写。普通403及非终止错误仍走原数字fallback；challenge/auth/rate/cancel仍终止。

这是CORS候选仍403后的独立后备。标准单源构建和实际WASM回放已完成：保留116659字节真实作品文档、完整61章，覆盖Native text/finalURL请求、无效最终地址、上游数字fallback和旧能力/reader URL搜索边界；受控最终地址元数据只验证接线，不冒充站点真实跳转或Native profile成功。普通宿主实际profile eligible/status、详情/目录/正文与下载仍待验，不宣称解除站点阻断。已移除的临时取证代码不恢复，distribution.available:false继续保留。

### Native text 阅读文档与实际最终地址候选

详情与当前105章目录已由237普通宿主取得Native profile200；chapter104页表仍走旧WebHTTP并返回403，尚无正文图片或完整下载验收。此次只将协商source_http_response_metadata_v1后的阅读文档接到已有document_text读取；没有该能力仍逐字沿旧文档通道。图片/封面BodyRef与显式POST选路、版本、metadata和distribution不可用门禁均保持。

固定8947c37的Response.asJsoup以真实response.request.url作基址；Madara getPageList读取原chapter URL，必要时再请求原地址的style=list，parsePages以document.location给相对图片及Page referer，浏览计数同用该文档地址。Koma复用同一Native text GET、DOCUMENT公共头、默认BrowserGrant与真实responseMetadata.finalUrl，不填虚构地址切路。真实授权UA/Cookie和mobile131 profile选择仍归Host，不冒称已匹配授权UA主版本。

阅读最终地址必须是精确HTTPS原点、无userinfo/fragment，仍为当前作品及章节的原完整reader path和query；不能把重定向的其它作品、章节、登录页或不同阅读模式当成本章。有公开postID时还须等于chapter ID的数字owner。通过校验的真实地址用于相对/加密页表资源基址、浏览计数Referer及reader归属；原权限/工具提示在页表解析前保持。页ID仍从原请求路径按历史工作slug规则生成，含style=list，不因finalURL或encoded case重命名旧下载；图像resolver继续按既有页ID还原Referer和既有Web/Native二进制通道，不增加网络或新的会话字段。

本候选用已有实际WASM/生产HTML桥对照旧作品原件和既有reader边界样本；受控响应地址只证明校验与接线，不能冒充今日reader200。正文跨章、真实图片及下载须由设备owner继续验收后才可公开。

### 图片请求的实际参考合同

固定8947c37的MadaraBase.getImageRequest显式传递page.url作为完整Referer；KeiSource只附加裸Fetch.IMAGE，Baka没有调用.from(page.url)。BrowserHeaders仅在存在FromPage标签时才收窄Referer，因此它的跨域收窄函数不属于Baka默认图片链。本源保留完整阅读Referer，不把框架可选规则直接套到该源。UA/Cookie仍由已有Host授权归属决定，页ID、图片URL和下载身份保持。

此前候选2074的跨域收窄依据有误，已停止使用；代码只定点更正这一步，Native文本与真实最终URL读取保持。原失败与更正依据保留于Koma本次native-image-headers-readonly报告。当前正文Native403继续调查，不以这项更正宣称图片已恢复。

### 已确认图域的原生 IMAGE 请求元数据候选

固定上游 `BrowserHeaders.hop` 对 IMAGE 请求使用 `Sec-Fetch-Mode: no-cors`、`Sec-Fetch-Dest: image`，并按实际发起页与图域关系确定 `Sec-Fetch-Site`。普通宿主第104话P1已实际确认图片在 `https://t1.bakamh.de`，完整阅读 Referer 在 `https://bakamh.com`，两者跨注册域。本候选仅为该精确 HTTPS 图域及本站 Referer 的原生 BodyRef 图片请求补三项公开元数据。237普通路径复测清洁c0fb候选的原第104话P1（1/73）仍为HTTP403，三项公开元数据未解除该阻断；本项只记录上游请求语义对齐，不冒称已知实际原生 UA 或完整浏览器请求指纹；不补 CH-UA、Storage-Access、Priority、Origin、Cookie 或固定 UA，也不把主站授权转授图域。

保持完整章节 Referer、公开 Accept/Language、原 URL/页ID/章ID、超时和 BodyRef 所有权、错误及关闭路径。BASE WebHTTP 图片、其它图域与旧 descriptor 输出不变；不改 App、binary profile 门槛或 source 版本。包内无临时路径/正文诊断。已用真实P1 URL做同 curl backend/UA/Ref 的两次无凭据对照，只有三项元数据不同；两次均403且为CF明确block页，不能外推设备 NetworkKit 的结果。普通宿主原P1、首中末、跨章及一章自有下载仍须完成，distribution不可用门禁继续保留。
