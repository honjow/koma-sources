# 王者荣耀世界观漫画 0.1.0

`com.tencent.honorofkings.zh.koma`，`zh`，minimum Koma0.1.1，既有SDK/ABI。只接王者荣耀官方 `pvp.qq.com/ip` 世界观体验站已声明的公开漫画 JSON。源码/包/Wasmtime回放不代替WAMR或197 Native验收；不提交/推送/内置App。

## 原合同和真实范围

官网主JS实际世界观链接→world HTML实际漫画reader href→common.js的 `getCartoonList`→`cartoonlist.json?t=Date.now`；原search与reader把短篇映射dp、连载映射lz，以原btpy_6556标识作品。候选只省略无clock ABI的Date.now缓存参数：`https://pvp.qq.com/zlkdatasys/yuzhouzhan/cartoonlist.json` 一次正常公开GET200，137273字节，与原带t响应逐字节相同。使用此实际候选原响应回放，非旧带t响应冒充精确请求。CURL取证为默认UA/无Cookie；WASM使用现有未改SDK标准UA和Referer `https://pvp.qq.com/ip/`；这些生产header的真实Native网络行为尚未验。

本次原端点包含22短篇+2连载=24唯一原slug、53章节、1047原图片计划；不称全平台，原ngr及英雄/动画/其他列表不请求。作品作者保留zz_69，不能全部改为王者荣耀或腾讯；原sflzwc_5805只将字符串1→completed、0→ongoing，其他unknown。标题/简介/章节标题按原JSON文本解码一次，无重新标题、HTML高亮或slug冒充中文书名。逐书/全站年龄及使用率unknown，源级contentRating unknown。

原客户端 `arrayData` 对单对象包为一章、数组保顺序；章pictures原数组保顺序，字符串原机制允许一张。实际24记录：21单对象作品、3章数组作品；实际53个pictures均数组。候选支持已声明对象/数组章与字符串/字符串数组图，其他形状、重复key、任何转义对象key、错误类型/越界/容量拒绝，不输出残缺计划。原章节号没有独立原数值字段：chapterNumber仅是官网原chapterIndex+1导航映射，ID保0basedordinal；volumeNumber、章节publishedAt/updatedAt均null，不能从作品上线日期猜每章日期。

## 行为和分页

listings为全部/短篇/连载。全部保原22dp后2lz，类内原列表顺序；每页20，以本次完整原列表匹配总行数给真实下一cursor。标题搜索保留原showList三字段 btlz_1638/btpy_6556/mhyxmc_8401（中文标题/原英雄拼音/英雄名），去首尾空白和ASCII大小写，再原顺序过滤；不发额外搜索API，也不混入非漫画内容。未用原reader为“更多漫画”的日期排序替换用户要求的原目录顺序。

## 动态身份与成本

MID含dp/lz+原slug；CID及pageID含明确章ordinal，pageID再含明确0basedpageordinal。每次get_manga/get_chapters/get_pages都GET此一原目录JSON，按实际分类/slug选唯一原记录。get_pages保全部原图片顺序，根绝对HTTPS地址仅是原renderer协议相对src在HTTPS页面的正常解析（原renderer strip http(s) scheme）；原path/filename/query逐字不变，禁止造文件名、批发取图或合并页。

get_image_request每次fresh Module再次1目录JSON GET：严格sourceId（空值亦拒绝）、MID类别/slug、mangaUrl有值时必须为输出的精确作品URL、CID章ordinal、pageordinal及完整URI校对。封面pageId=MID必须匹配该原record真实fmlz_2740，不以正文首图代cover；bodypage必须匹配该真实章与页。Wrongowner/错页/不符query/外部URI均失败。资源前缀只限当前原计划实证game.gtimg.cn/images/yxzj/zlkdatasys/images/image/；前缀不是归属许可，仍每次逐项校对。其他资源域/形状暂失败关闭。

**每个图片解析增加1次完整目录JSON GET**（当前137273B），不隐藏成本，不跨调用/Module缓存，不依赖不存在的cache ABI。只有每操作临时buffers，无内部retry/签名/登录/watchman/凭据。Host/CDN缓存命中与更新鲜度unknown，不能据“fresh Module”声称真实无缓存网络。HTTP/正文/payload各约1MiB、搜索解码512B、每类browse256条、单本1000章、单章10000图；容量不足明确失败，不扩App/SDK限制。原HTTP200及原计划不证明其它作品实体可解码或年龄。

## 官方图标与格式

当前图标沿真实官网iOS下载href→原Tencent MCTCFG.iosURL公布AppStore id989673964→实际Apple301 Location→王者荣耀主产品app-icon方形组件的明确400w WebP source声明取得，保原400×400 WebP字节（icon.webp），出处/精确URL/SHA见 ASSET_SOURCES.md。原图有透明四角，角色/5v5完整；原标自身圆角保持，App显示裁剪独立保留。

历史旧素材记录（当前不再打包）：同组件原400x400JPEG srcset。其页面开发者原名Shenzhen Tencent Tianyou Technology Ltd，description反链pvp.qq.com；保实际法人名称，不推断与TiMi同法人。原JPEG通过授权仅作确定性PNG编码，400×400/480000 RGB字节逐像素相同；原圆角/白角已经是JPEG像素，未加mask/backing/shadow/crop/resize/alpha/重绘。Pillow版本/原JPEG、PNG、RGB字节SHA、pixel-equality回执完整保留。旧197×53全白透明网站logo不代替它，白底无对比失败边界保留；不猜其它PNG路由。

## 有限实际验证

生产交付WASM每操作新Module，146次有限离线回放：24metadata、4种代表作品get_chapters、53get_pages实际1047输出逐一对原JSON；仅26正文image_request（李信20、镜两章及两连载首/末关键章各一）和2种category封面，均1callback。其它1021正文image_request归属未实际回放，不写成全1047逐图通过。14owner/URI等负例、2错误源ID、3未支持操作、1错误cursor、9明确synthetic上游故障，0远程HTTP/新正文。70既有export ABI边界/envelope拒绝、40init/20alloc检查，输出绑定交付WASM。actual实体仍只原李信第一页633912B中文普通营地正文，其他1046实体未取。

原84合同、13白logo、42App图标材料保持；阴阳师R1 accepted反例/R2已冻结包保持，未动App/Kit/旧源WIP。交付和复核材料 `/private/tmp/koma-honorofkings-zh-implementation-20261014`。Root独立审核精确diff与真实输出后负责197普通安装/阅读/下载/cold，Native仍OPEN。
