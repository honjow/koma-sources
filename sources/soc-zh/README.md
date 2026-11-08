# 铃兰之剑中文・官方漫画（SOC 0.1.1）

提供铃兰之剑官方认证发布者在 TapTap 的《伊利亚没问题！》中文漫画。游戏官网为 https://soc.xd.cn/ 。官方漫画目录是 https://www.taptap.cn/app/209866/topic?group_label_id=946587&type=group_label 。网站于 2026-11-06 公开目录共 6 页 57 个不同作品。数量随站点变化。

## 上游与宿主调用对照

| 功能 | 官网依据 | Koma 调用与源内消费 |
| --- | --- | --- |
| 官方漫画目录 | 普通网页的 `link[rel=next]`、`link[rel=prev]` 与 Nuxt SSR `list → item.moment` | 保留 `listing:current-first-page` 身份，`run_get_manga_list → listing → catalogue`。`BrowseViewModel.loadMoreBrowse` 送 `args.page.cursor`，每次只请求当前一页。以同目录真实下一页链接续页，末页无下一页链接并返回 `hasMore:false` |
| 标题搜索 | 同一官方目录的原始标题 | 保持现有标题子串查询，`run_search` 按相同游标查询每一页。空命中但有后页仍返回游标，`BrowseViewModel.runSearch/loadMoreSearch` 自动穿过空页 |
| 详情、目录与阅读 | 原 `/moment/<id>?group_id=291080`，唯一主 `moment + first_post`、原图与正文 DOM | `request_work → fetch_work` 直接检查具体作品的官方身份、固定分组和图像，不为保存的 ID 重扫目录。保留原作品、唯一章节及第 0 张图片的 ID。章节页数以正文实际张数为准 |
| 登录、筛选、设置与独立首页 | 此公开漫画目录没有对应专用入口 | 现有能力声明保持 |

每条作品验证官方标记、发布者 391283780、游戏 209866、分组 291080、标签 946587、官方认证理由以及原文章链接。标题通常为「【伊利亚没问题！】」，也保留同一作者和分类实际刊登的「伊利亚没问题！这次真的没问题！！」。6 页完整回放无重复。源只使用公开网页导航，不调用站内 API 的签名游标。

每篇为一个完整阅读单元，`chapterNumber/volumeNumber=null`，`readingGroup` 为原 MID，`continuationEligible=false`。保留原章、页次序和完整原图。官方发布者与每篇实际创作者是不同字段，逐篇作者仍留空。源内保持原 HTML 模板、身份校验、Nuxt 索引与描述符回收限制。

## 正文与封面

详情校验 canonical URL、发布者与作品身份，将 topic 和首条正文的原图数组逐项与正文 DOM 对照。正文懒加载图以对应节点的 `data-src` 对照，网络图片只采用 SSR 明示的 `original_url`。仅接受原 HTTPS `img2-tc.tapimg.com/moment/etag/` 单层安全文件名与 PNG/JPEG 原格式、非零元数据尺寸。保留现有首图作为封面。普通匿名图片声明随封面输出空 `headers`，可由新宿主直接请求，旧宿主使用 `get_image_request`。显式封面 ID 仅允许本作品首图，正文页 ID 绑定正确的作品及图序。无 ID 的旧封面请求仍只核原首屏。独立的 `mangaUrl` 存在时须匹配完整原文章 URL。

2026-11-06 原网页 57 个详情 HTTP 200，生产 WASM 在本地 C++ HtmlHost 对照 6 个目录页、57 个详情和章节、85 个原图及请求描述。使用末页独有的「中秋特别篇」核对连续空页、命中与终止。另有 10 部作品共 22 张独立原图实际 HTTP 200 并解码，包含单图、多图、懒加载、不同尺寸和 JPEG。「六一篇」原正文的第二张是发布者附图，官网将它列入同一正文图组并展示，源按原顺序保留。237 当前候选待复验。

## 原图标

图标采用官网 `author.app.icon.original_url` 声明的游戏专属像素画 PNG https://img-tc.tapimg.com/market/images/f8342359df8fa63b0de1600629035bb0.png 。实际 512×512、23419 字节，SHA256 `02562c854d245c33876e9ee8700c6ff29b57f3157f334328449a6df9d314428a`。此版与原正式包图标完全一致，画面背景、像素边缘和图案保留。

## 2026-10-25 Native 首篇验证

237（VDE-AL00，展开竖屏）使用 Koma 0.1.2-pre 的正式签名 Release 包，正常安装本源0.1.0后，浏览页显示当前首屏10篇。进入实际首篇“行动观察日记”，详情、章节、取页及图片描述均完成；正文 HTTP 200，收到7,619,323字节，完整四格长图按一页显示。原图加载时有进度提示；正常返回后原书架和所有旧阅读记录保持，只新增本次作品的正常阅读记录。

本次只确认这一首屏及首篇正文，不代表其余9篇正文、下载、冷启动、后续47条或整源性能均已通过。封面当前也使用原图，本次单张为数MB至十余MB，不能据本数出现即称封面加载充分优化；原URI校验和有限目录合同保持。
