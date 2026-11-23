# 肉漫屋

## 0.1.1 簡介與多語言顯示

肉漫屋收錄韓國漫畫，包含劇情、現代奇幻等分類，兼有連載與完結作品。

零參 `source_info` 固定使用以上基準簡介；`metadata.localization` 僅新增 `source.description` 的繁中、簡中與英文文案，供支援的宿主顯示。

Koma `com.roumanwu.koma`，Mihon 来源 `3647420805839021718`。行为移植自 [Keiyoushi 固定版本](https://github.com/keiyoushi/extensions-source/tree/8947c372cd87f546a0b80cb0e4b0a5a8add5ef88/src/zh/roumanwu) 与其 [Apache-2.0 许可](https://github.com/keiyoushi/extensions-source/blob/8947c372cd87f546a0b80cb0e4b0a5a8add5ef88/LICENSE)。

| 原有行为 | Koma 调用与消费 |
| --- | --- |
| 热门读 `/books?page=N`，最新只取 `/home` 的「最近更新」区。搜索走 `/search?term=...&page=N`。空词筛选用 `continued`。 | `get_listings` 提供两个原有列表。`list` 把 `args.page.cursor` 转成从 0 起算的页码，按移动端页码文案决定是否续页。最新去重且无续页，关键词搜索忽略状态。 |
| 详情和目录共用 `/books/<作品 ID>` 网页。 | `details` 保留标题、别名、简介、作者、连载状态和分类。`get_chapters` 反转目录并用 SDK typed chapters 写入站点完整路径。原站该页只有一份书级更新时间，按上游只用于第一章，时区为台北。 |
| 阅读兼容 Next.js 和 TanStack 两种页表。仅 `sr:1` 路径按文件名的 Base64 原字节和 MD5 尾字节计算块数，逆序还原含余行的下方块。 | `page_paths` 先读取 Next.js 数据，再读取当前真实响应中的 TanStack `imagePaths:$R[n]=[...]`。`get_image_request` 保留原 URL，仅为 `sr:1` 图附像素阶段参数，使用随包处理器 `koma_reversed_strips_processor`。 |
| 扩展可设置新基础 URL，留空为 `rouman5.com`。额外深链接到 `roum29.xyz/books/...`。上游未提供登录配置。 | `baseUrl` 文本设置处理有效 HTTPS 站点 origin，不授予私密会话。当前宿主仅支持有限的官网映射，官网公告列出的四个域已入 `homepageChoices`。其他自定义 HTTPS 地址可用于源内请求，普通官网按钮回退包首页。两域工作链接和 Mihon 路径保留独立作品与章节身份。 |

本机默认域连接被拒。配置官网公告的 `https://roum29.xyz` 后，开发宿主跑通最新、热门分页、状态过滤、搜索、详情、18 章目录、38 页页表。首张真实 `sr:1` 图为 720×3600，5 块还原的 RGBA 与独立参考逐字一致。同章另有普通图描述透传。旧 Next.js 的 Pages Router、拆分 Flight 及引用 URL 已用受控回放验证，暂无现站样本。源登记最低 Koma 版本 1.5.5。图标依据见 [ASSET_SOURCES.md](ASSET_SOURCES.md)。

默认站原生响应偶尔在正文页表之前结束。只在默认域收到不完整的 TanStack 阅读文档时，按完全相同的章节路径请求官网公告备用域 `roum29.xyz`，核备站文档的 canonical 后解析。备站的 `/books/` 使用宿主已支持的 HTTP 配置，其余正文错误、显式选择的其他域、作品及章节身份沿用原逻辑。
