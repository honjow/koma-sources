# COMIC OGYAAA!!

集英社集团旗下ホーム社（Home-sha）的官方漫画站，刊载《邦画プレゼン女子高生 邦キチ！ 映子さん》等作品，既有搞笑和日常，也有奇幻、恋爱喜剧与悬疑。可阅读原站免费公开的章节。

源ID `com.comicogyaaa.ja.koma`，日语 `ja`，safe，版本0.1.0，最低Koma 1.5.5。源码与素材仅新增在本目录；既有书源、tools、image-processors及eighteenmanhua本地改动保持不变。

## 移植前的源码对照

固定上游 [Keiyoushi c09b174459c037c62292e4e7a929a0cd59430bc4](https://github.com/keiyoushi/extensions-source/tree/c09b174459c037c62292e4e7a929a0cd59430bc4/src/ja/comicogyaaa) 的本站Kotlin入口和build.gradle.kts；同版GigaViewer.kt、Filters.kt、Queries.kt、Dto.kt、ImageInterceptor.kt，以及ExtensionPlugin.kt的实际ID算法。实际读取官网、GraphQL全部分类节点、目录API和公开阅读器后实施。宿主调用链只读核对本仓SKILL.md、sdk/src/runtime.rs、source_protocol.rs、chapters.rs及tools/koma-source-dev/src/host.rs；不访问Koma仓库，不操作设备。

| 行为 | 上游合同、默认值与停止条件 | Koma调用与本源消费 |
| --- | --- | --- |
| 热门／最新 | getPopularManga/getLatestUpdates → fetchSeriesList(seriesListIds)，节点合并去重，点赞／latestEpisode.publishedAt降序；无点赞保留节点原序。 | get_manga_list → listing → browse::collection；不是官网按日访问量榜单。仅热门、最新两个栏目，无独立首页。 |
| 分类 | getFilterOptions → CollectionFilter，SerialGroup／Genre全量请求；非空词搜索忽略分类。 | get_filters → browse::filters，通用标签filter.collection、选项collection.*均完整五语。空词search和热门／最新消费args.filters.collection；真实参数与ID如下表。 |
| 分页 | 上游一次读取合集，无服务器分页；关键词单批结果。 | browse::page按80部输出，消费args.page.cursor中的offset与前一部ID锚点；目录变化报catalog_changed，终点hasMore=false；不截断原站全集。 |
| 搜索 | POST /graphql，searchSeries(keyword)，有词时不应用合集筛选。 | search → browse::search → Site::search，args.query；有词且传续页cursor时报invalid_request，避免重复返回。 |
| 详情 | series(databaseId)，标题、作者、简介、首话或首卷链接，封面优先thumbnailUri再latestVolume.thumbnailUri。 | get_manga → Site::detail。作品 `comicogyaaa:<seriesId>`；话 `comicogyaaa-ch:<seriesId>:episode:<productId>`，卷为volume；页 `comicogyaaa-page:…`。 |
| 目录 | episodes/volumes readableProducts.totalCount；pagination_readable_products按type、aggregate_id、sort_order=desc、offset逐批抓取。 | typed get_chapters → Site::chapters，话与卷分组；核对总量、前页锚点、重复及变化，保留阅读位置。cursor从args.page.cursor读取，终点完整返回。 |
| 公开、付费、下架 | Dto.PurchaseInfo：can_read=true且未unavailable为公开；!can_read且!unavailable为付费；unavailable为下架。本站无isUnavailable覆盖。 | 使用默认chapters，绝不启用Zenon缺失status专属策略。lock_unavailable_titles=true只维持宿主锁标，不把缺失status判为下架；viewer仍须isPublic=true，锁定时chapter_locked，不拿预览页冒充正文。 |
| 隐藏设置 | hide_locked、hide_unavailable均默认false，各自只隐藏付费／下架；不改变授权。 | get_settings → browse::settings；settings中的布尔值，经source_chapter_exclusions_v1协商后按真实类别输出排除ID。无能力时保留目录手动阅读边界。 |
| 正文、方向 | script#episode-json的data-value，只提取pageStructure.pages中type=main；保留readingDirection；choJuGiga=baku做4×4块转置。 | get_pages → Site::pages；样本真实方向rtl，11页，块大小floor(width/32)*8、floor(height/32)*8，边缘余数不动。 |
| 图片与还原 | 原图由本站CDN提供；ImageInterceptor对baku执行像素块转置。 | get_image_request → Site::image_request；正文只允许cdn-img.comic-ogyaaa.com/public/page/，声明wasm_image_v1与mode=baku；缺少source_required_image_decode_wasm_v1能力明确报错。随包处理器为既有koma_gigaviewer_tile_processor，不新建或修改。封面不拼块。 |
| 请求、登录 | 普通匿名GraphQL、目录和viewer访问，无本站账号专属扩展。 | 不声明credentials或任何登录／付费绕过，图片带本站Referer。未读取HTTP响应头，因此不存在把Value::Array误当String的分支。 |

### 固定节点与现场核对

| 筛选键 | 官网／上游分类 | 精确节点ID | 现场节点条数（去重前） |
| --- | --- | --- | --- |
| all | 作品一覧 | 2550912965179775890, 2550912965179775891 | 53 + 31 |
| daily | 日常系 | Genre:2550912965179776778 | 34 |
| fantasy | ファンタジー | Genre:2550912965179776774 | 17 |
| romcom | ラブコメ | Genre:2550912965179776763 | 13 |
| comedy | ギャグ | Genre:2550912965179776769 | 22 |
| drama | ヒューマンドラマ | Genre:2550912965179776783 | 30 |
| action | アクション | Genre:2550912965179776779 | 5 |
| horror_suspense | ホラー・サスペンス | Genre:2550912965179776784 | 7 |

连载中与连载结束节点按上游合并去重，原站空集合也保留其身份。 封面已核对为cdn-img.comic-ogyaaa.com；共享实现允许标准cdn-scissors.gigaviewer.com包装，正文主机严格限定为cdn-img.comic-ogyaaa.com。官网部分模板的 `/series` 路由不存在（404），不是访问阻断；本源按上游真实GraphQL节点读取，完整目录和正文均独立实测。

### Mihon身份

上游真实name=`Comic Ogyaaa`，lang=`ja`，versionId默认1；ExtensionPlugin.kt计算键精确为 `comic ogyaaa/ja/1`。MD5前8字节大端序与Long.MAX_VALUE取与，十进制sourceId **`3754263546482783493`**。不使用Koma显示名计算。identity.mihon.work对当前上游SManga.url的裸databaseId加 `comicogyaaa:`；SChapter.url是裸readable_product_id，chapter.targetIdSegment=3匹配本源稳定末段。未声明未经核对的samePageOrder；旧版Mihon遗留episode路径作品ID不在这份裸ID映射声明内。

### 本地化与素材

简中、繁中、英、日、韩覆盖名称、差异化简介、栏目、分类、设置、章类型、下架及错误；专有刊名按官网保留或加清晰译名。localizedNames与presentation投影同一文案目录；固定source_info简介采用日语基准，不受请求语言改变。图标原件、恢复工艺、SHA-256和包内审图链见 [ASSET_SOURCES.md](ASSET_SOURCES.md)。

## 验证与交付边界

初探代表作《邦画プレゼン女子高生　邦キチ！ 映子さん》，首个公开话 https://comic-ogyaaa.com/episode/3269754496829572092，11页、baku、rtl。最终候选 `dist/wave-18/sources/comicogyaaa-ja/comicogyaaa-ja-0.1.0.koma`；从实际包内提取source.wasm，用target/release/koma-source-dev重新实测info、五语、热门／最新、每个分类、关键词、详情、完整目录和hide_locked／hide_unavailable，再下载前3张正文原图，实际调用随包处理器并与独立转置参考逐像素比较。记录HTTP、MIME、尺寸、SHA及还原前后像素证据于dist/wave-18/verification.json和evidence。

本轮为真机前主机实网预检；未运行App书源列表或普通真机阅读路径，不以主机结果替代后续QA。未创建分支／工作树，未提交或推送。

现场默认合集去重后为82部，全部筛选节点并集为83部。《恋ってなんで変と似てるの》仅出现在独立分类，不在固定上游默认seriesListIds中；可通过对应分类找到，未擅自并入默认热门／最新。
