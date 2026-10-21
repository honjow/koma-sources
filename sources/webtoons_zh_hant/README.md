# WEBTOON (Traditional Chinese) 0.1.1

Independent source `com.webtoons.zh-hant.koma`, content language `zh-Hant`.
Canonical name remains WEBTOON (Traditional Chinese); localized display names are optional metadata.
English `com.webtoons.koma` and Simplified-Chinese Dongman are separate sources; no aliases, migration or shared-history claim.

Current Keiyoushi protobuf identity is 2959982438613576472, extension all.webtoons 1.6.2,
fixed primary revision 10f06eb833af5990bc5470c84c5e93edc159ad4e (Apache-2.0).
Upstream labels the extension SAFE: packaged nsfw=false/sourceInfo safe describes that source-wide label,
not a classification or acceptance of every work. Actual usage is unknown.

Public contract: four finite popular ranking buckets trending/popular/originals/canvas, desktop detail,
mobile episode JSON and ordinary viewer `#_imageList` images. Original href/title/query URLs are preserved.
Manga identity includes language, webtoon/canvas type, actual title_no and actual list path.
Chapter identity retains the exact viewerLink (including encoded Chinese path and episode_no);
page identity includes the actual title, episode and page index. Explicit Chinese title numbers only;
unknown chapter numbers stay null. Primary catalogue order is explicitly reversed, with index as order only.
Complete catalogue requires nextCursor zero/null and at most 2048 records; no truncated-success fallback.
All-types search is limited to a terminal first page; an advertised next page is explicitly unsupported,
never returned with a fake terminal cursor. Search has not had live acceptance.

No latest/day scheduling, filters/settings, MotionToon, synthetic author notes, query quality mutation,
login/paid/account access, ageGatePass/needGDPR cookies, automatic retry, challenge/mirror behavior.
Requests use the existing SDK Firefox121 and fixed public Referer/Origin; mobile catalogue has no Origin.
Image descriptors serialize only literal ASCII space as %20 for transport, preserving all other UTF8 and escapes.
Stored cover/page URI, IDs and public desktop headers remain unchanged, with no cookies.
197 observed five Chinese-filename covers cached but literal post sum.png failed with URL-format code2300003.
0.1.1 transport serialization still awaits native acceptance; frozen0.1.0 evidence is preserved.
HTML and output limits, UTF8/JSON/URL checks reject malformed, unknown, oversized or gated structures.
Logging contains only fixed stage/status/error values, never URLs/titles/queries/body/headers/account values.

Local source-owned packaging (public build/validator stay bound and unchanged):
`python3 sources/webtoons_zh_hant/package_candidate.py --runner /absolute/path/to/local-offline-host`
The wrapper uses paired Rustup stable compiler/cargo in offline mode, reads actual WASM info, validates metadata,
and creates a deterministic two-file .koma plus selected-only index with the exact package SHA.
No icon: the retained accepted English WEBTOON archive contains no icon; none is fetched or invented.

Retained live evidence: one trending GET (30 titles), title5486 detail, its JSON catalogue (154 episodes),
and the earliest ordinary 第1話 新手教學(1) viewer HTML (128 original image addresses).
The first image is the genuine site warning image, retained in place. No image was requested.
These bodies support offline production WASM replay only, not native/image/reading/download acceptance.
197 ordinary acceptance belongs to the root agent after exact source/package review.

## 0.1.3 详情来源网页

get_manga的detail仅新增kind=source链接，复用同次doc(manga_url(id))的原manga_url，保留真实语言path及title_no，未从API或App复合ID猜造。d0422d5按原件恢复时详情没有网页字段，未发现有意排除网页链接的记录；原理由未知，本次用户网页入口授权只允许此字段、交付版本及文档。runtime012→013，Cargo/lock旧011→013；旧012图标后继与metadata均保持。

原5486《變成蠻族！遊戲生存戰》作品页https://www.webtoons.com/zh-hant/fantasy/surviving-the-game-as-a-barbarian/list?title_no=5486的单次200/47936B原体，其canonical和og:url均精确等同该地址。只读取已有保全包remaining-materials原件到Koma .hvigor/outputs/user-flow-audit/webtoon-hant013-detail-fixture.html及同名前缀detail-fixture-original-{attempt,receipt}.json；未发新请求，未遇或绕挑战。原Native012是197《武士萬里行》255章、第2季65話P2/106及cold；它与5486原Host合同分开保留，不由此新增013或5486 Native/阅读接受。

单源offline/locked release构建及真实WASM info013通过；当前源码重编译runtime012与保全原正式012两种实际WASM均用同一5486原HTML和013比较，除新增links外全部manga字段、唯一HTTP请求深相等。禁网惰性Chromium DOM适配只证明该离线输出，不冒生产Host/Native等价；原件沿上述Koma产物分区`webtoon-hant012-{link-baseline,formal-original}-offline.json`与`webtoon-hant013-link{,-formal-compare}-offline.json`。正式`dist/webtoons-zh-hant-0.1.3.koma`29656B三成员，完整保留metadata.json名称；manifest继承012，仅version变更并按当前build.sh合同补入既有icon.png字段（原012含图标成员但未声明该字段）。6252B icon.png与保全原012成员逐字节相等；前文“无icon/两成员”仅是旧011阶段，013不删除012已交付原图标。未改请求、身份、章节/图片/传输合同、App或设备，013普通具名观察见下。

237 VDE-AL00展开竖屏1320×2120，复用现装Debug16289681B/23:30:52，经系统picker012→013普通更新明确成功，服务54包及最终Browse54已启用保持。普通热门一次返回30本，当前首屏未见5486，随后只在本源既有搜索框输入一次准确标题“變成蠻族！遊戲生存戰”，沿真实结果进入5486详情，现155/155章、开始阅读。两网页菜单可见；仅打开WebView，加载完成后亲看准确作品页标题、蓝色banner、作者、简介和章列表，无登录/挑战。自然Back详情仍155/155章及开始阅读，再回搜索结果/Browse；未打开正文、外部浏览器、下载或改变该作读态。旧197《武士萬里行》正文/cold不混入本次网页接受；未导出全库或据此宣称全部阅读位置逐项校验。仅本人传包已清理并读回不存在，最终10000ms/AWAKE并释放租约。

原图与回执沿Koma `.hvigor/outputs/user-flow-audit/device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership/`：`webtoon-hant013-import-list237-001/{import-success,services54}.png`，`webtoon-hant013-source237-002/{browse-unoccluded,source-list-terminal}.png`，`webtoon-hant013-webview237-004/{search-result-unoccluded,detail-before,both-web-actions,webview-workpage,detail-returned,browse-restored54}.png`，收尾`webtoon-hant013-webview-postflight237-004/run-metadata.json`。首次Browse点击落在导航遮挡区而切书架、随后搜索键盘联想导致重复标题断言均已依实际界面纠正，未重导/重搜，原失败产物保留，不作产品故障。
