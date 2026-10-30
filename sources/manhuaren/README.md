# 漫画人

支持章节阅读序能力的宿主按站点原数组和有效 sectionSort 组织续读；锁章仍保留序号，缺少序号的章节不自动续读。

源 ID `com.manhuaren.koma`，版本 0.1.6，API 0.2，要求 Koma 0.1.4 与 `source_private_session_v1`。业务请求使用站点正常游客初始化与私有会话；游客令牌不写入公开设置。未声明该宿主能力时业务操作返回 `host_feature_required`，不发送业务请求；作品和章节身份保持。

## 237 普通安装与独立搜索（2026-10-21）

在原主仓用既有 Rustup 工具链和标准 WASM 参数执行 offline/locked 构建，真实 WASM SourceInfo 为 0.1.1。使用原正式图标与 metadata 生成三成员正式包，经普通系统 picker 导入成功。237 原50项 sourceId/enabled 相对顺序逐项保持，正常新增漫画人后为51项；没有更改 App 容器、布局或源业务。

普通热门入口实际执行 get_manga_list，14:24:14.322 HTTP200 后于14:24:14.397返回 permission_denied。当前源码仅在上游 errorResponse.code=40011 时将此结果映射为“source requires an anonymous session”；没有重试该热门请求，也没有据此判定独立搜索失败。

从源包管理动作单直接进入独立 SourceSearch，未重进热门。原 Host 具名词“一拳超人”只输入一次，经已有自动提交后，14:34:49.109执行 search，14:34:49.269 HTTP200。原生结果实际显示《一拳超人》《一拳超人同人：琦玉VS龙卷》《一拳巫女》等标题与封面；收起键盘后至少六个封面/标题可见。只接受这个词的首批搜索及可见封面，不声明全部20项、分页、其他词、详情、章节、正文、下载或cold通过。

该次未点击作品或调用 createAnonyUser2 临时匿名握手。原详情/目录/正文仍会调用 ensure_auth，该次Native分支未验；既有静态匿名协议未被改写或绕过。后继Host详情证据见下方012记录。

全部原件在 Koma/.hvigor/outputs/user-flow-audit/device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership/：manhuaren011-imported237-001、manhuaren011-public-list237-001、manhuaren011-public-search237-001、manhuaren011-search-visible237-001。搜索会话最终恢复证据为 manhuaren011-search-session-restored237-001/run-metadata.json：普通返回原 Browse 末端，reader.json 与 Root 原 comicabc010-root-session-restored237-004 逐字节相同，原50项顺序/启用保持，新源保留；AWAKE/OverrideTimeout=10000ms 读回后释放 d57d478a。前一安装会话恢复回执为 manhuaren011-session-restored237-001，acb7d798 已释放；仅本次上传包被移除，所有设备回执哈希关闭，没有清用户数据或缓存。

## 0.1.2 真实作品网页链接（2026-10-22）

原 get_manga 的两条 source 链接自初始0636f19继承，分别是API地址及由ID拼出的网页路由。当前候选仅改为消费 response 直属、同 mangaId 的显式 shareUrl：只接受已实证的 https://www.manhuaren.com/manhua-<非空ASCII字母数字或连字号slug>/，不猜slug、不改域；缺失、错owner、外站、HTTP、首页、query或未知形状时 links 为空，仍保原成功详情。请求/鉴权/身份/章节/图片/图标/metadata/能力和build登记不变，只有SourceInfo、crate及对应lock版本步进012。

此前仅凭 createAnonyUser2 名称把此分支当作用户账户门槛并不成立。已核源码：返回的匿名userId及授权值只放本次WASM静态数组，无storage/凭据接口、用户注册/登录或验证码动作；服务端匿名对象寿命未知。本轮正式011 WASM对准确 mhr:10684《一拳超人》按原合同仅一次匿名POST（200/900B）及一次详情GET（200/3088B），无重试/挑战；详情明确 shareUrl=https://www.manhuaren.com/manhua-yiquanchaoren/。沿该显式URL一次正常公开GET得200/30894B、无重定向，真实标题/简介对应同作。匿名授权响应未落盘，令牌/签名/设备标识未记录或显示。之前单次公开搜索200/9718B只有封面URL，不能代替这次详情证据。

原011与候选012实际WASM以同份真实详情（敏感字段脱敏）及明确合成的离线匿名握手回放：非links响应字段深等，完整两请求在进程内逐项相等；请求中敏感值未落盘。012 offline/locked标准单源构建成功。缺shareUrl、错owner、外站、HTTP、首页、旧猜ID路由、query、仅嵌套同名字段、非字符串与仅嵌套owner这10项均为合成异常，结果仍成功详情且links为空，不冒实站失败。原件在Koma .hvigor/outputs/user-flow-audit/manhuaren011-detail10684-{response-redacted,normal-session-safe-meta}.json、manhuaren011-share10684-once.{html,json}、manhuaren012-{baseline-offline,candidate-offline,link-compare-offline}.json及build.log。该离线阶段没有设备或正文验收；后继普通012交付如下。

237普通picker从011更新012明确成功；43660B正式包恰manifest/source.wasm/icon.png，完整三语localizedNames、nsfw=false/minApp0.1.0及原104px图标保持。Manager独立搜索“一拳超人”一次→准确作品详情显示同作封面、村田雄介/ONE与简介；两网页菜单均可见，普通WebView真实显示一拳超人标题、作者、简介及网页当前封面，初始白屏加载图保留。自然Back详情仍0/0章、阅读/下载灰置，再Back原查询/结果保持，未阅读/下载/外部浏览器/热门/刷新。55项有序ID/enabled保持，原Browse顶部及索引恢复，唯一传包已删，10000ms/AWAKE读回并release469021cd；未导出私有业务全JSON。原件在Koma category-membership/manhuaren012-import237-001/{import-result,manager012,search-result,detail-current,webpage-menu,webview-initial,webview-current,native-return,query-return,browse-restored}.png及manhuaren012-restored237-001/。

0章不算可读通过：同租约当前进程安全标签事后有界读取stdout/stderr均0B，不能据此断言HTTP/章节成功或错误。现get_chapters仍请求同getDetail并读mangaEpisode/mangaWords/mangaRolls；已有同实体真实详情三数组全空。正式012 WASM对该保留响应离线回放实际ok:true/items[]。同一真实响应明确 mangaHideReason=“抱歉,该漫画已经被屏蔽”、authority=0；其readMode建议外站搜索，未跟随。该保留样本已能解释自身空目录，但未保留Native当次完整响应，不冒逐字绑定。网页HTML的成年确认只是隐藏通用模板，当前DM5_ISSHOWADULT=False，不能冒成本次原因；不重复API、不绕屏蔽或夹带章节分类修改。安全日志manhuaren012-safe-log237-001/run-metadata.json；离线原件manhuaren012-chapters10684-retained-offline.json。

搜索先解码请求中的 JSON 转义，再按既有站点请求规则编码用户输入；空查询、分页和其他已有行为保持不变。
