# LeerCapitulo

西班牙语日漫、韩漫和国漫目录。最低 Koma 版本为 1.5.5。采用已发布的列表、目录和普通 HTTPS 页图 ABI。

对照固定版 [Keiyoushi LeerCapitulo](https://github.com/keiyoushi/extensions-source/tree/c09b174459c037c62292e4e7a929a0cd59430bc4/src/es/leercapitulo) 和 [Koma 运行 ABI](https://github.com/honjow/Koma/blob/master/docs/source-runtime-abi.md)。

| 行为 | 上游及原站 | Koma 路径 |
| --- | --- | --- |
| 热门和最新 | 首页 `.lc-slide` 与 `.lc-release`，都只有一批。 | `get_manga_list` → `browse`。沿用原站顺序，去除同作品的重复卡片。 |
| 目录、搜索和筛选 | `/manga/` 的 `q`、`genre`、`theme`、`type`、`status`、`sort` 和 `page`。默认 `az`，按 `rel=next` 结束。 | `get_manga_list` 的目录栏目和 `search` 共用同一解析；`get_filters` 发布稳定选项值。题材与主题标签五语展现，原站不同值有同义译名时带原分类名区分。 |
| 详情和章节 | `/manga/{id}/{slug}/` 的详情和同文档全章目录。 | `detail` 与 typed `run_get_chapters`，保留真实章节身份与站点目录顺序，支持带小数章号。 |
| 正文 | `/leer/{id}/{slug}/{chapter}/` 中 `#lcPages img[data-src]` 按 DOM 顺序提供页图。 | `get_pages` 仅使用现网匿名 HTML 提供的 HTTPS 图片地址；站点拒绝时使用原 HTTP 错误处理。 |
| 网络、设置和登录 | 上游对官网限速每三秒一请求，使用公开 GET。 | 包声明同一主域限速。无独立源首页、源设置或账户流程。 |

五语覆盖包展示资料、栏目、筛选字段、类型/状态/排序文案、详情别名标签与源错误。Koma 1.5.5 使用旧宿主固定文案回退，提供本地化语言环境的宿主按界面语言选取资源。
