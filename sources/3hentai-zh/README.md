# 3Hentai 中文 0.1.5

独立源身份 `com.hentai3.zh.koma`，API 0.2，主页 `https://3hentai.net`，最低App0.1.1。公开中文popular列表、搜索、筛选、详情、单作品章节及原图描述已实现；latest、home不提供。混合成人源，单作品分级及用量未知。不登录、不发送凭据。

## 标题设置

`full_title`（Display full title）默认开启，保留当前Koma完整标题和无配置输出；固定Keiyoushi `949dea252eb8e867b7b40834a26e13088764ff55` 的默认值是关闭，这里按已定保当前默认适配。显式关闭后，列表和详情共同标题输出按参考 `(\[[^]]*]|[({][^)}]*[)}])` 删除标签再Kotlin.trim；复用SDK同算法显式布尔入口，不伪造请求。原HTTP、ID、单章名Chapter、原图顺序、封面和已入库旧标题保持，设置不会主动改Library。

## 归属与图像

作品ID为 `h3:zh:数字`，章节为 `h3ch:zh:数字`，页为 `h3page:zh:数字:index`。详情核canonical数字归属及实际中文语言声明；完整页数、唯一性和原图顺序检查失败则整次失败，不发布截断结果。列表next只接受实际同中文popular下一页地址。

共享图像校验仅接受已观察的s1/s2/s4/s9与精确3hentai.net、3hentai.xyz CDN、HTTPS和合法文件形状。封面目录数字不等于作品ID是已核真实卡片合同，封面以原实际卡片描述为准；正文仍核数字资产与作品/页号关系。get_image_request不新增HTTP，不变换图像，不清缓存或改会话。原180PNG图标保留。

## 证据边界

既有保全HTML及生产HTML解析桥支持具名列表/详情/页表合同；受控WASM重放仅证明相同原件的解析与标题选项，不等于当前站点HTTP、原图下载或Native完整可用。未具名通过的当前正文/图像路径继续保留未验证状态。

## 搜索

按固定参考向关键词追加 `language:chinese`，使用独立数字分页；保留既有作品ID、完整标题默认值和已入库身份。2026-10-28 官网搜索结果直接声明 s1.3hentai.xyz 封面，因此增加该已观察分片。生产WASM真实搜索 love 返回作品与下一页、游标2前进到3，实测末页78无下一页；无匹配词返回空，带双引号关键词以JSON解码后编码到请求。一次第2页 TLS EOF 已保留，后续真实重试成功。Native与公开包分别验收。

## 搜索与目录筛选（0.1.5）

按固定参考提供排序及 Tags、Male Tags、Female Tags、Series、Characters、Artists、Groups、Languages、Pages 文本筛选。逗号拆分、去首尾空白、完整 Unicode 小写、负号排除与性别标记沿参考；搜索和浏览均消费明确选择，并始终追加 `language:chinese`。原搜索默认 `sort=`，热门默认 `/language/chinese/?sort=popular` 及原完整URL游标不变；排序 schema 不设置跨操作统一默认，只有用户选择才改变。筛选后的浏览/搜索使用实际原 search 页面和数字游标，核原下一页查询、排序和页码。

桌面生产WASM真实 `love + yaoi + popular` 搜索及筛选热门各前两页25条、游标推进；recent、负号排除及空结果具名通过。原 C++ Host 真实保存的已筛选HTML核标签、排除及sort；旧0.1.4与候选默认搜索/热门URL和data对照使用明确受控HTML，不冒原站原体；完整/短标题关闭对照保持。非法筛选在HTTP前失败。设备 Native 筛选控件和普通分页另验。

参考还用静态 Header 解释逗号、负号及页数语法；现有宿主过滤器模型不支持该类说明行，当前仅提供原字段与选项，不把说明展示或设备交互称已完成。
