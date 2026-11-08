# Komiic 0.1.19

源 ID `com.komiic.koma`，语言及 NSFW 标记保持；crate、runtime 与 package 版本对齐。普通网站为 `https://komiic.com/`，不把 API 地址冒作网页入口，也不由网页元数据授予验证或登录权限。源图标使用官网 PWA 标志及其原蓝色背景，素材依据见 [ASSET_SOURCES.md](ASSET_SOURCES.md)。

搜索、详情、完整目录和热门分页使用既有 GraphQL 链；热门列表不作为独立首页重复展示。图像保留 page:index 身份、原 image URI 的 kid、返回 kid 的相等校验及原封面 Accept/Referer。所选镜像决定正文 CDN，不自动换站。

## 设置合同

`CHAPTER_FILTER` 默认 `all`，选择为 `all`（同时显示卷和章节）、`chapter`（仅章节）、`book`（仅卷）。每次 `get_chapters` 从请求 `settings` 读取；未知值保全量。

只在用户显式选 chapter/book 时保留同类型项，原相对序列、ID、标题、pageCount、章卷与续读元数据保持。没有照搬参考的排序去改变已采用默认，因为现有目录、下载与阅读记录消费原序列和身份。未改原图票据链或既有数据。

支持 `source_chapter_exclusions_v1` 的宿主在显式筛选时接收已验证完整目录中的实际 `excludedChapterIds`，只隐藏被筛掉的章节，不删除库、进度或下载。仅对属于该作品的精确旧章请求绕过筛选；目录不完整、ID重复或作品身份不符时返回错误，不发布排除集合。旧宿主及默认 all 保持原行为。非标准章节类型不能判定为已排除，返回错误并保留已有章节。

固定参考 949dea252eb8e867b7b40834a26e13088764ff55 的同一源以 mirrors 声明两个域并生成首选镜像设置；此前“独立源变体”说明有误。`preferredMirror` 默认 komiic.com，显式选择 komiic.cc 才切换 GraphQL 和图片描述域，不自动试探或更换；旧 komiic.com 图片描述在切换后仍按相同 kid 取得票据。旧书章 ID、默认请求、官网入口和 App 样式保持，不增加敏感明文字段或 Host import。

已缓存的两种镜像页描述仍按原 kid 取票据。.com 保留 img.komiic.com；.cc 保留服务端路径，使用同镜像 img.komiic.cc，失败不回退。封面 URL 不受正文 CDN 规则影响。

搜索查询按请求 JSON 字符串解码后再编码进 GraphQL，保留实际引号及反斜线；空查询、分页、镜像选择及原请求合同不变。

## 浏览及筛选

浏览参考固定 keiyoushi/extensions-source `4591fbc9bb571ad03aaea0f7428f403350e90e` 的 `src/zh/komiic/.../{Komiic,Queries,Payload,Filters}.kt`。`get_listings` 对应参考热门与最近更新入口，`get_manga_list` 读取宿主 `args.page.cursor`；热门与最近更新分别沿参考的 MONTH_VIEWS / DATE_UPDATED，站端每页30项；宿主分页游标绑定站端页号，不把宿主20项提示当站端偏移。类型采用官网 allCategory 动态读取，多类型为同时包含；排序、状态及色气程度沿参考。非空关键词搜索不消费浏览筛选；旧宿主缺少筛选作用域及动态加载能力时不暴露筛选。原作品、章节、镜像及搜索合同不变。

列表与搜索中，官网明确的 public.komiic.com/comics/<作品封面身份>/cover.jpg 或 cover.webp 可附既有图片解析器同样的 Accept 和所选镜像 Referer，方便新宿主首次公开封面加载；URL原样保留。票据图片、其他域和未知格式继续走原解析器，不发送凭据。

详情标签按官网返回顺序先保留 categories，再追加 warnings 原文，与固定参考 genre 语义一致；请求所需 warnings 缺失或格式错误时明确失败，不冒充没有警示。

## 推薦分類

