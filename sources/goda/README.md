# GoDa漫画

稳定源 `org.goda.koma`，运行时版本 0.1.6，API 0.2，中文。
与已装包子身份独立；目录重叠未知；无别名或用户记录迁移。

默认请求后端保持 baozimh.org；单独声明的网站首页保持 godamh.com。显式镜像选择见下文。固定 Keiyoushi `1319949ca9bc456908196e3ba2d104e94182c5d6` 叶 GoDaManhua、父 GoDa 和 Dto 公开解码器，Apache-2.0。上游 SAFE=1；单内容和实际用量未知。源排除广告、下载和外部英文链接。

HTML 热门/最新/搜索和详情、完整公开章节 JSON API、静态标记/重排/字母/Base64URL 图像地址协议；无 JS 执行、账号/cookie/挑战处理、自动镜像探测或图像字节解密。固定 SDK Firefox121 和发布 Referer/Origin。章节反转原完整 API 数组，保留每个原标题/slug/ID，从不把 API 顺序或序号用作章节编号。宿主索引只描述返回序列。页保留原数组顺序；API 顺序不是零基索引。

严格完整 UTF8/JSON/体和输出容量：1MiB 体和 1MiB 载荷，8192 章节，2048 HTML/页选择，有界单独文本/属性。溢出/缺失/无效数据错误，从不部分成功截断。实际最终封套必须适配 native 1MiB 运行时限制。日期字段省略而不是推断。

当前生产入口：`bash build.sh --source goda` 带当前规范 SDK/正式 CLI 和必需图标。历史 package_candidate.py 只保留在旧所有者，不导入规范。下面记录的历史 crate/运行时拆分早于当前生产声明。Cargo、sourceInfo 和包现在一致声明 0.1.6；crate 身份保持 koma_goda_source。最低 App 版本保持 0.1.0。无分发或设备操作。

证据在忽略的 .hermes-artifacts/goda-20261011。保留的公开响应和生产 WASM 回放是本地合同证据，不是 native 阅读/下载/冷验收。Root 拥有 197 普通用户路径验收。


最新验收后继是 0.1.1 原 G 图标更正。用户提供的 godamanga.com 重定向到 m.g-mh.org 并显式声明其 Logo.png；精确原 PNG 保留，见 ASSET_SOURCES.md。这不迁移旧请求后端或别名源 ID。历史 0.1.0 具名武炼巅峰/1扫地小厮 17 页阅读/下载/冷和 0.1.1 图标更新/冷是单独的有限记录；旧后端域和整源边界保持。新当前 SDK 标准 WASM 未 Native 验收。App 行30vp/圆角7/裁剪不变。


0.1.2 隔离候选：保留身份核对的 getinfo API 的完整根相对图像路径，而不是要求首个保留样本的 /scomic/<规范-slug> 存储布局。新保留、唯一匹配的武炼巅峰/夺酒响应含 70 个 /hp/wuliandianfeng/3864/... 路径。API mid/id/slug 校验保持不变；URL 仍只用既有固定 https://c-nd2-1.6wm.top 源，valid_path 仍拒绝网络路径引用、点段、编码斜杠/反斜杠/控制字符、非路径查询/片段字符和超大路径。数组顺序和序号页身份不变；不抓取图像字节或新宿主。旧 17 页正文单独保留。这个候选的页计划和离线结果不证明当前图像 HTTP 可用性或 Native 阅读；原 0.1.1 Native 失败保持。原图标字节和 App 裁剪不变。Cargo crate 保持继承的 0.1.0，运行时/包单独变为 0.1.2。


## 237 现装 0.1.2「夺酒」第 2 页有限结果（2026-10-21）

接续的是原 011 在《武炼巅峰／夺酒》HTTP200 后 invalid image URL 的失败，非旧「扫地小厮」17 页。原 canonical 提交 f0487ab 已修复身份核对后的根相对页路径；本轮未再改解析、版本或图标。现 012 生产 WASM 对精确 `godach:wuliandianfeng-pikapi/12-6059776-3864#12/1748368` 单次 Host get_pages，实际原 API HTTP200 并成功返回 70 页，首 URI 仍既有固定 CDN 的 `/hp/wuliandianfeng/3864/` 路径。原 stdout/stderr 在 Koma `.hvigor/outputs/user-flow-audit/goda012-current-duojiu-pages-host.{json,stderr.log}`；没有重试、取新 host 或图像实体，Host 页计划不冒 Native 正文。

