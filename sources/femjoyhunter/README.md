# Femjoy Hunter

Femjoy Hunter 按模特与摄影品牌整理成人写真，收录 Corinna F – Makeup 等套图。每套保留原站照片顺序，排除视频条目。

ID `com.femjoyhunter.koma`，language=all，version=0.1.0，API0.2，nsfw=true/contentRating=nsfw，minAppVersion1.5.5。Mihon sourceId `9185590742756804817`；key `femjoy hunter/all/1` 经MD5前8字节大端清除符号位。work/chapter保留原站以/开头的图集路径；单图集一个章节，页序与固定上游一致，不插入封面或推荐图。

## 移植前的源码对照

固定Keiyoushi [8d597e1bdbb0d9d961d941128703eaeb4bc8d1e3](https://github.com/keiyoushi/extensions-source/tree/8d597e1bdbb0d9d961d941128703eaeb4bc8d1e3)，实际读取本站build.gradle.kts、Mitaku.kt/Filters.kt或Masonry.kt/Filters.kt；请求与SDK导出宏、port_support.rs HTTP/Node、runtime::Context、localization::Catalog和GroupedChapters调用链逐项核准。

| 行为 | 上游 → Koma操作 → 源内消费 |
| --- | --- |
| 列表/分页 | Masonry getGridPage → get_manga_list → browse；gridapi content=updates，sort=trending/newest，mpage来自args.page.cursor；按原始figure数量>=20判断下一页，排除.icon-play视频卡片，不因过滤后不足20提前截断。 |
| 搜索/筛选 | 有关键词走/search/post/关键词/mpage/n/，忽略其他筛选；无词走gridapi，sort=filter，content按字段序JSON(content=pix,quality,label,ord)，标签以+连接。Order=0/popular/newest、Quality=0/hd。get_filters从/sort/newest/动态读取#filter-a section[data-slot=tags] label中的.txt和input value；多选标签仍是站方原文，静态UI五语。 |
| 详情/目录 | 原站图集路径 → get_manga/get_chapters → detail/gallery；先验证正文原图存在再返回单章，目录使用原URL稳定ID与原图张数，status=completed。 |
| 正文/归属 | 只取.list-gallery a[href^=https://cdn.]，不取封面、全页img或相关推荐。 get_pages核对mangaId与chapterId同一图集，保留DOM顺序，不合并或重排图片。 |
| 图片/安全 | get_image_request仅允许HTTPS本站与精确cdn.femjoyhunter.com，禁止外域、凭据、非标准端口、fragment，发送公开Referer。无登录、账号、挑战绕过或响应头读取；不会误处理宿主String/Array头值。 |
| UI/能力 | SOURCE_INFO固定英文基准与metadata/presentation同一文案投影；get_listings/get_filters、图库章名和错误按宿主environment/source_localization_v1选择五语；站方图集名和标签原样保留。无上游设置，settings=false并返回空schema；无首页或评论。 |

## 验证与边界

代表作：Corinna F – Makeup，搜索词 `Corinna`。最终包运行info、五语UI、列表分页/搜索/筛选、详情/完整页表和get_image_request，至少封面+前三张原图直连HTTP200并解码，原页表独立HTML比对；记录见dist/wave-31/verification.json。图标见ASSET_SOURCES.md；本轮未操作设备、不提交推送。

两Hunter仅通过新masonry_hunter.rs复用协议，各自BASE/IMAGE_HOST/METADATA及身份独立，不改已有来源。Mitaku为独立协议文件，不改变kiutaku/xiutaku的历史图片路由。完整保护已有eighteenmanhua/miwumh与根注册WIP；sources/README.md不改。
