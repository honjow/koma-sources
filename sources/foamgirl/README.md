# FoamGirl

亚洲写真与角色扮演图集，收录日本写真和中国摄影工作室作品。来源语言为 `all`，内容分级为 NSFW。

## 移植对照

对照 [Keiyoushi FoamGirl](https://github.com/keiyoushi/extensions-source/tree/c09b174459c037c62292e4e7a929a0cd59430bc4/src/all/foamgirl) 与 [Koma 运行 ABI](https://github.com/honjow/Koma/blob/master/docs/source-runtime-abi.md)。章节、身份、文案、HTTP 和图片去重复用 `photo_gallery.rs`，与 MissKon 使用同一单册图库合同。

| 行为 | 上游与官网 | Koma |
| --- | --- | --- |
| 目录 | 上游称首页归档为热门，实际来自官网时间顺序的 `/page/N`，读取 `.update_area .i_list`。 | `get_listings` 标为最新相册，另外提供官网中国、韩国、日本、泰国、角色扮演栏目。`get_manga_list` 根据下一个原站链接发布页码游标。 |
| 搜索与筛选 | 上游使用 `/page/N?post_type=post&s=...`；站上分类、`/tag/{slug}` 和关键词各有独立入口。 | `search` 沿相同搜索路径。`get_filters` 提供分类与标签别名输入；浏览分类或标签时先清空关键词。 |
| 详情和章节 | `/{id}.html` 提供标题、日期、标签，并以官方永久地址标识单章。 | `get_manga`、`get_chapters` 共享图库作品和整册章节模型，ID 取相册数字编号。 |
| 正文和子页 | `.imageclick-imgbox` 给出正文原图。查看器用 `{id}_{N}.html` 续页，末页的同一控件可能跳向下一册。 | `get_pages` 仅合并同一相册的连续子页，保留首现顺序并去重。后续页失败时返回错误。页表响应缓冲上限为 192 KiB。`get_image_request` 传官网 Referer。 |
| 设置和登录 | 官网公开相册可匿名读取，另有下载和账户服务。 | 公开图片沿原站权限读取。设置为空。 |

最低 App 版本为 0.1.0。图标见 [ASSET_SOURCES.md](ASSET_SOURCES.md)。
