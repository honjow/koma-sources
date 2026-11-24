# Hanime1.me

参考 [Keiyoushi Hanime1](https://github.com/keiyoushi/extensions-source/tree/f1d882a93ef34a33dc4f4467753ad5f99eab43f4/src/zh/hanime1)。上游漫画域名为 `hanimeone.me`，范围为 `/comics`。

- 浏览按漫画专区的「發燒漫畫」「最新上傳」两个区域提取。热门只有一页。最新和搜索按站点下一页链接继续。
- 搜索沿用 `query`、`page` 和可选的 `sort` 参数，排序提供最新、日热门、周热门、总热门。
- 详情保留作者或社团、分类和封面。目录保留站点展示的当前与关联集数，无相关集数时按单章读取。关联集数没有可靠的阅读顺序，作为手动选章。
- 阅读按首图的图片前缀以及逐页缩略图的真实文件扩展名拼接 URL。同一章节可以混用 JPG 和 WebP。
