# Comic Fury（全部语言）

汇集 Comic Fury 各语言专区的创作者作品，包括《Golden Shrike》。保留原文与作者附言，不限于英文目录。

ID `com.comicfury.all.koma`，宿主language=all，0.1.0，API0.2，minAppVersion1.5.5，nsfw=true、contentRating=unknown。其他语言/无文字不是一个BCP47语种，沿仓内peppercarrot/internetarchive的all声明，列表与章节不伪造具体语言。build.sh原样投影info.language到manifest.lang，source-dev不改写分组；五语名称由metadata/presentation投影。

## 移植前的源码对照

固定[Keiyoushi `90b6e6a905ca186945b21ec0b0be4ca9135b8e69`](https://github.com/keiyoushi/extensions-source/tree/90b6e6a905ca186945b21ec0b0be4ca9135b8e69/src/all/comicfury)；实际读取ComicFury.kt、Filters.kt、ContentWarningInterceptor.kt、build.gradle.kts及ExtensionPlugin.kt。当前上游由Gradle语言清单和@Source生成入口，并无独立ComicFuryFactory.kt；本变体上游name=Comic Fury、lang=all，查询language=all；Mihon计算键comic fury/all/1，大小写按固定上游。

| 行为 | 上游 → Koma调用 → 本源消费 |
| --- | --- |
| 热门/最新/搜索 | getPopularManga/getLatestUpdates/getSearchMangaList → get_manga_list/search → comicfury_common.rs browse；search.php固定language=all，sort=1/2/0，args.page.cursor为1起页号；仅存在div.search-next-page时返回下一页。 |
| 筛选 | TagsFilter/SortFilter/LastUpdatedFilter/CompletedComicFilter/四项Flags → get_filters → labels与browse；tags去除逗号后空格；completed勾选0、未勾选1；lastupdate默认0，fv/fn/fl/fs默认2；排序可覆盖栏目默认值。静态选项全部五语。 |
| 详情 | getMangaDetails → get_manga → detail；保留/comicprofile.php?url=slug；归属read/slug校验、作者、标签与简介回退沿英文源。作品内容保持原文，输出language=all。 |
| 目录 | getChapterList/collect → get_chapters → archive/archive_group；先归档分组再网页分页，128页上限、循环与取消检查；统一归档为空才读取同slug.webcomic.ws/archive/comics。去重后反向输出，以原归档位置表示阅读顺序，宿主结果分页用末条章节ID作cursor，输出LANG_BYTES=all。 |
| 正文与图像 | getPageList → get_pages/get_image_request → pages；统一布局严格校验首个漫画块data-comicid与目标话一致，仅取该块原图，避免无限滚动混入下一话；自定义布局仅img#comicimage。图片只允许公开HTTPS comicfury.com/img.comicfury.com，返回Referer/Origin。无响应头读取，不依赖String/Array差异。 |
| 作者附言 | showAuthorsNotes默认false → get_settings/settings → notes_enabled；仅hostFeatures包含text_page_image_v1时显示设置并追加webtoon-author-notes文字页。保留原图页序、段落和作者归属；新源将附言标题及错误按宿主UI五语本地化。 |
| 访问与停止 | 上游ContentWarningInterceptor会POST确认表单；本源保留已验英文源边界，不代替读者提交确认，遇确认页返回chapter_locked。没有登录、评论、额外首页或自造筛选；解析异常/无正文/分页循环明确报错。 |
| 宿主调用链 | sdk/src/lib.rs exports/ResultBuffer、sdk/src/chapters.rs GroupedChapters、runtime.rs Context、localization.rs Catalog、port_support.rs Node/HTTP → 原生WASM ABI；本地化按environment及source_localization_v1协商，零参info保持固定英文基准。 |

Mihon sourceId `5182817189941556265`：ExtensionPlugin的`comic fury/all/1`经MD5前8字节大端并清除符号位，经固定上游声明核算。work.pathPrefix/idPrefix=`/comicprofile.php?url=`，chapter.pathPrefix/idPrefix=`/read/`，不声称samePageOrder；自定义布局/作者附言可能改变分页。

## 独立性与验证

本波通过EXPORT_NAME显式选择其他语言或无文字查询，宿主LANG/LANG_BYTES均为all；既有11源的输入参数不变且重编译比较WASM，既有独立en/zh不接入共享模块。固定SOURCE_INFO、LANG、LANG_BYTES、EXPORT_NAME、METADATA和导出字面量在本源极简lib.rs声明，业务逻辑无三份复制。设置键showAuthorsNotes沿上游，设置由宿主按sourceId隔离。

包、真实WASM输出、样本目录/页表/作者附言与直连图片HTTP/MIME/尺寸/SHA256见dist/wave-30/verification.json。图标原件与包内审图见ASSET_SOURCES.md。所有5语简介从同一目录投影到presentation，作品标题不翻造。未操作设备、未提交/推送。

## Wave 30 实时路由核对

explore.php实测404，未用它作为列表请求。search.php真实对照保存于language-matrix.json；all、空值、遗漏参数首屏相同，按上游保留显式language=all，不改成猜测的参数。other/notext与en首屏明显不同；all热门由英语主导时可能等同英文，另以波兰语作品Przygody Krysi搜索验证跨语种范围。Koma language=all与上游Mihon语言独立，身份映射不因UI分组变化。未添加翻译或重排章节。

本轮新映射只在新包默认feature `comicfury_variants`下编译；既有11源未声明或启用该feature，仍编译原site_lang实现，WASM逐字节回归一致。
