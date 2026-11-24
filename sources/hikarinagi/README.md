# Hikarinagi

Koma `com.hikarinagi.koma`，Mihon 来源 `3266193817034354049`。行为移植自 [Keiyoushi 固定版本](https://github.com/keiyoushi/extensions-source/tree/8947c372cd87f546a0b80cb0e4b0a5a8add5ef88/src/zh/hikarinagi) 与其 [Apache-2.0 许可](https://github.com/keiyoushi/extensions-source/blob/8947c372cd87f546a0b80cb0e4b0a5a8add5ef88/LICENSE)。

| 原有行为 | Koma 调用与消费 |
| --- | --- |
| 公开热门和最近更新走 `/api/pages/mangas/browse`，分别默认热度和更新降序。支持关键词和排序方向、地区、受众、状态、年代、杂志。 | `get_listings`、`get_filters` 和 `browse` 保持参数及顺序。Koma 的单选项组合了排序字段与升降序，年代同时传原 `decade` 与当前官网的 `years`。传 `page_size=24`，以实际 API `meta` 判断续页，当前实测该请求返回 40 项。 |
| 官网已把漫画关键词从目录的 `search` 参数迁移到全站作品搜索。旧目录请求对本次对照关键词返回同一序列。 | 有关键词时按 [官网搜索跳转](https://www.hikarinagi.org/_nuxt/DBitKYBi.js) 和 [查询构造](https://www.hikarinagi.org/_nuxt/BXt4AP2k.js) 走 `/api/pages/search?types=manga&q=…`。保留原站相关性排序和页码。浏览筛选归浏览界面，现行关键词接口没有这些字段。仅在新入口明确返回 HTTP 404 时保留固定上游的旧请求。 |
| 详情采用 `/api/pages/mangas/<id>`，随详情返回人员、标签及逆序章节。 | `details` 采用中文名优先、原作者和作画分开，保留状态及覆图。`get_chapters` 用 SDK typed chapters 保留数字 ID、标题回退、页数及其 `P` 显示标签。 |
| Reader 获取 `/api/pages/mangas/reader/<作品>/<章>`，HTTP 401 要求网页登录。 | `pages` 保留作品与章节双身份，从 `manifest.pages` 生成原顺序和页 ID。包内声明可验证的正式登录页及唯一会话 origin。源没有普通设置。 |
| 正文页使用 POST `/api/v3/reader/mangas/…/content`，站点用本次客户端 token 派生的 AES-256-GCM 密钥封装响应。 | `image` 在宿主提供 `source_image_post_v1` 和随包处理特性时产生上游请求。从宿主随机源取得一次性 token，维持原会话和 `Origin`。随包处理器先校验标签再输出，分段处理字节。普通封面仍用 GET。 |

当前 Koma Reader 图片合同只接受 GET。现有宿主会收到明确的 `host_feature_required`，该候选需等真正支持 POST 及其正文下载合同的宿主后再确定最低 App 版本。公开列表、详情、270 章目录及未登录读者 401 已经本机开发宿主验证。现行官网搜索的首条精确命中及第二页另有实证。POST 描述和 GCM 处理只在受控输入中执行，站点登录后正文、离线和真机待验。图标依据见 [ASSET_SOURCES.md](ASSET_SOURCES.md)。
