# Zerobyw

## 书源简介

Zero搬运网的漫画资源，包含中文翻译与机翻版本；部分章节需登录或VIP权限。

参考 `extensions-source/src/zh/zerobyw/.../Zerobyw.kt`：站点目录实为最近更新，文本搜索与筛选互斥，搜索页按站点页号请求并根据下一页链接继续。旧域和用户自定义地址沿原设置处理。

0.1.10 修复搜索读取 App `args.page.cursor`：第 N 页使用站点同一关键词的第 N 页；不改变筛选、域名、作品 ID 或目录行为。


## 0.1.12 候选：旧目录容量与重复身份

固定上游：keiyoushi/extensions-source 5d54f0d02e2a7dbdda08e823de444f7527cc883c，
`src/zh/zerobyw/src/eu/kanade/tachiyomi/extension/zh/zerobyw/Zerobyw.kt`
（blob f5c5d04aa73413d09fc32c1b70470bc6a7923977）。现上游使用 PC 路由；它不提供
旧 `/comic/` 作品与新 `pc-` 身份的映射。现站已接受的完整元数据/游客锁章方案保持。

| 行为 | 已有宿主调用与源消费 | 本轮核对 |
| --- | --- | --- |
| 旧站目录 | get_chapters → 非 pc- 分支 → 原锚点选择器 | 正式0.1.11严格离线反例证实1001条截为1000且Complete；改为读取实际描述符缓冲区内全部条目，超出容量整项失败 |
| 旧目录身份 | 原 chapter:<旧作品ID>:<原路径> | 沿原normalize_chapter_path生成身份，按完整编码路径去重，保留首个原位置；不按标题或编号删条目 |
| 无号/方向 | 原数组、chapterNumber=null | 无号公告等继续保留；旧网站真实方向没有本轮原件，保持unknown，不新增ManualOnly或猜readingOrder |
| 现站目录 | pc_chapters/emit_pc_chapters，PC分支提前返回 | 五部官网真实完整mangaDownloadChapters共97条与正式WASM逐ID/位置一致；本轮不修改该分支 |
| 现站权限 | 原可读锚点、login/vip锁行及get_pages | 保留原业务ID、锁章提示和权限拒绝；不把不带链接的锁章误删 |
| 其它功能 | 列表、搜索、页表、设置、域名和素材 | 保留已接受行为；不改App/SDK或旧作品映射 |

边界输入仅证明实现缺陷与修正，不是旧站真实数据：1001条、重复同ID且同题不同ID、
4097条超过描述符容量均以现有dev runner宿主严格offline replay执行，未请求伪造作品ID。
旧站真实热门入口 `http://www.zerobywgbo2.com/comic/category/order/hit/page/1`
本轮返回传输 Unexpected EOF；不能据此指认CF、登录或站点失效。没有获得真实旧作品ID，
旧目录方向、分页与旧新身份映射仍未验，不用现站五部成功覆盖旧分支未知。

证据位于 Koma `.hvigor/outputs/published-source-order-audit-20261123/youku-investigation/`
的 `zero-legacy/` 与 `noy-zero-followup/zero-provider-order-comparison.json`。
根Cargo.lock版本由主代理中央同步，保护其它WIP；本代理只改该源三文件，不提交推送，
无设备操作。候选构建与真实普通设备验收分别记录。
