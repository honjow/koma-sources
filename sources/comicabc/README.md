# 無限動漫 / ComicABC 0.1.9

当前生产提案完整继承已验 019 owner，标准入口 `bash build.sh --source comicabc`，使用当前仓库 SDK 和必需 icon 合同。lib/records/s2t 与可信 owner 全 bytes 保持，不取别的 prototype 或更改 parser；旧目录 159/159 与原日文 P2/29 具名有限验收、后图标更新/cold 只属旧 Native WASM dc474ff2。新 standard WASM 必须单独绑定/兼容回放/Root 审核，不冒继承旧 Native。

官网原 favicon32 与出处原 bytes 完整沿用；小图适用性仍按 Root 已验具名画面，App 显示圆角裁剪独立保留。本提案不加底板/阴影/遮罩、不加工图片、不联网/设备或发布。

019 仅省略没有上游证据的章节 language，不推断正文语言；其完整继承 018 句柄释放、017 记录页数读取、016 字段完整性与原 015 半话数、动态角色读取等已审业务。不补未知编码/不扩大身份范围，不重验冻结正常矩阵。

## 2026-10-21 ComicABC 0.1.10 / 237 具名接续

当前 237 注册项实际读回 `com.comicabc.koma` / `manifestVersion=0.1.10`。沿普通源内搜索「最弱的劍鬥士」的唯一结果进入原 30513，详情标题「被稱為最弱的劍鬥士用遊戲知識開無雙 -在女尊男卑的鬥技場中一路發跡」，目录 6/6 章。

原第 01 话已显示「已下载」；普通打开真实正文为 2/62，仅证明现有下载正文可读，本次没有触发新 get_pages，不冒旧 0.1.0 第 1 章失败被当前在线复验。原第 5 话普通打开真实首图 1/24，但本次仅 get_image_request，页列表已有缓存，不计新 get_pages 通过。

同目录自然未下载的第 4 话，04:58:15 真实日志为 `comicabc get_pages` / `pages manga=30513 chapter=4`，随后实际显示第 4 话首图及 1/27；通过的是此章当前在线页列表与首图，不以 HTTP200 替代图片可读，不外推其余 26 页、全源、下载或 cold。随后普通返回目录→原第 5 话→返回详情，继续阅读已实际恢复第 5 话第 1/24 页，再 Back 回原搜索页。未删下载/缓存或清数据；最终 AWAKE / OverrideTimeout10000ms 读回，释放原 237 租约。

原件均沿 Koma 既有 `.hvigor/outputs/user-flow-audit/device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership/`：`comicabc010-ch01reader30513237-003`（下载正文原图/日志）、`comicabc010-ch05online30513237-003`（缓存首图/日志）、`comicabc010-ch04online30513237-003`（在线 get_pages 日志/真实首图）、`comicabc010-progress-restored30513237-003`（恢复 5 话 1/24 原图）、`comicabc010-sourceversion237-003`（当前版本回执）、`comicabc010-session-restored237-003`（返回原搜索/超时恢复）。所有回执 `artifactHashesEnabled=false`。本次无业务、版本、图标或 App 样式改动，未构建、未重新打包。

2026-10-21 后继具名补验：237 现装仍为 0.1.10（由完整 registry 读回）。普通同作品 30513 第 4 话翻到 2/27，Root 亲看完整繁中黑白正文；随后原执行者普通回第 4 话第一页、改回原第 5 话 1/24 并自然返回详情，reader 读回 chapter:30513:5/pageIndex0/total24。该执行者随后 429 中断，Root 按原 checked 门禁接管，返回原 Browse 末端；完整 50 项 registry 保持，10000ms/AWAKE 恢复并释放 cdda07c3。原件 category-membership/comicabc010-{ch04page2-30513237-004,restored30513237-004,root-session-restored237-004}。仅补现装包具名第二页和原进度恢复，不称在线首取、全 27 页或新下载；未改源码、版本、资源，未直接编辑用户数据文件。

历史实现/验收记录（以下原 owner 全文保留；其中旧「当前/未部署」只按对应历史版本解释，不作本轮结论）：

# 無限動漫 / ComicABC 0.1.2

公开中文列表、搜索、详情和章节。阅读器页只用观察到的有界 47 字节字面量记录 schema、83 字节标签/填充尾、语义 ps/图像字段映射和公开数字身份。变量和页/服务器偏移可能变化；不执行 JavaScript。未知编码明确失败。不提供账号、购买、挑战或浏览器引擎。

简体中文查询只在首个搜索页为空时重试一次其繁体字形。本地化显示名与源身份和中文内容语言分离。归因见 NOTICE。

历史 0.1.0 包通过设备 197 的普通独立订阅导入。观察到 Native 简体名称、热门封面、简体标题搜索、Iruma 459/459 目录和具名第 1 章首图 1/44。普通第 1 章下载产生 44 个非零图像文件；无清除冷改变了 PID，保持四个持久化 JSON 文件字节相等，Library 继续显示相同首图 1/44。原对象/偏好保留，临时索引/电源恢复。精确包/模块和原始证据绑定在忽略的 197-native-acceptance-receipt.json。这只是具名有限路径；不隐含整站、全页视觉、断网、其他 native 语言、账号或未知 schema 验收。本地兼容/App/Kit WIP 保持单独；无推送或发布。

