# Mitaku

Mitaku 收录 Hana Bunny – Bela Dimitrescu 等 Cosplay 写真，按角色、模特与标签查找套图。混合帖子仅保留图片，不收录纯视频。

ID `com.mitaku.koma`，language=all，version=0.1.0，API0.2，nsfw=true/contentRating=nsfw，minAppVersion1.5.5。Mihon sourceId `7541045796029140694`；key `mitaku/all/1` 经MD5前8字节大端清除符号位。work/chapter保留原站以/开头的图集路径；单图集一个章节，页序与固定上游一致，不插入封面或推荐图。

## 移植前的源码对照

固定Keiyoushi [8d597e1bdbb0d9d961d941128703eaeb4bc8d1e3](https://github.com/keiyoushi/extensions-source/tree/8d597e1bdbb0d9d961d941128703eaeb4bc8d1e3)，实际读取本站build.gradle.kts、Mitaku.kt/Filters.kt或Masonry.kt/Filters.kt；请求与SDK导出宏、port_support.rs HTTP/Node、runtime::Context、localization::Catalog和GroupedChapters调用链逐项核准。

| 行为 | 上游 → Koma操作 → 源内消费 |
| --- | --- |
| 列表/分页 | Mitaku Ero Cosplay分类/page/n，无独立latest，supportsLatest=false → get_manga_list → browse；只声明Cosplay栏目，latest请求返回unsupported。每个卡片过滤(Video)后验证正文存在图片锚点，无图片则不进入列表；原站wp-pagenavi决定cursor。额外逐帖GET是排除无图帖的明确适配，不伪造热榜。 |
| 搜索/筛选 | 搜索优先词query，其次category，其次单个tag；标签小写并以短横线连接 → search/get_filters → browse/filters。保留Any/Ero Cosplay/Nude/Sexy Set，明确去掉上游Online Video纯视频分类；其他视频与0锚点帖子同样不收录，混合贴保留全部原图。 |
| 详情/目录 | 原站图集路径 → get_manga/get_chapters → detail/gallery；先验证正文原图存在再返回单章，目录使用原URL稳定ID与原图张数，status=completed。 |
| 正文/归属 | 只取a.msacwl-img-link data-mfp-src优先，否则href；图片扩展校验排除video/iframe，不取全页img。 get_pages核对mangaId与chapterId同一图集，保留DOM顺序，不合并或重排图片。 |
| 图片/安全 | get_image_request仅允许HTTPS本站与精确mitaku.net，禁止外域、凭据、非标准端口、fragment，发送公开Referer。无登录、账号、挑战绕过或响应头读取；不会误处理宿主String/Array头值。 |
| UI/能力 | SOURCE_INFO固定英文基准与metadata/presentation同一文案投影；get_listings/get_filters、图库章名和错误按宿主environment/source_localization_v1选择五语；站方图集名和标签原样保留。无上游设置，settings=false并返回空schema；无首页或评论。 |

## 验证与边界

代表作：Hana Bunny – Bela Dimitrescu，搜索词 `Hana Bunny`。最终包运行info、五语UI、列表分页/搜索/筛选、详情/完整页表和get_image_request，至少封面+前三张原图直连HTTP200并解码，原页表独立HTML比对；记录见dist/wave-31/verification.json。图标见ASSET_SOURCES.md；本轮未操作设备、不提交推送。

两Hunter仅通过新masonry_hunter.rs复用协议，各自BASE/IMAGE_HOST/METADATA及身份独立，不改已有来源。Mitaku为独立协议文件，不改变kiutaku/xiutaku的历史图片路由。完整保护已有eighteenmanhua/miwumh与根注册WIP；sources/README.md不改。

标签筛选沿上游使用标签路径slug。原站Hana Bunny实际slug为hana-bunny-001，不能假设所有显示名转短横线后都存在；Bela Dimitrescu→bela-dimitrescu已实网验证。404原样返回，不用空列表掩盖错误。
