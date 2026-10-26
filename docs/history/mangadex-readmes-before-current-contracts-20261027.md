# MangaDex README 历史快照

仅供明确历史查证，不是当前合同，不从默认阅读链引用。

## mangadex

## 0.1.11：未知章节编号保持未知

`get_chapters` 不再把 API 的空编号、缺失编号或 `null` 填成 `"0"`，输出为 `chapterNumber: null`；真实 `"0"`、普通编号和小数编号保持。非字符串编号仍不作为有效编号，不猜测标题。章节 ID、标题、目录原序、请求参数和请求次数均保持；`publishedAt` / `updatedAt` 仍按原合同输出 `null`，本次不引入时间语义变更。

本次用原编译 WASM 与新编译 WASM 回放同一份明确标注的合成 API 输入（并非 Super Lovers 的真实 feed）：`"1"`、`"0"`、`null`、字段缺失、空串、空白串、`"1.5"`、数值型字段共八项；只允许未知编号变成 `null`，其余响应深比较一致，HTTP 请求内容及单次调用次数一致。生产入口 `bash build.sh --source mangadex` 生成 `mangadex-0.1.11.koma`。产物和回放明细：`/tmp/koma-mangadex-metadata-20261023/`。尚待 App 普通源更新、现有详情刷新及书架首次阅读验收，不能将本项称为实机首章已通过。

# MangaDex 0.1.10 current standard source

完整继承已审018设置/精确章节恢复、019分页/JSON结构/Home职责与0110完整external-only终态。源身份org.mangadex.koma、章节ch:身份、原图标/ASSET_SOURCES不变；不跟随MangaUP等外部正文。current canonical既有exact-chapter分支在此完整后继保留，读取直接data/attributes/relationships，不覆盖其它源WIP或SDK。

0110跨页cursor mdxc1携带known/hosted/external/other事实，仅完整遍历终页全external且无hosted/other才unsupported；真实total0正常空，mixed保留本站章。旧数字非首页历史未知不能冒断all-external。原页数据省流/原settings/filter与全部正文消费者保持原后继。

可信owner及原Root Host证据：/private/tmp/koma-sono-zero-reason-design-20261016/{candidate,delivery-binding.json,ROOT_REVIEW.json,production-replay}。真实原体只有Sono首100及LoveBullet21章；后页/终态/混合/错误是明确受控回放。0.1.10已在237旧App8b8151f1具名显示不支持章节（原m39/m41）；最新保存注册记录仍0110，不是本轮实时设备结论。较细220外链说明/全部语言未验。

生产入口 `bash build.sh --source mangadex` 使用当前canonical SDK/必需icon合同，新增manifest.icon元数据但保旧字段；新标准WASM与旧冻结WASM可能不同，不能据旧Native冒本轮新编已验。原crate-local Cargo.lock为历史非workspace构建输入保留；实际workspace Cargo.lock本crate同步0.1.10。本源根Cargo及build registry原已登记，不添加其它源。图标来自既有官方PWA声明，字节保留；App显示裁剪独立保留。无新网络/下载/部署/样式变更。

## mangadex-zh-hans

# MangaDex (Simplified Chinese)

Independent org.mangadex.zh-hans.koma0.1.0, API0.2. Language zh-Hans, public API zh. English source and IDs untouched. Usage unknown.

关注较多 is all-time followedCount descending, not primary30-day popularity or usage ranking. Listing/search explicitly require available Chinese chapters and safe/suggestive ratings. Search/title query is ordinary public GET. Details retain actual per-book ratings. Catalogue loops all100-item feed pages, checks UUID ownership/language/pagination/duplicates/total and fails wholly on missing data/capacity. External/unavailable/zero-page chapters are omitted; nullable chapter/volume numbers are not ordinal guesses.

Pages use original ordered chapter.data files. get_pages checks the complete owned zh feed before at-home; stable pageID carries actual chapterUUID/index. Each imageRequest makes a fresh normal at-homeGET, resolving the current indexed original URL; this replaces the primary5min source-clock/cache with one explicit descriptorGET per image. No account tokens/cookies/statistics POST/cache/retry; no App/SDK change. This increases public descriptor traffic; the named chapter download below is accepted, while general traffic/limit behavior remains unproven.

Allowed image HTTPS hosts: uploads.mangadex.org or descriptor-provided actual subdomain of mangadex.network. Full JSON/UTF8,1MiB response/input/output bounds; errors never silently return partial chapters. Official original icon retains its published deep-color background. The finite named native acceptance below does not close whole-source acceptance.


## Finite device acceptance — 2026-10-12

On device 197, ordinary subscription installation of 0.1.0 matched the frozen package, WASM, manifest and official icon hashes. The all-time followed listing displayed 20 actual entries with covers; the ordinary `Love Bullet` search displayed two entries with covers. The named book `cb1b1da5-9436-49f5-8a6b-a1689fb86d91` displayed its complete 21-item Chinese directory.

