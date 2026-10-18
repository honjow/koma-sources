# 战双帕弥什官方漫画 0.1.2

当前正式接入沿用已验0.1.1业务，实现未重写；新增官网同头像400px原件和当前SDK标准构建。237普通索引更新、已安装行/浏览行及数据保持已具名接受。其余章节/下载/cold沿用各原回执边界，下列调查主体是0.1.0原始记录，后续变化见末尾。

`com.kurogame.pgr.zh.koma` / `zh`，minimum Koma0.1.1，现有SDK/ABI1，源级与逐书contentRating unknown、usage unknown。仅接战双帕弥什官网实际声明的公开中文漫画接口；不称全作品由同一作者制作、不猜评级。不提交/推送/发布/内置App，不动App/SDK/Kit。生产WASM/Wasmtime/ABI回放与Native/WAMR验收分开。

## 原站与已观察范围

`https://pns.kurogames.com/` 原HTML声明首页JS→实际首页模块→共享组件PcComics href `/comics`，实际301到 `/comics/`，页面标题“漫画版块-战双帕弥什”。原footer声明著作权人/©2018广州库洛科技有限公司。漫画原API helper baseURL `/v2`、`withCredentials:false`、timeout5000、固定公开`Site-Id:1`（不是登录凭据），匿名四POST路由 `Comics/list/detail/chapterList/content`。没有远程Search接口声明、没有签名/登录/凭据或挑战绕过。

实际冻结目录list5项：79战双帕弥什的日常、81《战双帕弥什》间章旧闻、78天空霸主赛利卡、80天空霸主赛利卡Ⅱ、77多米尼克的孤儿，保原顺序。Browse只发list、不多发每书detail；未声明作者/简介保持authors空数组、description null，不能将源作者Koma或官方主体补给每书。status按原客户端0未发布/1连载中/2已完结/4删除，只投影1ongoing/2completed，其他unknown；tag/type/hot/zan不猜分类，hot不作使用率或排名。

只样本79有原detail/目录两页/content708实体证据。详情author原“广州库洛科技有限公司”、desc原中文纯文字；其余4书detail、目录和正文均未取得/未回放，不冒称已验证。目录page1为20、page2为13，current/totalPage/totalCount/perPage完整校验，33唯一CID，原asc顺序保留；首708第零话→677第一话，末721番外→707番外篇，禁止按CID数值排序。detail.newChapter=708只是原入口字段，不把它称最新一话。

首708原content返回MID79/name/chapter/title、prev0/next677、一个imageList原URI。唯一实体正常匿名GET200 JPEG674216B/1980×5039，目检中文“帕什么什”Q版名称误读日常笑话、完整衣着，未见成人内容，只该样本一般向观察。原URI完整OSS `x-oss-process=image/resize,w_1980/quality,Q_70` 参数保留，没有原图变换、合并双页、猜文件名、去query或额外正文实体。全部原合同109项与补充17项保持。

## 动态接口与身份

每任务重新创建模块；静态数组只作当前操作buffer，每操作从上游重新赋值，不跨调用缓存，不依赖不存在cache ABI。MID `pgr:<真实comicsId>`、CID `pgrchapter:<MID整数>:<真实comicsChapterId>`、PID `pgrpage:<MID整数>:<CID整数>:<0based原数组页ordinal>`；精确作品URL为原router `/comics/comicsOfficial/<MID>`。

get_manga先list证明MID唯一存在、再detail证明MID/name/verticalCover与list对应；作者/简介只来自该真实detail。get_chapters先list证明MID，再从chapterList asc page1直到totalPage逐页读取，保API原顺序，拒绝分页元数据变化、重复CID、丢项/额外项，不输出截断成功。get_pages同样先list和完整目录，再content仅该CID一请求。content不返回CID，CID依据实际request及同MID目录唯一成员，必须逐一核content MID/name/chapter/title、prev/next与目录原邻接；任一变化失败关闭，不能只核同host或把别章当页。

get_image_request每个fresh Module重新执行上述list/完整目录/content严格owner链，且pageId包含CID/页序，mangaId有值时必须一致，mangaUrl有值时必须精确匹配作品URL，可选chapterId也必须一致。原imageList按ordinal及完整HTTPS解析URI逐字核，包括路径/查询；错误作品/章/页/外部URL/改query均拒绝。封面实际App形状pageId=MID，每次list+detail核MID/name/verticalCover归属后对精确URI，正文不能代cover。CDN前缀只是受限资源形状，不能授权图片。imageRequest返回原URI、空headers（实际首图匿名curl即成功；原漫画页面referrer=no-referrer）；无Cookie/Authorization/Referer注入。