0.1.2 质量更正：保留的 30513 第 1 章原阅读器在其 47 字节记录（part46）中声明 chapter/code/page/server 偏移 0/2/42/44。0.1.0 和 0.1.1 都拒绝这个真实布局。只额外接受那个第四观察角色元组和偏移 2 处对应的 40 字节 code 字段。章节身份、步长、尾、服务器/页边界、code 校验和所有先前布局检查保持。不执行 JavaScript 且不重写 URL。

设备 237 在干净 App 0704d114 带原源 0.1.0 显示 30513 的 5/5 章，然后在其首次普通 StartRead 的页目录加载时失败。没有按重试。单次当前宿主阅读器 GET 是 200；它不证明 Native HTTP 状态。源 0.1.2 未在 237 安装或验收。先前 17224 零章节错误和 30513 记录布局错误是不同的。其他作品、章节图像和未知记录布局保持未验证。

0.1.3 质量更正：新 237 普通阅读 15059 首 1 话在页目录加载时失败，HTTP200 带显式源错误 `unsupported public literal record schema`。没有按重试。正常 App 路径为那个单一选择发出两个 get_pages 调用；两者都返回 HTTP200/21255B。随后对完全相同 URL 的一次匿名宿主 GET（带既有源 UA/Referer 且无重定向/重试）返回 200/37030B。其原脚本声明 chapter/code/page/server 偏移 44/2/42/0，part46 和 47 字节步长。冻结 0.1.2 用相同错误拒绝那个原体。只额外接受这个第五观察角色元组；字段大小、身份、尾、步长、页/服务器/part 边界、原请求和所有先前布局保持不变。无 JavaScript 执行或 URI 重写。宿主和 Native 响应字节未导出为相等，故那两个响应间的字节相等仍未知。更早 197 0.1.0 首 1/44 和下载/冷证据只对其原保留布局保持有效。这个 0.1.3 候选未在设备上安装或验收。

0.1.4 候选，2026-10-16：新 237 普通失败观察于 9154 第 1 章和 16264 第 1 章。旧五个位置元组拒绝两个真实角色排列。用相同有界 47 字节分区替换位置元组枚举：五个不同语义角色、固定 2/40/2/2/1 宽度、不重叠和完整覆盖，part 在 46。保留 ti 所有权、47 字节步长、83 字节尾和章节/服务器/页/part 边界。无 JavaScript 执行或新编码。七个保留真实响应回放：新两个返回 71/20 页；全部五个旧输出等于 0.1.3。十四个派生负记录拒绝。这只是宿主元数据证据；Native 0.1.4 阅读、图像、下载和冷验收保持 OPEN。原 0.1.3 失败和其旧具名 15059 验收保持单独事实。

后继原件明确取代上段的历史 OPEN：`comicabc-six-historical-parents-2-nested-full-tree-preserved.tar.gz` 内 `comicabc-six-historical-parents-2/tree/native-receipt.json` / `result.json` 记录 0.1.4 时 237 已有限接受 9154/ch1 的 P1/P2（71 页计划）与 16264/ch1 的中文分格 P1/橙色题图 P2（20 页计划）；后者《妖精的尾巴 百年任務》单章 20/20 下载及不清数据 cold 继续 P2 也已接受。原 20 张不等于逐张视觉或断网验收。归档现位于 Koma `.hermes-artifacts/recovery-20261019/`，本轮只读原回执，不新解包/计算摘要/请求或重验 16264。两具名旧失败按这些有限范围冻结；下面 0.1.10 的 9154 记录仅为当前图片消费者补证，不能称首次修复旧失败，也不补称新 Native 在线页计划通过。

## 0.1.5 明确半话章号

原 17224 目录可见标题明确「第 114.5 話」「第 149.5 話」，旧版从路由数字前缀输出 114/149。此版仅在原标题整体为 `第<同身份整数>.<小数>話` 或 `<同身份整数>.<小数>話` 时保留该原十进制字符串为 chapterNumber；不从 e 或任何字母后缀推算。原标题、所有 ID、原 DOM 顺序/sequenceIndex 及正文解析均保持。

仅复用冻结 17224 原 HTML 作 014/015 实际生产 C++ HtmlHost/WASM 回放，376 项只有上述两个 chapterNumber 变更，其余输出相同。无网络/设备，未 Native 验收；不称其它未核标题模板或全源通过。

0.1.6 仅章节字段完整性：onclick 与标题分别复用既有 HTTP_OUT/BODY，原 512/4096 仅作首次本地读取 slice，读满则在原 buffer 内增长；只有未读满确认完整，最终满已有 buffer 明确失败。保 8192 节点/8051 编码边界、ID/顺序/半章与其它操作。核完整返回信封 Native 1MiB，不把部分字段当完整成功。离线受控长字段和旧真实目录分开记，未部署/无新网络。

