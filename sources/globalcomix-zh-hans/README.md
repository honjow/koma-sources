# GlobalComix 中文 0.1.1

源身份 `com.globalcomix.zh-hans.koma`，语言 `zh`，API wire `cn`，版本0.1.1。完整业务继承已审011 owner，保全部API/权限、comic/release/page稳定ID及顺序、章号/语言、原请求头和图片描述符；本次无parser修改。源级MIXED按原合同报告nsfw，不表示所有作品成人。

## 标准构建与当前交付边界

在当前工作区使用 `bash build.sh --source globalcomix-zh-hans`；需要本机已安装wasm32目标、正式koma-source-dev、Python/Pillow及jq/zip。本候选以当前SDK8506离线标准编译；新WASM不同于旧Native包，不能把旧具名验收当本标准包Native通过。旧owner的 `package_candidate.py` 是历史临时工具，不作为当前标准入口，也不引入canonical。

当前包必需根资源由标准build生成manifest `icon: "icon.ico"`，source-owned原ICO及index旁置资源保持原bytes。三语言localizedNames只继承原metadata.json；不将SourceInfo当文件容器。当前源码/包准备为隔离候选，canonical和设备未应用。

## 历史业务证据与未验边界

原四个完整public JSON体覆盖19列表、侯門少爺寵上天cn详情、一条显式free Episode1、两页同owner/同release/未知或付费整次拒绝合同。原011 metadata修正仅通用中文，保wire cn与历史zh-hans suffix；不能承诺简体。旧Native曾具名首图/两页下载和进程保持；原onlineP2超时未关联URL仍OPEN，latest/search其他作品和权限不据此接受。

当前SDK兼容仅复用原JSON体作5操作离线回放，完整输出逐bytes等旧011；原最初HTTPheaders/信封缺失，200信封为明确受控重建，不能冒当前网络成功。标准新WASM与当前App路径尚未Native接受。

## 素材

当前根icon.ico来自官网HTML明确shortcut icon声明，原字节未转码/裁剪/重画。旧96PNG带黑色圆角板，保存在历史owner/010/011旧包，未删除历史证据，不沿用到本候选。官网触屏180PNG仍深实底，不选。来源、HTTP截断边界、SHA与图形身份见ASSET_SOURCES.md。当前小槽适用性仍Native待验；App显示圆角是独立层，本候选不改显示裁剪。
