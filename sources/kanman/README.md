# 看漫画

## 0.1.2 簡介與多語言顯示

看漫畫的中國漫畫書庫有《斗羅大陸》《鬥破蒼穹》《穿越西元3000後》等長篇。部分章節須付費。

零參 `source_info` 固定使用以上基準簡介；`metadata.localization` 僅新增 `source.description` 的繁中、簡中與英文文案，供支援的宿主顯示。

参考：[Keiyoushi `Kanman.kt`、DTO、筛选及构建配置](https://github.com/keiyoushi/extensions-source/tree/8947c372cd87f546a0b80cb0e4b0a5a8add5ef88/src/zh/kanman)。上游 `zh` v1 来源 ID `3710922009941786183`；作品 `/数字/` 映射为裸 ID，章节保留 `/作品/章节.html`。许可见 `LICENSE` 和 `NOTICE`。

| 上游行为 | Koma 入口与实现 |
| --- | --- |
| 热门、最新和无词无分类的全部检索 | `get_manga_list` 和 `search` → `full_list`。用站点 HTML 最末页定位 API 尾部，再倒查最后非空页；每组四个 48 条的原站分页，跳过空组、组内同 ID 去重。`args.page.cursor` 带实际起点、尾页及排序，无需依赖跨 WASM 调用的内存。 |
| 搜索、分类、排序 | `get_filters` 保留人气/更新/收藏和全部上游分类；`search` 有关键词时忽略分类。过滤在原站分页前执行，`filtered_list` 汇总至第一空页，与上游一次交付且无下一页。 |
| 作品和章节 | `get_manga` 和 `get_chapters` 都用桌面详情页，避免手机页遗漏付费章节。保留原站标题、作者、简介、更新文本、封面、分类和状态。章表反转；只在最新章节附实际更新日，`Chapter.locked` 保留原站 `lock`。同域作品链接可解析为数字 ID；作品卡仍链接回官网。 |
| 正文 | `get_pages` 校验归属及原站锁状态，再用该章 Referer 和 `Cache-Control: no-cache` 获取原站签发的图片列表；顺序不变。签名过期时 `get_image_request` 通过同一公开路径重新取该页。锁定章节不申请图片。没有独立首页、账号流程或设置。 |

2026-11-19 使用本地编译的 WASM 验证：热门首页和游标下一页、最新首页、关键词搜索、详情和桌面目录。样本《凤逆天下》作品 17745 返回 609 章，其中 574 章锁定。免费章 `/17745/1004.html` 返回 31 页，首图下载解码为 1500×2100 JPEG；把本地请求中的过期值置为过去后，源重新向原站取得同一页的签名。锁定章返回 `chapter_locked`，未发正文请求。未验证 App 安装或离线。
