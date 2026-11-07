# 嗶哩漫畫

`net.bilimanga.koma` 对应 `https://www.bilimanga.net` 的「嗶哩漫畫」。参考 Keiyoushi
`16cde44564d45c3fa9c14371b4d9152606a9821d` 的 `zh/bilimanga` 和固定 Core，扩展
1.6.15，来源语言 `zh`。Mihon 网络参考固定到 `70c73c728b6350dd9f1447420e9cef54e8403b41`。

| 行为 | 上游与 Koma 适配 |
| --- | --- |
| 浏览与分页 | `BiliManga.getPopularManga`、`getLatestUpdates`、`mangaPageParse`。`get_manga_list` 的 `list` 沿用 11 个热门榜、最新和 10 个分类字段，从 `args.page.cursor` 读页数。榜单按 50 项，分类按真实当前页与尾页。 |
| 作品地址与游客搜索 | `getMangaByUrl` 对应搜索框的同站 `/detail/{id}.html`，与 `get_manga` 共用 `write_detail_fields`。`ensureSearchTicket` 对应 `keyword_search`，CSS/JS 并发，同次调用仅消费网站签发的三个匿名 Cookie，redeem 无缓存，跟随宿主核验过的同域结果。最终详情和普通结果各自解析。游客 Cookie 不写存储，不使用 WebView 或账号会话。原扩展可复用进程间隔内的 CSS/JS Cookie，本移植按单次搜索隔离保存，redeem 每次执行。 |
| 目录 | `fetchMangaUpdate` 对应 `chapters`。按原分卷、锚点、标题和顺序返回。不按网页标题的总话数补造目录。保留原生成的 `#prev`、`#next` 章节身份。 |
| 正文 | `ChapterInterceptor`、`getPageList` 对应 `pages`。片段仅是本地跳转指示，第一跳不增加 `night=1`。次跳由网页给出的同书导航解析为 `_2.html`。缺失或非同书导航明确失败，不猜章节号。页面位置、原始图片地址、重复页与附页均保留。已实测的桌面浏览提示保留具体原因。 |
| 封面与图片 | `KeiSource` 的 `Fetch.IMAGE` 对应 `write_cover` 和 `image_request`。同站封面按路径作品归属，正文按原章节／页身份和真实 HTTPS CDN 校验。实际 CDN 文件名是内容摘要，本地不虚构章节与摘要的关联。已实测 JPEG 和 AVIF。公开图片头不携游客、登录或夜间 Cookie。 官网 `c.motiezw.com` 的作品封面先校验路径作品归属，再使用官网同域的 `/files/cartoon/image/` 对应地址，保留尺寸后缀与时间戳。浏览、详情及已保存 CDN 地址共用此转换，正文仍沿用实际图片 CDN。 |
| 设置 | `POPULAR_MANGA_DISPLAY`、`RATE_LIMIT`、`DESCRIPTION`。热门默认周点击榜。请求速率默认 10/10，由 Koma 宿主共享限速并在进程重启后应用新设置。简介默认显示公告、别名、小说链接。速率在宿主读取时校验，源设置仍是文本字段。 |

浏览与阅读需要宿主 `source_rate_limit_bilimanga_v1`，游客关键词另需
`source_bilimanga_guest_search_v1` 和 `source_http_batch_v1`。这些能力缺失时会在发出网络前提示。
最低 Koma 版本为 1.2.1。每部作品的目录更新时间映射到该作的章节日期，按 UTC+08:00
午夜输出。缺失或无效日期省略字段。相关作品保留 Komikku 专用差异。

原网页与 WASM 已对照六个榜单／分类页面、173 部不同列表作品、50 部详情与目录、9754 个
目录位置、16 个正常章节的 413 个图片位置，以及真实游客搜索的结果页和详情跳转。素材出处见
[ASSET_SOURCES.md](ASSET_SOURCES.md)。设备验收和公开登记单独记录。
