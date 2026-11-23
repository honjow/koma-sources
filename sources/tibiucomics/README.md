# TIBIU漫画

## 0.1.2 簡介與多語言顯示

TIBIU 的 BL 向漫畫書庫，收錄連載及完結作品，閱讀範圍限原站公開章節。

零參 `source_info` 固定使用以上基準簡介；`metadata.localization` 僅新增 `source.description` 的繁中、簡中與英文文案，供支援的宿主顯示。

- 栏目按原站 API 的人气、更新时间分页；关键词、连载状态和排序可组合。上游未启用分类页，没有引入其他分类入口。
- 详情从作品 ID 取得原站标题，再用标题查询并按 `/comic/{id}` 匹配同一作品；作品和章节链接都可映射回作品。
- 目录保留上游反转后的显示顺序和每条 `/chapter/{work}/{chapter}` 身份，时间按上海时区。完整原目录的位置独立生成 typed `readingOrder`；无话号、同标题但不同业务 ID 的章节继续保留邻章关系。只按业务章节 ID 去重，不按标题或编号删除，也不由重名猜测译本组。
- 正文走原站阅读器的图片 API，按 `xid` 排序。图片 API 拒绝或未返回图片时报告不可读。上游没有会员登录、购买或私有接口路径。
- Mihon ID 为 `6028517731014373149`。作品 `/comic/{id}` 映射到裸 ID，章节保留原路径；图片排序沿用上游。

0.1.0 的历史验证（2026-11-19）：热门首页和下一页、最新首页均为 30 项；搜索命中《蜂蜜味的S级向导》作品 17125。目录 131 章，含同名但不同 ID 的目录项，全部保留且该目录没有自动续读关系。公开章 `/chapter/17125/394910` 返回 121 页，首图下载解码为 2665×3200 WebP。未测试会员内容或 App 安装、离线。

237（Koma 1.5.4）验证热门、关键词搜索、作品封面及 131 章目录，从普通阅读入口读到正文首图，阅读器显示 1/109。


## 0.1.1 章序合同与源码对照

依据固定上游 [TibiuComics](https://github.com/keiyoushi/extensions-source/blob/8947c372cd87f546a0b80cb0e4b0a5a8add5ef88/src/zh/tibiucomics/src/eu/kanade/tachiyomi/extension/zh/tibiucomics/TibiuComics.kt) 和同提交的 MCCMS；不改变栏目、筛选默认值、详情定位、图片与访问权限。

| 行为 | 上游与提供方 | Koma 消费位置 |
| --- | --- | --- |
| 浏览、更新、搜索 | MCCMS 的页码、size、order、key 与 serialize 查询；TIBIU 不启用分类页 | `list` 消费 `args.page.cursor`、listing/query/filters，沿原停页条件 |
| 详情、身份 | comicinfo 取得标题后，以作品路径匹配搜索结果；章节身份为原路径中的业务 ID | `details`、`manga_id`、`chapter_ids`；已有作品、章节 ID 不重写 |
| 目录、邻章 | `fetchChapterList` 读取完整 `/api/comic/chapter?mid=` 后反转显示；原站 `chapterget` 与 `getSortedChapterList` 按 xid 正向读取完整目录，`findAdjacentChapter` 按 ID 找前后项，不按标题去重 | `run_get_chapters(args.mangaId)` 先校验所有行，再按业务 ID 去重，保留反转显示，以完整目录位置输出同组 `ChapterPlacement::InGroup` |
| 阅读、权限 | 原图片 API、xid 图片顺序；未获准或无图保持不可读 | `pages` 与原图片请求；没有新增登录、购买或私有接口 |

相同标题不是版本身份；只有提供方明确的版本元数据才能拆分阅读组。0.1.0 仅因重名就把整目录标为 `ManualOnly`，131 项均未输出明确阅读位置。2026-11-23 公开作品 17125 的 131 项中有 49 组重名，业务 ID 互异；漫画目录顺序与原站 data/chapter 按 xid 排序逐 ID 一致。0.1.1 保留这些实体及原显示顺序，将读取方向与显示方向分开。旧宿主不声明 `chapter_reading_order_v1` 时继续沿用原 ABI 字段。


0.1.1 实际验证（2026-11-23）：5 部作品共 491 项目录，全部阅读 ID 顺序与原站按 xid 排序逐项一致；正式包的 ID、标题、日期和显示数组不变，重名实体全部保留，每部阅读位置完整且唯一。作品 17125 的旧宿主响应与 0.1.0 逐字段一致，候选公开章返回 109 页页表。验证仅为真实联网 WASM 与页表，不代表 App 安装、图片逐页或真机验收。
