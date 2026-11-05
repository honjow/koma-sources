# Komiic 0.1.13

源 ID `com.komiic.koma`，语言及 NSFW 标记保持；crate、runtime 与 package 版本对齐。普通网站为 `https://komiic.com/`，不把 API 地址冒作网页入口，也不由网页元数据授予验证或登录权限。既有 180px 官方 touch icon 不替换，素材依据见 [ASSET_SOURCES.md](ASSET_SOURCES.md)。

搜索、详情、完整目录和热门分页使用既有 GraphQL 链；热门列表不作为独立首页重复展示。图像保原 page:index 身份、原 image URI 的 kid、returned-kid 相等校验、官方 img.komiic.com 边界以及原封面 Accept/Referer；不引入账户或自动镜像切换。

## 设置合同

`CHAPTER_FILTER` 默认 `all`，选择为 `all`（同时显示卷和章节）、`chapter`（仅章节）、`book`（仅卷）。每次 `get_chapters` 从请求 `settings` 读取；未知值保全量。

只在用户显式选 chapter/book 时保留同类型项，原相对序列、ID、标题、pageCount、章卷与续读元数据保持。没有照搬参考的排序去改变已采用默认，因为现有目录、下载与阅读记录消费原序列和身份。未改原图票据链或既有数据。

支持 `source_chapter_exclusions_v1` 的宿主在显式筛选时接收已验证完整目录中的实际 `excludedChapterIds`，只隐藏被筛掉的章节，不删除库、进度或下载。仅对属于该作品的精确旧章请求绕过筛选；目录不完整、ID重复或作品身份不符时返回错误，不发布排除集合。旧宿主及默认 all 保持原行为。非标准章节类型不能判定为已排除，返回错误并保留已有章节。

固定参考 949dea252eb8e867b7b40834a26e13088764ff55 的同一源以 mirrors 声明两个域并生成首选镜像设置；此前“独立源变体”说明有误。`preferredMirror` 默认 komiic.com，显式选择 komiic.cc 才切换 GraphQL 和图片描述域，不自动试探或更换；旧 komiic.com 图片描述在切换后仍按相同 kid 取得票据。旧书章 ID、默认请求、官网入口、图标和 App 样式保持，不增加敏感明文字段或 Host import。

两域同一具名作品3017的50章和首章226个图片 kid 实际一致；备用域生产 WASM 的首页、目录、搜索、详情、章节、页表和票据均已验证。备用域及0.1.8默认域同一首图请求均返回403，不能称正文图可见；设置持久和 Native 仍待具名设备验证，其他作品不可据样本外推。

搜索查询按请求 JSON 字符串解码后再编码进 GraphQL，保留实际引号及反斜线；空查询、分页、镜像选择及原请求合同不变。

## 浏览及筛选

浏览参考固定 keiyoushi/extensions-source `4591fbc9bb571ad03aaea0f7428f403350e90e` 的 `src/zh/komiic/.../{Komiic,Queries,Payload,Filters}.kt`。`get_listings` 对应参考热门与最近更新入口，`get_manga_list` 读取宿主 `args.page.cursor`；热门与最近更新分别沿参考的 MONTH_VIEWS / DATE_UPDATED，站端每页30项；宿主分页游标绑定站端页号，不把宿主20项提示当站端偏移。类型采用官网 allCategory 动态读取，多类型为同时包含；排序、状态及色气程度沿参考。非空关键词搜索不消费浏览筛选；旧宿主缺少筛选作用域及动态加载能力时不暴露筛选。原作品、章节、镜像及搜索合同不变。

列表与搜索中，官网明确的 public.komiic.com/comics/<作品封面身份>/cover.jpg 或 cover.webp 可附既有图片解析器同样的 Accept 和所选镜像 Referer，方便新宿主首次公开封面加载；URL原样保留。票据图片、其他域和未知格式继续走原解析器，不发送凭据。

详情标签按官网返回顺序先保留 categories，再追加 warnings 原文，与固定参考 genre 语义一致；请求所需 warnings 缺失或格式错误时明确失败，不冒充没有警示。
