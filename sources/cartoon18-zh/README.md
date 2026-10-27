# Cartoon18 中文 0.1.1

身份 `com.cartoon18.zh.koma`，API0.2，通用中文界面源；不保证每本漫画的图片语言或内容分级。源级NSFW沿原合同保留。只提供热门、站点查询、详情、完整目录、原页序图片描述及归属校验后的图片请求；最新、动态分类过滤、账户和镜像不在当前能力中。

## 设置与语言

`ZH_HANT`（Use Traditional Chinese）默认关闭，沿原 `/zh-hans` 路径。显式开启使用同站根路径的繁体界面。列表、搜索、详情、目录、章节和图片归属请求都消费同次请求的设置快照。

Koma既有 `c18:zh:`、`c18ch:zh:`、`c18pg:zh:` ID不含界面语言路径，所以从当前设置派生路径；不为切换语言新增或重组书/章/页ID。所有canonical、目录成员、章回链、cursor query/mode/所选语言路径检查保留。旧语言cursor在另一设置下拒绝，设置改变须从原列表刷新入口重新开始。

题名仍仅扣除实际唯一尾部Vue按钮的DOM文本。详情genre忽略站点空 `/q` 标签，沿参考的trim/filterNotEmpty语义，不因空占位使整本详情失败；其它标题和归属检查保持。

## 运行合同

原请求头、15秒HTTP预算、Cookie/账户边界、AVIF图片精确主机/路径/query校验及全部容量限制保持。页表保命名作品成员、章节canonical/作品回链、data-seq顺序、去重及完整性校验；图片请求仍对实际归属封面或页逐项比对，不猜图URL。不把语言开关当图片文字翻译。

使用 `bash build.sh --source cartoon18-zh`。独立实现及Apache-2.0归属见NOTICE.md，图标来源见ASSET_SOURCES.md；语言偏好依据Keiyoushi固定tree `949dea252eb8e867b7b40834a26e13088764ff55` 的Cartoon18.kt。能力、真实WASM HTTP、图片URL描述与Native像素各按具名证据区分，不据接口成功扩大为所有作品可读。
