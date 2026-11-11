# 绅士会所

完整 HentaiClub.kt/Filters.kt 的独立 Rust/WASM 移植，位于 Keiyoushi
de743aed846da53e00c45ab26ee0e77f7bb4355b（extension v1、lib1.6、Apache-2.0）。
名称绅士会所、zh 语言和原宿主 www.hentaiclub.net 保留。
参考是 MIXED：逐作品评级保持未知；不从网站名给单部漫画分类。该参考没有账号、偏好、最新列表、
相关作品或 URL 导入实现。

热门/搜索/标签/分类/详情/一个固定章节/全部原 data-src
位置和原图像头已实现。分类分页
刻意重复原分类 URL；不受支援的最新/URL 导入
保持不受支援。第一张内容图像是参考的实际封面。
不透明 CDN URL 不因猜测的归属被拒绝。PageID 和提供的
漫画/章节/URL 所有者在图像请求输出前严格检查。

匿名网络操作在 HTTP 前要求 source_rate_limit_hentaiclub_v1：
精确主宿主 www.hentaiclub.net 共享 2/1000ms，无关 CDN 不共享。
单固定章节元数据和图像请求描述符无需 HTTP。
Jsoup 的无引号子串属性选择器为宿主爬虫加了引号；
其含义不变。传输/拒绝/畸形/已知空情况保持
分开，无推断的下架或付费。

## 参考对照与验收

- 列表卡片标题去掉网页标签缩进产生的首尾空白，保留标题内空格、原文字及排序。
- `get_listings` / `get_manga_list` 对应 Mihon 热门，无独立首页及最新列表。
  `search` 先用非空关键词，再按标签、分类、热门的顺序选择。关键词和标签使用
  页码游标；分类沿原参考对每个游标请求同一分类地址。是否有下一页沿用 24 项阈值。
- `get_manga` 对应详情的标题、首张正文封面、首标签作者、标签顺序、浏览量
  与完结状态。`get_chapters` 对应固定的「章节 1」，独立调用无需 HTTP。
  Mihon 的 `ONLY_FETCH_ONCE` 是后台调度提示，Koma ABI 没有对应字段。
- `get_pages` 对应原页表的全部位置，保留重复图片。`get_image_request` 按原
  User-Agent/Referer 返回请求描述；Reader 用章节页 ID，封面用同一作品 ID。
  两者都校验附带的归属，无需请求正文页。CDN 路径本身不包含作品 ID。
- 原站网页实测 7 个列表共 144 个卡片位置、107 部不同作品，60 部逐一核对
  详情、封面、章节及 2696 个原图位置（含 11 个重复位置）。10 部不同 R-15
  作品各一张正文真实下载并解码。另一次 HTTPS 传输失败发生在收到 HTTP 响应前。
  Koma 1.2.0 提供该源所需的主站限流能力。
