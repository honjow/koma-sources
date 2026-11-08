# 禁漫天堂 JM（未发布候选）

独立 ID `com.jmcomic.koma`，不是 `com.hanman18.koma`，不迁移或改写后者历史。
本源独立 workspace，不加入生产 members/build registry/index。

采用用户指定 `keiyoushi/extensions/repo/index.pb` 中 source `6286738698187452081`（禁漫天堂、zh、18comic.vip），扩展 `1.6.64`。
参考固定为 Keiyoushi extensions-source `de743aed846da53e00c45ab26ee0e77f7bb4355b` 的完整 5 Kotlin 文件和 build.gradle；相关 core HTTP/browser headers、限速、加解密也固定于同一提交。许可证全文和原图标来源随源码保留。

匿名 API 的热门、最新、ID/URL/文字/筛选搜索、动态分类和标签、详情、完整分集、原始页表均在源内实现；HTTP 传输和 HTTP200正文传输失败先保留宿主错误，再解析 API。原 API 的未知作品空数组与明确已移除章节空数组分别返回作品/章节不可用；未知格式和数据库服务明文错误不猜成下架、付费或登录。

标题只做参考 Parser.unescapeEntities 的实体解码，保字面标签和连续空白；原 description 的作品/人物列表、连载标识、作者、目录原序及单章节规则保留。多分集按参考逐项标题后逆序；章 ID 包含作品和章节的原始身份，页 ID 加原始位置，重复图片位置不合并；get_pages 重新核完整所属目录，不猜 CID。get_image_request严格核已发PID的album/chapter/index格式与真实URI的/media/photos/chapter/归属，坏PID或跨章URI无HTTP拒绝，不限制CDN主机。

必要宿主适配及仍未完成的边界：

- WASM 每操作独立实例，使用既有 source_private_session_v1 的安全时钟签名；候选暂在该缓存保存非敏感 apiHost，网络传输失败按代失效。参考 apiHost 仅进程内 volatile；候选跨 App 重启保留直到失败的差异已由 Root 允许为必要适配，未称完全同生命周期。
- API 普通请求使用 SDK 已采用的固定 Firefox121 UA，并保参考的所选镜像 Referer、跨站 XHR Origin/Fetch 字段。参考 Android 的全局 UA、语言和 HTTP 线序由它的宿主提供；未假称字节级相同。没有账号、Cookie 或权限切换取通。
- 包必须声明 `requiredImageDecodes:["jm_stripes_v1"]`，由标准metadata/registry/安装身份透传，避免旧raw缓存缺Page hint后漏掉必需解扰；这与每页hint/专属feature双门控并存。V3仅新增此包声明，同V2 WASM，不新增网络。
- 必需图片条带还原不能借可关闭的图片插件代替。当前 Root已采用 `image.requiredImageDecode:{kind:"jm_stripes_v1",rows:N}` 与专属 feature `source_required_image_decode_jm_stripes_v1` 已定案，尚待 Host 实现；旧 Host 缺能力时整次页表失败，不回退乱码。get_image_request 也保提示。GIF 透传必须由 Host 实际字节判断，不按扩展猜；正文原字节保留，解扰派生图单独缓存。当前本地独立 Pillow 还原不是 Native/237 已验。
- 参考 API host 3 次/2秒共享窗口；当前本地 Node 验证桥真实执行，不是单纯声明能力。Root已采用 source_rate_limit_jm_v1：仅本源HTTPS且同时有Token/Tokenparam的API请求按精确origin共用3/2000ms；server-list与图片无签名不入窗，未知新Server也受窗限制。源所有真实API上下文先要求此feature，缺失0RPC/0HTTP；Host真实实现/237仍待核，不能把旧 generic rate feature当支持。
- 参考收藏夹、自动/立即签到需可选账号，普通浏览阅读不需登录。完整安全账号输入、AVS cookie、登录/401同代恢复和签到副作用须沿待定公共认证/普通私有 Cookie 合同实现。当前候选仅明确提示收藏需登录，未把缺私密输入藏进普通 settings；这部分尚未移植完成，不能据此发布完整源。
- related 漫画没有当前 ABI 操作，未伪造为首页；home=false。

验证事实、失败体、分层分母和候选精确 SHA 位于 Koma 本地 `new-source-ports/jm-web`，未据构建或 API 页表成功宣称 App 原生正文完成。
