# 俊美图

按模特、写真机构和主题整理的真人写真图库，兼收街拍与男性写真。含成人内容，源及图集均标记 NSFW；每套图集是一个完整章节。最低 Koma 1.5.5。

## 移植前的源码对照与适配

对照固定的 [Keiyoushi Junmeitu](https://github.com/keiyoushi/extensions-source/tree/c09b174459c037c62292e4e7a929a0cd59430bc4/src/all/junmeitu) 的 `Junmeitu.kt`，以及现网 HTML；参考 `jjcos` 的 `photo_gallery.rs` 表示和 `leercapitulo-es` 的 HTML 筛选。宿主请求形状对照本仓 SDK 和 runner，未触碰 Koma。

| 行为 | 上游与原站 | Koma 调用与消费 |
| --- | --- | --- |
| 热门、最新、分类 | `/{category}/hot-{page}.html`、`index-{page}.html`；美女、帅哥、街拍。 | `get_manga_list → browse`；`args.page.cursor` 为1起页码，`.pages` 的下一页决定结束，禁止用右栏推荐当列表。 |
| 搜索、筛选 | 上游 `search/{keyword}-{page}.html`，tag/model/xzjg 三类归档；搜索优先。现网搜索表单为 `/search/?q=`。 | `search → browse` 用原表单 query 和 page；支持分类、排序与 tag/model/group 代码。三项代码互斥，关键词存在时忽略归档筛选，提示五语说明；标签可填原站代码或规范 `/tags/*.html` URL，转分类分页路径。 |
| 详情与单章 | `.title/.news-title`、`.pictures/.news-body`、`.relation_tags`。 | `detail` 返回完整正文封面、原站标签与写真标记；`photo_gallery::run_get_chapters` 生成单个完整图集章节。 |
| 全分页 | 原 `.pages a` 有所有分页边界（含末页），`.pages span` 标识当前页。上游可能转 AJAX 懒取图片。 | `get_pages` 只读取原站第一页，按真实末页数发布完整目录，首图直接取原图。其余页面经现有 `get_image_request` 按需请求原站分页，逐页核对同套地址、原始标题、末页数、当前页和唯一正文原图。请求描述中的目录校验参数在实际发请求前移除。单页失败由该页报告和重试。 |
| 全尺寸图 | `i.wujituku.com/jmt/...`、街拍及 `s.wujituku.com` 原图。 | `media(full=true)` 拒绝 `thumb_*`、`/re/` 派生图及 statics；目录封面可用缩略图，正文绝不混入。图片请求附官网 Referer。每个分页保留独立页身份，解析失败不掩盖漏页。 |
| 登录、设置、首页 | 公开 GET，不接账号或下载统计。 | 五语栏目/筛选/错误；settings 为空；不把热门包装成独立首页。 |

整套分页完整性、正文容器边界和全尺寸图筛选属于本源兼容约定。不得为减少请求截断长图集，或以成功首图掩盖中间页失败；变化需原站实测依据或用户明确允许。最多 1000 页，超过预算明确返回错误。

## 验证

本地 `dist/wave-9/verification.json` 记录真实多页图集的 HTML 总页数、最终页图数与解码结果。素材原件、恢复稿和最终包已按浅深背景对照；本任务禁止触碰 Koma，App 书源列表和真机路径待验。原站对已结束搜索再手工请求下一页可能返回 HTTP 200 的 SQL 错误页；源返回解析错误，不伪装成功空列表，普通分页遵守 hasMore=false 后停止。
