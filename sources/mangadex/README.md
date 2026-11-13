# MangaDex 0.1.20

独立源 `org.mangadex.koma`，API 0.2，语言 en。既有 `mdx:` 漫画 ID、`ch:` 章节身份、原官方图标和[资产归属](ASSET_SOURCES.md)保持。普通公开搜索/列表/详情/Home、精确章节恢复和既有筛选保持；不跟随外部提供方体。

设置保留 `language`（默认 en）和 `dataSaver`（默认 false）。旧宿主保留 `defaultContentRating`（默认 suggestive）。显式请求筛选保留既有优先级。`thumbnailQuality_en` 遵循固定 Keiyoushi 949dea252eb8e867b7b40834a26e13088764ff55：original、medium `.512.jpg`、low `.256.jpg`。非空宿主值 `original` 编码上游空后缀。缺失/未知质量保持原 URL 不变。所选发布后缀被搜索、列表、详情和所有 Home 栏目消费；既有缺失封面处理不变。

章节 ID、标题回退和原目录顺序保持。null/缺失/空/非字符串章节编号保持未知；实际零和小数编号保留其值。不推断日期。既有 mdxc1 分页状态保留完整仅外部终端检测；空目录、托管/外部混合结果、分页错误和不受支持的仅外部结果保持区分。

既有省流页使用实际所选 API 文件名数组和对应 data/data-saver 路径，保留页索引 ID。不引入账号/token/Cookie 流程、统计副作用、新源/宿主授权或 App 样式。标准构建：`bash build.sh --source mangadex`。运行时/crate/包版本对齐到 0.1.20。

`altTitlesInDesc_en` 默认 false，只被漫画详情描述消费。显式 true 追加固定参考 `Alternative titles:` 块：扩展语言或原语言罗马化映射，每个映射恰好一个非空值，提供方顺序，不排序/去重。共享 SDK 纯文本助手解码一次 JSON Unicode 和 HTML 实体同时保留字面标记；原基础描述保留。缺失/false 保持先前描述字节、标题回退和所有其他字段不变。标题优先级和最终章节文本有下面独立的设置。

`usePort443_en`（仅使用 HTTPS 端口 443）默认 false。沿固定 Keiyoushi 949dea 的实际消费者，只在显式开启时给 at-home 服务器查询追加 `?forcePort443=true`；默认请求原样。图片 URL、页 ID/序和校验仍取真实响应，不向 URL 硬写端口，不改其它 API、封面、认证、网络、超时或重试。

`tryUsingFirstVolumeCover_en` 默认 false。显式 true 遵循固定参考的按卷排序批量 `/cover` 查询，为每个返回的漫画选择与其原语言 locale 相关的第一张封面。搜索/列表/详情和所有 Home 栏目在应用既有封面质量后缀前消费它。缺失 locale、无匹配封面、不安全文件名或可选 HTTP/解析失败保留原封面。默认请求和封面 URL 保持不变；不重写章节/页/身份或 Library 数据。既有 library 封面需要其普通显式刷新。共享提供方实现是 `sources/mangadex/first_volume_cover.rs`。

`originalLanguage_en` 是固定原出版语言多选（日/中/韩），默认空。只过滤公开列表/搜索和 Home 栏目请求；中文同时包含 `zh` 和 `zh-hk`。它不选择翻译章节语言、不重写 ID 或 Library 标题，也不过滤精确详情/章节/图像恢复。缺失/空选择保留先前请求和输出。

在声明 `source_settings_context_v1` 的宿主上，带存在 `args.settingValues` 的 get_settings 用固定 `contentRating_en` 多选（safe/suggestive/erotica/pornographic）替换旧三值 `defaultContentRating` 选择器。缺失新数组继承旧 safe→[safe]、erotica→[safe,suggestive,erotica]，suggestive/缺失/未知→[safe,suggestive]。旧 Erotica 选项从不隐含 pornographic。Schema 读取是纯的；只有普通显式 Save 持久化新数组。空或失败的上下文被区分：读空 `{}` 获得参考默认，缺失上下文保持旧 schema。缺失特性即使存了新数组也保持旧 schema 和遗留消费。支持宿主上新数组胜出，既有显式单类筛选仍优先；Home 使用保存的集合。详情、章节/页身份和 Library 标题不变。

## 完结卷章说明

`finalChapterInDesc_en`（描述中的最终章节）默认关闭，保当前 Koma 无配置的简介；固定参考默认开启，这里保留既定缺省输出。显式开启仅在详情末尾按固定 MangaDexHelper 拼接非空 `lastVolume`/`lastChapter` 为 `Final chapter:\nVol.x Ch.y`，在已有简介/别名后隔一空行，并按原实体表解码一次。沿上游原字符串，不猜最大章、不要求数字、不剥正文标签。缺少两值不造段落，非法字段或容量不足明确失败。列表、HTTP、标题、ID、目录/页顺序和已存 Library 不主动变化。

## 标题优先顺序

`preferExtensionLangTitle_en`（使用替代标题）默认 true，保当前 Koma 原标题优先链与原 JSON 输出，固定参考也是 true 但既有链的细节不据此重写。显式关闭沿固定 MangaDexHelper：扩展语种 primary 值仍优先，缺失才用 primary 首 value，再回扩展语种单值 alt、英文单值 alt，并实体解码一次。列表、搜索、详情及已有 Home 共用；不排序或去重标题，不主动重写 Library 名称、ID、目录及页面。显式关闭时按参考区分空字符串与缺失值，不因字符串为空继续回退；默认和开启仍保原空标题处理，容量不足明确失败。

`includeUnavailable_en` 默认 `false`，只在宿主声明 `source_chapter_exclusions_v1` 时显示和消费。显式 `true` 显示有正页计数且无外部 URL 的不可用章节，带 `UNAVAILABLE` 翻译组标记和原章节 UUID。禁用时源返回已校验的排除章节 ID，保留的 library 行保持隐藏，而保存的章节、下载和阅读进度保持完好。源流即使禁用也请求不可用元数据，可能需要额外页。与固定参考不同，外部和零页章节保持排除；这个设置不授予内容访问。

`blockedGroups_en` 和 `blockedUploader_en` 是逗号分隔 UUID 文本设置，只在宿主同时声明 `source_setting_uuid_list_v1` 和 `source_chapter_exclusions_v1` 时暴露。两者默认空；旧宿主保留既有请求和结果。有章节排除的宿主上，空黑名单保留可见结果；章节源流仍按下文请求不可用元数据。宿主在保存前校验字段；源也拒绝无效值而不是添加未检查的查询参数。普通章节源流应用两个排除并在成功的分页遍历中报告完整排除章节 ID，保留已存章节和进度。精确章节恢复和页所有权检查绕过这些筛选。排除激活时，默认英文 Latest 及其 Home 栏目使用章节端点并获取其不同漫画 ID。显式排序保留原漫画顺序并在相同排除下按章节可用性过滤；普通漫画搜索和其他列表保留既有请求。

排除下英文 Latest 返回超过 20 部漫画时，既有首卷封面偏好用相同有序 locale 匹配应用到每个有界批次；失败保留每个原封面。偏好默认保持禁用。

搜索在 URL 编码前解码 JSON 查询，保留字面引号、反斜杠和 Unicode。搜索应用既有状态、人口和排序筛选；缺失或默认值保留原请求和相关性顺序。空搜索和分页保留既有行为。

## 章节续读

续读使用站点的译组 UUID 集合及卷／话坐标。无译组时用明确上传者区分；同一组重复坐标由宿主拒绝自动排序，所有候选仍可手动选择。原游标、外链及黑名单合同保持。
