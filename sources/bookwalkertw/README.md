# BookWalker Taiwan

## 站点简介

台灣漫讀的繁體中文電子書店。漫畫區有出版社發行的日漫台版和台灣原創漫畫，以電子書卷冊上架，許多書可試閱。

固定上游为 [BookWalker Taiwan](https://github.com/keiyoushi/extensions-source/tree/8947c372cd87f546a0b80cb0e4b0a5a8add5ef88/src/zh/bookwalkertw) 与同提交的 [PUBLUS](https://github.com/keiyoushi/extensions-source/tree/8947c372cd87f546a0b80cb0e4b0a5a8add5ef88/lib/publus)，按 Apache-2.0 移植。Mihon ID 为 4168719360034135465。试读与正式版的页列不同，当前包不声明跨客户端页序等价。Mihon 声明直接识别系列路径，单本直查可走同源搜索，包内通用身份只支持一条作品路径规则。

| 上游 | Koma 调用与处理 |
| --- | --- |
| 热门 rank、最新 sell_desc，默认漫画 m=2 且以系列展示 | get_manga_list 保留两栏；search 接同一路径，应用排序、12 个漫画分类、连载及全年龄开关。仅 a[rel=next] 推进页码。 |
| 作者、标签、书系搜索链接与作品入口 | search 保留普通搜索链接参数。明确粘贴商品／阅读器链接时，从站点商品页和最早一卷识别系列；粘贴 /serial 时读取实际连载页。作品保留 /search?series=ID 或 /product/ID 身份。直达卡片与详情同时在顶层 `manga.url` 和 `links` 保留实际官网路径，前者由宿主传给后续 `args.mangaUrl`。详情、章表和页表以这个经源端核对同作品的地址作为直达路径；普通目录搜索沿原入口和错误。 |
| 系列首卷介绍、商品 Inertia、连载 HTML | get_manga 使用首卷或连载详情，保留作者及创作分工、出版社与译者等元数据。 |
| 单本、逆序卷列表、连载分页 JSON | get_chapters 处理三种目录，保留原商品章 ID、日期和原站实际所有权、试读标记。连载请求 order=desc、page_type=pc，读到 last_page。 |
| 原站读或试读 | get_pages 根据站点所有权选择 read、trial 或 chapter_locked。试读显示锁标与试读名称，仍可点开。给定已购分支仍须由官网阅读器确认授权。官方网页登录走 /login，游客正文与付费状态保持隔离。 |
| PUBLUS 最终阅读 URL → config.js → /c → configuration_pack.json | publus 模块使用真实跳转的 cid，试读仅带公开 CDN 签名，已购分支沿用站点给出的书级和 CDN 授权。页表保存稳定定位，get_image_request 短期签名按请求取得。 |

## 集成边界

- 主机上真实试读为官网 302 → preview.bookwalker.com.tw/trial-page → 同域 viewer.html?cid=…，目录标为可试读的商品 `177922` 实测返回 21 页；全部页都无 DummyWidth。游客访问同书 read 返回 HTTP 200 并明确显示「請登入會員」。公开试读首图返回 HTTP 200、JPEG。使用离线宿主向 WASM 回放真实最终 URL 后，确认 21 页公开试读定位与上一版逐页等同。客户端需要提供本文声明的最终 URL 能力，真实已购账号流程待验证。
- 公开试读由书源在启动阅读器的无正文 GET 请求中通过 `includeResponseUrl` 显式启用 `source_http_response_url_v1`。清单 `websitePolicy.responseUrlOrigins` 声明已确认的 `https://www.bookwalker.com.tw` 和 `https://preview.bookwalker.com.tw`，并从最终 `responseUrl` 读取 `cid`。未声明该特性的宿主返回 `host_feature_required`。网页授权仍只声明 `www`。固定上游按所有权选择 `read`，无独立租阅分支；租阅官网说明要求先登录，从購書＆租書一覽进入阅读。现有 `read` 分支期待的最终阅读器 origin 尚缺实证，未加入新传输白名单。旧宿主和授权后的已购／租阅正文均未验收。
- 2026-11-20，本机匿名热门、最新、词搜、筛选和系列搜索均收到 `403`，响应为 118 字节通用禁止页（`server: awselb/2.0`，无 `x-amzn-waf-action`）；0.1.2 在宿主可追踪的目录或搜索操作中，把官网公开 /search 收到的 403 关联到本次网页会话，直接打开同一地址。源声明书单卡片作为结果标志，宿主可按现有流程保存网页会话并重新请求。其他 HTTP 错误以及商品、正文授权仍按各自原有规则处理。同一热门地址在普通匿名浏览器显示首屏 24 条，使用官网搜索框搜索「膽大黨」显示 2 条。固定上游用 `202` 和 `challenge` 响应头识别 AWS 挑战，收到 `captcha` 响应头时提示在官网完成验证或稍候。保留上游 15 次/分钟的 /search 节流，由包内限速声明和搜索请求的 Sec-Fetch-User 共同匹配。
- 公开试读 `177922`（21 页）和 `14745`（22 页）使用明文配置且无 `DummyWidth`。另外两本试读的官方配置读取返回 `403 AccessDenied`，本轮未取得带键重排阳性样本。0.1.1 依固定上游解包已授权返回的加密配置，从包内派生文件名和像素重排所需材料。仅 `DummyWidth` 标记页请求随包 `wasm_image_v1` 像素阶段，覆盖无键、带键及有键但块宽为零时使用无键重排的分支；不把逐图材料写入页表。8 组受控响应的文件名、输出尺寸与像素和固定 Kotlin 参考一致。真实带键、无键重排，以及拥有授权后的阅读和下载还需实际响应确认。固定上游扩展无评论、独立发现首页和源偏好设置。

0.1.3 将最低 Koma 版本更正为 1.5.6，确保公开试读所需最终响应地址能力可用；内容解析与站点身份保持不变。
