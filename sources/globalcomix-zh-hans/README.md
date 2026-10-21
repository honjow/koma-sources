# GlobalComix 中文 0.1.1

源身份 `com.globalcomix.zh-hans.koma`，语言 `zh`，API wire `cn`，版本0.1.1。完整业务继承已审011 owner，保全部API/权限、comic/release/page稳定ID及顺序、章号/语言、原请求头和图片描述符；本次无parser修改。源级MIXED按原合同报告nsfw，不表示所有作品成人。

## 标准构建与当前交付边界

在当前工作区使用 `bash build.sh --source globalcomix-zh-hans`；需要本机已安装wasm32目标、正式koma-source-dev、Python/Pillow及jq/zip。本候选以当前SDK8506离线标准编译；新WASM不同于旧Native包，不能把旧具名验收当本标准包Native通过。旧owner的 `package_candidate.py` 是历史临时工具，不作为当前标准入口，也不引入canonical。

当前包必需根资源由标准build生成manifest `icon: "icon.ico"`，source-owned原ICO及index旁置资源保持原bytes。三语言localizedNames只继承原metadata.json；不将SourceInfo当文件容器。当前源码/包准备为隔离候选，canonical和设备未应用。

## 历史业务证据与未验边界

原四个完整public JSON体覆盖19列表、侯門少爺寵上天cn详情、一条显式free Episode1、两页同owner/同release/未知或付费整次拒绝合同。原011 metadata修正仅通用中文，保wire cn与历史zh-hans suffix；不能承诺简体。旧Native曾具名首图/两页下载和进程保持；原onlineP2超时未关联URL仍OPEN，latest/search其他作品和权限不据此接受。

当前SDK兼容仅复用原JSON体作5操作离线回放，完整输出逐bytes等旧011；原最初HTTPheaders/信封缺失，200信封为明确受控重建，不能冒当前网络成功。标准新WASM与当前App路径尚未Native接受。

## 素材

当前根icon.ico来自官网HTML明确shortcut icon声明，原字节未转码/裁剪/重画。旧96PNG带黑色圆角板，保存在历史owner/010/011旧包，未删除历史证据，不沿用到本候选。官网触屏180PNG仍深实底，不选。来源、HTTP截断边界、SHA与图形身份见ASSET_SOURCES.md。当前小槽适用性仍Native待验；App显示圆角是独立层，本候选不改显示裁剪。

## 237 现装 0.1.1 具名在线观察（2026-10-21）

237 VDE-AL00展开竖屏1320×2120，普通Browse现装行0.1.1，热门19本。原《侯門少爺寵上天》一章/两页已下载，继续阅读仍第2/2页；本次未打开或删除该下载，不能用它关闭旧onlineP2超时边界。

改取列表中《泰坦的餘燼》，普通详情显示Kwame Kosi、1/1章/38页、未下载，源按现有免费release合同允许打开。亲看1/38真实封面及2/38繁中文对话正文；第二图本身是封面与正文的横向构图，在原单页模式中按比例显示，未改偏好或样式。自然Back后新记录绑定comicId=`gc:41899:the-last-vestige-of-titan`、release=`gcr:41899:365554:1f6e566b-48f8-4231-b85b-4176d8f26131`、pageId=`gcp:41899:365554:12806057`、pageIndex=1、totalPages=38、completed=false（comicId在App记录有本源ID前缀）。未下载、收藏、登录、付费或直接编辑持久记录。

原Browse末端完整画面已亲看，50项registry与进入前完整相同，原OverrideTimeout=10000ms/AWAKE读回，原237租约已正常release。只接受现装版本上述具名公开在线首两图与自然返回；未核安装包字节身份，不扩大为当前源码WASM、全38页、下载、cold、其他作品或整源通过。原侯门在线P2问题仍未复验。

原件在Koma `.hvigor/outputs/user-flow-audit/device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership/` 的`globalcomix011-{browse237-001,popular237-001,houmen-chapters237-001,titan-chapters237-001,titan-first237-001,titan-page2-237-001,titan-natural-return237-001,titan-session-restored237-001}`。回执与原图保留，artifact哈希关闭。

Root 已亲审本条首两页原图及精确记录；结论仅限上述具名路径，原未闭边界保持。

## 原侯门在线第二页的当前 Host 边界（2026-10-21）

