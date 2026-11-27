# サンデーうぇぶり · Sunday Web Every

小学馆官方漫画站。作品含《名侦探柯南》等连载、短篇及「夜 Sunday」。正文跟随原站匿名权限，付费和下架的话保留在目录并标记锁定。需要 Koma 1.5.5 的随包图片还原能力。

## 移植对照

参考 Keiyoushi [SundayWebEvery](https://github.com/keiyoushi/extensions-source/tree/c09b174459c037c62292e4e7a929a0cd59430bc4/src/ja/sundaywebevery) 及同版 [GigaViewer](https://github.com/keiyoushi/extensions-source/tree/c09b174459c037c62292e4e7a929a0cd59430bc4/lib-multisrc/gigaviewer)。

| 行为 | 上游和官网协议 | Koma 接入 |
| --- | --- | --- |
| 热门、更新 | 九个 SerialGroup 一次 GraphQL 聚合。热门按点赞，更新按最新话发布日期。 | `get_manga_list` 调 `listing` 和共享 `collection`，按原站顺序构造稳定条目，`args.page.cursor` 用前一项 ID 校验续页。 |
| 短篇、夜 Sunday | 短篇用单独的 GraphQL 组。夜 Sunday 用官网 `/series/yoru-sunday` 的 HTML，接口分组没有这一栏。 | 栏目和空词筛选共用各自的目录。夜 Sunday 封面可推导系列 ID，缺封面时保留 viewer 路径并在详情解析系列。 |
| 关键词、筛选 | 非空词调用 `searchSeries`，连载优先于仅单行本的作品。原站分组只在空词搜索生效。 | `search` 调共享关键词实现或按 `filters.collection` 读取对应目录。 |
| 详情、话数与卷册 | 使用 `series(databaseId)` 查询统计，再从 `pagination_readable_products` 读取话数、卷册及权限。 | `get_manga` 和 `get_chapters` 复用 `Site`，每页校验前一项身份。锁定与下架均输出 `locked: true` 和标题首部 `🔒 `。 |
| 页图 | 官网 viewer 的 `episode-json` 提供公开态、正文页与还原模式。实测桌面配置为 `baku`。 | `get_pages` 要求匿名公开态。`get_image_request` 从官方 viewer 重新确认当前页和访问状态，然后调用包内 `gigaviewer-tiles` 像素阶段。锁定及下架返回 `chapter_locked`。 |
| 设置、登录 | 上游默认显示付费和下架话，可分开隐藏。目录和公开正文走匿名请求。 | 两项开关均默认关闭。支持章节排除的宿主可隐藏已知锁定项，五语目录、选项与错误来自同一文案表。 |
