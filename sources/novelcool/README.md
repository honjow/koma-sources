# NovelCool

NovelCool 的英文图像漫画目录，包含《One Piece》等连载。本源排除纯文字小说和成人分类，正文沿用原站分页阅读。

ID com.novelcool.koma；版本0.1.0，API0.2，language=en，safe/nsfw=false，minAppVersion1.5.5。Mihon计算核准为7570101320206504111，固定Keiyoushi提交8d597e1bdbb0d9d961d941128703eaeb4bc8d1e3；确切路径规则未完成双端验证者不虚填导入映射。

## 移植前的源码对照

已实际读取固定上游build.gradle.kts及本站请求/DTO实现，并核准SDK导出宏、port_support HTTP/Node、source_protocol严格HTTP状态/transport门禁、Catalog/Context及GroupedChapters。源码证据dist/wave-34/evidence/upstream；完整实网结果dist/wave-34/verification.json。
所有通用栏目/筛选/错误按宿主UI五语zh-Hans/zh-Hant/en/ja/ko消费。零参SOURCE_INFO英语基准与metadata/presentation一致，站方标题不擅自翻译。不创建假首页，settings为空，不暴露未实现的偏好或权限。

| 上游/站方行为 | Koma操作与源内消费 |
|---|---|
| 默认AppAPI=true，POST elite/hot、elite/latest、book/search；appId/lang/lc_type/page/page_size/secret | get_manga_list/search→browse，固定en、manga、20条；保留真实页游标，排除is_novel及Adult/Smut/Hentai/Erotica/Mature/Ecchi/Pornographic/Fetish/MILF/Netorare分类，详情再次核准。公开上游appId/secret是客户端标识，不是用户登录凭据。 |
| 网页详情bookinfo、bk-summary，chapter-item-list全部a | 详情/reader显式使用okhttp/4.12.0请求头，对齐上游Fetch.NONE；浏览器头会得到第三方/go/rds/广告链接，拒绝把这些链接当章节。get_manga/book/details，get_chapters完整解析后按100条页游标返回；源内HTML节点上限16384替代公共4096，不修改公共模块。过大响应明确result_too_large，不伪称完整。 |
| getPageList返回每页HTML定位符；getImageUrl取mangaread-manga-pic | get_pages→reader保留原站option顺序并去重复导航；image.url为稳定原站reader定位符，get_image_request请求当页再得到新鲜签名CDN图，避免把短期签名当永久页身份。支持单页图片章；无真实图返回不可读，不拿推荐图补齐。 |
| 网页/API模式、网页筛选 | 本批只暴露默认AppAPI图像漫画模式；不提供尚未实现的网页模式开关。无关键词时排序映射到真实hot/latest端点；关键词保留AppAPI搜索语义，题材与状态始终取交集。 |

超大作品HTML受宿主单响应预算限制，失败明确报错；不是登录墙或假空目录。正文定位符必须通过get_image_request解析，不直接把HTML当图片下载。Mihon复杂混合阅读模式的章节恢复未在真机核准，不虚报导入验收。

## 验证与保护

实际info、五语UI、popular第1/2页、latest差异、search、两组筛选、detail、完整目录分页、pages及图片请求均以最终.koma提取WASM执行。每源4张真实正文分别以curl无代理带/不带Referer下载并用Pillow/file验证JPEG/PNG/WebP；结果以verification.json和scratch/wave34_sol_report.md为准，不将构建通过称为真机通过。仅新增源目录与根三项登记，保护所有已有来源和外部WIP；sources/README.md不修改。未提交、推送或操作设备。

## 三台真机 QA 后的协议修复（0.1.0候选重打）

筛选args.filters为键值对象，搜索操作中的显式order独立于宿主保留的listingId；榜单操作严格按listingId选择hot/latest，忽略默认order筛选。固定上游AppAPI不接受题材/状态参数；网页search探测也未体现所传分类差异，因此从AppAPI真实category/ completed字段过滤，暴露经核实的6项非成人题材及状态。每次最多扫描5页，nextCursor沿用下一个原站页码；空窗口仍保留hasMore，不丢后续结果。关键词和题材/状态取交集；关键词保留上游AppAPI搜索排序，非关键词排序由hot/latest真实端点决定。详情按bookinfo-author/bk-status/bk-cates填作者/状态/标签；题材只取a[title]，去除桌面/移动重复和状态链接。仅network_error的连接/正文瞬断重试一次；HTTP拒绝/限流/取消不重试，所有请求仍经过宿主限速。

依据为用户103/197/237真机结果及dist/wave-34/qa-fix实网证据。修复后从最终.koma提取WASM复验；本机模拟/构建不替代用户后续真机复验。

## 筛选状态机兼容修正（197 / 237复验）

两个榜单的supportsFilters=false，榜单只负责热门/最新。get_filters中所有项为scope=search；仅协商到source_filter_search_without_query_v1时声明useSearchForFilters=true，让宿主空词提交与带词搜索共用searchFilterValues。未协商时不声明该能力，仍可在关键词搜索使用这些搜索筛选。NovelCool的search仅按filters.order决定搜索排序，get_manga_list仅按listingId决定榜单端点，不允许默认popular筛选覆盖latest。依据为用户核实的SourceBrowsePage/BrowseViewModel状态机和jcomic/mycomic的SDK范本。验证证据dist/wave-34/filter-scope-fix；未改宿主、Comics Kingdom或其他既有源。
