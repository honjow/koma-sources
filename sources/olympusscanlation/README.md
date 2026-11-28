# Olympus Scanlation

Olympus Scanlation 的西班牙语漫画，收录《El Blinker genio de la academia mágica》等奇幻连载，可按原站题材与连载状态选读；不含文字小说。

ID com.olympusscanlation.koma；版本0.1.0，API0.2，language=es，safe/nsfw=false，minAppVersion1.5.5。Mihon计算核准为1596411039134168890，固定Keiyoushi提交8d597e1bdbb0d9d961d941128703eaeb4bc8d1e3；确切路径规则未完成双端验证者不虚填导入映射。

## 移植前的源码对照

已实际读取固定上游build.gradle.kts及本站请求/DTO实现，并核准SDK导出宏、port_support HTTP/Node、source_protocol严格HTTP状态/transport门禁、Catalog/Context及GroupedChapters。源码证据dist/wave-34/evidence/upstream；完整实网结果dist/wave-34/verification.json。
所有通用栏目/筛选/错误按宿主UI五语zh-Hans/zh-Hant/en/ja/ko消费。零参SOURCE_INFO英语基准与metadata/presentation一致，站方标题不擅自翻译。不创建假首页，settings为空，不暴露未实现的偏好或权限。

| 上游/站方行为 | Koma操作与源内消费 |
|---|---|
| api/series/list提供id/slug/type；旧实现缓存slugMap | 每个操作即时解析公开comic清单，稳定作品ID仍为数字；不依赖上一次WASM实例缓存，不收录novel。 |
| api/rankings page+total_ranking、api/new-chapters page | get_manga_list→browse，按current/last_page真实分页，不伪造popular/latest。 |
| 关键词对series清单本地筛选20条；空词api/series按status/genres | search→browse，get_filters→genres-statuses动态标签；字段及当前站方全部状态/题材均按宿主UI五语本地化；未来未知标签保留站方原文，不猜测含义。 |
| api/series/{slug}详情，chapters分页meta.total | get_manga/details、get_chapters按源分页和total给游标及readingOrder；不因一次page返回结束而截断。 |
| capitulo/{chapterId}/comic-{slug} main.strip panel>img | get_pages→reader，核对parentID，完整提取真实正文。旧章部分返回/cp/cp-*.jpg占位且404，此时明确chapter_unavailable，不以占位图伪装正文。最新可读章35图已实测。 |
| 公开imagesolymp图片 | get_image_request只允许精确官方/CDN域HTTPS；尊重内容不可用，不绕过登录或内容移除。 |

上游versionId=3，不能用versionCode=5计算Mihon ID。固定站点经现有官方跳转定位到olympusxyz.com，本源不自动接受未知域。

## 验证与保护

实际info、五语UI、popular第1/2页、latest差异、search、两组筛选、detail、完整目录分页、pages及图片请求均以最终.koma提取WASM执行。每源4张真实正文分别以curl无代理带/不带Referer下载并用Pillow/file验证JPEG/PNG/WebP；结果以verification.json和scratch/wave34_sol_report.md为准，不将构建通过称为真机通过。仅新增源目录与根三项登记，保护所有已有来源和外部WIP；sources/README.md不修改。未提交、推送或操作设备。

## 三台真机 QA 后的协议修复（0.1.0候选重打）

详情字段核准为summary、status.name、genres[].name、team.name，分别填简介/状态/标签/作者并trim。带词+筛选不再走无筛选series/list：每次扫描真实status/genres目录最多5页再匹配关键词，nextCursor是后续原站页，直至完整结束；不得将窗口无命中假称全部搜索结束。

依据为用户103/197/237真机结果及dist/wave-34/qa-fix实网证据。修复后从最终.koma提取WASM复验；本机模拟/构建不替代用户后续真机复验。

## 筛选状态机兼容修正（197 / 237复验）

两个榜单的supportsFilters=false，榜单只负责热门/最新。get_filters中所有项为scope=search；仅协商到source_filter_search_without_query_v1时声明useSearchForFilters=true，让宿主空词提交与带词搜索共用searchFilterValues。未协商时不声明该能力，仍可在关键词搜索使用这些搜索筛选。NovelCool的search仅按filters.order决定搜索排序，get_manga_list仅按listingId决定榜单端点，不允许默认popular筛选覆盖latest。依据为用户核实的SourceBrowsePage/BrowseViewModel状态机和jcomic/mycomic的SDK范本。验证证据dist/wave-34/filter-scope-fix；未改宿主、Comics Kingdom或其他既有源。