对照官网 `/recommendations` 的 `assets/index-97tYF0_m.js`（`.com` 与 `.cc` 同版本）。书源在现有分类菜单并列读者推荐本周、本月、年度及 AI 推荐。读者榜按 `topRecommendedComics` 的 WEEK / MONTH / YEAR 请求普通漫画，保持官网排序。偏移按官网每批 30 部计算，额外取一部判断是否还有下一批，榜尾不发布游标。推荐分类不消费热门、最近更新或关键词筛选。

AI 推荐通过所选镜像已有的 Komiic 登录会话调用 `personalizedSuggestions`，按官网默认请求最多 25 部并排除已读及已收藏，批量查询作品资料后按推荐顺序显示。游客提示登录；账号没有推荐时返回空结果。登录 Cookie 不进入封面或正文图片地址。保留独立首页与普通分类的原合同。

## 账号会话

固定参考 HyperionHXH/E-extensions Komiic 1.6.13（`e3365a1ab3f1a4083bdf3aa98e744f610fc725c4`）
的 `Account.kt`，对照 Kototoro Parsers `b31a11184f6d4fa563fbc37f60fe8f9e0b391edc`
的 `KomiicParser.kt`，并核对两域官网同版本登录前端及实际端点。
原生表单沿公共认证入口。选中镜像仅使用其官方 `/api/login`，发送 email/password JSON。
HTTP 成功且本次响应带新有效 `komiic-access-token` 后，用同域 GraphQL `account { id }`
确认非游客账号，再用宿主私有事务提交。表单凭据仅用于本次登录，Asset 只持久化两个域
各自的访问 Cookie 和有效期，切换镜像与退出不影响另一域。Cookie 不进入封面或正文 CDN，
正文仍使用 `X-Image-Ticket`。游客可照常浏览和阅读其站点许可内容。

`get_authentication` 只读本地状态与镜像对应表单合同。已登录请求以官网 JWT 的 exp
为边界，采用参考的提前一小时窗口，或遇到明确认证失效后以同域 `/auth/refresh`
更新 Cookie。并发续期复用宿主原 Asset 的原代认领，账号查询通过才发布，失败保留旧代。
登出仅移除当前域的私有状态。最小宿主为 1.2.1，依赖
`source_native_cookie_session_v1` 及原生认证/私有会话能力。

官网游客 `getImageLimit` 返回 limit、usage 和 resetInSeconds，额度由站点决定。官网原图
GET 和 [Keiyoushi](https://github.com/keiyoushi/extensions-source/blob/main/src/zh/komiic/src/eu/kanade/tachiyomi/extension/zh/komiic/Komiic.kt)
以 HTTP 402 表示图片额度用完，Hyperion 1.6.13 同样区分 402 和 429。两个镜像在取票
阶段也会返回 HTTP 200 和 GraphQL `getImageTicket` 的 `QUOTA_EXCEEDED`。源保留这一
业务原因，旧宿主回退至 `rate_limited`。宿主声明
`source_image_response_errors_v1` 时，取票成功的正文描述附带这两项状态映射。403
仍可能是票据失效，不声明为额度用完。正常阅读不额外查额度或探测图片。

0.1.17 的正文额度修复最低需要 Koma 1.3.0。取票保留 GraphQL `QUOTA_EXCEEDED`，
原图 HTTP 402 显示为额度耗尽，HTTP 429 仍表示请求限流。两条路径都沿现有读取流程，
每页无需额外查额度。

## 正文大图

页面图保留为 `{当前镜像}/api/image/{kid}`，通过 GraphQL `getImageTicket` 取得票据，
再以 `X-Image-Ticket` 请求同一镜像端点。这一结构来自 Keiyoushi 与 Hyperion 的
`getPageList`（`baseUrl + "/api/image/" + kid`）以及 Kotatsu Redo 的
“若返回 kid，则拼接 `/api/image/{kid}`”。两个镜像的 API 主机都能用真实票据直接
返回图片，不能把页面图改写到 `img.komiic.com`/`img.komiic.cc` 子域：该子域在
部分直连网络不可达，也会偏离参考实现。封面仍按镜像把 `public.komiic.com` 改写为
`public.komiic.cc`；普通网页入口由 `homepageChoices` 跟随镜像。
