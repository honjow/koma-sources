# 喜漫漫画

- 上游在 Gradle 声明 .com、.xyz、.net、.cc 四个站点，第一项为默认，域名由用户手动选择。源设置使用 `preferredMirror`。列表、搜索、详情、章节、作品网页、图片请求和网页登录按所选站点生成地址，已保存的作品和章节标识仍使用原路径。站点页面直接给出的独立 CDN 地址保持原样。
- 排行、最近更新、类型及标签筛选、作品和目录沿用指定上游。封面和正文遇到官网 `encrypted-image` 标记时调用随包字节处理器。购买、金币和游客额度沿用原站状态。
- 遇到需要登录的章节，候选使用 Koma 的同源网页会话。上游另支持保存账号、按 `loginName/password` 提交表单，并在成功后重读本章。本次未取得官网有效登录响应，原生登录所需的实际会话 Cookie 尚未核对。
- 打包后的 WASM 已对照四组域名请求及旧标识。237 实际导入后确认四个选项、保存 .cc 及网页入口跳到 `favcomic.cc/lander`。.com 和 .cc 的真实目录均未加载，另外两个域名在本机同样返回域名停放页。正式候选最低要求 Koma 1.5.5。

[上游固定模块](https://github.com/keiyoushi/extensions-source/tree/f1d882a93ef34a33dc4f4467753ad5f99eab43f4/src/zh/favcomic)
