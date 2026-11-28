# Writing a Koma Source

A complete guide to creating manga sources for [Koma](https://github.com/honjow/Koma).

Sources are compiled to WebAssembly (`wasm32-unknown-unknown`, `#![no_std]`) and run inside the Koma app. The host provides HTTP, HTML parsing, and logging — your source handles site-specific logic.

## 书源声明

`SOURCE_INFO.description` 与本地化的 `source.description` 解释来源站点的内容。让读者在安装前知道能看到什么，以及这个站和别的站有什么区别。举一两个能从原站核实的特点，例如原创发行、主要语种、出版社书目、作者专栏或代表作品。只有影响实际可读范围时，才简短交代试读、会员、地区或图片额度。

用一两句自然的话。把站名换掉后，整段如果仍适用于一串同行站，就回到原站继续找能区分它的内容。“支持浏览、搜索、阅读”、逐项罗列源功能、泛泛的题材串列和开发验收状态都留在各自的说明里。题材、规模及免费范围按已核实的原站资料表述。

同一版本的零参 `source_info` 保持固定基准文案，多语言资料从同一文案目录投影。保留旧宿主能读取的固定简介。改动后实际执行最终 WASM 的 `source_info`，核对描述、源 ID 与版本，不能只改未被真实导出使用的常量；源目录 README 的用户备注同步更新。纯文案修订不改变访问权限、账号身份、解析、功能和图标。

| 内容 | 作者维护的位置 |
| --- | --- |
| 包身份、显示和安装版本 | 源的 `SOURCE_INFO`、`metadata.json` 中的显示资料和 `build.sh` 登记项 |
| 网站访问和会话 | 源实现或 `metadata.json` 中的 `requestLimits`、`websitePolicy` 和认证声明 |
| 作品与章节身份 | `metadata.json` 中的 `identity`。已有源的章节旧 ID 规则写成 `identity.chapter` |
| 目录、页表和交互能力 | 源的 WASM 操作及 SDK 类型，按 [Koma 源运行时 ABI](https://github.com/honjow/Koma/blob/master/docs/source-runtime-abi.md) 声明 |

`identity.chapter` 沿用原有规则对象，例如
`{"workPrefix":"manga:","chapterPrefix":"chapter:","chapterSeparator":":","titleSeparator":"#"}`。
打包器把它写入已发布客户端认识的 `websitePolicy.chapterIdentity`。
原有写法仍可打包，每个源只声明一份。书源更新后保持作品和章节标识稳定，
确有历史标识变化时使用该规则衔接已有进度和下载。

若源可读取 Mihon 扩展所记录的同一目录，在 `identity.mihon` 声明对应的十进制来源编号字符串（`sourceIds`）及 `work` 路径规则。规则去掉站内地址的 `pathPrefix` 与可选 `pathSuffix`，将剩余标识加上 `idPrefix` 与可选 `idSuffix`，交给现有详情接口。若 Mihon 同时记录 `/works/123/` 与 `/works/123/标题`，作品规则可保留 `pathSuffix: "/"` 并声明 `pathTitleAfterId: true`。宿主只取标题前的稳定标识。裸 ID 可用空前缀。固定查询键写进路径前缀，源记录完整地址时可声明固定 HTTPS 前缀。变量部分是普通标识或路径，一些源使用逗号分隔标识。宿主只把转换后的标识传给书源自己的接口。可选 `chapter` 使用同样规则，生成的标识须存在于该作品当前目录。可用 `pathIdSegment` 指定变量路径按斜杠拆分后的稳定标识位置，`0` 指第一段，`-1` 指最后一段。固定前缀为空时，两种带或不带首个斜杠的写法均支持。章节 ID 包含可变页数时，`targetIdSegment` 可声明按冒号拆分后从 0 开始的稳定标识位置。只有确认两端图片顺序一致才设置 `samePageOrder: true`。打包器输出可选字段 `mihonImport`，旧客户端仍通过原操作浏览和阅读。 章地址包含作品 ID 和章节 ID 时，可用固定 `pathWorkIdSeparator` 分隔两者，适用语义见 [Koma 源运行时 ABI](https://github.com/honjow/Koma/blob/master/docs/source-runtime-abi.md#作品与章节身份)。

站点行为由对应书源实现或声明，宿主按通用合同执行。包内规则与认证授权各有自己的校验，
一条站点声明不会授予访问凭据或跨域会话的权限。

新增能力按 [ABI 扩展与兼容](https://github.com/honjow/Koma/blob/master/docs/source-runtime-abi.md#扩展与兼容) 接入。

读取需要最终跳转地址的官方页面时，在 `websitePolicy.responseUrlOrigins` 登记精确 HTTPS origin，协商 `source_http_response_url_v1` 后用无正文 GET 的 `includeResponseUrl` 读取 `responseUrl`。跨域请求仅发送公开请求头。

官网网页登录令牌用 `authentication.webToken` 声明单个存储键、本站 API 和只读身份验证路径。协商 `source_web_token_v1` 后获取私有会话代标识，用 `webTokenGeneration` 发起授权 GET。书源保持站方章节授权规则，不能把账号材料放入页表或设置。

读取原站评论的源在 `metadata.json` 的 `comments` 声明 `version: 1` 及真实支持的 `manga`、`chapter`，用 `koma_source_comments_export!` 导出读取操作。请求、响应、分页及旧版兼容按 [原站评论 ABI](https://github.com/honjow/Koma/blob/master/docs/source-runtime-abi.md#原站评论)。章末吐槽原有设置继续生效。
其中作品和章节的外部标识归身份约定，网站访问的声明归访问策略。宿主运行时能力沿用
`hostFeatures` 协商。索引展示和安装所需字段由同一份源资料生成。

镜像设置是站点选择，不只是API基址。提供镜像选择的源必须在 `websitePolicy.homepageChoices` 中为同一设置声明每个取值对应的可浏览主页，并让当前镜像同时决定API、公开封面、图片CDN、阅读页和普通WebView入口。任何仍回落到默认域的公开资源都视为镜像未完成；宿主会按该声明打开右上角WebView，未声明时只能回退包默认主页。验证必须在直连网络下分别选择默认站与镜像：WebView应落到所选站点，列表和详情封面应来自所选镜像自己的资源域并完整解码。

发布前按包的最低支持版本验证基本使用路径。当浏览或阅读确实依赖新版能力时，提高源登记的
最低 Koma 版本，包清单和订阅索引由打包器写入同一值。可选扩展按宿主能力启用。
依赖能力的正式宿主尚未发布时，候选包不进入公开索引。

## Quick Start
### 验证页 UA

验证页默认使用系统 WebView UA，即由宿主从当前 ArkWeb UA 派生，在 OpenHarmony 平台标记内追加 `; Android 10`，保留实际引擎版本和移动标记。验证页的 UA 由宿主决定。书源的普通 HTTP 请求头只描述普通请求，制作源包时禁止用桌面浏览器 UA 或其他固定字符串配置验证页。

普通 HTTP UA 与 CF 验证 UA 分别管理。显式采用 `koma_source_sdk::image_headers`
时，普通 IMAGE 默认值参考 Mihon `c3bf7a78c074079a605ed7de86d8b7f58bea183b`
的 Chrome141 Android10/K，不改变旧源默认，也不设置或覆盖验证页 UA。共享策略保留
bare IMAGE 的完整 Referer，未知协议不发送 Priority 或声明压缩能力；匿名 HTTP 的
SourceCookies 与授权 BrowserGrant 分别选择，Cookie 始终按实际 origin 隔离。

有真机证据表明某个站点需要 ArkWeb 原生 UA 时，可用 `websitePolicy.systemWebAgentOrigins` 精确声明 origin。旧宿主仍需要 `webSessionAndroidAgent: true` 来显式启用 Android 兼容标记。验证时分别检查验证页与 Cookie 状态，以及应用重试后的原请求结果。

### WebView fetch bridge

原生请求被站点拒绝、但验证后的 WebView 会话可读取时，可声明 `websitePolicy.webFetchBridge: true`，沿已有精确同源授权使用隐藏 WebView。旧 `source_web_fetch_bridge_v1` 保留无正文 GET → 200 `bodyText` 的文档回放。

需要表单或二进制数据时，先协商 `source_web_http_v1`，再逐请求显式设置 `webFetch: {version: 1}`。新通道支持 GET、HEAD、POST、PUT、PATCH、DELETE，返回真实 HTTP 状态；GET/HEAD 禁止正文。正文类型、响应类型、预算和引用作用域遵守唯一 [Web HTTP ABI](https://github.com/honjow/Koma/blob/master/docs/source-runtime-abi.md#站点规则声明)。未显式选择本通道的 POST 等仍走原生路径；Web 请求已发出后不得转原生重放。

启用前须在真机验证列表、封面、详情、章节和阅读图片，并按实际依赖声明最低正式 App 版本，不能只声明就发布。


```bash
# 1. Create a new source from template
cd koma-sources
./build.sh --scaffold mysource
# This creates sources/mysource/ with Cargo.toml + src/lib.rs

# 2. Edit source metadata in sources/mysource/src/lib.rs
# 3. Implement the run_* functions
# 4. `--scaffold` already adds name|crate_dir|false|0.1.0 to SOURCE_ENTRIES
#    and adds the crate to workspace Cargo.toml; raise minAppVersion only when needed.

# 6. Add exactly one real local source icon: icon.png/jpg/jpeg/gif/webp/ico
# 7. Build
./build.sh --source mysource

# 8. Test
./target/release/koma-source-dev test-all \
  target/wasm32-unknown-unknown/release/koma_mysource_source.wasm
```

A source identity icon is required, not a manga cover or placeholder. Add exactly
one regular local `icon.png`, `icon.jpg`, `icon.jpeg`, `icon.gif`, `icon.webp` or
`icon.ico`. `build.sh` fails before compilation if the resource is missing,
empty, undecodable, duplicated, larger than 1 MiB or more than 1,048,576 total
pixels. Install Python 3 and Pillow for actual decoding. New manifests require
`icon` to equal the root filename; generated indexes must publish its actual
bytes using the same extension.

素材制作规范：以站点官网或站点正式发布的标识原件为素材。上游扩展的图标只用于
核对站点身份。成品必须是边角完整的正方形，去除任何阴影、额外底板与圆角
蒙版，由 App 负责界面所需的圆角显示。保留官方图案、字形与画面自带的
填色，缺少已核实的原件时保留待验状态。

素材制作与验收流程：选取素材前，主代理和执行子代理各自实际读取本段。
每张图标依次打开官方原件、处理稿和最终可安装包内的实物，在浅色和深色
背景核对真实身份、正方形边界、阴影、蒙版和留白，并在 App 书源列表检查
真实呈现。来源、尺寸、解码及哈希只能辅助检查。保留图案与文字原形，低清
素材只做保持原形的必要恢复。处理依据和前后证据记录在 ASSET_SOURCES.md。
单张返工保留其他已验收稿件。新生产包保留既有来源身份和用户数据。

## 移植前的源码对照

1. 固定上游扩展版本。从入口追到实际请求和结果，列出分类、筛选、搜索、
   分页、目录、阅读、登录和设置，以及默认值、适用条件和停止条件。
2. 在该源 README 按行为记下上游实现、Koma 调用位置与源内消费位置。
   逐项核对请求形状、分支、默认值和结果语义。比如分页要对照宿主传入的
   `args.page.cursor` 与源的实际读取处。通用栏目和提示使用界面语言；
   没有独立首页的参考不能把热门首屏包装成首页。
3. 实现后沿同一调用链复核。删改或适配写明代码位置及依据；漏项标为
   未完成。交付验证沿用项目现行规则。

GigaViewer 变体的访问规则由源入口显式选择：共享默认目录只使用原有 purchase
字段；仅当固定上游明确把缺失 `status` 视为下架时，使用
`chapters_with_missing_status_unavailable`。依据及现场复现边界见
[Comic Zenon 对照](../sources/comiczenon-ja/README.md#移植前的源码对照)。
不得把该源的缺值规则推广到其他站点，viewer 仍独立核实实际公开权限。

## Required image processors

When a source must restore encoded responses or rearrange decoded pixels, use
the [source-owned image processor ABI](https://github.com/honjow/Koma/blob/master/docs/source-runtime-abi.md#书源随包图片还原-v1).
Declare `requiredImageDecodes: ["wasm_image_v1"]` in `metadata.json` and set
the registry's minimum app version to the first release supporting the ABI.
Define a separate Rust workspace `cdylib` outside
`sources/` and set `wasmBuild.imageProcessorCrate` to its Cargo package
name. Keep its exported library name equal to the default Cargo name. The
standard build compiles it with an 8 MiB guest memory limit, embeds it in
`source.wasm`, and hashes the complete file after embedding. Include the
embedded module in any `wasmBuild.maxBytes` limit.

Use the same `get_image_request` handler for covers and pages. Return optional
`requiredImageDecode` stages in the resolved descriptor and check
`source_required_image_decode_wasm_v1` before emitting them. Verify normal
images, each required stage, and the actual offline download in the host.

## Architecture

```
Koma App (host)              Your WASM source
─────────────────            ──────────────────
koma_source_search()    →   run_search(req)        →  you call host::http_request()
koma_source_get_manga() →   run_get_manga(req)     →  you call host::html_parse()
koma_source_get_chapters()→ run_get_chapters(req)  →  host provides HTML parsing
koma_source_get_pages() →   run_get_pages(req)     →  host provides HTTP
         ↑                           ↓
    reads JSON result          writes JSON to payload_buf
```

The host calls your exported functions, you use host imports to fetch data, and you write JSON results to a static buffer. No allocator needed.

## Template Structure

Copy `template/` to `sources/<name>/`. The template uses three SDK macros that eliminate ~180 lines of boilerplate:

```rust
#![no_std]
use koma_source_sdk::host;
use koma_source_sdk::json_utils::{append_json_escaped, extract_json_string, write_bytes};
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};

// 1. Source metadata
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.example.mysource",   // unique reverse-domain ID
    name: "My Source",             // display name
    language: "en",                // ISO 639-1
    version: "0.1.0",
    api_version: "0.2",           // always "0.2"
    description: "A manga source.",
    author: "Your Name",
    content_rating: "safe",       // "safe" | "suggestive" | "nsfw" | "unknown"
};

const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    search: true,        // text search
    manga_detail: true,  // manga detail page
    chapters: true,      // chapter list
    pages: true,         // page image URLs
    listings: false,     // browse by listing (popular, latest)
    manga_list: false,   // browse with filters/pagination
    filters: false,      // filter options
    settings: false,     // source settings (cookies, quality)
    home: false,         // home page sections
    credentials: false,  // requires login
    image_request: false,// modify image request headers
};

// 2. Static buffers — adjust sizes for your source
koma_source_sdk::koma_source_buffers! {
    payload: 128 * 1024,   // output JSON
    http_out: 512 * 1024,  // HTTP response body
    body: 512 * 1024,      // decoded JSON body
    http_req: 2048,        // HTTP request (URL + headers)
    scratch: 8192,         // temporary buffers (scratch_a, scratch_b)
}

// 3. Helper functions
koma_source_sdk::koma_source_helpers!();

// 4. Implement operations
fn run_search(req: &[u8]) -> u32 { /* ... */ }
fn run_get_manga(req: &[u8]) -> u32 { /* ... */ }
fn run_get_chapters(req: &[u8]) -> koma_source_sdk::chapters::ChapterResult<'static> { /* ... */ }
fn run_get_pages(req: &[u8]) -> u32 { /* ... */ }
fn run_get_image_request(req: &[u8]) -> u32 { /* ... */ }
// Optional operations (return not_implemented if unsupported):
fn run_get_listings(_req: &[u8]) -> u32 { write_error("get_listings", "not_implemented", "") }
fn run_get_manga_list(_req: &[u8]) -> u32 { write_error("get_manga_list", "not_implemented", "") }
fn run_get_home(_req: &[u8]) -> u32 { write_error("get_home", "not_implemented", "") }
fn run_get_filters(_req: &[u8]) -> u32 { write_error("get_filters", "not_implemented", "") }
fn run_get_settings(_req: &[u8]) -> u32 { write_error("get_settings", "not_implemented", "") }

// 5. WASM exports — auto-generated
koma_source_sdk::koma_source_exports!("mysource", typed_chapters);
```

### What the macros provide

After `koma_source_buffers!`, these buffer accessors are available:
- `response_buffer()` — result buffer for writing responses
- `payload_buf()` — mutable `&[u8]` for building output JSON
- `http_out()` — mutable `&[u8]` where host writes HTTP responses
- `body_buf()` — mutable `&[u8]` for decoded body
- `http_req_buf()` — mutable `&[u8]` for building HTTP requests
- `scratch_a()`, `scratch_b()` — temporary working buffers
- `payload_slice(len)` — immutable view of payload_buf

After `koma_source_helpers!`, these functions are available:
- `write_error(op, code, msg) -> u32` — write error response
- `write_success_payload(op, len) -> u32` — write success from payload_buf[0..len]
- `read_request(ptr, len) -> Option<&[u8]>` — parse WASM ABI request
- `trim_ascii(bytes) -> &[u8]` — trim whitespace
- `decode_json_body(resp) -> Result<usize, FetchError>` — strip HTTP headers into body_buf
- `fetch_get(url, referer) -> Result<usize, FetchError>` — HTTP GET + decode in one call

After `koma_source_exports!("name", typed_chapters)`, all `koma_source_*` WASM exports are generated.

### Strict request and transport boundaries

For new ports, the opt-in SDK source_protocol module validates complete request
syntax and direct selected fields before HTTP. Its direct/raw_string/text helpers
distinguish absent from non-string or duplicate identities (including escaped key
aliases); legacy json_utils extraction helpers do not provide that guarantee.
text reuses plain_text::decode_json_content: decode a JSON search string before
URL encoding it, rather than encoding its literal escape bytes. feature reads
only the actual hostFeatures array. Existing sources are not silently switched.

source_protocol::body preserves HTTP refusal/rate status first, then checks the
Host's top-level ok completion, and reuses decode_json_body_into. For HTTP 403,
only the Host's strict top-level accessSignal:"challenge_required" becomes the
existing challenge_required error; absent or unknown string signals remain
forbidden. Nested fields and body text never promote a denial to a challenge;
duplicate or non-string accessSignal fields are invalid transport data and
return network_error. Other HTTP statuses retain their original mappings.
http::RequireSuccess uses this same policy. A 200 response
whose body transfer failed (ok:false, response_body/network_error) remains a
network error, not a site parse error. Site HTML parsing happens only after a
complete body. This centralizes the direct/duplicate/type checks already used
by strict catalogue sources without changing the legacy SDK helper behavior.

## JSON Request Formats

The App sends an envelope with an `args` object. The examples below show its contents.
For a pageable listing the first cursor is `null`; subsequent calls use the
returned cursor or, for numbered pages, a decimal string such as `"2"`.

| Operation | Request JSON |
|-----------|-------------|
| search | `{"query":"one piece","page":{"cursor":null,"limit":20},"filters":{}}` |
| get_manga | `{"mangaId":"manga:slug"}` |
| get_chapters | `{"mangaId":"manga:slug"}` |
| get_pages | `{"chapterId":"chapter:slug:001"}` |
| get_image_request | `{"pageId":"manga:slug","url":"https://..."}` |
| get_listings | `{}` |
| get_manga_list | `{"listingId":"popular","page":{"cursor":null,"limit":20},"filters":{}}` |
| get_home | `{}` |
| get_filters | `{}` |

Read identities from the actual `args` object. For image requests the App sends the work ID as `pageId` for a cover and the stable page ID for a reader page. When a listing already knows a public cover URL and all its GET headers, include
the complete `cover.headers` in that listing response, as described in the host
ABI. Reuse the listing's mirror or other already known inputs. Do not re-read a
session or fetch a detail page per cover. Keep the old resolver for older hosts
and for covers that actually need a ticket, session, or special transport.

`get_pages` and `get_image_request` may also receive optional `mangaId` and
`mangaUrl`. A bookshelf record without a provider URL uses Koma's internal
`source://<sourceId>/<mangaId>` locator. The App omits it for image requests and
may pass it to `get_pages`; either way, a source must treat any `source://` value
as "no URL" instead of parsing it as a detail page. Validate a detail URL only
when it is a real site URL, and bind identity through `mangaId` when the URL is
absent or internal. Do not fail a request merely because the optional URL is an
internal locator.

## JSON Response Formats

You build response JSON in `payload_buf()` using `write_bytes` and `append_json_escaped`.

### search

```json
{
  "items": [
    {
      "id": "manga:slug",
      "title": "Title",
      "subtitle": "Alt title (optional)",
      "cover": {"kind": "url", "url": "https://..."},
      "authors": ["Author"],
      "status": "ongoing",
      "contentRating": "safe",
      "sourceTags": ["tag1"]
    }
  ],
  "page": {"hasMore": false}
}
```

### 可选列表补充信息 listInfo

每个列表 item 可在协商 `source_manga_list_info_v1` 后带有序 `listInfo`，最多3项：
`{"type":"text","text":"第123话","label":"更新"}`（label可省）或
`{"type":"tags","items":["冒险","奇幻"]}`。均为纯文本，不传HTML或宿主样式。
SDK公开 `list_info::ListInfo` 和 `list_info::append(dst, cursor, request, entries)`；
在当前 item 的闭合花括号前调用。无 feature 时不写字段；协商后显式传空 entries
写 `listInfo:[]`，表示源主动关闭旧字段摘要；不调用helper才表示缺省fallback。
UTF-8按编码字节限制：text512、可选label96、tags1至8条且每tag96。
空白项、无效UTF-8、超限和容量错误拒绝整段optional；helper预检后原子追加，
Err不改已有buffer或cursor，调用方应省略增强并保留原item，而非丢弃正常列表。
原JSONv1及ID/封面/顺序/设置/旧字段保持；只复用当前列表已取到的资料，不增加HTTP。
显示标签和源文本仍由源提供普通 UTF-8 字符串；需要多语言时显式使用下述 Catalog，不从内容语言或 HTTP 请求头猜 UI locale。
宿主对坏optional可容错忽略；不得把它当整个列表无效。可选增强不提高最低App，
发布必须有新的源patch；复用真实列表原件核新实际WASM和无feature旧输出。
缺省摘要复用subtitle、作者/已知状态及明确的内容tags；旧sourceTags可能只是来源标记，
不得默认当题材展示。需要其它内容或顺序时由源返回listInfo；显式空数组关闭自动摘要。

### 可选书源界面文案本地化

当前 SDK 与打包器使用同一份 `metadata.json` 文案目录；本段记录实装用法，
不表示正式宿主已发布或真机验收完成。例如：

```json
{"localization":{"version":1,"defaultLocale":"zh-Hant","messages":{
  "settings.quality.name":{"zh-Hant":"圖片品質","zh-Hans":"图片质量","en":"Image quality"}
}}}
```

标准 `Source` 的 typed 请求（如 `SettingsRequest`）用 `raw_json()` 取得完整原信封，
在原方法内按显示字段显式调用，不重造 `args` 信封：

```rust
use koma_source_sdk::{localization::Catalog, runtime::Context, source::SourceError};
let context = Context::from_request(request.raw_json());
let catalog = Catalog::from_metadata(include_bytes!("../metadata.json"))
    .map_err(|_| SourceError::source_error("invalid localization metadata"))?;
let mut buffer = [0u8; 256];
let label = catalog.text(&context, "settings.quality.name", "Image quality", &mut buffer)
    .map_err(|_| SourceError::source_error("localization output failed"))?;
// label 为缓冲内 &str；用原 JSON 转义写出器写 label.as_bytes()，响应字段仍是普通字符串。
```

Host 同时提供 `source_localization_v1` 与合法 `environment` 时，Context 才读取 UI 语言。
`localeFallbacks` 必须与该语言完整 canonical 链逐项一致，不能缺项、额外、重复或乱序。
Catalog 按精确→保留脚本的地区/脚本→语言候选，再到明确 `defaultLocale` 与字段 literal 回退；
缺环境的旧 Host 仍走 defaultLocale→literal，目录缺失/未知版本走 literal，不猜英语或内容语言。

打包器仅将 `source.name`、`source.description`、`source.author` 及已有 credentials 认证的
`authentication.usernameLabel/passwordLabel` 投影到可选 `presentation.texts` 的
`name/description/author` 及同名认证显示角色，
并由同一 name 资源生成旧 `localizedNames`；冲突打包失败。没有显式 `source.name` 时，旧名称继续
使用原 `localizedNames` 及其回退，不受新增简介默认语言影响。翻译投影不按 UI 语言改写固定
SourceInfo 或认证声明；作者随源版本修订基准简介时应同步译文。无资源沿旧值，不增加 ZIP 附件或 UI/授权权限。每 key 最多16个原始 locale项；名称、作者和认证
译文沿单行128 UTF-16限制，输出仍受调用方缓冲与原字段预算约束。详见 [SDK用法](../sdk/README.md#书源文案与调用语言环境)。

本地化是可选增强，不单独抬最低 App；未接 Catalog 的旧源无需改写。只翻译源自编的显示文案，
不翻译业务 ID、option value、default、scope、游标、URL、正则或网站正文。旧字符串选项在接入前
须显式固定与旧请求一致的 ID/value，保持保存值、默认值、顺序与行为，不递归翻译任意字段。

### get_manga

```json
{
  "manga": {
    "id": "manga:slug",
    "title": "Title",
    "alternateTitles": ["Alt Title"],
    "description": "Description text",
    "cover": {"kind": "url", "url": "https://..."},
    "authors": ["Author"],
    "artists": ["Artist"],
    "status": "ongoing",
    "contentRating": "safe",
    "language": "zh",
    "tags": ["Action", "Comedy"],
    "links": [{"kind": "source", "url": "https://..."}]
  }
}
```

### get_chapters

New sources and changes to an existing chapter parser use the typed SDK export.
The chapter handler returns `ChapterResult`, whose successful value can only be
created through one of these SDK entry points:

- `ChapterResponse::single` for one chapter.
- `ChapterResponse::ordered` for a complete, verified catalog with an explicit
  `ChapterOrder::OldestFirst` or `ChapterOrder::NewestFirst`. It preserves the
  returned array's order and assigns reading positions independently of titles.
- `GroupedChapters` for mixed, paginated, or partially verified catalogs. Each
  `push` requires `ChapterPlacement::InGroup { group, position }` or
  `ChapterPlacement::ManualOnly`. Positions are nonnegative and unique within
  each group and span all catalog pages. Where the provider cannot distinguish
  equal positions, leave those entries manual or split the sequence at the
  ambiguous boundary. Use `ManualOnly` for entries whose
  order cannot be verified and leave manifest `chapterOrder` unset when using
  grouped catalogs.

With `chapter_reading_order_v1` negotiated, an explicit `ManualOnly` serializes
`continuationEligible:false`. Omitting that exclusion would let the host treat
an entirely manual catalog as legacy input and infer unintended neighbors.
Without negotiation, retain the original unextended output; this does not make
ordinary `ChapterResponse::single` entries implicitly manual-only.

Reading positions come from the complete provider catalog with a verified direction,
not from chapter numbers, titles, dates or the App's display reversal. Missing or
repeated display numbers and titles do not make a chapter `ManualOnly`. Preserve
different business IDs; collapse repeated occurrences of the same verified chapter
identity without deleting distinct chapters or editions. Keep positions stable
across catalog pages and App display/filter settings. A source preference that
explicitly changes the catalog sort basis retains its upstream reading semantics.
Separate reading groups only
for actual provider editions or sections; a missing number is not a group boundary.

For sources with custom WASM exports, use
`koma_source_chapter_export!("name", run_get_chapters)` for this operation.
The macro also accepts the source's existing error writer and ABI request reader
to retain structured error information and existing request validation.
Sources using the full export macro can pass the error writer as the third
argument. Existing sources migrate when their chapter parsers change.

Run `cargo check --locked --release --target wasm32-unknown-unknown -p <crate>`
to catch missing placements or a raw success return before building WASM.
Replay a saved provider catalog to check real chapter IDs, display order, and
neighbor transitions, including pagination or mixed sections when present.
The SDK uses the existing JSON ABI and emits explicit reading positions only
when the host advertises `chapter_reading_order_v1`.

```json
{
  "items": [
    {
      "id": "chapter:slug:001",
      "mangaId": "manga:slug",
      "title": "Chapter 1",
      "chapterNumber": "1",
      "volumeNumber": null,
      "language": "zh",
      "publishedAt": null,
      "updatedAt": null,
      "pageCount": 20
    }
  ],
  "page": {"hasMore": false}
}
```

### get_pages

```json
{
  "pages": [
    {
      "id": "page:0",
      "index": 0,
      "image": {"kind": "url", "url": "https://..."}
    }
  ]
}
```

**Important**: Page images use nested `image.url`, not a flat `url` field.

#### 页表与图片预载

`get_pages` 返回完整、有序的页身份和图片描述。原站的章节接口一次就返回整章图片地址时，直接使用这些地址。需要逐页请求 HTML 或接口才能得出图片地址时，页表只读取提供页数和定位信息的目录文档，将各页解析留到已有的 `get_image_request`。

书源不得在进入阅读器前逐页请求整章来收集图片地址。每次 `get_image_request` 只处理宿主指定的那一页，阅读器按用户设置的页数和网络条件安排预载，下载队列沿用同一逐页解析入口。单页请求失败只影响该页，由宿主负责重试。验收分页源时，用长图集确认入场请求数不会随总页数增长，再阅读首、中、末页核对顺序和完整性。

### get_image_request

```json
{"url": "https://...", "headers": {"Referer": "https://..."}}
```

Used to add custom headers (e.g. Referer) to image requests. Most sources can pass through unchanged.

### get_listings

```json
{"listings": [{"id": "popular", "name": "Popular"}, {"id": "latest", "name": "Latest"}]}
```

### get_manga_list

Same format as search response. Request includes `listingId` and `page`/`limit`.

### Cover / Image kinds

- URL: `{"kind": "url", "url": "https://..."}`
- None: `{"kind": "none"}`
- Placeholder: `{"kind": "placeholder", "label": "...", "width": 800, "height": 1200}`

## Building JSON Responses

Since there's no allocator, you build JSON by writing bytes into `payload_buf()`:

```rust
fn run_search(req: &[u8]) -> u32 {
    let query = extract_json_string(req, b"query").unwrap_or(b"");
    // ... fetch and parse data ...

    let p = payload_buf();
    let mut c = 0usize;  // cursor

    // Each write_bytes/append_json_escaped returns false on overflow
    let ok = write_bytes(p, &mut c, br#"{"items":[{"id":"manga:1","title":""#)
        && append_json_escaped(p, &mut c, title_bytes)  // escapes " \ etc.
        && write_bytes(p, &mut c, br#"","cover":{"kind":"none"}}],"page":{"hasMore":false}}"#);

    if !ok {
        return write_error("search", "internal_error", "buffer overflow");
    }
    write_success_payload("search", c)
}
```

### Key JSON utilities

| Function | Purpose |
|----------|---------|
| `write_bytes(dst, &mut cursor, src)` | Write raw bytes, advance cursor |
| `append_json_escaped(dst, &mut cursor, src)` | Write bytes with JSON escaping (handles `"`, `\`, control chars) |
| `write_usize(dst, &mut cursor, val)` | Write a number as ASCII digits |
| `write_url_encoded(dst, &mut cursor, src)` | URL-encode bytes |
| `extract_json_string(data, key)` | Find `"key":"value"` in JSON, returns the value bytes |
| `extract_json_number(data, key)` | Find `"key":123` in JSON |
| `JsonArrayIter::new(data, key)` | Iterate `"key":[{...},{...}]` |

## HTTP Requests

### Simple GET

```rust
use koma_source_sdk::host::http_request;

// Build HTTP request (URL + headers)
let req_len = koma_source_sdk::build_get_request(
    http_req_buf(),
    b"https://example.com/api/manga/1",
    None,           // referer
    &[],            // extra headers
).unwrap();

// Execute request — host writes response to http_out
let resp_len = http_request(&http_req_buf()[..req_len], http_out()).unwrap();

// Strip HTTP status line + headers, decode body to body_buf
let body_len = koma_source_sdk::decode_json_body_into(
    &http_out()[..resp_len],
    body_buf()
).unwrap();

let json_body = &body_buf()[..body_len];
```

### One-shot GET (helper)

```rust
// fetch_get does build_get_request + http_request + decode_json_body in one call
let body_len = fetch_get(b"https://example.com/api/search?q=test", None).unwrap();
let json_body = &body_buf()[..body_len];
```

### POST request

```rust
let req_len = koma_source_sdk::build_post_request(
    http_req_buf(),
    b"https://example.com/api/search",
    None,                          // referer
    b"application/json",           // content-type
    br#"{"query":"test","page":1}"#,  // body
).unwrap();

let resp_len = host::http_request(&http_req_buf()[..req_len], http_out()).unwrap();
```

### Custom headers

```rust
// build_get_request accepts extra headers as key:value pairs
let req_len = koma_source_sdk::build_get_request(
    http_req_buf(),
    url,
    Some(b"https://example.com"),   // referer
    &[
        b"X-Api-Key: your-key-here",
        b"Accept: application/json",
    ],
).unwrap();
```

## HTML Scraping

For sites without a JSON API:

```rust
use koma_source_sdk::host;

fn run_search(req: &[u8]) -> u32 {
    let query = extract_json_string(req, b"query").unwrap_or(b"");
    let url = b"https://example.com/search?q=test";

    let req_len = koma_source_sdk::build_get_request(http_req_buf(), url, None, &[]).unwrap();
    let resp_len = host::http_request(&http_req_buf()[..req_len], http_out()).unwrap();
    let html = &http_out()[..resp_len];  // raw HTTP response (includes headers)

    // Parse HTML — host provides HTML parser
    let doc = match host::html_parse(html) {
        Ok(d) => d,
        Err(_) => return write_error("search", "parse_error", "html_parse failed"),
    };

    // Select first match
    if let Ok(item) = host::html_select(doc, b"div.result-item") {
        let title = host::html_text(item, scratch_a()).unwrap_or(b"");
        let link = host::html_attr(item, b"href", scratch_b()).unwrap_or(b"");
        // ... build JSON ...
    }

    // IMPORTANT: close documents to free memory
    host::html_close(doc).ok();

    // ... return result ...
}
```

### HTML API reference

| Function | Returns | Description |
|----------|---------|-------------|
| `html_parse(html) -> Result<HtmlDescriptor, i32>` | Document handle | Parse raw HTML (including HTTP headers — host handles stripping) |
| `html_select(doc, selector) -> Result<HtmlDescriptor, i32>` | Element handle | CSS select first matching element |
| `html_select_all(doc, selector, out, cap) -> i32` | Count | Select all matches, write descriptors to out buffer |
| `html_attr(element, attr, out, cap) -> Result<usize, i32>` | Written length | Get attribute value |
| `html_text(element, out, cap) -> Result<usize, i32>` | Written length | Get text content |
| `html_close(descriptor) -> Result<(), i32>` | — | Free descriptor. **Always close documents when done.** |

Selected element descriptors refer to nodes in their original parsed document. Closing an element releases only that descriptor; closing its document invalidates all selected descriptors from that document.

## Source Settings

For sources that need user-provided values (cookies, tokens, quality preferences):

```rust
// In SOURCE_CAPS:
settings: true,
credentials: true,  // if auth is needed

// Read a setting at runtime:
fn get_setting_or(key: &[u8], default: &[u8]) -> &[u8] {
    let mut buf = scratch_a();
    match host::get_setting(key, buf) {
        Some(val) => val,
        None => default,
    }
}

// Usage:
let cookies = get_setting_or(b"cookies", b"");
let quality = get_setting_or(b"image_quality", b"high");
```

## Testing

### Individual operations

```bash
DEV=./target/release/koma-source-dev
WASM=./target/wasm32-unknown-unknown/release/koma_mysource_source.wasm

# Check metadata
$DEV info $WASM

# Search (2>/dev/null for clean JSON output)
$DEV run --op search --request '{"query":"one piece"}' $WASM 2>/dev/null

# Chain: search → manga → chapters → pages
$DEV run --op get_manga --request '{"mangaId":"manga:SLUG"}' $WASM 2>/dev/null
$DEV run --op get_chapters --request '{"mangaId":"manga:SLUG"}' $WASM 2>/dev/null
$DEV run --op get_pages --request '{"chapterId":"chapter:SLUG:001"}' $WASM 2>/dev/null
```

### Full test suite

```bash
$DEV test-all $WASM
```

This automatically chains search → get_manga → get_chapters → get_pages and reports pass/fail.

### Web UI

```bash
$DEV serve target/wasm32-unknown-unknown/release --port 3010
# Open http://localhost:3010 in browser
```

## Common Pitfalls

### No std library
No `String`, `Vec`, `format!`, `println!`. Use `write_bytes`, `append_json_escaped`, and static buffers for everything.

### Non-ASCII byte literals
`br#"中文"#` causes compile errors in `no_std`. Use UTF-8 hex arrays with a comment:
```rust
// "最新" in UTF-8
const LABEL_LATEST: &[u8] = &[0xe6, 0x9c, 0x80, 0xe6, 0x96, 0xb0];
```

### Buffer overflow
Always check return values of `write_bytes` / `append_json_escaped`. They return `false` when the buffer is full.

### HTTP response format
The host writes a JSON envelope to `http_out`, not a raw HTTP status line or headers:

```json
{"ok":true,"status":200,"bodyText":"response body","networkPerformed":true}
```

`ok` means the transport completed; an HTTP 404 or 503 can also have `ok:true`.
`decode_json_body` / `decode_json_body_into` check HTTP status before decoding
`bodyText`. The official field is `status`; `statusCode` is accepted as a legacy
alias, with `status` taking priority if both are present. HTTP 2xx continues to
body decoding, 404/429/other 4xx/5xx uses the existing `FetchError` categories,
and other status codes (including 3xx) produce `FetchError::Network`; the helper
does not follow redirects or retry. Transport `ok:false` produces an error.
Missing both status fields retains the legacy behavior: `ok:true` decodes the
body, and `ok:false` is a network error. This compatibility case is not proof of
HTTP success. Successful body string decoding and resource budgets are unchanged.

### HTML descriptor leaks
Always call `host::html_close(doc)` when done with a parsed document. The host has limited descriptor slots.

### Dev runner output
HTTP request logs go to stderr. Use `2>/dev/null` when parsing JSON programmatically:
```bash
$DEV run --op search --request '{"query":"test"}' $WASM 2>/dev/null
```

### Dev runner needs .wasm files
The `serve` and `run` commands need raw `.wasm` files from `target/wasm32-unknown-unknown/release/`, NOT `.koma` zip packages.

### Chinese/JSON text in output
When writing Chinese text to JSON output, use `append_json_escaped` — it handles the `\uXXXX` escaping correctly.

## Reference Sources

| Source | Type | Lines | Good for learning |
|--------|------|-------|-------------------|
| `example-demo` | JSON API | ~260 | **Start here** — minimal working source with SDK macros |
| `terrahistoricus` | JSON API | ~350 | Clean JSON API with pagination |
| `baozimh` | HTML scraping | ~1300 | Full HTML scraping reference |
| `mangadex` | JSON API | ~1200 | Complex API with auth, covers, dedup |

## Distribution

### Building packages

```bash
./build.sh              # Build all registered sources
./build.sh --source foo # Build single source
```

Output: `dist/sources/<name>/<name>-<version>.koma` (zip with `manifest.json`, `source.wasm`, and exactly one required package-owned root icon). The generated manifest carries `author`, `description`, and `contentRating` from `SOURCE_INFO`; its `minAppVersion` comes from that source's `SOURCE_ENTRIES` row. Keep these source-level fields accurate because Koma preserves them in the installed source details. `minAppVersion` must use canonical `MAJOR.MINOR.PATCH` text, for example `0.1.0`.

### 正式书源库增量发布

主分支构建仍生成完整目录、书源包、图标和版权交付包。交付 ZIP 的文件时间和权限固定，内容相同的重建结果可以直接复用。旧版 App 从索引同目录解析相对路径，因此正式订阅继续使用 `releases/latest/download/index.json`。

发布在 `source-library-a` 和 `source-library-b` 两个固定发行目录间轮换。只更新当前未被选为 latest 的目录，匹配 GitHub 已存附件的大小和 SHA-256 时直接复用，沿用 GitHub CLI 的有界并发上传。两目录第一次使用各自补齐基础附件。索引仍使用相对路径；同名文件内容发生变化时用带摘要的文件名，保留当前索引引用的旧文件，使用户已缓存的索引仍可下载。

发布先核对书源包、图标及旧索引所需的附件，再上传并核对新索引。确认主分支版本及现有 latest 未变后才切换。检测到索引和附件内容均不变时跳过发布。上传遇到 GitHub 二级限流或暂时故障时才退避，重新核对草稿并自动续传缺失附件。多次重试仍失败时可重跑发布任务，继续复用已经写入且校验一致的附件。附件接近 GitHub 上限时，只在备用目录清理当前和新索引均未引用的历史附件。已发行的历史 `sources-<commit>` 标签及手动发行快照保留独立用途。

### 发布时保留已发布稳定包

依赖尚未正式发布宿主的候选在源 `metadata.json` 中声明 `distribution.available: false` 和具体原因。它仍保留在候选 `catalogue.json`；不可通过降低最低版本发布候选，也不应因此删除已有稳定版本的安装入口。

确需保留旧入口时，只在仓根 `release-package-pins.json` 明确登记源 ID、固定 `sources-<commit>` tag、版本、索引／整包／图标 SHA256 及保留原因。两个既有发布工作流在完整打包后、上传和创建 Release 前运行：

```bash
python3 scripts/apply-release-package-pins.py --pins release-package-pins.json --output-dir dist
```

脚本只读取明确 pin 的官方 GitHub Release，验证旧索引、完整包的身份／版本／manifest／三个根成员及原图标，按原 `release-packages`／`release-icons` 布局恢复完整旧条目。旧 manifest、WASM、包及图标不重写，不与候选字段混合；其余索引记录及候选 catalogue 保持。SHA、身份或未发布资产文件名冲突使发布前步骤失败；脚本不调用 Release 写接口，不覆写已发布资产。普通 `build.sh` 和本地单源构建不调用该联网步骤；没有 pin 时脚本不联网。正式宿主发布后由维护者复核解除条件，同时移除对应 pin 和不可用声明；局部 App 构建或提交成功不自动解除。

### Third-party source repos

Create your own repo with the same structure. Users add your release index URL in Koma settings.

Minimum:
1. A `sources/your-source/` crate depending on `koma_source_sdk`
2. A `build.sh` that produces `.koma` packages + `index.json`
3. GitHub releases with downloadable assets

The `index.json` format:
```json
{
  "sources": [
    {
      "id": "com.example.mysource",
      "name": "My Source",
      "version": "0.1.0",
      "lang": "en",
      "nsfw": false,
      "author": "Example author",
      "description": "A concise source description.",
      "contentRating": "safe",
      "minAppVersion": "0.1.0",
      "pkg": "mysource-0.1.0.koma",
      "icon": "sources/mysource/icon.png"
    }
  ]
}
```

`pkg` must be a safe relative `.koma` path resolved from the index document's directory. For a GitHub Release, `index.json` and `mysource-0.1.0.koma` are sibling release assets, so the flat filename above resolves correctly. Koma then fetches the package and lists the available source for download.
