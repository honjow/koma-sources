# MYCOMIC - 我的漫畫 0.1.5

独立公开HTML适配 `com.mycomic.koma`，API0.2，主页 `https://mycomic.com`。中文内容、源级nsfw，单作品分级和用量未知；官网繁中与简中页面标题分别为“MYCOMIC - 我的漫畫”和“MYCOMIC - 我的漫画”，显示名保留完整品牌；英文保留 MYCOMIC。现红色 C 是此前保留身份的重绘图，不冒称官网原图，图标字节不变。popular、latest、search、筛选、排行、详情、目录、页描述与图像请求已实现；home/credentials不开放。不登录、不改Cookie、UA、网络或权限。

## 首选语言

`pref_key_lang`（設置首選語言）提供繁體中文与简体中文，默认 `cn` 保留Koma原简体路径；固定Keiyoushi `949dea252eb8e867b7b40834a26e13088764ff55` 默认是空前缀繁体，这里按保当前默认适配。ABI选项不允许空值，`original`表示参考空前缀，也兼容已有空值及zh-hant；cn/未配置保持 `/cn/comics`，显式original使用 `/comics`。

列表、搜索、分页与详情/目录的共同manga_doc均消费所选前缀。Koma作品ID一直为无语言前缀的slug，因此详情与目录从当前设置派生路径，保持原作品身份；不主动重写已入库标题。参考Dto章地址始终 `/chapters/<id>`，正文请求保此无前缀路线，不按语言设置擅加cn；图像URL和请求头保持原输出。原正整数字符串章ID、页ID `page:<chapterID>:<index>`、目录顺序及读态不变；App沿已采用设置摘要合同按需重新取页，不清图片缓存。

## 解析与可用性

目录完整解析实际owner中的strict JSON chapters数组，保title/原序/notes；仅从原章名明确的「第N话／第N話」提取话数，卷号、裸数字、无编号番外和未知日期不猜。分页next仅接受当前选定语言下实际下一页号。原狭窄ASCII slug规则、HTTPS图片、非2xx与容量/截断守卫保持；未知HTML或缺图明确失败，不把部分解析冒完整目录。

具名公开请求曾返回 HTTP 403，当前站点内容链可用性未确认；不对受拒请求反复重试。红 C 是保留原身份的重绘图，不是官网原图。

## 实现来源

独立Rust适配依据Apache-2.0 Keiyoushi `10f06eb833af5990bc5470c84c5e93edc159ad4e` 的 `src/zh/mycomic`；本次语言项核固定 `949dea252eb8e867b7b40834a26e13088764ff55` 的MyComic.kt/Dto.kt。许可见NOTICE/LICENSE，不复制Kotlin实现或图像。

## 参考筛选与宿主调用链

固定参考 949dea252eb8e867b7b40834a26e13088764ff55：独立热门按 -views、最新按 -update；查询默认最新上架（无 sort），提供最近更新、最高人气、日/週/月/历史排行，另含地区、类型、受众、年份、进度。四种排行走 /rank、忽略搜索词且不翻页；普通查询及分类走 /comics，按站点 next 续页。参考无 home 或账户功能。

Koma 仅在宿主声明 source_filter_search_without_query_v1 时提供 search 作用域筛选并声明 useSearchForFilters；筛选以 search 执行，允许空查询。热门和最新 get_manga_list 不消费筛选，保持原 sort；显式最新上架使用内部值 new 并省略 sort，普通查询未选择排序亦省略 sort。旧宿主保留原无筛选和非空搜索行为。排行仅消费排序和其他筛选，保留原站无封面数据，不伪造封面；排行续页请求在网络前拒绝。cn 与 original 前缀沿当前设置，不改变已有默认和 ID。筛选项按源参考原值发送，未知值拒绝。

本轮未发布候选仅通过合成 HTML 的实际 WASM 请求、解析及旧响应/403对照；原站先前返回403，真实筛选、排行 HTML 与 Native 普通路径均未验证，不能据此声称站点恢复。
