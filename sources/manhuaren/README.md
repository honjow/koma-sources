# 漫画人 0.1.1 当前边界

当前稳定身份为 `com.manhuaren.koma`，SourceInfo 0.1.1/API0.2。043a3dd 保留原公开搜索/列表、游标和错误分类成果；既有匿名鉴权及受保护详情/目录/正文实现没有在本轮修改。当前此前没有本源 README，本文件仅记录这次实际交付与验证，不冒称恢复了原历史文档。

## 237 普通安装与独立搜索（2026-10-21）

在原主仓用既有 Rustup 工具链和标准 WASM 参数执行 offline/locked 构建，真实 WASM SourceInfo 为 0.1.1。使用原正式图标与 metadata 生成三成员正式包，经普通系统 picker 导入成功。237 原50项 sourceId/enabled 相对顺序逐项保持，正常新增漫画人后为51项；没有更改 App 容器、布局或源业务。

普通热门入口实际执行 get_manga_list，14:24:14.322 HTTP200 后于14:24:14.397返回 permission_denied。当前源码仅在上游 errorResponse.code=40011 时将此结果映射为“source requires an anonymous session”；没有重试该热门请求，也没有据此判定独立搜索失败。

从源包管理动作单直接进入独立 SourceSearch，未重进热门。原 Host 具名词“一拳超人”只输入一次，经已有自动提交后，14:34:49.109执行 search，14:34:49.269 HTTP200。原生结果实际显示《一拳超人》《一拳超人同人：琦玉VS龙卷》《一拳巫女》等标题与封面；收起键盘后至少六个封面/标题可见。只接受这个词的首批搜索及可见封面，不声明全部20项、分页、其他词、详情、章节、正文、下载或cold通过。

未点击作品、未创建匿名账户、未调用 createAnonyUser2 POST。原详情/目录/正文仍会调用 ensure_auth，这一分支保持未验；既有静态匿名协议未被改写或绕过。

全部原件在 Koma/.hvigor/outputs/user-flow-audit/device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership/：manhuaren011-imported237-001、manhuaren011-public-list237-001、manhuaren011-public-search237-001、manhuaren011-search-visible237-001。搜索会话最终恢复证据为 manhuaren011-search-session-restored237-001/run-metadata.json：普通返回原 Browse 末端，reader.json 与 Root 原 comicabc010-root-session-restored237-004 逐字节相同，原50项顺序/启用保持，新源保留；AWAKE/OverrideTimeout=10000ms 读回后释放 d57d478a。前一安装会话恢复回执为 manhuaren011-session-restored237-001，acb7d798 已释放；仅本次上传包被移除，所有设备回执哈希关闭，没有清用户数据或缓存。
