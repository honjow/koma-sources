# MangaBall 中文 0.1.9

## 0.1.10 簡介與多語言顯示

MangaBall 的中文章節書庫，同一作品可能有不同翻譯組發布的章節。收錄範圍以章節語言為準，書名不一定是中文。

零參 `source_info` 固定使用以上基準簡介；`metadata.localization` 僅新增 `source.description` 的繁中、簡中與英文文案，供支援的宿主顯示。

独立源 `com.mangaball.zh.koma`，API 0.2，语言 zh。中文选择章节语言；不保证中文标题或特定书写字母。既有 `mb:`/`mbc:`/页身份、提供方章节顺序、实际章节/卷号和精确页所有权保持。既有 512px MangaBall 身份图标不变；官网品牌使用 MangaBall；归因见 NOTICE。

公开列表/搜索/详情/中文章节/页使用规范 `mangaball.com`、既有精确封面/页宿主和既有有界 JSON 和分页校验。不引入 cookie、账号凭据、镜像、脚本、统计请求、推断章节编号或 URL 重写。畸形、重复、不完整和禁止实体保持错误。付费/锁定章节和页守衛独立于列表设置。

`nsfw_pref`（隐藏 NSFW 内容）默认 **true**，保留 Koma 显式采用的非成人默认和 `adult_mode=no_18`。显式 false 使用参考 `adult_mode=all`；未知/缺失成人标志仍失败校验。这控制列表/搜索请求，不改变既有单书权限。

`scanlator_blacklist_pref`（翻译组黑名单）默认空数组，只在宿主同时声明 `source_setting_observations_v1` 和 `source_chapter_exclusions_v1` 时可用。成功中文章节列表把其实际非空白原始组名返回在 `data.settingObservations.known_scanlators_pref`；`get_settings` 从 `args.settingObservations` 消费宿主的源特定并集。选项保留原始名并按 Unicode 小写 UTF-16 顺序排序。初始空并集产生空多選，匹配固定参考；不插入虚构选项。宿主拥有观察持久化，独立于所选设置。

筛选在 trim 和完整 Unicode 小写后比较每组和所选值，精确相等。Rust `alloc::str::to_lowercase` 和 `dlmalloc` 支援既有 no_std WASM 而不近似上下文 Unicode 规则。原所有权、权限、编号/标题校验在筛选前运行；筛选掉全部有效中文章节以空列表成功，而原本缺失的中文目录在完整响应校验后返回显式 `no_chapters` 结果。观察在黑名单筛选前收集，故隐藏一组不擦除其可选名。缺少任一能力的旧宿主忽略保存的黑名单值；观察能力宿主仍可独立学习真实名。

完整筛选目录也返回 `data.excludedChapterIds`，只含当前选择移除的实际已校验完整章节 ID。宿主可以从保留详情 UI 省略这些 ID 而不删除 Library 章节、下载或阅读状态；其他缺失章节保留既有保护。精确 `args.chapterId` 恢复请求校验完整响应并只返回那个合法原章节，独立于目录黑名单，无虚构替换或 ID 回退。缺失精确章节保持未找到。

设置遵循 Keiyoushi `949dea252eb8e867b7b40834a26e13088764ff55` MangaBall/ScanlatorBlacklist；默认 true 的成人设置是显式 Koma 适配。用 `bash build.sh --source mangaball-zh` 构建；标准打包器从 metadata.json 保留本地化名称。历史封面访问失败不由这些设置声明已解决。成功构建或受控响应回放不确立当前内容或 native 图像可用性。

搜索筛选按固定参考提供排序、类型、受众、状态、标签模式与四组三态标签，仅在支援作用域及三态的宿主搜索中显示；未筛选请求、热门与最新排序和成人内容默认保持。旧宿主不暴露新筛选。