每正文image_request额外 JSON POST成本=`1 list + N chapterList pages + 1 content`，当前样本N2即4次；cover成本list+detail=2。get_pages同4次，get_chapters3次，get_manga2次，Browse/Search1次。没有隐藏缓存/内部retry，分页需要顺序N请求；响应刷新/网络异常可能失败关闭而不可读，接受这一owner严格取舍。Host/CDN实际缓存和未来更新unknown。

请求源内按原helper组装POST及公开Site-Id/Accept/有body时Content-Type。现有ABI字段虽叫bodyBase64，实际App SourceRuntimeHttpTransport执行`sourceString(bodyBase64) ?? sourceString(body)`并原样extraData；此字段承载JSON原文，不做Base64变形，不修改公共SDK/ABI。timeout5000沿原helper。取证curl默认UA，WASM未额外设置User-Agent；实际Native NetworkKit自动header行为未验。本轮实现零新远程请求/正文实体。

## 文本、日期、搜索与边界

本地搜索只匹配list原中文标题，首尾空白/ASCII大小写规范后子串匹配，保原顺序；没有远程搜索调用，slug不冒充书名。Browse/Search每页20，基于实际完整list匹配数量决定nextCursor，不造终页。只一个“官方漫画”listing。

章节标题=原chapter文本+空格+title文本；原没有独立数值chapterNumber，输出null，volumeNumber/pageCount未知null，不用CID或ordinal造章号/未请求页数。createdAt仅输出有效原YYYY-MM-DD日精度字符串，不擅补时分秒/时区；异常/其它精度null，updatedAt null。当前App MangaDetailModels.optionalSourceTimestamp支持Date.parse字符串，但源输出保持原日精度，App时间显示行为由其现有转换负责。未声称自动续章的数字排序合同，不填continuationEligible=true。

JSON必须有效UTF8/完整结构；重复对象key或任何转义对象key失败关闭（阻止语义同名绕过），转义字符串值正常解码再一次JSON转义；对象/数组错误形状、错误类型、越界、容量不足均拒绝，不输出成功前缀。未知新增普通字段可忽略，不使用它们作归属。payload/HTTP/body约1MiB各，list/detail256KiB各，合并目录1MiB，max256作品/1000章/50目录页/10000页描述符，单URI2048B、普通文本4096B/搜索512B，有限buffer与取消检查，无OS/脚本运行。

## 图标、生产验证与未验

原官网HTML准确rel=icon URL `https://pns-cdnstatic.kurogames.com/resource/pns_website2.0/favicon.png`。原PNG30×30RGBA2936B，包字节全等，未放大/裁切/重编码/遮罩/底板/阴影。小尺寸在App显示清晰度未知。

有限最终生产Wasmtime每操作新Module：46操作（9原响应正例、21请求/功能负例、16明确synthetic上游负例），92离线JSON回调；真实输出5Browse、样本detail、全部33章及唯一1plan、1正文image owner和实际MID cover归属逐项核原JSON。70导出ABI envelope/范围拒绝、40init/20alloc检查绑定最终WASM。synthetic只标合成响应，不伪称真实HTTP；无网络回放/更多正文实体/设备。其他作品动态路由、另32章计划/实体/图标Native显示/阅读下载cold仍未验证。

原始证据 `/private/tmp/koma-pgr-zh-public-contract-20261014` 及 `/private/tmp/koma-pgr-zh-two-read-supplement-20261014`；完整交付 `/private/tmp/koma-pgr-zh-implementation-20261014`。Root独立审源码/原合同后负责Native，不以Host通过代替实机。


## Canonical 0.1.2 integration

The earlier sections document the original 0.1.0 investigation. The reviewed
0.1.1 chapter-label implementation is retained: explicit 第…话/話 labels set
chapterNumber, chapter readingGroup and continuationEligible; unrecognized
labels retain null, special and false. IDs, order, titles, dates, page plans
and image ownership remain unchanged. Existing capacity boundaries remain;
this integration does not claim every title or boundary has native coverage.

0.1.2 registers that existing provider in the canonical workspace/build and
uses the current SDK HTTP-status decoder. PGR already checks status 200 before
body decoding. The official 400px same-portrait original replaces the 30px
favicon; provenance and prior image-only acceptance are in ASSET_SOURCES.md.
Current integration review: /private/tmp/koma-pgr012-original-canonical-20261018.
