# MissKon

亚洲写真图集，收录角色扮演与摄影工作室作品，提供原站浏览量榜单和标签。内容分级为 NSFW，语言为 all。

## 移植对照

对照固定版本 [Keiyoushi MissKon 1.6.6](https://github.com/keiyoushi/extensions-source/tree/c09b174459c037c62292e4e7a929a0cd59430bc4/src/all/misskon) 和 [Koma 运行 ABI](https://github.com/honjow/Koma/blob/master/docs/source-runtime-abi.md)。

| 行为 | 上游与官网 | Koma |
| --- | --- | --- |
| 热门与更新 | `/top3/` 返回一页，首页和 `/page/N/` 根据 `.pagination .current + a` 续页。 | `get_listings` 和 `get_manga_list` 经 `browse` 读取原始顺序，核对下一页是否为 N+1，再生成游标。 |
| 分类与标签 | 榜单和标签均为独立归档。官网 `/categories/` 列出标签，`/tag/{slug}/page/N/` 提供标签续页。上游选择分类后优先打开分类归档。 | `get_filters` 输出五语分类，以及官网公布的标签和原始名称。角色扮演和日本有直达栏目。组合分类、标签和关键词与官网冲突时以本地化错误提示。 |
| 搜索 | `/page/N/?s=...` 是站内全文搜索，空结果可以正常结束。 | `search` 沿相同 WordPress 查询参数逐页读取。 |
| 详情与章节 | `article > .post-inner` 提供标题和标签，首图有日期路径时用于推断发布日期。原扩展每个相册子页形成一章。 | `get_manga` 及 `get_chapters` 复用共享图库协议。用 `args.mangaId` 对应的相册永久路径定位作品和单册章节，附原站地址及五语章节名。 |
| 阅读与图片 | 正文 `.entry p > img` 的懒加载地址。本站实测以数字子页呈现相册。 | `get_pages` 由 `args.chapterId` 定位相册，按编号取回官网公布的子页。归并时保序去重，兼容同相册的 `-pageN` 链接。一页失败则返回可重试错误。页表限制在 256 KiB。`get_image_request` 返回官方站点 Referer，公开封面声明相同请求头。 |
| 账号、设置与相关推荐 | 公开相册匿名可读，站上可能另有外链下载。上游有相关推荐。 | 公开相册沿共享 `photo_gallery.rs`。设置为空，相关推荐未接入。 |

最低 App 版本 0.1.0。公开正文页数按原站展示内容计算，标题标注的整套数量按原文保留。
