# NamiComi（阿拉伯语）

NamiComi 阿拉伯语分区，收录《⁨نترو⁩》等创作者与发行方发布的阿拉伯语章节。仅提供原站匿名可读内容。

源ID `com.namicomi.ar.koma`，语言 `ar`，版本0.1.0，API0.2，safe，最低Koma1.5.5。homepage：https://namicomi.com/ar。沿用已验收波兰语、韩语兄弟源的完整lib/ui/comments架构；不修改旧源、SDK、脚本或工具，不创建工作树、分支，不提交推送，不操作设备。

## 移植前的源码对照

固定 [Keiyoushi c09b174459c037c62292e4e7a929a0cd59430bc4](https://github.com/keiyoushi/extensions-source/tree/c09b174459c037c62292e4e7a929a0cd59430bc4/src/all/namicomi)。已实际读取NamiComi.kt、Filters.kt、build.gradle.kts、ExtensionPlugin.kt；上游name=NamiComi、baseUrl=https://namicomi.com，lang=ar，默认versionId=1。extLang映射对本语种无特殊分支，API与宿主章节语言均为ar。宿主链只读对照tools/koma-source-dev/src/host.rs及sdk/src/runtime.rs、source_protocol.rs、chapters.rs。

| 行为 | 上游 → Koma调用与本源消费 |
| --- | --- |
| 列表和搜索 | getPopularManga/getLatestUpdates/getSearchMangaList → get_manga_list/search → lib::list。热门按views降序，最新及关键词默认publishedAt降序；availableTranslatedLanguages[]=ar，20项/页，args.page.cursor保留offset并核对total/offset/limit。浏览可读性默认all，搜索默认available，两个scope独立。 |
| 筛选 | Filters.kt → get_filters → filter_schema/apply_filters；可读性、评级、状态、16项有效排序、包含/排除标签及and/or。需scope header协商；fetchFilterData及动态/三态能力协商后请求/title/tags。沿用已验收契约，不恢复原API不接受的title排序。本轮实际验证views升序与available条件，并独立对照API顺序。 |
| 详情与回退 | get_manga → detail → /title/<id>。标题、简介按ar → en → 首个站方值回退；公开链接/ar/title/，作品前缀namicomi:ar:。不自行翻译站方标题。 |
| 目录与分组 | typed get_chapters → chapters，/chapter?titleId=...&translatedLanguages[]=ar；卷章双降序，每批200，核对总量、作品、语言及组织，保留小数章/无编号/同编号多版本。按组织或上传者建立稳定readingGroup，不按标题推断页序。Chapter.language=Some(b"ar")。 |
| 权限 | gating/check只采信匿名逐章布尔权限；pages再次复核同章授权，不移植上游网页登录令牌、付费或账号回退。默认隐藏不可读章节；仅目录排除能力允许时按showLockedChapters_ar显示锁话，不改变访问权限。 |
| 阅读与图片 | get_pages → /images/chapter/<id>?newQualities=true，source/low数组filename构造官方uploads.namicomi.com URL，image_request复核路径并附Referer/Origin。原始页数组顺序完整保留，不因界面语言改序；不混入UI或相关推荐。 |
| 设置 | thumbnailQuality_ar=original，可选.512.jpg/.256.jpg；dataSaver_ar=false；showLockedChapters_ar=false。仅作用于本源身份，版本化请求从settings读取。 |
| 评论 | get_comments → comments::load，只接受同作品同章，10条/页；核对响应offset、作者所属章节，保留剧透/已删过滤和原站nextCursor。不写入任何站方数据。 |
| 本地化 | ui::remember/error与Catalog依hostFeatures+environment读取界面语言；五语listing/filter/setting/error和作者后记提示完整继承。零参source_info使用固定英文基准，metadata source.description与presentation.texts.description完全同源。 |

## 身份与双向文本

独立作品、章、页前缀为 `namicomi:ar:` / `namicomich:ar:` / `namicomipage:ar:`。Mihon实际计算键 `namicomi/ar/1`：UTF-8 MD5前8字节按大端序累积，与Long.MAX_VALUE取与，sourceId为 **`7830959603474646022`**。work规则将上游裸ID加本源前缀，chapter稳定末段用targetIdSegment=3匹配。不使用versionCode或内容显示名替代真实name/lang/versionId。

阿拉伯语标题保持站方原始码点，搜索词、详情title、作品/章节/页ID及页表不加方向控制符、不逆序。仅在源自编的LTR简介中，带阿拉伯字母的代表作名以U+2068（FSI）/U+2069（PDI）成对隔离；Rust字面量使用显式Unicode转义，避免隐形方向控制符混入源代码。俄语、印尼语以及英文标题不额外隔离。显示方向由宿主界面决定，不从语言猜漫画阅读方向。

## 本语言实网核对

独立抓取首页、热门可读条件、完整样本目录、匿名gating及图片。availableTranslatedLanguages[]=ar&hasAvailableChapters=true返回11部作品；样本《نترو》，作品 `Mftiwm44`，章节 `3jQYqQU9`，API完整目录33章，样本38页。本样本所有原始章节匿名授权均为true；前3张图HTTP200并完整解码，实际尺寸[[1200, 745], [1200, 744], [1200, 745]]。这是真实站方标题，不沿用其他语种代表作。

初探证据：dist/wave-21/evidence/ar/preflight.json及preflight-receipts.json。最终验收从dist/wave-21/sources/namicomi-ar/namicomi-ar-0.1.0.koma提取实际source.wasm，执行info、五语、popular/latest/filter/search/detail/chapters/pages/image-request；下载前3张原图，核对MIME、像素尺寸、文件头及SHA-256，并对照匿名完整目录与页表。报告dist/wave-21/verification.json；包及.wasm保留供后续真机QA。
