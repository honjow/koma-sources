# 禁漫天堂 JM

书源 ID `com.jmcomic.koma` 对应 `https://18comic.vip`，与 HANMAN18 分开。参考用户指定 Keiyoushi 索引中的 source `6286738698187452081`，扩展版本 1.6.64。行为参考和图标均固定在 [Keiyoushi `de743aed` 的 `zh/jinmantiantang`](https://github.com/keiyoushi/extensions-source/tree/de743aed846da53e00c45ab26ee0e77f7bb4355b/src/zh/jinmantiantang)。最低 Koma 版本为 1.2.1。

| 行为 | 上游与 Koma 实现 |
| --- | --- |
| 匿名列表与检索 | `Jinmantiantang.getPopularManga`、`getLatestUpdates`、`getSearchManga` 对应 `get_manga_list`、`search`。保留热门、最新、ID 和地址直达、文字、分类及标签搜索、原分页。动态分类和标签由 `get_filters` 读取实际 API。 |
| 作品与章节 | `fetchMangaUpdate` 对应 `get_manga`、`get_chapters`。保留原标题实体处理、详情字段、分集标题和顺序。章 ID 包含作品与章节原始身份。正文先验证所属目录，页 ID 使用原始位置，重复位置照常保留。 |
| 正文图片 | `ScrambledImageInterceptor` 的原条带算法映射为必需 `jm_stripes_v1`。包声明、`get_pages` 和 `get_image_request` 一起交给 Koma。正式 App 在可选图片处理前按行数还原，真实 GIF 依实际字节透传。封面有独立作品校验，无解扰提示。 |
| 镜像与限流 | 匿名 API 从上游服务目录取主机。Koma 的每次 WASM 实例复用已有私有会话保存非敏感主机，网络失败按代失效。专属限流按 API origin 共用 3 次/2 秒，普通封面和图片请求不计入 API 窗口。 |
| 账号范围 | 原参考 `/login` 以 multipart 登录，随后同时使用 AVS Cookie 与 JWT。收藏夹及自动或立即签到仍在原站登录使用。Koma 1.2.0 提供私有凭据存储和单一精确 API origin 的授权。JM 的动态 API origin 与响应 AVS Cookie 需要另外支持。现包提供匿名浏览和阅读。 |

请求需要 `source_rate_limit_jm_v1`。需解扰的图片需要 `source_required_image_decode_jm_stripes_v1`。缺少能力时返回明确提示。匿名 API 的传输失败、原站拒绝和已移除作品或章节分别处理。

本地以 73 部真实作品、4842 个目录位置和 9142 个页图位置验证匿名路径。生产解扰算法与 8 张真实条带图的独立还原结果逐像素一致。原图标和许可见 [ASSET_SOURCES.md](ASSET_SOURCES.md)、[LICENSE](LICENSE)、[NOTICE](NOTICE) 和 [THIRD_PARTY_LICENSES.txt](THIRD_PARTY_LICENSES.txt)。