接续时先核现存回执，确认原 owner 已完成唯一请求，因此没有重复联网。现存 readV2 原体的 payload.results 为 release177649、comic31985、key a658f6d5-2f9e-42fb-a9c8-d824d4cd1c46，is_free=true、premium_only=0；两页均为 cn、同 comic/release 且 is_page_paid=false。pageIndex1 的 pageId7348860 原公开 JPEG 请求返回200、4944829B、0.872秒；本次亲看原件，确为完整中文长条正文，可正常解码。readV2 本身200、21867B。两请求均无Cookie，15秒有界，无重试或登录。

原件为 Koma/.hvigor/outputs/user-flow-audit/globalcomix011-houmen-current-readv2.json、同名前缀 readv2-receipt.json/page2-receipt.json/page2-original.jpg。只证明当前 Host 的准确原第二页可取；不解释历史未关联URL的超时原因，也不将已下载原章的237离线路径冒在线复验。原 Native onlineP2 边界继续OPEN，源实现/版本/资源与设备数据均未改。

## 0.1.2 详情来源网页

只在get_manga的detail输出消费同次API响应Comic实体声明的原url，且仅在其为https://globalcomix.com/c/加已经校验的同实体slug时新增kind=source链接；缺失/不匹配沿原详情成功输出，不拼网址或猜语言query。原API请求是https://api.globalcomix.com/v1/read/<slug>，不把它当网页。共享列表emit_manga、id/slug/cn语言、章节/免费权限、正文/图片、icon.ico及metadata保持，版本011→012。原6fce37b恢复时已省略网页字段，没有查到刻意排除该字段的理由；本次按用户网页入口授权只补该字段及交付版本/记录。

原31985《侯門少爺寵上天》完整detail-once.json的payload.results.url明确为https://globalcomix.com/c/the-marquis-s-cherished-one，来源为既有保全包；现存2026-10-21 readV2原体内同comic.url再次相同。该地址是原响应网页声明，不是当前网页HTTP成功，也不强加cn或默认页面语言保证。原详情JSON提取到Koma .hvigor/outputs/user-flow-audit/globalcomix012-detail-fixture.json；原响应/headers不完整边界保留，离线200信封为受控重建，未发新请求。旧侯门在线P2超时仍OPEN，泰坦旧正文成功不混作本作或网页通过；012普通具名网页观察见下。

单源offline/locked release构建和真实WASM info012通过。同一侯门原JSON的实际WASM对照包括当前源码重编译011、保全原正式011与012，除新增links外全部manga字段与唯一HTTP请求深相等；缺失url、异域url、同域不同实体url三个受控边界仍与原011完整成功输出/请求相等且不造链接。输出沿Koma同产物分区`globalcomix011-{link-baseline,formal-original}-offline.json`、`globalcomix012-link-offline.json`及`globalcomix012-link-optional-boundaries.json`。这是原JSON/受控信封的离线输出，不单独证明当前公网作品页或Native通过。正式`dist/sources/globalcomix-zh-hans/globalcomix-zh-hans-0.1.2.koma`三成员继承原011完整manifest，仅version变化，localizedNames与metadata.json相等、icon.ico与原正式011逐字节相同；不改变图标。

237 VDE-AL00展开竖屏1320×2120，复用现装Debug16289681B/23:30:52，经系统picker011→012普通更新明确成功，服务54包与最终Browse54启用保持。普通热门仍19本，沿首项同一31985《侯門少爺寵上天》进入详情，1/1章、Episode1第2/2页及已下载保持。两网页菜单可见；只打开WebView，加载完成后为原响应声明的作品页，真实英文题名The Marquis’s Cherished One、同封面、Azuki Studio、状态/分类/简介可见；默认网页为英文，未追加语言query或保证中文网页。未遇拒绝/挑战、未刷新/登录。自然Back仍1/1章、Episode1第2/2页和已下载，再回列表/Browse；未打开正文、外部浏览器或下载，原侯门onlineP2超时仍OPEN，不能由网页或下载标记关闭。未操作泰坦作品，未导出全库，不扩为全部历史读态逐项校验。仅本人传包已清理并读回不存在，最终10000ms/AWAKE并释放租约。

原图/回执沿Koma `.hvigor/outputs/user-flow-audit/device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership/` 的`globalcomix012-import-detail237-001/{import-success,services54,browse012,source-list,detail-before}.png`、`globalcomix012-webview237-002/{both-web-actions,webview-workpage,detail-returned,browse-restored54}.png`及`globalcomix012-webview-postflight237-002/run-metadata.json`。
