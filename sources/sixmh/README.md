# 六漫画源候选

规范源 `com.sixmh.koma`，中文内容，按当前 Keiyoushi 元数据为 NSFW；实测用量未知。实现参考：Keiyoushi extensions-source `10f06eb833af5990bc5470c84c5e93edc159ad4e` 的 SixMH.kt/Crypto.kt 与 MCCMSWeb.kt/config（上游 Apache-2.0）。源合同与有限原生验收边界分别记录如下。

热门/最新使用规范公开 PC HTML 路由；详情/目录保留规范 href ID 和目录顺序。章节编号/日期保持未知。搜索沿固定 MCCMS 参考移动端请求合同 `/search/{query}/{page}`。源对 JSON 解码后的查询做百分号编码，保留目录 ID，识别站点的显式空结果，对未知或挑战响应显式失败而不是返回假空列表。可选筛选/设置/登录为 false。缺失/含糊/溢出/验证码/错误结构显式失败。

章节参数含 Base64 IV+密文；RustCrypto aes 0.8.4/cbc 0.1.2 解密元数据 URL JSON，带严格填充/UTF8/完整 JSON 校验。图像字节从不变换；无 JS、会话、挑战、账号或付费墙路径。父级 PC User-Agent 和省略 Referer/Origin 是初始公开合同，不是拒绝重试。AES/CBC 许可为 MIT OR Apache-2.0，版本/校验和已锁定。

有限日志只含静态阶段/操作/错误标签和数字 HTTP 状态；无 URL、查询、标题、cookie 或内容。

## 有限原生验收（197，2026-10-10）

精确的 0.1.0 归档 `6faa4081b099667a1bd324c6256f3b683b955cdd3ae03cd1ac72697dccb49ca8` 和 WASM `800213e75c28d507e41294a3cb0ffacd4a5d5b225bc9d44c5c3fc796edbfbe5b` 经普通订阅路由安装并逐字节读回。普通热门分页把 30 追加到 60 个条目；初始可见封面显示了真实图像。公开非成人《鬼灭之刃》`/255429` 加载 212 章；显式 第1话 残酷 `/255429/58497.html` 返回 54 页并显示真实第 1 页和第 6 页。单章下载完成 54/54；54 个非零物理文件共 9,009,709 字节，每个大小与清单匹配。不清数据冷启动保持全部四个状态文件逐字节不变，普通书架继续显示同一真实第 6/54 页。所有原始源、书、阅读器和队列对象都保留。

这只验收具名公开路径。搜索保持禁用；最新列表、终点目录/全站覆盖、登录/付费内容和断网设备阅读未证明。站点 HTML 本身把这个样例标记为 爱情/耽美；这些值不是从标题虚构或纠正的。源保持未知章节编号为 null，但 App 当前用源索引替换它们；排序语义另行等待用户选择，此处未改。Root 证据保留在 Koma 生产树 `.hermes-artifacts/20261005-production/src014-sixmh-20261010/` 下。README 验收说明不改变冻结的模块/包字节。

## 当前标准交付候选（2026-10-18）

完整的原 49780a37 业务实现、crate 身份/版本和本地化名称保持逐字节不变。当前规范 SDK 和标准构建生成新 WASM；这不继承历史 Native 验收。上面历史 App 索引回退是带日期的观察，不是当前消费者需求。

Root icon.ico 是未处理的站点声明 32px 原件；ASSET_SOURCES.md 记录 URL、声明和 SHA。生产构建校验必需图标并导出其实际文件名。小槽可读性和当前 WASM Native 路径保持未验证。本候选不含新上游请求、图像处理、解析器变化或设备工作。

## 0.1.1 传输分类（隔离候选）

只有正式宿主 ok:false/error.code=network_error 封套且无 status 也无 statusCode，或 HTTP 宿主 ABI 失败，才报告为 network_error。未知/无效状态和所有既有 HTTP 非 2xx 分类保持显式失败。既有成功解析、完整体校验、UA、超时、TLS 和 SDK 不变。首次当前标准 Native 热门尝试在 TLS 失败（CURL35/RespCode0/零字节）；没有到达详情/正文/下载/冷结果，这个分类变化不修复也不证明远端连接。受控封套不是原 Native 回调输入。

