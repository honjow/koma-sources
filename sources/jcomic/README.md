# JComic

独立包 `com.jcomic.koma`。对照 [Keiyoushi 固定模块](https://github.com/keiyoushi/extensions-source/tree/8947c372cd87f546a0b80cb0e4b0a5a8add5ef88/src/zh/jcomic) v4 及网站真实响应。Mihon 来源 ID 为 `7044663852863780949`。上游作品以 `/page/<标题>` 或 `/eps/<标题>` 存档，章节用 `/page/<标题>[/序号]`。现行宿主的 Mihon 路径变量只接收 ASCII 普通标识，无法覆盖这些 Unicode 与转义路径，包暂不声明自动导入。Koma 目录和阅读直接保留各自的站内路径身份。

| 上游行为 | Koma 入口与实现 |
| --- | --- |
| 随机、最近更新 | `get_listings` 和 `get_manga_list` 分别读取 `/cat/隨機/{页}` 和 `/cat/最近更新/{页}`。`list` 从 `args.page.cursor` 读取页号，当前分页项后仍有条目才继续。无独立首页。 |
| 标题、作者、分类 | `get_filters` 提供默认、作者及固定分类。带关键词的 `search` 访问 `/search` 或 `/author`，忽略分类。空关键词按上游访问 `/cat`，首选全彩，依赖宿主的空关键词筛选入口。 |
| 列表和详情 | `work_card` 提取标题、页数、作者、分类和封面。`get_manga` 把 `/page` 详情请求映射到 `/eps` 并保留原站外链。真实单章的集数页无作品正文时，回读公开单章页的标题与作者。详情中的整书时间经 `chapterContext` 传给单章目录，独立目录调用缺少单章时间时保留未知。 |
| 单章和连载 | `get_chapters` 直接返回单章条目；连载读取第二列的章节链接并倒序，与上游一致。SDK 显式输出阅读顺序，不从标题猜话数。 |
| 页表和图像 | `get_pages` 按 `.comic-thumb` 顺序读取原网址，兼容固定上游的 `JCOMIC_TRAP_` 和现场页面脚本的 `JCOMIC_TRAPS_`。图片 URL 由网站签发，有效期约一分钟。宿主当前仅允许将原图 403 映射为访问被拒绝，尚不能复现 Mihon 的专用失效提示或自动刷新已缓存页表。 |
| 会话 | 文档与可归属到原站的图片沿用公开客户端 Cookie 标记。固定上游没有正式账号、凭据设置或付费解锁。 |

代码许可见 `LICENSE` 和 `NOTICE`，素材来源见 `ASSET_SOURCES.md`。

主机运行器直连验证了最新前两页、随机、作者搜索、分类、连载和单章。连载样本列出 4 章和 32 页，单章样本列出 51 页。两次首图请求均收到 CDN 的 Cloudflare 403 页面，正文实际出图及离线下载待验。
