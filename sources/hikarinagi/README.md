# Hikarinagi

Koma `com.hikarinagi.koma`，Mihon 来源 `3266193817034354049`。行为移植自 [Keiyoushi 固定版本](https://github.com/keiyoushi/extensions-source/tree/8947c372cd87f546a0b80cb0e4b0a5a8add5ef88/src/zh/hikarinagi) 与其 [Apache-2.0 许可](https://github.com/keiyoushi/extensions-source/blob/8947c372cd87f546a0b80cb0e4b0a5a8add5ef88/LICENSE)。

| 原有行为 | Koma 调用与消费 |
| --- | --- |
| 公开热门和最近更新走 `/api/pages/mangas/browse`，分别默认热度和更新降序。支持关键词和排序方向、地区、受众、状态、年代、杂志。 | `get_listings`、`get_filters` 和 `browse` 保持参数及顺序。Koma 的单选项组合了排序字段与升降序，年代同时传原 `decade` 与当前官网的 `years`。传 `page_size=24`，以实际 API `meta` 判断续页，当前实测该请求返回 40 项。 |
| 官网已把漫画关键词从目录的 `search` 参数迁移到全站作品搜索。旧目录请求对本次对照关键词返回同一序列。 | 有关键词时按 [官网搜索跳转](https://www.hikarinagi.org/_nuxt/DBitKYBi.js) 和 [查询构造](https://www.hikarinagi.org/_nuxt/BXt4AP2k.js) 走 `/api/pages/search?types=manga&q=…`。保留原站相关性排序和页码。浏览筛选归浏览界面，现行关键词接口没有这些字段。仅在新入口明确返回 HTTP 404 时保留固定上游的旧请求。 |
| 详情采用 `/api/pages/mangas/<id>`，章节数组按连载、卷本及番外分段，每段正序。 | `details` 优先读取中文名和中文简介；`get_chapters` 保留 ID、数字及页数，对前导零章节号标准化，仅当标题为空时使用卷号或番外序号。数组维持上游倒序展示，按站点章节类型分别设置阅读顺序，避免连载与卷本混为同一序列。 |
| Reader 获取 `/api/pages/mangas/reader/<作品>/<章>`，HTTP 401 要求网页登录。 | `pages` 保留作品与章节双身份，从 `manifest.pages` 生成原顺序和页 ID。包内声明可验证的正式登录页及唯一会话 origin。源没有普通设置。 |
| 正文页使用 POST `/api/v3/reader/mangas/…/content`，站点用本次客户端 token 派生的 AES-256-GCM 密钥封装响应。 | `image` 通过现有宿主 HTTP 发送 POST 并以作用域内 BodyRef 接收响应；原站会话仍由宿主按源和域名授权。源以 32 KiB 分块先验证 GCM 标签再解密到新的 BodyRef，交给阅读和下载共同消费，不在页表持久化一次性密钥。普通封面仍用 GET。 |

0.1.1 需要 Koma 1.6.0 的 `source_http_body_v1`、`source_body_v1` 和 `source_image_body_v1`；缺任一能力时保留明确的客户端不兼容错误。103 实机已验证热门、封面、中文简介、270 章目录和连载／单行本／番外分组选项；选择连载显示 232 章。未登录阅读提示登录，官网首话页表直接请求也返回 401。现行官网搜索的首条精确命中及第二页另有实证。真实候选 WASM 已对真实 270 章数据验证标题、分组和位置，并通过分块解密、认证标签篡改拒绝、401 和宿主能力缺失的受控响应检查。站点登录后正文与离线下载仍待真实账号验收。图标依据见 [ASSET_SOURCES.md](ASSET_SOURCES.md)。