## 2026-10-21 237现装0.1.2《一人之下》有限Native结果

当前VDE-AL00展开竖屏1320×2120，普通Browse实读六漫画v0.1.2，未更新源包、App或偏好。沿普通热门入口返回30本，19:07:04请求HTTP200；只打开可见《一人之下》，19:07:29 get_manga HTTP200/body_ready/html_ready，实际封面、动漫堂、简介可见。独立get_chapters于19:07:30同样HTTP200/body_ready/html_ready，随后准确日志为 `sixmh error op=get_chapters code=source_error`。实际画面显示“详情已加载，但章节加载失败。”，开始阅读和下载章节按钮为灰；未取得目录、未进入Reader，首两页正文未验。仅凭HTTP200和source_error不能确定目录失败原因；未留存该次原始HTTP体，不能猜成站点挑战、选择器变化或容量问题。

本轮没有另行Host网站请求、重试、换标题、UA/Cookie/TLS/方法变化，也未请求此前已403的首页或 `/255429`。旧010鬼灭具名正文接受及旧标准TLS失败保留各自版本边界，本轮不扩为全站可用或全部详情失败。经普通Back返回原Browse末端，原OverrideTimeout10000ms及AWAKE读回，租约96009100已释放；所有回执artifactHashesEnabled=false。未导出registry、阅读会话或偏好私有文件，不宣称全库持久内容不变。

原图、布局及精确日志保留在Koma原场景 `.hvigor/outputs/user-flow-audit/device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership/sixmh012-yiren-{awake,browse-top,browse-middle,browse-entry,popular,detail,session-restored}237-*`；决定性失败为 `sixmh012-yiren-detail237-001/screen.png` 和 `run-metadata.json`，恢复为 `sixmh012-yiren-session-restored237-001`。本条只补原有验收账，不修改源实现、样式、版本或既有图标。

后继受限诊断仍为0.1.2，仅追加固定白名单token日志，普通同版本导入后对同一作品取一次目录。19:44:38精确读回 `sixmh chapters failure=chapter_structure_missing`，将失败收窄到现有目录选择器返回0节点；未取得原始HTML，实际DOM差异仍未知，不能据此改选择器或宣称已修复。原件为同场景 `sixmh012-token-yiren-detail237-001/run-metadata.json`。237已普通恢复Browse和10000ms/AWAKE，本轮两处上传已清理、租约已释放；未改请求/版本/图标/错误封套。

## 2026-10-21 下架分类候选与独立正常样本

此前单次公开热门取样得到两个同名《一人之下》href `/260264`、`/236126`；各自单次详情HTTP200，准确状态span显示“已下架”且目录选择器为空。它们未与19:44设备作品ID绑定，不宣称该次设备一定请求其中一个ID；原HTTP体未落盘，不把后续控制用例称为原体回放。候选仅在原目录0节点时，沿现有get_manga状态合同（`div.cy_info`内首个`div.cy_xinxi`的`span:nth-child(2)`）明确含“已下架”才返回既有`public comic unavailable`/`not_found`；状态缺失、读取失败或其他状态仍为原`chapter structure missing`/`source_error`。有效目录优先，简介中的同词不参与判断；不增加请求或改目录/身份/页数、UA/TLS/容量/状态枚举/App。

现装仍为含固定安全token的0.1.2诊断包，本候选尚未安装。237普通切“更新时间”，真实首项《水行群侠》详情5/5章，第01卷实际产出23页；P1/23完整彩色封面、P2/23“曼哈顿”中文彩色分格正文已目视，普通Back仍为5/5目录，滚回详情顶部只有“继续阅读”，未显示章页文字。仅接受此具名前两页，不冒全23页、其余卷、下载或cold；未导出私有进度文件，不能声称持久进度字段已实读。原列表选择已经普通菜单恢复热门人气动作，最终Browse末端原图与OverrideTimeout10000ms/AWAKE读回，306569fc租约已释放，哈希关闭。原件在Koma同一category-membership场景的`sixmh012-aquaman-page1-ready237-001`、`sixmh012-aquaman-page2-237-001`、`sixmh012-aquaman-natural-return237-001`、`sixmh012-aquaman-return-top237-001`与`sixmh012-aquaman-browse-end-restored237-001`。

