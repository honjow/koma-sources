# Buon Dua 0.1.0

完整 Keiyoushi BuonDua.kt 的独立 no_std Rust 移植，位于
de743aed846da53e00c45ab26ee0e77f7bb4355b（versionCode 13 / lib1.6 / Apache-2.0）。
提供者 https://buondua.com，sourceId com.buondua.koma，参考 NSFW/全语言。

热门和最新使用精确的 20 项 offset 请求。非空文本搜索优先于 Tag ID；空搜索/浏览可用原 Tag ID 路由。详情保留标题的页后缀去除、文章信息、原下载链接描述、公开存档码文本和标签。不抓取下载链接。详情和 URL 搜索在同一文章所有者匹配所请求图库且图像在提供者宿主上时使用该文章的官方 Open Graph 封面。缺失或外部元数据使其余详情仍可用。默认 pref_split_pages=true 按最新优先列出网页章节，Page N 到 Page 1；false 返回一个 Gallery 并按网页/原 DOM 升序拼接每个真实网页的图像。重复图像 URL 保留其位置。任一网页失败则整个合并失败；无部分成功或虚构页。

宿主适配：独立 Koma 章节 ID 显式记录网页/图库模式，跨设置变更保留精确既有章节 ID。上游的可选相关作品函数没有 Koma ABI 操作。整图库读取是顺序的因为宿主有界调用；内容顺序不变。日期保留参考的无时区本地日期/时间，不添加时区。文本/头筛选 schema 使用受支援宿主声明。标准图像传输即足够：实际 CDN 原字节无 Referer 或 Cookie 即匹配。不声称图像处理/头/认证特性。

对 buondua.com 的请求共享参考的每秒 10 请求宿主限制。该规则不含 i2.buondua.com 或其他 CDN 宿主。网络操作在执行 HTTP 前要求特定 source_rate_limit_buondua_v1 能力；旧的通用速率特性不能替代。生产包要求 Koma 1.2.0 或更新，其宿主提供该规则。

设置身份合同：当宿主声明 source_chapter_exclusions_v1，完整拆分目录显式排除其 Gallery ID；完整合并目录显式排除其已验证 Page 1..N ID。这通过既有宿主合同隐藏替代行而不删除进度/下载。旧的精确 CID 仍解析其自身编码模式。在发布那些排除前，通告的 Page 1/N 和真实分页 End 必须一致；畸形分页失败而非发布部分目录。
