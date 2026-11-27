# MangaTaro

英语日漫、韩漫与国漫书库，包含 One Piece、Gachiakuta 等，按题材、类型与连载状态分类。最低 Koma 1.5.5。小说不在本源范围内。

## 移植前的源码对照与适配

固定上游 [Keiyoushi c09b1744 的 MangaTaro](https://github.com/keiyoushi/extensions-source/tree/c09b174459c037c62292e4e7a929a0cd59430bc4/lib-multisrc/mangataro) 与 [英文入口](https://github.com/keiyoushi/extensions-source/tree/c09b174459c037c62292e4e7a929a0cd59430bc4/src/en/mangataro)，实际阅读 `MangaTaro.kt`、`Dto.kt`、`Filters.kt`。另对照官网 browse/detail 的正式 JS 和 JSON 响应，不能因为 multisrc 名称推断普通 WordPress 或 Madara 协议。

| 行为 | 上游与原站 | Koma 调用与消费 |
| --- | --- | --- |
| 热门、最新 | POST `/wp-json/manga/v1/load`，`popular_desc`/`post_desc`，24 条一批。数组筛选值本身是 JSON 字符串。 | `get_manga_list → list → api`；`args.page.cursor` 为 1 起页码，原始返回不足 24 条停止；跳过 Novel，按作品 ID 去重。 |
| 搜索、筛选 | 上游关键词默认 `/auth/search`（最多25条），勾选后才走过滤接口；官网 load 同时支持 search、genres、types、statuses、sort。 | `search → list` 始终使用可组合过滤、可分页的 load。提供 16 个常用题材、三种漫画类型、连载/完结及八种排序，均为实测官方稳定值；未移植数百标签、年份和多选匹配开关。各字段和选项五语本地化。 |
| 详情 | `/wp-json/wp/v2/manga/{id}?_embed` 的正文、featuredmedia、term；WP `status=publish` 仅表示可见性。 | `get_manga → detail`；补取作品 HTML 的实际连载状态，不把 publish 当 ongoing；重复 HTML 实体最多还原三次。原站无封面占位地址返回 kind=none，保留作品且不访问外部占位图服务。 |
| 全量章节 | `/auth/manga-chapters`；`offset/limit=500/order=DESC`，`has_more` 决定结束。时间令牌取服务器时钟。 | typed `get_chapters → chapter_page`；先从无令牌响应的 Date 取当前 GMT，兼容宿主字符串数组与开发工具单字符串（头名不区分大小写，取首个非空字符串并去除两端空白），缺失、空值或错误类型不回退设备时钟；构造与官网相同的 MD5 前16位令牌。累计原始条数推进 offset，仅去重相同章节地址，保留不同 ID/译者；仅收录英文。原站仅空白的章节副标题按缺省处理，不送入空 HTML 解析，章节记录保留。完整倒序交给 `ChapterResponse::ordered`，不按标题数值重排。 |
| 正文 | `/auth/chapter-content?chapter_id=` 的 images 数组。 | `get_pages → pages`；只接受 `mangataro.yachts/storage/chapters/` 的真实图，去除重复地址，不扫描整个页面 img，因此不混入 favicon、广告或推荐。图片与封面请求通过页图 headers / `get_image_request` 统一附 `Referer: https://mangataro.org/`。 |
| 统计、登录、设置 | 上游额外 POST 阅读计数、提供部分用户组配置。 | 不上报计数，不接账号或组源，settings 为空，无独立首页；不影响匿名目录与图片。 |

保持原站 DESC 全目录的顺序、章节稳定 ID 和英文范围是兼容边界；不能因重复章号删除不同版本，也不能在请求失败时提前结束并把残缺目录当成功。改变这些行为须有原站实测依据或用户明确决定。

## 验证

逐项结果见本地 `dist/wave-9/verification.json`，包括超过 500 条目录的翻页、筛选、搜索与真实页图解码。App 列表和真机普通阅读路径受本次不触碰 Koma 的约束，保留待验。

响应头兼容回归通过 `tests/test_date_header.py` 直接执行最终 WASM 的 `get_chapters`，检查固定服务器 Date 的字符串/数组形式都生成相同令牌，并覆盖空值、错误类型及无效日期。运行方式：`python3 sources/mangataro-en/tests/test_date_header.py --package dist/wave-9/sources/mangataro-en/mangataro-en-0.1.0.koma`（需要 Python wasmtime）。
