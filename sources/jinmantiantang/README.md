# 禁漫天堂 JM

## 0.1.11 作品只读评论

`metadata.comments` 仅声明作品作用域；`comments::get` 经现有 `Context::api` 读取 `forum?mode=all&aid=<作品>&page=<页>`，沿原 API 主机发现、请求限速、令牌和解密链路。参考 JMComic-Crawler-Python `f60b752c771dbc50212a7c9b7929951573c76001` 的 `jm_client_impl.py:1073–1088`、`jm_entity.py:667–760` 与 `jm_toolkit.py:896–930`，不以 Mihon 没有评论入口推断原站没有评论。

读取 `list`、主评论 `total`、稳定 CID、昵称／用户名、HTML正文和 addtime；按主评论每页10条判断续页，不按展开回复后的行数或去重数判断。原返回内嵌的 replys 逐层保留，宿主v1为平面列表，回复正文标明回复对象，保留剧透标记；不增加发言、点赞或账号能力。不支持章节评论，不把作品评论冒充每章独立讨论。

原作品／章／页身份、图像还原与镜像、普通设置及最低App1.4.3不变。未支持评论的旧宿主忽略可选声明。最终 WASM 已完成加密评论、内嵌回复、去重分页与错误分类的受控响应回放。237 从普通源内搜索和详情打开《秘密教学》，最终包首屏12行及两次续页10／11行正常显示，关闭后立即重开仍从首屏读取。最初旧缓存API主机在响应头到达前失败，普通重试沿已有主机失效及发现流程恢复，不将失败视为空评论。


## 0.1.10 簡介與多語言顯示

禁漫天堂（JM）的漫畫與分集圖集，按站內分類及標籤整理。此入口提供匿名公開內容，帳號收藏與簽到請在原站使用。

零參 `source_info` 固定使用以上基準簡介；`metadata.localization` 僅新增 `source.description` 的繁中、簡中與英文文案，供支援的宿主顯示。

书源 ID `com.jmcomic.koma` 对应 `https://18comic.vip`，与 HANMAN18 分开。参考用户指定 Keiyoushi 索引中的 source `6286738698187452081`，扩展版本 1.6.64。行为参考和图标均固定在 [Keiyoushi `de743aed` 的 `zh/jinmantiantang`](https://github.com/keiyoushi/extensions-source/tree/de743aed846da53e00c45ab26ee0e77f7bb4355b/src/zh/jinmantiantang)。最低 Koma 版本为 1.4.3。

| 行为 | 上游与 Koma 实现 |
| --- | --- |
| 匿名列表与检索 | `Jinmantiantang.getPopularManga`、`getLatestUpdates`、`getSearchManga` 对应 `get_manga_list`、`search`。保留热门、最新、ID 和地址直达、文字、分类及标签搜索、原分页。动态分类和标签由 `get_filters` 读取 `/categories`。宿主将所选值交给 `get_manga_list` 或 `search`，源内 `list` 按 API 原样编码分类 slug（包括大小写），默认 `c=0` 对应最新 A 漫。禁漫汉化组按 [官网搜索入口](https://18comic.ink/search/photos?main_tag=0&search_query=%E7%A6%81%E6%BC%AB%E6%B1%89%E5%8C%96%E7%BB%84) 使用站内搜索。保留有合法作品 ID 的空标题条目，由宿主显示原有编号备用标题。 |
| 作品与章节 | `fetchMangaUpdate` 对应 `get_manga`、`get_chapters`。保留原标题实体处理、详情字段、分集标题和顺序。章 ID 包含作品与章节原始身份。正文先验证所属目录，页 ID 使用原始位置，重复位置照常保留。 |
| 正文图片 | `ScrambledImageInterceptor` 的原条带算法映射为必需 `jm_stripes_v1`。包声明、`get_pages` 和 `get_image_request` 一起交给 Koma。正式 App 在可选图片处理前按行数还原，真实 GIF 依实际字节透传。封面有独立作品校验，无解扰提示。 |
| 镜像与限流 | 匿名 API 从上游服务目录取主机。Koma 的每次 WASM 实例复用已有私有会话保存非敏感主机，网络失败按代失效。专属限流按 API origin 共用 3 次/2 秒，普通封面和图片请求不计入 API 窗口。 |
| 账号范围 | 原参考 `/login` 以 multipart 登录，随后同时使用 AVS Cookie 与 JWT。收藏夹及自动或立即签到仍在原站登录使用。Koma 1.2.0 提供私有凭据存储和单一精确 API origin 的授权。JM 的动态 API origin 与响应 AVS Cookie 需要另外支持。现包提供匿名浏览和阅读。 |

请求需要 `source_rate_limit_jm_v1`。需解扰的图片需要 `source_required_image_decode_jm_stripes_v1`。缺少能力时返回明确提示。匿名 API 的传输失败、原站拒绝和已移除作品或章节分别处理。

本地以 73 部真实作品、4842 个目录位置和 9142 个页图位置验证匿名路径。生产解扰算法与 8 张真实条带图的独立还原结果逐像素一致。原图标和许可见 [ASSET_SOURCES.md](ASSET_SOURCES.md)、[LICENSE](LICENSE)、[NOTICE](NOTICE) 和 [THIRD_PARTY_LICENSES.txt](THIRD_PARTY_LICENSES.txt)。