For chapter `a6daac4d-f4bd-4102-84f3-f8b78cf95fc6` (DTO title `爱上恋爱`), ordinary reading displayed original pages 1/33 and 2/33 with matching HUD and saved index 1. Ordinary download completed 33/33; all 33 downloaded PNG entities were decoded and their sizes, content hashes and manifest integrity matched, totalling 38,829,379 image bytes. After a process-death cold start without clearing data (PID 32723 → 44881), seven state files were byte-identical and ordinary Library Continue returned to the same page 2. Original Library/queue/registry objects and order, other Reader collections, three subscriptions and Reader preferences were preserved; only the temporary test index was removed through the GUI.

Authoritative receipt: `/Users/honjow/git/Koma-production-20261010/.hermes-artifacts/20261005-production/mangadex-zh-hans-20261012/native040-finite-acceptance.json`, SHA256 `fc9d9274a173369dde22ea11d99e369320eae61e789950cd5995e1b53de07267`. Installed package SHA256 `1ead24f19f3d77fb93228767da8177d2dbde99663cf7d40c0104bf7c3e3ce6f4`; WASM `b259046f6e2b1838ae92c4762a195858b8666dae5e141308806e46d2841abb7a`.

Usage remains unknown. Other listing/search pages, books and chapters remain OPEN. This does not claim all 33 pages were visually reviewed, an uncached transition, airplane-mode offline operation, a traditional-Chinese package, whole-source acceptance or completion of Koma's first production version.

## mangadex-zh-hant

# MangaDex (Traditional Chinese catalogue)

Independent `org.mangadex.zh-hant.koma` 0.1.0, API0.2, content language zh-Hant with provider API and metadata keys zh-hk. English/Hans IDs and packages remain untouched; no alias/migration. Usage unknown. This source filters MangaDex provider-designated traditional-Chinese chapters; the designation does not guarantee traditional characters in every image.

关注较多 means all-time followedCount, not30-day popularity or real usage ranking. Listing/search explicitly require available zh-hk chapters and safe/suggestive books. Per-book ratings remain actual. Detail title/altTitle/summary prefer zh-hk then the existing English/primary fallbacks. Full100-item feed ownership/language/pagination/duplicate/total checks and bounded strict UTF8/JSON preserve whole-operation failures; external/unavailable/zero-page chapters are omitted. Chapter/volume numbers retain null/fraction/zero rather than ordinal guesses.

Original ordered chapter.data filenames are validated, including duplicate rejection. get_pages checks the complete owned feed; stable chapterUUID/page-index identities resolve fresh public at-home descriptors per imageRequest. Covers bind uploads.mangadex.org/covers/sameUUID/safeFilename and return the exact original URL with normal headers without descriptor HTTP. No tokens/cookies/retry/statisticsPOST/source clock/cache or SDK/App changes. Allowed image hosts are official uploads or the actual descriptor mangadex.network subdomain. Official icon original deep-color background remains unchanged, no added frame/mask/shadow.

Read-only real contract at /private/tmp/koma-mangadex-zh-hant-readonly-20261012: list20/total706, exact named title search1, safe book55ace2fb-e157-4d76-9e72-67c6bd762a39 with complete29/29 owned public zh-hk chapters; Chapter1 16dca81b-88ab-4561-bf24-cb5f856350e1 has49 unique original files. Actual first image is Japanese novel cover; second is simplified-Chinese promotion; third is simplified-Chinese manga dialogue. Preserve all original bytes/order and accurately separate provider language metadata from observed writing system. These finite host images do not establish native or all49visual acceptance.

Finite native acceptance on device197 ALN-AL80 (1260×2720): ordinary independent subscription installation, actual package/WASM/manifest/icon hashes, followed20 and exact search1, named complete29/29 zh-hk chapter directory and cover, original P1 then Chinese P3 with HUD3/49. The selected Chapter1 completed49/49 via ordinary download manager. All49 original JPEG files (40,758,909 bytes) fully decoded with exact size/runtime content hashes and manifest integrity; the first3 matched the retained original body hashes. No-data-clear process death/cold PID5490→16068 preserved all7 state/manifest files byte-for-byte, followed by ordinary Library continue showing the same P3/49.

Only the owned temporary subscription was removed through the GUI and its delivery server stopped. Original44 Library books,51 queue entries,36 source records, other Reader collections and original3 subscription/Reader preferences were preserved. Source/new book/download/progress remain. App916 tracked files, original135 other source files, acceptedHans138 and shared16 protected files remained exact. No push/release.

Root receipt: /Users/honjow/git/Koma-production-20261010/.hermes-artifacts/20261005-production/mangadex-zh-hant-20261012/native041-finite-acceptance.json (SHA256 e7ba401065e961a4031257039bf2e2039a8de13360957e17b0452fef5afc1593). This is one named finite path, not whole-source or whole-page visual acceptance. General pagination/other books/rate limits, all49 visually observed pages, uncached entry motion and airplane-mode offline remain unverified. Existing list/detail/Reader long-title ellipsis was observed and is not claimed as a whole-page visual pass. No App UI changed in this source-only slice.
