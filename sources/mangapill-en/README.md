# Mangapill

英语漫画书库，收录《ONE PIECE》《Berserk》《Gachiakuta》等日漫、短篇、同人及其他英语作品。原站已停止韩漫支持；不自行补入已移除的类型。版本 0.1.0，最低 Koma 1.5.5；完整五语栏目、筛选与错误信息。图标见 [ASSET_SOURCES.md](ASSET_SOURCES.md)。

## 移植前对照与保留边界

固定上游 [MangaPill.kt](https://github.com/keiyoushi/extensions-source/tree/c09b174459c037c62292e4e7a929a0cd59430bc4/src/en/mangapill)，并在 2026-11-28 直接实测首页、搜索表单、筛选结果、详情及多年份正文。调用链根据本仓 SDK、开发宿主与 docs/WRITING_A_SOURCE.md 对照；本次不访问 Koma 仓库。

| 行为 | 上游／原站 | Koma 调用与消费 |
| --- | --- | --- |
| 热门、最新 | 首页 Trending Mangas；/chapters 最近更新。均无网站分页。 | get_manga_list → list，精准选取作品卡片并按作品路径去重。latest 中同一作品多次更新只保留首次出现。两栏目不暴露无效分页或筛选。 |
| 搜索与完整筛选 | GET /search，q、page、type、status，重复 genre 参数对应复选框。实测 46 个题材、6 种类型、5 种状态；没有排序字段。 | search → list；genre 为 multiselect，type/status 为 select，保留官网原值与五语标签。关键词与筛选可组合，页码只来自 args.page.cursor；只在结果区存在明确 Next 链接时续页。 |
| 详情 | h1、作品封面、简介、Type/Status/Year 与题材链接。上游 author/artist 为空；当前官网未提供作者字段。 | get_manga → detail；准确映射状态和类型标签，保留简介和题材。原站有 Author/Artist 字段时读取，缺失时返回空数组，不猜测作者或额外查询外站。 |
| 全量章节 | #chapters > div > a，页面完整倒序目录，编号可含小数及附录。 | typed get_chapters → chapters；按真实路径去重，保持原站顺序，ChapterResponse::ordered 声明 newest_first；不根据标题重排，不按重复章号删除不同章。 |
| 正文 | 章节页 picture img.js-page[data-src]，正文路径 /file/mangap/ 或 /file/mangapill/。 | get_pages → pages；严格选取正文元素和路径，去除重复 URL，过滤 UI、广告和 favicon；附 Referer: https://mangapill.com/。get_image_request 复用同样媒体校验与 Referer。 |
| CDN | 实测 One Piece、Berserk 与 Gachiakuta 的新旧章节均使用 cdn.readdetectiveconan.com。 | 已核实的 CDN 精确放行；正文也允许官网 DOM 给出的其他 HTTPS CDN 域名，但必须同时满足独有的 /file/mangap/ 或 /file/mangapill/ 正文路径与图片扩展名，拒绝凭据、端口、IP 和非正文路径，避免因固定域名过窄丢失不同年份漫画。封面采用 /file/mangapill/i/ 专属路径。 |
| 登录、设置 | 无账号或用户设置要求；原站小说类型保留在表单。 | 无独立首页或设置。小说条目若没有漫画图片返回 chapter_unavailable，不混入站内其他图片假装可读。 |

保留作品／章节的完整原站路径作为稳定身份。禁止通过猜测 CDN 路径或作者填补原站缺失数据，也不得把网络失败、解析失败或不完整章节当作空成功。

## 验证

最终包内 source.wasm 的在线调用、五语筛选、完整章节与真实图片下载／解码写入 dist/wave-10/verification.json。App 书源列表及真机普通阅读路径因本次不操作 Koma 而保留待验。
