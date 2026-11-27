# GlobalComix（法语）

源 ID `com.globalcomix.fr.koma`，语言 `fr`，NSFW。收录 GlobalComix 法语独立漫画与创作者发行作品，例如《Entwined》；阅读范围依原站匿名授权，不能读取的章节不会绕过权限。

| 行为 | 上游 → Koma → 本源实际消费 |
| --- | --- |
| 语言与列表 | `GlobalComix.kt` 法语实例，`lang_id[]=fr` → `get_manga_list` / `search` → `list`。热门和最新使用站方排序，搜索按相关性；`args.page.cursor` 保留页号。`payload.results` 同时承载列表和单对象。 |
| 详情与身份 | `/comics/{id}` → `get_manga` → `detail`；`gc:fr:` 独立作品，`gcch:fr:` 独立章节，`gcp:fr:` 独立页。详情核对法语版本，章节核对 `lang_id=fr` 与作品归属。 |
| 目录及权限 | `/comics/{id}/releases?lang_id=fr&all=true` → `get_chapters` → `chapters`。免费、Gold、预览和锁定依返回授权字段；显示锁定章节只在宿主协商目录排除能力后启用，默认与现有 Koma 参考一致为开。 |
| 阅读和图片 | `/readV2/{uuid}` → `get_pages` → `pages` 再核对作品、发行 ID 和授权；桌面/移动图片数组由 `dataSaver_fr` 决定。`get_image_request` 返回站方公开客户端头和 Referer，无图片瓦片还原。 |
| 设置和评论 | `get_settings` → `settings`，低流量默认关、显示锁定按能力协商。此平台没有筛选 schema，使用设置，不伪造筛选；`get_comments` 只读同发行 `/releases/{id}/comments`。 |
| 登录范围 | 沿现有 Koma 英文源匿名访问边界，不移植上游账号凭据、购买与会话写入。 |

法语 Mihon source ID `1331353277450289630`：依据固定上游 `ExtensionPlugin.kt` 对 `globalcomix/fr/1` 的 MD5 前八字节大端序取正 63 位计算；同公式核对既有英文源 ID 一致。`identity.mihon.work.idPrefix` 为 `gc:fr:`，并沿当前可对应的章节稳定段匹配。没有改动任何既有 GlobalComix 源。


## 移植前的源码对照

固定上游 [Keiyoushi extensions-source `c09b174459c037c62292e4e7a929a0cd59430bc4`](https://github.com/keiyoushi/extensions-source/tree/c09b174459c037c62292e4e7a929a0cd59430bc4/src/all)。实际对照该平台 Kotlin 源、Filters、DTO 与 `build.gradle.kts`，并沿仓库 `sdk/src/runtime.rs`、`sdk/src/source_protocol.rs`、导出宏、`tools/koma-source-dev/src/host.rs` 复核宿主请求和结果。宿主 `args`、`args.page.cursor`、`settings`、`hostFeatures` 与 `environment` 均由源入口实际消费；没有独立首页，不将热门首屏声明为首页。

通用栏目、设置、筛选和错误提供简中、繁中、英、日、韩五语；作品、章节和站方动态标签保留原文。零参 `source_info` 使用 metadata 同目录中的固定英文基准简介，安装资料由 metadata 投影。HTTP 响应头不参与解析，故不存在将宿主数组头值误作字符串的路径。

版本 `0.1.0`，最低 Koma `1.5.5`。本轮仅交付宿主实网验证的候选包，未操作设备、未验证 App 列表图标呈现、未发布。包内 WASM 调用、图片 HTTP 状态、尺寸与 SHA-256 见 `dist/wave-14/verification.json`。图标工艺见 [ASSET_SOURCES.md](ASSET_SOURCES.md)。
