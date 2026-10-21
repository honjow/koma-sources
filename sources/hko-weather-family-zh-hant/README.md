# 天氣家族漫畫廊・精選六期（繁中）0.1.2

香港天文台《談天說地》「天氣家族」漫畫廊的獨立圖片源，**僅精選第77至82期六篇**，每期一個獨立閱讀單元，不是全檔案。SourceInfo.author 的 Koma 是源開發者；六篇原文均標「編劇：黃婷婷」，authors 保留黃婷婷、description 明說編劇；繪者、正式評級、狀態與 usage 未知，不冒稱全部創作者。

## 原目錄與限定範圍

原第82期頁明確連到 `https://www.hko.gov.hk/tc/education/friends_hko/newsletter/ttsdpastindex.htm`。該檔案原有25個HTML期刊focus（101至77）及44個PDF（76至33）；逐個原HTML可見desktop連結／mobile選單獨立核出只有82至77六期明確指向漫畫，83至101十九期沒有有效漫畫導航，註釋內已停用連結不算入口。較早PDF及其他期刊排除，不合成25篇、不猜缺期或將focus檔名替換成comics。

版本內六條完整comic URI均來自這些原可見a/option，配置同原recordId。get_listings 靜態清楚顯示「精選六期・第77至82期」，不為開源先發26個HTTP。目錄按82/81、80/79、78/77三頁各兩篇；每篇僅GET該原文章一次，從當次原HTML驗唯一og:url、recordId、原h1及可見自身漫畫導航，保留當次原題、編劇與首正文圖作封面。這是固定精選範圍的三頁，不宣稱動態全站搜尋或未來完整目錄；第3頁明確終態，第4頁、非法游標及未知作品網路前拒絕。沒有狀態快取、沒有修改App10000ms或共同SDK。

## 原正文與身份

MID `hkofamily:zh-hant:issue:82`、CID `hkofamilyread:zh-hant:issue:82`、PID `hkofamilypage:zh-hant:issue:82:0`，其他期同樣綁定各自真URI。期刊號只屬作品身份，不作正式章號；chapterNumber/volumeNumber=null，readingGroup=本作品MID、continuationEligible=false，不構造跨期自動章序。URL和source link均為真文章，不造閱讀route或anchor。

生產HtmlHost解析原HTML，不執行JS。原主體圖由 `.border_blank` 直接img/a或其直接無class div內img/a的結構選取，排除期刊banner、desktop/mobile導航及其他樹；全部候選數與presence-qualified `[src][alt]` 數須一致，再讀屬性，不探測不存在的Attr。第77期直接img、沒有width600，同樣支持；不依賴重複alt猜唯一圖片。當次正文每圖均需官方同host、同issue的原resource/images路徑、唯一PNG URI及原DOM順序，額外／缺屬性／重複／跨域／跨期／query／fragment／未知父樹明確失敗，不猜檔名回退。

六篇當次DOM圖片數依序13/11/3/5/5/3，共40個真描述符；不是將六期硬寫為同頁數。第81期原src有空格及括號（如`081 (1).png`），僅作標準URI路徑空格`%20`編碼，原HTML不改、括號保留、未換圖。a包YouTube的原img均保留在DOM序，不請求視頻或解碼QR；第82期第6圖含必要科普敘事、第12圖是原強光轉場、第13圖為結尾，未刪裁或合成。

詳情／章／頁各1文章GET；get_image_request 先核合法URI、封面MID或正文PID、ordinal、獨立mangaId/mangaUrl歸屬，再1同文章GET核當次真DOM URI相等。封面接受正確MID或省略pageId，僅可為該篇第一正文圖；正文需真PID，其他章ID／跨期／封面當其他頁拒絕。圖片計畫顯式使用SDK既有標準Firefox User-Agent與本篇原URI作Referer，圖片實體由App取，不打包分發，不處理憑據／挑戰或任意URI。

## 原圖標

2026-10-20 Root重新讀取官方 `https://www.hko.gov.hk/js/menu.js`，其導航明確連到天文台發布的 Google Play「我的天文台」頁（`hko.MyObservatory_v1_0`）；該頁 `itemprop=image` 的2x srcset聲明完整480×480 PNG：`https://play-lh.googleusercontent.com/Fvp7BdDnGAk6W8YC_vngzpoAeBs5-WJ-vpisjf1fJwlYgenlvdCdCox6w5v-ftbaOGjHZE2o1ITn1xdC3gLH19c=w480-h960`。本版採用該獨立藍白天文台標識完整原件，取代小槽內無法辨讀的451×140官方橫字標；這是官網明確連結的官方應用標識，不稱網站favicon。依使用者「優先原始資源、官方認可標識」要求；沒有裁切、重繪、轉碼、縮放或新增底板／遮罩／陰影，白色方畫布是官方原件自身內容。App圖標槽、Contain及布局保持。197經普通文件選擇器導入0.1.2，源包管理與Browse兩行均已由Root目視：藍白獨立標識可辨，原橫標小槽文字過小問題在這兩處關閉；不擴大為全源／正文／下載／cold驗收。

