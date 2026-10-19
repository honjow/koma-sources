# 0.1.2 原业务保全边界（不代表发布或站点验收）

本组保留原 firstFetchError 完整 0.1.2 业务及其前序 UA R2/章节完整性；Root 原回执仅接受明确受控 Host 范围，没有当前站点成功、正文或 Native 接受。未引入后来的 0HTTP 导航候选，不采用 R1 自设 UA 长度/ASCII 限制。原成功状态空正文 fallback 合同仍未知，原首页 TLS 失败及旧 API 拒绝保持，不据此重试。

当前缺可信合格必需图标，本组不加入素材或新 build registry，也不生成标准包。原标准 registry 行已存在，不把它当作当前可打包或可用证明。当前 SDK 与共同打包/CLI 原样保持；下方为完整原 README 历史字节。

# Happy漫画 0.1.2：已有 User Agent 设置执行链

基于 com.happymh.koma 0.1.0源码原型；没有证据把它当作当前设备已接受包。
仅请求内 settings.customUserAgent 非空时贯通现有GET（列表/搜索/详情/章节/页）、现有POST、imageRequest。空值/缺省仍用原GET FirefoxUA、原POST AndroidUA、原图片仅Referer，原URI/body/Referer/时限/调用次数不变。沿现有SDK处理JSON quote/backslash和Unicode；不另设UA长度/ASCII格式协议，HTTP头合法性与失败语义仍由现有Host/平台负责。
未调用原dead write_headers、不读取/发送Cookie；不改变原搜索失败POST分支，不以新UA向被拒站点重试。不执行JS或新增身份。

本轮0网络请求；没有可信Happy完整原响应，已知同名readonly目录实际为ComicFury材料，未挪作证据。生产WASM+真实C++Host仅以明确controlled503验证请求头构造/原输出保持。受控GET503会走源码原有POST路径，只为覆盖已有消费者，不代表实站可达或允许挑战后重试。图片探针example.invalid不请求实体。不宣称目录/正文/Native可用，未部署。

R1曾新增自定1024/ASCII限制，审查后R2移除；R1完整源码/包/回放保留但不作为当前候选。无新增站点请求。

0.1.2仅修章节目录完整性：后页HTTP/结构失败整体返回错误；空页仅在原数值isEnd=1时为终态，未终态空页明确错误；保留100页预算，耗尽未终结返回未完成错误。请求、条目解析及资源预算不改。所有新增验证为受控生产WASM输入，不代表真实站点可用或当前原站分页兼容。原R2中POST覆盖表述曾有误；独立POST纠证已确认原GET空body才进入POST，旧误报材料保留。

未部署0.1.2搜索错误修订：仅非空搜索首次GET的FetchError立即按原分类返回，不再因该错误请求备用POST。成功响应、200空正文旧分支与其未知上游合同保持；受控Host验证不代表站点可用，遇真实挑战不执行备用端点。独立树保留原SDK和所有旧冻结物，无新网络或Native部署。
