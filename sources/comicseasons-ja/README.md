# Seasons

文藝春秋旗下的恋爱漫画站，刊载《悪役令嬢のお嫁さま》《ぼくは青くて透明で》等作品，也提供原著小说试读。每周五更新，阅读范围以原站免费公开章节为准。

源 ID `com.comicseasons.ja.koma`，语言 ja，safe，0.1.0，最低 Koma 1.5.5。真实运营方：**文藝春秋 / Bungeishunju Ltd.**；官网页脚版权已实网核对，原始 HTML 与请求哈希在 dist/wave-19/evidence/intake.json。

## 移植前的源码对照

固定 [Keiyoushi c09b174459c037c62292e4e7a929a0cd59430bc4](https://github.com/keiyoushi/extensions-source/tree/c09b174459c037c62292e4e7a929a0cd59430bc4/src/ja/comicseasons)。已实际读取 ComicSeasons.kt、build.gradle.kts，同版 GigaViewer.kt、Filters.kt、Dto.kt、Queries.kt、ImageInterceptor.kt 和 ExtensionPlugin.kt。build 声明精确 name=`Comic Seasons`、lang=`ja`、baseUrl=`https://comic-seasons.com`，默认 versionId=1；versionCode 不参与来源哈希。宿主调用链只读对照 SDK source_protocol、runtime、chapters 及 tools/koma-source-dev/src/host.rs；未操作 Koma 仓库或设备。

| 行为 | 上游语义与 Koma 调用链 |
| --- | --- |
| 发现与分类 | 完整保留固定上游 seriesListIds 与 getFilterOptions 顺序。popular 按 likeCount 降序，latest 按 latestEpisode.publishedAt 降序；空关键词按原合集顺序，关键词非空时按上游忽略分类并以 searchSeries 搜索。 实网「シリアス」「シンデレラ」「偽装」「ループ」四个固定上游 Genre 节点返回 null。仅本源 collection() 对已登记 Genre 读取 nullable 节点：不存在时返回合法空列表，存在时将系列边交给共享 series_items 排序与映射。保留全部分类身份，节点恢复后自动读取，不修改共享模块；两个 SerialGroup 仍走共享 collection。 Koma get_manga_list → 本源 listing；分类空搜索 → listing_search 或 browse::search。 |
| 搜索 | 非空关键词 → Site::search → GraphQL searchSeries；保持上游纯数字 databaseId 身份及仅卷作品排后规则。 |
| 分页 | 原上游首屏全量，Koma 按 args.page.cursor 每 80 项切片；锚点校验防目录变动漏项/重排，末页 nextCursor=null。章节由共享 typed chapters 完整分页。 |
| 详情 | get_manga → Site::detail → series(databaseId)，作者、介绍、封面和原站首话链接；作品 ID `comicseasons:<seriesId>`。 |
| 章节 | get_chapters → Site::chapters → pagination_readable_products，包含话与单行本。稳定 ID `comicseasons-ch:<seriesId>:episode:<productId>`，volume 分支对应 type；保持站方完整顺序，不用标题推断缺章。 |
| 权限与设置 | 默认仅按 purchase_info.can_read/unavailable 区分公开、锁话与下架；hide_locked / hide_unavailable 默认 false。明确不启用 Zenon 缺失 status 策略。get_pages 独立核实实际 viewer 权限，锁话返回 chapter_locked。 |
| 阅读 | get_pages → episode-json readableProduct，只收 type=main；已核对 baku/rtl。get_image_request 校验同源 CDN 正文路径，提供 Referer 与 wasm_image_v1；缺宿主解码能力返回 host_feature_required。 |
| 图片处理 | 打包复用 koma_gigaviewer_tile_processor，原图解码 RGBA 后执行随包 WASM，与独立 4×4 转置参考逐像素比对；保留未参与拼块的边缘。 |
| 本地化 | metadata Catalog → port_catalog::Labels，五语 listing/filter/setting/chapter/error；固定零参 source_info 为日语，presentation 与 source.description 同目录投影。 |
| 停止条件 | 网络/状态/解析失败返回结构化错误，空节点专门适配仅限已说明的 Seasons 分类；不绕过登录/付费，不混入推荐图，不伪造无用功能。 |

## 分类身份

| Koma value | 固定上游名称 | 真实节点 |
| --- | --- | --- |
| all | 作品一覧 | 2550912964592144518 |
| novels | 原作小説試し読み | 2550912964592144524 |
| tag_2550912964583261787 | 溺愛系 | Genre:2550912964583261787 |
| tag_2550912964583261798 | 逆転系 | Genre:2550912964583261798 |
| tag_2550912964583261807 | 日常系 | Genre:2550912964583261807 |
| tag_2550912964583261815 | ファンタジー | Genre:2550912964583261815 |
| tag_2550912964583261826 | BL | Genre:2550912964583261826 |
| tag_2550912964583261831 | イッキヨミ | Genre:2550912964583261831 |
| tag_2550912964583261841 | 毎週更新 | Genre:2550912964583261841 |
| tag_2550912964583261851 | 完結 | Genre:2550912964583261851 |
| tag_2550912964583261853 | メディア化 | Genre:2550912964583261853 |
| tag_2550912964583261855 | コミカライズ | Genre:2550912964583261855 |
| tag_2550912964583261858 | 受賞作 | Genre:2550912964583261858 |
| tag_2550912964583261866 | 話題作 | Genre:2550912964583261866 |
| tag_2550912964583261867 | 読み切り | Genre:2550912964583261867 |
| tag_2550912964583261871 | 一途 | Genre:2550912964583261871 |
| tag_2550912964583261881 | ヤンデレ | Genre:2550912964583261881 |
| tag_2550912964583261887 | 片想い | Genre:2550912964583261887 |
| tag_2550912964583261894 | 両想い | Genre:2550912964583261894 |
| tag_2550912964583261897 | ドロドロ | Genre:2550912964583261897 |
| tag_2550912964583261906 | シリアス | Genre:2550912964583261906 |
| tag_2550912964583261912 | 復讐 | Genre:2550912964583261912 |
| tag_2550912964583261921 | 不倫 | Genre:2550912964583261921 |
| tag_2550912964583261922 | 浮気 | Genre:2550912964583261922 |
| tag_2550912964583261931 | モラハラ | Genre:2550912964583261931 |
| tag_2550912964583261933 | シンデレラ | Genre:2550912964583261933 |
| tag_2550912964583261939 | ハイスペ | Genre:2550912964583261939 |
| tag_2550912964583261946 | 元カレ | Genre:2550912964583261946 |
| tag_2550912964583261948 | イケメン | Genre:2550912964583261948 |
| tag_2550912964583261952 | 偽装 | Genre:2550912964583261952 |
| tag_2550912964583261957 | 結婚 | Genre:2550912964583261957 |
| tag_2550912964583261964 | 和風 | Genre:2550912964583261964 |
| tag_2550912964583261965 | 再会 | Genre:2550912964583261965 |
| tag_2550912964583261966 | ギャップ | Genre:2550912964583261966 |
| tag_2550912964583261973 | 悪役令嬢 | Genre:2550912964583261973 |
| tag_2550912964583261978 | ループ | Genre:2550912964583261978 |
| tag_2550912964583261986 | モフモフ | Genre:2550912964583261986 |
| tag_2550912964583261995 | 家族 | Genre:2550912964583261995 |

## Mihon 身份

精确计算键 `comic seasons/ja/1`：UTF-8 MD5 前 8 字节按大端序累积，与 0x7fffffffffffffff 取与，得到 **`5198063407173750376`**。metadata.identity.mihon.work 把当前裸 series ID 加 `comicseasons:`；chapter 的 targetIdSegment=3 对齐裸 readable_product_id。原站卡片 episode 不是作品 ID；先解析真实 series ID，之后与搜索结果身份一致。不使用 Koma 显示名或 versionCode 计算 Mihon ID。

## 实网与候选包

初探样本《回って避ける》，series `12207421984513305450`，首话 https://comic-seasons.com/episode/12207421984513315666，28 页。封面主机 `cdn-img.comic-seasons.com`；正文主机 `cdn-img.comic-seasons.com`，标准 scissors 包装由共享模块解出。最终测试实际提取 dist/wave-19/sources/comicseasons-ja/comicseasons-ja-0.1.0.koma 内 source.wasm，执行 info、五语、列表、适用筛选、搜索、详情、完整章节、设置、页表、图片请求与实际随包 WASM 还原。结构化记录在 dist/wave-19/verification.json；候选包保留供后续真机 QA，未提交、推送或操作设备。
