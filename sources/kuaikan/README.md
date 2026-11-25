# 快看漫画

## 0.1.11 簡介與多語言顯示

快看是《怦然心動》《甜美的咬痕》《谷圍南亭》等中國原創條漫的連載平台。作品既有校園愛情，也有懸疑與奇幻，部分章節須購買。

零參 `source_info` 固定使用以上基準簡介；`metadata.localization` 僅新增 `source.description` 的繁中、簡中與英文文案，供支援的宿主顯示。

Source ID `com.kuaikan.koma`，版本 `0.1.10`。公开列表、搜索、作品与章节数据沿用站点公共接口；阅读仅接受站点明确标为未锁定且免费的章节，并核对章节及作品身份。图片请求重新取得对应章节的公开图片地址，不沿用过期签名。

站点明确标记付费或会员的章节，以及已锁定且 `locked_code` 为数字 `10103` 的章节返回 `paid_chapter`；该码对应[官网阅读器脚本](https://static3w.kuaikanmanhua.com/_nuxt/static-kkfront-pc/chunk/pages/webs/comic-next/_comicId.c1bab0c7caefbba54695.js)中当前章节的购买流程。明确锁定但无足够付费证据返回 `chapter_locked`；缺失或异常的访问字段保持 `permission_denied`，不推断付费原因。明确可公开访问但无图片返回 `chapter_unavailable`。HTTP、身份校验及未知内容继续按原错误边界处理。

Nuxt 参数通过容量固定且有界探测的名字索引查找，保留同名拒绝、未知引用、全部参数值、深度及容量校验。这里只减少同一次解析的重复扫描；每页仍重新核对作品、章节、免费状态及原始图片归属，并刷新站点签名地址，不能改为跨调用复用过期地址。

接口与上游参考见 [NOTICE](NOTICE)；Nuxt 数据仅用有界数据解析，不执行网页 JavaScript。

作品评论展示原站作品页公开的「热门评论」，复用既有作品响应。

0.1.10 沿已固定的 Keiyoushi 快看参考及站点 topic 目录，Koma `get_chapters` 从早到晚保留原目录位置。已核站点《怦然心动》第13话（上）的下一话链接指向（下）。章节号仍只采信已有标题规则；序章、分话与番外按原站位置接续，正文权限判断沿用 `get_pages` 的原合约。
