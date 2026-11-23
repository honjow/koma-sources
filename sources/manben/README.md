# 漫本

## 0.1.1 簡介與多語言顯示

漫本收錄港台、日韓、大陸及歐美漫畫，包含搞笑、奇幻、熱血、科幻與戀愛等題材，並區分少年、少女及青年向作品。

零參 `source_info` 固定使用以上基準簡介；`metadata.localization` 僅新增 `source.description` 的繁中、簡中與英文文案，供支援的宿主顯示。

参考 [Keiyoushi Manben](https://github.com/keiyoushi/extensions-source/tree/f1d882a93ef34a33dc4f4467753ad5f99eab43f4/src/zh/manben)。分类列表和关键词搜索各自使用站点公开表单接口，按真实返回条数翻页。分类、地区、读者、状态和排序在无关键词时生效。保留站点作品与章节路径。

详情和目录由普通作品页提供，各组按网站顺序读出并保持同组续读。阅读解析站点 JavaScript 中的图片列表数据，每张图沿用对应章节页的 Referer。
