# GoDa漫画

Stable source `org.goda.koma`, runtime version 0.1.2, API 0.2, Chinese.
Independent from installed Baozi identity; catalogue overlap unknown; no aliases or user-record migration.

The default request backend remains baozimh.org; the separately declared website homepage remains godamh.com. Explicit mirror choices are described below. Fixed Keiyoushi 1319949ca9bc456908196e3ba2d104e94182c5d6 leaf GoDaManhua, parent GoDa and Dto public decoder, Apache-2.0. Upstream SAFE=1; individual content and actual usage unknown. The source excludes advertisements, downloads and external English links.

HTML popular/latest/search and details, full public chapter JSON API, static marker/reorder/alphabet/Base64URL image-address protocol; no JS execution, account/cookie/challenge handling, automatic mirror probing or image-byte decryption. Fixed SDK Firefox121 and published Referer/Origin. Chapters reverse the original complete API array, retain every original title/slug/ID, and never use API order or ordinal as a chapter number. Host indices describe returned sequence only. Pages retain original array order; API order is not a zero-based index.

Strict complete UTF8/JSON/body and output capacity: 1MiB body and 1MiB payload, 8192 chapters, 2048 HTML/page selections, bounded individual text/attributes. Overflow/missing/invalid data errors, never partial-success truncation. Actual final envelope must fit native 1MiB runtime limit. Date fields are omitted rather than inferred.

Current production entry: `bash build.sh --source goda` with current canonical SDK/formal CLI and required icon. The historical package_candidate.py remains only in the old owner, not imported into canonical. Original crate identity/version remains koma_goda_source 0.1.0, while sourceInfo/package runtime is 0.1.1; this intentional inherited distinction is not normalized. Minimum App version remains 0.1.0. No distribution or device action.

Evidence in ignored .hermes-artifacts/goda-20261011. Retained public responses and production-WASM replay are local contract evidence, not native read/download/cold acceptance. Root owns 197 ordinary user-path acceptance.


Latest accepted successor is 0.1.1 original-G icon correction. User-supplied godamanga.com redirected to m.g-mh.org and explicitly declared its Logo.png; exact original PNG is retained, see ASSET_SOURCES.md. This does not migrate the old request backend or alias source IDs. Historical 0.1.0 named 武炼巅峰/1扫地小厮17-page read/download/cold and 0.1.1 icon update/cold are separate finite records; old backend-domain and whole-source boundaries remain. New current-SDK standard WASM has not been Native accepted. App row30vp/radius7/clipping are unchanged.


0.1.2 isolated candidate: retain the identity-checked getinfo API's complete root-relative image paths instead of requiring the first retained sample's /scomic/<canonical-slug> storage layout. A newly retained, uniquely matched 武炼巅峰/夺酒 response contains 70 /hp/wuliandianfeng/3864/... paths. API mid/id/slug validation remains unchanged; URLs still use only the existing fixed https://c-nd2-1.6wm.top origin, and valid_path still rejects network-path references, dot segments, encoded slash/backslash/control characters, non-path query/fragment characters and oversized paths. Array order and ordinal page identities are unchanged; no image bytes or new host are fetched. Old 17-page body is separately preserved. This candidate's page plans and offline results do not prove current image HTTP availability or Native reading; the original 0.1.1 Native failure remains. Original icon bytes and App clipping are unchanged. Cargo crate remains the inherited 0.1.0, runtime/package alone becomes 0.1.2.


## 237 現裝 0.1.2「奪酒」第2頁有限結果（2026-10-21）

接續的是原011在《武炼巅峰／夺酒》HTTP200後invalid image URL的失敗，非舊「掃地小廝」17頁。原canonical提交f0487ab已修復身份核對後的根相對頁路徑；本輪未再改解析、版本或圖標。現012生產WASM對精確 `godach:wuliandianfeng-pikapi/12-6059776-3864#12/1748368` 單次Host get_pages，實際原API HTTP200並成功返回70頁，首URI仍既有固定CDN的 `/hp/wuliandianfeng/3864/` 路徑。原stdout/stderr在Koma `.hvigor/outputs/user-flow-audit/goda012-current-duojiu-pages-host.{json,stderr.log}`；沒有重試、取新host或圖片實體，Host頁計畫不冒Native正文。

237普通Browse已顯示GoDa0.1.2，因此沒有重新導入。熱門18本→武炼巅峰詳情3875/3875章；進入前已存在「繼續閱讀・夺酒・第2/70頁」，不能把它冒成本輪首次閱讀或過去已親看正文。沿原繼續閱讀後，實際親看第2/70頁完整彩色敘事畫面（人物眼部與飛船構圖），21:31:05原生日誌有ReaderImage display_ready/display_complete page=2。該頁缺足夠可讀對白，不據此確認正文語言；未核在線首取/快取來源、全70頁、下載、cold或其他章節。原011失敗和舊17頁結果均保留，各自版本與驗收邊界不變。

自然Back後仍為原夺酒第2/70頁。臨時升序已透過原「重置」恢復全部/預設話數降序，且重置標記消失；Browse末端恢復，10000ms/AWAKE讀回後釋放本人b5fcf3cb租約。本輪未改53項源啟用、導入包或匯出私有狀態，未將GUI首末觀察冒作全量持久內容逐項相同。

原件在Koma `.hvigor/outputs/user-flow-audit/device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership/goda012-entry237-001/`：`goda-current-row.png`、`goda-detail-ready.png`、`duojiu-existing-page2-current.png`、`natural-return-original-detail.png`、`final-browse-restored.png`及entry/chapter/continue/return-actions.json。Reader原日誌在旁列 `goda012-continue-log-restored237-001/run-metadata.json`，最終回復在 `goda012-session-restored237-001/run-metadata.json`；全部本輪artifact雜湊關閉。

2026-10-26：用户报告普通网页误开包子。0.1.3将独立homepage元数据纠正为用户指定、实际首页自称GoDa漫畫的https://godamh.com/；只修品牌主页声明，原baozimh.org请求后端、章节/图片ID及权限保持。旧metadata把后端当主页的ae83e07被本修正取代，不能因后端仍保留而复活错误网页入口。

## 0.1.4 显式镜像设置

固定参考 949dea252eb8e867b7b40834a26e13088764ff55 声明六个首选镜像。`preferredMirror` 默认 `https://baozimh.org`，只有用户显式选择才使用另五项；未知值回退原默认，绝不自动探测或切换。设置影响目录、搜索、详情的站点域和 API/图片请求 Referer、Origin；共享 API 与原图地址仍由原实现及身份校验决定。`metadata.json` 的官网入口始终是 `https://godamh.com`，不随此选择重写。原漫画、章节和页面 ID 不改，也不自动迁移书库。

六域真实首页与具名《有鬼拍不到》详情均能读取、作品ID及首章ID一致；生产WASM分别验证目录、搜索、详情、同一首章页表。实际一次实时API更新使第一域章节116条、后五域117条，116个已有章节ID均相同，不推断其他作品或站点全量一致。备用域godamh.com完整图片请求头实取首图200；此前缺UA的直接请求两种Origin均403，不把错误当图片。默认图像请求与0.1.3相同。真实237设置持久、图像渲染和跨镜像旧库仍需独立验证。
