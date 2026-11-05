# 拷贝漫画：未发布的独立移植

固定参考为 coffee522/extensions-copymanga 的 Apache-2.0 commit
655896cce1dd8a235c10d4c9d4b370d7f265c651；已读全部六个 Kotlin 源文件、
build 定义与许可证。当前 Rust/WASM 独立实现位于同一源仓，不参加根 workspace、
registry 或公开 index。没有复制缺乏明确许可的 APK/smali，也没有复制 Aidoku/Homo。

已实现参考的热门推荐、最新偏好、关键词搜索、排行频道、地区/状态/题材及四种排序，
21 条分页、动态题材、三个 API/Web 域名对、URL 导入、详情、所有分组的完整目录分页，
以及原 words/contents 稳定排序后的完整原图页表。默认繁体；简体转换使用原依赖
com.luhuiguo:chinese-utils:1.0 的完整原始 simp.txt / simplified.txt，Rust 独立实现
最长词组优先转换。原 Java Converter 的整张 20902 字符表、全部 549 词组与边界输入
已和实际 WASM 比较，未用小字表冒充完整转换。两份资源字节未改，归属见 NOTICE、LICENSE。

默认域对为 api.mangacopy.com / www.mangacopy.com；另有 copy3000、copy4000 对。
旧网页抓取成功的 copy4000.com 无 www 地址不是当前参考默认 API 已可用的证明。
HTML 列表只解析 `.exemptComicList .exemptComic-box` 的受限数据 literal，包含单/双引号、
标识键、布尔/空值和转义；不执行网页 eval、函数或任意 JavaScript。

必要宿主适配与差异：
- Koma 没有 Mihon Sort.Selection，采用四值 select 表达完全相同的两种排序乘升降方向。
- 作品分级未由网页逐作提供，保持 unknown；包的成人许可边界不把每作强标成人或安全。
- 章节上传日用现 dateUpload ABI，保原参考 +08:00 的日期语义。完整分组按原 JSON
  成员顺序处理，default 优先、组内逆序；没有用有序字典按字母重排分组。
- 原 zip/mapNotNull 会截断长度不等的页对或忽略坏图行。遵守 Koma 已采用的不丢页边界，
  这类响应整次返回解析失败；有效重复图片保留各自位置和 PageID。
- 无效的旧 latest 偏好按原 allowedValue 回退；明确的搜索筛选输入保持严格类型/取值校验。
- 当前 get_filters 的实际加载请求由 fetchFilterData 门控，普通筛选元数据不发网络。
  参考保存的 theme cache 在其 getFilterList 中不用于替代 data；本移植不另造公开缓存键。

宿主适配：
- API 访问要求 `source_http_copymanga_api_v1`。宿主仅在 API 26 起为三个已声明的
  HTTPS API 域提供 HTTP/1.1、独立连接、无 Cookie 和缓存的请求。未声明能力时，
  源在 API 联网前返回 unsupported。普通网页、图片与设置元信息保留原路径。
  HTTP 210 以提供方拒绝处理，不改域、请求身份或传输后重试。
- “最新模式”沿公共设置表单切换排行与普通排序、地区、状态、题材的可见性。
  User-Agent 的空白输入在保存前拒绝，旧空值由 getter 回退到原默认值。
- 作品的原站链接跟随已保存域名；标题栏普通网页入口的动态映射须经宿主正常入口验证。

本地 WASM 已执行 36 个协议和恢复场景，并按两份原站网页核对 100 条列表。
237 使用内部 V2 打开最近更新及《啟示錄四騎士》，取得 280 章目录和 190 页的
真实页表，图片请求实际完成。这次访问与此前另一次章节请求的 HTTP 210 拒绝分别
记录。收到 210 时保留拒绝语义，沿同一身份停止该次请求。

正式工作区使用原 ABI 编译参数、大小优化和 LTO。图片 URL 中唯一的分辨率正则只
编入它使用的 Unicode 大小写和十进制数字类别，表达式及替换规则保持一致。
安装包在打包前检查实际 WASM 是否超过 Native 1 MiB 上限。最低 App 为 Koma 1.2.0。

0.1.1 区分作品封面和章节页图的请求身份。封面沿用站点原图片请求头，保留已有正文身份校验。237 已用普通列表确认冷封面实际显示。
