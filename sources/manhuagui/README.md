# 漫画柜 / Manhuagui 0.1.7

源 ID `com.manhuagui.koma`，语言 `zh`，NSFW 标记保持。生产主站固定 `https://www.manhuagui.com`，没有 `settings.baseUrl` 或自由镜像。当前图标是用户已采用的满幅桌面快捷方式身份衍生图，不新增底板、阴影或圆角；具体素材依据见 [ASSET_SOURCES.md](ASSET_SOURCES.md)。

支持普通公开热门/最新分页、文字搜索、详情、目录、固定 Base64-LZ/radix62 图片元数据解析与原图片请求。保既有 Chrome 140、Accept-Language、Referer 和 Origin。HTTP 失败、网络超时、损坏/截断、容量及未知打包形式明确失败，不重试或换 UA 制造成功。

不提供 home、filters、credentials、deeplink、成人隐藏 VIEWSTATE、登录检查/投票副作用、任意 JavaScript 或图片字节解密。限速设置不改变这些访问边界，也不预先同意年龄门或操作 Cookie。

## 设置与宿主合同

仅请求顶层 `hostFeatures` 明确含 `source_rate_limit_v1` 时显示两项；旧 Host 返回空 schema，避免展示不生效设置。

- `mainSiteRatelimitPreference`：1..10，默认 `10`，精确主站 `www.manhuagui.com` 每 10000ms 的共享滑动窗口。
- `imgCDNRatelimitPreference`：1..10，默认 `4`，`i.hamreus.com` 与 `cf.hamreus.com` 每 1000ms 共用一份额度；不是各主机各有一份。当前正文产出 i.hamreus.com URL。

Host 的普通 HTTP、封面和 Reader 图片快取 miss 共用同一进程 owner；快取命中不等待。设置通过原请求快照发布，不伪装成 HTTP header，不在热路径读取整份偏好文件。排队时间不能侵占原 HTTP 超时预算，超时参数本身不改变；无关源/主机不受影响。

## 实现来源

Source-only ordinary public HTML adapter for https://www.manhuagui.com, based on Keiyoushi extensions-source commit 10f06eb833af5990bc5470c84c5e93edc159ad4e (Apache-2.0). LZ-string algorithm attribution: Keiyoushi LZString (Apache-2.0), with an independent algorithm check against pieroxy/lz-string 1.5.0 (reference file licensed WTFPL v2); bounded source-local implementation.
