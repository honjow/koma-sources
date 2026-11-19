# 漫小肆韓漫

独立源 `com.ikanmh.koma`。按用户决定沿用 Keiyoushi 的独立移动端扩展和 Mihon 源 ID `8826152585666516354`。站点身份参考 [原移植](https://github.com/keiyoushi/extensions-source/pull/20221) 与 [镜像验证](https://github.com/keiyoushi/extensions-source/pull/20903)。

| 功能 | 上游与 Koma |
| --- | --- |
| 热门与分类 | `Ikanmh.browseBooklist` 对应 `list`，请求 `/booklist/`，默认全部分类，地区、进度各为 `-1`。筛选值与上游一致，按启用的下一页按钮推进。 |
| 最新 | `getLatestUpdates` 请求 `/update`。首屏省略页号，末页由按钮决定。 |
| 搜索 | `/search?keyword=`，首屏一次性结果；文字搜索独立于分类筛选。作品和章节的站内链接由所选镜像定位。 |
| 详情和目录 | 读取同一个 `/book/{id}` 文档。目录从旧到新改为新到旧，整书更新时间仅赋给最新章，只有站点的明确话号参与编号。 |
| 正文 | 对应 `#cp_img img[data-original]`，保留目录和图片的站点顺序。 |
| 设置 | 同时支持 ikanmh.xyz、ikanmh.cc、www.jjmhw12.top。默认 ikanmh.xyz。 |

上游固定于 8947c372cd87f546a0b80cb0e4b0a5a8add5ef88。本机对照了实际热门、最新、搜索和详情的两种响应。站点按文档 User-Agent 切换模板，三个已声明域名共用上游移动版解析，原生文档请求明确使用移动浏览器 UA。素材取自官网原件。

官网两个移动端域名的存档在同一详情地址也返回桌面模板。按已取回的两份实物兼容详情和目录，保留上游移动模板为首选。

237（Koma 1.5.4）在书源管理同时列出漫小肆 0.1.10 和漫小肆韩漫 0.1.0；独立移动源的热门、搜索、封面、详情、7 章目录和阅读首图验证通过。默认入口读到 1/292，另两处入口在主机验证目录可访问。
