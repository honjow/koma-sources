# 阴阳师漫画屋 0.1.0

`com.netease.onmyoji.zh.koma`，`zh`，minimum App 0.1.1，现有 SDK/ABI。独立源，不内置 App。公开平台由阴阳师官方 `yys.163.com/index.html` 实际链接 `comic.163.com`，包含官方栏目与同人作者作品。`official_cartoon` 是栏目标签，不是作者认证；《与君结缘》原作者是一支桃花，其他五项原作者为阴阳师工作室。原 `author_nickname` 独立保留；不把 nested author.nickname 或 authenticated 字段当作出版身份。平台/逐书年龄与使用率 unknown。

## 动态合同与范围

使用官网原 JS 声明的匿名 `huajia.tongren.163.com/article/apps/web/` GET JSON 路由，`isNeedWm:false`。不运行脚本、签名/watchman、登录、凭据或重试挑战。SDK 固定标准 User-Agent，Referer/Origin 为原漫画站。元数据与首章采用原 SSR 的无 query 路线；列表/搜索/目录保留原语义参数，省略 JS Date.now 缓存参数（无 clock ABI，不造固定时间戳）。候选实际七个 URL 各一次匿名正常 HTTP200 对照，元数据另一次HTTP200。实际字节全部保留；目录只有三个 view_count 变化，原37章及1004页计划不变。

Browse 对应原分类：首请求 cartoon/update/span10/start0，成功后 official_cartoon 相同参数；第一屏 official 在前再 cartoon，后续只 cartoon，ID 去重。实证为 6 official + 10 cartoon 首屏、10 个不同 cartoon 次页。首响应声明 cartoon count103，后页 count=-1 是哨兵，不据此终止。候选以原单页实际行数判断下一页，满10继续，短页结束；不会因无 count 或 count=-1 制造终页。未枚举剩余83项，非全平台完成验收。原主页8热门/6新/6官方不是平台总量。

Search 用原客户端两个分支：official 然后 all，span30。保留原顺序，按原 ID 去重；翻页依据原 all 分支实际行数（满30继续），不以去重后行数、缺失count判断终页。原搜索响应是 array，无 count；原客户端却读取 array.count，矛盾保留。已知同样本 query 第一分支0项、第二分支1项。未验证超过30结果的真实查询；原高亮 <font> 是客户端展示加工，候选不插入它。输入沿原客户端最多16个Unicode字符。

详情与目录由实时 JSON 生成，不嵌入静态1004许可表。get_chapters 保留真实CID和显式index（不猜章号），只接受完整 count 相符、唯一CID和递增显式index的目录。pageCount 来自原 body 数组；volumeNumber 和章日期不投射为猜测值（null）。原序列状态只将实际0/1映射 ongoing/completed，其他unknown；accepted 是审核状态。原简介/作者/标题按JSON解码一次，保留实际literal\\n，未给其造换行。章节只接受原image且text_content为空的正文形状；元数据只接受原单text_content段与字符串tags数组，多段未知元数据正文失败关闭，不悄悄截断。未知形状失败关闭。

## 图片成本和身份

每次 get_pages 独立 GET 一章详情，以原 response.article/id/index/status/game 对实际 MID/CID/显式章号核对，保留 body 原顺序/URI/query，生成0起页序。每次 get_image_request 新 Module **再执行一次该章 JSON GET**，严格核对 sourceId、MID、mangaUrl（有值时必须为原作品详情URL）、CID、章号、页序与 byte-exact 原URI/query。封面调用 pageId=MID，独立元数据 GET 核作品id与其原cover字段，不能拿第一张正文充当封面。错源/空sourceId、错owner、错route、错CID、错页、外部或变化URI均拒绝；没有跨调用缓存、没有不存在的cache ABI。成功结果仅声明原URI与Referer，Host随后按原 bytes 取图；每图增加1 JSON GET的网络与取消成本如实承担，无内部重试。

图片形状只支持已实证 `https://g37-manga-resource.fp.ps.netease.com/file/` 下原声明URI，仍须每次精确owner校对；前缀不是许可依据，其他资源域/形状需新合同审查，当前失败关闭。任何 URI 差异、不可取得/非200/上游拒绝/结构异常均失败关闭。结构验证拒绝 malformed JSON、重复对象键、任何转义对象键、混合正文数组；章节数最多1000、单章页最多10000、HTTP/正文/每操作临时首响应各1MiB，容量不够明确失败，非部分结果。

## 原证据与验证

原合同 `/private/tmp/koma-onmyoji-zh-contract-20261014`；列表/搜索 `/private/tmp/koma-onmyoji-zh-range-review-20261014`；首章独立owner `/private/tmp/koma-onmyoji-zh-first-chapter-owner-20261014`。本候选实际 HTTP、差异、production WASM回放与完整绑定 `/private/tmp/koma-onmyoji-zh-implementation-20261014`。

已核一个样本《今天的平安京也很和平》真实37章/1004页计划，独立首章23页全部与目录/SSR一致；唯一已取实体是首张标题/作者画，不代表全章内容或平台年龄判断。未取其他1003图片，未批量37章详情API。R1原始生产WASM的75次离线原响应/明确标识synthetic故障回放，每操作新Module，首章23次image各1 JSON，封面1 metadata；R1的70导出ABI拒绝、40init/20alloc检查。这些证据仍绑定R1 WASM，不当作R2重复执行结果。代理/Wasmtime回放不是WAMR或197 Native验收。Root负责实际集成与Native路径。

## R2 转义键反例修正

旧 R1 生产 WASM 对同一 data 对象 `id` 和 `\u0069d` 不同原 bytes 的同义键未拒绝，已保留实际 accepted 反例，不能称其重复键门控完整。R2 仅在对象键完成结构扫描后拒绝任何含 JSON 转义的 key；已知原 schema 的键均为非转义字面键，未知转义 key 失败关闭。正常字符串值的 Unicode/换行/引号转义未变。仅复验同一 synthetic 反例和原真实首章 image owner；零新增远程请求/正文实体，不以此声称 Native。旧 R1 树、包、605绑定材料均保持原样。

## 237 当前 0.1.0 具名原生观察（2026-10-21）

现装源经普通 Browse 列表进入《今天的平安京也很和平》，详情及目录显示 37/37 章；普通打开《内卷的平安京》，亲看真实彩色多格正文 1/6 与 2/6。它实际是六页长条漫画，不能写作单张四格 1/1。当前单页适高显示为窄栏，取得正文不代表该模式阅读体验或整章视觉通过。

后续顺序翻页命令曾推进至第六页，但未逐页做视觉验收；自然 Back 回目录，该章显示已读。自然保存的 reader 记录绑定原 MID `5dafb0b0a14fb93e15d3de51`、CID `60e7b40fcc26f015d1de4766`、章号 37，pageIndex=5 / totalPages=6 / completed=true；此记录不能替代未审各页的可见内容。没有直接编辑持久状态、收藏或下载，也没有改变阅读偏好。

原 Browse 末端已亲看恢复，50 项源 registry 与进入前完整相同（包括 ID、enabled 和顺序），原 OverrideTimeout=10000ms / AWAKE 已读回。原件在 Koma `.hvigor/outputs/user-flow-audit/device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership/` 的 `onmyoji010-peace-{detail,chapters,involution}237-001`、`onmyoji010-peace-involution-page2-237-001`、`onmyoji010-natural-return237-001` 与 `onmyoji010-session-restored237-002`。只接受具名章首两页及自然返回，不扩大到全部六页、其他作品、下载、冷启动或整源通过。

Root 已亲审本条首两页原图及精确记录；结论仅限上述具名路径，原未闭边界保持。
