# 90漫画网

## 0.1.1 簡介與多語言顯示

90漫畫網收錄日本、韓國、國漫、港台及歐美漫畫，題材包含戀愛、校園、冒險與科幻，兼有連載及完結作品。

零參 `source_info` 固定使用以上基準簡介；`metadata.localization` 僅新增 `source.description` 的繁中、簡中與英文文案，供支援的宿主顯示。

参考 [Keiyoushi 90Manhua](https://github.com/keiyoushi/extensions-source/tree/f1d882a93ef34a33dc4f4467753ad5f99eab43f4/src/zh/jiulingmanhua) 及同提交的 SinMH 模板。桌面列表提供热门、更新、关键词搜索与动态筛选，移动作品页提供详情和章节。词搜沿用原站搜索结果，分类与排序用于浏览。

正文按移动阅读页显示的总页数建立页表。第一张图随章节返回，后续图片进入对应网页时才解析，保留原站每次只读取单页的方式。章节和分页都使用站点路径作为身份依据。沿用上游主站每秒两次请求的限速。

筛选、列表、词搜、作品与逐页正文共用本源的 HTML 请求。Koma #023 在 201 复现了 15 秒超时，公开阅读页曾在超过 50 秒后返回有效 HTML，因此本源使用宿主允许的 60 秒等待上限。页图下载沿用宿主原有图像请求。

Mihon 的 `SinMH.fetchMangaUpdate` 一次读取移动详情并解析目录。Koma 详情页声明 [同事务目录上下文](https://github.com/honjow/Koma/blob/master/docs/source-runtime-abi.md) 时，`manga_data` 从同一份 HTML 生成完整章表，`run_get_chapters` 核对协议版本、源版本、作品路径及各组条目后，由同一个目录输出器生成章节。保留原章路径、归属、去重顺序、组内位置及最新章时间。旧宿主、独立目录请求、精确章恢复或缺失、无效、过大的上下文走原移动详情请求。上下文只存于当次详情事务。
