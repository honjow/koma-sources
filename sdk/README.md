# Koma Rust Source SDK

生产Rust WASM书源的`no_std`公共ABI层，负责受控host调用、请求/结果信封、JSON有界处理和导出辅助；具体提供方路由、归属、权限和设置消费属于各源。它不保存设置、凭据、会话或图像缓存，不授予网络或账户访问。

`koma_source_buffers!`声明源容量，`koma_source_helpers!`复用请求/结果/HTTP解码，`koma_source_exports!`导出操作。自定义导出的源仍自行保护指针、长度、完整输入和回包。`hostHints.network=false`是固定信封标记，不能据此判断实际是否发生HTTP。

## 可组合宿主操作

源通过 SDK 主动调用宿主，HTTP/数据/会话共用原 `http_request` import。`Request::from_json` 和旧拦截器保持兼容；新写法使用 `http::RequestSpec`、`Header`、`Body` 和 `ResponseKind` 编码到调用方缓冲，再交给 `Client.send`，因此签名、重试和响应解析仍能放在源内拦截器中组合。

```rust,ignore
let context = runtime::Context::from_request(request_json);
if !context.supports(b"source_http_body_v1") || !context.supports(b"source_image_body_v1") {
    return original_image_request();
}
let mut request = http::RequestSpec::get(image_url);
request.response = http::ResponseKind::BodyRef;
let encoded = request.encode(&mut request_buffer)?;
let response = http::Client::<http::CallError>::new(&[]).send(encoded, &mut response_buffer)?;
// A 404 is still an HTTP response: source policy decides how to recover.
if response.status() != Some(200) { return source_error(); }
let input = response.body_ref()?;
// Read multiple input references, run source-local codecs/crypto, and write
// an output BodyRef in blocks. No host path or WASM pointer is exchanged.
let mut output = body::BodyRef::create()?;
let mut offset = 0;
while offset < input.len() {
    let count = input.read(offset, &mut chunk, &mut wire_buffer)?;
    if count == 0 { return source_error(); }
    output.write(offset, &chunk[..count], &mut wire_buffer)?;
    offset += count;
}
input.close()?;
output.image_result(&mut operation_data)
```

可执行的完整分块例子在哔哩漫画的编译特性 `host-body-example`；公开默认构建关闭它，普通图片保留原生直连。引用只在本次调用有效，不能保存到设置、私有状态或交互 checkpoint。只有成功 `get_image_request` 的 `image_result` 可转移所有权，阅读、预载、下载随后进入同一缓存与解码流程。失败或未转移的引用由宿主自动回收，源可提前 close 降低资源占用。

`runtime::Context` 查询实际特性和限制；缺限制字段表示未知。`unix_time_ms/random_bytes` 包装已存在的原生操作。`SessionRef::SourceCookies` 使用源隔离 Cookie jar，`BrowserGrant` 只复用宿主已授权的网页会话；它不自行授予登录权限。`PrivateSession` 包装既有并发初始化 lease、提交、终止和按 versionTag 失效。Ready 的 value_json 是 JSON 字符串 token，解码后再解释站点私有值。

新特性属于包含实现的宿主构建，不能从相同 App 版本号猜测。可选增强缺能力时保留旧路径；核心依赖须在正式宿主版本明确后声明最低版本。完整协议与资源限制以 App 仓 `docs/source-runtime-abi.md` 为准。浏览器 POST/二进制/DOM 与通用交互 checkpoint 仍是同一框架的后续实现范围。

## 设置与公共文本

源从请求顶层`settings`快照读值；能力、schema与实际消费者须一致。不要使用旧声明`koma_host.get_setting`替代请求设置，当前App不提供此import。敏感账号、token、Cookie不因参考存在偏好就开放普通明文存储。

`gallery_titles::display`服务四个实际GalleryAdults消费者，仅在顶层`settings.pref_short_title=true`时按固定 `(\[[^]]*]|[({][^)}]*[)}])` 与Kotlin.trim删标签，其余返回原输入。Unicode原字节与混合闭符语义保持；非法UTF8或容量不足失败，不截断。`gallery_titles::display_short`为3Hentai的`full_title`反向开关提供同算法显式布尔入口，不构造伪请求；原四源入口仍按各自真实settings读值。它不是任意正则接口，不引入动态分配、提供方请求或章节变化。

### Plain text entity adaptation

`plain_text::append_entity_decoded_json_content` accepts the unquoted serialized content of a JSON string and appends valid JSON string content to a bounded caller buffer. It validates Unicode escape pairs, uses the existing HTML5 entity table once, and escapes literal angle brackets before parsing so they remain text. It introduces no host import beyond the established HTML ABI and does not normalize any existing source field automatically. Sources opt in only at the explicit text consumer; invalid input or capacity overflow returns failure.

`plain_text::decode_json_content` decodes unquoted JSON string content into a bounded caller buffer, validating UTF-8 and Unicode pairs without HTML parsing or normalization. Callers choose their own whitespace and URL handling.

