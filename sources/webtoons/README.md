# English WEBTOON / CANVAS 0.1.6

公开英文搜索、详情、集目录和阅读器页。既有 ID、API 目录顺序、图像页顺序和默认输出保留。请求保持既有 `locale=en` cookie；这个源不接受年龄/同意或不引入账号设置。

## 设置

所有设置默认 `false` 并从每请求快照读取。

- `showAuthorsNotes`：使用宿主 `webtoon-author-notes` 文字图像模板追加当前集的创作者注释。此设置只在 `text_page_image_v1` 时显示和消费。HTML 实体由源 DOM 解码一次；文字空白遵循 Jsoup 1.22.2。既有锁定/MotionToon 守卫在注释前应用。
- `useMaxQuality`：只从规范化图像 URL 移除完整的 `type=q90` 查询对。其他参数、片段、页 ID 和索引不变。
- `useSequentialNumbering`：以参考的 `pageSize=99999` 请求完整目录，按 API 顺序分配一基编号并拒绝不完整响应。设置关时，原游标分页和官方标题编号不变。参考反转其最终列表；这个源保留其既定 API 顺序而非改变目录合同。

源保持其既有响应限制和显式错误。不通告不受支援的列表或绕过上游访问限制。文字描述从不发送到外部图像服务。

## 实现来源

独立 Rust 实现和提供者资产归因在 NOTICE.md 和 ASSET_SOURCES.md 文档化。设置行为还遵循 Keiyoushi `extensions-source` 树 `949dea252eb8e867b7b40834a26e13088764ff55`、`src/all/webtoons/.../Webtoons.kt`，只有上述宿主能力和既有目录顺序适配。

搜索先解码请求中的 JSON 转义，再按既有站点请求规则编码用户输入；默认、空查询与分页规则保持原行为。

章节标题中的 Unicode 转义遵循 [SDK 公共文本解码合同](../../sdk/README.md)，合法表情不使整份章节响应失效。

## 独立列表候选

固定参考 `949dea252eb8e867b7b40834a26e13088764ff55` 的热门依次请求 trending、popular、originals、canvas 四个榜单，最新按宿主本地星期请求 `/en/originals/{weekday}?sortOrder=UPDATE` 并单页结束；旧宿主没有时钟特性时仅显示热门。2026-11-06 四个榜单官网均 HTTP 200，实际 WASM 与 HTML 桥各解析 30 项，第四页终止；同日星期五最新 162 项、无下一页。热门与最新为独立栏目。支持空词筛选的宿主显示参考的 ALL、Originals、Canvas 搜索分类；旧宿主不显示新筛选。英文官网 love 查询的两类前两页各30项、空关键词 Originals 为空并终止；Canvas 第10页下一按钮指向11，空格关键词 high school 下一页链接也已用实际 WASM 回放。默认 ALL 与正式源在同一原页逐字段一致。既有目录对照回放通过；普通实机和正式发布未验。