## 回放、成本與未驗範圍

實施新增live請求0。原合同有2頁HTML、archive/首圖/tmp.js三補點、101/menu/其餘12原PNG、24focus加logo、五篇comic HTML；全部原件與父片分批凍結保留。第82期13張完整原PNG共9031577字節，均1080×1080 RGBA，已逐張看原圖；其餘27圖只核真HTML URI，實際格式、尺寸、完整圖文和第81期實際HTTP編碼仍未知，不冒稱40實體已下載／已顯示。

獨立runner僅回放這六份原HTTP(status/headers/body)，連結未改App C++ HtmlHost/.h及原1057份Lexbor，保原實際Parse/SelectAll/Attr/Text/Close輸入輸出與每操作零句柄。生產WASM核靜態listing、三頁2+2+2、六詳情／章／頁、40原URI圖片計畫、六個App實際MID封面和省略pageId封面、必要非法身份與6個清楚標記合成畸形body，以及既有只讀ABI範圍gate；合成失敗不當實站/Native通過。ABI採現存已接受源的範圍與operation/args前置保護，未修改SDK。

Host成本：get_listings0，browse每頁2 metadata GET，詳情／章／頁各1，封面／正文plan各1；非法語法／host／owner0，合法同目錄但不在當次DOM的URI須1文章GET才拒絕。沒有每次掃25focus、沒有加快取或改App時限。實際NetworkKit對SDK UA/Referer的可達性、10秒範圍延遲、Native、下載／保存／cold、其他27實體、寬圖標可讀性與未來站點變更均未驗；Root全diff/原合同/實際Host審查後串行197普通驗收。不改App/Kit/共同SDK/主源WIP/舊凍結，不設備、提交、推送或發布。


## 237 現裝 0.1.2 第81期首兩頁有限結果（2026-10-21）

本輪只接第81期原檔名帶空格/括號的圖片路徑；沒有重驗第82期已下載的13圖。準備時對原準確 `vol81/comics.html` 一次正常匿名GET返回200/13495B，原頁仍明確聲明 `081 (1).png`、`081 (2).png` 等相對圖URI；不猜檔名或新host。既存正式HKO012產品56216B的WASM與現target逐位元組相同，實際source_info讀回 `com.hko.weatherfamily.zh-hant.koma` / `0.1.2`，沒有計算雜湊。

237 VDE-AL00展開豎屏1320×2120，普通本地選擇器首次導入準確HKO012包，回「導入成功」。Browse現有53項之後新增天氣家族012，GUI讀回54個已啟用，新增源在醫院也瘋狂之後末項；未切換其他源啟用或修改索引。沒有匯出私有狀態，因此不把GUI首末段觀察冒作原53項持久內容逐項相同。

普通精選六期列表→第81期 (2021年4月) →一章→開始閱讀，頁計數11。首批1/11與2/11原圖仍是「圖片解碼中」，保留而不當成功；同回執21:42:32/37才分別記錄page1/page2的display_ready/display_complete。後續沒有點重試，等待當前「圖片解碼中」消失後親看兩個不同完整原圖：1/11為太陽與紫色角色、繁中「美麗嘅清風…天係藍色的」；2/11加入雲角色及繁中「雲是白色的」。正常2→1→2後自然Back，詳情出現「繼續閱讀」。當前詳情未顯示章頁文字，沒有把它冒作第2/11的持久資料讀回。此結果只閉同源012第81期兩頁普通顯示/翻頁與返回；其餘9圖、下載、cold、完整六期及無快取首取不據此通過。同會話早前transport/ReaderImage2300028仍獨立保留，不由後來顯示成功抹去或猜原因。

原件在Koma `.hvigor/outputs/user-flow-audit/device237__VDE-AL00/unfolded/portrait-1320x2120/category-membership/hko012-import237-001/`：`import-actions.json`、`issue81-reader-actions.json`、`issue81-visible-actions.json`、`return-actions.json`及原圖；成功終態為 `issue81-page1-terminal.png`、`issue81-page2-terminal.png`，初始 `issue81-page1.png`/`issue81-page2.png`仍只記等待畫面。原日誌在旁列 `hko012-issue81-reader-log-restored237-001/run-metadata.json`。最終Browse末端、54啟用畫面及HKO012行已保留，本輪唯一傳入產品包已刪/test不存在；`hko012-session-restored237-001`明確讀回AWAKE/OverrideTimeout10000ms後釋放94a5ac1f。未修改源業務/版本/圖標、App或用其他設備，全部本輪artifact雜湊關閉。
