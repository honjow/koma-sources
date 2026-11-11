# Tapas

`en.tapastic` v1.6.6 的独立 Rust/WASM 移植，固定到 Keiyoushi `fc7a827769f612dfa6dccb51e819f5b4eecaeab5`。参考是 Apache-2.0 并使用公开英文 Tapas 漫画目录。官方 origin 是 tapas.io。参考是 MIXED 且并非每部作品都有年龄分级。

| Koma 操作 | 参考行为 |
|---|---|
| `get_listings`、`get_manga_list` | 热门（排行）和最新（类型、最新集）使用同一 COMIC 分类、子标签、顺序、25 条和零基站点页索引。分页使用官方 `last` 字段。 |
| `search` | 原漫画搜索 HTML、美术、简介和分页。精确 Tapas 系列链接解析数字 ID 或页面元数据中发布的系列 ID。 |
| `get_manga` | 原系列信息标题、封面、简介、出版者署名、作者、类型和排期状态。 |
| `get_chapters` | 抓取完整最新优先集列表带其原分页和每操作一个时间戳。保留原场景编号、发布时间、付费墙和排期状态。基于设置的排除使用既有 Koma 章节合同。 |
| `get_pages`、`get_image_request` | 精确集所有权和原页顺序，包括原重复位置。保留原图像 Referer 和 UA。可选作者注释使用既有本地文字页渲染器。确认的锁保持付费状态；小说或缺失正文有单独结果。 |
| `get_settings` | 四个参考选项保留其有效默认：显示付费墙集、显示排期集，以及显示作者注释开、前缀编号关。集显示标题改变时其 ID 保持稳定。 |
| `get_authentication` | 真实站点的 `/account/signin` 在 Koma 的显式源 web 会话中打开。扩展的硬编码生日不使用。公开请求保持匿名且账号和年龄检查仍归 Tapas。这个源不购买或解锁集。 |

Koma 1.2.0 提供时钟、逐源 web 认证、本地文字页和章节排除合同。文字模板使用带参考标题的纯 Unicode 注释。账号的已解锁内容取决于 Tapas 授权。这个检出使用真实匿名免费和锁定页；未提交个人登录。
