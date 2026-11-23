# 熱辣漫畫

## 0.1.1 簡介與多語言顯示

熱辣漫畫收錄戀愛、冒險、奇幻、校園等題材。普通瀏覽為免費作品，搜尋亦可能列出付費作品，其鎖定預覽不等於完整正文。

零參 `source_info` 固定使用以上基準簡介；`metadata.localization` 僅新增 `source.description` 的繁中、簡中與英文文案，供支援的宿主顯示。

参考 [Keiyoushi Relamanhua](https://github.com/keiyoushi/extensions-source/tree/f1d882a93ef34a33dc4f4467753ad5f99eab43f4/src/zh/relamanhua)。普通列表仅浏览免费作品，按热度或更新时间排序；筛选精确传递站点题材和升降序。关键词用独立搜索入口，可返回付费作品，不套用浏览筛选。

详情以站点 `path_word` 识别，章节以作品及 UUID 识别。逐分组分页取全目录，每组保留自己的阅读顺序；付费锁定的预览不作为完整正文。作品网页链接及直接输入原站链接的搜索沿用站点现行桌面和移动路由。
