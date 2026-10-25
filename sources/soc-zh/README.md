# 铃兰之剑中文・官方漫画（SOC 0.1.0）

只提供铃兰之剑官方认证发布者在 TapTap 的《伊利亚没问题！》中文漫画。当前范围是 `app209866/group291080/group_label946587` 普通网页的首屏10篇；不是全连载57篇。官网为 https://soc.xd.cn/ ，漫画实际发布入口为 https://www.taptap.cn/app/209866/topic?group_label_id=946587&type=group_label 。官网现有静态游戏推广配置本身不提供漫画正文。

## 实际合同与范围

- 当前原 Nuxt `data` 的 feed 容器通过 `list → item.moment` 绑定10篇；逐项核 `moment.is_official=true`、group.id291080/group.app_id209866、group_label.id946587/name趣味漫画、author.app.id209866/identifier com.xd.ssrpg、author.user.id391283780 及 verified.reason《铃兰之剑》官方。feed `identification` 的实际形式为 `moment:<moment.id_str>`，还须匹配原 HTML 的该作品 href。
- 不扫描推荐树充当目录，不调用 SSR 中的 API `next_page`，不执行 JavaScript、Nuxt hydration、签名、Token 或账户接口。当前 `total=57` 不证明有57篇可读漫画。列表明确支持 `supportsPaging=false`；只接受首屏 cursor null/1，其他页显式拒绝。响应 hasMore=false 指本源已声明的有限首屏集合，不声称官方站无后页；当前官网 HTML rel-next 尚未请求。
- 每篇是独立作品、一个完整阅读单元、一张完整原长图；原标题保留，不裁四格成四页，不人为串成57章。稳定 MID `soc:zh:comic:<18-digit-moment>`，CID `socread:zh:comic:<moment>`，PageID `socpage:zh:comic:<moment>:0`。chapterNumber/volumeNumber=null，readingGroup 为同一 MID，continuationEligible=false。不把数组顺序或日期推为章号。
- 作者是官方发布者而非已证创作者，逐篇 authors/artists 留空；status/contentRating/usage unknown。非成人样本只证明所留首篇为普通中文趣味漫画，不冒充正式评级。
- 源内有界 JSON 语法扫描与 Nuxt 混合数组索引解引用：原 JSON 上限1MiB、4096条索引、嵌套64层，direct-member 关键属性重复拒绝，无网络或代码执行回退。源内解析不改 SDK。当前生产 C++ HtmlHost/Lexbor 原代码读取 script#__NUXT_DATA__[type=application/json][data-ssr=true] 的原 Text；不存在属性的 Attr 从不探测，所有属性选择器 presence-qualified，所有描述符关闭。macOS Wasmtime/C++回放不是197 WAMR验收。
- 详情须有唯一主 data 容器 `moment + first_post`，重复同样的官方/work/title/images身份校验；first_post.is_topic=true/position1，first_post 和 topic 原 id_str 均存在但两者是不同身份，不能强行相等。first_post.images 只有1项，URI须与本作品 topic.images 唯一 original_url 相等；原 HTML og:url、唯一正文 wrapper、唯一 `.tap-rich-content__image--topic img[src]`、原发布者 href 全部核对。邻接攻略推荐、分享图、游戏推广下载卡和广告不纳入正文。
- 正文只采用原数据明确的 original_url，且 original_url 与原网页图片 url（同 URI + `/_tap_ugc.jpg`）相互验证；不删 URL 后缀猜原图。要求实际已核图片元数据2362×6385及匹配 png/jpeg原格式，原URI只允许 HTTPS `img2-tc.tapimg.com/moment/etag/` 下单个安全文件名 .png/.jpg；无query、额外子目录、encoded slash、跨host回退。get_image_request只接受当前首屏已验证作品图片完整URI；MID/None用于原图封面，正文 PageID:0 必须重新验证该详情。独立 mangaUrl 若存在须完整匹配该作品原URL（含group_id291080）。不发任意URI的HTTP。

## 原图与图标

首篇“【伊利亚没问题！】启示录特别篇：行动观察日记”原详情 href 是 https://www.taptap.cn/moment/826828574290872188?group_id=291080 ，明确 original_url https://img2-tc.tapimg.com/moment/etag/lpjiPsdCMefxZWcDjCg4iP72K_jD_20260716143934.png 。原GET200，PNG2362×6385、7619323B、SHA256 `2ac4d352cb7d6ba66d1313572fb3eb793a0f6c30cf8b6763f2d2968c68b27a0c`；原完整四格已目检。imageRequest headers={}，原PNG一次普通匿名GET本身无Referer/Cookie、成功200；不把计划替代真实Native图片加载/下载。源包不包含漫画原图。

图标采用原 author.app.icon.original_url 声明的游戏专属原PNG https://img-tc.tapimg.com/market/images/f8342359df8fa63b0de1600629035bb0.png ，原GET200、实际512×512/23419B、SHA256 `02562c854d245c33876e9ee8700c6ff29b57f3157f334328449a6df9d314428a`。页面旧metadata270×270/24839B与当前原体不符，实际字节和IHDR尺寸为准；没有改像素/重绘/裁切/缩放/转码/透明/外底板/遮罩。图像本身的像素画、光影和背景都是原资产。并非平台泛用图标。

## 成本与未验收

列表/本范围标题搜索/封面计划各1次目录GET；详情/章节/页面/正文图片计划各2次（目录 + 当前详情）GET，均同步匿名公开，无重试或缓存注入。未核页和界外URI显式拒绝，不扩大目录。源引擎通常每操作新实例，无共享ABI缓存承诺；本包未擅自添加跨操作状态。

原响应、原headers、原素材、原JSON/HTML父切片和30份SHA绑定在 `/private/tmp/koma-soc-zh-public-contract-20261015/`。源实现/真实C++HtmlHost回放/独立合成拒绝材料/完整ABI/冻结物位于 `/private/tmp/koma-soc-zh-implementation-20261015/`。真实回放只用已保存原体，合成异常另外标记，不冒充实站响应；没有再远程抓取。

首屏10条目录/封面描述符可离线核对；只有首篇详情和原完整PNG已取得实站原件，其余9篇详情、原图读取、后续47 feed条目、真实使用率及全部197路径未知。构建/生产C++HtmlHost离线回放均不等于197普通导入、浏览、图片解码、阅读、下载、cold验收。Root审完整差异后负责设备。保护全部旧冻结包、主源WIP、App/ReaderKit/SDK；不提交、推送或发版。

## 2026-10-25 Native 首篇验证

237（VDE-AL00，展开竖屏）使用 Koma 0.1.2-pre 的正式签名 Release 包，正常安装本源0.1.0后，浏览页显示当前首屏10篇。进入实际首篇“行动观察日记”，详情、章节、取页及图片描述均完成；正文 HTTP 200，收到7,619,323字节，完整四格长图按一页显示。原图加载时有进度提示；正常返回后原书架和所有旧阅读记录保持，只新增本次作品的正常阅读记录。

本次只确认这一首屏及首篇正文，不代表其余9篇正文、下载、冷启动、后续47条或整源性能均已通过。封面当前也使用原图，本次单张为数MB至十余MB，不能据本数出现即称封面加载充分优化；原URI校验和有限目录合同保持。
