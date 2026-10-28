# 綜合口味・生活小漫畫

來源為作者原站生活小漫畫分類；**整篇排除同時有原「開箱心得」或「食」category URI的文章**。這是收錄範圍，不是說8481沒有漫畫，亦不剪其前兩圖。依原next逐頁瀏覽，不掃全站、不從97標籤數推已證作品總量/終態。兩原頁raw20篇，範圍保留9+10=19篇；其餘頁未知。每post獨立作品與完整閱讀單元，正式chapter/volume null、readingGroup=MID、continuationEligible=false。原author資料為mixflavor，原/about明確筆名綜合口味為小莓與大軒共用；作品authors保綜合口味，不猜逐篇分工。評級/連載狀態/usage unknown。

sourceID=`com.mixflavor.life.zh-hant.koma`，0.1.2，ABI0.2，zh-Hant；能力為search/listings/mangaList/detail/chapters/pages/imageRequest。無home/filter/settings/credential能力。

身份：`mixflavor:zh-hant:post:<原postID字符串>:<原文章path UTF8字節hex>`；CID/PID用read/page前綴，PID另加DOM ordinal。11152原path `/2023/11/caught.html` 編為`2f323032332f31312f6361756768742e68746d6c`。不經f64/wasm32 usize處理postID，不按數字造URI/猜WP API。path由原card href取得，保原大小寫/百分號，例Phone.html、Pantsu.html、circleLenses.html。ID自含路由，App未附mangaUrl仍可解析；若附mangaId/mangaUrl則必須一致。每次正文GET後核body獨立postid token、canonical、原Article/WebPage/author及原分類。path僅接受已核`/YYYY/MM/<ASCII/合法percent filename>.html`形狀，拒外域/遍歷/query/fragment；原URI字面日期不編章序。未來canonical遷移與新路徑格式未知，不造映射。

目錄嚴格取主masonry direct post父層，逐項原post class/token、h3/title href、cover href、原categories/author/date配對，按post去重。原cover URI完整query不改。範圍排除不靠標題、圖片數或副檔名；原category link均須完整rel/href，不能缺失排除字段後悄悄混入。分頁核原current/back/next href，next完整URI作cursor；WordPress page2 canonical本來含/page/2，不套PIXNET基礎canonical。原分類頁無og:url，以canonical及CollectionPage識別；詳情有og:url，按原聲明再驗。

詳情僅主`eskimo-page-content > div[itemprop=articleBody]`，不混main其它區域/推薦/footer。每img須有src/srcset且位於原p內，所有主Body圖片與選中數須一致，拒iframe/video/script/picture或未知圖片樹，不以JSON-LD image1代全篇。srcset逐個校驗原作者CDN、同一原媒體path及唯一正數w描述符，選實際最大w的原完整URI；不擅加w、改query、去參數、換host、猜上游或轉碼。原src可能是blog.mixflavor.com/wp-content/uploads的1200變體，而原srcset是img.mixflavor.com的CDN變體，兩者分開保證據，不宣稱同字節或未代理原始文件。

首《你的事，我都知道。》原兩張PNG完整保DOM順序，原標誌/字樣/尾格不裁；原2150×2678與2150×1402實體已於調查完整目檢。原主Body文字附文與精確datePublished進description；Reader頁仍為原圖，不聲稱重排原HTML文字。8513原5圖/敘事alt與附文合同及生產Host頁描述符已核，但只有首圖實體取過，其餘4未知。8481整篇返回out_of_scope，不把已核的漫畫開場默剪成單獨作品。

圖片角色：MID封面0 metadata GET，限定`https://i0.wp.com/img.mixflavor.com/YYYY/MM/<10digit timestamp>-<16..64hex>.(png|jpg|jpeg)`及原w/fit/resize+ssl=1 query語法；原樣URL、標準Firefox UA、身份內原post Referer。封面信任此前catalog/detail配對，**不是獨立重新驗證post→cover映射**，不把其它同作者合法URI的可接受邊界誤稱正文fail-closed。PID正文每次1 GET重核身份內真文章/範圍/當前主DOM最大srcset URI與ordinal，未列或錯序的同作者URI也拒絕。無mangaUrl正向已回放，提供錯owner明確拒絕；原圖實體HTTP由App另取。

成本：listings0、browse每頁1 GET（不自動跨頁補足被排項）、detail/chapters/pages各1 GET、MID封面0、PID正文計畫1。前置非法ID/host/query/owner0 GET；語法正確但不屬當前主DOM或post/path錯配需1 GET後拒絕。HTML<1MiB、每頁≤24post、每篇≤64圖、每圖≤16 srcset候選/寬高≤16384、JSON深度≤64。無跨操作狀態快取、登入、JS、憑據、重試或延時，不改App時限。

