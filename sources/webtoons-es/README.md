# WEBTOON（西班牙语）

源 ID `com.webtoons.es.koma`，语言 `es`，分级 safe。收录 WEBTOON 西语 ORIGINALS（例如《Lector omnisciente》）与 CANVAS 作者作品，限原站匿名公开章节。

| 行为 | 上游 → Koma → 本源实际消费 |
| --- | --- |
| 语种入口 | `Webtoons.kt` 西语实例 → `get_manga_list` / `search` → `list`，所有常规入口 `/es/`。作品 ID `manga:webtoons:es:{webtoon或canvas}:{titleNo}:{完整站内list路径}`，章 `chapter:webtoons:es:{viewer路径}`，页 `page:webtoons:es:`。 |
| 热门与最新 | 热门按 trending、popular、originals、canvas 四榜分页；游标由 `args.page.cursor` 读取。最新是设备本地星期对应 `/es/originals/{weekday}?sortOrder=UPDATE`，仅协商 `source_private_session_v1` 时展示，调用只读 `clock`，没有账号操作。 |
| 搜索 | `/es/search?keyword=`，分 ORIGINALS / CANVAS 继续分页；Type 参数 `searchType`。非空关键词时只应用 Type，与上游保持一致。 |
| 筛选 | 当前 `Filters.kt` 的 Type、更新日、题材、排序 → `get_filters` / `browse.rs`。空词筛选按日期优先、再题材；26个题材来自西语官网实际路由，CANVAS 特有项明确标注。`MANA` / `LIKEIT` / `UPDATE`；CANVAS 按实际页码续页，协商空词搜索能力时返回 `useSearchForFilters`。 |
| 详情与完整目录 | HTML `get_manga` 核对 title_no/路径；`get_chapters` 使用移动站目录 API 按完整公开话集读取并检查归属，CANVAS 翻译目录保留 `readingLanguageCode=es` 验证，不把英语原章混入。 |
| 正文 | `get_pages` 只读取 `#_imageList` 正文 `img[data-url]`，保留原站随章发布的警告页，排除推荐与界面资源；`get_image_request` 附站方 Referer，最大画质去掉原站压缩参数。没有瓦片还原。 |
| 设置与编号 | `useMaxQuality` 默认关，`useSequentialNumbering` 默认关，`showAuthorsNotes` 默认关。西语显式 Episodio/Capítulo/Ep./# 数字用于阅读编号，否则空；选择顺序编号才按目录序号，不把 episode_no 错当公开章号。保留现有 Koma 参考的保守编号适配，未照搬当前上游 Season/Bonus 的复杂名称推断。作者后记仅在宿主协商文本页能力时可选追加，章内评论只读现有合同。 |

上游西语 ID 依据 `webtoons.com/es/1` 为 `8664735734660636295`。本源 ID 携带种类、title_no 和路径，通用 Mihon 单前缀映射不足以无损转换，因此不虚报 `identity.mihon` 或 `samePageOrder`。评论模块复制现有 WEBTOON 实现并独立适配原站 `Language: SPANISH`，未修改旧源。



开发运行器新增独立只读 `private_session` / `clock` 响应（`tools/koma-source-dev/src/clock.rs`），使用操作系统本地时钟，无网络、账号或会话模拟；其他私有操作仍拒绝。由此最新栏目也通过 `koma-source-dev` 运行最终包内 WASM，另保留 Wasmtime 补充验证证据。

## 移植前的源码对照

固定上游 [Keiyoushi extensions-source `c09b174459c037c62292e4e7a929a0cd59430bc4`](https://github.com/keiyoushi/extensions-source/tree/c09b174459c037c62292e4e7a929a0cd59430bc4/src/all)。实际对照该平台 Kotlin 源、Filters、DTO 与 `build.gradle.kts`，并沿仓库 `sdk/src/runtime.rs`、`sdk/src/source_protocol.rs`、导出宏、`tools/koma-source-dev/src/host.rs` 复核宿主请求和结果。宿主 `args`、`args.page.cursor`、`settings`、`hostFeatures` 与 `environment` 均由源入口实际消费；没有独立首页，不将热门首屏声明为首页。

通用栏目、设置、筛选和错误提供简中、繁中、英、日、韩五语；作品、章节和站方动态标签保留原文。零参 `source_info` 使用 metadata 同目录中的固定英文基准简介，安装资料由 metadata 投影。HTTP 响应头不参与解析，故不存在将宿主数组头值误作字符串的路径。

版本 `0.1.0`，最低 Koma `1.5.5`。本轮仅交付宿主实网验证的候选包，未操作设备、未验证 App 列表图标呈现、未发布。包内 WASM 调用、图片 HTTP 状态、尺寸与 SHA-256 见 `dist/wave-14/verification.json`。图标工艺见 [ASSET_SOURCES.md](ASSET_SOURCES.md)。
