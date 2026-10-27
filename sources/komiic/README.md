# Komiic 0.1.8

源 ID `com.komiic.koma`，语言及 NSFW 标记保持；crate、runtime 与 package 版本对齐。普通网站为 `https://komiic.com/`，不把 API 地址冒作网页入口，也不由网页元数据授予验证或登录权限。既有 180px 官方 touch icon 不替换，素材依据见 [ASSET_SOURCES.md](ASSET_SOURCES.md)。

搜索、详情、完整目录、热门分页和首页使用既有 GraphQL 链；首页复用热门请求与原卡片解析，提供 12 项预览及完整列表目的地。列表导航描述不代替真实内容。图像保原 page:index 身份、原 image URI 的 kid、returned-kid 相等校验、官方 img.komiic.com 边界以及原封面 Accept/Referer；不引入账户或镜像。

## 设置合同

`CHAPTER_FILTER` 默认 `all`，选择为 `all`（同时显示卷和章节）、`chapter`（仅章节）、`book`（仅卷）。每次 `get_chapters` 从请求 `settings` 读取；未知值保全量。

只在用户显式选 chapter/book 时保留同类型项，原相对序列、ID、标题、pageCount、章卷与续读元数据保持。没有照搬参考的排序去改变已采用默认，因为现有目录、下载与阅读记录消费原序列和身份。未改原图票据链或既有数据。

支持 `source_chapter_exclusions_v1` 的宿主在显式筛选时接收已验证完整目录中的实际 `excludedChapterIds`，只隐藏被筛掉的章节，不删除库、进度或下载。仅对属于该作品的精确旧章请求绕过筛选；目录不完整、ID重复或作品身份不符时返回错误，不发布排除集合。旧宿主及默认 all 保持原行为。非标准章节类型不能判定为已排除，返回错误并保留已有章节。

镜像是上游独立源变体，不在此设置中伪造自由域名；不增加敏感明文字段、Host import 或 App 样式。
