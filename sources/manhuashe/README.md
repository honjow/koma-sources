# 漫画社公开适配器

合同参考：Keiyoushi extensions-source `src/zh/manhuashe/.../Manhuashe.kt`，Apache-2.0，修订 `10f06eb833af5990bc5470c84c5e93edc159ad4e`。https://github.com/keiyoushi/extensions-source/blob/10f06eb833af5990bc5470c84c5e93edc159ad4e/LICENSE

稳定源 ID 和旧规范名称保持不变。漫画 `manga:<原始路径>`、章节 `chapter:<原始漫画路径>:<原始章节路径>` 和页 `page:<零基索引>` 身份保持兼容。主站的显式目录反转保留。直到 0.1.5 chapterNumber 为 null；索引只表示那个原始顺序。

上游 MIXED 保守映射为当前 nsfw=true/contentRating=nsfw，不是声明每个标题都是成人内容。单作品分级和实际用量未知。无认证、cookie 引导、镜像回退、任意 JS、图像字节解密或自动重试。缺失/含糊/超大的解析输入失败。只有普通公开 HTML 合同；构建和宿主门禁不建立物理验收。

当前生产入口是 `bash build.sh --source manhuashe`，使用当前规范 SDK 和正式 CLI。历史 package_candidate.py 只保留在原所有者处；不导入规范树。其过期 registry 更正由本源 nsfw=true 行表示。最低 App 版本保持 0.1.0。原官网首页声明的 JPEG 现在未处理保留为 icon.jpg；见 ASSET_SOURCES.md。这只闭合包资源前置条件；小槽视觉适用性保持未验收。无发布。

0.1.2 只添加主站固定公开图像请求 Referer/Origin 描述符。它保留输入 HTTP(S) URL，不设 User-Agent/cookie。描述符成功不证明图像下载/渲染，也不修复观察到的 197 失败。


历史 197 0.1.2 只验收了具名 一人之下/767 坦率 17 页路由、P1/P3、单下载和不清数据冷/书架继续。那个历史原生成功不认证新标准 WASM 或当前网站。原五个保留响应结果和图像描述符幸存；其三个原始 HTML 固件体目前不在了，所以当前 SDK 的 HTTP/DOM 等价未知。本集成不改解析器、ID 或公开图像头。App 显示裁剪保持独立不变。


官方 150×50 JPEG 横版 logo 是源所有者的美术作品，不是本适配器许可的代码。没有裁剪、缩放、改色、去底板、阴影/遮罩、重绘或格式转换。URL 以 .png 结尾且服务器标记 image/png，但实际原始字节解码为 JPEG/RGB 无 alpha；包名 icon.jpg 跟随实际格式。App 显示裁剪独立且不变。

## 237 现装 0.1.2 具名在线观察（2026-10-21）

237 VDE-AL00 展开竖屏1320×2120，普通 Browse 行实际显示 0.1.2；不能把197的资源交付或本地产物文件名当作237版本。普通热门返回20本，《一人之下》详情及目录显示800/800章。原767《坦率》行已有下载，本次没有重开它作为在线证据，而打开未下载的766《反杀》。首帧仍为图片解码中，随后只读观察亲看真实中文正文1/17；普通右侧下一页后亲看不同正文2/17。

自然 Back 后原读档记录绑定 comicId=`com.manhuashe.koma:manga:/comic_13871.html`、chapterId=`chapter:/comic_13871.html:/chapter_13871_1342899.html`、pageId=`page:1`、pageIndex=1、totalPages=17、completed=false。未直接修改持久状态、未收藏或新增下载。原Browse末端完整画面已亲看，50项源registry与进入前完整相同，原10000ms/AWAKE读回。只接受这个现装版本的具名列表、目录、首两页与自然返回；没有核验安装包字节身份，不外推当前源码WASM、其他章节、下载、冷启动或全源。

原件沿 Koma `.hvigor/outputs/user-flow-audit/device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership/` 的 `manhuashe012-popular-terminal237-001`、`manhuashe012-yirenzhixia-chapters237-001`、`manhuashe012-yirenzhixia766-{first-ready237-001,page2-237-001,natural-return237-001,session-restored237-001}`。最初定位回执误名为manhuashe013-browse237-001，实际界面明确012，保留原名且不据此升级版本。一个日志命令被参数校验阻止，后一次-x/-z组合返回多命令错误，均不算有效日志或产品故障。

Root 已亲审本条首两页原图及精确记录；结论仅限上述具名路径，原未闭边界保持。


## 0.1.4 详情来源网页（2026-10-21）

