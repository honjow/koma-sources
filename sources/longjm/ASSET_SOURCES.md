# 龙禁漫包素材

- 官网身份素材：`https://longjm.com/imgs/logo.png`（328×96 RGBA 横排字标，红块内白色“龍”后接“禁漫”）与 `https://longjm.com/imgs/favicon.svg`（120×120，内嵌同身份“龍”图）。2026-11-11 正常 GET 200。
- 页面归属：`https://longjm.com/` 首页 `<img class="logo2" src="/imgs/logo.png" alt="龙禁漫">` 与 `<link rel="shortcut icon" href="https://longjm.com/imgs/favicon.svg">`。
- 打包 `icon.png`：96×96，官网红块“龍”字方形身份图；按用户指定取第一个“龍”字范围、裁成正方形并铺满底色。没有重绘，没有额外底板、外部遮罩、阴影或留白；四边像素均为纯红 `(255,0,0)`。App 显示圆角仍由宿主裁剪，不由素材承担。
- 原文件 SHA-256：logo `183e48ec2cb805a48b46f0c9ddb73c94d4efa6d520e60144aab679d164f4c2a4`；favicon.svg `dec337ea8b4409db243a0ccb427c523207b2806255cfbdb3cddc84765f7a04dd`；打包 icon `aa09c944fd0aced6842bf9b72e806ff3f7997cf679bc476778c627c16822fd53`。
- 视觉验收：已打开原横排 logo、favicon 内嵌图与最终 `icon.png`，在浅、深背景核对；身份一致，无第三方标识或占位图。候选包构建后从 `.koma` 内提取实际图标复看。
- 用途仅限本源的品牌身份图标，不作为漫画封面或替其他源复用。
