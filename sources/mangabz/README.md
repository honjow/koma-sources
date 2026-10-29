# Mangabz 0.1.9

独立源 `com.mangabz.koma`，主页 `https://www.mangabz.com`，API 0.2。提供原热门、最新、搜索、详情、目录和页面；既有源码图像/HTTP守卫与内容分级保持，不增加账户、权限、镜像、重试或脚本执行。

## 简繁语言

`showZhHantWebsite`（使用繁体中文）沿固定Keiyoushi `949dea252eb8e867b7b40834a26e13088764ff55` 的同名偏好。支持 `mangabz_language_cookie_v1` 的Host上，false/未配置默认简体 `mangabz_lang=2`，true繁体 `mangabz_lang=1`。默认从旧无语言Cookie改为明确简体，是本次已采用的差异。缺该能力的旧Host不展示此项，继续原无Cookie请求，不把旧验证会话下被覆盖的Cookie头冒设置生效。

源按每次请求的实际设置显式传参，不依赖全局残留状态。语言只加到HTTP(S) `mangabz.com`及其子域：热门/最新、普通和ID搜索、详情、目录、章节图片解析请求，以及相同域的图片描述；外部CDN、仿冒域及含用户信息的URL不带此Cookie。Host只把这一合法语言键值合并到同源验证会话，保原其它Cookie和UA；源不读取会话、不写WebCookieManager、不存明文凭据。

Koma新请求直接消费当前设置，不需要复制参考的重启要求。切换不主动重写库内作品标题、不删章、下载、图像缓存或读态。已保存设置值变化沿既有设置摘要按需重取页表；旧空设置摘要不会因Host新增默认而宣称已自动刷新，既有缓存合同保持。

## 身份与页面

作品和章节ID、原站顺序及真实页数编码保持。目录识别明确话/話/章和完整卷标签；资源cid、末尾页数或未知番外不推断为章节序号。原chapterNumber/volumeNumber、readingGroup、continuationEligible及日期未知语义不随简繁重组。正文仍按原完整公开图像列表解析，不发布截断结果或伪造URL，不拿单张图或构建通过代表整章、下载或整源可用。

## 图标与来源

当前512像素BZ几何重建及App原裁切/顺序不变；原官网符号、可编辑输入与权利说明见 [素材归属](ASSET_SOURCES.md)。原数据合同独立适配自Apache-2.0 Keiyoushi `1319949ca9bc456908196e3ba2d104e94182c5d6`；语言偏好及域限定Cookie消费者依据固定 `949dea252eb8e867b7b40834a26e13088764ff55` 的Mangabz.kt/Preferences.kt和core/network/Cookie.kt。现有许可不授予站点内容所有权。

搜索先解码请求中的 JSON 转义，再按既有站点请求规则编码用户输入；空查询、分页和其他已有行为保持不变。
