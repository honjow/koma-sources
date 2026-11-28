# GlobalComix（德语 / Deutsch）

GlobalComix 德语漫画分区，收录《Gottseibeiuns》等创作者发行作品。仅提供原站无需登录即可阅读的免费章节。

ID `com.globalcomix.de.koma`，语言`de`（Deutsch），版本0.1.0，API0.2，nsfw=true、contentRating=nsfw，最低Koma1.5.5。五语资料完整；零参source_info固定英文简介，与metadata/presentation同一基准文案投影。直接复用既有globalcomix_common.rs、globalcomix_comments.rs、globalcomix_ui.rs；本波仅更新共享模块的一行适用范围注释，解析、权限、设置和输出逻辑不变；既有es/it/pt源文件不修改。

## 移植前的源码对照

固定并实际读取 [Keiyoushi `e8c7f53b7994da527c78d54f36bcc7c6d7c4f9bf`](https://github.com/keiyoushi/extensions-source/tree/e8c7f53b7994da527c78d54f36bcc7c6d7c4f9bf/src/all/globalcomix) 的GlobalComix.kt、GlobalComixConstants.kt、build.gradle.kts、dto全部六个文件与gradle/build-logic/src/main/kotlin/ExtensionPlugin.kt。当前入口由Gradle语言清单和@Source生成；de未覆写source ID，也未作extLang映射，API使用lang_id[]=de / lang_id=de。

| 行为 | 上游 → 宿主调用 → 实际消费 |
| --- | --- |
| 热门/最新 | simpleQuery无sort / sort=recent → get_manga_list → shared list；args.page.cursor为从1起的API页号；核对pagination，终态null，不伪造独立首页 |
| 搜索 | simpleQuery sort=relevance+q → search → shared list，对query URL转义并保留原站排名；API搜索通常重排全语种目录，total_results不收缩，不在源端擅自过滤 |
| 作品/详情 | MangaDto.url=真实数字ID → get_manga → /comics/{id}；校验comic_langs中的de归属，采用目标语言名称与简介；作品gc:de:ID独立 |
| 章节与锁定 | releases?lang_id=de&all=true → get_chapters → 校验作品、语言、发行、key和免费/付费字段；按真实order生成续读组，锁话打断公共续读链；gcch:de:comic:release:uuid不合并其他版本 |
| 隐藏/恢复 | showLockedChapters_de默认true → settings与get_chapters；仅有source_chapter_exclusions_v1时展示锁话设置，隐藏后返回真实excludedChapterIds，精确chapterId恢复独立于隐藏；旧宿主仅返回免费话，不删除Library数据 |
| 正文与省流 | readV2/{uuid}，desktop/mobile_image_url → get_pages → 免费发行、完整page_count、is_page_paid=false和页归属校验；dataSaver_de默认false；gcp:de:comic:release:page按原数组顺序输出 |
| 请求/图片 | 公共CLIENT_ID、Origin、Referer与x-gc-identmode=cookie → SDK host.http_request / get_image_request → 仅官方精确HTTPS origin；不读账号或Cookie，不改写签名URL；不读取HTTP响应头，兼容宿主头值String/Array而无误判路径 |
| 评论 | 上游扩展未导出；官网/releases/{id}/comments → get_comments → comments version1 scopes[chapter]，校验object_id与作品/发行归属，保留删除状态；无写评论、登录或假分页 |
| UI与能力 | sdk/src/lib.rs导出宏、sdk/src/chapters.rs GroupedChapters、sdk/src/runtime.rs Context、sdk/src/localization.rs Catalog、sdk/src/comments.rs → args/page/settings/hostFeatures/environment → 五语栏目/设置/错误；作品名与站方内容原样保留；无本地化能力默认英文 |
| 过滤与访问 | 上游无独立filter schema → capabilities.filters=false、get_filters=not_implemented；沿用已发布源匿名阅读边界，不移植账号、购买、统计写请求或挑战绕过 |

Mihon sourceId **8246492784517686839**：`globalcomix/de/1`的MD5前8字节大端并清除符号位，按最新ExtensionPlugin实际算法计算。metadata work为空pathPrefix加gc:de:，chapter按targetIdSegment=4匹配UUID；仅因本源完整保留可读页的原始顺序，声明samePageOrder=true。章节scope、页ID、dataSaver_de与showLockedChapters_de均与其他语言隔离。

## 实网、图标与交付

当前目录88部；《Gottseibeiuns》（comic=52121）共9话、9话免费，首个免费发行34页。最终包内WASM核验热门/最新确有差异、搜索、详情、目录、阅读页与图片请求，并用无代理curl验证封面及至少前两张正文HTTP200和真实像素解码；签名仅忽略每请求变化的Date/Signature字段作独立资源比对，下载请求始终使用WASM原URL。

.koma由build.sh --package-only独立输出到dist/wave-24，未提交或推送。原件、512×512图标工艺及审图链见ASSET_SOURCES.md；完整实测记录在dist/wave-24/verification.json。另编译、打包、实网回归既有globalcomix-es，并比较基准WASM与最终包WASM。未操作设备，不能将本地联调等同真机验收。
