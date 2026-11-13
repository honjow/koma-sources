# MangaDex — 繁體中文 0.1.10

獨立源 `org.mangadex.zh-hant.koma`，API 0.2，內容語言 zh-Hant，提供方 API 語言 `zh-hk`。其他語言源 ID 保持獨立；無別名或遷移。提供方語言元數據不保證每張圖像的書寫系統。實現/許可和原官方圖標歸屬保留在 NOTICE/LICENSE；圖標字節/背景/遮罩不變。

列表 關注較多 是全時段 followedCount，不是 30 天熱度或用量。搜尋/列表要求有可用提供方語言章節並預設 safe/suggestive 標題。原本地化標題/別名/摘要回退和實際逐標題分級保持。完整自有源流分頁、UUID/語言/總數/重複檢查和整操作邊界保持；外部和零頁章節保持省略；不可用章節預設隱藏。null/小數/零章節和卷元數據不用序號猜測替換。

`thumbnailQuality_zh-hk` 遵循固定 Keiyoushi 949dea252eb8e867b7b40834a26e13088764ff55：original（預設宿主值 `original`，編碼上游空後綴）、medium `.512.jpg`、low `.256.jpg`。列表/搜尋/詳情只把所選發布後綴追加到實際封面檔名。缺失/未知設定保留原封面。

`dataSaverV5_zh-hk` 預設 false。只有顯式 true 選擇實際 `chapter.dataSaver` 檔名和 `/data-saver/`；否則原有序 `chapter.data` 和 `/data/` 保持。get_pages 仍校驗完整自有源流和聲明頁計數；穩定頁 ID 攜帶原章節 UUID/索引。每個圖像請求仍做其既有新鮮普通 at-home GET 並按索引解析所選質量。所選和原檔案集都保留安全檔名、容量和重複守衛；不猜檔名。

圖像 HTTPS 宿主保持 uploads.mangadex.org 或實際描述符提供的 mangadex.network 子域。封面請求保持綁定同一漫畫 UUID。無 token、Cookie 處理、統計 POST、認證/重試回退、新宿主授權或 App/樣式變化。標準構建：`bash build.sh --source mangadex-zh-hant`。運行時/crate/包版本對齊到 0.1.10。

`altTitlesInDesc_zh-hk` 預設 false，只被漫畫詳情描述消費。顯式 true 追加固定參考 `Alternative titles:` 塊：擴展語言或原語言羅馬化映射，每個映射恰好一個非空值，提供方順序，不排序/去重。共享 SDK 純文本助手解碼一次 JSON Unicode 和 HTML 實體同時保留字面標記；原基礎描述保留。缺失/false 保持先前描述字節、標題回退和所有其他欄位不變。標題優先順序和最終章節文字有下面獨立的設定。

`usePort443_zh-hk`（僅使用 HTTPS 埠 443）預設 false。沿固定 Keiyoushi 949dea 的實際消費者，只在顯式開啟時給 at-home 伺服器查詢追加 `?forcePort443=true`；預設請求原樣。圖片 URL、頁 ID/序和校驗仍取真實回應，不向 URL 硬寫埠，不改其它 API、封面、認證、網路、逾時或重試。中文圖片請求重用同一 descriptor/settings 合約。

`tryUsingFirstVolumeCover_zh-hk` 預設 false。顯式 true 遵循固定參考的按卷排序批量 `/cover` 查詢，為每個返回的漫畫選擇與其原語言 locale 相關的第一張封面。搜尋/列表/詳情在應用既有封面質量後綴前消費它。缺失 locale、無匹配封面、不安全檔名或可選 HTTP/解析失敗保留原封面。預設請求和封面 URL 保持不變；不重寫章節/頁/身分或 Library 數據。既有 library 封面需要其普通顯式刷新。共享提供方實現是 `sources/mangadex/first_volume_cover.rs`。

`originalLanguage_zh-hk` 是固定原出版語言多選（日/中/韓），預設空。只過濾公開列表/搜尋請求；中文同時包含 `zh` 和 `zh-hk`。它不選擇翻譯章節語言、不重寫 ID 或 Library 標題，也不過濾精確詳情/章節/圖像恢復。缺失/空選擇保留先前請求和輸出。

