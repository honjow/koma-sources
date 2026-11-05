# 快看漫画

Source ID `com.kuaikan.koma`，版本 `0.1.7`。公开列表、搜索、作品与章节数据沿用站点公共接口；阅读仅接受站点明确标为未锁定且免费的章节，并核对章节及作品身份。图片请求重新取得对应章节的公开图片地址，不沿用过期签名。

站点明确标记付费或会员的章节，以及已锁定且 `locked_code` 为数字 `10103` 的章节返回 `paid_chapter`；该码对应[官网阅读器脚本](https://static3w.kuaikanmanhua.com/_nuxt/static-kkfront-pc/chunk/pages/webs/comic-next/_comicId.c1bab0c7caefbba54695.js)中当前章节的购买流程。明确锁定但无足够付费证据返回 `chapter_locked`；缺失或异常的访问字段保持 `permission_denied`，不推断付费原因。明确可公开访问但无图片返回 `chapter_unavailable`。HTTP、身份校验及未知内容继续按原错误边界处理。

Nuxt 参数通过容量固定且有界探测的名字索引查找，保留同名拒绝、未知引用、全部参数值、深度及容量校验。这里只减少同一次解析的重复扫描；每页仍重新核对作品、章节、免费状态及原始图片归属，并刷新站点签名地址，不能改为跨调用复用过期地址。

接口与上游参考见 [NOTICE](NOTICE)；Nuxt 数据仅用有界数据解析，不执行网页 JavaScript。
