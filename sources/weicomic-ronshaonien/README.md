# 創漫繁中・隆少年 0.1.1 候选

只处理同一既有隆少年完整阅读单元，按当前官网XML頁元素顺序完整消费；保原MID/CID及ordinal PageID，不扩作品、不补印刷页、不猜图片。移除固定30/NAMES/RAW/ORIGINAL逐样本锁与256B页URI槽。先完整校验XML才返回目录或正文图片计划；借用当次body，不在借用期间再fetch，不保存跨operation缓存。正文请求URI保存受原1MiB请求ABI限制，get_pages输出受原payload预算限制，容量不足整次失败。

页URI从当次圖片字段组合原官网/shocomic/前缀，只接受comic/ronshaonien/所属路径，禁止可逃逸的dot/编码分隔/控制字符；不设ASCII或扩展名白名单。官方viewer只读名稱/圖片，寬度/高度/超連結不参与图片计划，不再冻结为空；仍保既有五属性形状、标题身份及XML实体/DTD/双引号等源内语法边界，不冒通用XML支持。

章节/页表/正文image_request共用全量解析，后段错误不能回成功单页；image_request按当前count+ordinal逐项核原URI。封面/目录/search/HTTP头及请求次数不变；错ordinal或过时同作品URI需当次2GET才可判定，外站/越界路径早拒绝。原真实30完整输出与40条原请求/HTTP/DOM全等；31/删页/重排/长URI/无用字段值/安全/容量反证均明确合成，不冒现站已发生失败。

候选只隔离构建：使用当前canonical SDK8506、标准build及验证器/局部registry。fetch仅新增根ok:false且无status/statusCode时network_error分类；其他HTTP状态与失败守卫保持，未改timeout/请求。当前canonical实际落地和Native未执行；原ownerSDK动态候选/分类候选均冻结保留。原icon.ico原bytes保留，素材低清适用性仍OPEN，App显示圆角无改动。

## 历史010实现与验收记录（下文为原记录，不是011当前限制）

# 創漫繁中・隆少年 0.1.0

仅提供創意漫畫大亂鬥原创平台公开一般作品《隆少年》，作者 ULTIMATE。原 slug ronshaonien；官方 XML 集數=全，作为一个完整阅读单元，不扩其他作品、成年区、图库或小说。status、正式 contentRating、usage 为 unknown，不用首页200或目录条数冒充使用率。

- Source ID：`com.weicomic.ronshaonien.zh-hant.koma`，API0.2，最低App0.1.1。
- Work ID：`weicomic:zh-hant:ronshaonien`，chapter ID：`weicomicread:zh-hant:ronshaonien:all`。
- chapterNumber/volumeNumber null；readingGroup 为本作品 ID；continuationEligible false，无自动相邻合同。
- 30 原实体顺序保留，跨页02-03、22-23各为一个图，不按印刷页名称造32图。PageID为上述真实单元的0..29原ordinal，不从文件名推章号。

## 官方原合同与源实现

https://www.weicomic.com/ 首页明确正式网址/原创平台，真正一般article内链接 `comicview-shocomic/ronshaonien/`、title隆少年、authorULTIMATE、一般标记、cover `image/shocomic/ronshaonien.jpg`。目录/详情/搜索只验证这一个作品的现存article，用当前生产HtmlHost/Lexbor的真实ASCII CSS与presence-qualified `[src]`读取，不调用缺失属性。全部node/doc按结果关闭。

实际reader https://www.weicomic.com/comicview-shocomic/ronshaonien/ 声明 `makeRequest('../..//shocomic/ronshaonien.xml','/shocomic');`，原UTF8 XML https://www.weicomic.com/shocomic/ronshaonien.xml 明确名/作者/集數/作者介紹/留言位置。官方loader→template→viewer的原父链保留：XML按頁顺序读取名稱/圖片；`../../`+cpath+`/`+圖片经浏览器普通URL解析，赋给imgcontainer.src。不存在凭据、签名、JS执行或文件名猜测。

