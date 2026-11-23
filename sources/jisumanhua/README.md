# 极速漫画

## 0.1.1 簡介與多語言顯示

極速漫畫的原創與日本漫畫，包含熱血冒險、奇幻、戀愛及校園等題材；原站鎖定或下架章節依其狀態保留。

零參 `source_info` 固定使用以上基準簡介；`metadata.localization` 僅新增 `source.description` 的繁中、簡中與英文文案，供支援的宿主顯示。

参考 [Keiyoushi Jisu Manhua](https://github.com/keiyoushi/extensions-source/tree/f1d882a93ef34a33dc4f4467753ad5f99eab43f4/src/zh/jisumanhua) 与同一提交的 DM5 模板。

热门、更新与搜索使用桌面站，作品保留数值作品路径，章节保留站点的完整 `/ch…/` 路由。保留各组目录及顺序，区分原站下架和锁定。封面和正文各用所属页面的 Referer；章节需要动态地址时在请求单页时解析站点返回的图片数据。

章节评论用原站的 pagerdata，通过独立原站评论面板按页读取。
