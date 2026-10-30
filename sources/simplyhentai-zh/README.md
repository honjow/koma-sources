# Simply Hentai 中文 0.1.2

身份 `com.simplyhentai.zh.koma`，API0.2。提供原中文列表、公开中文搜索、详情、单册无编号章节、完整原页序及图片请求。源级NSFW沿原合同保留，不把中文过滤当内容分级。最新、动态过滤、账户/登录、浏览统计与镜像未接入。

## 搜索设置

`blacklist`（Blacklist）默认空，按参考将逗号分隔的标签字符串作为一个query参数传给 `/v3/search/complex`。它只作用于搜索；普通中文列表不消费此值。搜索保中文language过滤及默认Relevance排序，返回真实分页；站点明确的零结果成功返回空列表，不冒权限错误或虚构终点。

## 身份与图片归属

保原数字作品、章和图片ID，及独立传递的 `mangaUrl`。作品路由按站点实际series/slug规范编码一次，不从显示题名猜路由。列表与搜索共用原Chinese Album/type、完整性、重复ID及分页检查。

图片仅接受原 `images.sh-cdn.com`、四段hash/正整数storage目录及合法JPEG/WebP文件名，无query、路径穿越或未知主机。storage目录不等于Album ID的真实API记录已经出现，不能用该伪等式拒绝或重写URL。真实作品归属仍检查API Album ID/type/语言、声明路由、series、页数、页ID/页序，并在图片请求中逐项核对同作品API给出的实际URL；不从调用方URL直接授予归属。

原请求头、传输预算、source-local严格JSON/ABI、容量与完整错误边界保留。普通 4xx/5xx 按既有 `http_error` 与固定 `HTTP status N` 返回；普通 403 不推断验证码或登录，解析和归属错误保原分类。宿主声明 `source_http_challenge_cause_v1` 时，源附带因果请求标记，并仅对宿主明确标记的验证挑战回传该请求标识；旧宿主保持原请求及错误分类。网页会话仅声明实际 API 来源 `https://api.simply-hentai.com`，不假定官网与 API 的 Cookie 共享。状态在每次导出调用复位，不沿用前次请求。未配置、空黑名单不改变原列表；新增JPEG支持修复原当前列表被整批拒绝的缺口，图片原URL保持。搜索wrapper只解包真实object，不丢弃所谓“目录号不等ID”的合法条目。

使用 `bash build.sh --source simplyhentai-zh`。实现及Apache-2.0归属、原官网图标来源见NOTICE.md；本设置/搜索按Keiyoushi固定tree `949dea252eb8e867b7b40834a26e13088764ff55`。真实HTTP、受控WASM回放与普通Native按各自具名结果区分，不据一个结果扩大为全站可用。

当前首页请求仍收到 HTTP 403，验证后原请求未通过；因果接线不代表站点验证或首页可用性已经恢复。
