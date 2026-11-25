# vomic

## 站点简介

vomic 漫的推荐区常见中文短篇、日常漫画和同人创作，也能找到以 FGO 等游戏为题材的安科与 AA 连载。

以 [Keiyoushi vomic](https://github.com/keiyoushi/extensions-source/tree/8947c372cd87f546a0b80cb0e4b0a5a8add5ef88/src/zh/vomic) 为固定上游，按 Apache-2.0 移植。Mihon ID 为 5428862423208834108。此源的域是 vomicmh.com，旧 Koma vomic 源指向被移除的 www.vomic.com，其作品和章标识不继承。

| 上游行为 | Koma 路径 |
| --- | --- |
| Dart UA、平台字段、秒级 t 与有序查询参数签名 | request_api 仅对精确 API 域签名。图片请求保留公开客户端头，无账号字段。 |
| 热门查询 getCartoonList 20/页，最近更新 getUpdateBook 15/页 | get_manga_list 进入原 API，热门固定 hot_value。实际返回足量才继续，cursor 为下一页号。 |
| 非空关键字 10/页、空词筛选 20/页 | search 保留关键词优先及空词互斥。get_filters 提供排序、状态、来源、地区、类型；打开筛选再向 getCatList 获取分类。 |
| getCartoonInfoV1 与 getDetailRecommendList | get_manga 保留详情、作者、简介、分类、标签、阅读数、喜欢数及可选同源推荐。相关请求失败保留作品详情。本站作品链接进 search 时转作品。 |
| chapter_list[].values | get_chapters 使用 typed chapters，保存各组原展示顺序、原章 ID、组名与发布时间。无法唯一确定续读位置的章节保留手动打开。 |
| getChapterInfo 的 img_list | get_pages 先带当前 Bearer（游客为空）访问，保持有 URL 时的原页索引。站点确认需要认证时，沿同源私有会话刷新并仅重试一次。 |
| 邮箱、密码、Bearer | 宿主显式登录表单提交同源登录，账号、密码与 token 进本源私有会话，支持退出。无凭据时遇站点授权要求才询问登录。普通 403、限流与网络错误各自保留。 |

上游没有独立首页、评论或镜像设置。签名依赖 source_private_session_v1 时钟，登录依赖 source_authentication_v1、私有会话 peek 和 source_private_session_replace_v1。真实账号流程需要用户拥有网站账号。

## 主机验证

已通过 WASM 与临时时钟／私有会话宿主请求实时热门 20 条、更新 15 条和 37 项分类。作品 `12607` 共 255 章、两组目录，原站按含小数的 `sort` 值严格降序。源保留该展示顺序，只在同名唯一且顺序明确的组内发续读位置。该样本游客读章 HTTP 401 正确映射为登录；真实账号、凭据续期与登录后的实际页图未验。依赖 `peek` 的读章和登录在宿主声明 `source_native_cookie_session_v1` 后启用。暂不声明 Mihon 页进度等价。
