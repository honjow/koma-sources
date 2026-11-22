# Koma Rust Source SDK

生产Rust WASM书源的`no_std`公共ABI层，负责受控host调用、请求/结果信封、JSON有界处理和导出辅助；具体提供方路由、归属、权限和设置消费属于各源。它不保存设置、凭据、会话或图像缓存，不授予网络或账户访问。

`koma_source_buffers!`声明源容量，`koma_source_helpers!`复用请求/结果/HTTP解码，`koma_source_exports!`导出操作。自定义导出的源仍自行保护指针、长度、完整输入和回包。`hostHints.network=false`是固定信封标记，不能据此判断实际是否发生HTTP。

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
