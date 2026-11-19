# 优酷漫画 / Manhuadui 0.1.4

稳定源身份com.manhuadui.koma；目录与标准构建名manhuadui，显示名优酷漫画，不创建youku别名。完整业务继承b994ad0380df8f7da7e185c98dc093a233502883（前序ce96f8e）；当前SDK保持canonical原字节。源码/Cargo版本均0.1.4；三语言metadata继承原012。

仅声明search/listings/mangaList；detail/chapters/pages/imageRequest不可用，不用未接受正文实现声称可读。保原热门栏目、operation、对象cursor/legacy page、真实next控件终态与有界固定诊断，401/403明确permission_denied。不更换原UA/站点，不绕过挑战或重试已拒路由。NSFW=true继承原包与sourceInfo，不声称所有单作品评级已知。

标准入口 bash build.sh --source manhuadui，minAppVersion0.1.0与原012 index相同。原0.1.2补齐必需官网原PNG；本次icon.png为下述独立标识高清还原，出处见ASSET_SOURCES.md，App圆角及槽位保持。标准构建使用当前SDK/正式CLI/既有flags，新WASM与旧Native647a0332不同，不能冒旧二进制复现或当前Native通过。

历史197具名012只接受一拳超人搜索两作品/真实封面/终态与不清数据cold身份；默认列表当次403保持，详情正文不接受。本次current SDK离线重建适配仅info/listings0HTTP和两原HTML输出，非现站可用证明，page2原体缺失不补造。原回执：Koma .hermes-artifacts/20261005-production/src010-manhuadui-20261010/diagnostic-012-native-acceptance-receipt.json。上述为原0.1.2业务边界；当前0.1.3图标的设备结果见下段。

当前沿用独立蓝圆标识的512px高清还原资源，业务与能力边界见上文。它不是官网高清原件；完整身份依据、重绘差异及待验边界见ASSET_SOURCES.md。上文原PNG/旧包/历史原图描述保留为历史，不作为当前资源未处理或内容已通过的结论。

0.1.3在237 VDE-AL00展开1320×2120经普通系统选择器更新成功，Root亲看管理及Browse两处独立蓝圆标识清楚，无可见彩边或长影；45源ID顺序、原启用和索引保持。原图在Koma user-flow-audit/device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership/ykmh013-manager237-001及ykmh013-browse237-002。只接受这两处图标，未访问该源列表或正文，原403与内容能力边界保持。

搜索先解码请求中的 JSON 转义，再按既有站点请求规则编码用户输入；默认、空查询与分页规则保持原行为。

## 0.1.6 候选：WebView 请求桥接

按用户反馈普通 WebView 可访问、普通网络请求被拒的现象，声明宿主已有的同源 GET WebView fetch 桥接；最低 Koma 1.4.2。当前只开放列表和搜索，详情、章节与阅读能力未开放。候选尚未设备验证；是否能在用户网络环境下实际读取列表和搜索、封面能否显示，以及需要的会话行为均未确认，不能视为修复完成或发布。

2026-11-12：237 读回 0.1.6 已安装。热门列表及封面可见，但未排除缓存；搜索真实关键词失败，日志显示桥接超时，原生请求返回 520/403。同设备打开并刷新 WebView，均显示 Cloudflare 520。用户此前报告 WebView 正常，该差异按本次具体时段保留；候选未通过、未发布。

## 0.1.7 候选

原生热门请求取得带 Cloudflare 挑战标记的 403。当前源依宿主的
`accessSignal=challenge_required` 显示验证入口，并声明精确的官网验证 origin；
普通 401／403 仍报站点拒绝。已授权的同源 GET 继续走现有 WebView 桥接。
本地响应证据不代表验证后热门或搜索已经可用。

237 普通浏览已弹出验证页，验证后原请求网页出现实际漫画列表。候选复用现有页面回流，
以解析器所需的漫画卡片判断浏览网页已可用，再交给当前待处理请求。
237 日志证实验证后的网页跳到另一个 origin，原精确请求不能接收该文档。
热门和搜索从头请求官网移动站，沿用核验过的路径和查询参数。作品身份与对外地址继续使用原规则。
237 实际热门卡片用 `li.list-comic a.txtA` 与 `a.ImgA`，搜索用 `div.itemBox .itemTxt > a.title`。
原选择器无法读出作品，本次补上两种结构，保留旧页面解析和作品身份。

2026-11-19 0.1.7 237 已具名验证热门首屏、《一拳超人》搜索和验证页自动关闭，作品出现在应用原生搜索页。
