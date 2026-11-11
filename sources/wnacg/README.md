# 紳士漫畫 (WNACG)

Keiyoushi `de743aed846da53e00c45ab26ee0e77f7bb4355b`（extension v6、lib1.6、Apache-2.0）处完整公开 WNACG.kt、Filters.kt、Preferences.kt 和镜像行为的独立 no_std Rust/WASM 移植。保留源名称、zh 语言、NSFW 分类、默认 buzz/站点镜像和精确原桌面 User-Agent。相关作品和 ONLY_FETCH_ONCE 不由当前 Koma ABI 暴露。

匿名热门/最新、关键词搜索、标签/分类/排行/排序筛选、支持的画廊/阅读器/合集 URL 导入、元数据、单画廊和完整分页合集章节，以及每个原画廊图像匹配都已实现。合集保留每个 data-chid 的首次出现并反转完整原目录。重复图像 URL 保持原位置和独立 ID。单画廊日期和合集日期保留 Asia/Taipei 午夜。图像不筛选、不裁剪、不解乱。每个宿主进程的首次请求（包括缓存页图像请求）的镜像发现遵循原 BaseUrlFetchInterceptor。为精确先前镜像宿主构建的请求以不变路径/查询移到当前镜像；外部 CDN 从不猜测或重写。源侧所有者校验覆盖 PageID、可选漫画/章节/URL 所有者和 /data/<前缀>/<两位>/ 路径中可证明不同的 CID 分区。不透明 CDN 路径接受而不猜测其宿主或归属。

宿主适配：
- 无状态调用用不透明 next-source-page 游标做黑名单跳过，替代 Mihon 的可变 generation/offset 对象。查询/设置生命周期仍由宿主拥有。逗号/换行关键词、大小写不敏感匹配、搜索开关和默认最多扫描五页保留参考行为。
- html_text 缺少 br 节点回调。真正的 br 标签在 HTML 解析前变成字面文本换行；普通节点文本单独规范化空白，而描述保留整段文本换行。
- 原进程作用域镜像刷新使用 source_private_session_process_v1 和 CAS source_private_session_replace_v1。一个宿主进程标签触发一次普通公开域名列表发现。失败/空发现为该进程保留先前镜像列表，无 TTL、备用 URL 或重试。带版本的私有存储是安装作用域的；这与上游 SharedPreferences 生命周期不同，作为文档记录而非视为相同。
- 镜像选项值是不透明 URL 令牌加自动首项别名。可见选项保持实际镜像宿主。默认跟随首项；显式非首项选择跟随其 URL；消失或变为首项的选择变为自动。这保留参考的选择语义而不从 WASM 写公开设置。设置/登录元数据只读取就绪缓存，或立即中止空所有者；从不发现镜像、调用 process_scope、提交值或发送 HTTP。
- 额外分类在每次实际列表后替换保存的菜单快照。这使用同一私有 CAS 所有者；仅合并的 settingObservations 不能表示登出/移除语义。
- 文本扫描计数消费遵循 Kotlin 的可选 plus、BMP 十进制数字、前导零、Int 范围和无效/非正回退五。参考也在保存前拒绝无效值。get_settings 要求 source_setting_positive_integer_v1 并声明其既有 positive_integer 校验器带原错误消息。普通读取保留参考 getter 对遗留无效保存值的回退；不依赖表单校验作为替代。宿主实现存在；Native 验收由 Root 单独跟踪。

所有真实网络入口在 RPC/HTTP 前要求 source_private_session_v1、source_private_session_process_v1、source_private_session_replace_v1 和 source_rate_limit_wnacg_v1。速率所有者对所有匹配 `^(?:www\.)?wn(?:acg\w*|\d+)\.` 的实际宿主共享 50/60000ms；无关图像 CDN 排除。这个正则从不授予 Cookie 或登录权限。Web 认证描述符提供精确桌面 UA 和实际所选镜像 /users-login.html。只有显式声明的源才能获得宿主 Web 会话；新发现的未声明镜像在那个授权上失败。authenticated:false 表示无可源验证的账号状态；不用 Cookie 检查或推测性登录请求。

404 和缺失 h2 保留参考的含糊：内容可能不存在也可能需要登录。两者都不证明下架、付费或认证。传输、403 拒绝、429、5xx、已知空相册和畸形响应保持分开。失败的合集页从不发布部分目录。不引入假账号、访问限制绕过或拒绝后重试。

本地验证 2026-11-06：固定 60 作品分层计划产生 59 个完整原网站样本和一个保留的 RemoteDisconnected 失败。实际 WASM 匹配五个列表层和全部 59 个详情、92 个原章节和 5807 个原图像位置。12 个不同作品的中间原件已下载、解码并目视审查（11 漫画页、一张照片）；未计广告或空白。独立的实际 WASM 状态/归属/传输恢复使用显式本地宿主测试替身。本地回放不建立全站健康。生产需要 Koma 1.2.0 或更高以获得上述宿主能力；Native 验收和公开分发必须单独验证。

审查更正：内部 V1 用了 uploadTime，而实际宿主章节解析器没有消费它。V2 使用既有 dateUpload ABI。原 V1 WASM 通过 SourceModels 丢失全部 92 个预期日期；V2 WASM 对不变原网站 oracle 保留全部 92 个。V1 保持不可变失败证据，没有改宿主解析器。

生产工作区保留本源的原尺寸优化 release profile；打包拒绝超过宿主 1 MiB 限制的 WASM。

封面请求沿上游桌面 User-Agent 和当前镜像 Referer；宿主以作品 ID 作为 pageId 时，书源只在本地校验已观察到的 WN 缩略图 CDN 与路径，并在给出作品网址时核对作品 ID，不为每张封面重复请求详情。合集封面路径可能属于子相册，不能据此假设它等于合集 ID。正文图片仍使用原章节身份校验。

0.1.1：当宿主通告 `source_public_cover_user_agent_v1` 时，目录在其已选镜像 Referer 和原 User-Agent 旁携带每个批准的公开封面 URL。宿主安排下载。较旧宿主仍使用原图像解析器。
