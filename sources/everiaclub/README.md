# Everia.club

亚洲写真图集，收录日本写真杂志、韩国和泰国写真、华人写真与角色扮演。站点包含成人内容。

## 移植对照

上游为 [Keiyoushi Everia Club](https://github.com/keiyoushi/extensions-source/tree/c09b174459c037c62292e4e7a929a0cd59430bc4/src/all/everiaclub)。调用形状以 [Koma 运行时 ABI](https://github.com/honjow/Koma/blob/master/docs/source-runtime-abi.md) 为准。

| 行为 | 上游 | Koma 调用及实现 |
| --- | --- | --- |
| 栏目与翻页 | 最新使用 WordPress REST，每页 20 条。热门解析首页组件，一次返回。 | `BrowseViewModel` 传 `args.listingId` 与 `args.page.cursor`。`browse` 和 `html_listing` 保留两条路径，日本、韩国、泰国、华人和 Cosplay 栏目使用对应的 WP 分类。 |
| 筛选 | 压入 `categories`、`tags`、`tags_exclude`，从 WordPress 获取热度前 100 个标签。 | `get_filters` 初次返回已核实的分类。支持按需加载及三态的宿主打开筛选时取标签，旧宿主使用分别包含和排除的下拉框。最新栏目和搜索可以组合筛选。固定分类栏目直接打开该分类。 |
| 搜索 | REST `search`，另支持文章及分类、标签网页地址。 | `BrowseViewModel.searchSource` 的 `args.query` 在 `browse` 中分流。同站文章返回一个条目，分类与标签链接沿原站分页。关键词检索限定在作品标题，避免实测需要 22 秒以上的全文请求。过长输入按词和 Unicode 字符边界收紧，后续页使用同一检索词。 |
| 详情与章节 | 文章标题、标签、日期，单篇文章映射为一册。 | `SourceMangaDetailRequest` 与 `SourceChapterListResolver` 传 `args.mangaId`，以文章永久路径定位。`detail` 取标题、封面和标签，SDK 单册响应输出 `chapter:<文章路径>`。 |
| 阅读 | 从正文取懒加载图片，跟随文章的 `.page-links` 聚合。 | `SourceChapterPageHydrator` 传 `args.chapterId`。`get_pages` 按同篇文章的子页编号聚合、保序去重。已公布的子页遇到错误时返回失败，可重试整册。 |
| 账号、设置与相关推荐 | 匿名读取，另有 Mihon 相关推荐入口。 | 当前源提供匿名栏目、筛选、搜索和整册阅读。设置回放返回空列表，相关推荐未接入。 |

`wp_gallery.rs` 共用 WordPress DTO、编号游标和 HTML 图片工具。正文图床由本源按已核实主机和路径允许，HTTP URL 规范化为 HTTPS。最低版本 0.1.0，响应头和扩展筛选通过宿主能力协商。
