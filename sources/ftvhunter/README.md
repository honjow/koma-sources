# FTV Hunter

FTV Hunter 集中收录 FTV Girls 模特写真，包括 Lara 的 Shy First Timer 和 Cecee 的 Kitchen Duty。图集按模特整理，视频条目不收录。

Koma ID `com.ftvhunter.koma`，language=all，version0.1.0，API0.2，nsfw=true/contentRating=nsfw，minAppVersion1.5.5。Mihon sourceId `2424130356866336342`；标准键 `ftv hunter/all/1`，MD5前8字节大端清除符号位。versionCode与source versionId分开，前者不参与ID哈希。work/chapter均保留以/开头的原站图集路径，单套单章；samePageOrder=true。

## 移植前的源码对照

固定[Keiyoushi 8d597e1bdbb0d9d961d941128703eaeb4bc8d1e3](https://github.com/keiyoushi/extensions-source/tree/8d597e1bdbb0d9d961d941128703eaeb4bc8d1e3)，实际读取src/all/ftvhunter/build.gradle.kts。源码证据存dist/wave-33/evidence/upstream。name=`FTV Hunter`、lang=all、baseUrl=`https://www.ftvhunter.com`与本地身份一致。

该站theme=masonry，Gradle生成入口，无独立站点.kt；实际逐行阅读Masonry.kt与Filters.kt。仅新增参数化lib.rs，通过未修改的masonry_hunter.rs复用；不改变Wave31/32源码、已有详情描述或编译结果。

| 上游行为 | Koma调用→实际消费 |
| --- | --- |
| 热门/最新gridapi content=updates、sort=trending/newest、mpage；原始figure>=20续页 | get_listings/get_manga_list→browse(args.listingId,args.page.cursor)；排除.icon-play视频，不用过滤后的卡片数量截断分页。 |
| 关键词/search/post/<query>/mpage/n/，忽略筛选 | search→browse query.trim；以两套原站套图名分别验证可搜到。 |
| 空词GridFilter字段序content=pix/quality/label/ord；多选标签+连接 | get_filters→filters与search→browse；order=0/popular/newest、quality=0/hd；标签来自/sort/newest/#filter-a section[data-slot=tags] label中的.txt/input。上游“关键词忽略筛选”说明在此保留。 |
| header h1、p.link-btn作者/标签、#content>p | get_manga→detail；明确继承原#content>p可能包含标签文本的行为，不暗改旧包。 |
| 图集单章，原图.list-gallery a[href^=https://cdn.] | get_chapters→GroupedChapters完整pageCount；get_pages→gallery，作品/章节归属一致。保留全部原图DOM序，不混入封面/UI/相关推荐。混合帖仅取图锚点。 |
| 图像公开请求 | get_image_request→本站与精确CDN host的HTTPS图片，附公开Referer；无/有Referer各做真实curl对照。 |
| 无登录/设置/独立首页/评论 | settings=false且get_settings空schema，无首页或评论能力，不把热榜包装首页。 |

## 本地化、验证与边界

零参SOURCE_INFO.description固定英语，metadata/presentation同源投影；五语UI通过合法hostFeatures/environment→Context/Catalog显式消费，不根据站点内容语言猜UI。章节使用宿主typed目录契约，language=all，pageCount是真实完整图数。

最终包内WASM运行info、五语UI、热门/最新差异、分页、筛选、关键词搜索、两套作品详情/单章/完整页表及get_image_request；每套至少2张正文原图以无代理curl带/不带Referer对照，实际Pillow解码并保留字节/像素SHA256，与独立HTML逐项比对。结果在dist/wave-33/verification.json与scratch/wave33_sol_report.md。无设备操作，不提交推送；既有所有源、Masonry、eighteenmanhua/miwumh与sources/README.md均保持起始哈希。
