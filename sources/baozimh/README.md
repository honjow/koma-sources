# 包子漫畫

## 0.1.28 镜像与设置对齐

对照 Keiyoushi `5d54f0d02e2a7dbdda08e823de444f7527cc883c` 的
[build.gradle.kts](https://github.com/keiyoushi/extensions-source/blob/5d54f0d02e2a7dbdda08e823de444f7527cc883c/src/zh/baozimanhua/build.gradle.kts)
及同提交 `Baozi.kt`、生成器 `MirrorPreferences` / `SourceProcessor`。
镜像来自生成器，而非 `Baozi.setupPreferenceScreen` 内的两个显式偏好；旧移植漏掉了生成项。

| 行为 | 上游 | 本源消费 |
| --- | --- | --- |
| 首选镜像 | `preferred_mirror`，17项、索引字符串0–16 | `mirror::selected` 同17项；列表、搜索、详情、目录及公开作品地址共用所选站点 |
| 默认站点 | CN、索引0 | 保留既有WWW、索引2；已有用户更新不会被自动切到CN。无设置或无效值回WWW |
| App镜像 | AppCN / AppGB页面加`/baozimhapp`，卡片和章节可能由onclick提供身份 | `append_card` / `chapter_link` 将onclick作为数据解析，保持原作品ID与section0章节ID；非0section保留在章slot中 |
| 主页与网页会话 | 当前所选镜像 | 包默认主页覆盖WWW，`homepageChoices`列出另16项；会话按17个精确origin声明，仍需宿主原授权，不跨站复制Cookie |
| 封面 | `data-src`优先、兼容`src`，解包`/cover/https://...`嵌套地址 | 卡片和详情读取真实图片地址；普通图片查询参数保留，只有嵌套封面去掉包装参数；旧静态地址只作缺图回退 |
| 热门、搜索、筛选 | 所选`siteBaseUrl`；非空关键词不分页 | 原热门及筛选保留，空搜索按筛选，模板卡片不作为漫画输出 |
| 标题开头筛选 | `filter`组，全部及ABCD至0-9共9项 | 补齐原缺的整组和请求消费，不再固定`filter=*`；原地区/进度/标签34个值及顺序保持。选项采用已核网页原件，未实现上游动态获取筛选数据 |
| 最近更新 | 网页镜像`/list/new`无分页，App镜像使用热门 | 新增`latest`栏目，分支与停止条件同上游；网页latest不显示无效筛选，App保留热门筛选；原首页四栏保持 |
| 横幅 | `BAOZI_BANNER`，默认1、0/1/2 | 保留既有图片处理能力协商及三个值 |
| 章节身份纠错 | `CHAPTER_ORDER`，默认0、0/1/2；不修改数组排序 | 保留既有章节身份协商、标题片段及进度匹配；主目录位置仍用已验证的provider坐标 |
| Koma原有设置 | 上游没有默认地区/状态 | 保留`defaultRegion` / `defaultState`及具体筛选优先规则 |
| 正文 | App镜像的整章`.comic-contain img[data-src]` | App镜像及非0section用`app_pages::run`；网页镜像保留已验AMP长章分页，首入口不可用或无图时一次回退上游App入口；稳定章ID和页ID保留 |

上游网页镜像的App阅读基址固定AppCN；AppGB镜像使用AppGB，不是将全部资源强改成所选
网页主机。旧AMP路径在当前普通阅读已验收，因此本次保留；不宣称该优先顺序与上游相同。
所有结果仍要求完整响应，不能把前50图或部分数组作为完整章节。App整章入口只尝试一次，
不在原运输重试之外叠加一轮三次重试。

本次未替换已验的每章有界CDN选路及横幅处理。当前上游另有`.baozicdn.com`到
`.baozimh.com`的图片改写/404回退和占位图重定向拦截；Koma原CDN机制与之不完全相同。
站点验证继续沿现有网页会话，不在源中新增自动挑战求解器。镜像/模板和这些网络差异分别
记录，不能从设置项齐全推断整源完全等价。

AppCN/AppGB在237曾出现网页有内容、原生请求验证后仍403的差异；现仅为这两个精确origin声明既有`chrome145`普通HTTP配置，不改变WWW/CN等网页镜像或验证页UA。该配置已存在于正式App1.4.0；最终候选在237已读回两域列表、搜索、详情、目录及页表实际HTTP200，并取得正文实图。原403证据保留，不将普通传输配置当作验证页UA。

App目录实际还会输出`https://undefined.baozimh.com/comic/<别名>`这类错误SEO地址。
仅在App镜像中，把仍属本站HTTPS域族且严格等于输入slug加下划线字母数字后缀的canonical
路径作为坐标绑定；仍要求唯一canonical、主目录全部同owner、section0连续完整及摘要槽位
落在主目录内。它不成为请求地址或会话授权；WWW等网页镜像原有同源校验不放宽。
本次浏览器原件证实剑尊App主目录194行、总218行；旧稳定slug的App第一话为整章195图。

最低App保留1.4.1；所需通用主页选择已在1.4.0正式提供。本版无新增宿主接口。
最终候选的默认WWW四份完整目录、两部195/182图及标题后缀完整输出已与0.1.27相等。
237已核17镜像设置显示与保存、CN所选主页实际URL、标题开头ABCD筛选和最近更新；
AppCN及AppGB的旧稳定ID《劍尊歸來》均返回194条唯一主线关系、位置0–193，第一话
195页并可明确切到第二话152页及清晰正文；普通返回后另开《紹宋》237章及角色设定
正文也已验。无痕、原设置和用户原阅读位置均已读回保留。该范围不代表17域名全部可达
或所有CDN已逐张验证。早先长会话有请求及图片准备完成却仍停在详情的记录，具体导航
原因未证；正常冷重开及普通Back后的起读已通过，不把旧失败写成宿主缺陷已修。

## 0.1.27 候选：canonical 别名与主目录章序

官网部分作品的 canonical 与主目录 comic_id 已加后缀，输入仍是既有稳定作品ID。
0.1.26把这个provider owner与输入旧slug直接比较，导致两份真实完整目录（194与210条）
丢失全部主目录位置；“最新章节”倒排摘要保留在显示数组前部后，宿主全无合同回退
产生了52条错误邻章。不是按标题或章节编号判断方向，也不是CLI403或设备验收结论。

provider坐标与对外身份分离：主目录owner与输入相同时沿原合同；不同时，必须有唯一
同源HTTPS canonical，其/comic/路径owner须与全部主目录href一致。初始与展开两区仍
要求section_slot=0且chapter_slot按真实文档连续覆盖0..N-1，任意缺槽、混owner、其它
section、截断字段或不明确canonical均不能新增主线位置。同owner主线最新摘要的槽位也必须全部位于已验证主目录内，不能把截断后连续前缀当作完整目录。主目录之外的版本/分区保持
原边界，不推入baozi-main；原标题、精确事实去重、显示数组和作品/章节业务ID不变。
CHAPTER_ORDER三个选项及旧宿主特性门禁保持；不改网页会话、页表、CDN选路或素材。

上游与宿主对照：本合同沿0.1.22主目录/摘要区分和0.1.26初始+展开主目录修复
（f999e3b）；当前公开9e4索引为0.1.26，SOURCE_INFO直接读Cargo版本。网站真实
canonical和全量href是provider owner依据；run_get_chapters仍用输入slug构建旧ID，
仅main_catalog及每行placement使用已验证owner。SDK GroupedChapters和App现有
sourceCatalogChronology/continuationChapterIds负责消费显式位置，显示倒序独立。

正文也有同一别名边界：两作旧reader入口实际200并重定向到后缀canonical，下一part
使用新owner；旧版仅比较旧slug拼出的下一part，实际195图/182图都只返回前50图。
本版仅在首文档存在唯一同源reader canonical、相同section0/章slot，且owner为原slug
或其明确下划线后缀别名时，用该owner比较同章下一part。未确认、异源、其它章或section
不会扩大跳转范围；首个URL、chapterId、pageId、图片原序与边界重复去除仍沿原规则。
CDN选路继续绑定稳定slug的真实图片目录，不以reader别名替换图片路径。

候选已完成五份完整目录原件的2293条回归及当前App函数消费，52条错误邻章归零；
其中四份是本轮完整文档，第五份是11月20日《大象无形》604条官方镜像原件的旧owner
解析兼容回放，不冒充本轮WWW联网。两作真实reader四份同章文档已完整返回195/182图，
保留旧首50图的页ID与URL。仍待237代表普通路径复核后再发布；回放不能当作设备阅读通过。第四份截在a属性内的旧捕获继续排除，不以它的碰撞证明缺陷。
证据统一保存于Koma本轮audit的baozi-order-followup；旧原件、0.1.26包和失败结果保留。

## 0.1.26 完整目录续章

完整目录同时读取初始章节区和展开后续区，仍按同作品主线的连续站点槽位校验。
《大象无形》第二十四章和第二十五章跨越两区，从第二十四章继续阅读时可取得第二十五章的续章关系。原章节 ID、目录显示顺序和正文请求保持。详情的章节编号还识别「第二十四章」一类明确的中文数字标题。

## 0.1.22 跨章阅读

“最新章节”是完整目录的重复摘要，顺序与完整目录相反。用同作品完整目录验证站点主线的连续槽位，供阅读器预载及切章。保留章节 ID、目录显示顺序和正文请求。完整目录无法验证时，仍可从目录手动选章。

## 当前修复：0.1.12 章节编号

0.1.11更新后，《大奉打更人》696章中688条有编号，降序已显示687→686→685；升序仍留下8条未知。实际目录确认其中“138”“285”为完整三位裸数字，“624“雞”不可失”为三位数字紧接中文弯引号题名。本版仅补这两种形式；“放料”“預告”及通告、通知仍保持未知，日期、卷季页等排除继续保留。

只补章号，不改章节ID、DOM顺序、去重、阅读进度或App未知章号/同号排序规则。十项编号模块测试通过；0.1.12远端采用、完整目录及Native排序尚未验收，不扩为整书或正文通过。

## 历史调查与验收记录

以下按当时版本记录，旧候选和失败结论不代表当前版本的完整可用性。

### 0.1.3：已有列表默认设置

基于独立生产候选 online.baozimh.koma 0.1.2，而非主树0.1.0原型。
仅 get_manga_list：region/state 缺省或 all 时采用 settings.defaultRegion/defaultState；具体非all筛选优先。设置仅接受原schema枚举，未知设置回到all。无设置/默认all的原URL及头逐字保持。
home四个固定推荐和search职责、正文解析/URI/请求头、图标原字节均不改。

已有403 challenge_required与5xx回执继续保留；本轮0网络请求，未找到可重放的完整成功原HTTP体。生产WASM+真实C++Host只以明确controlled503探针验证query、默认请求及错误语义，不宣称页面解析、当前可达或Native恢复。未部署。

未部署013 Home错误修订：四栏任一HTTP/传输错误按原分类立即返回；HTML解析失败明确parse_error，不继续后续栏。成功空DOM仍保原空栏目成功。只修错误传播，不含栏目id/listingId、500预算或章节slot变更。生产Host仅受控合同比较，无原站成功体、无新网络/Native，不部署。

Home栏目身份修订：仅四原栏目增加home:popular/cn/jp/kr稳定id，无listingId、新列表或更多按钮。原标题/顺序/条目/四请求及错误传播保持，原App按其现有栏目键消费；受控单卡片仅synthetic，不宣称实站或Native可用。

正文完整性修订：保留现500图预算；超过预算明确未完整错误，不将前500图冒完整成功。选择失败、src读取或写入失败立即报错，所有实际分配子句柄统一回收后关闭根。真正0图仍保原成功语义，正常URI/顺序/slot/HTTP保持。500缺上游依据仍OPEN，错误不等于支持长章或全源修复；验证仅synthetic生产Host，未联网部署。

正文容量修订取代上一500图错误预算：移除500图/512字节src及正文4000句柄限制，count-only后复用原BODY缓冲收句柄，原HTTP_OUT缓冲读取完整src。只保当前Native完整响应1MiB及既有实际缓冲容量，超容量明确未完成错误并清理；原四位页ID保持，>=10000保完整整数不回绕。无共享SDK/App/HTTP/slot变更，不新增buffer或网络，不部署；原站成功/Native仍未证。
原离线runner的Attr桥另有1MiB缓冲guard（Native C++无此字段cap），首2MiB尝试的失败已保存。现读取slice取Native完整响应1MiB；恰满URI连同任何JSON开销必超真实响应容量，明确报错，不将guard上限当512新字段协议。

本组合候选保留已审Home句柄所有权及详情全部tag，合入章节count/完整响应处理和未知章节language省略。分支非线性，原各候选冻结不改。全部HTML回放为受控输入，不宣称上游成功或Native可用；既有403不重试。标题/href等其它字段边界仍OPEN。

## 237 0.1.3 普通入口失败（2026-10-23）

用户新报安装后无法浏览，本轮只从普通Browse打开一次现装0.1.3：get_listings、get_filters成功；get_home首个GET在888ms收到HTTP403、98字符响应，源返回permission_denied。WAMR调用完成，无已观察到的trap或HTML解析失败；App却显示“源运行失败：source_runtime”，属于错误展示丢失分类。403只证明该请求被拒，未读取响应正文，不据此命名具体挑战服务，也不宣称首页或正文可用；未点重试、修改请求头或补请求。

原件位于Koma既有237 category-membership的`baozi013-runtime237-entry-001/{hilog.txt,screen.png,layout.json}`；`baozi013-runtime237-restored-001`读回原Browse喵趣锚点、AWAKE/10000ms及本轮两scratch不存在，租约已释放、哈希关闭。未改源代码、图标、版本、App或用户数据。

后继可读错误复核：237明确覆盖安装App 0.1.2-pre/code3的8381854B Release后，仅普通打开013一次，仍收到HTTP403、98字符、1064ms；未点重试或改请求。画面改为“书源网站拒绝访问，可能需要网站验证或登录。”，不再source_runtime，源业务仍失败。真实同链get_home操作与HTTP请求均有起止；源操作日志reason/error=permission_denied、kind=access，但http=0，Browse日志reason=unknown/http=0，故不宣称403状态已跨封套传递或已显示。原件`baozi013-readable237-{installed,error}-001`；后者保留完整失败原图、布局和有界hilog。

## 0.1.4 保留实际HTTP错误状态

只让fetch_html的HTTP失败携带原status，并在原错误封套新增error.status；原code/message、URL、UA、15秒时限、传输失败重试、HTML解析和成功输出保持，不带原响应正文或头。该修改修复App无法从013封套获取HTTP状态的诊断缺口，不修复站点拒绝。Cargo/Info/lock013→014，图标和其他源保持。

标准单源offline/locked/WAMR兼容构建与27337B三成员正式包通过。真实013/014 WASM受控Host共137项：六网络operation的400/401/403/404/408/429/499/500/503/599及status/statusCode两种输入，仅error.status新增，其余原封套和请求对象/次数保持；无状态/传输失败保原错误，无伪造HTTP状态；get_listings/filters/settings及受控200空DOM的Home/list输出保持。没有在线请求，不把空DOM受控成功当实站解析通过。

237普通013→014更新后仅打开一次原入口，HTTP403/98字符/883ms；完整画面显示访问拒绝并带“HTTP403”。同一op链get_home与source_http有起止，源操作reason/error=permission_denied、http=403、kind=access，Browse的error/http/kind同值但reason仍unknown。原件`baozi014-readable237-{picker,imported,error}-001`；error含完整原图、布局和有界hilog。源业务仍失败，原98字符拒绝正文及响应头未保存，具体拒绝主体/原因尚未知；不能据提示中的可能性断言需要登录或验证。

后继限定根因取证：临时App桥只对准确classify的403短JSON提取公开primitive错误字段及Content-Type/Server，237普通入口一次实际读回`error=challenge_required`、`Content-Type=application/json; charset=utf-8`、`Server=nginx/1.24.0 (Ubuntu)`；同请求403/98字符/898ms。原件`baozi014-rejection-probe237-001/hilog.txt`及完整screen/layout。该字段说明服务端要求挑战验证，不从Server名推断安全厂商，也未查明触发策略；没有处理/绕过挑战、改头域或点重试。临时诊断与正式014状态透传分开，不将失败文案改善当业务恢复。


## 0.1.4 网页会话声明与237普通正文（2026-10-23）

包内增加准确origin声明 `webSessionOrigins: ["https://www.baozimh.com"]`；构建只将本地声明写进manifest，不包含Cookie、登录态或通配地址。是否使用网页会话仍由App逐源征得用户允许，声明本身不授权自动共享凭据。App修复原worker遗漏原生运行身份后，本轮原生请求能够取得这份已授权会话；原403并非HTML解析失败，先前短响应已明确给出`challenge_required`，触发挑战的完整站点策略仍未知。

237普通入口本轮首页实际显示12个封面，《紹宋》详情258/258章；打开“人設物料”后首图1/6为实际角色设定正文，自然返回详情立即显示“继续阅读 · 人設物料 · 第1/6页”，再返回到“熱門漫畫”。同链get_manga/get_chapters以native身份、session=attached实际HTTP200；get_pages因不同origin按边界不附会话（session=none），仍实际HTTP200并完成；6张图片请求均HTTP200。只接受本作这次首页、详情、首图及自然返回链，不等于其余5页已逐张看图、全站/下载/冷启动或所有源会话通过。

原件沿Koma既有237 category-membership的`baozi014-native-context237-{settled-002,detail-003,reader-004,return-settings-005,full-log-006}`。`return-settings-005/failure.png`来自测试脚本错误期待不属于该设置sheet的“断开网页会话”；原图实际为正确源设置（預設地區、預設狀態），不记成产品失败，也不宣称设置保存已验证。此前identity=none的403和错误提示/状态透传取证继续保留，不重试覆盖。

## 0.1.7 明确网页错误信号

保留 HTTP 401/403 完整 JSON 响应中顶层 `error` 明确返回的 `challenge_required` / `auth_required`，以及原 HTTP 状态，供宿主区分网页挑战和登录要求；普通 403 不猜测为 Cloudflare 挑战。未知、嵌套、畸形或歧义信号继续保留原 HTTP 错误分类。

## 0.1.8 数字前缀章节编号

197 官方 WebView《紹宋》最新章节区实际显示“215 如之奈何”“224 潰敗”等11条标签，官网原序也非数字降序。本版仅增加严格“数字＋空白＋非空标题”的 `chapterNumber` 提取；原 ID、请求 slot、目录返回顺序和网络保持。日期格式、无标题裸数字、夹杂数字及溢出值不认作编号，其他未知编号仍为 null。原第N話/话识别与精确重复合并保留。证据仅覆盖上述可见11条，不声称完整234章均已核查或排序实机已验收。

## 0.1.9 三位补零章节标签

承接197《紹宋》0.1.8排序复验的新边界，仅补三位补零纯整数与直接连接中文标题的编号，例如“045”“006扶腰壯膽韓良臣”。日期、卷季页等单位及其他歧义形式继续未知，不将任意裸数字泛化为话号。原 ID、请求 slot、正文请求、目录原序和进度引用保持；已有明确话号及带空白标签规则保留。这两种标签来自主线程实际观察，本版生产解析器行为验证不替代后继安装及完整目录实机验收。

## 0.1.10 每章一次的有界 CDN 选路候选

- **依据**：197《大奉打更人》`chapter:dafengdagengren-chuyingshe:2` 的目录与24页解析成功，图片在302后TLS失败。当前同章网页仍给24张图；其真实 `page_runtime_v5.js` 明确列出CDN候选与选择机制。匿名Mac原首图及追加`t`均重定向到失败节点；网站所列rsa节点的同首图GET为200/JPEG/2500×4482，随后同URI HEAD为200、空正文。不是403或Cookie问题的证据，也不保证所有节点/章节可用。
- **候选行为**：只在`get_pages`内先对原首图HEAD，失败后按网站level降序、同级原声明顺序尝试有限候选；每项一次1500ms、首个200且空正文即停止，最多6项。无计时/并发ABI，不冒称浏览器竞速。只替换同原authority、同作品章节目录的严格HTTPS `s数字`官方后缀，保ID/index/页数/path/query；未知host、混合章节和全部失败时保原URI。取消仍返回取消；`get_image_request`不探测、不加凭据或权限。
- **本地验证**：实际候选WASM与生产C++ HTML Host/Lexbor在保留96271B原HTML上离线回放13项通过，另纯选择模块4项通过；覆盖默认成功、继续至rsa、全部失败、非空HEAD正文拒绝、取消及混合URL。每次新`get_pages`重新选择；换URI可能新增缓存项，不清旧缓存。
- **Native有限结果**：197的342普通导入0.1.10后，《大奉打更人》详情695章成功，首章“001牢獄之災”第1/24页完整显示。`get_pages`完成5004ms，3次HEAD前两次失败、第三次200/空正文，日志为`probes=3 outcome=fallback`，随后6张实际图片均HTTP200。
- **翻页与重开**：347沿346确认的“纵向分页/从上到下”普通滑动依次到第2/24、3/24页；Root亲审349整图为真实第3页。348正常BACK返回同695章详情后，继续阅读重开仍为本章第3/24页。349注册与书架内容同342，reader仅本书同章正常更新到第3/24页。仅接受该首话前三页及返回重开，cold、全章和整书仍未验。
- **宿主兼容边界**：旧公开App使用`method=GET`加`customMethod='HEAD'`；[官方API23+契约](https://developer.huawei.com/consumer/cn/doc/harmonyos-references/js-apis-http)明确customMethod优先于method，不能由枚举写法断言旧App不支持。已亲读公开`v0.1.2-pre`的`build-profile.json5`：target与compatible均为`6.1.0(23)`；当前compatible也仍为API23，已覆盖该API能力，不因这项HEAD写法再抬高源`minAppVersion`。342–349只验当前显式`RequestMethod.HEAD`宿主，旧公开HAP同候选路径未重验。200且空正文是探测成功条件，不是旧包实际发GET的证据。
- **原件**：`/private/tmp/koma-baozi-dafeng-image-20261024/` 内 `page.body`、`runtime-script.body`、`cdn-static-decoded.txt`、`probe-results.json`、`same-first-image-candidate-receipt.json`、`head-rsa-first-receipt.json`、`verify-0.1.10.cjs` 与 `candidate-0.1.10-verification.json`；Mac取样、离线Host和Native证据严格分开。

搜索先解码请求中的 JSON 转义，再按既有站点请求规则编码用户输入；空查询、缺失查询、网页验证和其余行为保持原合同。

## 0.1.25 备份续读与长章节

Mihon 旧目录地址的作品 ID、正文分组和章节槽位对应 Koma 目录中的同章 ID。由包清单声明转换，标题的简繁差异和 ID 的标题后缀不影响匹配。Mihon 的 App 镜像返回整章，Koma 使用的网页镜像每份文档最多提供 50 张新图。按页面链接补齐同章后续文档，并去除分页边界重复的图片，沿用已有图片 ID 和页序。
