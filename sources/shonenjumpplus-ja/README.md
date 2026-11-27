# 少年ジャンプ＋

集英社的日文连载和读切服务，可读原站当前公开的正文。图标记录见 [ASSET_SOURCES.md](ASSET_SOURCES.md)。

基线为 [ShonenJumpPlus.kt](https://github.com/keiyoushi/extensions-source/tree/90b6e6a905ca186945b21ec0b0be4ca9135b8e69/src/ja/shonenjumpplus) 和 [GigaViewer / ImageInterceptor.kt](https://github.com/keiyoushi/extensions-source/tree/90b6e6a905ca186945b21ec0b0be4ca9135b8e69/lib-multisrc/gigaviewer)。

| 上游行为 | Koma 操作和源内入口 |
|---|---|
| `fetchSeriesPage` | `get_manga_list` 读取连载、读切及完结栏目。今日更新从宿主时钟计算东京星期，只读取对应标题后的列表。读切和完结按原网页顺序用光标续页，跳过专题与站外链接。 |
| `SEARCH_QUERY`、`SERIES_QUERY` | `search` 和 `get_manga` 使用共享协议查询，校验 GraphQL errors 和字符串 ID。缩图可识别系列时保留系列 ID，原站只给出产品路径时在详情解析系列。 |
| `fetchReadableProducts`、两个可见性开关 | `get_chapters` 按 `totalCount` 分别请求话、卷，Koma resolver 回传 `args.page.cursor`。续页核对前一条记录，原站排序变化会显式失败。锁定与下架分开，默认可见。带章排除能力的新宿主消费隐藏设置。 |
| `getPageList` 和 `ImageInterceptor` | `get_pages` 从 viewer HTML 校验公开话、系列和产品归属，只采用 `main` 页。保持页序，默认 RTL 由宿主消费的 `Manga` 格式标签表达。阅读页用普通桌面请求头取得已验证的 viewer 版型。图片描述从稳定页身份回查公开 viewer，`baku` 通过 `requiredImageDecode.pixels` 传给随包 WASM。 |

[共享模块](../gigaviewer.rs) 持有协议读取和校验。[处理器](../../image-processors/gigaviewer-tiles/src/lib.rs) 按上游 4×4 块坐标转置并原样复制余边。最低 Koma 1.5.5，其他站点适配、购买、登录、统计写入及其他拼块模式均超出此版范围。
