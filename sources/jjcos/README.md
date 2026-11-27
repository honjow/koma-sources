# JJCOS

角色扮演图集与日本、韩国写真。原站含 R18 分类，源与作品分级均为 `nsfw`，语言为 `all`。最低 Koma 版本为 1.5.5。

对照固定版 [Keiyoushi JJCOS](https://github.com/keiyoushi/extensions-source/tree/c09b174459c037c62292e4e7a929a0cd59430bc4/src/all/jjcos)、本仓 FoamGirl、MissKon、共享 `photo_gallery.rs` 和 [Koma 运行 ABI](https://github.com/honjow/Koma/blob/master/docs/source-runtime-abi.md)。

| 行为 | 上游及原站 | Koma 路径 |
| --- | --- | --- |
| 最新、分类与标签 | 上游以全量静态索引顺序作为默认列表；官网按日期列出每页 20 个相册，分类本身是固定标签页。 | `get_manga_list` → `browse` → `listing` 读取真实分页 HTML。支持官网四个分类以及标签 ID 或标签链接，以原站页数停止。 |
| 搜索 | 上游下载全量 `/api/index.html` 后在标题和内容内匹配。实测服务器忽略 `page`、`query` 和 Range，约 11.9 MB。 | 全站 `search` → `search_index` 按上游匹配标题与正文、每页 20 条。官方原始索引用搜索专属入站缓冲接收，在缓冲内解码，逐条读取并只保留当页命中。选定分类或标签时改为按该归档显示的标题和标签查找，每次最多四个原站分页。 |
| 详情和虚拟单册 | 上游用标题、首图、标签及发布日期，单帖为一章。 | `detail` 与 `photo_gallery.rs` 的 `Gallery` 和 typed `get_chapters`。 |
| 页图和相册边界 | 上游从 `#post-content` 取图并去重。官网相邻 Set 编号可能是独立帖，底部导航也可直接指向另一相册。 | `get_pages` 仅接受同一帖的顺序子页，保留首次出现的图，遇到坏掉的同帖序列返回错误。当前现网页面样本每帖已内嵌完整图集，多子页与末页跳他册通过离线回放覆盖。 |
| 登录及源设置 | 扩展不提供。 | 普通匿名 HTTP，复用标准错误合同。 |

五语覆盖包展示资料、栏目、分类筛选、标签输入提示、单册名称和源错误。

全站搜索使用 Koma 1.5.5 普通 HTTP 的 API 23+ NetworkKit 路径，当前原始下载约 11.9 MB，单个源响应仍低于 192 KiB。索引体积超过 16 MiB 入站预算时返回错误。