`contentRating_zh-hk` 遵循固定四選項內容分級多選；缺失設定預設 `["safe","suggestive"]`，保留先前列表/搜尋請求和守衛。顯式選擇只提供那些 API 查詢值並接受其已知返回分級。顯式空選擇省略這個 API 篩選，如同參考；未知/缺失分級數據仍被拒絕。精確詳情、完整章節源流（包括其既有全分級查詢）、頁、圖像權限、ID 和既有 Library 數據不受影響。不添加帳號/登入或權限繞過。

## 完結卷章說明

`finalChapterInDesc_zh-hk`（描述中的最終章節）預設關閉，保當前 Koma 無設定的簡介；固定參考預設開啟，這裡保留既定缺省輸出。顯式開啟僅在詳情末尾按固定 MangaDexHelper 拼接非空 `lastVolume`/`lastChapter` 為 `Final chapter:\nVol.x Ch.y`，在已有簡介/別名後隔一空行，並按原實體表解碼一次。沿上游原字串，不猜最大章、不要求數字、不剝正文標籤。缺少兩值不造段落，非法欄位或容量不足明確失敗。列表、HTTP、標題、ID、目錄/頁順序和已存 Library 不主動變化。

## 標題優先順序

`preferExtensionLangTitle_zh-hk`（使用替代標題）預設 true，保當前 Koma 原標題優先鏈與原 JSON 輸出，固定參考也是 true 但既有鏈的細節不據此重寫。顯式關閉沿固定 MangaDexHelper：擴展語種 primary 值仍優先，缺失才用 primary 首 value，再回擴展語種單值 alt、英文單值 alt，並實體解碼一次。列表、搜尋、詳情及已有 Home 共用；不排序或去重標題，不主動重寫 Library 名稱、ID、目錄及頁面。顯式關閉時按參考區分空字串與缺失值，不因字串為空繼續回退；預設和開啟仍保原空標題處理，容量不足明確失敗。

`includeUnavailable_zh-hk` 預設 `false`，只在宿主聲明 `source_chapter_exclusions_v1` 時顯示和消費。顯式 `true` 顯示有正頁計數且無外部 URL 的不可用章節，帶 `UNAVAILABLE` 翻譯組標記和原章節 UUID。停用時源返回已校驗的排除章節 ID，保留的 library 行保持隱藏，而儲存的章節、下載和閱讀進度保持完好。源流即使停用也請求不可用元數據，可能需要額外頁。與固定參考不同，外部和零頁章節保持排除；這個設定不授予內容訪問。

UUID 黑名單：`blockedGroups_zh-hk` 和 `blockedUploader_zh-hk` 是逗號分隔 UUID 文字設定，只在宿主同時聲明 `source_setting_uuid_list_v1` 和 `source_chapter_exclusions_v1` 時暴露。兩者預設空；舊宿主保留既有請求和結果。有章節排除的宿主上，空黑名單保留可見結果；章節源流仍按下文請求不可用元數據。宿主在儲存前校驗欄位；源也拒絕無效值而不是添加未檢查的查詢參數。普通章節源流應用兩個排除，而精確章節恢復和頁所有權檢查繞過它們。版本 0.1.11 沒有 Latest 或 Home 欄目；其普通漫畫搜尋和關注列表保留既有請求。

狀態篩選支援固定參考 Ongoing、Completed、Hiatus 和 Cancelled 多選。顯式值作為 status[] 發送；無選擇保留原請求、可用章節和語言約束、分級、設定和排序。舊宿主看不到新篩選。其他參考篩選保持未實現。

## 0.1.12 繁中文案候選

已有狀態篩選和設定的名稱、選項、說明與校驗提示改為繁體中文，欄位 ID、值、預設、條件門控和實際處理不變；新增參考的最近章節聚合列表，僅在支援逐欄目篩選的宿主顯示並停用該欄篩選；歷史真實章節原體與受控漫畫回應、順序和分頁已作本地 WASM 回放。目前線上 API 返回 200 驗證頁面，真實最新結果、真機及發布均未驗證；其他參考篩選仍未實現。

## 章节续读

续读使用译组 UUID 集合与卷／话坐标，规则见同仓 MangaDex。原章节、分页校验和黑名单合同保持。
