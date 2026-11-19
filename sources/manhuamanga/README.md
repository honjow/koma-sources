# 漫画manga

独立源 `com.manhuamanga.koma`，主页 mh03.com。对应 Mihon 来源 ID `325122966187249774`。作品、章节分别沿用官网 `/comic_*.html` 与 `/chapter_*.html`。源保留上游 MIXED 分级，Koma 当前把该值显示为未知。参考 [ManhuaManga、Dto、Filters](https://github.com/keiyoushi/extensions-source/tree/8947c372cd87f546a0b80cb0e4b0a5a8add5ef88/src/zh/manhuamanga) 和 [MCCMSWeb／MCCMSConfig](https://github.com/keiyoushi/extensions-source/tree/8947c372cd87f546a0b80cb0e4b0a5a8add5ef88/lib-multisrc/mccms)，代码许可 Apache-2.0。

| 上游行为 | Koma 入口及实现 |
| --- | --- |
| 热门、更新为 `/category/order/hits/page/{n}`、`/category/order/addtime/page/{n}` | `get_manga_list` → `browse`。解析 `.comic-list .comic-item`，末页的 `a.next` 和 `a.on` 同地址时停止。 |
| MCCMS 桌面词搜 `/search/{query}/{n}`。空词筛选是分类查询，包含连载状态、热门/更新排序及官网题材 | `search` → `browse`，路径分段编码。基础筛选立即返回，打开筛选再按宿主能力从 `/category/` 加载 `.filter-options` 中的题材。默认热门排序，词搜保留上游忽略分类选择的行为。 |
| 搜索出现 `#code-div` 时返回指向 `/search` 的验证码条目 | `search`、`get_manga`、`get_chapters` 保留上游的提示、WebView 链接和空目录。 |
| 详情解析标题、作者、题材、进度、评分和简介。章节链接按 JSON-LD 面包屑解析到作品页 | `get_manga` → `details`。普通 `search` 支持同站作品及章节 URL。当前书源 ABI 无独立站点链接分派导出。 |
| 作品页同时内含正序章节，MCCMSWeb 反转显示 | `get_chapters` 从 `#chapter-list .chapter-item` 读取路径，按 `ChapterResponse::ordered(NewestFirst)` 保留目录展示和续读次序。 |
| 阅读页取 `img[data-src]`，正文默认沿用上游桌面 UA，封面需要官网 Referer | `get_pages` 保留原页序和 HTTPS 地址，`get_image_request` 按封面或正文给出对应请求头。现场两张图片的 `manhua.5um.net` 响应证实直连返回 HTTP 200 HTML，只有增加真实章节 Referer 才返回图片，故针对该 CDN 由既有页 ID 回读章节地址，其他 CDN 仍沿用上游请求。 |
| 针对首页域名每秒两次请求，拒绝返回到 `/err/comic` 的缺失作品 | `requestLimits` 声明同等限速。宿主未提供可依赖的最终重定向 URL，`document` 只认精确已知缺失通知，其余按原 HTTP 或结构错误返回。 |

上游无独立首页、源设置或源内登录。已取得 [官网分类](https://www.mh03.com/category/)、[作品](https://www.mh03.com/comic_yirenzhixia.html)、[首章](https://www.mh03.com/chapter_13871_4992.html) 及词搜和最近更新原页。该作品的 DOM 为正序，原章页的 `img[data-src]` 顺序和读取合同一致。

当前 WASM 经宿主 runner 读取了实际热门、最近更新、组合筛选与搜索。用《一人之下》验证了从章节链接回到作品详情，以及完整的 800 项倒序目录和首章 18 项页表。题材选项来自官网分类页。

237 在《一人之下》验过搜索、800 章目录和首图（1/18）。
