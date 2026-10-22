# 那些需要守护的中文・官方漫画 — 0.1.0

原发布入口为 https://www.taptap.cn/app/167261/topic?group_label_id=476119&type=group_label 。只提供该“四格漫画”栏目的已核官方子集，不代表游戏当前仍运营，不从用户认证或站点首页200推断未列作品。

## 原6条栏目与源5篇

原 Nuxt feed 容器 `list` 长度6、`total=6`，没有 next_page/rel-next。源只纳入 moment.is_official=true、author.app.id167261/identifier com.hegshy.nxxyshd.tt、group.id200225/app_id167261、group_label.id476119/name四格漫画，以及唯一 primary forum binding 的 is_official=true 和相同 group/app/label 全部相符的条目：

| moment ID | 原标题 | 原发布用户 ID |
| --- | --- | --- |
| 185451606127414743 | 【四格漫画】具现灵小课堂 | 365506972 |
| 175331388852014799 | 【四格漫画】伽利略部长平凡的一天（二） | 365506972 |
| 154978458336758814 | 【四格漫画】伽利略部长平凡的一天 | 365506972 |
| 121982545804920934 | 【四格漫画】初代见证者 | 8995683 |
| 24456982647802746 | 具现灵小剧场1——所谓友谊 | 74839056 |

原第5条 `27013539889679837`「具现灵小剧场2——新朋友奥杜威石斧」缺 moment.is_official 和 author.app，primary binding也缺官方标记。其用户有 verified.reason「《那些需要守护的》官方博士噗」，这个真实字段保留在证据中，但不能补造缺失的帖子/app官方归属；源不纳入。原总6与源5不得混说。

前三篇用户365506972没有 verified 字段；不同原用户均保留，不硬编码首用户让另外两篇整页失败。作者用户属于发布者，不等于漫画创作者；authors/artists留空。author.app.can_view=false、group.has_official=false、label.is_disabled=true均是实际字段；不生成认证徽标、补标记或声称游戏仍服务。逐篇正式评级、真实usage、游戏当前服务状态均unknown。

## 阅读与静态合同

每篇是一个独立作品、一个完整阅读单元、一张完整原图，不裁四格成四页，不把另作编成同书章节。稳定源ID `com.protectors.zh.koma`；MID `protectors:zh:comic:<moment>`、CID `protectorsread:zh:comic:<moment>`、PageID `protectorspage:zh:comic:<moment>:0`。当前真实moment ID有17和18位，保留原值；不按数组位置/日期/文件名制造编号。chapterNumber/volumeNumber=null，readingGroup为同作品MID、continuationEligible=false。status/contentRating未知。列表supportsPaging=false，非首屏cursor显式拒绝；搜索只匹配这个已核5篇子集。底层原6/筛选5或原分页合同变化显式failclosed，不伪造分页终态或扩抓更多URL。

源内有界静态 Nuxt JSON索引解析，复用同一已审实现思路，但不共享修改任何SDK、runner或旧源。1MiB原JSON、4096索引、64层JSON嵌套、16个主data/binding槽；direct关键字段重复/非法索引/异常JSON均拒绝。只在Nuxt主data的feed容器取list，不扫描推荐树；每条 identification 须精确为 moment:<原momentID>并有该漫画原href。官方归属字段缺失/冲突不能降级成任意推荐条目。未获官方标记的原条只跳过，不借verified字段推断；已标官方条目若归属异常则拒绝，不返回不完整的5篇假成功。

详情唯一主容器moment/first_post绑定相同ID、原标题、原用户、原图URI和尺寸；first_post.is_topic=true/position1，topic/post不同id_str均真实存在，不强行混为一个ID。first_post.images、topic.images及first_post.contents.json中唯一type=image的info.img_url/info.image需完整同URI/尺寸；原HTML og:url、唯一正文wrapper/正文图片src及该实际用户href也同时匹配。分享小图、推荐/评论/广告、游戏推广图均不纳入。

