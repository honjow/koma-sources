# 漫小肆（Manga Xiao Si）0.1.6

源身份 `com.mangaxiaosi.zh.koma`，API 0.2；提供人气榜、搜索、显式目录筛选、作品详情、章节及图片请求。源级 NSFW，单部作品内容分级未知。没有独立首页、最新列表、账号或 Cookie 管理、付费绕过、JavaScript 执行或自动重试。

## 镜像与请求

`preferredMirror` 在固定参考的 `https://www.jjmhw2.top`（默认）和 `https://www.jjmh.top` 中选择；缺省或未知值保留原主站，不自动切换。人气榜、搜索、筛选、详情、目录和页表使用同一个所选来源；原 Chrome 140 User-Agent、方法和请求路径保持。主页声明仍为原主站。

人气榜与文字搜索保持原有限查询合同；搜索不声称精确匹配或额外分页。题材、地区和进度只在显式目录筛选时消费，下一页必须属于同一筛选，旧宿主不展示筛选。无效或不匹配的分页、站点结构和身份明确报错，不猜终点。

## 身份、目录与图片

作品 `mx:zh:<work>`、章节 `mxch:zh:<work>:<chapter>` 和页面 ID 不含镜像域；两个固定站点返回的有效绝对链接均可解析。目录保原输出和 `chapterOrder: newest_first`；章号只从明确的章节标题提取，不用数组位置冒充。

页表必须先在该作品目录确认章节成员，并核对章节页面所属作品。封面和正文沿用服务端图片原 URL，仅允许 `www.jjmhw9.top`、`www.jjmhw10.top` 和 `www.jjmhw11.top` 的 `/static/upload/book/` 路径；严格校验 HTTPS、路径字符、作品、章节和文件归属，拒绝路径穿越、未知主机及跨作品或跨章节请求。宿主既有重定向处理不变。

图标和来源见 `ASSET_SOURCES.md`；标准构建使用 `bash build.sh --source mangaxiaosi-zh`。真实站点响应、设备路径和受控回放分别按具名结果记录，不推及全部作品或镜像。
