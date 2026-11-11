# 哔咔漫画 (Picacomic)

独立 no_std Rust/WASM 实现，完整移植 Keiyoushi `de743aed846da53e00c45ab26ee0e77f7bb4355b`（v14/lib1.6，Apache-2.0）的 Picacomic.kt、Dto.kt、Filters.kt 和 ChannelDns.kt。保留采用的源名称、NSFW/zh 分类和 manhuabika.com 公开链接。后者是参考声明的公开 Web 前端。凭据只发送到 Picacomic API origin。浏览、详情、章节和页需要 Picacomic 账号。要求 Koma 1.2.0 或更高。

认证使用已批准的凭据描述符和显式 Native 私有表单。用户名映射到线上 email。无用户名/密码/token 设置、请求参数、Web Cookie、操作结果或日志提供认证。get_authentication 读取就绪私有状态，或中止空所有者，零 HTTP。无输入浏览在任何登录/API 请求前返回 auth_required，包括用模仿凭据的公开参数/设置。

显式登录只在其授权的 authenticate 调用中使用 credential_input，然后申领初始或 replaceVersionTag 租约。先前账号在新登录和原子提交都成功前保持发布。拒绝、取消、提交失败和中止保留先前账号。登出只使本源/安装代失效。普通 401 刷新过期保存 token 代一次；并发调用采用新发布 token。原请求的 URL/方法/体/时间/nonce/签名在重试时保留，只替换 Authorization。403 不触发登录或隐藏重试。凭据和 token 只存在于既有私有 Asset 所有者。

所有原公开偏好保留：BLOCK_GENRES、IMAGE_QUALITY（original/low/medium/high）、APP_CHANNEL（默认 2，选项 1/2）和 APP_CHANNEL_URL。旧 Android channel3 有效迁移解释为 2；不静默写宿主公开设置。在源设置中用账号管理通过私有表单登录、登出或替换账号。参考的分类/标签回退和精确屏蔽词匹配保留。

热门分页、参考 Latest/随机（总是更多）、收藏优先、排行优先、空关键词/分类 API、高级搜索 JSON 体、公开漫画/阅读器 URL 导入、元数据、完整分页 eps 和完整分页页都实现。章节顺序和原 page*limit 序号保持；重复图像 URL 保留独立位置。失败的后页从不返回部分内容。可空 ISO 时间戳解析为已验证夹具保留毫秒和时区。相关/深链注册不由当前源 ABI 暴露。标准图像描述保留原站点 Referer，不向不透明 fileServer URL 泄漏 API/认证头。

签名保留固定公开 API key/客户端头和精确内容类型。HMAC-SHA256 用 RustCrypto SHA256 带 RFC2104 填充和参考的公开 key。时钟和安全随机来自既有私有 RPC。拒绝采样从原字母数字集均匀生成 32 字符。遗留 Host bodyBase64 字段携带原始 UTF-8 文本，如同既有 SDK/CLI/App 实际实现；JSON 体不 base64 编码。

Channel 2 通过 `source_dns_pica_channel_v1` 请求宿主 DNS 计划。宿主选择提交的 init URL 并执行公开初始化。源把返回的不透明证明带入原签名 HTTPS API 请求和同域阅读器图像描述符。证明从不持久化在凭据、设置或诊断输出。缺失能力、拒绝初始化或缺失证明在 API 访问前停止该请求。宿主为同一安装包在进程内存保留已验证计划。封面请求使用其漫画 ID，跟随与原缩略图相同的公开头和 origin 限定传输。Channel 1 和无关图像 CDN 保持普通传输。匿名浏览和空显式凭据不发 init 或 API 请求。

本地验证在 Node 适配器下运行实际 WASM 和生产私有时会所有者。44 个先前协议和所有权用例和 22 个 channel 用例使用离线提供方夹具。签名请求由独立 Node crypto 对照 Kotlin 签名 key 检查。实际 SourceModels 消费收藏复选框和章节日期。实际宿主计划重用由宿主 RCP 适配器测试覆盖。账号状态和设置元数据不发 HTTP 请求。成功认证后的登录和阅读路径用夹具执行。用真实账号成功登录和阅读仍未验证。

内部 V2 评审更正：作品 ID 是规范安全 URL 段（点或编码路径穿越在 RPC 前失败），普通和 URL 导入详情都要求响应 _id 所有权。实际冻结 V1 响应/HTTP 反例保留在 owner-before-fix.json；新用例保留原提供方/宿主夹具，不为迁就实现而改变预期所有权。
