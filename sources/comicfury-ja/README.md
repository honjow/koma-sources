# Comic Fury（日语 / 日本語）

Comic Fury 上由创作者自行发布的日语网漫，包括《ササフラス・ロード》。部分作品提供作者随原图发布的逐话附言。

源ID `com.comicfury.ja.koma`，语言`ja`，版本0.1.0，API0.2，最低Koma1.5.5。内容评级沿用已发布英文入口unknown，安装登记nsfw=true，保留四种内容尺度的上游默认值2，不把平台所有作品标为安全。

## 移植前的源码对照

固定[Keiyoushi `90b6e6a905ca186945b21ec0b0be4ca9135b8e69`](https://github.com/keiyoushi/extensions-source/tree/90b6e6a905ca186945b21ec0b0be4ca9135b8e69/src/all/comicfury)；实际读取ComicFury.kt、Filters.kt、ContentWarningInterceptor.kt、build.gradle.kts及ExtensionPlugin.kt。当前上游由Gradle语言清单和@Source生成入口，并无独立ComicFuryFactory.kt；ja使用相同siteLang，无pt-BR→pt或No Text特殊映射。

| 行为 | 上游 → Koma调用 → 本源消费 |
| --- | --- |
| 热门/最新/搜索 | getPopularManga/getLatestUpdates/getSearchMangaList → get_manga_list/search → comicfury_common.rs browse；search.php固定language=ja，sort=1/2/0，args.page.cursor为1起页号；仅存在div.search-next-page时返回下一页。 |
| 筛选 | TagsFilter/SortFilter/LastUpdatedFilter/CompletedComicFilter/四项Flags → get_filters → labels与browse；tags去除逗号后空格；completed勾选0、未勾选1；lastupdate默认0，fv/fn/fl/fs默认2；排序可覆盖栏目默认值。静态选项全部五语。 |
| 详情 | getMangaDetails → get_manga → detail；保留/comicprofile.php?url=slug；归属read/slug校验、作者、标签与简介回退沿英文源。作品内容保持原文，输出language=ja。 |
| 目录 | getChapterList/collect → get_chapters → archive/archive_group；先归档分组再网页分页，128页上限、循环与取消检查；统一归档为空才读取同slug.webcomic.ws/archive/comics。去重后反向输出，以原归档位置表示阅读顺序，宿主结果分页用末条章节ID作cursor，输出LANG_BYTES=ja。 |
| 正文与图像 | getPageList → get_pages/get_image_request → pages；统一布局严格校验首个漫画块data-comicid与目标话一致，仅取该块原图，避免无限滚动混入下一话；自定义布局仅img#comicimage。图片只允许公开HTTPS comicfury.com/img.comicfury.com，返回Referer/Origin。无响应头读取，不依赖String/Array差异。 |
| 作者附言 | showAuthorsNotes默认false → get_settings/settings → notes_enabled；仅hostFeatures包含text_page_image_v1时显示设置并追加webtoon-author-notes文字页。保留原图页序、段落和作者归属；新源将附言标题及错误按宿主UI五语本地化。 |
| 访问与停止 | 上游ContentWarningInterceptor会POST确认表单；本源保留已验英文源边界，不代替读者提交确认，遇确认页返回chapter_locked。没有登录、评论、额外首页或自造筛选；解析异常/无正文/分页循环明确报错。 |
| 宿主调用链 | sdk/src/lib.rs exports/ResultBuffer、sdk/src/chapters.rs GroupedChapters、runtime.rs Context、localization.rs Catalog、port_support.rs Node/HTTP → 原生WASM ABI；本地化按environment及source_localization_v1协商，零参info保持固定英文基准。 |

Mihon sourceId `8819952928004218630`：ExtensionPlugin的`comic fury/ja/1`经MD5前8字节大端并清除符号位，与用户给定ID一致。work.pathPrefix/idPrefix=`/comicprofile.php?url=`，chapter.pathPrefix/idPrefix=`/read/`，不声称samePageOrder；自定义布局/作者附言可能改变分页。

## 独立性与验证

仅新三源include comicfury_common.rs；已发布comicfury-en/zh未迁移且零改动。固定SOURCE_INFO、LANG、LANG_BYTES、EXPORT_NAME、METADATA和导出字面量在本源极简lib.rs声明，业务逻辑无三份复制。设置键showAuthorsNotes沿上游，设置由宿主按sourceId隔离。

包、真实WASM输出、样本目录/页表/作者附言与直连图片HTTP/MIME/尺寸/SHA256见dist/wave-27/verification.json。图标原件与包内审图见ASSET_SOURCES.md。所有5语简介从同一目录投影到presentation，作品标题不翻造。未操作设备、未提交/推送。
