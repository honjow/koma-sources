# カドコミ · KadoComi

角川官方漫画站（原 ComicWalker）。作品来自多个漫画厂牌，包括《转生成蜘蛛又怎样！》。目录显示原站锁定状态，阅读限匿名公开话。正文处理需要 Koma 1.5.5。

## 移植对照

参考 Keiyoushi [KadoComi](https://github.com/keiyoushi/extensions-source/tree/c09b174459c037c62292e4e7a929a0cd59430bc4/src/ja/kadocomi) 及同版 [XorInterceptor](https://github.com/keiyoushi/extensions-source/blob/c09b174459c037c62292e4e7a929a0cd59430bc4/core/src/main/kotlin/keiyoushi/network/XorInterceptor.kt)。官网首页实为 Next.js 页面，目录和 viewer 仍使用该域名下的 JSON 接口。

| 行为 | 上游和官网协议 | Koma 接入 |
| --- | --- | --- |
| 热门、更新、续页 | `/api/ranking` 和 `/api/series/new` 提供 `limit=30`、`offset` 与总量。 | `get_manga_list` 的下一游标是服务器偏移，按总量结束。 |
| 关键词、筛选 | `/api/search/keywords` 合用 `keywords` 和 `sortBy=popularity/update`。空搜索也可带排序。 | `search` 消费关键词、排序与游标。支持筛选搜索的宿主可单独提交排序，普通栏目保留官网各自排序。 |
| 详情与章节 | `/api/contents/details/work?workCode=…` 提供作品资料和 `latestEpisodes`。按 `isActive` 区分不可读话。 | `get_manga` 映射作者、作画、状态和更新文案。`get_chapters` 对照总量，每页最多 100 项并核对上页末项；锁定同时输出状态和标题 `🔒 `。 |
| 单话权限 | 官网详情调用 `/api/contents/details/episode`，使用 `workCode`、`episodeCode`、`episodeType=latest`。实测活动话与下架话各有对应的 `isActive`。 | `get_pages` 先校验单话归属和公开权限。不可读话返回 `chapter_locked`。 |
| 页图、XOR | `/api/contents/viewer?episodeId=…&imageSizeType=width:1284` 提供短期 CDN 地址及每页 `drmHash`。公开章返回 `xor`，不可读话返回空 `manuscripts`。 | `get_image_request` 按稳定路径取回当前签名地址。密钥仅作为 `requiredImageDecode.bytes` 传给包内 XOR 模块；分块处理中维持掩码位置，保留有效 WebP 字节。 |
| 设置、登录 | 上游的隐藏不可读章节默认关闭，普通公开目录和正文按匿名访问。 | 宿主支持章节排除时使用该设置。界面、错误与下次更新标签支持五语。 |
