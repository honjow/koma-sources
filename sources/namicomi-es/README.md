# NamiComi（西班牙语）

源 ID `com.namicomi.es.koma`，公开语言 `es`，分级 safe。收录创作者在 NamiComi 发布的西班牙语章节，例如《RE: united》的西语版；站方真实语言标识为 `es-es`。

| 行为 | 上游 → Koma → 本源实际消费 |
| --- | --- |
| 语言契约 | `NamiComi.kt` 西语实例 `lang=es, extLang=es-es` → `get_manga_list` / `search` → `list`。API `availableTranslatedLanguages[]=es-es`，目录 `translatedLanguages[]=es-es`；宿主章节语言 `es`。 |
| 列表与分页 | `/title/search`，热门浏览量降序、最新与关键词默认发布时间降序，每页20项，游标保留 offset、total 一致性检查。非空搜索默认仅含可读作品，浏览默认全部，分别用 `availableSearch` / `availableBrowse` 消费。 |
| 筛选 | `Filters.kt` → `get_filters` → `filter_schema` / `apply_filters`。可读性、分级、状态、排序、标签包含/排除及 and/or；`fetchFilterData` 与宿主动态/三态能力共同启用 `/title/tags`，未协商时保留静态筛选。 |
| 详情 | `/title/{id}` → `get_manga` → `detail`；标题/简介优先 `es-es`、再 `es`、`en`、最后站方首个值。公开页 `/es/title/…`。 |
| 目录与阅读 | `get_chapters` 按卷章双降序完整读取，每批200并核对总量、组织和语言；`/gating/check` 的匿名映射决定章可见性。`get_pages` 再复核 gating，并读取 `/images/chapter/{id}?newQualities=true`，只接收站方 CDN 与返回源文件名。保留分组、卷号、小数章与稳定阅读顺序。 |
| 设置与评论 | 封面 `thumbnailQuality_es` 默认 original，可选512/256；`dataSaver_es` 默认关，低流量使用站方独立数组。显示锁定默认关且协商目录排除；评论只读同章并保留分页和剧透过滤。 |
| 登录范围 | 保持既有 Koma 匿名边界，不搬运上游账号凭据回退。 |

Mihon 西语 source ID `7482985666811016125`，按固定上游 `ExtensionPlugin.kt` 的 `namicomi/es/1` 计算并用英文既有 ID 交叉验证。独立前缀 `namicomi:es:`、`namicomich:es:`、`namicomipage:es:`；作品裸 ID 与章节末段映射沿上游契约。未改动任何既有 NamiComi 源。


## 移植前的源码对照

固定上游 [Keiyoushi extensions-source `c09b174459c037c62292e4e7a929a0cd59430bc4`](https://github.com/keiyoushi/extensions-source/tree/c09b174459c037c62292e4e7a929a0cd59430bc4/src/all)。实际对照该平台 Kotlin 源、Filters、DTO 与 `build.gradle.kts`，并沿仓库 `sdk/src/runtime.rs`、`sdk/src/source_protocol.rs`、导出宏、`tools/koma-source-dev/src/host.rs` 复核宿主请求和结果。宿主 `args`、`args.page.cursor`、`settings`、`hostFeatures` 与 `environment` 均由源入口实际消费；没有独立首页，不将热门首屏声明为首页。

通用栏目、设置、筛选和错误提供简中、繁中、英、日、韩五语；作品、章节和站方动态标签保留原文。零参 `source_info` 使用 metadata 同目录中的固定英文基准简介，安装资料由 metadata 投影。HTTP 响应头不参与解析，故不存在将宿主数组头值误作字符串的路径。

版本 `0.1.0`，最低 Koma `1.5.5`。本轮仅交付宿主实网验证的候选包，未操作设备、未验证 App 列表图标呈现、未发布。包内 WASM 调用、图片 HTTP 状态、尺寸与 SHA-256 见 `dist/wave-14/verification.json`。图标工艺见 [ASSET_SOURCES.md](ASSET_SOURCES.md)。
