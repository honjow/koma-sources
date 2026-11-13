# DongmanManhua 0.1.4
稳定 cn.dongmanmanhua.koma 和规范名称保留。原漫画/路径、章节/漫画/路径和 page:index 身份保留；无别名/迁移。仅公开源。上游 SAFE；单内容/用量未知。
源本地状态优先、完整 20×65536 字节 UTF8/HTML 体和有界输出/节点检查；无传输重试、静默截断或占位 src 回退。搜索发布静态合同；详情限定自有作者/封面/状态；完整观察目录分页；只显式人工标题编号。图像使用原 data-url 和发布 UA/Referer/Origin 描述符。
保留的 2026-10-18 完整 dailySchedule HTML 是 1,245,612 字节；0.1.2 只把本源 BODY 升到 20×65536 字节（严格容量余量），保留 2MiB HTTP 封套和 1MiB 结果限制。旧 1MiB curl 前缀及其显式拒绝保持历史证据；不完整前缀从不接受。这个候选仍需生产宿主验证和 App 8MiB 池/Native 验收；CLI 成功不是那个证明。未添加最新/时钟。搜索 /episodeList 别名 301 保留原身份；native 正常重定向行为未验证且离线不模拟为别名 200。Root 拥有 197 普通验收。无离线结果证明 native 可读。
当前生产构建：`bash build.sh --source dongmanmanhua` 从规范仓库，带其当前 SDK 和校验器。历史源所有 package_candidate.py/借用构建是先前候选工具，不导入规范。当前标准编译 WASM 必须与先前 Native 验收的二进制单独绑定。原图标和 ASSET_SOURCES 保持字节精确；App 显示裁剪保持独立。这里无新解析器修复、部署或网络。

0.1.3 只更正列表 DTO key 从 `label` 到文档的 `name`；`dailySchedule` 和每日连载保持不变。原 Git 源用 `name`；0.1.1 重写引入了 `label`，当前 App 在调用 get_manga_list 前丢弃那个无效描述符。这个后继保留已审 0.1.2 BODY 预算和所有业务解析/身份/顺序/图像函数。0.1.2 Native 空结果证据保持保留；更正的普通列表/真实封面/有限滚动仍需 Root 审查 Native 验证。无 App/SDK/池变化。

0.1.4 只改变共享浏览/搜索卡片解析器中的卡片封面 URI 生产：对精确观察的 `https://cdn.dongmanmanhua.cn/` 卡片查询 `?x-oss-process=image/resize,w_126,limit_0/sharpen,100` 和 `?x-oss-process=image/resize,w_138,limit_0`，保留相同资产路径并选择 `?x-oss-process=image/resize,w_306,limit_0`。这是从真实同资产官方详情声明和原 126/306 JPEG 对的有界模板推断，不证明每个站点资产可用或 Native 质量被验收。完整保留卡片 DOM 只有 src/width/height/alt，无 srcset/data 替代。未知、签名、质量和更大图像查询保持字节精确。详情已消费站点显式 306 声明；章节/页/image_request 函数不变。改变发出的卡片 URI 自然使其有不同既有缓存身份；旧缓存不清除，不声称自动缓存刷新机制。App 裁剪、几何、SDK 和运行时池保持不变。只有新候选；规范/Native 需要 Root 审查。


## 2026-10-21：237 现装 0.1.4 具名普通路径

现装独立源 `cn.dongmanmanhua.koma` 0.1.4（非繁中 WEBTOON）经普通 Browse 进入「每日连载」，真实列表显示 1317 本及封面；`get_listings` 成功，`get_manga_list` 为 HTTP 200 / `operation_ok`。旧 0.1.2 空列表在该现装版本及该入口已被后继 DTO 修正取代；不扩大为全部列表/搜索通过。

沿列表进入《如出一辙的女儿》（`titleNo=1954`），详情、11 章目录、既有第 1 话 `get_pages` 均为 HTTP 200 / `operation_ok`。普通进入第 1 话后先见声明页 1/176，再用原翻页操作回到原有第 2/176 页，亲看真实漫画正文。返回详情后，进入前/返回后的该 comic 读档片段逐字节相等：同一 comicId、chapterId（episode_no=1）、pageId=page:1、pageIndex=1、totalPages=176、progressRatio、completed=false、updatedAt 与 columnMode=odd_left 全部保持；没有注入或直接写持久状态。仅接受此具名列表→详情→第 1 话正文→返回路径；未验整章读完、其他章、下载、冷启动或所有图片。

