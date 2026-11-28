# Comics Kingdom

报刊漫画与政治漫画的官方平台，收录 Blondie、Hägar the Horrible 等经典连载，按日期保存每日刊载。

ID com.comicskingdom.koma；版本0.1.0，API0.2，language=en，safe/nsfw=false，minAppVersion1.5.5。Mihon计算核准为3350274514125477391，固定Keiyoushi提交8d597e1bdbb0d9d961d941128703eaeb4bc8d1e3；确切路径规则未完成双端验证者不虚填导入映射。

## 移植前的源码对照

已实际读取固定上游build.gradle.kts及本站请求/DTO实现，并核准SDK导出宏、port_support HTTP/Node、source_protocol严格HTTP状态/transport门禁、Catalog/Context及GroupedChapters。源码证据dist/wave-34/evidence/upstream；完整实网结果dist/wave-34/verification.json。
所有通用栏目/筛选/错误按宿主UI五语zh-Hans/zh-Hant/en/ja/ko消费。零参SOURCE_INFO英语基准与metadata/presentation一致，站方标题不擅自翻译。不创建假首页，settings为空，不暴露未实现的偏好或权限。

| 上游/站方行为 | Koma操作与源内消费 |
|---|---|
| wp-json/wp/v2/ck_feature英文作品，20条分页、search/order过滤 | get_manga_list/search→browse/all：读取公开完整书目，按真实ck_favorite_count排序热门、modified排序最新、title排序标题；本地按20条args.page.cursor分页。上游orderBy=relevance对WP排序存在歧义，本源不把默认date排序假称人气；使用公开收藏计数。 |
| 详情title/content/meta/yoast缩略 | get_manga→manga/details，简介HTML转纯文本。 |
| 默认compactPref=true，每100天刊载合为一章 | get_chapters→chapters：从官网正文中的对应ck_comic查询精确统计归档块；不依赖响应头；get_pages→reader保留日期asc的原始块ID、范围和页序；原站深asc页实测429，因此按公开desc分页取出同一范围再反转（末块不足100按实际），不改分组含义。不以猜测章数代替完整目录，不返回缺assets的付费/不可读页。 |
| 公开single.url图片 | get_image_request→精确comicskingdom.com/wp.comicskingdom.com HTTPS白名单，不登录、不解挑战。 |

本批固定英语、公开日期归档与compact章节。上游未压缩逐日目录偏好和三态题材筛选未暴露为可操作设置；本源仅声明已实现的排序筛选。不会改动现有来源。

## 验证与保护

实际info、五语UI、popular第1/2页、latest差异、search、两组筛选、detail、完整目录分页、pages及图片请求均以最终.koma提取WASM执行。每源4张真实正文分别以curl无代理带/不带Referer下载并用Pillow/file验证JPEG/PNG/WebP；结果以verification.json和scratch/wave34_sol_report.md为准，不将构建通过称为真机通过。仅新增源目录与根三项登记，保护所有已有来源和外部WIP；sources/README.md不修改。未提交、推送或操作设备。

## 三台真机 QA 后的协议修复（0.1.0候选重打）

生产宿主1.5.5不暴露响应头，归档数量改从官网script#__NEXT_DATA__的props.pageProps.fallback中精确匹配postType=ck_comic及本站feature的pageInfo.totalItems；绝不读取headers/responseMetadata。Zits实测10340条，仍分104个原归档块，desc读取后反转保留日期顺序。官网JSON超过64KiB，源内文本缓冲明确扩至1MiB。详情使用真实ck_genre taxonomy、byline及活跃归档标记填充标签/作者/状态。

依据为用户103/197/237真机结果及dist/wave-34/qa-fix实网证据。修复后从最终.koma提取WASM复验；本机模拟/构建不替代用户后续真机复验。
