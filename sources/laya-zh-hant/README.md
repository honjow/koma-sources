# 醫院也瘋狂（繁中）0.1.1

來源為作者品牌官網 `https://www.laya.url.tw/comic/` 精選漫畫藝廊，與 `https://www.laya.url.tw/hospital/` 真實雙向連結。原作者頁明確分工：雷亞（林子堯）編寫故事腳本、兩元（梁德垣）繪製漫畫。SourceInfo.author 的 Koma 是源開發者。源不使用官網粉絲數作使用率、不把官網14/15冊的不同宣傳文字合併為全冊合同；正式評級、狀態、usage 未知。

## 真實目錄與閱讀

原HTML內 `mockFiles` 明確列80個文件，`predefinedTitles` 有80鍵、79不同題；`ch-0019.jpg` 與 `ch-0063.jpg` 同題「萬聖節裝扮」，必須保留不同filename身份。每次列表／詳情／章／頁／圖片計畫均只GET同一真實藝廊HTML，經當前生產Cpp HtmlHost檢查唯一canonical、原author，再取唯一 `body > script:not([src])` 原文字。有界源內掃描顯式單引號陣列／對應標題物件，不執行JS、不猜連號、不內嵌80標題或原圖許可表。要求鍵一一對應、無重複filename/鍵、合法UTF8、最多256條且標題各不超512字節；未支援跳脫語法／格式改動顯式拒絕，沒有推測回退。按原頁number排序邏輯保留實際filename數字的目錄展示順序，不據此填正式章號。

原 `INITIAL_LOAD_COUNT=12`、`LOAD_MORE_COUNT=24`，源按同一現行陣列本地切片為12/24/24/20四批；不是四個遠端目錄HTTP，也不硬編碼80為永久數量。後續修改在已核字面量／256條容量內重新計算終態；過量或原關係改動明確失敗。回應 `hasNextPage` 與 `page.nextCursor/hasMore`，cursor為實際下一批數字字串；非法游標／超容量批次網路前拒絕，現行第5批需讀原HTML後確證超出終態。支持listings/manga_list/detail/chapters/pages/image_request；無真實search/home/filter/settings/credentials合同，明確不支援。

原 `createComicItem` 點擊綁 `openModal(allComics.indexOf(comic))`，`showInModal` 使用同一comic.title與comic.path，完整正文 `data/${filename}`；源驗證這些原關係與切片常數仍存在。沒有獨立詳情HTTP路由：所有作品url／links均保留真canonical藝廊，不造anchor、UUID或不存在頁。作品身份為 `laya:zh-hant:comic:ch-0001.jpg`；章前綴 `layaread:`，頁前綴 `layapage:` 加 `:0`。每則一個原完整圖片閱讀單元，chapterNumber/volumeNumber=null、readingGroup=本作品MID、continuationEligible=false。這個單圖計畫來自原每則一個comic.path的viewer合同，其他79實體HTTP/解碼仍未驗，不把首四格外觀推到所有作品，也不裁格為頁。

## 圖片與身份邊界

get_image_request只接受精確 `https://www.laya.url.tw/comic/data/` 下當前顯式filename；缺pageId可作封面、MID必須是同filename、正文pageID必須同filename且ordinal=0。封面與正文原站本來使用同一完整圖片，身份分支分別核對。獨立mangaId（若有）必須本作品，mangaUrl（若有）必須真canonical；null、空錯ID、額外query/fragment、越目錄、跨host/協議、未知當前filename均拒絕。公開原URI與藝廊Referer原樣輸出，並明確輸出與SDK metadata既有標準相同的User-Agent，不下載或包內分發漫畫，不使用原JS可選重試query、IndexedDB/blob、任意外部URI或共享快取。

## 無圖標包與原素材

原head `ico.ico` 雖MIME為image/x-icon，實際是64×64 RGB JPEG，13240字節，SHA256 `c4c3070f9e7322e5dc1e2bc24a2d177da237f80addce2f462f1c3ac85242d4af`，不能冒作icon.png。原品牌Organization.logo明確聲明 `https://www.laya.url.tw/menu-2.png`，實際1209×319 RGBA PNG／51165字節，SHA256 `8e7cf7e86a82b6536168ff7a761c96a5af8f4d95fb9b0c152f3d1a0ca5ace01c`；寬度超現有1024校驗上限，白字透明背景小槽可見性亦未知。兩份原素材完整保留在獨立證據，不縮放／裁切／轉碼／改字節／加底／遮罩，不改validator/App；本包按現有可選契約不含icon.png，index不宣稱icon。

