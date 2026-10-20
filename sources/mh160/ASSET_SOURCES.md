# 漫画160官方图标依据

2026-10-21正常匿名GET https://www.mh160mh.com/ 返回HTTP200，65485B。完整父树mod-top-logo-wr > h1.mod-top-logo > a[title=漫画160][href=/] > img.header-pic-logo明确声明 /template/skin1/logo/logo2.png、alt=漫画160。一次GET该准确URL返回HTTP200 image/png，5685B，280×95；当前icon.png保留完整原件，未裁切、重绘、放大或加底板/阴影/遮罩。素材权利归原权利方。

同主页明确alternate/mobile-agent链接https://m.mh160mh.com/，正常读取HTTP200；其mainlogo声明的220×72 PNG仍为横标。主站另一个134×134 logo2.jpg实为二维码，已排除，不能当品牌图。上述原HTML/图像在Koma既有user-flow-audit的mh160-official-*保留。没有请求旧403搜索、封面或正文。

此次仅补回有当前官网父链的源码标识基线，源版本/业务/能力/SDK、标准注册和App呈现均不改。280×95横标在现有小槽仍可能过小，未作当前设备交付，不将文件存在或原图像素较大称为质量通过；小槽辨识继续OPEN。

2026-10-21独立标识接续调查：先读现存已成功父页原HTML，未找到独立品牌方图或官方账号声明。仅对父页明确声明的 https://www.mh160mh.com/template/skin1/css/head2.css?20260915202656 和 https://m.mh160mh.com/template/wap2/css/style.css?20260915202656 各作一次普通匿名Python urllib.request.urlopen GET；两次均真实返回HTTPError HTTP Error 403: Forbidden。没有cookie、挑战处理、UA切换或重试，不再请求这两条准确URL；没有新素材可采用，原横标/二维码排除及小槽OPEN边界保持。