237 普通 Browse 已显示 GoDa 0.1.2，因此没有重新导入。热门 18 本→武炼巅峰详情 3875/3875 章；进入前已存在「继续阅读・夺酒・第 2/70 页」，不能把它冒成本轮首次阅读或过去已亲看正文。沿原继续阅读后，实际亲看第 2/70 页完整彩色叙事画面（人物眼部与飞船构图），21:31:05 原生日志有 ReaderImage display_ready/display_complete page=2。该页缺足够可读对白，不据此确认正文语言；未核在线首取/快取来源、全 70 页、下载、cold 或其他章节。原 011 失败和旧 17 页结果均保留，各自版本与验收边界不变。

自然 Back 后仍为原夺酒第 2/70 页。临时升序已通过原「重置」恢复全部/默认话数降序，且重置标记消失；Browse 末端恢复，10000ms/AWAKE 读回后释放本人 b5fcf3cb 租约。本轮未改 53 项源启用、导入包或导出私有状态，未将 GUI 首末观察冒作全量持久内容逐项相同。

原件在 Koma `.hvigor/outputs/user-flow-audit/device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership/goda012-entry237-001/`：`goda-current-row.png`、`goda-detail-ready.png`、`duojiu-existing-page2-current.png`、`natural-return-original-detail.png`、`final-browse-restored.png` 及 entry/chapter/continue/return-actions.json。Reader 原日志在旁列 `goda012-continue-log-restored237-001/run-metadata.json`，最终回复在 `goda012-session-restored237-001/run-metadata.json`；全部本轮 artifact 哈希关闭。

2026-10-26：用户报告普通网页误开包子。0.1.3 将独立 homepage 元数据纠正为用户指定、实际首页自称 GoDa 漫畫的 https://godamh.com/；只修品牌主页声明，原 baozimh.org 请求后端、章节/图片 ID 及权限保持。旧 metadata 把后端当主页的 ae83e07 被本修正取代，不能因后端仍保留而复活错误网页入口。

## 0.1.4 显式镜像设置

固定参考 `949dea252eb8e867b7b40834a26e13088764ff55` 声明六个首选镜像。`preferredMirror` 默认 `https://baozimh.org`，只有用户显式选择才使用另五项；未知值回退原默认，绝不自动探测或切换。设置影响目录、搜索、详情的站点域和 API/图片请求 Referer、Origin；共享 API 与原图地址仍由原实现及身份校验决定。`metadata.json` 的官网入口始终是 `https://godamh.com`，不随此选择重写。原漫画、章节和页面 ID 不改，也不自动迁移书库。

六域真实首页与具名《有鬼拍不到》详情均能读取、作品 ID 及首章 ID 一致；生产 WASM 分别验证目录、搜索、详情、同一首章页表。实际一次实时 API 更新使第一域章节 116 条、后五域 117 条，116 个已有章节 ID 均相同，不推断其他作品或站点全量一致。备用域 godamh.com 完整图片请求头实取首图 200；此前缺 UA 的直接请求两种 Origin 均 403，不把错误当图片。默认图像请求与 0.1.3 相同。真实 237 设置持久、图像渲染和跨镜像旧库仍需独立验证。

## 0.1.6 显式不可用章节与大型声明图像数组

`woduzishengji-duburedicestudio-a04` 和 `guizhuoxiandao` 两个保留的 HTTP200 漫画详情页把 h1 状态显式标记为 `章節下架` 并省略整个 `#mangachapters`/data-mid 容器。这是该漫画的章节目录，不是一个失败章节或已证明的整作品删除。在那个精确组合中详情/章节返回 `error.code=content_removed` 带 `源站已标示该作品章节下架`，旧宿主 `reasonCode=not_found`。普通 HTTP 404 保持原 `not_found` 结果。未知缺失节点保留先前解析错误；可用作品不受影响。不推断漫画 ID，不虚构空完整章节列表，不自动探测镜像。这遵循固定参考 GoDaManhua 的已删目录处理。

真实身份核对的 `32982/1788073` 章节含一个 41,288 字节编码字符串：先前本地 32KiB 输入缓冲区拒绝了它，尽管独立解码的数组只有 30,958 字节且含 221 个唯一图像 URL。图像字符串解转义现在临时使用既有 1MiB 载荷缓冲区。完整解码器先写入单独的 REORDER 和 DECODED 缓冲区；其临时载荷借用序列化开始前结束，页数组只从 DECODED 借用。这不添加内存缓冲区或 SDK/宿主限制。最终 1MiB 封套、2048 页容量、完整 JSON/UTF8 校验、标记/字母检查、mid/id/slug 检查、固定 CDN、路径限制、完整原数组顺序和页身份保持不变。不使用截断或合成图像 URL。

参考读回使用不可变 Keiyoushi `949dea252eb8e867b7b40834a26e13088764ff55` GoDa.kt、GoDaManhua.kt 和 Dto.kt。静态解码器不变；独立字面参考管线为全部 221 项提供完整预期数组。元数据/API 回放和这个分类不声称当前 Native 阅读、图像二进制可用性或访问公开漫画页显式扣留的章节。
