# NakedPics

NakedPics 通过模特、摄影品牌与主题目录串联写真图集，包括 All Over 30 的 Isadora 与 Femjoy 的 Laria F。每套图集保留原站全部正文照片，不混入推荐缩略图或视频。

Koma ID `com.nakedpics.koma`，language=all，version0.1.0，API0.2，nsfw=true/contentRating=nsfw，minAppVersion1.5.5。Mihon sourceId `6711216111994206341`；标准键 `nakedpics/all/1`，MD5前8字节大端清除符号位。versionCode与source versionId分开，前者不参与ID哈希。work/chapter均保留以/开头的原站图集路径，单套单章；samePageOrder=true。

## 移植前的源码对照

固定[Keiyoushi 8d597e1bdbb0d9d961d941128703eaeb4bc8d1e3](https://github.com/keiyoushi/extensions-source/tree/8d597e1bdbb0d9d961d941128703eaeb4bc8d1e3)，实际读取src/all/nakedpics/build.gradle.kts。源码证据存dist/wave-33/evidence/upstream。name=`NakedPics`、lang=all、baseUrl=`https://hotnakedwomen.com`与本地身份一致。

实际逐行阅读NakedPics.kt、Dto.kt、Filters.kt，并对照SDK标准导出、port_support HTTP/Node、source_protocol HTTP门禁、Catalog/Context和GroupedChapters。为非Masonry独立协议，不修改任何已有共享文件。

| 上游行为/默认值/停止条件 | Koma调用→本源消费及适配 |
| --- | --- |
| getPopularManga /popular/；getLatestUpdates /recent/ | get_listings/get_manga_list→browse，首屏HTML [itemprop=associatedMedia]>a[href*=/gals/]；按原始30卡决定是否续页，作品路径去重。 |
| page>1：pageUrl?cat=<CID>&limit=30&offset=(page-1)*30&lang=en，XHR | args.page.cursor数字→page→fetch_listing/json_cards；每次操作重取首屏解析CID和宿主finalUrl，替代上游跨调用缓存，语义不变但多一次GET。Cookie/会话不缓存。 |
| search先关键词（空白合并为-）；其次分类；否则热门/最新；Sort默认Most popular | search→browse；query优先；category按首页真实单层路径，newest加入/newest/；query+newest使用解析后的路径/newest/以及offset0 API，保留重定向丢失newest的上游修复。 |
| 无结果搜索HTTP404虽返回popular HTML，仍为空列表 | fetch_listing按SDK严格HTTP/transport门禁，只有搜索not_found映射items=[]/hasMore=false，其他404保留错误，不伪装成功列表。 |
| GalleryDto g_url/desc/t_url_460/gid，外站partner=null | json_cards保留本站/gals/规范路径，desc只去精确gid后缀，跳过外站合作链接。图片扩展限定JPEG/PNG/WebP；不把AVIF封面发给宿主。 |
| 首页分类动态抓取，Sort Most popular/Newest | get_filters→filters；首页associatedMedia直接a中的span及单层path；默认Any空值。静态字段五语，动态站方分类保持原文。 |
| h1[itemprop=name]、ImageGallery模型/频道/标签图标 | get_manga→detail；authors为model，tags为主题，description按宿主五语显示照片数、模特、摄影品牌；真实站方名字不翻译。封面取正文首图img[itemprop]真实地址，避免lazy src占位。 |
| 单Gallery章、PAGE_SELECTOR=[itemprop=associatedMedia]>a[data-pswp-width] | get_chapters/get_pages→gallery，同一图集路径及完整pageCount，保留全部原图顺序，不插入推荐图/视频。无图返回chapter_unavailable。 |
| rateLimit 3/site；无认证/设置 | metadata requestLimits=3/1000ms仅网站域；get_settings空schema，settings=false。get_image_request仅本站/精确cdni.hotnakedwomen.com HTTPS JPEG/PNG/WebP，公开Referer；不读取响应头，不受headers String/Array差异影响。 |
| getMangasByUrl多种站方列表路径与fetchRelatedMangaList | 当前SDK无这两个独立宿主入口，不伪造首页承载。保留规范/gals/<slug>/ work/chapter导入、source URL；相关图不混入阅读页。非规范带语言前缀的外部深链不声明为本源章节身份。 |

图片URL与HTML均现场核准。首次无代理curl有一次TLS35，HTTP1.1独立复测成功，原始失败证据保留；最终包再通过宿主请求及多次curl读图。未关闭证书校验、绕过挑战或传登录凭证。

## 本地化、验证与边界

零参SOURCE_INFO.description固定英语，metadata/presentation同源投影；五语UI通过合法hostFeatures/environment→Context/Catalog显式消费，不根据站点内容语言猜UI。章节使用宿主typed目录契约，language=all，pageCount是真实完整图数。

最终包内WASM运行info、五语UI、热门/最新差异、分页、筛选、关键词搜索、两套作品详情/单章/完整页表及get_image_request；每套至少2张正文原图以无代理curl带/不带Referer对照，实际Pillow解码并保留字节/像素SHA256，与独立HTML逐项比对。结果在dist/wave-33/verification.json与scratch/wave33_sol_report.md。无设备操作，不提交推送；既有所有源、Masonry、eighteenmanhua/miwumh与sources/README.md均保持起始哈希。
