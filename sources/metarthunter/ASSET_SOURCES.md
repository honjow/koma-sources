# metarthunter 官网素材与验收

官网原件：https://cdn.metarthunter.com/wp-content/themes/masonry/images/favicons/apple-touch-icon.png ，180×180 RGBA PNG，SHA256 `030907864b7af1045ba9bb2ff5f4ccb533bd09241bdc26c6f18f2a5f298d9ce7`。原件为带透明圆角/圆形遮罩的官方白色字母标。

以原图内部均匀底色RGB(36, 37, 39)恢复完整正方形边界，去除透明圆形遮罩及圆弧边缘；白色字母及其抗锯齿像素保持原值，不缩放、不重绘、不加额外底板或阴影。最终180×180 RGBA PNG，SHA256 `a67b674de42edbed5fb2b11eb2ff151010cab0bb67620535de953e06cff2185e`。原件与处理稿均留在dist/wave-31/evidence，主代理依次实图核对，再从最终.koma解出PNG在#F4F4F5与#18181B双底及64/32px下复核。证据为scratch/wave31_pkg_icons_review.png。

品牌权利归原权利人，代码Apache-2.0不转授图标或商标权利。本轮未操作设备，App书源列表呈现仍待用户真机QA。

复审去除了原透明圆形遮罩的抗锯齿边缘残留，中心字母及其原有抗锯齿像素完整保留，外围仅使用已核实的均匀品牌填色；未按圆形蒙版输出。
