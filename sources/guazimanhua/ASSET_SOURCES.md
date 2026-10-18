恢复审计补记（2026-10-19）：下方保留原候选说明全文。其“未部署/Native未接受”是后续g08之前的状态，已由原 /private/tmp/koma-guazi-icon-native237-20261017/root-review.json 与 native-receipt.json 取代：0.1.2包c80f930d及图标e379a353精确部署，237普通列表的具名32vp图标观察接受。此处只恢复已有CSS资产及其来源，不新增正文、下载、冷启动或当前设备验收；全线图标质量仍OPEN。

# 当前官网CSS身份标（候选未部署）

首页 https://www.guazimanhua.com/ 的真实 `a.logo` 文字为“瓜子漫画”。其明确最后引用 https://www.guazimanhua.com/assets/common/mobile-shared-nav.css?v=20260825stickyad，在 `@media (max-width:640px)` 内的 `body .header .logo::before` / `.page .header .logo::before` / `.page-shell .header .logo::before` 声明独立红种子图形。前一个mobile-home-a.css同样声明该形，并非绿色默认favicon。

icon.png 是原CSS consumer计算样式的直接离线Chromium栅格化，不是网站提供的原PNG，也不是手绘/生成模型重绘。保17×17、四角50%/50%/50%/4px、#f0043c、inset6px白形、rotate(-25deg) translateY(2px)及中心不变；只在26×26透明画布留足完整旋转边界，以8×设备像素倍率栅格为208×208。白内形是原标志组成，不是外部底板或额外阴影。未添加外mask/投影/新背景，App显示圆角独立保留。

现存237旧图144×144/SHA bc96f8a58321e064aa71b7faa42a7348b5c4d7a7602653215d7ec392f984768d写“漫画社 MANHUASHE”，与官网瓜子身份不对应，旧图/物理包保持原证据。此处不推断那张旧图具体原来源链。

渲染器、三个原CSS/headers/URL、computed consumer与receipt保留在 /private/tmp/koma-guazi-mh160-icon-current-followup-20261017/。`icon-source.css` 是最后原CSS全文件；不在标准打包时重新联网或重新生成图。最终视觉适用性/Native未接受，先交Root审。

当前打包素材SHA256：`e379a353752a8c303b66e4928405f8d430b775bfcb267bac4f1a42c16de1ba96`。标准build仅复制存储资源，不保证重编WASM字节等于冻结旧包；当前图标候选保旧WASM精确，正式编译交付需独立验收。
