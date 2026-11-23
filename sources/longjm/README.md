# 龙禁漫 0.1.4

独立公开 HTML 适配 `com.longjm.koma`，API 0.2，主页 `https://longjm.com`。中文成人内容，源级 nsfw；单作品分级与用量未知。当前支持热门、最近更新、新书发布、推荐漫画、热搜榜、周榜、日榜、月榜、总榜，以及关键词搜索、详情、目录、页描述和图片请求。不登录、不写 Cookie。

## 栏目与请求

- 热门：`/bookrank/weekly`，可在设置里改为日榜或月榜；默认周榜。
- 最近更新：`/newbook`
- 新书发布：`/newbook/newbook`
- 推荐漫画：`/newbook/recommend`
- 热搜榜：`/hotsearch`
- 周/日/月/总榜：`/bookrank/weekly|daily|monthly|all`
- 搜索：`/search/{关键词}`，按时间排序，支持 `/page/N` 分页；当前站点不提供分类或连载状态筛选，源不显示这两项。
- 详情/目录：`/manhua/{id}.html`
- 阅读：`/manhua/{id}/{hash}.html`，正文图优先取原 URL；源声明 `img.nnpic.xyz` 404 时回退到 `thumb.niaopic.com` 的同一路径。最低 App 1.4.2。

## 设置

- `preferredDomain`：备用域名，默认 `https://longjm.com`，共 15 个站点提供的备用域名。
- `popularRank`：热门使用的排行榜，默认周榜，可选日榜/月榜；不影响独立榜单，不提供总榜映射。

## 边界

列表页为单页返回，没有可用的 `page` 路由；源以 `hasMore:false` 返回全部卡片。搜索支持站点自身的 `page/N` 分页。站点搜索页虽然显示“已完结/更新中”和分类链接，但实际请求返回同一结果集，源不把这两项伪造成筛选。

## 章节续读

从官网完整播放目录保留原序，单独声明连续阅读位置。《异界猎妻人》的第 31 话、另一段第 1 话及第 32 话之间，官网上一章／下一章链接与目录顺序一致，因此不按话号重排。无法确认使用同一个完整播放目录时保留手动选章。

## 可选列表补充信息（0.1.4）

协商 `source_manga_list_info_v1` 后，同一张列表卡片已有的 `.module-item-note` 更新原文
通过 SDK `list_info::append` 写入有序 `listInfo` 的 text 项。保持原 `subtitle`、作品 ID、
封面、数组顺序、分页和设置；不增加请求，不猜章节号，不按源名或域名添加宿主特例。
不加固定 label；文本直接复用原 provider 内容，未来本地化标签也由源按请求已有 locale 处理。
无 feature 时不产生字段。空白、无效 UTF-8、超过512字节或 optional 容量不足时保留
原列表项并省略增强。最低 App 仍1.4.2。真实 `/newbook` 原件的88条更新文本用于
新实际 WASM 离线回放；UI与Native展示由宿主验收，不把回放作为设备成功。