`get_manga` 的实际 `detail` 输出新增 `kind=source` 链接，复用同一 `fetch(mp)` 请求的 `BASE + mp`，其中mp由原网页href身份经manga_path验证。例如已有237正文成功的《一人之下》对应 https://www.311s.com/comic_13871.html；不从App复合ID猜造网址，不填站首页。原244d63b业务及5965937的013图标交付保持，版本013→014只供普通更新；请求、身份、章节、正文、图标和App不动。

按现源Firefox121/Referer/Origin合同对该准确作品页单次Host读取200/175915B，未请求已403首页或处理挑战。原件在Koma `.hvigor/outputs/user-flow-audit/manhuashe014-detail-fixture.html` 与 `manhuashe014-detail-fixture-receipt.json`。013基线WASM逐字节等原正式013包。013/014实际WASM在同一真实HTML上离线详情均成功，除新增links外全部manga字段深比较相同、唯一HTTP请求合同深比较相同；输出为同目录manhuashe013-link-baseline-offline.json和manhuashe014-link-offline.json。离线适配器仅用禁网惰性Chromium DOM查询，不代表生产Lexbor/scraper或Native等价。单源offline/locked构建及真实WASM info014通过，正式dist/sources/manhuashe/manhuashe-0.1.4.koma三成员读回，原013 manifest核心字段保持，补入旧013包漏打的本源既有localizedNames（zh-Hans漫画社、zh-Hant漫畫社），icon.jpg原bytes保持；未运行会清空dist的全量脚本。普通实机结果见下，仅限具名路径，不由Host或离线输出替代。


237 VDE-AL00（展开竖屏1320×2120），现装Root指定16289728B/22:29:30 Debug，普通本地选择器012→014更新后重导已补既有中文名的29732B包；服务54包、Browse54已启用可见。当前行实际仍显示“漫画社 (Manhuashe)”v0.1.4，不把manifest含中文名冒运行名称已切换，未改App/源名。《一人之下》两网页菜单可见；仅执行WebView，等待加载完成后显示真实封面、标题、评分9.9与人气171740。自然Back回详情800/800章，原“766反杀・第2/17页”保持，再返回源列表/Browse。未开正文/外部浏览器、未刷新/登录或遇站点拒绝，不扩全源接受；原图标小槽问题仍OPEN。最终10000ms/AWAKE、本人唯一传包不存在均读回，租约释放。

原图在Koma `.hvigor/outputs/user-flow-audit/device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership/manhuashe014-webview237-004/` 的 `{browse014,both-web-actions,webview-workpage,detail-returned,browse-restored}.png`；最终恢复回执为 `manhuashe014-webview-postflight237-004/run-metadata.json`。`detail-before.png`仍在加载，不作终态；服务54包原图在 `manhuashe014-webview237-002/services54.png`，54启用原布局在 `manhuashe014-webview-postflight237-003/layout.json`。

同版本metadata局部刷新边界：上述重导后旧Browse行仍为raw名称；随后237不清数据冷启（PID8283→stop后空→37211）后，Browse实际恢复“漫画社”v0.1.4，54个已启用保持。仅检查列表，未进入源页或请求站点；这不改变前述网页通过结论，也不证明同进程同版本行会立即刷新。原图/回执沿同一category-membership分区的`manhuashe014-name-cold237-001/{enabled54.png,run-metadata.json}`、`manhuashe014-name-cold237-003/browse-name.png`和`manhuashe014-name-cold-postflight237-003/run-metadata.json`。最终10000ms/AWAKE并释放租约；自动化的不存在scroll调用与重复浏览文本断言失败原件保留，不计产品故障。

## 0.1.5 用户批准的图标衍生

用户在 2026-10-22 从 source-icons-four-proposals.png 明确批准了这个资源处理。使用批准的 744×612 RGBA 黄色风格化漫/巧克力棕气泡衍生，平滑闭合原右侧连接并省略画社/英文。identity-reconstruction.svg 保留平滑几何输入。完整官方 150×50 JPEG 保留为 official-header-logo-original.jpg；它不是官方独立原件或精确字形复原。这个显式选择取代早前的不提取/连接字标保留。不添加额外背景板、阴影或外部遮罩。App 30vp/R7 和源顺序保持不变。只有源资源交付版本对齐到 0.1.5；提供方请求、身份、章节/页、本地化元数据和其他业务行为不变。离线/锁定单源构建和真实 WASM info0.1.5 通过；52592B 标准包恰好含 manifest.json/source.wasm/icon.png，完整注册的评级/minAppVersion/本地化元数据和批准的候选字节。图标校验和 diff 检查通过无哈希；dist/index 不变。Root 完成普通 237 picker 0.1.4→0.1.5 更新并审查了完整浅色主题管理/浏览图像：批准的身份完整清晰。证据：Koma `.hvigor/outputs/user-flow-audit/device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership/three-icons-package237-001/{manager-manhuashe015,browse-manhuashe015}.png`。这只验收这两个浅色主题图标消费者；不建立深色主题渲染或源内容。最终 registry 读回：59→60 条目，只有 H-Comic 新增；原相对顺序和所有无关条目保持不变。Manhuashe/MH160 变化限于其预期的包引用和版本。原系统主题/浏览组和 10000ms/AWAKE/窗口208 恢复，只删除三个上传的安装器文件并确认不存在，租约释放。这只验收图标交付，不代表源阅读/下载。

