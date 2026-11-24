# 漫本

## 0.1.1 簡介與多語言顯示

漫本收錄港台、日韓、大陸及歐美漫畫，包含搞笑、奇幻、熱血、科幻與戀愛等題材，並區分少年、少女及青年向作品。

零參 `source_info` 固定使用以上基準簡介；`metadata.localization` 僅新增 `source.description` 的繁中、簡中與英文文案，供支援的宿主顯示。

参考 [Keiyoushi Manben](https://github.com/keiyoushi/extensions-source/tree/f1d882a93ef34a33dc4f4467753ad5f99eab43f4/src/zh/manben)。分类列表和关键词搜索各自使用站点公开表单接口，按真实返回条数翻页。分类、地区、读者、状态和排序在无关键词时生效。保留站点作品与章节路径。

详情和目录由普通作品页提供，各组按网站顺序读出并保持同组续读。阅读解析站点 JavaScript 中的图片列表数据，每张图沿用对应章节页的 Referer。

## 原站评论

原站移动作品页以 `/showcomment/?mid=…` 打开作品评论，首屏由该页 HTML 提供；向下滚动后 `loadComment.js` 向 `/showcomment/pagerdata.ashx` POST `t=4、mid、pageindex`，从第 2 页开始，返回空数组时停止。Koma 的 `get_comments` 使用 SDK 的 `source_comments_v1` 请求与游标合同，读取同一入口和续页，仅声明作品评论。章节页的评论按钮也跳转作品评论，未声明章节独立评论。

实站样本中，评论页首屏和接口第 1 页内容不一致；站点自己的续页从第 2 页开始，可能漏掉接口第一页中未出现在首屏的条目。此处沿原站流程，不将接口第一页伪装成网页首屏或宣称它们一致。
