# MangaDex — 简体中文 0.1.10

独立源 `org.mangadex.zh-hans.koma`，API 0.2，内容语言 zh-Hans，提供方 API 语言 `zh`。其他语言源 ID 保持独立；无别名或迁移。提供方语言元数据不保证每张图像的书写系统。实现/许可和原官方图标归属保留在 NOTICE/LICENSE；图标字节/背景/遮罩不变。

列表 关注较多 是全时段 followedCount，不是 30 天热度或用量。搜索/列表要求有可用提供方语言章节并默认 safe/suggestive 标题。原本地化标题/别名/摘要回退和实际逐标题分级保持。完整自有源流分页、UUID/语言/总数/重复检查和整操作边界保持；外部和零页章节保持省略；不可用章节默认隐藏。null/小数/零章节和卷元数据不用序号猜测替换。

`thumbnailQuality_zh` 遵循固定 Keiyoushi 949dea252eb8e867b7b40834a26e13088764ff55：original（默认宿主值 `original`，编码上游空后缀）、medium `.512.jpg`、low `.256.jpg`。列表/搜索/详情只把所选发布后缀追加到实际封面文件名。缺失/未知设置保留原封面。

`dataSaverV5_zh` 默认 false。只有显式 true 选择实际 `chapter.dataSaver` 文件名和 `/data-saver/`；否则原有序 `chapter.data` 和 `/data/` 保持。get_pages 仍校验完整自有源流和声明页计数；稳定页 ID 携带原章节 UUID/索引。每个图像请求仍做其既有新鲜普通 at-home GET 并按索引解析所选质量。所选和原文件集都保留安全文件名、容量和重复守卫；不猜文件名。

图像 HTTPS 宿主保持 uploads.mangadex.org 或实际描述符提供的 mangadex.network 子域。封面请求保持绑定同一漫画 UUID。无 token、Cookie 处理、统计 POST、认证/重试回退、新宿主授权或 App/样式变化。标准构建：`bash build.sh --source mangadex-zh-hans`。运行时/crate/包版本对齐到 0.1.10。

`altTitlesInDesc_zh` 默认 false，只被漫画详情描述消费。显式 true 追加固定参考 `Alternative titles:` 块：扩展语言或原语言罗马化映射，每个映射恰好一个非空值，提供方顺序，不排序/去重。共享 SDK 纯文本助手解码一次 JSON Unicode 和 HTML 实体同时保留字面标记；原基础描述保留。缺失/false 保持先前描述字节、标题回退和所有其他字段不变。标题优先级和最终章节文本有下面独立的设置。

`usePort443_zh`（仅使用 HTTPS 端口 443）默认 false。沿固定 Keiyoushi 949dea 的实际消费者，只在显式开启时给 at-home 服务器查询追加 `?forcePort443=true`；默认请求原样。图片 URL、页 ID/序和校验仍取真实响应，不向 URL 硬写端口，不改其它 API、封面、认证、网络、超时或重试。中文图片请求复用同一 descriptor/settings 合同。

`tryUsingFirstVolumeCover_zh` 默认 false。显式 true 遵循固定参考的按卷排序批量 `/cover` 查询，为每个返回的漫画选择与其原语言 locale 相关的第一张封面。搜索/列表/详情在应用既有封面质量后缀前消费它。缺失 locale、无匹配封面、不安全文件名或可选 HTTP/解析失败保留原封面。默认请求和封面 URL 保持不变；不重写章节/页/身份或 Library 数据。既有 library 封面需要其普通显式刷新。共享提供方实现是 `sources/mangadex/first_volume_cover.rs`。

`originalLanguage_zh` 是固定原出版语言多选（日/中/韩），默认空。只过滤公开列表/搜索请求；中文同时包含 `zh` 和 `zh-hk`。它不选择翻译章节语言、不重写 ID 或 Library 标题，也不过滤精确详情/章节/图像恢复。缺失/空选择保留先前请求和输出。

