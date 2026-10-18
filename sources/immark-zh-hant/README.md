# 我是馬克・圖文並靠《今周刊》

來源為作者原部落格的個人漫畫分類 https://markleeblog.pixnet.net/blog/categories/2001687837 。不包含混合首頁的桌布或其他個人分類；按原可見下一頁逐頁瀏覽，不掃描49頁，也不將頁數乘每頁數當作品總量。每篇獨立作品、一個完整閱讀單元，正式章號/卷號null，readingGroup為該作品MID，continuationEligible=false；不從日期/頁位置造章序。作者標識「我是馬克 i'm mark」，正式評級、連載狀態、使用率unknown。

- source ID：`com.immark.pixnet.zh-hant.koma`；版本0.1.0；ABI0.2；語言zh-Hant。
- 功能：listings/mangaList/detail/chapters/pages/imageRequest；search/home/filters/settings/credentials不支持。
- 真實靜態分類當前每頁12項；頁1與頁2各12項原ID/標題/URL/封面/個人分類已核，兩頁無重合。返回原next完整URI作cursor；頁2基礎canonical/og不含query，是原站事實；可見頁碼與back/next共同校驗當前頁。後續頁、終態及其它文章正文未核。
- 18位post/image ID只作原字節字符串，不經f64或wasm32 usize。MID=`immark:zh-hant:post:<post>`，章ID=`immarkread:zh-hant:post:<post>`，頁ID=`immarkpage:zh-hant:post:<post>:<0-based ordinal>`。只有本地頁碼/ordinal使用整數。
- 目錄限定`#article-area > #article-box > div.article[id]`，不依賴與li.title重複的單一ID選擇。逐項驗外article ID、data-article-link、h2原URL、個人漫畫分類與單一直接封面。詳情原div.article無id、li.title無data-article-link，按唯一主div.article和h2原URL校驗，不探不存在屬性。詳情footer同時有全站心情日記和個人漫畫分類，只以個人分類原域路徑鎖定精確分類。
- 兩種頁面均驗canonical、og:url、meta author與原PIXNET name/blog_id/post_id/AdultWarning=false；目錄驗Person JSON-LD，詳情只驗原有BlogPosting的作者PERSON引用、URL/@id/mainEntityOfPage/headline/articleSection/isPartOf/inLanguage與原發表日期。詳情沒有目錄Person graph，不合成補入。共用成人提醒模板不等於啟用門檻，亦不推全站評級。靜態JSON有界讀取，不執行JS。
- 詳情只取主`article-content-inner > p > img[src]`，須等於主內容內全部img數、同post原媒體路徑，拒外域/查詢/任意子目錄/重複圖、嵌入影片或圖片樹變化。不能拿JSON-LD只有第一圖的image陣列代替全文。首篇《職場太極法門》完整9張900×900原JPEG保DOM原序，保留首圖今周刊贊助及版權原字，不裁/轉/縮圖。原尾圖是「離線遁形術」。
- 圖片計畫每次1 GET重新核該文章，無跨操作狀態快取；正文pageID/owner/ordinal/完整URI必須相符。封面僅接受未帶pageID或該作品MID且必須原首圖。可獨立傳App實際無mangaId的mangaUrl，須精確post原URL。原圖URI、metadata標準Firefox UA、文章Referer明確輸出；這一Host計畫不等於NetworkKit圖片層已驗。
- 作者頭像由原head/JSON-LD/sidebar共同聲明，420×420原WebP保留在合同證據。現包只接受可選PNG圖標，本源無icon；不把WebP冒名PNG，不改App/SDK或轉碼，不借PIXNET平台favicon。

成本：get_listings 0 HTTP，browse每頁1 GET，detail/chapters/pages/每次imageRequest各1 GET；非法URI/owner前置拒絕0 GET，語法正確但未在原正文的URI需1 GET後拒絕。正文圖實體由App圖片層另取。源限制HTML<1MiB、每頁≤24項、每篇≤64原圖、JSON巢狀≤64，無回退猜URI、無身份/憑據、無重試、無新live請求。

只用凍結原兩頁/首詳情在現有生產Cpp HtmlHost+Lexbor/Wasmtime ABI上回放；真實原站合同和明確synthetic拒絕例分開。兩頁24項、首篇9頁/圖片計畫與ABI完成，不代表其餘作品/後續頁/Native已驗。初期模板誤解造成的開發失敗另留證據與說明，沒有改原HTTP。Root負責普通197導入、原分頁/正文、下載與cold驗收。

原合同：`/private/tmp/koma-immark-zh-hant-public-contract-20261015`（70文件）、`/private/tmp/koma-immark-zh-hant-page2-addon-20261015`（13文件）。實現及Host證據：`/private/tmp/koma-immark-zh-hant-implementation-20261015`。包不分發漫畫原圖。未commit/push/release，無App/Kit/SDK/設備改動。
