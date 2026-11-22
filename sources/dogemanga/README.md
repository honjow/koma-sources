# 漫畫狗

参考 [Keiyoushi Doge Manga](https://github.com/keiyoushi/extensions-source/tree/f1d882a93ef34a33dc4f4467753ad5f99eab43f4/src/zh/dogemanga)。热门和最近更新沿用服务器的下一页游标，关键词页用真实的 24 条偏移量。目录卡片忽略原站空卡。详情和章节仍以原始作品、章节 ID 为准，界面语言固定为英文以可靠判断原站作品状态。

同一作品的「全部」可混排不同卷、章和版本。保留上游的全部目录数组、标题和原始作品／章节 ID，只按章节业务 ID 去重，不按标题或编号合并。图片继续直接使用站点提供的逐页地址。

## 完整目录与阅读位置

2026-11-23 对照上述固定上游 `toSChapters()`、五部作品的官网原件及正式包结果：官网 `site-manga__tab-pane-all`、`issue`（话）、`tankobon`（卷）均按新到旧排列。海贼王的 797 个唯一条目实际分为 729 话和 68 卷；不能把混合的「全部」倒排成同一阅读组。

`run_get_chapters` 仍以 All 的首次出现顺序发布完整目录，用网站实际标签的成员关系建立 `dogemanga:<作品ID>:<标签ID>` 独立组，位置从各组最旧条目起计 0。标题、无编号、公告或番外不参与续读许可判断；同标题的不同业务 ID 全部保留。仅出现在 All 的条目保留为独立 All 子序列，不虚构话／卷类型。现有网站没有给出译本等价合同，不能仅因编号或标题相同就合并替代版本。

SDK 仅在宿主声明 `chapter_reading_order_v1` 时发出位置；旧宿主仍得到原始 ID、标题、缺失编号和显示次序。分页、详情英文界面、5 次／秒站点限速、原图请求及权限路径不变。阅读正向由位置决定，列表反序不能改变位置。

| 调用链 | 本源采用的合同 |
| --- | --- |
| 热门／更新／关键词 | 上游 `getCards` → `directory`，保留服务器 next 游标与 24 条搜索偏移 |
| 详情 | 上游 `fetchMangaPage` → `work`／`detail`，保留 `?l=en` 与真实作品 ID |
| 目录 | 上游 `toSChapters` → `run_get_chapters`，保留 All 身份／数组，另用官网标签给完整组位置 |
| 阅读／图像 | 上游 `getPageList` → `pages`／`image_request`，保留页面声明及 DOM 图像顺序 |


## 2026-11-23 章序候选验证

候选 0.1.1 已用既有 dev CLI 实际联网读取五部作品的 1401 章，对照官网原件逐条核对 ID、标题、All 显示次序、话／卷成员关系及每组完整 0 起位置。旧宿主未声明能力时的原目录身份／次序仍一致；两个正文页表分别保持 36 页和 121 页，非法作品 ID 在请求网站前拒绝。实测证据：`/Users/honjow/git/Koma/.hvigor/outputs/published-source-order-audit-20261123/runtime-2/order-fix-validation/results.json`。这些是候选 WASM 与页表结果，不是 App／设备阅读验收，未发布。
