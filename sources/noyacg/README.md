# NoyAcg 0.1.13（同源 v4 适配待验候选）

独立源身份为 `com.noyacg.koma`，主页为 `https://noymanga.com`。提供热门/最新导航及现有搜索、详情、目录和页面接口；接口声明不代表匿名内容当前可用。已有作品、普通章节和页面的 ID 编码保留，图标保持官网完整角色图；未通过真实会话目录验收。

作品网页入口使用官网 `/manga/{id}` 路由。目录与详情优先官网同源 v4；仅明确缺失资源时保留旧 b1 条件路径，不按标题或顺位重映射已有身份。

## 验证与登录

源声明 `https://noymanga.com` 网页会话。用户可在源管理的“验证／登录”打开官网并自行登录；登录后的同站列表、搜索、详情、目录和正文请求可复用该站会话，实际是否可用取决于网站及账号状态。会话仅用于声明的同一 HTTPS origin，不与其他站点共享；包内不包含 Cookie 或账号密码，也不新增原生凭据设置。

## 设置与请求合同

搜索词先还原 JSON 字符串转义，再按表单编码发送。

`POPULAR_MANGAS` 为热门漫画显示内容，默认 `day`（日閱讀榜），另有 `week`（周閱讀榜）和 `month`（月閱讀榜），沿固定 Keiyoushi `949dea252eb8e867b7b40834a26e13088764ff55` 的偏好键、选项及默认值。仅 `listingId=popular` 消费此项；显式 `day/week/month` 导航优先，最新和搜索不消费它。无配置及非法值均保原日榜请求。

排行榜仍使用 `POST https://noymanga.com/api/readLeaderboard`，表单字段为 `type/page`。`ADULT` 按参考提供全年龄、成人或全部三项，默认 `both`；搜索、榜单、详情、目录和页表的 API 请求及图片请求均携带所选 `allow-adult`。列表和搜索的原端点、作品/章节 ID 编码及本地数据保持；GET 和图片请求传递默认 `both` 或所选头。账户凭据不属于普通设置。受控 WASM 已核请求字段，站点筛选结果及旧章节续读尚未核验。

## 目录字段与当前接口合同（待真实会话验证）

固定 Keiyoushi `949dea252eb8e867b7b40834a26e13088764ff55` 与当前已固定参考 `5d54f0d02e2a7dbdda08e823de444f7527cc883c` 的 NoyAcg.kt、完整 Dto.kt 和官网当前公开前端为依据。两版普通章均把 `ChapterDto.count` 写入页数备注并生成图片，不赋普通 `chapter_number`。本源最初移植 `097c3437` 误把它同时当章节号，并在1000条后报告完整；字段修正已由 `223126243a477005d2dd46908748ef49271c8fea` 保护，本增量继续保留。

