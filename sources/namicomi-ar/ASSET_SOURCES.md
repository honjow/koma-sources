# 官方图标来源与核查

独立抓取官网 https://namicomi.com/ar，从img[alt="NamiComi"]的data:image/svg+xml内嵌资源解码正式SVG。原件viewBox `0 0 2000 2000`，SVG SHA-256 `174263901610820502049a384355bc1ff5b2f710095490e1401caf108df87a6c`，原件文件`dist/wave-21/evidence/ar/official.svg`。HTML、完整HTTP收据与原件均保留在dist/wave-21/evidence；本语种文件独立核对，与已验收波兰语及韩语SVG仅差末尾换行，全部图形路径、样式和坐标一致，未以旧来源说明代替现场抓取。

以CairoSVG渲染2048×2048，再用Pillow RGBA LANCZOS缩为512×512。完整保留官方2000×2000坐标、蓝色双弧路径、配色及透明边界，无裁切、拉伸、额外底板、圆角蒙版、外框或阴影；图案本身的曲线不是附加遮罩。制作稿assets/icon.svg仅补齐文本末尾换行，其SHA-256为`3e24dbf3df76bada27187a84d969e35eb60c6abd0e5ca9c6a22e1d1e0cd67ee4`；成品icon.png SHA-256 `b6dc49a97b999f965843245379ae26d9b9b28a96fa841eb14b2f052d42144a11`。品牌与素材权利归NamiComi，代码许可证不转授商标所有权。

验收按官方SVG预览→制作稿→实际.koma包内提取PNG依次打开。最终包内图标须与源成品逐字节一致；Python/Pillow生成128/64/32像素、#F4F4F5和#18181B双底复核图，保存于`/Users/honjow/.gemini/antigravity/brain/cecc3fcb-d070-43ba-8d77-3593835b7ffc/scratch/wave21_pkg_icons_review.png`。真实审图记录在dist/wave-21/verification.json；尺寸和哈希仅为辅助。本轮不操作设备，App书源列表中的实际呈现待后续真机QA，不以拼图代替真机验收。
