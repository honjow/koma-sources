# NoyAcg 0.1.4

独立源身份为 `com.noyacg.koma`，主页为 `https://noymanga.com`。提供热门/最新导航及现有搜索、详情、目录和页面接口；接口声明不代表匿名内容当前可用。作品、章节、页面 ID、顺序和分页保持原合同，图标保持官网完整角色图。

## 设置与请求合同

`POPULAR_MANGAS` 为热门漫画显示内容，默认 `day`（日閱讀榜），另有 `week`（周閱讀榜）和 `month`（月閱讀榜），沿固定 Keiyoushi `949dea252eb8e867b7b40834a26e13088764ff55` 的偏好键、选项及默认值。仅 `listingId=popular` 消费此项；显式 `day/week/month` 导航优先，最新和搜索不消费它。无配置及非法值均保原日榜请求。

排行榜仍使用 `POST https://noymanga.com/api/readLeaderboard`，表单字段为 `type/page`。原 POST 头、`allow-adult=both`、超时与传输失败重试保持。旧 GET 详情/章节头及 b1 端点保持，未迁移到参考的 v4，也未把成人筛选或账户凭据开放为普通设置。参考的统一成人头与当前 GET 消费者仍有差异，此处不声称全设置等价。

## 可用性边界

当前匿名排行请求实际返回 `auth_required`；源仅在精确 `status=login` 标记出现时报告该错误。未登录、未发 Cookie 或凭据、未绕过认证；未取得作品详情、目录或正文，旧 b1 与当前站点兼容性仍未知。排行榜偏好请求选择的正确性与内容可用性分别核验。
