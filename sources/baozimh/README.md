# 包子漫畫 0.1.3：已有列表默认设置

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