原图与原码沿 Koma `.hvigor/outputs/user-flow-audit/device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership/` 保留：`dongman014-list-result237-001`、`dongman014-detail-ready237-001`、`dongman014-original-progress237-002`（进入前准确进度）、`dongman014-original-reader237-001` / `dongman014-reader-ready237-001`（get_pages）、`dongman014-original-page2-237-001`（真实正文）、`dongman014-return-original-progress237-001`（返回后准确进度）、`dongman014-session-restored237-001`（恢复）。已普通返回原 Browse 末端，49 源 sourceId/enabled 与 `noy013-manager-open237-001` 完全相等；OverrideTimeout 恢复 10000ms 且读回 AWAKE，原租约已释放。本轮未传包/新增下载，Download 读回无本任务遗留的 Noy 包；197 未操作，哈希关闭。

## 0.1.5 详情来源网页

本次只为 get_manga 新增 kind=source 链接，直接复用同次 fetch(path) 的 BASE+path；原 manga: 路径身份已验证且保持，章节、请求、正文、图标及 App 均不改。原 11be01d 输出 links 空数组，ecee8ed 规范化后省略；这些记录未说明有意禁止网页链接，不编造原理由。用户本轮已授权详情网页入口，允许差异只限该字段、交付版本及文档。

父链复用原 1954《如出一辙的女儿》真实作品页 https://www.dongmanmanhua.cn/SUSPENSE/ruchuyizhedenver/list?title_no=1954 的 200/117618B 原体；只从已有保全包提取至 Koma .hvigor/outputs/user-flow-audit/dongman015-detail-fixture.html，原回执为同名前缀 detail-fixture-original-receipt.json，未发新请求。原 episodeList 别名 301 独立保留，不冒成 200。014 旧第 1 话 P2 正文仅保留上述具名历史边界；015 普通观察见下。

单源 offline/locked release 构建及真实 WASM info015 通过。同一 1954 原 HTML 分别回放当前源码重编译 014、保全原正式 014 与 015 实际 WASM，除新增 links 外 manga 字段、唯一 HTTP 请求均深比较相等；惰性 Chromium DOM 禁网适配只证明这些离线输出，不冒生产宿主/Native 等价。输出为 Koma 同产物分区 `dongman014-{link-baseline,formal-original}-offline.json` 及 `dongman015-link{,-formal-compare}-offline.json`。正式 `dist/sources/dongmanmanhua/dongmanmanhua-0.1.5.koma` 为 26164B 三成员包；manifest 仅 version 变更并完整保留既有 metadata.json 中文名，icon.png 与保全原正式 014 逐字节相等（3558B），不改变图标素材。普通观察只接受下述具名路径。

237 VDE-AL00 展开竖屏 1320×2120，Root 指定 Hvigor 签名 Debug 16289681B/23:30:52 普通覆盖安装明确成功，未清数据；系统本地 picker 014→015 普通更新后，服务 54 包、Browse 54 启用与咚漫 015 可见，Browse/Manager 原紫色标识均亲看。沿原每日连载首项 1954《如出一辙的女儿》进入详情，两网页菜单出现；只打开 WebView，加载完成后为准确作品网页，真实 banner、YIDAHM、评分 9.83、简介及章列表可见，同时保留站点青少年模式说明弹层，未点击其按钮。自然 Back 仍 11/11 章及「第 1 话·第 2/176 页」，再回原列表/Browse。未打开正文/外部浏览器、未刷新/登录或新增下载；未导出全库，不能声称全书籍逐项校验。只本次传包已清理并读回不存在，最终 10000ms/AWAKE，租约释放。

原图/回执沿 Koma `.hvigor/outputs/user-flow-audit/device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership/` 的 `dongman015-detail237-001/{manager-icon,browse-icon54,services54,detail-before}.png`、`dongman015-webview237-001/{both-web-actions,webview-workpage,detail-returned,browse-restored}.png` 及 `dongman015-webview-postflight237-001/run-metadata.json`；普通导入成功原图在 `dongman015-import237-001/import-success.png`。导入后脚本误在索引弹层找服务行，已自然返回纠正，未重导；原工具失败保留，不计产品故障。

## 0.1.6 候选：最新与日程去重

固定参考 949dea 的独立最近更新读取公开连载日程并按本地星期筛选；仅在宿主声明 source_private_session_v1 时展示，旧宿主保持原单栏。两个日程列表按参考 URL 去重，保留首次出现、作品 ID 和次序。现有 0.1.5 具名真机证据仍只属于当时版本；本候选仅完成本地真实网页原体重放，尚未真机验收或发布。

237 在 Koma 1.2.0 验证了从每日连载切到最近更新，实际周五栏目 54 部作品与原站对应，并可切回每日连载。其余星期在同份官网 HTML 上用时钟回放。

## 章节续读

官网公开目录可缺少中间话数，例如第 10 话的官网下一话为第 11 话，但公开列表下一条已是后记。只有同一目录内 data-episode-no 与链接一致、且集数连续的条目自动跨章。
