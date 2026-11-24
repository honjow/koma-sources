# CCC追漫台

参考：[Keiyoushi `Creativecomic.kt`、DTO 及构建配置](https://github.com/keiyoushi/extensions-source/tree/8947c372cd87f546a0b80cb0e4b0a5a8add5ef88/src/zh/creativecomic)。上游 `zh-Hant` v4 来源 ID `8346821279138091413`；作品和章节沿用同一个数字 ID，章节图片采用相同页序。分发原件见 `ASSET_SOURCES.md`，上游许可见 `LICENSE` 和 `NOTICE`。

| 上游行为 | Koma 入口与实现 |
| --- | --- |
| 热门、最新和站内关键词搜索 | `get_listings` → `get_manga_list`；`search` 传原词和 `category=all`。`list` 读取 `args.page.cursor`、保留 24/12 条页长、`class=2` 及点赞/更新时间排序，用 `total` 判末页。没有上游独立首页、设置或筛选。 |
| 作品标题、封面、作者、简介、类型与标签、完成状态 | `get_manga` → `details` 调 `/book/{id}/info`；详情来源链接进入官网 `/zh/book/{id}/content`。 |
| 目录反转、显示卷名及章节名、日期、可读标志 | `get_chapters` → `catalog`，按上游倒转返回 `ChapterResponse::ordered`；锁定以四个原站字段判定并传入 `Chapter.locked`。标题沿用 Mihon 的锁形前缀，保证目录当前即可识别受限章节。 |
| 匿名免费阅读 | `get_pages` → `pages` 先从该作品目录校验章节归属及匿名可读，保留 `proportion` 页序。`get_image_request` → `image` 从官网签发只用于该图的参数，并由随包 `koma_ccc_page_processor` 还原站点下发的正文。签发参数只在字节阶段请求中，页表及图片 URL 均无密钥。宿主缺少 `source_required_image_decode_wasm_v1` 时返回不兼容；封面仍是普通 URL。 |
| 网页已登录后读取 `accessToken`、检查到期并用 Bearer 访问 | Koma 书源设置及章节登录复用官网验证码。Host 在精确官网 origin 读键，向精确 API origin 验证并保存当前代，WASM 只取得代号和逐图签发所需的派生材料。目录、授权检查、页表和签发随 API 请求使用该代；没有有效代时走游客，受限章返回登录或锁定。仅支持已获授权的阅读，登录过期可回官网确认。 |

历史样本为《無能戀愛諮商中心》作品 83、免费章节 178（21 页）；`ccc-discovery/REPORT.md` 同时保留两部其它免费章的首图实解证据。历史证据和本包验证分别记录，且不代表 App 的实际阅读或离线验收。

2026-11-19 使用本地编译的 WASM 验证：热门首页和下一页、最新首页均为 24 项；搜索命中作品 83，匿名目录返回 76 章（6 章锁定）。章 178 保留 21 页，首图 16659 用独立 WASM 处理器恢复为 1200×2400 JPEG，解码字节与历史样本一致。锁定章未发正文请求，返回 `chapter_locked`；未声明必需解码能力的请求返回 `unsupported_feature`。处理器也拒绝截断和取消输入。197 已在普通 App 路径验证免费章和从锁章直接打开官网登录表单；201 已验证免费章离线。实际登录后的会员阅读及授权离线仍待验证。

会员交接依赖 `source_web_token_v1` 宿主，含该清单的候选包需与实现该能力的 Koma 配套验证。已有匿名证据保留原验收范围，会员正文与离线权益仍需实账号验证。
