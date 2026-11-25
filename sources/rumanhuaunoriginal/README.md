# 如漫画（同名站）

## 站点简介

rumanhua.org 的同名书站。它的榜单既有《神印王座》《绍宋》等国漫，也能找到《海贼王》和《神之塔》。

源 `org.rumanhua.koma` 对接 [m.rumanhua.org](https://m.rumanhua.org)，沿用 Mihon 来源编号 `6075779468319464999`。站点身份见 [Keiyoushi #20916](https://github.com/keiyoushi/extensions-source/pull/20916)。显示名按用户确认使用「同名站」。

| 上游能力 | Koma 实现 |
| --- | --- |
| 排行和最新 | `custom/top`、`custom/update` 用原站目录卡片。 |
| 列表筛选与分页 | 保留 `ListFilter` 的八种列表。完整目录跟随下一页链接，其他列表单页。 |
| 关键词与 URL | `/index.php/search?key=` 单页。站内 `/news/` 与 `/show/` 链接均可查找作品。 |
| 详情和章节 | 使用原详情页元数据与章节链接，按新到旧返回；整书日期只赋给最新章。存在重复章节名称时保留原章节 ID，等待手动选章。 |
| 正文 | 原公开阅读页的 `params` 是封装后的页图地址，沿用同模板元数据解包，地址及页序与原站一致。 |

没有账号和镜像设置。与 `rumanhua2.com` 使用独立身份和目录。
