# ガンガンONLINE

Square Enix 官方日文漫画杂志。可浏览连载、完结作品、ガンガンpixiv 和ガンガンGA，并阅读官网当前公开话。

对照固定版本 [Keiyoushi Gangan Online 1.6.1](https://github.com/keiyoushi/extensions-source/tree/c09b174459c037c62292e4e7a929a0cd59430bc4/src/ja/ganganonline) 和 [Koma 运行 ABI](https://github.com/honjow/Koma/blob/master/docs/source-runtime-abi.md)。

| 行为 | 上游和官网合同 | Koma |
| --- | --- | --- |
| 目录和今日更新 | 上游默认读取 `/rensai`，有连载、完结、pixiv、GA 四个分类，目录一次返回。官网首页有独立的「今日の更新作品」分区。 | `get_manga_list` 的 `popular` 打开连载目录，`latest` 读取官网首页的当日更新分区。两个入口均用官网排序，排除标为小说的结果，按标题 ID 去重。 |
| 查询和筛选 | 非空关键词用 `/search/result?keyword=...`。空关键词以筛选确定官网目录，GA 保留连载到完结的顺序。 | `search` 用 `args.query` 和 `args.filters.category`，保留空结果和无续页语义。`get_filters` 提供五语分类。 |
| 作品和话数 | `/title/{id}` 的 `__NEXT_DATA__` 包含 `data.default`。上游隐藏状态 1–3，官网字段定义为未公开、已结束、待公开。缺省或 0 为公开状态。 | `get_manga` 与 `get_chapters` 以原站 ID 保持同一作品身份。目录保留可识别的已锁定话，加 `🔒 ` 和 `locked`，排除自动续读。有效公开期仍按原站当下的状态判断。 |
| 正文 | `/title/{id}/chapter/{id}` 返回公开话的 `pages`，含画页和原站附图。官网同源 Next.js JSON 的公开话 DTO 与 HTML 一致。 | `get_pages` 先核对当前目录状态。已锁定返回 `chapter_locked`，公开话读取查看器的同站签名 WebP，保留官网页序和页身份。图像已是标准像素顺序。 |
| 设置和登录 | 本网页目录、搜索和公开正文支持匿名读取。 | 设置为空，锁定内容依原站权限展示。 |

最低 App 版本为 0.1.0。签名由官网查看器生成，重新取页表时读取官网当前签名。图标出处见 [ASSET_SOURCES.md](ASSET_SOURCES.md)。
