# IP手札mini! 官方直達專欄（繁中）0.1.0

來源為經濟部智慧財產局主站 `https://www.tipo.gov.tw/tw/tipo1/295.html` 的漫畫目錄及其原 `cloud.tipo.gov.tw/S400/article/columnist/ID` 直達專欄。這是獨立於此前 `/tw/copyright/701.html` 校園混合宣導品／PDF的真漫畫入口，不重取舊入口、不取既有CCC作品。SourceInfo.author的Koma是源開發者；逐作原author欄位如「財團法人中衛發展中心彙編」原樣放authors和說明，不冒稱繪師，artists為空。正式作品評級、狀態與usage未知。

## 明確範圍與真實分頁

本次原主站兩頁12＋10共22目錄記錄、17不同href。動態篩選主要 `section.lp > .listAlbum > .Flex-set > .thumbnail` 內精確 `https://cloud.tipo.gov.tw/S400/article/columnist/正整數`，目前15直達作品，真分頁輸出首12／次3。#16 href末尾空白只依正常URL語義trim外圍ASCII空白，原HTTP不改。按article ID穩定標識，不以title、位置或章數去重；不硬編碼15白名單、標題或正文URI。listing動態顯示「官方直達專欄・目前15篇（不含舊泛目錄）」；description明確僅直達專欄。

其餘7原記錄不支持且不冒作作品：#8..#4五條原href皆`/S400/article/4`，#2/#1兩條皆`/S400/article/3`。已取一次article/4實為「產業輔導分享」42篇分類，非五篇漫畫；不能把多個泛目錄同URL折疊成漫畫、拿封面代正文或猜修舊路由。#3實為`/article/columnist/391`，是15之一，已單獨核正文；舊調查14/8誤分類保原凍結並另correction addon，實作按原兩頁DOM獨立核15/7，不沿用誤結論。

首屏匿名GET原目錄；下一頁按原contentSearchForm/action/method與原listChangePage函數，只讀POST原六字段：`q_stitle=&q_xbody=&q_xpostDate_start=&q_xpostDate_end=&nowPage=2&pageSize=12`。Host現有bodyBase64欄位原樣作extraData，與SDK build_post_request現有行為相符，不修改SDK或作base64／簽名猜測。每次操作檢查唯一form的六input、pageSize12、當前頁／總頁／總筆數與真next data-args；第2/2無next，輸出`hasNextPage=false`和null cursor，不猜第3頁或假終態。游標只接受1/2或App第一頁null；第3頁、URL／前置零／零游標網路前拒絕。原12筆頁大小、最多2頁／24記錄為已核有界容量，變更超界明確失敗，不冒稱未來無限頁合同。

## 詳情、原題與正文

當前目錄找到ID後只GET其原columnist詳情；不把輸入mangaUrl當任意HTTP入口。生產Cpp HtmlHost取唯一`script#__NEXT_DATA__[type='application/json']`原文本，源內有界JSON解析`props.pageProps.article`和並列`props.pageProps.content`。article.id須同作品、columnistArticleCategory.id須8；title／author／summary／startDate原字段保留。summary在詳情說明的「原文簡介」後按原字串保留，不trim、改寫或冒稱本站文案；缺失/null/空summary不編造，其他型別或超8192字節明確失敗。日期只有yyyyMMdd日精度，放說明原字串，不製造epoch／時區。目錄title和詳情title可以合法不同：#3目錄「IP手札mini!#3待辦事項-強化IP管理制度」，詳情「IP手札mini! #3 強化IP管理制度」；各按其原字段輸出，不用字節相等拒絕，也不改原題。inner content h1只在比對時折疊ASCII空白與詳情原title一致，不改對外字串。

content僅作JSON字串解碼後交同一未改Cpp HtmlHost解析，非JS執行、非合成成功HTTP。正文只取`body > .container > p > img[src]`的原DOM順序；header/logo、英文版連結與footer不納入。所有container圖片須符合此已核父樹，1–16原圖有界；缺src／無圖／重複／多餘巢狀圖片／跨host／跨article／query／fragment明確拒絕。URI必須精確`https://cloud.tipo.gov.tw/S400/bapi/article/columnist/本ID/image/正整數`，且真DOM存在；不從metadata mainImage或檔名猜圖。#22原mainImage2281、正文2282；#3原mainImage1545、正文1544。原JPEG包含完整四格與知識解說，不裁格、切頁或改圖。

