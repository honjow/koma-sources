# コミックボーダー

リイド社（LEED）旗下的官方漫画站，刊载《皮肉屋魔神と死にたがりの少女 シン・悪魔の辞典》《カムヤライド》等作品，每周五更新。作品按官网首页顺序展示，可阅读当前免费公开章节。

源 ID `com.comicborder.ja.koma`，语言 ja，safe，0.1.0，最低 Koma 1.5.5。真实运营方：**リイド社 / LEED PUBLISHING Co., Ltd.**；官网页脚版权已实网核对，原始 HTML 与请求哈希在 dist/wave-19/evidence/intake.json。

## 移植前的源码对照

固定 [Keiyoushi c09b174459c037c62292e4e7a929a0cd59430bc4](https://github.com/keiyoushi/extensions-source/tree/c09b174459c037c62292e4e7a929a0cd59430bc4/src/ja/comicborder)。已实际读取 ComicBorder.kt、build.gradle.kts，同版 GigaViewer.kt、Filters.kt、Dto.kt、Queries.kt、ImageInterceptor.kt 和 ExtensionPlugin.kt。build 声明精确 name=`Comic Border`、lang=`ja`、baseUrl=`https://comicborder.com`，默认 versionId=1；versionCode 不参与来源哈希。宿主调用链只读对照 SDK source_protocol、runtime、chapters 及 tools/koma-source-dev/src/host.rs；未操作 Koma 仓库或设备。

| 行为 | 上游语义与 Koma 调用链 |
| --- | --- |
| 发现与分类 | 首页严格按 section.top-series 顺序读取 h3、.top-key-image[data-src]、li.first-episode a, li.this-episode a。优先复用 Site::series_id_from_thumbnail；无有效缩略图 ID 时只读同源 episode 页 script.js-valve[data-giga_series]。所有 card 取真实数字 series ID，去重保留首次位置，随后详情、章节和阅读全部复用 Site。supportsLatest=false：仅 popular listing，supportsFilters=false、capabilities.filters=false、get_filters 返回空数组；不伪造 latest 或首页能力。空关键词搜索回到官方首页，非空关键词使用共享 GraphQL searchSeries。 Koma get_manga_list → 本源 listing；分类空搜索 → listing_search 或 browse::search。 |
| 搜索 | 非空关键词 → Site::search → GraphQL searchSeries；保持上游纯数字 databaseId 身份及仅卷作品排后规则。 |
| 分页 | 原上游首屏全量，Koma 按 args.page.cursor 每 80 项切片；锚点校验防目录变动漏项/重排，末页 nextCursor=null。章节由共享 typed chapters 完整分页。 |
| 详情 | get_manga → Site::detail → series(databaseId)，作者、介绍、封面和原站首话链接；作品 ID `comicborder:<seriesId>`。 |
| 章节 | get_chapters → Site::chapters → pagination_readable_products，包含话与单行本。稳定 ID `comicborder-ch:<seriesId>:episode:<productId>`，volume 分支对应 type；保持站方完整顺序，不用标题推断缺章。 |
| 权限与设置 | 默认仅按 purchase_info.can_read/unavailable 区分公开、锁话与下架；hide_locked / hide_unavailable 默认 false。明确不启用 Zenon 缺失 status 策略。get_pages 独立核实实际 viewer 权限，锁话返回 chapter_locked。 |
| 阅读 | get_pages → episode-json readableProduct，只收 type=main；已核对 baku/rtl。get_image_request 校验同源 CDN 正文路径，提供 Referer 与 wasm_image_v1；缺宿主解码能力返回 host_feature_required。 |
| 图片处理 | 打包复用 koma_gigaviewer_tile_processor，原图解码 RGBA 后执行随包 WASM，与独立 4×4 转置参考逐像素比对；保留未参与拼块的边缘。 |
| 本地化 | metadata Catalog → port_catalog::Labels，五语 listing/filter/setting/chapter/error；固定零参 source_info 为日语，presentation 与 source.description 同目录投影。 |
| 停止条件 | 网络/状态/解析失败返回结构化错误，空节点专门适配仅限已说明的 Seasons 分类；不绕过登录/付费，不混入推荐图，不伪造无用功能。 |

## 分类身份

| Koma value | 固定上游名称 | 真实节点 |
| --- | --- | --- |
| 不提供 | 上游无分类 | 无 |

## Mihon 身份

精确计算键 `comic border/ja/1`：UTF-8 MD5 前 8 字节按大端序累积，与 0x7fffffffffffffff 取与，得到 **`1077630719925082256`**。metadata.identity.mihon.work 把当前裸 series ID 加 `comicborder:`；chapter 的 targetIdSegment=3 对齐裸 readable_product_id。原站卡片 episode 不是作品 ID；先解析真实 series ID，之后与搜索结果身份一致。不使用 Koma 显示名或 versionCode 计算 Mihon ID。

## 实网与候选包

初探样本《皮肉屋魔神と死にたがりの少女 シン・悪魔の辞典》，series `2551460909751203802`，首话 https://comicborder.com/episode/2551460909751211416，11 页。封面主机 `cdn-img.comicborder.com`；正文主机 `cdn-img.comicborder.com`，标准 scissors 包装由共享模块解出。最终测试实际提取 dist/wave-19/sources/comicborder-ja/comicborder-ja-0.1.0.koma 内 source.wasm，执行 info、五语、列表、适用筛选、搜索、详情、完整章节、设置、页表、图片请求与实际随包 WASM 还原。结构化记录在 dist/wave-19/verification.json；候选包保留供后续真机 QA，未提交、推送或操作设备。