## 0.1.6 显式章节编号

只有显式以 `第N话` 或 `N <章节标题>` 开头的标题现在提供 chapterNumber。公告和其他无编号标题保持 null；不从索引推断日期、季、卷或编号。原章节 ID、标题、目录反转/顺序、索引、分组、请求和阅读继续边界保持不变。MH160 不变。

实际 Rust 辅助通过了保留的成功 `manhuashe014-detail-fixture.html` 全部 800 个标题：786 个显式编号条目和 14 个公告/活动条目保持无编号，另有九个负例覆盖季、卷、番外、日期和溢出。单源离线锁定 WASM 构建通过；这些检查不替代普通 App 更新、排序或首读验收，那些保持待定。


## 0.1.7 无分隔符章节标签

237 普通更新到 0.1.6 暴露了真实反例：大奉打更人有 672 个条目，含 `669阴物`、`624“鸡”不可失` 和纯数字 `583`/`571`。这些显式前导标签现在也提供 chapterNumber；原空格形式含 `721 TA` 仍接受。季/卷/日期前缀、公告、请假条、活动和番外保持无编号。ID、目录反转/顺序、索引、分组和所有请求不变；MH160 不变。

保留的成功 800 标题目录加同一精确作品的单次成功生产 Host 结果（`manhuashe016-dafeng-chapters-host.json`，HTTP200）通过了全部 1472 个实际标题用例：分别 786/800 和 671/672 编号；13 个负例通过。初始更严格的候选拒绝了真实空格 `721 TA`，验收前已更正。普通 0.1.6 首读对 comic_13836 无先前历史，选择了 `第1话 001牢狱之灾`（chapter_13836_17244），显示完整 P1/24 封面并在自然返回时保存 P1/24；0.1.6 数字排序在无编号新标签上失败，不标记通过。0.1.7 普通更新/排序验证保持待定。


237 普通 picker 0.1.6→0.1.7 更新成功，Browse 和 registry 都读回 0.1.7。同一 672 条目作品中，降序现在从 669/668/667 开始；升序显示无编号的请假条后接第 1/2/3 章。这只验收更正后的编号元数据和编号排序：当前 App 对未知章节编号的处理仍是观察到的问题，不是完整排序通过。证据是 Koma 现有 237 `category-membership/manhuashe017-number237-{desc,asc-oldconsumer}-001/{screen.png,layout.json}`；更早的首读和自然返回证据是 `manhuashe016-number237-{firstread,firstreturn}-001`。

`manhuashe017-number237-final-restored-001` 保留普通 Browse 锚点、63 条源记录原相对顺序，只有本源版本/包引用变化。所有先前进度记录不变；只添加了 comic_13836 的自然 P1/24 进度。临时顺序覆盖经普通控件重置；AWAKE/10000ms 和两个上传安装器及两个临时文件的不存在在读回后释放租约。本源交付未修改源图像/身份、App UI 和 MH160。

## 0.1.9 显式镜像设置

固定参考 949dea252eb8e867b7b40834a26e13088764ff55 的镜像选择首项为 `https://www.311s.com`，另可选 `https://www.m206.com`。`preferredMirror` 默认原地址，未设置或未知值沿旧请求；仅用户显式选择备用地址时，列表、搜索、详情、章节、正文及图片请求 Referer/Origin 使用该地址。不自动试探或切换镜像；旧路径形式的漫画和章节 ID 保持。两站是否对每个作品均同一身份不能推定。

备用站实际返回的搜索第一页无 `/1` 链接；仅将精确匹配查询路径且无页号的链接识别为第一页，其他分页仍校验精确查询和页序。现有用户身份、图标和默认域不变。仅真实站点/WASM与具名旧作品测试，普通 Native 设置持久和设备阅读仍需独立验证。

## 0.1.11 章节容量与缺失作者

章节列表单独接受最多 8192 个选中节点；所有其他选择器保持既有容量，超出仍是错误。站点可以在当前作品元数据中显式显示空作者；这表示为空 authors 数组，而缺失或重复的作者标签仍是错误。超出既有响应缓冲的目录使用绑定作品的游标，每页最多 1000 个条目，需要时进一步缩减以适配响应预算；较短目录保持原单次响应。章节身份、全局索引和反转顺序不变。
