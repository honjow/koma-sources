# Cartoon18 中文 0.1.3

身份 `com.cartoon18.zh.koma`，API0.2，通用中文界面源；不保证每本漫画的图片语言或内容分级。源级NSFW沿原合同保留。提供热门、最近更新、站点查询、可选分类和排序、详情、目录、原页序图片描述及归属校验后的图片请求；账户和镜像不在当前能力中。

## 设置与语言

`ZH_HANT`（Use Traditional Chinese）默认关闭，沿原 `/zh-hans` 路径。显式开启使用同站根路径的繁体界面。列表、搜索、详情、目录、章节和图片归属请求都消费同次请求的设置快照。

Koma既有 `c18:zh:`、`c18ch:zh:`、`c18pg:zh:` ID不含界面语言路径，所以从当前设置派生路径；不为切换语言新增或重组书/章/页ID。所有canonical、目录成员、章回链、cursor query/mode/所选语言路径检查保留。旧语言cursor在另一设置下拒绝，设置改变须从原列表刷新入口重新开始。

题名仍仅扣除实际唯一尾部Vue按钮的DOM文本。详情genre忽略站点空 `/q` 标签，沿参考的trim/filterNotEmpty语义，不因空占位使整本详情失败；其它标题和归属检查保持。

## 运行合同

原请求头、15秒HTTP预算、Cookie/账户边界、AVIF图片精确主机/路径/query校验及全部容量限制保持。页表保命名作品成员、章节canonical/作品回链、data-seq顺序、去重及完整性校验；图片请求仍对实际归属封面或页逐项比对，不猜图URL。不把语言开关当图片文字翻译。

使用 `bash build.sh --source cartoon18-zh`。独立实现及Apache-2.0归属见NOTICE.md，图标来源见ASSET_SOURCES.md；语言偏好依据Keiyoushi固定tree `949dea252eb8e867b7b40834a26e13088764ff55` 的Cartoon18.kt。能力、真实WASM HTTP、图片URL描述与Native像素各按具名证据区分，不据接口成功扩大为所有作品可读。

## 参考功能与调用链

固定参考 Keiyoushi 949dea252eb8e867b7b40834a26e13088764ff55 的 Cartoon18.kt：热门 hits、最新 created；查询排序 created/hits/score/likes，默认 score。分类选项来自所选语言的 /category，只有空查询时使用关键词。参考以 next 链接是否存在及 disabled 判断续页；详情、目录、正文沿同一上游站点；未见账户或镜像功能。

Koma get_listings/get_manga_list 分别承载独立热门和最新，search 承载文本及筛选查询。仅同时声明 source_filter_scope_header_v1、source_filter_fetching_v1、source_filter_search_without_query_v1 的宿主可收到 useSearchForFilters 和 search 作用域筛选，基础及动态响应均声明，旧宿主返回原空筛选。热门/最新不消费筛选，保持 hits/created；筛选 search 允许空词，用户非空词优先，空词可用关键词；未选排序沿参考 score。游标绑定关键词、栏目、排序和语言，失败不能转向其他上下文。站点默认封面仅允许确切的 imgdef/node.avif；其它封面仍依原精确检查。

此为未发布工作区候选：简繁分类、首屏和分页等价已本地回放，实际普通筛选界面、连续滚动及旧宿主设备行为尚未验证；并非全功能验收。
