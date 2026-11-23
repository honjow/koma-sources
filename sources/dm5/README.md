# 动漫屋 / DM5 0.1.31

## 0.1.32 簡介與多語言顯示

動漫屋（DM5）的原創漫畫與日本漫畫，涵蓋熱血、冒險、奇幻及戀愛等題材。付費章節依原站權限開放。

零參 `source_info` 固定使用以上基準簡介；`metadata.localization` 僅新增 `source.description` 的繁中、簡中與英文文案，供支援的宿主顯示。

源 ID `com.dm5.koma`，语言 `zh`，NSFW 标记保持。普通网站及生产请求主站为 `https://www.dm5.cn`；中文显示名为动漫屋，英文身份仍为 DM5。图标身份与原字节见 [ASSET_SOURCES.md](ASSET_SOURCES.md)，不因设置接入更换素材或 App 样式。

支持搜索、热门/最新完整分页、详情、目录、原图和图片请求描述。热门与最新是既有列表，不作为独立首页重复展示。

章节身份继续使用原 `manhua` 标识、网站 CID 和原 pageCount；身份/章卷元数据仍沿用既有 legacy 标题字段；续读资格与位置来自用户选择的有效目录依据，完整展示标题独立处理。保留这一边界是为了不破坏已有书架、下载和阅读位置；不能因设置、标题展示或重构改写身份，也不从源语言推断章节正文语言。卡片封面保原父容器关联，正文签名、图片顺序及失败传播保原链路。

## 设置合同

- `sortChapter`：默认 `false`，原网站序列不变。仅用户显式开启时按网站章节 URL 数字由新到旧排列，同号保原相对顺序；ID、pageCount、标题与序列元数据不变。
- `chapterComments`：默认 `false`。仅请求顶层 `hostFeatures` 含 `text_page_image_v1` 时显示并消费，旧 App 即使留有 `true` 也只接收原 URL 页。

吐槽开启后，从原 viewer 保存 `DM5_PAGEPCOUNT` 和 `DM5_TIEBATOPICID`，原图完整取得后按网站 `pagerdata.ashx` 的 pageindex/pagesize/tid/cid/t=9 合同获取评论。每个响应数组转成 `Poster: PostContent` 段落；空数组使用「没有吐槽」。追加的描述为 `image.kind=text`、`image.text.template=dm5-chapter-comments`，保原图身份和排列。

JSON Unicode 与 HTML 实体由源侧解码，保正文标签字面量和 CR；复用既有 HTML5 Host 表并关闭 descriptor，不增加依赖库或新 Host ABI。损坏、缺必要元数据、容量或网络失败整次拒绝。Host 只把普通 Unicode 按固定本地模板绘制成 PNG，不解释站点 HTML、不外发文字。设置对之后加载的章节生效。

Android CookieManager 的清 Cookie 动作不是普通可保存设置，本源没有伪造该开关；不增加账户、Cookie、UA 或网络权限。

搜索先解码请求中的 JSON 转义，再按既有站点请求规则编码用户输入；默认、空查询与分页规则保持原行为。

## 0.1.26：目录边界和停止提供

章节列表容器内含嵌套封面和信息 div；以完整匹配容器为界，保留原章节身份、顺序与显式排序，结构缺失、未闭合或无法识别时仍失败。仅无目录且本作品详情主栏明确显示站点不再提供该作品时返回 `not_found`；核作品路径及名称，不扫描推荐区，也不让警告覆盖有效目录。


## Mihon 恢复

上游动漫屋扩展把作品和章节链接去掉站点前缀保存，原章节 `/m{cid}/` 对应本源章节 ID `chapter:{work}:{cid}:{pages}` 的第 3 段。双方正文使用同一章节的站点图片顺序，章末评论在正文之后。Koma 按目录实际图片数校验页码。参考 [Mihon 动漫屋扩展](https://github.com/keiyoushi/extensions-source/blob/main/src/zh/dm5/src/eu/kanade/tachiyomi/extension/zh/dm5/Dm5.kt) 的 `chapterListParse` 和 `getPageList`。

半模态章节评论使用原章节页的讨论 ID，再按官网 `pagerdata.ashx` 的分页取评论。按原站顺序保留作者、正文和时间。章末吐槽的开关及原页表继续使用既有规则。


## 2026-11-23 完整目录位置与源排序依据

get_chapters成功结果使用typed SDK。实际官网manhua-yidengjiading目录
chapterlistload包含detail-list-select-1的390项，order控件为正序；固定上游
949dea2的Dm5.kt同样据a.order文本识别DOM方向。读取所有order控件并要求方向
一致，按真实UL分区（含嵌套隐藏chapteritem）的完整原位置赋readingOrder，未编号或“第N回”等原来没有
数字元数据的条目仍能沿该分区续读。缺少/矛盾方向或非已识别分区的条目保留
为可选独立单章组，不猜跨分区连接；实际版本栏目始终独立。

默认false沿网站DOM方向；开启sortChapter后遵循“依照上傳時間排序章節”
的源内目录依据，按业务URL ID由新到旧返回，并在各真实分区内按上传依据
重新分配正向阅读位置。该源设置允许改变阅读顺序，不能降为仅展示；App自身
列表正反序才是纯展示投影。不同分区仍不串读，不按标题/话号替代上传依据。

采用版本949dea2的chapterListParse和当前main入口→multisrc.dm5.parseChapterList
均明确默认false、开启即按业务ID降序；Mihon Reader以sortDescending=false
消费章节序列。原“设置前位置不变”检查已作废；以真实上游合同与所选有效目录为准，
不能因某些样本的DOM与上传ID排序恰好相同，就把该源选项降为仅展示。
只按业务cid合并重复，保留不同ID同标题/编号；原
256字节身份/页数算法、chapter:work:cid:pageCount、完整卡片显示标题、编号
元数据、嵌套div目录闭合检查和明确下架识别均保留。分页、正文、登录及图片
访问规则未改。原站和固定上游依据在Koma审计static-provider-dm5-work-real.json、
static-provider-dm5-chapters.json，候选结果另记static-*，不替代真机验收。


## 2026-11-23 最终候选回归

候选版本0.1.31由最终.koma包内WASM的source_info与manifest一致读回。原前5槽为4完整+1站点下架拒绝，既有第6槽补采47章完整，合计5完整；拒绝原件保留。
目录逐项核ID、标题、原显示顺序、元数据和完整位置；实际执行App现有投影、
章序、邻章函数及反向显示投影，get_pages代表回归独立记录。原件同响应回放
与最终包代表性真实HTTP均保留于Koma审计static-expanded/及static-final-packages.json。
这些结果不冒充设备安装、图片解码或普通App UI验收。
