# 读漫屋

参考 [Keiyoushi 读漫屋](https://github.com/keiyoushi/extensions-source/tree/f1d882a93ef34a33dc4f4467753ad5f99eab43f4/src/zh/dumanwu) 与同提交的 MMLook 模板。

- 浏览调用桌面站 `/rank/1`、`/rank/5`。空关键词筛选优先用榜单，其次分类，结果为单页。非空关键词提交 `POST /s` 的 `k` 字段，沿用上游 12 字符限制。
- 作品身份沿用不带斜杠的漫画 ID，章节身份沿用 `<漫画 ID>/<章节 ID>`。详情以桌面站为准。目录有更多按钮时用 `POST /morechapter` 追加，保留站点目录顺序。同一作品的展开章节与可见章节都归到原作品。
- 移动端阅读页解包站点自己的图片 URL 数据，并写出原有顺序的图片。请求时取最新签名地址，不存为章节身份。
- 当前站点 HTTPS 证书的主机名与桌面和移动域名均不匹配。此源使用两个域名的公开 HTTP 入口；图片保留原 URL。无需修改宿主 TLS 验证。
