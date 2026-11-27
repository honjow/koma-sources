# AralosBD

法语原创与翻译漫画目录。提供热门、最新上架、题材标签、搜索、公开目录和原尺寸正文图。最低 Koma 版本为 1.5.5。

Wave 8 首选 BDplus 的首页和目录语料在匿名请求时均返回 Cloudflare 403，按指定顺序采用了 AralosBD。上游固定为 [Keiyoushi AralosBD](https://github.com/keiyoushi/extensions-source/tree/c09b174459c037c62292e4e7a929a0cd59430bc4/src/fr/aralosbd)，宿主契约见 [Koma 运行 ABI](https://github.com/honjow/Koma/blob/master/docs/source-runtime-abi.md)。

| 行为 | 上游及原站 | Koma 路径 |
| --- | --- | --- |
| 热门及最新 | `AralosBD.kt` 的 `/manga/search` 分别按 `allviews` 和 `id` 倒序，过滤站点 ID 3。 | `get_manga_list` → `catalog`。由 `args.page.cursor` 转站点零基页号，使用 `page_count` 停止。最新指新上架作品。 |
| 题材及搜索 | 扩展搜索用 `text:`。官网高级搜索把经过验证的标签作为独立条件。 | `get_filters` 提供原站标签选项，`catalog` 用同一站点查询串组合条件，URL 编码前拒绝站点查询分隔符。 |
| 详情及目录 | `/manga/api?get=manga` 和 `get=chapters`。只采用 `chapter_released=1`。 | `detail` 和 typed `run_get_chapters` 使用数字 ID 和网站目录原序。章号 `2.5.1` 等文字保留在标题。简介解码实体，保留宿主能渲染的原站 Markdown。 |
| 阅读 | `get=pages&chapter=` 的 `links` 提供原尺寸图。 | `get_pages` → `pages`，按原数组顺序发 HTTPS 地址。 |
| 首页、设置与登录 | 扩展没有独立首页、源设置或登录流程。 | 公共匿名请求及既有错误合同。 |

五语覆盖包展示资料、栏目、源自编筛选标签及用户可见错误。
