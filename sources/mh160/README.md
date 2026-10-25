# 漫画160 source

Public contract reference: Keiyoushi extensions-source `src/zh/mh160/.../Mh160.kt`, revision `10f06eb833af5990bc5470c84c5e93edc159ad4e`, Apache-2.0: https://github.com/keiyoushi/extensions-source/blob/10f06eb833af5990bc5470c84c5e93edc159ad4e/LICENSE . Preserve upstream attribution. Adapter code uses Koma source SDK and source helper patterns.

Upstream rating SAFE; actual source contents are unverified, so Koma source/content rating is unknown. Actual usage rate unknown. No account, arbitrary JavaScript evaluation or anonymous registration. Only explicit quoted reader literals; unsupported/missing/ambiguous/overflowed inputs fail. Declared capabilities describe implemented public contracts and do not imply host or native acceptance.

Canonical source id `com.mh160.koma`; manga identity `mh160:<original slug>`, chapter identity `mh160ch:<original public path>`, page identity `page:mh160:<original chapter path>:<1-based index>`. Upstream catalogue order is preserved, chapterNumber remains null.

## 0.1.5 原源序与首次未读

按本轮用户采用的Mihon规则，包显式声明`chapterOrder: newest_first`。固定Keiyoushi10f06eb的`Mh160.parseChapters`与本源`chapters`都按相同DOM选择直接输出，遵循Mihon最新在前接口；声明不改变任何目录项或请求。237现存《风起苍岚》782章原快照首为第2季352，末为第1话苍岚之巅1/43350001，且没有该书进度。整表含番外、请假条、标题漏字和第8话异常插入，不能宣称话数严格单调，不能据此从标题猜数、去重或整理目录。

宿主对声明源从原序的最早端找未读，已有进度和显式选章优先；章号继续null、原路径/顺序/图标/网页授权不变。固定参考代码、完整旧快照分析与生产选择器结果保存在Koma `user-flow-audit/mh160-chapter-contract-review-256`。本次不重请旧403章节/详情、跳转失败页或首图，不把首次目标修正写成正文恢复。标准22644B三成员包已237普通文件选择器014→015更新；80源同序，仅本源7个交付/版本/方向字段变化，旧79源及书库、Reader、网页会话元数据完整保持。生产选择器在实际782章旧快照和本包声明下，首次为43350001第1话，标读该话的受控选择分支为73620002第2话，原数组不变；这不是设备上新开正文。259观测主进程40851→49736，新进程书架/继续阅读真实显示，260全部四域JSON同258；没有观察空PID阶段，不写成空进程证明。自传包已ABSENT。范围提交、精确CI及公开发布仍分别记录于Koma CURRENT_STATUS。

## 0.1.4 错误原因与网页会话

请求失败现在保留 Host 返回的 HTTP 状态；只有 Host 明确标记网页挑战时才报告需要网页验证，普通拒绝、找不到页面和限流仍分别显示原原因。包声明的网页会话只限源本来使用的 `www.mh160mh.com`、`m.mh160mh.com` 两个 HTTPS origin，仍须由 App 按既有流程取得用户同意；不改 UA、网址、作品/章节身份或排序。

生产 WASM 编译及 info 0.1.4 已通过；本机真实 WASM 的失败封包检查覆盖明确挑战、普通403、404、429、500与伪似信号，不访问网站。这不代表挑战已成功通过，也没有修复已观察的跨 origin 网页白屏或宣称正文可读；远端发布及普通更新后的实际表现仍待验证。

## 0.1.3 作品网页入口

列表和详情现在返回本源已有的公开作品地址，App可据此显示现有“在WebView中打开”和“在浏览器中打开”动作。地址与详情请求的`/kanmanhua/<slug>/`一致；作品ID、章节顺序、未提供的话数及图标保持。跨季目录与图片失败仍在调查，此更新不代表整源已可正常阅读。

## Current native boundary (197, 2026-10-10)

Exact 0.1.0 package SHA `77ce60bd21d05227fe02e28d2627e5db2d20e4670b385255c89179cce4664619` and WASM SHA `b8da3dd365ce0be0be902fa609bc49be2eb9fc25bdd290f37ae384ce81f2bd8d` were read back after ordinary subscription installation. Native popular displayed 50 actual Chinese titles; covers were placeholders. Chinese keyword search was rejected with HTTP403. Actual `mh160:12094` 风起苍岚 detail and full 782-chapter catalogue returned200. Chapter `mh160ch:/kanmanhua/12094/43350001.html` 第1话 苍岚之巅1 returned200 and parsed10 pages, but Reader P1 failed; the historical first-image-window record noted404. Its raw response/request linkage is insufficient here to establish the failure cause, and it does not identify the cause of later cover placeholders or the unbound zero-chapter stage. Literal PID742697 and all10 image URL mappings match the fixed primary contract. No image Retry, UA/domain variation, download, full readability or all-site acceptance. Server-side failure causes remain unknown.