`json_utils::append_json_unescaped_then_escaped` 将合法的 UTF-16 高低代理对合成为一个 UTF-8 字符后继续既有 JSON 转义；孤立、非法代理或输出容量不足返回失败。原有普通文本、BMP 字符和其余处理路径保持。依赖此函数的可安装书源通过各自补丁版本分发该修复；采用公共解码修复不代表各站全部内容或设备路径已验收。


## 章节通用URL元数据

`chapters::Chapter.url`承接既有章节`url`元数据。`Chapter::new`默认`None`，
writer仅在`Some`时输出并使用原有JSON字符串转义；默认不输出新键，其他源的
原JSON保持不变。URL不参与业务ID、分组、阅读位置、权限或网络授权，不是新
宿主能力，不增加hostFeatures或最低宿主版本。雀魂等已有URL的源显式传入原值，
无需为typed目录迁移删除字段。`items`/`chapters`兼容包装仍由既有App消费链处理。

## 书源文案与调用语言环境

`localization::Catalog` 不需要 `alloc`。采用同一 `metadata.json` 的
`localization: {version:1, defaultLocale:"zh-Hant", messages:{key:{locale:text}}}`；
`source.name`、`source.description`、`authentication.usernameLabel` 和
`authentication.passwordLabel` 是静态显示 key，其他 key 仅由源显式接入已有显示字段。

```rust,ignore
use koma_source_sdk::{localization::Catalog, runtime::Context};
let catalog = Catalog::from_metadata(include_bytes!("../metadata.json"))?;
let context = Context::from_request(request);
let mut buffer = [0u8; 256];
let label: &str = catalog.text(&context, "settings.quality.name", "Image quality", &mut buffer)?;
// 以 label.as_bytes() 交给原 JSON 转义写出器；字段仍是普通字符串。
```

标准 `Source` trait 的全部 typed 请求及底层 `request::Request` 提供
`raw_json() -> &[u8]`，借用完整原始信封；例如 `get_settings(request: SettingsRequest<'_>)`
内使用 `Context::from_request(request.raw_json())`。不从 `args` 重造信封，也不使用
`raw_contains` 猜语言；原操作签名和私有字段不变，外部实现者不需新增必需 trait 方法。

`Context::environment() -> Option<localization::Environment>` 同时核对
`source_localization_v1`、顶层 `environment.version == 1`、合法 `uiLocale` 及
`localeFallbacks`。环境仅限当前请求快照；不猜系统语言、内容语言、UA 或 Accept-Language。
语言标签使用旧名称字段的 ASCII 子集及 63 字符上限；语言项原始数量先限 16，再规范化去重。
环境回退链须与 `uiLocale` 的精确→脚本/地区→语言链相符，不能混入另一个语言或脚本。
中文 HK/TW/MO 推 Hant、CN/SG 推 Hans，显式脚本优先。

`Catalog::from_metadata(&[u8]) -> Result<Catalog<'_>, localization::Error>` 借用输入；
`text<'o>(&self, &Context, key:&str, literal:&str, out:&'o mut[u8]) -> Result<&'o str, Error>`
返回调用方缓冲内的完整 UTF-8。已知语言先查其候选，再查目录的明确 defaultLocale 候选，最后
退该字段 literal；无环境仍走 defaultLocale→literal。目录缺失/未知版本不改变旧源。
坏的可选译文跳过；同一 key 超过 16 个原始 locale 项或有大小写别名重复时，该 key 不启用。
名称和认证标签沿原 128 UTF-16 单元、非空、首尾无空白及单行限制；描述保留有效换行和制表符。
其他显示字段仍由源调用方容量和原宿主字段预算约束。

`format<'o>(&self, &Context, key, literal, args:&[Argument<'_>], template:&mut[u8], out:&'o mut[u8])`
同样返回 `Result<&'o str, Error>`。`Argument {name, value: Value::Text/Unsigned/Signed}`
仅替换 `{name}`，`{{`/`}}` 表示字面大括号；缺参数/重复参数/坏模板明确失败。
模板及结果都由调用方提供缓冲，最终容量不足不会产出半个 UTF-8 字符。文本参数仍须通过原 JSON
转义写出；模板不执行 HTML/代码，也不翻译 ID、枚举值、游标、URL、认证声明或网站正文。

打包从同目录生成可选 `presentation:{version:1,defaultLocale?,texts:{role:{locale:text}}}`，
仅投影上述静态角色；认证显示角色只用于已有 credentials descriptor，不能创建认证权限。
同一 name 资源同时生成旧 `localizedNames`，旧映射保严格校验，冲突打包失败。
仅旧 `localizedNames` 适配 name 资源，不假造默认语言。规范 SourceInfo/name/description 及认证对象
均保持旧值，零参 source_info 和 ZIP 的 manifest/WASM/icon 三类成员不变。可选增强不提高最低宿主版本；
未采用 Catalog 的旧源不会批量改写。
