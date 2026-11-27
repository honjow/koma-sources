# 4KHD

亚洲写真图集，收录韩国杂志与摄影机构写真、华人写真和角色扮演，提供原站相册大图。站点包含成人内容。

## 移植对照

上游为 [Keiyoushi 4KHD](https://github.com/keiyoushi/extensions-source/tree/c09b174459c037c62292e4e7a929a0cd59430bc4/src/all/fourkhd)。调用形状以 [Koma 运行时 ABI](https://github.com/honjow/Koma/blob/master/docs/source-runtime-abi.md) 为准。

| 行为 | 上游 | Koma 调用及实现 |
| --- | --- | --- |
| 栏目与翻页 | WordPress `index.php?rest_route=/wp/v2/posts` 每页 20 条，最新按 `date`，原扩展热门按 `modified`。 | `BrowseViewModel` 传 `args.listingId` 与 `args.page.cursor`。`browse` 显示「最近编辑」，保留原排序。写真与角色扮演栏目使用官网 WP 分类 ID。 |
| 分类筛选 | 详情 DTO 读取 WP 术语，上游无筛选器。 | `get_filters` 提供从官网 REST 核实的分类，`browse` 把 `args.filters.category` 作为 `categories`，可与关键词同用。固定分类栏目直接打开该分类。 |
| 搜索与直达 | REST `search`，文章链接按 slug 查询。 | `BrowseViewModel.searchSource` 传 `args.query`。`browse` 保留两条路径，文章搜索和列表返回同一个 WP 数字作品 ID。 |
| 详情与目录 | 由 post ID 或 slug 读取文章，单篇为一册。 | `SourceMangaDetailRequest` 和 `SourceChapterListResolver` 传 `args.mangaId`。`fetch_post` 用稳定文章 ID 取作品信息、术语与日期，SDK 单册响应输出对应章节 ID。 |
| 阅读 | 从 REST 的 `content.rendered` 读取正文图片。 | `SourceChapterPageHydrator` 传 `args.chapterId`。`album_images` 优先读取正文大图链接，缺少大图链接时从正文 img 取懒加载地址。去除原站图片的数字宽度查询项并保留其他查询参数。 |
| 账号与设置 | 公开相册为匿名 HTTP。 | 设置回放返回空列表。阅读只取相册页图。 |

从官网实测的 1,300 像素样本是该站页面实际发布规格。共用 `wp_gallery.rs` 的 DTO、目录和图片工具，本站的 REST 路由、排序、限速和图床独立定义。