No-clear cold PID40136→42019 preserved all four JSON states byte for byte. Original24 comic objects and15 source objects were retained; only this own source/title were added. Reader/Queue bytes matched the pre-phase baseline. Only the owned `/src013/` repository was removed; original three repositories/selected URL were restored byte exactly, power override10000ms read back, owned delivery stopped and lease released. App/Kit and borrowed WIP were kept. Remaining search/cover/image branches stay OPEN until their specific public contract/service conditions change.


## Current source-only recovery boundary

This recovery preserves the original business, identity and metadata with the current canonical SDK. It does not register standard packaging or add an icon. Historical artwork/API/Native results above retain their named version and path only; no new request, compile, package or device acceptance is claimed by this proposal. Original candidate wrappers and source/asset evidence remain preserved outside this production closure.

## Current icon provenance baseline

The current official desktop header directly declares the unmodified280×95 PNG now retained as icon.png; ASSET_SOURCES.md records its exact parent and URL. This corrects the earlier source-only omission without changing the0.1.0 provider/version or standard registration. The official mobile mark is also horizontal and the square footer image is a QR code, so neither is misrepresented as a superior standalone icon. Small-slot suitability and current ordinary device delivery remain OPEN; this is not source/content acceptance.


## 2026-10-21 原首图失败章节的当前父链条件

沿上方197旧首图失败记录，只对当时曾200的准确公开章节 `https://www.mh160mh.com/kanmanhua/12094/43350001.html` 作一次默认匿名Python urllib.request.urlopen GET（timeout20），实际返回 `HTTPError 403: Forbidden`。没有取得新的章节HTML、reader脚本声明或原图URI，不请求图片，不重试、改UA/客户端/域或处理挑战。本次只限制这条准确章节请求，不判定其他章节、独立首页或整站；旧10页计划/首图404绑定不足及当前服务端拒绝原因仍未知，没有依据修改图片域映射或解析。

现有目录path身份、顺序和旧ID保持，本次没有目录、版本、源业务、App或设备改动。Root本轮交接指出同path不同标题不得盲去重；相关原始调查由Root维护，本次没有独立重核，不把该交接或此403冒成目录因果证据。

## 0.1.1 完整主字标图标

Root亲看同站官方280×95原件及小槽浅深对照后接受原像素完整红色“漫画160”主字标：只去下方辅助域名及透明余边，190×51、四周8px透明安全边；不截单字、不重排/重绘/放大或换字体。原图保留official-header-logo-original.png，候选为icon.png。Cargo/Info/lock010→011仅资源版本，源身份/请求/章节与App30vp/R7/排序不动。可辨性改善但横长细笔画仍有限，不称全质量通过。旧403/正文边界保持。2026-10-22设备237普通本地010→011更新完成，管理/Browse完整原图均显示整行红主字标及v011，管理仍启用；58项sourceId/enabled逐项同序。原Browse喵趣组、原10000ms/AWAKE恢复，只清本人传包并核不存在，租约释放。证据：Koma `.hvigor/outputs/user-flow-audit/device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership/mh160011-icon-awake237-001/{manager-mh160011.png,browse-mh160011.png,browse-original-group.png}`，registry/清理/电源回执在同分区`mh160011-icon-restored237-001`（completed，哈希关闭）。仅接受本次完整主标呈现的有限改善，横长细笔画仍OPEN，不冒正文或全源质量通过。

## 0.1.2 user-approved icon derivative

The user explicitly approved this resource treatment from source-icons-four-proposals.png on2026-10-22. Uses the approved114×96RGBA original red glyphs arranged as漫画 above160; it is a two-line derivative, not the official original horizontal layout. The complete280×95original remains official-header-logo-original.png without overwrite; the former190×51primary mark is preserved as official-primary-wordmark-original.png. This explicit selection supersedes the previous no-rearrangement hold; no font replacement or glyph redraw is introduced. No additional background plate, shadow or external mask is added. App30vp/R7 and source order remain unchanged. Only source resource delivery versions are aligned to0.1.2; provider requests, identities, chapters/pages, localized metadata and other business behavior are unchanged. Offline/locked single-source build and actual WASM info0.1.2 passed; the21301B standard package has exactly manifest.json/source.wasm/icon.png, complete registered rating/minAppVersion/localized metadata and the approved candidate bytes. Icon validation and diff checks passed without hashes; dist/index is unchanged. Root completed the ordinary237 picker 0.1.1→0.1.2 update and reviewed the complete light-theme manager/Browse images: the approved identity is complete and clear. Evidence: Koma `.hvigor/outputs/user-flow-audit/device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership/three-icons-package237-001/{manager-mh160012,browse-mh160012}.png`. This accepts only these two light-theme icon consumers; it does not establish dark-theme rendering or source content. Final registry readback: 59→60 entries, only H-Comic newly added; the original relative order and all unrelated entries remain unchanged. Manhuashe/MH160 changes are limited to their expected package references and versions. Original system theme/Browse group and10000ms/AWAKE/window208 were restored, only the three uploaded installer files were removed and confirmed absent, and the lease was released. Receipts: `three-icons-final-restored237-001` and `three-icons-package237-001/browse-original-restored.png`. This does not establish source-content or reading acceptance.
