# 嬉皮漫畫

## 0.1.1 簡介與多語言顯示

嬉皮漫畫的韓漫、國漫與日漫書庫，涵蓋穿越、玄幻、復仇、冒險與武俠等題材。

零參 `source_info` 固定使用以上基準簡介；`metadata.localization` 僅新增 `source.description` 的繁中、簡中與英文文案，供支援的宿主顯示。

参考 [Keiyoushi Xipi Manhua](https://github.com/keiyoushi/extensions-source/tree/f1d882a93ef34a33dc4f4467753ad5f99eab43f4/src/zh/xipimanhua)。搜索、列表和章节使用站点 JSON 接口，沿用地区、类型、状态和排序，关键词搜索合并同作品的别名索引。作品保留原始 mid。

章节保留完整 hid，解码其中的所属作品和章节身份再请求页表。复用网页的页表排列和占位条目剔除规则。线路 1 和线路 2 继续使用站点默认 CDN，并兼容线路 9 的图床变体。该源限速沿用上游 API 的每秒四次。
