# GlobalComix 中文 0.1.3

源身份 `com.globalcomix.zh-hans.koma`，语言 `zh`，API语言 `cn`。提供公开列表、搜索和详情；目录与正文只接受明确免费的 release/page，并校验 comic/release/page 归属，未知或付费章节内容保持拒绝。原ID、目录与图片顺序、原请求头、详情中同实体声明的网页链接保持。

## 设置

`dataSaver_cn` 为默认关闭的省流量开关。关闭或未配置时使用原 `desktop_image_url`；开启时使用同次API同页的 `mobile_image_url`。两种输出保相同页ID/index，均须通过原免费和归属检查。图片只接受原站及API实际声明的精确desktop/mobile CDN，缺失或无效图片报错，不猜URL或退回付费内容。

参考的 `showLockedChapters_cn` 默认开启会改变本源既有免费目录合同，当前不提供这个开关；不以无效设置或擅改默认值替代处理。

## 构建与实现来源

使用 `bash build.sh --source globalcomix-zh-hans`。源级MIXED沿既有合同报告nsfw，不表示每本作品成人。只消费公开API，不添加账户、Cookie或付费权限设置。

实现依据Keiyoushi固定版本及Apache-2.0归属见NOTICE.md；省流量项按固定tree `949dea252eb8e867b7b40834a26e13088764ff55` 的GlobalComix.kt/Constants/DTO。原官网声明的icon.ico逐字节保留，来源见ASSET_SOURCES.md。

既有《泰坦的餘燼》具名在线首两图观察只限其原版本/路径；原《侯門少爺寵上天》已下载章不能代替在线第二页复验，该旧边界仍未关闭。不将旧响应原体的受控WASM回放冒充当前HTTP或新包Native结果。