`contentRating_zh` 遵循固定四选项内容分级多选；缺失设置默认 `["safe","suggestive"]`，保留先前列表/搜索请求和守卫。显式选择只提供那些 API 查询值并接受其已知返回分级。显式空选择省略这个 API 筛选，如同参考；未知/缺失分级数据仍被拒绝。精确详情、完整章节源流（包括其既有全分级查询）、页、图像权限、ID 和既有 Library 数据不受影响。不添加账号/登录或权限绕过。

## 完结卷章说明

`finalChapterInDesc_zh`（描述中的最终章节）默认关闭，保当前 Koma 无配置的简介；固定参考默认开启，这里保留既定缺省输出。显式开启仅在详情末尾按固定 MangaDexHelper 拼接非空 `lastVolume`/`lastChapter` 为 `Final chapter:\nVol.x Ch.y`，在已有简介/别名后隔一空行，并按原实体表解码一次。沿上游原字符串，不猜最大章、不要求数字、不剥正文标签。缺少两值不造段落，非法字段或容量不足明确失败。列表、HTTP、标题、ID、目录/页顺序和已存 Library 不主动变化。

## 标题优先顺序

`preferExtensionLangTitle_zh`（使用替代标题）默认 true，保当前 Koma 原标题优先链与原 JSON 输出，固定参考也是 true 但既有链的细节不据此重写。显式关闭沿固定 MangaDexHelper：扩展语种 primary 值仍优先，缺失才用 primary 首 value，再回扩展语种单值 alt、英文单值 alt，并实体解码一次。列表、搜索、详情及已有 Home 共用；不排序或去重标题，不主动重写 Library 名称、ID、目录及页面。显式关闭时按参考区分空字符串与缺失值，不因字符串为空继续回退；默认和开启仍保原空标题处理，容量不足明确失败。

`includeUnavailable_zh` 默认 `false`，只在宿主声明 `source_chapter_exclusions_v1` 时显示和消费。显式 `true` 显示有正页计数且无外部 URL 的不可用章节，带 `UNAVAILABLE` 翻译组标记和原章节 UUID。禁用时源返回已校验的排除章节 ID，保留的 library 行保持隐藏，而保存的章节、下载和阅读进度保持完好。源流即使禁用也请求不可用元数据，可能需要额外页。与固定参考不同，外部和零页章节保持排除；这个设置不授予内容访问。

UUID 黑名单：`blockedGroups_zh` 和 `blockedUploader_zh` 是逗号分隔 UUID 文本设置，只在宿主同时声明 `source_setting_uuid_list_v1` 和 `source_chapter_exclusions_v1` 时暴露。两者默认空；旧宿主保留既有请求和结果。有章节排除的宿主上，空黑名单保留可见结果；章节源流仍按下文请求不可用元数据。宿主在保存前校验字段；源也拒绝无效值而不是添加未检查的查询参数。普通章节源流应用两个排除，而精确章节恢复和页所有权检查绕过它们。版本 0.1.11 没有 Latest 或 Home 栏目；其普通漫画搜索和关注列表保留既有请求。

状态筛选支持固定参考 Ongoing、Completed、Hiatus 和 Cancelled 多选。显式值作为 status[] 发送；无选择保留原请求、可用章节和语言约束、分级、设置和排序。旧宿主看不到新筛选。其他参考筛选保持未实现。

## 0.1.12 简中文案候选

已有状态筛选和设置的名称、选项、说明与校验提示改为简体中文，字段 ID、值、默认、条件门控和实际处理不变；新增参考的最近章节聚合列表，仅在支持逐栏目筛选的宿主展示并禁用该栏筛选；历史真实章节原体与受控漫画响应、顺序和分页已作本地 WASM 回放。当前线上 API 返回 200 验证页面，真实最新结果、真机及发布均未验证；其他参考筛选仍未实现。
