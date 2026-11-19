# 177漫畫

独立源 `com.yiqiqimanhua.koma`，原站 177pica.com。对应 Mihon 来源 ID `4420888243278089080`，作品与唯一章节沿用 WordPress 数字文章 ID。上游内容分级 NSFW。参考 [YiqiqiManhua、Dto、Filters](https://github.com/keiyoushi/extensions-source/tree/8947c372cd87f546a0b80cb0e4b0a5a8add5ef88/src/zh/yiqiqimanhua)，代码许可 Apache-2.0。

| 上游行为 | Koma 入口及实现 |
| --- | --- |
| 热门先以 REST 取得最新文章的链接，再读取文章侧栏的 `#hot_post_widget li`，无分页 | `get_manga_list` → `popular`。封面还原 timthumb 查询参数中的原图地址。 |
| 最新为首页及 `/page/{n}/`。按分类浏览用 `/html/category/…/`，带词搜索为 `?s=`，选分类时附 `cat` | `get_manga_list`／`search` → `browse`。六个分类的 ID 和路径来自上游 `CATEGORIES`。是否续页由 `a.next.page-numbers` 决定。 |
| 作品参数为 `?p={id}`，公开永久链接文件名为数字文章 ID | 普通 `search` 识别同站作品地址。详情和分享链接使用 `?p={id}`。当前书源 ABI 无独立站点链接分派导出。 |
| REST 通过 `?rest_route=/wp/v2/posts/{id}`，用文章标题、分类、正文首图构造完结作品 | `get_manga` → `details`。当前宿主没有 Mihon 的 `ONLY_FETCH_ONCE` 作品更新标记。 |
| 单篇画集返回一个 `Ch. 1`，日期取 `date_gmt`，页序等于正文 HTML 图片顺序 | `get_chapters` 用 SDK `ChapterResponse::single`，`get_pages` 提取 `img[src]`，图片按上游切换到 HTTPS。 |
| 上游遇到带 `Retry-After` 的 429 时等待后重试一次 | 当前宿主 ABI 没有延时重试钩子，返回现有频控错误。下次请求沿原搜索和游标进入。 |

分类筛选走 `get_filters`，空词筛选需宿主声明 `source_filter_scope_header_v1` 和 `source_filter_search_without_query_v1`。上游无独立首页、源设置或登录入口。

本机取得 [官网首页](https://www.177pica.com/)、[第二页](https://www.177pica.com/page/2/) 和 REST 最新文章、指定文章及其排行侧栏。此前同一来源有 403 记录，现场接入以实际网络和宿主验证为准。

当前 WASM 经宿主 runner 读取了站点实际热门、最近更新的前两页、分类和带分类词搜。样本文章 `8826672` 返回完整详情、单章及 70 项 HTTPS 页表。

237 验过热门、文字搜索、作品链接、详情和单章首图。