| 环节 | 上游与官网合同 | 本候选行为与边界 |
| --- | --- | --- |
| 详情、完整目录 | 上游 `GET /api/v4/book/{bid}?comment=false`；官网 client 是同源 `/api`、`withCredentials:true` | 优先 `https://noymanga.com/api/v4/book/{bid}?comment=false`；只用已有同源会话声明。GET 使用 `NoyAcg/3.0`、官网 Referer 与所选 `allow-adult`。仅 HTTP404 试旧 `/api/b1/bookInfo/{bid}`；登录、403/429、网络、服务端、DTO与身份冲突不换接口。 |
| DTO与身份 | `book.info`；`Bid/id`、`Mode/mode`、`Bookname/name`、`Description/description`、`Author/author`、`Time/time`、`Status/status`，单章数为 `Len` | 详情校验返回作品ID等于请求数字身份，冲突报错。保留 `manga:{bid}`、普通 `chapter:{bid}:{cid}` 编码；不按同名或目录顺位猜旧新业务ID。推荐列表不在本次新增功能范围。 |
| 分类 | `categories:[{id,name}]` 与 `data:{categoryId:[{id,name,count,created_at,...}]}`；上游按分类顺序展开后逐组反转供Mihon消费 | 按真实 categories 原序完整读取每组 data，组内保留提供方数组显示顺序；不为套Mihon的消费者反转既有Koma显示。分类ID生成 `noy-category:{id}`，原分类名写入scanlator；同名不同分类ID保留。缺组、额外未声明组、重复分类ID报错，不能漏组后complete。 |
| typed位置 | 固定/当前ChapterDto没有sort；扩展对完整分类数组map后逐组reversed；Mihon正向源序读取反向sourceOrder，分类内恢复原data顺序。官网另用sort稳定排序显示 | 使用 typed `GroupedChapters` 和 `koma_source_chapter_export!`。普通chapterNumber留空，count只pageCount。完整、校验成功的分类数组中，每个不同业务ID都取原数组零基索引为typed位置，独立于显示反转；sort缺失或并列、无编号、重编号或同标题均不导致ManualOnly。相同ID保首次，原位置允许空档、不挤压其他条目。sort不参与读取位置，不把官网额外显示排序引入已采用扩展的源序合同。 |
| 去重 | 章节业务身份是作品bid与章节id | 相同业务ID的重复出现只保分类原序中的首次，即使展示元数据有差异也不把它当新身份；依据是上游与官网均只按bid/cid寻址。不同ID同标题保留；不按标题、count或编号去重。 |
| 单章回退 | 上游无分类时使用book.Len；官网reader以cid=0表示单章 | 仅当前v4无分类章节且有明确正Len/len时新增 `chapter:{bid}:0`，chapterNumber=0、pageCount=Len；有分类但空组不伪装为单章。特殊0是官网路由标识，不是猜旧普通章ID；旧b1空目录不套用v4回退创建新身份。 |
| 正文与图片 | 上游用目录count生成普通章 `img/{bid}/{cid}/{1..count}.webp`，单章 `img/{bid}/{1..Len}.webp`；官网阅读API另为 `/api/v4/book/detail/{bid}/{cid}`，返回chapter.this/next/last和count | 采用上游目录页数合同：get_pages重取完整book并精确匹配cid，不信任请求传来的count。仅v4工作不存在、或完整当前目录缺该旧cid时保留旧 `/api/b1/chapter/{cid}` 路径，不按标题找新ID。原 `page:{cid}:{index}` 与零基index保留；图片Referer使用官网。 |
| 时间和状态 | 普通章created_at为秒时间，MangaDto以mode/status判断完结 | 普通章数值created_at按App已有秒时间消费合同输出；保留旧字符串日期。v4详情按mode=0或status=1显示completed，其他明确组合ongoing；旧条件响应仍保unknown。 |
| 完整性 | v4详情一次给完整分类目录，无1000章限制；作品列表另有20条分页 | 取消静默截断，typed builder只有全部遍历成功才Complete；1MiB响应/2MiB正文或内部安全容量不足明确失败。8192条内部安全上界高于1MiB能编码的记录数，超出不能返回部分成功。 |

宿主调用链为 `SourceChapterListResolver` → `SourceChapterRow` → `SourceChapterChronology`/`ReaderChapterSequence`。完整v4分类目录不会因缺sort走ManualOnly，所有不同业务章ID均有明确ordinal。保留的旧b1条件端点尚缺同一目录方向的实证，不能仅凭JSON形态套用v4合同，仍不能据typed ManualOnly声称当前App拒绝所有未知方向兜底；此旧路径宿主边界单列待验，未改App或SDK。普通章标题保原文，不添加页数后缀改变已存标题。

## 验证与迁移边界

真实匿名v4详情返回HTTP200、顶层精确 `status:login`；候选必须报告auth_required，不因描述中出现login误判，也不借登录失败换到旧端点。此前旧b1详情匿名返回404；尚不能断言登录后也已退役，所以保留窄条件路径。Cookie可复用只说明传递通道，不证明目录/正文可用。

本候选版本仍0.1.13，保持待验，不覆盖公开包。源码/DTO/typed纯本地合同检查不是已登录HTTP回放，不计作真实目录或正文通过。分类数组方向已按固定/当前扩展与Mihon消费者源码合同落实；真实站点是否仍遵循该合同、同一作品旧新业务ID、单章及普通章代表pages仍待授权原宿主现有会话核验；没有映射时限制迁移，不自动替换历史ID，不宣称已恢复内容或完成章序验收。

目录方向的唯一依据：Keiyoushi固定/当前NoyAcg.kt的分类内map.reversed，加Mihon `f8e30a86086020aaee1e89a9d015373b1a4ff026` 的SyncChaptersWithSource（按URL去重并记录返回索引）、ReaderViewModel（sortDescending=false）及ChapterSort的SOURCE反向索引比较。官网相同sort的稳定先后不丢条目；它不把sort变成ChapterDto必需字段。真实会话未验与源码顺序合同明确是两项，不因前者把后者降为逐条禁续。
