# NHentai.xxx Chinese source 0.1.0

Anonymous canonical `https://nhentai.xxx` metadata source. Stable ID
`com.nhentaixxx.zh.koma`; language `zh`; minimum Koma 0.1.1.

Only Chinese popular listings, detail, a single whole-gallery chapter, ordered
page descriptors and image requests are provided. Search/latest/home/settings
are unsupported rather than fabricated. Listing cards require actual language
code `3`; detail requires the provider Chinese language tag and both work IDs.
The package is adult/nsfw (published warning 3); per-work rating and usage are
unknown. No manga image was fetched or rendered in implementation acceptance.

The full ordinary JSON `fl` map is parsed without JS. Pages must be unique,
consecutive and complete (1..count, at most 1000), with observed jpg/webp types
and positive declared dimensions. Detail and first-reader fields and `fl` map
must agree. The actual first reader `img#fimg[data-src]` establishes the
`i2/i4/i5.nhentaimg.com` asset contract; the inline SVG `src` is not a page.
The old parent-derived `iN.nhentai.xxx` host is rejected. The first image URL
was observed; other URLs are derived from the same gallery, not individually
observed or decoded. Declared dimensions do not prove decoded image dimensions.

Requests emit only canonical metadata URLs and observed normal anonymous headers.
No auth, challenge solving, JS evaluation or guessed fallback hosts. Redirect
handling belongs to the current host HTTP transport, not a new source option.
Pagination preserves Chinese/popular/canonical ownership and page progression;
page 2 has not been fetched. Unexpected hosts and changed contracts fail explicitly.

Image requests bind the supplied route, page ID, asset path and ordinal.
They do not independently authenticate a consistently forged descriptor and
route; cover IDs carry no independent asset-owner assertion. Only provider
descriptors already selected by the host are expected here.

Uses SDK buffer/helper macros and custom bounded exports. This is an explicit
exception to the guide's all-three-macro rule: the fixed SDK export macro forms
an unsafe slice before checking actual WASM memory bounds and omits localized
names. The existing consumer-compatible guards and metadata are preserved;
the shared SDK and other sources are unchanged. Not literal macro compliance.

Build one production module with `cargo build --release --offline --target
wasm32-unknown-unknown -p koma_nhentaixxx_zh_source`. Retained actual HTML replay
proves parsing and descriptors only; it does not prove live image availability
or native reading/download/cold acceptance. Historical ordinary installation/identity/icon/persistence was accepted separately;
this current-SDK source-only restore does not inherit new Native acceptance.

Official favicon: one canonical `/favicon.ico` anonymous GET, 4286-byte ICO,
SHA256 1adf72d595b3e8bb1570b03a7f7ec3010214ace4673c8b431e911133a1d1cd18.
It contains one original 32×32 bitmap and no embedded PNG. Deterministic lossless
PNG conversion preserves every decoded RGBA pixel (SHA256
b2cf73614a580032154a9f97e6ff1401236f50c6e6cae40a60b97703b9860ef2).
The dark background is part of the official original asset; no backing, crop,
resize, shadow or mask was added. The original ICO remains retained locally.


## Historical source-only recovery boundary

The business-only recovery preserved the original business, identity and metadata with the current canonical SDK, without an icon or standard-package registration. On 2026-10-20 the existing installed-package 32x32 PNG was restored to this primary source tree. It is the historical lossless PNG conversion of the official ICO described above, not original ICO bytes. No new network request, image processing, hash check, version or business change, compile, package or device acceptance is claimed. Low resolution remains OPEN; the other historical artwork/API/Native boundaries above remain separate.

## 0.1.2 identity restoration candidate

The current 512×512 PNG is a mathematical Bézier restoration, not an official vector original. The retained official 32px favicon is an opaque deep-blue identity canvas: its dominant background is #223445, its n is white, and its paired wings are red #ec2854. The restoration preserves that official canvas and uses those same colors. The clearer third-party structural reference is Keiyoushi's `lib-multisrc/galleryadults/res/mipmap-xxxhdpi/ic_launcher.png` (https://raw.githubusercontent.com/keiyoushi/extensions-source/main/lib-multisrc/galleryadults/res/mipmap-xxxhdpi/ic_launcher.png). The current NHentai.xxx build explicitly selects `theme="galleryadults"`; that shared theme does not establish an official exclusive logo or affiliation.

The editable design source is `identity.restored.svg`; it is retained with the source and is not a package member. The n and wings use smooth Bézier curves. The official square background is preserved without added rounding, shadow or mask; the third-party gradient plate/long shadow is not reproduced. A failed uncommitted 0.1.1 candidate incorrectly removed the official canvas, making white n invisible on the light 237 management card. That failure remains evidence, and this 0.1.2 successor corrects only that resource mistake. It does not add a new decorative plate or change the original n to black.

Version 0.1.0→0.1.2 enables ordinary delivery after the failed 0.1.1 local candidate. Parser, capabilities, requests, SDK, App slot/rounding/order and prior content evidence remain unchanged. Root reviewed the corrected SVG/512PNG. On 2026-10-21, device 237 imported 0.1.2 through the ordinary system download picker with an explicit import-success result. The manager and Browse raw images both show the white n and red wings clearly on the preserved deep-blue canvas; Root independently reviewed both raw images and the corrected source remains enabled. All 45 installed IDs retained their exact order across the 0.1.1→0.1.2 update. Before this session, only 44 sources existed on 237 and NHentai.xxx was absent; the failed local 0.1.1 was its first ordinary install, not an upgrade from 0.1.0. The original 44 kept their relative order. This accepts only the named icon consumers, not content availability, reading, downloads or all themes. Raw evidence is in Koma user-flow-audit/device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership/nhxxx012-{manager,browse}237-001/screen.png; the earlier 0.1.1 light-card failure is preserved separately.