规范全语言章节数组在中文选择前完整校验。公开 2026-11-05 证据包括 6,322,808 原始字节 / 9,847 项（One Piece）和大于旧 1 MiB 体 / 2 MiB 封套 / 2,048 项限制的效中文目录。源现在把 HTTP 封套和解码体限制在 8 MiB，章节项限制在 16,384；出向结果仍限于 1 MiB。章节跨度在堆上，精确 ID 重复检测用有序集而不重排输出。这保留 native 调用栈并避免对完整多语言数组的二次重复扫描。宿主 ABI -3 分类为本地封套容量，不是站点网络失败。按当前源错误消息要求，完整有效空数组和无中文章节的目录现在返回结构化 `no_chapters`，与 not_found/移除分开。缺失/无效 JSON、被拒 HTTP 响应、所有权/重复失败和网络错误保持其实际失败分类；不引入替代语言/ID 回退。这些限制不隐含无限站点或 native 设备验收。

实际章节编号是有限有符号 JSON 数字，匹配固定参考 Float 和 App 章节编号模型。保留的 Log into the Future API 有最终英文 `Chapter -1`；这个合法外语行必须不使其 329 个中文章节失效。卷号保留既有非负/null 校验。不合成或重写章节编号。

## 2026-11-10 显示名精简

按用户要求去掉单源语言后缀，当前显示名为 `MangaBall`；源 ID、内容、图标和阅读行为不变。历史名称仅保留在旧版本记录中。

## 章节续读

续读使用规范完整多语言数组中的唯一全局位置（从最早条目开始），先分配位置再做原有中文选择和翻译组黑名单筛选。实际卷／话号只作为显示字段；同号拆分、无号、负号及缺少 group_id 均不因这些字段本身丢失邻章，不补号、不删不同 ID，不用显示排序替换 provider 位置。该 API 一次返回完整目录，返回终态 null cursor；不把宿主 page.limit/cursor 当成 provider 分页，更不按显示页重置坐标。

阅读组按作品、原始语言、真实 site 与译组身份划分。优先 group_id，再用嵌套 group.id/_id；缺 ID 时只复用同语言/site/组名下唯一的已知身份，同名但不同真实 ID 不合并。仍无法确定身份的条目保留独立组名/未分配组的 provider 位置链，不改成 ManualOnly；语言、site 和明确版本边界保留。黑名单只移除用户确实选中的条目，不改变保留 ID 的全局位置。精确旧章恢复仍独立于黑名单、仅返回合法原 ID。

本轮修复取代 `8f720ab` 引入的“相邻同卷同话号就 ManualOnly/切 run”条件。真实 Tales of Demons and Gods 的 933 条中文中有 814 条相邻重号，Fights Breaking Through The Heavens 的 329 条中有 110 条；这些均有有效话号、cn 语言、同一 Dodrio 组 ID 和 godamh site，不是用户筛选或真实版本切换。上下/分段章的不同业务 ID 全部保留。原详情 API 的 prev_chapter/next_chapter 在已取样本中可为空或跨到西语/印尼语，不能拿它覆盖当前语言/版本链。

源码对照固定为当前 Keiyoushi `5d54f0d02e2a7dbdda08e823de444f7527cc883c`：`MangaBall.kt:198-207` 直接消费规范 `/api/v1/chapter/chapter-listing-by-title-id` 数组并按语言/黑名单选择；`Dto.kt:137-172` 允许 number/group 缺省，没有同号排除。Koma 源 `chapters` 保留该请求与全量输入/权限校验；`chapter_edition` 仅负责真实边界，typed GroupedChapters 输出序位。宿主实际经 `projectSourceChapterResponse → parseSourceChapterRow → sourceCatalogChronology → continuationChapterIds` 消费。本次 Cargo 与安装版本同步步进为0.1.9，SourceInfo继续由Cargo版本生成；锁文件只同步本源条目。未改SDK、其他消费者或素材。

作品和章节评论复用网页的 `/api/v1/comment/load-comments` 只读列表。作品使用站点的 `title_only` 筛选，章节按章节 ID 读取，翻页沿用原站分页。
