# 喵趣漫画 — independent Koma source 0.1.6

Stable source ID: `com.miaoqu.koma`; Chinese canonical name: 喵趣漫画. The user explicitly chose 喵趣 to match the retained icon and Mihon source name; website title variants do not override that choice. Localized labels are optional presentation metadata. Actual usage: unknown. Upstream SAFE (protobuf 1 and Gradle SAFE), individual works unknown. This source shares the SixMH catalogue but has independent host and slug identities; no aliases, migration or merging.

Public contract follows Keiyoushi revision `1319949ca9bc456908196e3ba2d104e94182c5d6`. HTML popular/latest/search, mobile detail/catalogue, canonical chapter DATA literal. Fixed SDK Firefox121 UA-only; no Cookie, Referer or Origin. Only 2xx accepted; upstream HTTP500 exception is intentionally unimplemented without current evidence. No auth, JS execution, image-byte decryption, mirrors, challenge interception, retry or filter background fetch.

`miaoqu:<actual slug>`; `miaoquch:<original chapter path>`; `miaoqupage:<original chapter path>:<zero-based index>`. Original catalogue order reverses per primary; unknown chapter number stays null, ordinal is not a chapter number. Duplicate titles or explicit numbers are not deduplicated. Payload, complete UTF8 body and output are bounded; overflow or malformed structure is failure.

Retained actual HTTP200 bodies prove 35 popular cards / next2, 1204 catalogue links and 12 image URLs for 斗破苍穹 第131回 卑鄙的联手（上）. Production WASM replay checks these named paths only. Latest/search are implemented from primary but live unverified. Root later accepted only a finite named native chapter; search and covers remain OPEN. This current-SDK source recovery adds no Native/download/cold acceptance.

Current recovery preserves the original0.1.0 business, Apache license and localized metadata only. The old0444d3a4 migration icon is excluded, and no build registry entry is added without a trusted usable identity asset. The original borrowed-compatibility candidate wrapper and its package evidence remain historical materials; they are not the current standard build entry or approval to bypass the required icon. No package, website request or deployment is produced by this recovery.

## 0.1.1 webpage-link candidate

Detail now declares one `kind=source` link using the same MOBILE + detail path as its existing GET. For the existing `miaoqu:doupocangkong` (斗破苍穹) entity, one ordinary anonymous GET to `https://m.miaoqumh.org/doupocangkong` returned HTTP200/113257B and the named work HTML, without redirect. No canonical/og:url was present; the link follows this successful detail request, not a guessed desktop URL. Original identity, requests, catalogue, reading and metadata remain unchanged.

Actual 0.1.0 and 0.1.1 WASM replayed that same saved HTML with network blocked: both succeeded, every non-links manga field and the sole HTTP request were deeply equal. Offline locked release build passed. The page's sole declared identity candidate `/template/wap/tiantangmanhua/img/favicon.ico` returned HTTP404 on one GET; no asset was obtained. Missing trusted icon still blocks standard packaging; no package or new Native acceptance is claimed. The separate prior www homepage403 remains stopped. Evidence: `Koma/.hvigor/outputs/user-flow-audit/miaoqu011-{detail-once.html,detail-once.json,baseline-offline.json,candidate-offline.json,link-compare-offline.json,declared-favicon.json}`. Existing NOTICE WIP is retained separately.

## 2026-10-22 distribution asset restoration

The current `icon.png` restores the unchanged original from the installed device237 0.1.0 package, received as `miaoqu010-current-asset237-001/installed-icon.png`. This restores the required package asset for remote installation/update; it does not claim a new official high-resolution resource, resolved icon quality, or new provider/content acceptance. Existing plate/shadow/small-slot limitations remain OPEN. No image processing, source identity, request, chapter or App presentation changes. Prior business-only/missing-file statements above describe the earlier recovery checkpoint; the original source documents and notices are retained.

## 0.1.2 icon board/shadow cleanup

