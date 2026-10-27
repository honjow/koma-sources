# 3Hentai 中文 0.1.2

独立源身份 `com.hentai3.zh.koma`，API 0.2，主页 `https://3hentai.net`，最低App0.1.1。公开中文popular列表、详情、单作品章节及原图描述已实现；搜索、latest、filters、home不提供。混合成人源，单作品分级及用量未知。不登录、不发送凭据。

## 标题设置

`full_title`（Display full title）默认开启，保留当前Koma完整标题和无配置输出；固定Keiyoushi `949dea252eb8e867b7b40834a26e13088764ff55` 的默认值是关闭，这里按已定保当前默认适配。显式关闭后，列表和详情共同标题输出按参考 `(\[[^]]*]|[({][^)}]*[)}])` 删除标签再Kotlin.trim；复用SDK同算法显式布尔入口，不伪造请求。原HTTP、ID、单章名Chapter、原图顺序、封面和已入库旧标题保持，设置不会主动改Library。

## 归属与图像

作品ID为 `h3:zh:数字`，章节为 `h3ch:zh:数字`，页为 `h3page:zh:数字:index`。详情核canonical数字归属及实际中文语言声明；完整页数、唯一性和原图顺序检查失败则整次失败，不发布截断结果。列表next只接受实际同中文popular下一页地址。

共享图像校验仅接受已观察的s2/s4/s9与精确3hentai.net、3hentai.xyz CDN、HTTPS和合法文件形状。封面目录数字不等于作品ID是已核真实卡片合同，封面以原实际卡片描述为准；正文仍核数字资产与作品/页号关系。get_image_request不新增HTTP，不变换图像，不清缓存或改会话。原180PNG图标保留。

## 证据边界

既有保全HTML及生产HTML解析桥支持具名列表/详情/页表合同；受控WASM重放仅证明相同原件的解析与标题选项，不等于当前站点HTTP、原图下载或Native完整可用。未具名通过的当前正文/图像路径继续保留未验证状态。
