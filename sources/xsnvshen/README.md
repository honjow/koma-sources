# 秀色女神

## 0.1.1 簡介與多語言顯示

秀色女神的模特寫真相冊，可按模特、服飾、風格、地區與拍攝機構尋找圖集。

零參 `source_info` 固定使用以上基準簡介；`metadata.localization` 僅新增 `source.description` 的繁中、簡中與英文文案，供支援的宿主顯示。

参考 [Keiyoushi Xsnvshen](https://github.com/keiyoushi/extensions-source/tree/f1d882a93ef34a33dc4f4467753ad5f99eab43f4/src/zh/xsnvshen)。热门、最新与服饰、风格、地区、机构标签均使用移动网页。搜索模特名返回单页。相册、模特与标签链接可按原站路径识别。

每个相册有一章，按 HTML 选择器中的真实页数拼接原图。每张正文图带所属相册分页的完整 Referer。到最后一页停止，避免站点绕回第一页。图集详情使用站点单独开放的缩略封面地址。
