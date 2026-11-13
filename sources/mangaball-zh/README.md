# MangaBall 中文 0.1.5

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

按 API 的语言、译组 ID 及卷／话坐标验证续读。同号候选保留，译组切换、黑名单排除和同号边界截断自动续读。
