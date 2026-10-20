# 幻塔官方中文漫画 0.1.1

来源为 `https://ht.wanmei.com/videostation/cartoon/index.html` 的独立漫画栏目。原站将漫画与视频、壁纸分开；本源只读取漫画，不涉及登录预约、验证码、视频、壁纸或同人动态。作者、正式评级和真实使用率没有足够依据，均保持 unknown；SourceInfo.author 的 Koma 指源开发者。

## 范围与操作

当前冻结原目录第一页9条、原 `index1.html` 第二页7条，共16条。源动态读取目录，未硬编码这16个作品或其正文图片。分页从原 `td.nlist` 的 current/pages/per/total 四个 strong 与真实下一页链接核对；9条/页，仅接受已核两页、最多18作品（缓冲上限128）。下一页必须由前一页明确声明，数量或总页不一致、缺少下一页链接、重复身份、超出容量即失败；不会猜分页终点或返回假空目录。只接受1/2页，page3及非法cursor在网络前拒绝；站点改变总页数也显式失败，不能跨过有问题的页继续。

声明 listings/manga_list/detail/chapters/pages/image_request；search/home/filters/settings/credentials 不支持，返回 not_implemented。本源没有额外搜索功能。目录游标使用当前App `args.page.cursor`（首屏null→"2"，data.hasNextPage=true；第二页hasNextPage=false与nextCursor=null）；保留旧数字page输入兼容，非法游标拒绝。

每个原详情路径是独立作品、一个完整阅读单元、一张完整原JPEG，不裁四格成四页。稳定ID绑定日期目录与文章ID，例如 `huanta:zh:comic:20211009:233573`、`huantaread:zh:comic:20211009:233573`、`huantapage:zh:comic:20211009:233573:0`。chapterNumber/volumeNumber=null，readingGroup=作品MID，continuationEligible=false。首篇原图确实写第八话，但不据这一处对16条推断连续章号或自动相邻。

## 详情和图片归属

详情仅取目录实际已声明路径，拒绝任意URL/查询串/跨域/越目录。目录标题、详情h2及head title必须一致；detail_time第二项必须是《幻塔》手游官网，第一项日期须对应真实路径。正文只接受唯一 `.box_main > .text_cont > a > img[src]`，其src须与唯一父a[href]逐字节一致，URI限定官方 `/resources/jpg/六位日期/数字.jpg`，保持原字节路径。其他弹窗/预约图被排除。

封面取目录img[src]，与正文URI分别绑定。get_image_request先重新核目录；封面可接受缺pageId或正确作品MID，正文只接受该作品的`:0`页ID；独立mangaUrl/mangaId若提供必须与该作品详情一致。错MID、正文ID作封面、封面作正文、其他作品图、其他页号、query、其他host/protocol/目录均拒绝，不把调用者URI作为HTTP取证入口。返回正文原URI和对应详情Referer；图像实体由App正常图片路径获取，包不包含漫画正文。

SDK普通metadata请求使用其原User-Agent与官网BASE Referer，不修改SDK。已保留公共GET最初使用Mac Firefox UA且无Referer；实现阶段无新网络。离线桥验证源产生的SDK请求字段，但不能据离线桥宣称该UA/Referer在197真实网络已通过。

## 原件、图标与验证

图标更新（0.1.1）：原漫画栏目链接官网 https://ht.wanmei.com/ ，官网声明 https://static.games.wanmei.com/public/commonData/gamesData/gameDownload/ht-gameDownload.js ，其iOS地址为 https://apps.apple.com/cn/app/id1522617722 。该官方应用页srcset明确声明 https://is1-ssl.mzstatic.com/image/thumb/PurpleSource211/v4/9c/e4/51/9ce4514d-aaa0-5f5f-2cbd-121018e2f786/Placeholder.mill/400x400Icon.ApSiOS01-75.webp 。采用完整400×400 WebP原件；原角色、完美标识及透明圆角外形保持，未裁切、补角、放大或新增遮罩/底板/阴影，App列表容器保持原样。该图是官方应用身份，不冒充漫画栏目独立徽标；设备呈现另验。

原件目录 `/private/tmp/koma-huanta-zh-public-contract-20261015` 保留6份一次匿名GET的完整body/headers/receipt与原字节父切片：两页目录、首详情、官网CSS、完整首JPEG、原logo。目录16条身份/标题/封面由生产WASM与原件逐项比较；已核首详情《试用体验》及1696×5173完整四格，其余15详情/正文实体未读取，不能称16篇正文已接受。

历史0.1.0原候选 `icon.png` 是官网 `inside210630.css` 的 `.top_logo` 明确声明 `https://ht.wanmei.com/images/web2010/logo.png` 原PNG，198×131 RGBA，32671字节，SHA256 `1aa99e6438f774c9ae2f84eb97b9f198a207d1a68b9faf9f6b8239de2a2e6073`。未裁切、缩放、转码、加透明底、底板、遮罩或阴影。原白字在浅底低对比，小图标槽显示尚未Native验收。

真实详情img重复alt属性；独立当前生产C++ HtmlHost/Lexbor探针在原HTML中返回第一项“试用体验”，正常关闭描述符；源不靠任一alt取得身份。attr查询一律presence-qualified，根与选择结果均关闭。使用未改App html_host.cpp/.h和1057份Lexbor指纹、独立只读原HTTP桥、生产WASM/Wasmtime做原目录→首详情→章节→页→图片计划回放，合成畸形HTML与非法请求单独标明，并完整核ABI和operation end的0存活描述符。离线Host不是197/WAMR，不是正文网络/下载/cold/全源验收。

成本：首Browse 1次目录GET；第二页先核首页关系再GET第二页（2次）。当前详情/章节/页/正文图片计划各2次目录+1次详情，封面计划2次目录；未加缓存或SDK状态假定。page3/非法cursor网络前拒绝（0次）。Native速度、并发、图片传输、其他15正文及将来目录变化未知；Root负责普通导入、9→16分页、首篇正文/下载/cold验收。无新网络、无设备/IDE/App/Kit修改、无提交/推送/发布。

使用现有缓冲/响应助手和已接受源的显式边界检查ABI exports，保留全部操作/alloc/init/info/free，避免旧宏未检查的指针范围；未修改共同SDK、ABI或runner。离线runner只在本源证据目录独立复制后修改原HTTP映射及该源请求断言。


## Source-tree recovery boundary

The original implementation and its dated stage notes above are preserved. This recovery uses the current canonical SDK and does not add artwork, standard-package registration or new HTTP/native acceptance. Original named runtime evidence remains separately bound to its original version and package.