图标更新（0.1.1，2026-10-20）：官网 about 页面 https://blog.mixflavor.com/about 明确链接 https://fb.com/blog.mixflavor ，后者正常重定向官方 Facebook 账号并声明本次原头像。采用完整 720×720 JPEG（44295 字节），两作者与猫的原画、原蓝白背景保持；未裁切、重绘或新增底板、阴影、遮罩，App 图标容器样式不改。准确原图：https://scontent-tpe5-1.xx.fbcdn.net/v/t1.6435-1/128738156_225811845567752_1338166142110710132_n.jpg?stp=dst-jpg_tt6&cstp=mx1000x1000&ctp=s720x720&_nc_cat=102&ccb=1-7&_nc_sid=3ab345&_nc_ohc=KOqFOsf0Bk0Q7kNvwFTqiMK&_nc_oc=Adoc2dhhvS7hvN9rUFOTK027GQK9I-I_-HiCQQj8fMCDY8P8TeIJzXgcTVriVd3iC90&_nc_zt=24&_nc_ht=scontent-tpe5-1.xx&_nc_gid=gZ19FISx9qNJnaHT7pKZyA&_nc_ss=7920f&oh=00_AQOVPpLYfKEVN2SqKWZaFo0GmgFZ9Lepa7o15dpJTFOHcA&oe=6AFE6004 。当前仅完成资源接入，设备呈现另验；不冒额外商业分发许可。

历史 0.1.0 的缺图原因：原head rel icon32x32 CDN URI按聲明一次讀取；原32×32 P模式PNG201B，SHA`21b95b15612f8dde9564b625085ce1b1dde31b04e66ef0c078a849c9fda48831`完整保留原證據。但IHDR=bitDepth4/colorType3，現有包圖標validator只接受8-bit palette或8/16-bit direct，實際guard已拒絕。故本次沿既有optional無icon包；不轉碼/升位深/冒名、不改validator、不新抓192圖。沒有本地裁/縮/轉/遮罩/加底，亦不從此icon guard推正文PNG解碼結論。這只是原官網聲明CDN變體，不冒稱上游源文件。原/about分享/商業條款完整留證，本地正常公開讀取與功能實現不等於對外轉載或商業發佈；本次不對外發佈、不聲稱額外分發許可，包不含漫畫圖。

僅使用凍結原兩頁、11152、8513、8481與現有production Cpp HtmlHost+Lexbor/Wasmtime作有限回放：30操作/真ABI，18 metadata callbacks=15原HTTP回放+3明確synthetic malformed body。原20/範圍19、case/URI/next、首2正文計畫/附文、8513五描述符、8481整篇拒絕及有限身份反例成立；descriptor末0指資源關閉不是0圖片。除調查已取的首2實體及8513/8481各首圖外，未補其它實體/頁/作品。初期錯要求分類og:url的probe原失敗保留，依完整原head修正；不合成成功原體。Host與構建不等於Native/NetworkKit成功，197普通導入、原分頁、正文、下載/cold待Root具名驗收。

base `d5ee46d9821c40c2931161517d27027671e6f0c9`；獨立clone `/private/tmp/Koma-sources-mixflavor-life-zh-hant-20261015`，證據 `/private/tmp/koma-mixflavor-life-zh-hant-implementation-20261015`。原54+32+14=100合同、舊源與主WIP保護。無App/Kit/SDK/共享runner原件、style、設備、commit/push/release操作。


## 237 0.1.1 首篇普通正文（2026-10-23）

普通系统选择器首次导入正式0.1.1，登记62→63仅新增本源，原62项完整记录及相对顺序保持。普通Browse分类9本→11152《你的事，我都知道。》1章→阅读，完整繁中彩色叙事原图1/2及普通右侧翻页2/2均实际显示，无加载层；自然Back后会话绑定原post/path，pageIndex=1、totalPages=2、completed=true，其他原阅读记录完整保持。只接受本篇两图、自然返回及读态，不外推其他文章、全分页、下载/cold或商业分发许可。

原件沿Koma既有237 category-membership的`mixflavor011-native237-firstpage-001/screen.png`、`mixflavor011-native237-page2-return-001/{page2.png,reader.json}`及`mixflavor011-native237-{browse-restored,final-restored}-001`。最终普通Browse喵趣完整锚点、63项登记保持、10000ms/AWAKE、本人传包及两scratch不存在均读回，租约已释放，哈希关闭；未改源/App/图标/版本、偏好或新增下载。

## 分類內搜尋（0.1.2）

搜尋使用原生活小漫畫分類的 `?s=` 及站點實際分頁，保留查詢經 URL 編碼；每個卡片仍核原身份、作者、分類及排除「開箱心得」「食」。搜尋頁與瀏覽頁不同：原 CollectionPage 保留分類及頁身份，但搜尋沒有 canonical；單頁與空結果也不一定有 pagination。故搜尋另核原查詢回顯、結果標題、原 current/前後頁和查詢歸屬，空頁僅在原站明示無結果時返回空；瀏覽原 canonical、current 和非空校驗保持不變。搜尋 cursor 為實際原下一頁數字，不從結果數猜下一頁。

以實際保存原 HTML、production WASM 和原 C++ Lexbor Host 回放：`漫畫` 首頁10條/next 2、第二頁4條/終點；`食` 原6條排除同時標禁類的8481，保留5條；`你的事` 與帶引號查詢均為原11152；`開箱` 與 `漫畫 & 食` 為空。舊0.1.1與候選在同一原瀏覽首頁及第2頁 data 全字段相等。桌面 dev runner 實時 `漫畫` 兩頁與 `食` 正常；這些不是設備 Native 證據。