当前App生产HtmlHost对该原XML中文selector Select=-1/SelectAll0，这个失败保留。本源不修改SDK/HtmlHost、不把XML改造成合成HTML。源内有界UTF8原字节扫描仅接受这个真实XML结构：匹配原根/两段、完整5身份属性、30自闭合頁；所有重复属性、DOCTYPE/外部实体/任何entity引用、异常结构、缺关键属性、身份错误、页数变化、重排、不同名称或路径均fail closed。原静态30许可表只是逐项核对；实际输出URI由这次真实XML圖片属性组合生成，失败没有硬编码回退。原UTF8错误源码明确拒绝；保留host文本transport未运行invalidUTF8 fixture，不把源码检查称该ABI路径测试成功。

Browse/detail为1次首页GET；chapters为首页+reader+XML三GET；get_pages为reader+XML两GET；每正文image_request再次reader+XML两GET验证全部30当前页及ordinal完整URI（5570B保留metadata）；封面计划为1首页GET。无共享或跨实例缓存，没有改变ABI。图片计划不抓取图片，输出原URL及空headers；真实匿名首图/封面已经无凭据GET成功，App实际图片transport仍未验。

封面仅原 https://www.weicomic.com/image/shocomic/ronshaonien.jpg ，接受pageId缺失或正确MID，拒绝其他MID、空串及正文PageID。正文PageID必须0..29canonical十进制、同ordinal原完整URL；无ID、错work/ordinal、额外query、跨host/协议/目录/traversal均拒绝。若独立mangaUrl字段存在，只接受原reader完整URL。

## 原图标格式具体边界

首页唯一明确身份图为 `seasonLogo` https://www.weicomic.com/pos/top-index/top-06.gif ，原67435B单帧GIF1026×102，SHA62cb49f927a250748d5c0171d8857c6f2251ed794968fd47df08964c1ba9c645。源目录 official-brand-original.gif 与原bytes相同，不裁切/缩放/重绘/抠图/加遮罩或底板、不转码/伪装扩展名。现有build和App importer只接受可选root icon.png，PNG_SIGNATURE/maxDimension1024，因此该GIF格式及宽度不兼容。本候选包只有manifest.json/source.wasm，不携带icon；原GIF仍作为源素材/证据保留，小槽可辨性及图标未验收。不借用别站/品牌图，不修改App/SDK，也不猜隐含favicon请求。

## 证据与验收边界

原官网绑定 `/private/tmp/koma-weicomic-zh-hant-public-contract-20261014/readonly-binding.json`，41文件原SHA全部保留。原首JPEG343245B/800×1133灰阶、多格繁中叙事已查看；这次只新增经授权封面GET20011259B/RGB141×200，SHAabbe179c2c3c6e682fc97cc83e052abe74811b77b66242173b4dd7a7a6a3101f。其余29正文实体没有请求、解码或视觉验收。

69生产WASM操作：40真实原HTTP正向合同（含30image计划）；19合成错误请求；10单列合成异常XML，后者是fail-closed测试，明确不是官方HTTP响应。原站成功回放均绑定原body字节，不造响应。原当前html_host.cpp/.h及1057Lexbor文件保持，macOS Wasmtime22＋真实App C++ HtmlHost用于首页操作；完整ABI gate及所有operation descriptor0。不是197/WAMR或下载接受。

Overwatch繁中PDF分支与探測機首图522原失败保持，不重试或换域/协议/client，不据另一作品成功宣称这些分支通过。没有设备/IDE/签名/提交/推送/发版，App/Kit/SDK/主源WIP与BNS及所有旧冻结源/包保持。Root下一步审全部Rust/README/diff/真实parent/XML/Host轨迹/ABI和冻结后决定普通197安装/30页阅读、下载、进度与cold。

## 2026-10-17 required identity resource

The new icon-only candidate uses the genuine local resource documented in ASSET_SOURCES.md. The earlier optional-PNG/no-icon package description remains historical evidence, not the current production resource rule. Manifest.icon names the actual root file; WASM, version, stable identities and parser behavior are unchanged. Old frozen packages remain separate. Native icon display has not been accepted; App rounded clipping remains.
