# 漫画屋 0.1.8

源身份 `com.manhuawu.koma`，主页 `https://www.mhua5.com/`；参考 Keiyoushi 固定 revision `10f06eb833af5990bc5470c84c5e93edc159ad4e` 的 Manhuawu / MCCMSWeb / KeiSource，Apache-2.0。源级分级为 SAFE，单部作品的内容分级未知。标准构建：`bash build.sh --source manhuawu`，最低宿主版本 0.1.0。

## 身份和目录

新公开作品沿用 `mh:<实际 slug>:0`；已有 `mh:<slug>:<数字 PICID>` 按原样回传输入 ID，并按 slug 获取当前详情，不据此声称全部历史非零 PICID 已端到端验证。章节 ID `ch:<原始完整 href>` 原样保留 `/index.php`，页面 ID 使用 `p:<index>`；不迁移或合并旧身份。目录输出声明 `chapterOrder: newest_first`，只从明确的“第 N 话／話／回／章”提取章号，不将数组位置伪造成章号。

列表及搜索沿用原路由与分页：根据 `current`、`end`、`next` 判断；末页须 current 等于 end 且 next 自指或缺失，非末页的 next 必须指向下一页。搜索第一页的规范地址可以省略 `/1`，但不能接受其他查询的分页地址或无效页数。

## 请求和验证

普通 HTML 与图片请求沿用 SDK 的 Firefox 121 User-Agent，不添加 Referer 或 Origin。请求地址、超时、图片身份和页序保持原合同；不引入镜像、源内账号／Cookie 管理、验证绕过、JS／图片解密或自动重试。解析、容量和 HTTP 错误保持明确分类。

仅当宿主明确返回 `accessSignal=challenge_required` 且 HTTP 状态为 401、403 或 503 时返回挑战；普通 403 不推断验证。宿主声明 `source_http_challenge_cause_v1` 时附带因果请求标记并返回对应标识，旧宿主保留原请求；验证窗口仅允许 `https://www.mhua5.com`，由宿主处理原请求回流。

当前首页请求仍返回 HTTP 403，站点安全验证未通过，尚未取得首页列表；因果接线不代表站点可用性已经恢复。
