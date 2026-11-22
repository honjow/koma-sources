# 漫画柜 / Manhuagui 0.1.8

源 ID `com.manhuagui.koma`，语言 `zh`，NSFW 标记保持。生产主站固定 `https://www.manhuagui.com`，没有 `settings.baseUrl` 或自由镜像。当前图标是用户已采用的满幅桌面快捷方式身份衍生图，不新增底板、阴影或圆角；具体素材依据见 [ASSET_SOURCES.md](ASSET_SOURCES.md)。

支持普通公开热门/最新分页、文字搜索、详情、目录、固定 Base64-LZ/radix62 图片元数据解析与原图片请求。保既有 Chrome 140、Accept-Language、Referer 和 Origin。HTTP 失败、网络超时、损坏/截断、容量及未知打包形式明确失败，不重试或换 UA 制造成功。

不提供 home、credentials、deeplink、成人隐藏 VIEWSTATE、登录检查/投票副作用、任意 JavaScript 或图片字节解密。限速设置不改变这些访问边界，也不预先同意年龄门或操作 Cookie。

## 设置与宿主合同

仅请求顶层 `hostFeatures` 明确含 `source_rate_limit_v1` 时显示两项；旧 Host 返回空 schema，避免展示不生效设置。

- `mainSiteRatelimitPreference`：1..10，默认 `10`，精确主站 `www.manhuagui.com` 每 10000ms 的共享滑动窗口。
- `imgCDNRatelimitPreference`：1..10，默认 `4`，`i.hamreus.com` 与 `cf.hamreus.com` 每 1000ms 共用一份额度；不是各主机各有一份。当前正文产出 i.hamreus.com URL。

Host 的普通 HTTP、封面和 Reader 图片快取 miss 共用同一进程 owner；快取命中不等待。设置通过原请求快照发布，不伪装成 HTTP header，不在热路径读取整份偏好文件。排队时间不能侵占原 HTTP 超时预算，超时参数本身不改变；无关源/主机不受影响。

## 实现来源

仅源普通公开 HTML 适配器，面向 https://www.manhuagui.com，基于 Keiyoushi extensions-source commit 10f06eb833af5990bc5470c84c5e93edc159ad4e（Apache-2.0）。LZ-string 算法归因：Keiyoushi LZString（Apache-2.0），并对 pieroxy/lz-string 1.5.0（参考文件许可 WTFPL v2）做独立算法核对；有界源本地实现。

搜索筛选仅用于目录：沿参考提供排序、地区、剧情、受众、年份、字母与进度；未选择排序保留当前热门或最新原路由。排行仅输出网站真实标题和链接，不推断封面或分页；文字搜索不消费目录筛选。旧宿主不显示新筛选。

## 正文静态字典的进制

网页的固定六参数字典编码函数已实际返回42、51、52、61及62进制。按函数字面参数在2–62范围内解码词典索引，数字必须小于该进制；原有字典数量、文本容量、文件路径、查询参数和允许域名检查保持。不执行站点JavaScript，非法进制与未知语法继续失败。


## 2026-11-23 栏目完整位置

get_chapters改用typed SDK。沿用固定上游10f06eb及现有实现的栏目原序、
栏目内ul逆序、ul内链接原序显示。每个真实chapter-list栏目单独建立阅读组，
以该栏目全部ul的完整遍历逆位置声明readingOrder，不能每个ul重新从零起步。
实际官网104目录由第5卷到第1卷，开始阅读指向末项第1卷。

保留单行本/单话/番外等真实栏目边界；官网会给单话和番外复用chapter-list-1，
故组键仍带栏目次序，不能只按HTML id合组。原有章号/卷号只作元数据，
番外和栏目内无编号标题不再因此失去邻章；未知栏目也只在自己的栏目内续读。
只按稳定业务chapter路径合并重复项，不按同标题/同编号删除；保留章节URL、
作品ID和显示index、公开status0筛选、隐藏/R18限制、章节解码及全部访问错误。
目录与方向证据为Koma审计static-provider-gui-chapters.json和
static-provider-gui-work.json，候选检查另记static-*，设备验收仍由主代理协调。


## 2026-11-23 最终候选回归

候选版本0.1.14由最终.koma包内WASM的source_info与manifest一致读回。原5槽中4完整；comic/12912仍拒绝隐藏/成人目录，没有补造第5个成功。
目录逐项核ID、标题、原显示顺序、元数据和完整位置；实际执行App现有投影、
章序、邻章函数及反向显示投影，get_pages代表回归独立记录。原件同响应回放
与最终包代表性真实HTTP均保留于Koma审计static-expanded/及static-final-packages.json。
这些结果不冒充设备安装、图片解码或普通App UI验收。
