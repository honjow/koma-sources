# マガポケ · Magazine Pocket

讲谈社官方日文漫画平台，收录《蓝色监狱》等 Magazine 连载和平台原创。目录保留锁定话，阅读限官网匿名公开话。正文图片需要 Koma 1.5.5 的随包像素还原能力。

## 移植对照

对照 [Keiyoushi Magazine Pocket](https://github.com/keiyoushi/extensions-source/tree/c09b174459c037c62292e4e7a929a0cd59430bc4/src/ja/magazinepocket)、[v1.5.5 运行 ABI](https://github.com/honjow/Koma/blob/v1.5.5/docs/source-runtime-abi.md) 与官网匿名 JSON 响应。

| 行为 | 上游与官网 | Koma |
| --- | --- | --- |
| 热门与更新 | `/ranking/all` 默认排行 30，以 `offset` 取 26 条，`/title/list` 补作品资料。每周连载接口 `/web/title/weekly` 按最新公开更新时间排序。 | `get_listings` 提供排行和更新两个栏目。`get_manga_list` 保留排行顺序，每页输出 25 条；周更结果按公开更新时间排序后分批输出。 |
| 分类与搜索 | 上游在关键词非空时查询 `/web/search/title`，空词按 `/ranking/all` 或 `/search/title`。现网榜单包含女性向，入门榜单 ID 为 20。 | `get_filters` 输出五语排行和题材。站内搜索和题材接口按现网确认的 `offset` 续页，以源接口顺序稳定分页。非空关键词优先，全站查询。 |
| 详情和目录 | `/title/list` 读取作品和章节 ID，`/genre/list` 读取题材，`/episode/list` 通过匿名表单 POST 查询章节。 | `get_manga` 写作者、简介及题材。`get_chapters` 倒序分批，请求游标是上一批末话 ID，作品及章节身份长期稳定。付费话显示 `🔒 `、`locked` 并从自动续读排除。支持章节排除的宿主可隐藏锁定话，默认关闭。 |
| 权限与页图 | `/web/episode/viewer` 返回当前可读页和种子。公开样本返回 200，未发布与未购买分别返回 3104、3105。 | `get_pages` 使用查看器当前的匿名权限，拒绝锁定话并返回 `chapter_locked`。页表保存稳定资源路径，`get_image_request` 通过同一查看器刷新签名，限制图片所属作品和章节。普通封面直接返回官网请求描述。 |
| 图片处理 | 官方像素顺序从种子、作品和章节 ID 计算，按四乘四网格复位，保留不足整块的边缘。 | `get_image_request` 经 `source_required_image_decode_wasm_v1` 把 16 项排列传入包内像素模块。原站正文图还原后优先交给阅读及下载消费者。 |
| 设置与登录 | 参考扩展可以使用官网 WebView 租借或购买，公开路径使用匿名签名。 | 匿名接口固定使用空账户签名字段。书源只提供公开话，锁定内容由官网处理。 |

Wave 7 的首选 MANGA Plus 已对照 [Keiyoushi MANGA Plus](https://github.com/keiyoushi/extensions-source/tree/c09b174459c037c62292e4e7a929a0cd59430bc4/src/all/mangaplus)。带 `format=json` 的目录、详情及查看器返回 403。去掉该参数后，Protobuf 可匿名读取，公开话经 XOR 能解码。v1.5.5 的源 HTTP 返回文本，无法无损传输这些 Protobuf 响应，因此按本批兼容条件采用 Magazine Pocket。图标见 [ASSET_SOURCES.md](ASSET_SOURCES.md)。
