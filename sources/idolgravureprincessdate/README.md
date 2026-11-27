# Idol.gravureprincess.date

日本偶像与声优的写真档案，按团体与杂志刊号收录，包括《Young Champion》和《声优大奖赛》。站点包含成人内容。

## 移植对照

上游为 [Keiyoushi Idol.gravureprincess.date](https://github.com/keiyoushi/extensions-source/tree/c09b174459c037c62292e4e7a929a0cd59430bc4/src/ja/idolgravureprincessdate)。调用形状以 [Koma 运行时 ABI](https://github.com/honjow/Koma/blob/master/docs/source-runtime-abi.md) 为准。

| 行为 | 上游 | Koma 调用及实现 |
| --- | --- | --- |
| 栏目与翻页 | 默认 Blogger Atom Feed 的 JSON 表示，每页 25 条。上游默认浏览叫什么由 Mihon 决定。 | `BrowseViewModel` 传 `args.listingId` 与 `args.page.cursor`，`browse` 将首页按发布顺序标为「最新发布」。`start-index` 从 1 开始。筛选和搜索实测总数字段只报告单页数量，满 25 条时继续请求下一页。 |
| 标签与搜索 | Idol、Magazines 和 Feed 补充标签。以 `label:"…"` 和关键词组成 `q`。 | `get_filters` 保留上游团体和刊物值，新宿主打开筛选后请求 `max-results=0` 的 Feed 取补充标签。`browse` 同时消费 `args.filters` 和 `args.query`，可筛杂志再搜刊号。 |
| 详情与目录 | Feed 记录原文链接、分类和发表时间，一个条目一册。 | `SourceMangaDetailRequest` 和 `SourceChapterListResolver` 传 `args.mangaId`。`detail` 用规范化 HTTPS 文章路径读取标题、封面、标签，以原文提供的文章 ID 取 Blogger 发表时间。 |
| 阅读 | `.post-body` 内获取图片锚点。 | `SourceChapterPageHydrator` 传 `args.chapterId`。`get_pages` 依原顺序用正文锚点的大图地址，旧式纯图片正文使用 img。HTTPS 规范化限定 Blogger 图床。 |
| 账号与设置 | 公开档案为匿名 HTTP。 | 设置回放返回空列表。 |

Feed 的 HTTP 旧链接直接转成 HTTPS。Koma 作品和章节 ID 只使用文章路径，保留跨调用的一致身份。
