# まんがタイムスクエア

芳文社官方漫画站，汇集《まんがタイム》《まんがホーム》《まんがタイムオリジナル》三刊连载，并设 Extra 栏目。可阅读《夜の缶詰》等作品在原站免费公开的章节。

源ID com.mangatimesquare.ja.koma，语言ja，safe，0.1.0，最低Koma 1.5.5。官网页脚 © HOUBUNSHA CO., LTD 已独立核实；出版社为芳文社。官网、原始GraphQL、免费viewer及图片请求留证于dist/wave-20/evidence。

## 移植前的源码对照

固定上游 [Keiyoushi c09b174459c037c62292e4e7a929a0cd59430bc4](https://github.com/keiyoushi/extensions-source/tree/c09b174459c037c62292e4e7a929a0cd59430bc4/src/ja/mangatimesquare)：实际读取 MangaTimeSquare.kt、build.gradle.kts，同版 GigaViewer.kt、Filters.kt、Queries.kt、Dto.kt、ImageInterceptor.kt 与 ExtensionPlugin.kt。build声明精确name=Manga Time Square、lang=ja、baseUrl=https://mangatime-square.com、versionId默认1；versionCode不参与身份计算。宿主调用链只读对照sdk/src/source_protocol.rs、runtime.rs、chapters.rs及tools/koma-source-dev/src/host.rs，不访问Koma仓库或设备。

| 行为 | 固定上游与宿主消费位置 |
| --- | --- |
| 默认合集 | seriesListIds=12207421983452274168,12207421983452274175,12207421983452274180；popular按likeCount降序，latest按latestEpisode.publishedAt降序 → get_manga_list → listing → browse::collection。Extra仅作为独立筛选，不扩大默认合集。 |
| 五项分类 | 作品一覧=前三组；まんがタイム=12207421983452274168；まんがホーム=12207421983452274175；まんがタイムオリジナル=12207421983452274180；Extra=12207421983452274185。get_filters → browse::filters → Catalog五语，稳定value保存在args.filters.collection。 |
| 搜索与分页 | 空关键词按选中合集的原站顺序；非空关键词依上游忽略分类 → GraphQL searchSeries，纯数字series ID与首页保持一致。上游无网络分页；宿主args.page.cursor按80项切片并验证锚点，末页null。 |
| 详情 | get_manga → Site::detail → series(databaseId)，作品前缀mangatimesquare:。作者、介绍、封面与首话原站链接采用共享解析。 |
| 章节与权限 | typed get_chapters → Site::chapters → pagination_readable_products；话和卷均保留，稳定ID mangatimesquare-ch:<series>:episode:<product>，volume对应类型。只按标准purchase字段；不启用Zenon缺失status策略。hide_locked/hide_unavailable默认false；viewer独立复核阅读权限，锁话chapter_locked。 |
| 阅读与图片 | get_pages → episode-json readableProduct，只取main页；已核实baku/rtl。正文CDN=cdn-img.mangatime-square.com，封面实际为cdn-img.mangatime-square.com，站方静态资源cdn.mangatime-square.com不混为正文。get_image_request附Referer，声明wasm_image_v1并执行随包koma_gigaviewer_tile_processor；缺能力host_feature_required。 |
| 本地化与边界 | 五语listing/filter/setting/chapter/error；固定日语source_info与同目录presentation投影。没有独立首页、账号或评论能力。不变更任何共享文件、处理器或旧源。 |

## Mihon身份与验收

精确计算键 `manga time square/ja/1`：UTF-8 MD5前8字节大端序与Long.MAX_VALUE取与，sourceId **5972758064357912813**。identity.mihon.work将当前裸series ID加mangatimesquare:；章节targetIdSegment=3匹配readable_product_id，保留实际page顺序；不把显示名或versionCode用于计算。

匿名样本《夜の缶詰》series=12207421984289785793，首话https://mangatime-square.com/episode/12207421984289830557，10页，初探原图1350×1938。最终验证从dist/wave-20/sources/mangatimesquare-ja/mangatimesquare-ja-0.1.0.koma提取source.wasm，使用真实dev runner重测五语、热门/最新、全部5项分类、搜索、详情、章节、pages和image-request；下载前3页并实际执行包内瓦片WASM，与独立转置参考逐像素比较。结构化HTTP/MIME/尺寸/SHA-256与保护基线记录见dist/wave-20/verification.json。包保留供真机QA；不提交、推送或操作设备。
