# 崩坏3 IP站 0.1.2

既有稳定 ID com.bh3.koma 和规范崩坏3 IP站保留。语言 zh；本地化中文名可选。
Keiyoushi 当前 protobuf 源 5943234929466346733，zh.bh3 1.6.0/106000，固定主 10f06eb833af5990bc5470c84c5e93edc159ad4e，Apache2。
仅源级 SAFE；作品未知。实际用量未知。

公开匿名 /book HTML 列出实际 div.container 数字 ID/标题/封面；不执行 javascript href。
/book/:id 详情、/book/:id/get_chapter JSON、/book/:id/:chapter 普通 img.lazy.comic_img data-original 跟随主参考。
保留 bh3:book、bh3ch:book:chapter 和 bh3page:book:chapter:index 身份。章节 ID 不是猜的章节编号：
只有显式阿拉伯数字或受支援中文数字 第N话/話/章/回 标签提供 chapterNumber；未知 null。原目录顺序保留。
原时间戳作为 publishedAt 文本保留；uploadDate null 因为当前源没有独立验证的时间戳转换。
终端单一热门列表，严格游标 1 只。搜索/最新/首页/筛选/设置/认证不受支援。
无 cookie、自动重试、挑战/镜像、JS、图像字节转换或新 ABI。固定 SDK Firefox121+公开 Referer/Origin。
全部 HTML 节点释放，至多 2048 记录/页；UTF8/JSON/URL 和缓冲区有界，缺失/溢出拒绝无部分成功。
图像描述符只接受 HTTPS 官方 mihoyo.com 子域或 comic.bh3.com，原地址/头；未知图像宿主拒绝。

历史 0.1.1 初始证据保留一个规范 /book HTTP200/16468B。那个原有限声称不是当前整源验收。
已审 0.1.2 合同保留实际 book1001 目录：六个显式中文章节标签映射到 6..1；序章保持未知。原七个身份/顺序和页实现保留。源合同/diff/宿主证据：/private/tmp/koma-bh3012-chapter-semantics-20261017/；其原所有者和临时历史打包脚本保留在那里，那个旧脚本不是规范发布入口。
当前标准打包：DEV_RUNNER=/path/to/existing/koma-source-dev bash build.sh --source bh3（或省略 DEV_RUNNER 构建标准运行器）。必需图标和已知元数据由这个根入口验证。历史/当前官方图标身份见 ASSET_SOURCES.md。
这个集成保留规范当前 SDK 和标准编译标志；不声称与旧所有者直接 cargo 构建的字节相同 WASM。保留宿主等价和 Native 验收是单独的有界证据。这个集成不执行新 HTTP/图像/下载或设备工作。


## 237 现装 0.1.2 具名首两页与返回（2026-10-21）

沿现装 0.1.2 普通 Browse 进入崩坏3 IP站，实际目录 23 本；《逃离长空篇》详情 7/7 章。现有「开始阅读」进入 `com.bh3.koma:bh3:1001` / `bh3ch:1001:2` 第一话「崩坏、爆发」，实际亲看完整 1/27 扉页，再普通右侧翻页亲看 2/27 中文黑白正文（「香港中环」「好美的夜景」）。没有打开其他章节或更改阅读模式。

自然 Back 后同一详情仍为 7/7 章，按钮显示「继续阅读」；不能据此声称详情按钮已显示章页文案。真实 reader 落盘为 pageId=`bh3page:1001:2:1`、pageIndex=1、totalPages=27、completed=false。离开阅读器前的一次导出尚无新持久记录，Back 后的导出才证实这次自然进度，不以离开前空结果冒丢失或已保存。

只接受当前包上述具名章节首两页及自然返回/进度保存；未核图片缓存来源，不声称无缓存在线首取、全部 27 页、其他章节、下载、cold 或整源通过。无登录、匿名建号、收藏、重试、持久状态注入或源/App/Kit 代码变化。

原件在 Koma/.hvigor/outputs/user-flow-audit/device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership/：bh3012-public-list237-001、bh3012-detail237-001、bh3012-first237-001、bh3012-second237-001、bh3012-natural-return237-001、bh3012-session-restored237-001。恢复回执记录原 Browse 末端及 AWAKE/OverrideTimeout=10000ms；52 项 registry 全字节相同，原 reader 43 条 progress/17 条 sourceHistory/54 条 chapters 各对象逐项保持，只自然新增本书 progress 与 chapter 各一条，未删除或覆盖原 ComicABC 进度。租约 119ec760 已释放，全部设备回执关闭哈希。
