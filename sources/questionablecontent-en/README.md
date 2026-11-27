# Questionable Content

Jeph Jacques 自 2003 年连载的英文网漫，以友情、恋爱与机器人为主题。官网全部漫画归档为一本连载。

## 移植对照

对照固定版本 [Keiyoushi Questionable Content 1.6.2](https://github.com/keiyoushi/extensions-source/tree/c09b174459c037c62292e4e7a929a0cd59430bc4/src/en/questionablecontent) 和 [Koma 运行 ABI](https://github.com/honjow/Koma/blob/master/docs/source-runtime-abi.md)。

| 行为 | 上游与官网 | Koma |
| --- | --- | --- |
| 列表、搜索及筛选 | `getPopularManga` 提供唯一归档作品。上游无独立更新列表和搜索结果。 | `get_listings` 返回五语的漫画归档栏目。`get_manga_list` 和 `get_manga` 共用作品 ID 与官网封面。筛选为空，搜索返回空列表。 |
| 章节与分页 | `/archive.php` 的 `#container` 按站点显示顺序取 `view.php?comic=...` 链接，同一 URL 保留第一条。上游为新发现的首章记录本地时间。 | `get_chapters` 以原站顺序和公开编号写章节，保留官方链接。`args.page.cursor` 接续上次返回的末条章节 ID，每页最多 750 章，正文和阅读身份不因续页变化。 |
| 正文 | 访问 `view.php?comic=...`，读取 `#strip`。上游避免接受 zstd 编码，防止部分客户端只读到第一帧。 | `get_pages` 从 `args.chapterId` 取原站编号，解析公开同站 HTTPS 图片，HTTP 请求支持 gzip 压缩。 |
| 作者附注 | `showAuthorsNotes` 默认关闭，可选追加 `#newspost`。 | `get_settings` 与 `get_pages` 均核对 `text_page_image_v1`。有附注时用 Koma 现有模板输出纯文字页，标题五语，正文保留作者原文。 |
| 登录与付费 | 正文和归档匿名可访问。 | 全程匿名读取。 |

基础漫画阅读以 0.1.0 为最低版本，附注沿宿主能力协商。