限定候选已离线locked release编译，并读回真实WASM元数据仍为com.sixmh.koma/0.1.2。WASM实际导出经受控host ABI结构/分支检查：空目录+准确下架状态→not_found；有效目录即使下架状态也保持原章ID/index/title；空目录其他状态、仅简介提及下架、状态缺失、详情卡缺失均保留source_error，每例仅一次模拟HTTP回调。这是控制用例，不证明站点DOM解析或候选Native验收。首轮测试替身缺bodyText/ok字段及误读sourceInfo层级均已更正，没有据此修改生产合同。

## 0.1.3 限定Native复核（237，2026-10-21）

Root审过分类与版本diff后，仅SixMH Cargo、内嵌SourceInfo及对应lock包项升0.1.3；正式本地.koma为manifest.json/source.wasm/icon.png三成员，图标原字节保留，无发布index或哈希计算。20:25:31普通picker导入日志为`import_ok id=com.sixmh.koma`，普通Browse实读v0.1.3，原件`sixmh013-imported237-001`及`sixmh013-browse-version237-002`。当前相邻来源位置可见，但未导出registry，不能宣称全序或全库内容相同。

普通“更新时间”首项水行群侠仍5/5章，打开第01卷自然显示既有P2/23，左翻P1/23完整原封面，再右翻P2/23“曼哈顿”中文彩色正文，普通Back详情再Back列表；原件`sixmh013-aquaman-detail237-001`、`sixmh013-aquaman-open237-001`及`sixmh013-aquaman-page{1,2}-237-001`。只覆盖该卷前两页及本次返回，不冒全23页、其他卷、下载或cold。列表经原菜单恢复热门人气，30本及首个一人之下具名可见，原件`sixmh013-popular-restored237-001`。

对该热门首个一人之下仅一次普通打开：20:32:53 get_manga/get_chapters均HTTP200，章节准确日志`code=not_found`、`failure=public_comic_unavailable`；实际画面仍为原“详情已加载，但章节加载失败”，阅读/下载灰置。原件`sixmh013-yiren-notfound237-001/{screen.png,run-metadata.json}`。这验证了空目录准确状态分支的真实Native分类，不表示下架内容恢复；未重试、未另开第二个下架ID、未触已403首页或/255429、未改UA/TLS/请求合同。

最后普通Back回原热门列表，再回Browse末端；两处仅本轮013上传路径已删除且读回不存在，10000ms/AWAKE读回、9acb0ad0租约已释放，哈希关闭，原件`sixmh013-session-restored237-001`。首次上传位置未出现在picker，按原回执改用公共Docs/Download后才导入；恢复manifest首轮因不支持allowFailure字段在动作前校验失败，删除该字段后同协议完成，未重导或重复正文。没有私有状态导出、App/样式变更或提交推送。

## 0.1.4 网页链接候选

`get_manga` 声明一个 `kind=source` 链接，使用与其既有 GET 相同的 BASE + 规范化公开详情路径。保留的 HTTP200 最新列表父级明确链接了 `/620554`（罗梅莉娅战记~伯爵千金，打倒魔王之後发现人类处境实在不妙於是组建军队~）。一次普通匿名 GET 用不变的 PC Firefox150 合同在 `https://www.liumanhua.com/620554` 返回 HTTP200/29294B，无重定向，匹配作品标题。没有 canonical/og:url；这个链接就是成功的原详情请求 URL。请求、身份、章节解析、下架目录分类、图像、权限和图标都不变。

实际 0.1.3 和 0.1.4 WASM 在禁网下回放同一保存的作品 HTML：两者都成功，所有非 links 的 manga 字段和唯一 HTTP 请求深比较相等。离线锁定单 crate release 构建通过。那个离线阶段没有产生包、索引、哈希、设备工作或阅读验收。此前 `/255429` 和首页 403 保持停止；本工作与旧鬼灭和水行群侠阅读证据独立。证据在 `Koma/.hvigor/outputs/user-flow-audit/`：`sixmh014-latest-parent{.html,-receipt.json}`、`sixmh014-{detail-once.html,detail-once.json,baseline-offline.json,candidate-offline.json,link-compare-offline.json,build.log}` 和 `sixmh013-baseline.wasm`。

## 0.1.4 更正包网页验收（237）