正文采用真实 original_url，不通过删除网页后缀猜文件：原Nuxt original_url与同记录url（原URI + `/_tap_ugc.jpg`）及原DOMsrc分别校验。仅HTTPS img2-tc.tapimg.com/bbcode/images/下单个32位原小写十六进制文件名 `.jpg`/`.JPG`；大小写完全保留，格式字段须jpeg；禁query/额外目录/encoded slash/跨域/任意URI。图片尺寸从每条原字段读取并核对，不套用首图859×2232到其他386×987等真图片。

get_image_request接受MID/None作为原封面身份，正文PageID必须是同moment/index0且重新核该详情；独立mangaUrl若提供须完整匹配同作品原momentURL及group_id200225。每图完整URI须归属当前官方5篇，错work/page/URI均拒绝，不向请求者任意图片URL发HTTP。封面使用同一完整原漫画图，不用分享小图代替正文；源包不包含漫画原图。

## 原素材与Host证据

首篇详情 https://www.taptap.cn/moment/185451606127414743?group_id=200225 ，原post169370794。正文 original_url https://img2-tc.tapimg.com/bbcode/images/16271a25913856417bf35c5bf4f9f18e.JPG 。一次正常匿名GET200，真实完整中文四格JPEG859×2232/664126B/SHA256 `7ac454ac6fc8952b74e7458355d94b3d9d81bc0848365ef1807d1f9fc7d5d544`，已目检；页面旧size1038347与实际原体不同，实际响应字节为准。imageRequest headers={}；原图GET无Cookie/Referer也成功200，不据此称App Native图片/下载通过。

图标直接保留 author.app.icon.original_url 的游戏专属原PNG https://img-tc.tapimg.com/market/images/ac840dfe5139db2d66d4bf80f47d9a1d.png 。实际512×512/507621B/SHA256 `c0777b2847e8940fc600ee79e2a4d1e36b94fb436a36be76b2bad4d815d57828`，旧元数据270×270/618171差异明确；全部原字节保持，无转码/裁切/缩放/重绘/遮罩/底板。原角色、背景和构图都是原资产，不是平台泛用图标。

所有原响应/headers/完整6条父数据/首详情/原正文与图标/22文件绑定在 `/private/tmp/koma-protectors-zh-public-contract-20261015/`。实现与回放在 `/private/tmp/koma-protectors-zh-implementation-20261015/`。真实生产路径使用未改的当前App C++ HtmlHost/Lexbor，原SSR页面和#__NUXT_DATA__ Text作原字节核验；src/content/href全部presence-qualified，不以旧Rust字符串host的不存在属性空串模拟Native。所有描述符关闭；raw请求字节不经过CLI键归一化。

目录/本子集搜索/封面计划各1次目录GET；详情/章节/页面/正文计划各2次目录+同篇详情GET，首篇元数据369203B。实现中0新网络；HTTP只回放已存原body/status/headers，异常fixture另标synthetic，不冒实站响应。完整ABI/操作/失败边界与包绑定由Root独立审核。macOS Wasmtime22+C++Host不是197 WAMR或真实网络。

只有首篇原详情和完整JPEG实体已取得；另外4官方篇的详情/正文实体、被排除的原第5条正文、下载/cold及usage均unknown；首篇Native读取/解码及返回的后继验收见下。源列表5篇及5原封面描述符不冒充5篇正文已验。原神/SOC及所有旧源冻结、主源WIP、共同SDK/runner、App/Kit保持，未操作设备、IDE、提交、push或发布。


## 237 普通阅读验收（2026-10-23）

现装0.1.0从普通Browse进入首篇《具现灵小课堂》（moment185451606127414743），开始阅读后完整中文四格原图实际显示为1/1页；普通返回详情立即显示“继续阅读 · 【四格漫画】具现灵小课堂 · 第1/1页”。自然保存的reader记录绑定同作品/章节，pageIndex=0、totalPages=1、completed=true。此项只接受首篇Native读图与返回读态，不把列表阶段缓存命中或HTTP200绑定为正文首次网络成功，也不宣称下载、冷启动或其余四篇通过。
