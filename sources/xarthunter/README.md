# XArt Hunter

XArt Hunter 收录 X-Art 品牌写真，包括 Nikki Hill 的 Darling Nikki 与 Isabella 的 Sweet Memories。以独立照片图集呈现，排除原站视频条目。

Koma ID `com.xarthunter.koma`，language=`all`，API0.2，version0.1.0，contentRating/nsfw=`nsfw`/true，minAppVersion1.5.5。Mihon sourceId `7425515716363969566`；标准键 `xart hunter/all/1`，MD5前8字节大端清除符号位，与固定上游及用户指定身份核准。Work和chapter使用站内单层图集原始路径，页序一致，samePageOrder=true。

## 移植前的源码对照

固定[Keiyoushi 8d597e1bdbb0d9d961d941128703eaeb4bc8d1e3](https://github.com/keiyoushi/extensions-source/tree/8d597e1bdbb0d9d961d941128703eaeb4bc8d1e3)：实际读取src/all/xarthunter/build.gradle.kts及Masonry.kt/Filters.kt。该站theme=masonry，Gradle生成入口（没有独立站点.kt）；name=`XArt Hunter`、lang=all、baseUrl=`https://www.xarthunter.com`。源码证据存dist/wave-32/evidence/upstream。

| 行为 | 固定上游 | Koma调用和源内消费 |
| --- | --- | --- |
| 热门/最新/分页 | getGridPage(content=updates,sort=trending/newest,mpage)，原始figure>=20判下一页 | get_listings/get_manga_list→browse→args.listingId/args.page.cursor；过滤.icon-play但按过滤前数量续页，不把热门伪装首页。 |
| 搜索 | 非空query优先，路径/search/post/<query>/mpage/<n>/；忽略筛选 | search→browse，query.trim；独立验证搜索命中选定样本。 |
| 筛选 | 无词时GridFilter序列化content=pix,quality,label,ord；tags用+；order=0/popular/newest；quality=0/hd | get_filters→filters与search→browse；动态标签来自/sort/newest/#filter-a section[data-slot=tags] label的.txt/input；静态标签和栏目五语、站方动态标签不擅自翻译。关键词忽略筛选的上游提示在此说明，不制造新的筛选行为。 |
| 详情 | header h1、p.link-btn中model/tag、#content>p | get_manga→detail；原实现#content>p可能含标签文本，本批明确原样沿用，不悄改已验Wave31行为。 |
| 章节/正文 | 图集即一个Gallery章；.list-gallery a[href^=https://cdn.]全部原图 | get_chapters→GroupedChapters单章/pageCount；get_pages→gallery，核对作品/章节同一路径。保留DOM顺序，不插封面/UI/推荐图；混合帖只取图片，视频卡片不入列表。 |
| 图片请求 | 站内图链接 | get_image_request→valid_url，仅HTTPS本站及精确`cdn.xarthunter.com`，公开Referer=`https://www.xarthunter.com`。无Referer与带Referer都做直连实测；不据局部成功断言整站无防盗链。 |
| 登录/设置/默认值 | 上游无认证/可配置设置 | get_settings返回空schema、settings=false；无登录/评论/独立首页，不新增宿主权限。 |
| 本地化/宿主链 | Koma已有Catalog与typed目录契约 | 标准导出宏→request envelope/args.page.cursor→port_support HTTP/Node→Context+Catalog按hostFeatures/environment选择五语；零参SOURCE_INFO.description固定英语，同metadata与presentation目录。 |

## 验证、复用与边界

仅src/lib.rs参数化调用未修改的sources/masonry_hunter.rs；SOURCE_INFO、BASE、IMAGE_HOST、METADATA和export各自独立，不改metarthunter/femjoyhunter与已有源。对包内最终WASM实测info、五语UI、热门/最新差异、分页/筛选/搜索、两套详情/章节/完整页表和图片请求。每套至少两张原图，以无代理curl对照Referer并用Pillow验证HTTP200、JPEG/PNG/WebP、字节、像素、SHA256；独立HTML原图列表逐项比对。结果与完整样本路径见dist/wave-32/verification.json及scratch/wave32_sol_report.md。

未操作设备、不提交推送；图标与App呈现边界见ASSET_SOURCES.md。eighteenmanhua/miwumh/sources/README.md与Wave31源码全部受保护。
