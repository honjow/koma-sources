# HentaiFox（中文）

独立源身份 `com.hentaifox.zh.koma`，API 0.2。保留现有匿名中文画廊元数据、详情、单章及页面接口；能力声明不代表当前上游或所有正文已可用。成人或混合内容源分级、作品实际值/未知合同保持，语言不改变分级。

## 上游栏目与筛选（0.1.7 候选）

以 Keiyoushi `8947c372cd87f546a0b80cb0e4b0a5a8add5ef88` 的 HentaiFox 中文实例和 GalleryAdults 为参考。热门与最近更新分别使用中文目录的 popular 与普通路由。保留作品、章节及 Mihon 映射身份；榜单、数字 ID 和随机可指向同站非中文目录的作品，详情仍检查站点的作品 ID。

普通带词搜索可使用热门或最新排序。选择一个题材且关键词为空时读取原站标签目录；复数题材或关键词拼成上游的搜索表达式。题材在宿主打开筛选时读取原站五页热门标签。分页键对关键词、排序或标签路由作绑定，沿用普通热门及原关键词的兼容游标。

四个榜单使用上游 `includes/sidebar.php` 的表单值。榜单由原站公开页提供 CSRF；收到 403 时，使用宿主已声明的匿名请求能力从精确同源页面取一次性 PHP 会话和同页 CSRF，将两者用于同一个榜单请求。现有网页授权仍由宿主管理；站内收藏走 `includes/user_favs.php`，遇站点明确要求登录时显示同源官方登录入口。未经帐号实测的收藏结果保留待验状态。

## 短标题设置

`pref_short_title`（Display Short Titles）默认关闭，保原长标题及请求合同。显式开启时，列表和详情按固定GalleryAdults表达式 `(\[[^]]*]|[({][^)}]*[)}])` 删除标签并沿Kotlin.trim修剪；详情附 `**Full title**:` 保留完整标题。状态说明沿参考为Showing Long Titles / Showing short Titles。章节名、ID、顺序、页表和图像URL不随此显示设置改变，不主动改写库内旧标题。

四源复用SDK唯一有界 `gallery_titles::display`，仅匹配上述固定分隔规则，不支持任意正则，不引入正则引擎或动态分配。非标签Unicode原字节保留；允许混合闭符是参考实际语义，不改成嵌套配对解析。未配置、false和错误作用域不清理。

## 提供方与权限边界

实际中文语言码6筛选、原查询规范化和下一页；单一未编号画廊章，完整静态页面描述，不加loader回退。 原完整分页、归属、UTF8/JSON、ABI指针和容量守卫保持；失败不发布截断或伪空结果。账号由宿主 WebView 管理，源不读取或保存凭据；公开栏目仅按上述最小范围使用匿名 PHP 会话。图像、封面与完整正文按具体证据分别记录；受控DOM/HTTP ABI回放和构建不冒当前站点或正文成功。

公开热门请求曾在 TLS 握手时中断，未返回 HTTP 状态。宿主明确返回的 `network_error` 或 `timeout` 由源原样传递给 App。HTTP 请求、状态与网络失败继续沿用原路径。

## 封面属性

热门、搜索和详情共用 `imgattr`。官网卡片用 `data-src` 保存封面，`src` 可以是占位图。沿用 [上游 GalleryAdults 的属性优先级](https://github.com/keiyoushi/extensions-source/blob/bcb5d5547b6adb5f57ea3cb72d4524f3cdc0751d/lib-multisrc/galleryadults/src/eu/kanade/tachiyomi/multisrc/galleryadults/GalleryAdultsUtils.kt#L21-L27)。Koma 的 HTML ABI 在属性缺失时返回 `-1`，源跳过该候选并继续读取。缺少全部候选时仍返回错误。201 上，目录与详情有时分别指向 `i` 和 `i2`，但同一作品缩略图的完整路径一致。解析图片请求时，先校验站点的 HTTPS 图片域名及路径，再用详情确认作品与完整缩略图路径，允许站点已许可的 CDN 副本。详情原图和正文仍校验作品归属。正文页必须与详情声明的页序和完整图片路径一致，只允许站点已有 HTTPS CDN 副本。

## 图标与许可

当前图标为512像素H/狐标记的已采用高清还原，参考官网16像素标识及扩展同身份，不声称官方发布高清资源。 本设置修复不改变资源或App外观。

原公开合同独立适配自Apache-2.0 Keiyoushi `1319949ca9bc456908196e3ba2d104e94182c5d6`；短标题偏好及共同GalleryAdults消费者依据固定 `949dea252eb8e867b7b40834a26e13088764ff55`。当前归属及许可见 `NOTICE` 和 `LICENSE`，不宣称站点从属关系或内容所有权。

## 2026-11-10 显示名精简

按用户要求去掉单源语言后缀，当前显示名为 `HentaiFox`；源 ID、内容、图标和阅读行为不变。历史名称仅保留在旧版本记录中。
