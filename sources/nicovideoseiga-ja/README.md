# ニコニコ漫画

日本出版社官方连载与用户投稿平台，收录《ワンパンマン》《蜘蛛ですが、なにか？》等作品。最低 Koma 1.5.5。只读取匿名公开话，保留需登录、收费和已结束公开章节的锁定状态。

## 移植前的源码对照与适配

固定参考版本为 Keiyoushi `extensions-source` 的 `c09b174459c037c62292e4e7a929a0cd59430bc4`；该快照没有此源实现，不能冒称已沿用其解析器。本源以 2026-11-28 抓取的[原站搜索表单](https://manga.nicovideo.jp/search?q=%E7%8C%AB)、作品 HTML 以及 [app API](https://api.nicomanga.jp/api/v1/app/manga/contents/3580) 的实际响应为协议依据，同时对照 `kadocomi-ja` 的图片请求与 SDK 类型化章节导出。宿主请求形状对照本仓 SDK/runner，未读取或修改 Koma 工程。

| 行为 | 原站证据与停止条件 | Koma 调用与消费 |
| --- | --- | --- |
| 热门、最新、分类 | `contents/ranking`、`contents`；`limit=20&offset`，`category` 为官方分类值。`data.extra.has_next` 结束。 | `get_manga_list → list → api`；`args.page.cursor` 是从 1 起的批次；支持 `listing:` 前缀。分类不翻译请求值。 |
| 搜索、排序 | 官方 API 的试探 `search` 路径返回空对象，`contents` 的 keyword/query/title 参数被忽略；改用真实 `/search?q=&sort=&page=`。11 个排序值来自原表单，下一页链接决定结束。 | `search → list` 只解析 `.search_result__item`。`get_filters` 在列表返回分类，在搜索返回排序。没有确认的标签接口不伪造。 |
| 详情 | `contents/{id}`，`meta` 提供封面、作者、简介、分类和连载状态。 | `get_manga → detail`；简介去 HTML，原站正文不翻译。 |
| 全目录 | `contents/{id}/episodes` 返回完整 oldest-first 目录；包含下架记录，不能以作品 counter.episode 当全目录长度。 | typed `get_chapters` 保留每个业务 ID；公开话写原目录位置，锁定话 `ManualOnly`，保留 `locked` 与锁图标。 |
| 锁定 | `is_playable=false`、非 free sell_status、付费、结束公开均不开放匿名阅读。 | `get_pages/get_image_request → frames` 先核对 episode 所属作品与权限；限制统一 `chapter_locked`，不返回空页。HTTP 限制须有站点 JSON 错误信号，CF/未知拒绝仍走标准错误。 |
| 帧与图片 | `episodes/{id}/frames` 的 `source_url` 和 `drm_hash`；无 hash 是明文。 | `get_pages → frame`；页 ID 绑定章节和索引。`get_image_request` 重新取帧并匹配域名/路径，前 16 个 hex 字符作为 `requiredImageDecode.bytes`；协商必需还原能力后复用 `koma_kadocomi_xor_processor`，不建新处理器。 |
| 登录、设置、首页 | 不提供账号授权或额外偏好；排行榜不是独立首页。 | settings 为空；home 不支持；全源面向用户的固定资料、栏目、筛选和错误五语本地化。 |

原站读取范围、锁定语义和 XOR 配置属于本源兼容边界。只有原站协议的实测变化或用户明确决定才能更改，不能把不可读章节当网络失败或从目录中静默删除。

## 验证

复现命令与逐项证据保存在本地 `dist/wave-9/verification.json` 与 `dist/wave-9/research/`。最终包中图标另有浅深底核查；受本任务禁止触碰 Koma 的约束，App 列表呈现和真机普通阅读路径待验。
