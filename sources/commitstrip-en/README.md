# CommitStrip

CommitStrip 英文版是以程序员和网站开发公司日常为题材的独立网漫。

## 移植对照

对照 [Keiyoushi CommitStrip](https://github.com/keiyoushi/extensions-source/tree/c09b174459c037c62292e4e7a929a0cd59430bc4/src/all/commitstrip) 的英文入口与 [Koma 运行 ABI](https://github.com/honjow/Koma/blob/master/docs/source-runtime-abi.md)。

| 行为 | 上游与官网 | Koma |
| --- | --- | --- |
| 目录和年份 | 上游按 2012 年起的年度归档建作品，`.wp-pagenavi` 提供续页，`.excerpt` 给出每日条目。 | `get_manga_list` 直接浏览官网英文漫画归档。年份筛选保留官网年度路径；保留官网顺序并用游标逐页读。 |
| 搜索 | 上游按年度标题本地筛选。官网 `/en/?s=...` 能查到单篇漫画。 | `search` 取英文站内搜索，同一条英文永久链接在列表、搜索与详情中使用同一个 ID。搜索也可能包含静态页面，跳过非漫画永久链接。无缩略图的旧帖读取正文区分漫画和公告。英文搜索主题不输出翻页控件，满 20 张原站卡片时提供下一页。 |
| 详情和章节 | 上游把每条漫画作为年度归档的一章。官网每篇都有固定日期与 slug。 | 英文帖子是一部作品和一个正文阅读单元，站内搜索命中后可直达。详情使用当前帖子的标题，章节保留其日期和链接。 |
| 正文 | 上游读取 `.entry-content p img`。 | `get_pages` 限于当前帖子正文及官网图片目录，排除页眉、归档缩略图和小图标，包括旧帖正文末尾的小幅招聘横幅，保留原漫画长图。 |
| 账号和设置 | 英文漫画公开。 | 匿名读取。设置为空。 |

最低 App 版本为 0.1.0。图标见 [ASSET_SOURCES.md](ASSET_SOURCES.md)。
