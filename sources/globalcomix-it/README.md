# GlobalComix（意大利语）

GlobalComix 意大利语漫画分区，收录《Whisper ITA》等创作者发行作品。提供原站无需登录即可阅读的免费章节，订阅或购买内容仍受原站权限限制。

ID `com.globalcomix.it.koma`，语言`it`，版本0.1.0，API0.2，contentRating/nsfw均为nsfw/true，最低Koma1.5.5。零参SOURCE_INFO固定为英文简介，与五语metadata/presentation同一文案投影。新三源仅共享globalcomix_common.rs、globalcomix_ui.rs和globalcomix_comments.rs；既有GlobalComix源零改动。

## 移植前的源码对照

固定最新上游 [Keiyoushi `e8c7f53b7994da527c78d54f36bcc7c6d7c4f9bf`](https://github.com/keiyoushi/extensions-source/tree/e8c7f53b7994da527c78d54f36bcc7c6d7c4f9bf/src/all/globalcomix)。实际读取GlobalComix.kt、GlobalComixConstants.kt、dto下全部六个Kotlin文件与build.gradle.kts；当前入口为Gradle语言清单和@Source生成类，不是旧factory。

| 行为 | 上游合同 → Koma调用 → 新源消费 |
| --- | --- |
| 语言、身份 | Gradle实例`it`；GlobalComix.extLang默认原值 → get_manga_list/search/detail → LANG=it，API lang_id[]=it、lang_id=it；作品gc:it:、章节gcch:it:comic:release:uuid、页gcp:it:comic:release:page独立命名 |
| 热门/最新 | simpleQuery未指定sort / recent → get_manga_list → list；真实API分页payload.pagination.page/total_pages，args.page.cursor从1起，终态null，无独立首页 |
| 搜索/筛选 | simpleQuery relevance+q；上游无筛选表 → search/get_filters → query经URL转义；capabilities.filters=false，get_filters返回not_implemented，不伪造筛选。设置保留真实站点能力 |
| 详情 | 上游MangaDto作品url为数字ID，刷新可按read/slug → get_manga → /comics/{id}；验证id及comic_langs中的it所有权，优先目标语言名称/简介；不翻译站方书名、正文或标签 |
| 免费/锁话 | releases?lang_id=it&all=true；showLockedChapters_it默认true → get_chapters → 完整校验lang_id、comic_id、premium_only、is_free、key，按原order建立可读组；锁话打断续读链且不自动进入阅读；无章节排除能力旧宿主仅返回免费话 |
| 隐藏及恢复 | 上游开关隐藏premium → get_chapters(settings.showLockedChapters_it=false) → 仅在source_chapter_exclusions_v1下发实际excludedChapterIds，不删除用户目录和进度；精确args.chapterId恢复不受隐藏开关影响 |
| 阅读/省流 | readV2/{uuid}，DTO按dataSaver选择desktop/mobile_image_url → get_pages → 独立验证免费授权、发行/作品/语言/页面归属、is_page_paid=false、完整page_count与原数组顺序；dataSaver_it默认false |
| 图片/访问 | 官方公共x-gc-client、x-gc-identmode=cookie与Referer → SDK host.http_request / get_image_request → 精确HTTPS官网和官方desktop/mobile CDN，不读Cookie/账号，不绕过付费或CF，不解析响应头值（无String/Array差异路径） |
| 评论 | 上游扩展未导出；官网只读releases/{id}/comments → get_comments → comments version1、scope chapter；验证漫画/发行/评论object_id，保留已删除状态，原站一次返回列表；不伪造分页、写评论或登录 |
| UI/宿主 | SDK导出宏typed_chapters、sdk/src/chapters.rs GroupedChapters、sdk/src/runtime.rs Context、sdk/src/localization.rs Catalog、sdk/src/comments.rs → args/page/settings/hostFeatures/environment → 五语通用栏目、设置、错误，站点正文原样；无本地化能力时固定英文回退 |

Mihon精确sourceId **680772027911898954**：最新gradle/build-logic/src/main/kotlin/ExtensionPlugin.kt使用`globalcomix/it/1`的MD5前8字节大端、清符号位。build.gradle.kts未为此语种覆写ID。metadata.identity.mihon.work接受上游裸漫画数字ID并添加gc:it:；chapter用targetIdSegment=4匹配上游UUID；samePageOrder=true仅因本源完全保留原站page_objects数组顺序，实网逐项核对。

葡语核对：上游pt独立于pt-BR；pt-BR映射br，而pt仍为pt。现场API目录：pt=124部，pt-br=0部，br=144部；因此本波葡语源采用pt，不替换或混入巴西葡语目录。

## 实测与交付

编码前从本语种热门选中《Whisper ITA》（漫画ID 47911），16话中16话免费，首个免费话70页。封面与前三张正文经显式移除大小写代理变量并用curl --proxy '' --noproxy '*'直连，均HTTP200且完整解码。最终仍以包内WASM重新执行info、列表、分页、搜索、详情、免费目录、设置开关、页表、图片请求与章节评论，并核对最终包/WASM哈希；完整证据在dist/wave-23/verification.json和evidence/。无账号、购买、CF挑战或设备操作；App真实呈现留待用户QA。

搜索现场语义：`q` 配合 `sort=relevance` 会重新按相关度排列整个语种目录，`total_results`仍为该语言目录总量；不应以总数缩小作为成功条件，也不在源端擅自筛掉原站结果。本轮额外用热门中非首位作品做包内WASM搜索，确认对应作品移至第一位，证据见verification.json的searchEffect。