每篇獨立作品／一章，MID `tipoipmini:zh-hant:article:ID`，CID `tipoipminiread:zh-hant:article:ID`，頁PID `tipoipminipage:zh-hant:article:ID:ordinal`。正式chapterNumber/volumeNumber=null，readingGroup=本作品MID、continuationEligible=false，不據原題#序號建立跨作品自動閱讀。pageCount來自本篇真content圖片數；目前兩份已核詳情各1原圖，其餘13正文數量／實體未驗。

## 圖片計畫與圖標

封面只用真主站目錄Attachment原JPEG，不混正文：精確主站`/wSite/public/Attachment/0/f數字.jpg`，pageId可為本MID或省略；省略時讀完整支持目錄排除歧義。正文只接受本PID/合法ordinal與當次真content原URI逐項相同，其他article／mainImage猜URI不通過。獨立mangaId/mangaUrl若提供皆核本ID，非法host／ID／URI／owner形狀在HTTP前拒絕；語法合法但未列作品或不屬於該封面需讀真目錄方能判斷。圖片計畫明確輸出SDK metadata既有標準Firefox UA；封面Referer為原主站目錄，正文Referer為本詳情。不下載或包內分發漫畫，無任意URI請求／憑據／挑戰處理／JS執行。

`icon.png` 保持新主站head原`https://www.tipo.gov.tw/wSite/styles/tipo/images/favicon.png`的3389原字節，32×32 RGBA PNG，SHA256 `709c771f8de75bb80c857b89afb40f804f2abad7430401346c5d9113fcfaab1c`。雖type宣告image/x-icon，實際原簽名PNG；未轉碼／裁切／縮放／加底／遮罩／陰影。是官方IPO身份，不冒稱漫畫角色原畫，小槽效果尚未Native驗收。

## 原件、生產回放、成本與邊界

原34凍結含6正常匿名請求（5GET/1只讀POST），共5543909響應body字節；correction addon只有離線更正；#3 addon再2匿名GET共6944449字節。原#22 JPEG2480×3508 RGB、5351599字節，SHA256 `2069b55c0164ad7c09af8465cd3c6965a330e409f24c54764bba29b2facf2025`；#3同尺寸RGB、6928829字節，SHA256 `ad401167d7c583900c17da5a07835069e093287ce37c5466e038bff519f3a3d3`。原匿名響應cookie/CSRF/testcookie僅保留HTTP原頭，不保存jar／復用／登入。實施新增live請求0，不重新下載原大圖或擴抓其餘13/generic7。

獨立Wasmtime工具鏈連結當前未改App Cpp HtmlHost/.h與1057份原Lexbor；只以原兩目錄及兩詳情HTTP/status/headers離線回放，HTTP橋核GET/POST六字段、UA/Referer及Content-Type，不碰共同runner。R1生產WASM的56操作包含動態listing／真12+3／15封面計畫，兩份真詳情／章／頁／正文計畫、22記錄篩15排7、#16原href外空白、#3原題差異、四個不支持操作、18非法請求、8個明確標記合成畸形失敗及完整ABI邊界。兩份原Next文本與生產Cpp Text逐字節相等，每操作descriptor歸零；合成失敗不冒作實站或Native。未重跑Laya447，未取其他實體。

Host callback成本不是Native耗時：listing2GET/POST（116937原metadata字節）；Browse首1GET、次2（先核原首頁表單再POST）；前12作品封面各1metadata，後3各2；#22詳情／章／頁／正文計畫各2metadata（74636字節），#3各3（132557字節）；非法前置請求0，合法未知ID要核兩頁才拒絕。R1的56回放共60保留metadata回調，原圖實體網路0。支持listings/manga_list/detail/chapters/pages/image_request；search/home/filter/settings/credentials無合同明確不支持，不寫Browse空殼。

Native/WAMR、其他13詳情／正文、7舊記錄真漫畫落點、主站／cloud實際SDK UA/Referer網路、下載/保存/cold及未來變更仍未知。Root全差異/原父樹/Host證據審查後負責197串行普通導入、兩頁和首具名閱讀驗收。未改App/Kit/SDK/主源WIP/任何舊凍結物，無設備/IDE/簽名/提交/推送/發布操作。


metadataR2仍為未安裝的0.1.0獨立候選。R1完整樹／包／56回放保持凍結；R2僅補原article.summary到詳情簡介及本README，無其他行為／capabilities／身份／圖片／圖標變更。R2生產WASM只重放受影響#22與#3的get_manga，確認原summary逐字保留、支持範圍／日期／編署仍在，除summary新增段落以外其餘輸出與R1逐項相同，描述符歸零。R1其他回放不冒稱R2已重新執行；Root審精確R1→R2差異及兩份當次Host輸出後再197串行驗收。
