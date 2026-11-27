# Pepper&Carrot

David Revoy 创作的开放版权奇幻漫画，跟随小女巫 Pepper 和她的猫 Carrot。官网收录多语言译本、迷你短篇和作者的艺术作品档案。主页为 <https://www.peppercarrot.com>。源 ID 为 `com.peppercarrot.all.koma`，最低 Koma 版本声明为 1.4.1。

## 上游与 Koma 调用对照

固定 Keiyoushi `extensions-source` 提交 `90b6e6a905ca186945b21ec0b0be4ca9135b8e69`，目录 `src/all/peppercarrot/`。参考 `PepperCarrot.kt`、`Dto.kt`、`Filters.kt`、`Preferences.kt` 与 Gradle 入口。Koma 经 BrowseViewModel、SourceSettingsStore、SourceMangaDetailRequest、SourceChapterListResolver、SourceChapterPageHydrator、ReaderPageSourceAdapter 消费下列响应。

| 上游行为 | Koma 消费与实现 |
|---|---|
| 上游热门混排已选语言的主线、迷你短篇和 11 个艺术目录；不支持关键词搜索 | `get_listings` 将三类作为独立栏目暴露，`get_manga_list` 按栏目读取，均不分页。艺术档案无需语言请求 |
| `last_updated.txt` 控制 Android 语言缓存，`episodes.json`、`langs.json` 决定语言名、译者、进度；首次预选系统语言和英语 | `get_settings` 提供本地语言多选，默认 `en,cn,tw,ja,fr,kr`。浏览和详情用原站两个 JSON 计算实时进度与有效语言；本地选项来自 `langs.json` 样本，选择「全部可用语言」也可纳入后续官网新增的键 |
| 上游可选从 Framagit 取得发行译名，失败回退为原标题加语言码 | 延用原标题与语言码的回退形式；详情显示原站语言名、进度与译者。作品以类型和语言键或画集键区分，切换设置不改旧作品的标识 |
| 官网漫画目录按指定语言列出 `figure.translated`，先编号再筛选；艺术目录倒序列出 11 个 `low-res/` 路径的 JPG | `get_chapters` 保留官网绝对章节链接、显示编号、可解析日期和顺序。画集按上游文件列表的逆序作为单图章，文件名无日期时留空 |
| 上游直接返回画集 JPG；漫画依序读取 `.webcomic-page img` 和 `.mft-cv-image`，主线补封面 | `get_pages` 解析成嵌套 URL 描述符；主线首图匹配 `P00.jpg` 才补对应标题图，迷你短篇保留原图次序 |
| 高清开关默认关闭，开启后把图像的 `/low-res/` 改成 `/hi-res/` | `get_image_request` 根据顶层 `settings.hi_res` 即时改图像地址；缓存和下载沿宿主的现有行为 |

艺术目录为 `artworks`、`wallpapers`、`sketchbook`、`misc`、`book-publishing`、`comissions`、`eshop`、`framasoft`、`press`、`references`、`wiki`。官网样本存在 `cn` 和 `kr`，没有 `tw` 或 `ko`；只有语言表和集数表都确认的译本会显示。即使将来默认语言调整，作品详情仍按原键访问。

Mihon source ID 为 `279282683217001983`，裸语言键可直接恢复主线作品。当前宿主的路径规则无法把带 `#` 的上游短篇和画集身份映射到这两类作品，页级进度也未声明自动迁移。联网需核对默认语言实际译集、迷你短篇图序、全部 11 个目录、首图和高清端点。真机需检查语言多选、切换设置后的实际图片及书源图标。

## 本地验证

从最终包解出 WASM 回放了实站语言表。另用构造集数、漫画页和艺术目录核对进度、默认及指选语言、译文章节编号、两套正文选择器、11 个目录路由和高清地址。