This version removes the external white board/grey shadow from the retained fixed-upstream192×192 Miaoqu reference. Chroma separation restores alpha around its original104×104 glyph without scaling or tracing; the foreground uses the original body's modal orange RGB240/160/110. The120×120 PNG retains8px transparent safety margins. This is background cleanup of the existing identity, not a new official resource or high-resolution redraw; no new backing, shadow or mask is added. The separate generated image with yellow fringe/hole specks was rejected and is not used. App radius/slot/order, source identity/requests/chapters and all provider behaviour remain unchanged; only source/Cargo/lock asset-delivery version011→012. Website403/404 branches remain stopped; no content acceptance is added. Device237 ordinary local update0.1.0→0.1.2 succeeded; both management and Browse display the orange mark without the former board/grey edge. All56 sourceId/enabled pairs retain their exact order, and the original Browse position/10000ms override were restored, read back AWAKE, with the owned transfer removed and lease released. Root reviewed both full-resolution images and the restoration receipt; the board/shadow cleanup is accepted for these two consumers. Original full images and receipts: Koma `.hvigor/outputs/user-flow-audit/device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership/miaoqu012-icon-import237-001/{manager-current,browse-current}.png` and `miaoqu012-icon-restored237-001`.


## Current displayed name

The adopted simplified/traditional display names are 喵趣漫画 / 喵趣漫畫, following the user's explicit 2026-11-05 choice and the fixed Mihon source name. The saved desktop and mobile HTML titles use 妙趣漫画; this known title variant does not change the chosen display name. Preserve the stable source ID, domains, icon, manga/chapter IDs and user history; a later naming change requires a new explicit user decision.

The desktop homepage's declared favicon and logo URLs both returned HTTP 404 during this check. The existing icon is the previously accepted cleaned form of the fixed upstream Miaoqu reference; the original downloaded reference, retained original and installed 0.1.0 asset have matching SHA256. This establishes provenance of the retained icon, not its identity with a currently available official site asset. It is not replaced or claimed as verified-current.

## 0.1.4 explicit removed-work response

The mobile route for `miaoqu:moutianchengweigongzhu` returns a final HTTP200 at `/err/comic` with the exact public notice “很遗憾，该漫画不存在或章节已被删除。” and a category redirect script. The adopted MCCMSWeb reference already rejects this final error route before parsing. Since the current host response does not expose the final redirect URL, detail and catalogue recognize only that complete retained error-body template and return the existing `not_found` category with the provider meaning. This does not assert whether the work itself or only its chapters were removed, invent an empty catalogue, change IDs, or broaden content scope. Unknown or partial notices and unexplained missing selectors retain their original parse errors.

The separate `miaoqu:jiesuomoshide99genuzhu` catalogue receipt is HTTP520 `server_error`, not a parse error; it was not retried or accepted as content. Normal detail/catalogue payloads, request headers, catalogue order and chapter/page identities remain unchanged. Device237 ordinary update confirmed the adopted 喵趣 name; opening the actual popular card 某天成为公主 showed the existing explicit unavailable-content guidance instead of a source runtime error. The prior source/data and power state were restored. This named check does not establish all removed works, normal chapter reading, downloads or cold reads.

## 搜索筛选

支持筛选作用域、动态筛选及空词筛选的宿主可按参考以空词搜索筛选连载状态、排序和标签；带词搜索不消费这些筛选。分类标签依参考从移动站 `/category/` 读取，仅采用可验证的标签链接；桌面分类页仅展示部分标签；缺失、重复或异常结构明确失败。旧宿主保留原浏览和有词搜索合同。官网单页结果的同一路径无页码下一页链接仅作为第一页末尾标记，其他未知分页结构仍失败。

章节选择使用独立有界容量8192，保留官网完整ID和次序；超过边界或输出缓冲不足时明确失败，不把截断目录冒充完成。其他选择器保持原容量。
