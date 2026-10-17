# 英雄联盟官方漫画（中文）

独立候选 `com.tencent.lol.zh.koma`，0.1.0，语言 `zh`，最低 App 0.1.1；使用既有 version-1 请求/响应和 Host ABI，不修改 SDK、App 或 Kit。

只支持本轮有完整中文系列元数据的六部漫画：织星（3话）、刚愎之心（6话）、和音（5话）、劫（6话）、拉克丝（5话）、战母（4话）。目录来自原站中文 Hub 的 `sections.series.data`，不把最近新增的六话和七个短篇卡片当作十九本书。原 `seriesList` 的十二个 slug 不是中文书名；其中五个短篇的系列详情路由真实404，韦鲁斯系列 name/subject 仍是英文 slug，均不列入本候选，也未重试这些分支。

官方关系有腾讯英雄联盟 `lol.qq.com/main.shtml` 的真实“LOL宇宙”导航指向 `yz.lol.qq.com`。原站 app JS 声明了同源中文 Hub、系列 metadata 和章节计划 JSON 的端点。全部六部作品的名字、简介、话名和显式 index、29话原计划已留证；章节号只来自显式 index，volume 始终 null。明确完整且连续的同系列目录可以用既有 continuation 字段；这只声明元数据关系，不证明 Native 自动交接。

浏览/本地中文文字搜索各取一次当前 Hub；详情/章节各取一次所选系列 metadata，身份、文字和全部显式目录与已审合同比对。搜索只覆盖六个已核对系列的标题/简介，不是全站搜索；没有 latest、home、filters、settings 或账户能力。

`get_pages` 每话一次正常公开计划 GET，验证实际 id、空 `mobile-pages`、全部 `desktop-pages` 分组大小和原 uri 顺序。29原计划的 mobile-pages 都为空，原站 ComicResolver 的既有 fallback 是按组和组内次序展平 desktop-pages。共531个base descriptor（包括原封面/标题页）全部保留；不合并双页、不裁图、不替换图、不把2x替代项添加为额外页。拉克丝第一话原22张中的前两张封面/标题图保持，唯一取过的实体是第3 descriptor 的中文首张故事正文。

原计划逐项生成 `frozen_contract.rs` 的不可变精确URI许可表，不是跨操作内存缓存。每次新Module的 `get_image_request` 都独立验证：正文必须带对应工作/话/原下标的 pageId；封面支持实际App的 pageId=MID 以及无pageId的Host形状；可选 mangaId 和 mangaUrl 若给出必须匹配该作品。URL（含query）必须逐字命中该身份的官方表；外站、错图、错页、错作品/话、错误mangaUrl、变更query、2x当base页、路径穿越均拒绝，零HTTP。页计划/目录/metadata改变即fail closed，需重审并更新源包；不为每图反复下载目录或计划，也不依赖不存在的cache ABI。

沿当前已冻结独立源采用 source-local checked_request/alloc/exports 门禁。仓库旧 `koma_source_exports!` 宏直接读取未检查的ABI slice且init只检查非零length，没有alloc；因此保留 buffers/helpers 两宏，在此源本地导出全部既有操作并检查内存范围、取消、args、operation/sourceId，不扩大SDK/ABI。实际ABI边界测试单独留证。

当前图标采用官网 HTML 明确声明的192×192 android-chrome PNG原字节，出处、原URL和SHA在 `ASSET_SOURCES.md`；原官方金L、青色圆面和金环保留，外部不增加遮罩/底板/阴影。历史230×230官网PNG及其外圈灰黑halo反例保留在该出处文档和原件中，当前不再打包230版本。使用率、逐书年龄评级和整站支持均未知。描述符计划及零HTTP图片请求证明不等于图片实体下载/解码：只目检了原唯一Lux中文实体，其他530实体、全部2x、Native读取/下载/cold均未验收。

本轮证据：`/private/tmp/koma-lol-zh-public-contract-20261014`（最初原合同）和 `/private/tmp/koma-lol-zh-implementation-20261014`（11单次metadata回执含404、28新计划、许可生成依据、生产WASM回放、ABI、精确diff/绑定）。没有设备操作、推送或发布。

简介修正R2：原metadata/身份/URI许可表保持，source-local `generate_description_text.py` 从同一原JSON生成 `description_text.rs`；有HTML时以标准HTML5实体解码和p/br文字投影保留原文及段落，空p也保留；五个纯文本简介（包括原换行）不变。Guest仍逐字验证原metadata，变化需重审，不把合成语义例子当原站/Native证据。织星旧包的HTML标签直出失败和旧包保留在原实施产物，新对照/交付在 `/private/tmp/koma-lol-zh-description-r2-20261014`。不改App文字消费或样式。
