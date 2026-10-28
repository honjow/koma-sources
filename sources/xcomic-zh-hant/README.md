# XCOMIC 0.1.6（繁體中文 / API zh_hk）

独立源身份 `com.xcomic.zh-hant.koma`，API 0.2，主页 `https://xcomic.me`。保持原匿名公开 GraphQL、12卡分页、语言筛选、作品/版本/章/页 ID 和原图顺序。混合内容源标记 nsfw，作品分级依实际响应，缺值保未知；不登录、不取 token、不发 mutation、不换域或变换图像。

## 设置

`REMOVE_TITLE_VERSION`（Remove Version Information From Entry Titles）默认关闭。显式开启时，列表、搜索和详情的共同标题输出按固定 Keiyoushi `949dea252eb8e867b7b40834a26e13088764ff55` 删除版本标签并 trim；关闭和未配置保持原 JSON 标题逐字输出。清理后通过既有SDK的HTML5实体表解码一次，字面尖括号保持文字，不剥除标签。清理后为空的列表标题沿参考回退到实际作品 ID。设置不改变身份、目录或页面，也不主动改写库内旧标题。

使用成熟 `regex` 的无 std 引擎执行固定表达式；Java负向前瞻的方括号分支改写为等价排除字符类，保留 Java 行终止符边界；`Official` 的大小写匹配保留 Java UnicodeCase 的 I/i/İ/ı。每次列表/详情操作只编译一次，默认关闭不编译。自定义正则、服务器去重与分组排序设置尚未接入，不用空 schema 冒实现。

## 归属与可用性

所有实际版本 ID 均逐个核对，不信任声明的版本总数。只接受本语言、public/normal 的版本；详情、目录和页面仍校验作品/版本/章节归属。完整分页的总数、进展、重复 ID 和容量守卫保持，失败不发布截断目录。跨版本同号章节保持不同身份和稳定顺序。页面保留真实原图 URL，权限和 HTTPS 图像校验不变。

标题清理的受控生产 WASM/JDK 对照不代表当前上游 HTTP 或正文可用；原已保全样本只证明其具名范围，当前未验路径继续保留。SDK、App 样式和图标未改变。

## 实现来源

原公开数据合同独立适配自 Apache-2.0 的 Keiyoushi `1319949ca9bc456908196e3ba2d104e94182c5d6`；新增标题偏好及表达式依据固定 `949dea252eb8e867b7b40834a26e13088764ff55`。原JSON有界解析来源及许可见 `NOTICE`/`LICENSE`。当前图标沿已采用的192×192双颜色X去底处理；参考扩展资源归属保留，不声称获得站点官方认可或新增高清资源。

`SOURCE_INCLUDE`（Chapter Editions To Include）仅在Host声明 `source_chapter_exclusions_v1` 时展示/消费，默认空保留原完整目录、请求、标签、ID和相对顺序。显式非空按固定949dea逗号拆分、Kotlin trim与完整Unicode lowercase全等匹配；版本标签复用SDK实体解码和成熟Unicode titlecase，沿参考归一化并为同名不同版本加 `[版本ID]` 区分。基本标签包含同名版本，带后缀标签选择对应版本。源先完成全部原归属/权限/完整分页检查，再返回保留项及实际 `excludedChapterIds`，不把缺失/未加载章当排除。宿主只控制本次可见目录，原Library/下载/读态不删除；显式旧章恢复只返回其合法原身份，页面恢复保持原校验并忽略目录过滤。旧Host即使保存过此字段也保持旧目录。版本筛选格式见schema.description，当前Koma文本设置在输入框下显示完整说明。

`IGNORE_GENRE_BLOCKLIST`（Ignore WebView Genre Blocklist）默认关闭。列表与搜索共用的browse items/pager请求仅在显式开启时向原select加入 `ignoreGlobalGenres:true`，沿固定参考跳过网页类别黑名单。缺失/关闭保留原请求字节；不更改其它类型/语言/权限筛选、详情/目录/页面或原Web会话。

## 版本探测调度

宿主声明 `source_http_batch_v1` 时，详情和目录的独立版本probe沿固定参考每批最多5个，使用原HTTP import的requests/responses合同；原单请求方法、地址、头、正文和超时保持。按输入顺序逐项检查状态、GraphQL和版本身份，全部成功后才发布原目录，保留取消、容量、语言及后续作品/章节归属检查和原排序。无该能力的旧宿主继续原逐次请求，不复用跨操作缓存，不把多个GraphQL根字段拼进一个请求。

`source_detail_chapter_context_v1` permits the host to carry the non-sensitive version resolution from a successful detail request only to the immediately following chapter request in the same live transaction. The source re-fetches the work, verifies the source version, work identity, complete ordered edition IDs and every result shape, and falls back to fresh probes if the context is absent or invalid. Exact chapter recovery never uses it; no source-side cache or persistence is added.
