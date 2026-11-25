# TOPTOON頂通

## 站点简介

TOPTOON 的官方繁中條漫平台，主要發行韓國漫畫，有愛情、職場與成人連載。部分章節可試閱，後續依站方規則解鎖。

上游固定为 [Keiyoushi TOPTOON](https://github.com/keiyoushi/extensions-source/tree/8947c372cd87f546a0b80cb0e4b0a5a8add5ef88/src/zh/toptoon)。按 Apache-2.0 移植，许可原文见 LICENSE-APACHE-2.0。源身份独立，Mihon ID 为 1680248666203661128。

| 上游 | Koma 操作与实现 |
| --- | --- |
| 成人切换的公开站点 Cookie | 官网 HTML 请求由 document 发送 adultStatus=1，第三方 JSON/CDN 不携带会话。 |
| 排名页的第一个 jsonFileUrl → adult | get_manga_list/popular 读榜单文档和 CDN JSON，保留返回顺序。 |
| 搜索页的 comicTotal JSON | search 按作品名、作者不区分大小写过滤；get_manga_list/latest 按 lastUpdated.pubDate 降序。两栏及搜索只有一页。 |
| /comic/epList、/comic/epView 站内作品链接 | search 接受站内作品或章节链接，解析到作品；get_manga 使用原始作品路径，保留作者、简介、标签及连载状态。 |
| episodeBox 与 coin/gift/waitFree | get_chapters 逆序输出上游目录，保留原章节路径、站点时间和锁定状态，使用 typed chapters。 |
| epContent 下 data-src | get_pages 保留站点页序，走公开图片；无图时只依据目录的明确锁定标记返回锁定。 |

固定上游未实现独立首页、筛选、账号设置、付费开锁、赠券领取或等免领取。普通官网入口继续可用于站点自己的账号流程。

## 主机验证

WASM 运行器已请求到实时热门榜单 100 条、最近更新 1,102 条，能按关键词找到字母 ID 的作品。作品 `81163` 的详情和 86 条章节保留官网原路径，最新章到首章顺序已核对。游客打开全年龄样本 `80011/21` 时，原站跳回作品目录，未提供 `epContent`，源返回 `chapter_unavailable`。付费与游客真实页图仍需验证，因此暂不声明 Mihon 页进度等价。