## 回放與未驗收

原4GET（藝廊／首完整JPEG／原favicon／品牌父頁）共667098響應body字節，原logo addon另1GET51165字節；原件和addon各自舊binding不變。實施新增live請求0。首「最常被問」完整JPEG1247×3363 RGB／535730字節，SHA256 `e6dd74d83fce4c24b8aee5f1dcb6f2243cb0c8875addfcc4b991da94297e5348`，已本機目檢；未取其餘79原圖。

0.1.0的447操作回放與原Cpp/Lexbor指紋按原凍結保持；不以舊WASM回放稱本候選已完整重驗。0.1.1只用同一未改生產Cpp HtmlHost實作及原HTML/status/headers，檢查App真正不帶mangaId的封面MID、正文PID兩個正向image_request，以及一個已有非法mangaId owner拒絕。兩個成功結果除新增User-Agent外與0.1.0輸出原字段相同；非法owner網路前拒絕，每操作descriptor歸零。沒有重跑447或新增live請求。

每次有效業務操作1個metadata GET、51139原HTML字節，get_listings和不支持操作0HTTP；非法URI/ID/owner在HTTP前拒絕，未知合法filename/現行終態要讀1GET方可判定。回放不發原圖HTTP。metadata SDK正常UA/Referer形狀在離線橋核對，原公開取證UA為Mac Firefox且無Referer，不宣稱197已接受。Native/WAMR、其餘79圖、實際下載/保存/cold、速度與未來網頁變更均未知；Root全差異審查後串行普通導入驗收。本次未改App/Kit/SDK/主源WIP/舊凍結物，未操作設備/IDE/簽名，未提交／推送／發布。


## 0.1.1 最小請求頭一致性候選

Root在197觀察0.1.0封面與首正文原圖下載為HTTP500/537字節；目錄metadata為200/51139字節，WASM圖片計畫已成功，因此不是已證實的pageId/owner拒絕。Root另一次正常Host GET使用原Referer及既有Mac Firefox UA取得535730原JPEG字節且SHA精確。這只能支持「圖片請求頭差異需實機驗證」假設，不能證明500原因或本候選已修好；Mac UA與SDK Windows UA亦不是同一字串。

0.1.1僅讓imageRequest.headers同時明確帶出 `User-Agent: Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0`，其字節與現有SDK build_get_request metadata預設UA一致；原Referer不變。URI/ID/目錄解析、80現行條目、切片、正文、capabilities、無icon合同及所有拒絕邊界均不變。source/crate/manifest/index版本步進0.1.1，最低App版本註冊原已0.1.1不變。不使用任意自造身份、憑據或挑戰繞過，不改App/SDK/HTTP guard。Root後續197一次對應普通圖片路徑驗證之前，本候選未Native驗收；舊0.1.0完整樹/包/原失敗保持。

## 当前图标格式合同补齐（2026-10-20）

0.1.1采用官网艺廊明确声明的64×64 JPEG favicon，仍有低清限制。0.1.2只替换为新核实的官方认可品牌原件：`https://www.laya.url.tw/hospital/` 导航与Organization.sameAs明确指向 `https://www.facebook.com/Crazyospital`；该页og:title与og:image:alt均为「醫院也瘋狂」，og:image声明图片ID `317259963_695807238568682_7194057852934992248_n.jpg`。Root实读父链并亲看匿名200取得的完整720×720 JPEG（67776B）：原书法字、纸箱头医生、作者角色与灰底均保留。没有转码、缩放、描摹、裁切或新增底板／遮罩／阴影，原App图标槽及布局保持。197已从普通文件选择器导入0.1.2，Root亲看源包管理与Browse两行：原书法字和两个角色完整保留，纸箱头及作者轮廓可辨，像素模糊较原64px减轻；只关闭这两个消费者的低清交付缺口，不扩大为全源、正文、下载或cold。原漫画请求头、身份、80条目录及阅读业务保持，仅源/crate/manifest版本步进0.1.2。
