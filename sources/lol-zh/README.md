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


## 237 现装0.1.0有限观察（2026-10-21，可读性未关闭）

普通目录实际六部中文系列。《拉克丝》详情已有原第一话2/22页进度，且进入前reader含同一记录，本次未打开其阅读、不把旧进度冒作正文成功或重跑该样本。改用无现存进度的《织星》，详情实际3/3章，现有开始阅读进入 `com.tencent.lol.zh.koma:lol:zh:star-guardian` / `lolread:zh:star-guardian:issue-1`，普通右侧翻页实际1/17→2/17，两张原图均解码。

两页均为极长彩色条漫，在当前既有分页模式中按整高显示为窄栏、中文对白很小。因此本次只证明上述两张图像加载与普通翻页，不以“实际可读”关闭当前阅读体验，不冒整17页、其他系列/章节、下载、cold或无缓存在线首取。没有更改模式、切图、缩放源图、App/Kit/源业务或访问原404分支。

自然Back后真实进度保存为 `lolpage:zh:star-guardian:issue-1:1`、pageIndex1、totalPages17、completed=false。原件在Koma/.hvigor/outputs/user-flow-audit/device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership/：lol010-public-list237-001、lol010-lux-detail237-001、lol010-star-{detail,first,second,natural-return}237-001、lol010-session-restored237-001。恢复回执实读原Browse末端、AWAKE/OverrideTimeout10000ms；52项registry全bytes同，全部原reader记录（含Lux及BH3）逐项保持，只自然新增织星progress与chapter各一条。ec8c1b88已释放，设备回执关闭哈希，无登录、匿名建号、收藏或下载。


## 同一织星章本书连续滚动可读性及恢复（2026-10-21）

随后按本轮明确授权，只在同一《织星》游说章原P2临时验证已有本书连续滚动，不重复首图增加验收数量。先通过普通设置→阅读实读原全局为单页、从左到右、跨页合并、黑色背景；原Reader设置为单页，无本书标记。源码既有saveChangesForComic/persistForComic仅对当前comicId存模式/方向覆盖，全局两值保留；设回与全局相同值时删除该书覆盖，未修改这条逻辑。

通过普通Reader齿轮→模式→连续滚动，原2/17页按屏宽显示；在同页普通滚动后，中文“据说，它代表了宇宙中所有的美好。”清楚可读。只接受这一本书这一页在已有连续模式中的可读性；原单页整高窄栏现象仍保留，不推断所有长条、全部17页或默认选择均已验收。

通过原模式菜单选回单页，原2/17·游说状态实际恢复。普通返回再进全局设置，前后22个可见标签/值逐项相同；最初现存偏好目录无Reader文件，普通保存后真实生成koma_reader_preferences_v1，最终只读XML为pageMode=single_page、readingDirection=left_to_right、reader.seriesModeOverrides.v1={}。这是模式/覆盖与全局语义恢复，不是原不存在文件的全字节恢复；没有直接编辑或删除持久偏好。

原图/回执为category-membership/lol010-global-reader-{original,restored}237-001、lol010-book-settings-original237-001、lol010-book-mode-menu-original237-001、lol010-star-continuous237-001、lol010-star-continuous-readable237-001、lol010-book-mode-restored237-001。最终lol010-continuous-session-restored237-001含reader-preferences.xml、reader.json、registry.json和run-metadata.json：52项registry全bytes同，其他书原reader对象全保持，织星P2/17保留且只其自身时间戳自然更新；原Browse末端及AWAKE/OverrideTimeout10000ms恢复，df6b1c67已释放，哈希关闭。
