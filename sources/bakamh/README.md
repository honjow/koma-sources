# 巴卡漫画

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