0.1.7 仅移除正文 record 页数额外 256 拒绝；官网原脚本按 ps 完整迭代，保零页/编码/角色/URI/请求，get_pages 完整返回信封继续核 Native 1MiB 边界。257 只由明确受控原字段变异验证，不冒实站 257 章或图片/Native 通过；原 376 条 record 解码一致。

0.1.8 仅目录句柄完整性：get_chapters 改按生产 SelectAll 真实 count 两阶段选择，句柄放既有 BODY 前缀、完整标题读取仅用剩余后缀，onclick 仍独立 HTTP_OUT。去章节 8192 人为节点拒绝，不增静态内存/SDK/上游请求；按第二次实际分配量回收。保其它操作原 Selection、8051 编码/去重/DOM 序/原完整 envelope 容量。大重复锚只受控验证，不冒真实 8192 目录或全源通过。

0.1.9 仅修章节语言声明：原 get_chapters 对全部章节固定写 language=zh，未读章节语言证据；237《終結的熾天使》第 159 话 P2 日文反例否定该断言。本版仅省略未知章节 language，不猜 ja/zh，不改 sourceInfo/书级语言、ID/标题/顺序/正文。复用原 17224 目录生产 WASM，376 项除移除 language 外全部相同；未新增网络或设备，错误 ZH 徽标消失待 Root 普通路径验证。

## 2026-10-17 必需身份资源

新的仅图标候选使用 ASSET_SOURCES.md 记录的真实本地资源。更早的可选 PNG/无图标包描述保持历史证据，不是当前生产资源规则。Manifest.icon 命名实际根文件；WASM、版本、稳定身份和解析器行为不变。旧冻结包保持单独。Native 图标显示未验收；App 圆角裁剪保持。


## 237 0.1.10 / 9154 第 1 话有限接续（2026-10-22）

只接上述 9154 第 1 章旧 record-schema 失败，不重跑 30513 或把旧 159 话日文反例当同章。当前实际 WASM SourceInfo 0.1.10 对原 `comicabc:chapter:9154:1` 一次 Host get_pages 成功返回 71 页；同实体一次 get_manga 给出《終結的熾天使》/鏡貴也,山本大和。CLI 未输出原 HTTP 状态或完整 HTML，不补称 HTTP200。未修改源码、版本、图标或封包。

237 现装仍 0.1.10；Manager 直达独立搜索准确题名一次→同作 159/159 章，原进度为 159 话 2/29。临时升序普通选择 1 话实际恢复既有 2/71，真实彩色题图含繁中标题及日文书名；普通前页到 1/71，亲看繁中三格叙事正文。自然 Back 详情显示 1 话 1/71。当前有界进程日志保留 get_image_request、ReaderImage display_ready/display_complete 及缓存命中，但没有保到该章新 get_pages 记录；不据缺日志断言没有调用，也不将 Host 71 页计划或实机图可读冒作新 Native 在线解析/首取通过。其他 69 页/全章语言/下载/cold 均未验。

经普通「重置」恢复全部/默认话数降序；仅为恢复原进度重开 159 话，未翻页，Back 详情及普通历史均为原 159 话 2/29，不计新增 159 话验收。最后 Browse 55、完整 registry 与本轮前相同，原 10000ms/AWAKE 读回并释放 dfc078d6；无传包、下载、私有状态写回或受拒请求重试。

原件：Koma `.hvigor/outputs/user-flow-audit/comicabc010-9154-{ch1-pages,detail}-host.{json,stderr.log}`；既有设备分区 `device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership/comicabc010-9154-body237-001/` 的 `chapter1-first.png`（实际 P2）、`chapter1-page1.png`、`chapter1-page1-chrome.png`、`chapter1-progress-return.png`、`original159-progress-restored.png`、`history-final.png` 及 `browse-final.png`；日志 `comicabc010-9154-current-log237-001/run-metadata.json`，恢复 `comicabc010-9154-restored237-001/run-metadata.json`。

## 站内阅读入口候选

2026-11-06 两部官网作品的原参考阅读域返回 HTTP 200 的《玩桌遊》网页，不含漫画数据。官网自身 `comicview.js` 的 CKVP 阅读偏好分支明确提供 `/view/{id}.html?ch={chapter}`；候选仅在原域返回已确认的该桌游页面时回退，其他 HTTP 错误、挑战和未知结构仍直接失败，原参考阅读页可解析时保持原路径。站内脚本按已观察的 47 字节记录及图片表达式静态校验作品、章节、服务器、页码角色，不执行 JavaScript。

真实站内页经 WASM 回放：《入間同學入魔了》第 1、2 话分别 44、30 页，《無良公會》第 1 话 37 页。两部第 1 话首末生成图按既有 Referer 下载均 HTTP 200 且为 JPEG；我亲审两张首图分别为入間封面与無良公會正文，未将其他全部图片称已核。受控构造的旧参考渲染形状，新旧 WASM 输出完全相同；未知外站页不回退，错作品及改变图片表达式均拒绝。Native 普通路径和正式发布仍待验收。