在 Root 签名的 App（16317008B / 2026-10-22 01:23:08）上，普通 picker 安装了 0.1.4。第一个本地包错误地声明了 `nsfw=false`；其已装 manifest 和初步图标证据保留。Root 只更正了那个 manifest 字段，普通同版本 picker 重导安装了 197542B 包。已装 manifest 与更正后的包完全一致（`nsfw=true`、`contentRating=nsfw`）；全部 54 个源 ID、registry 顺序和启用值在更正前和最终返回时保持相等。管理/浏览同组原图标保留批准的 1.05 资源缩放。

普通更新时间列表暴露了精确的 `/620554` 作品。`get_manga` 和 `get_chapters` 各返回 HTTP200；原生详情显示 3/3 章和开始阅读。两个网页菜单项都出现。WebView 显示了同一作品的真实封面/标题/作者/三条目录项；自然 Back 保持原 3 章、3/3 目录和开始阅读。没有打开章节、Reader、外部浏览器或下载；旧鬼灭/水行群侠阅读验收独立。原热门人气选择和 Browse54 恢复，本人传包删除，10000ms/AWAKE 读回并释放租约。

原始证据在 Koma 现有 `category-membership` device237 分区：`sixmh014-{import-icons237-001-gate,manifest-before-fix237-001,corrected-import237-003-restored,manifest-corrected237-001}`、`sixmh014-exact620554237-001-{gate,restored}`、`sixmh014-webview620554237-001-gate/webview620554.png` 和 `sixmh014-return-restored237-001-{gate,restored}`。这只验收具名更正包的网页/返回和图标路径，不做全站或新阅读声明。

搜索 0.1.5：匿名生产 WASM 对一人之下返回两条结果，漫画第 1 页和第 2 页带推进的游标，末页无游标，zzqxv987654321 显式空结果。特殊字符查询返回网站 JSON“搜索繁忙”并作为错误呈现；Native 验收保持独立。

## 0.1.8：长章节目录

与其他 HTML 列表共享的 2,048 描述符数组拒绝了三个标题为全职法师的在线搜索结果的整个章节目录。章节选择现在有独立的 8,192 描述符容量。源仍按站点顺序返回所有章节及其原始 ID，从不静默截断；既有运行时响应大小限制仍适用。搜索、详情、页请求和显式不可用结果不变。

四个在线目录含 3,277、1,025、2,206 和 2,197 个条目。本地 WASM 和原生 HTML 回放匹配每个条目，1,025 条目目录保持旧结果。Pura X 237 运行 Koma 1.1.1，普通包导入和源搜索后显示全部四个计数。3,277 条目目录中 01 世界大变的前两页渲染，阅读器返回详情页。

## 0.1.9：显式待浏览章节列表

当既有章节列表为空、站点现有列表元素无子元素、最新章节链接 href 为空且恰好为“待浏览”时，报告 `no_chapters`。显式“已下架”保持 `not_found`；缺失列表、无法识别的子元素、缺失链接或任何其他标记保持 `source_error`。这防止未知布局伪装成确认的空目录。既有有效目录顺序和容量不变。

## 状态、排序与标签候选

固定参考 `949dea252eb8e867b7b40834a26e13088764ff55` 的 MCCMSWeb 将空关键词筛选作为独立搜索：状态 all/finish-1/finish-2，排序 hits/addtime/score，标签由官网分类页动态取得；带关键词搜索忽略这些筛选，热门和最新保持各自原入口。候选仅在宿主支持 scope、动态选项和空词筛选时提供该 schema；旧宿主保持空 schema 和空词拒绝。动态选项初次不阻塞列表，实际展开时读取官网标签。

2026-11-06 真实分类原体包含100个标签；校园热门前两页各30项、站点末页40为4项并终止，连载/评分/校园为1项终止，完结/评分/科技n为空且无提示文字。空页只有在站点分类控件和当前页身份均匹配时接受，挑战页失败。原热门、最新、非空词搜索同一真实 HTML 上与正式 WASM 返回逐字段一致；动态失败、非法值及旧宿主经回放验证。Koma 1.2.0 在 237 验证了打开动态标签、选择校园、实际续页和重置为全部。其余标签与旧宿主兼容按网页与本地回放证据记录。
