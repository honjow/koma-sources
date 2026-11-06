// Original upstream option values/order, fixed reference documented in README.
use super::*;
pub const FILTERS: &str = r##"{"filters":[{"type":"header","scope":"browse","name":"篩選條件（搜尋關鍵字時無效）"},{"id":"theme","name":"作品主題","type":"select","scope":"browse","default":"0","options":[{"value":"0","label":"不限"},{"value":"1","label":"奇幻"},{"value":"2","label":"冒險"},{"value":"3","label":"異世界"},{"value":"4","label":"龍傲天"},{"value":"5","label":"魔法"},{"value":"6","label":"仙俠"},{"value":"7","label":"戰爭"},{"value":"8","label":"熱血"},{"value":"9","label":"戰鬥"},{"value":"10","label":"競技"},{"value":"11","label":"懸疑"},{"value":"12","label":"驚悚"},{"value":"13","label":"獵奇"},{"value":"14","label":"神鬼"},{"value":"15","label":"偵探"},{"value":"16","label":"校園"},{"value":"17","label":"日常"},{"value":"18","label":"JK"},{"value":"19","label":"JC"},{"value":"20","label":"青梅竹馬"},{"value":"21","label":"妹妹"},{"value":"22","label":"大小姐"},{"value":"23","label":"女兒"},{"value":"24","label":"愛情"},{"value":"25","label":"耽美"},{"value":"26","label":"百合"},{"value":"27","label":"NTR"},{"value":"28","label":"後宮"},{"value":"29","label":"職場"},{"value":"30","label":"經營"},{"value":"31","label":"犯罪"},{"value":"32","label":"旅行"},{"value":"33","label":"群像"},{"value":"34","label":"女性視角"},{"value":"35","label":"歷史"},{"value":"36","label":"武俠"},{"value":"37","label":"東方"},{"value":"38","label":"勵志"},{"value":"39","label":"宅系"},{"value":"40","label":"科幻"},{"value":"41","label":"機戰"},{"value":"42","label":"遊戲"},{"value":"43","label":"異能"},{"value":"44","label":"腦洞"},{"value":"45","label":"病嬌"},{"value":"46","label":"人外"},{"value":"47","label":"復仇"},{"value":"48","label":"鬥智"},{"value":"49","label":"惡役"},{"value":"50","label":"間諜"},{"value":"51","label":"治癒"},{"value":"52","label":"歡樂"},{"value":"53","label":"萌系"},{"value":"54","label":"末日"},{"value":"55","label":"大逃殺"},{"value":"56","label":"音樂"},{"value":"57","label":"美食"},{"value":"58","label":"性轉"},{"value":"59","label":"偽娘"},{"value":"60","label":"穿越"},{"value":"61","label":"童話"},{"value":"62","label":"轉生"},{"value":"63","label":"黑暗"},{"value":"64","label":"溫馨"},{"value":"65","label":"超自然"},{"value":"66","label":"青春"}]},{"id":"type","name":"作品分類","type":"select","scope":"browse","default":"0","options":[{"value":"0","label":"全部"},{"value":"1","label":"奇幻冒險"},{"value":"2","label":"戰鬥熱血"},{"value":"3","label":"懸疑驚悚"},{"value":"4","label":"校園青春"},{"value":"5","label":"愛情浪漫"},{"value":"6","label":"職場都市"},{"value":"7","label":"歷史文化"},{"value":"8","label":"科幻未來"},{"value":"9","label":"奇異幻想"},{"value":"10","label":"治癒溫馨"},{"value":"11","label":"末日生存"},{"value":"12","label":"其他分類"}]},{"id":"region","name":"作品地區","type":"select","scope":"browse","default":"0","options":[{"value":"0","label":"不限"},{"value":"1","label":"日本"},{"value":"2","label":"韓國"},{"value":"3","label":"港台"},{"value":"4","label":"歐美"},{"value":"5","label":"大陸"}]},{"id":"sort","name":"排序方式","type":"select","scope":"browse","default":"lastupdate","options":[{"value":"lastupdate","label":"最近更新"},{"value":"monthvisit","label":"月點擊"},{"value":"weekvisit","label":"周點擊"},{"value":"monthvote","label":"月推薦"},{"value":"weekvote","label":"周推薦"},{"value":"monthflower","label":"月鮮花"},{"value":"weekflower","label":"周鮮花"},{"value":"words","label":"字數"},{"value":"goodnum","label":"收藏數"},{"value":"postdate","label":"最新入庫"}]},{"id":"anime","name":"是否動畫","type":"select","scope":"browse","default":"0","options":[{"value":"0","label":"不限"},{"value":"1","label":"已動畫化"},{"value":"2","label":"未動畫化"}]},{"id":"novel","name":"是否輕改","type":"select","scope":"browse","default":"0","options":[{"value":"0","label":"不限"},{"value":"1","label":"輕改漫畫"},{"value":"2","label":"普通漫畫"}]},{"id":"status","name":"連載狀態","type":"select","scope":"browse","default":"0","options":[{"value":"0","label":"不限"},{"value":"1","label":"連載"},{"value":"2","label":"完結"}]},{"id":"time","name":"更新時間","type":"select","scope":"browse","default":"0","options":[{"value":"0","label":"不限"},{"value":"1","label":"三日內"},{"value":"2","label":"七日內"},{"value":"3","label":"半月內"},{"value":"4","label":"一月內"}]},{"id":"year","name":"發表年代","type":"select","scope":"browse","default":"0","options":[{"value":"0","label":"不限"},{"value":"2026","label":"2026年"},{"value":"2025","label":"2025年"},{"value":"2024","label":"2024年"},{"value":"2023","label":"2023年"},{"value":"2022","label":"2022年"},{"value":"2021","label":"2021年"},{"value":"2020","label":"2020年"},{"value":"2019","label":"2019年"},{"value":"2018","label":"2018年"},{"value":"2017","label":"2017年"},{"value":"2016","label":"2016年"},{"value":"2015","label":"2015年"},{"value":"2014","label":"2014年"},{"value":"2013","label":"2013年"},{"value":"2012","label":"2012年"},{"value":"2011","label":"2011年"},{"value":"2010","label":"2010年"},{"value":"2000","label":"00年代"},{"value":"1990","label":"90年代"},{"value":"1980","label":"80年代"},{"value":"1970","label":"更早"}]},{"id":"award","name":"這本漫畫真厲害","type":"select","scope":"browse","default":"0","options":[{"value":"0","label":"不限"},{"value":"2027","label":"2027"},{"value":"2026","label":"2026"},{"value":"2025","label":"2025"},{"value":"2024","label":"2024"},{"value":"2023","label":"2023"},{"value":"2022","label":"2022"},{"value":"2021","label":"2021"},{"value":"2020","label":"2020"},{"value":"2019","label":"2019"},{"value":"2018","label":"2018"},{"value":"2017","label":"2017"},{"value":"2016","label":"2016"},{"value":"2015","label":"2015"},{"value":"2014","label":"2014"},{"value":"2013","label":"2013"},{"value":"2012","label":"2012"},{"value":"2011","label":"2011"},{"value":"2010","label":"2010"},{"value":"2009","label":"2009"},{"value":"2008","label":"2008"},{"value":"2007","label":"2007"},{"value":"2006","label":"2006"}]}]}"##;
pub const SETTINGS: &str = r##"{"settings":[{"id":"POPULAR_MANGA_DISPLAY","name":"熱門漫畫顯示内容","kind":"select","default":"/top/weekvisit/%d.html","options":[{"value":"/top/monthvisit/%d.html","label":"月點擊榜"},{"value":"/top/weekvisit/%d.html","label":"周點擊榜"},{"value":"/top/monthvote/%d.html","label":"月推薦榜"},{"value":"/top/weekvote/%d.html","label":"周推薦榜"},{"value":"/top/monthflower/%d.html","label":"月鮮花榜"},{"value":"/top/weekflower/%d.html","label":"周鮮花榜"},{"value":"/top/monthegg/%d.html","label":"月雞蛋榜"},{"value":"/top/weekegg/%d.html","label":"周雞蛋榜"},{"value":"/top/postdate/%d.html","label":"最新入庫"},{"value":"/top/goodnum/%d.html","label":"收藏榜"},{"value":"/top/newhot/%d.html","label":"新書榜"}]},{"id":"RATE_LIMIT","name":"請求速率限制","kind":"text","default":"10/10","description":"每段時間的請求數／秒數，例如 10/2 表示每 2 秒允許 10 個請求。預設 10/10"},{"id":"DESCRIPTION","name":"作品信息顯示偏好","kind":"multiselect","default":["A","B","C"],"options":[{"value":"A","label":"作品公告"},{"value":"B","label":"作品别名"},{"value":"C","label":"跳轉連結"}]}]}"##;
pub fn filter_value<'a>(r: &'a [u8], k: &[u8]) -> R<&'a [u8]> {
    let (default, values): (&[u8], &[&[u8]]) = match k {
        b"theme" => (
            b"0",
            &[
                b"0", b"1", b"2", b"3", b"4", b"5", b"6", b"7", b"8", b"9", b"10", b"11", b"12",
                b"13", b"14", b"15", b"16", b"17", b"18", b"19", b"20", b"21", b"22", b"23", b"24",
                b"25", b"26", b"27", b"28", b"29", b"30", b"31", b"32", b"33", b"34", b"35", b"36",
                b"37", b"38", b"39", b"40", b"41", b"42", b"43", b"44", b"45", b"46", b"47", b"48",
                b"49", b"50", b"51", b"52", b"53", b"54", b"55", b"56", b"57", b"58", b"59", b"60",
                b"61", b"62", b"63", b"64", b"65", b"66",
            ],
        ),
        b"type" => (
            b"0",
            &[
                b"0", b"1", b"2", b"3", b"4", b"5", b"6", b"7", b"8", b"9", b"10", b"11", b"12",
            ],
        ),
        b"region" => (b"0", &[b"0", b"1", b"2", b"3", b"4", b"5"]),
        b"sort" => (
            b"lastupdate",
            &[
                b"lastupdate",
                b"monthvisit",
                b"weekvisit",
                b"monthvote",
                b"weekvote",
                b"monthflower",
                b"weekflower",
                b"words",
                b"goodnum",
                b"postdate",
            ],
        ),
        b"anime" => (b"0", &[b"0", b"1", b"2"]),
        b"novel" => (b"0", &[b"0", b"1", b"2"]),
        b"status" => (b"0", &[b"0", b"1", b"2"]),
        b"time" => (b"0", &[b"0", b"1", b"2", b"3", b"4"]),
        b"year" => (
            b"0",
            &[
                b"0", b"2026", b"2025", b"2024", b"2023", b"2022", b"2021", b"2020", b"2019",
                b"2018", b"2017", b"2016", b"2015", b"2014", b"2013", b"2012", b"2011", b"2010",
                b"2000", b"1990", b"1980", b"1970",
            ],
        ),
        b"award" => (
            b"0",
            &[
                b"0", b"2027", b"2026", b"2025", b"2024", b"2023", b"2022", b"2021", b"2020",
                b"2019", b"2018", b"2017", b"2016", b"2015", b"2014", b"2013", b"2012", b"2011",
                b"2010", b"2009", b"2008", b"2007", b"2006",
            ],
        ),
        _ => return Err("invalid_request"),
    };
    let v = extract_json_string(r, k).unwrap_or(default);
    if values.contains(&v) {
        Ok(v)
    } else {
        Err("invalid_request")
    }
}
pub fn popular(r: &[u8]) -> R<&[u8]> {
    let v = extract_json_string(r, b"POPULAR_MANGA_DISPLAY").unwrap_or(b"/top/weekvisit/%d.html");
    match v {
        b"/top/monthvisit/%d.html" => Ok(b"monthvisit"),
        b"/top/weekvisit/%d.html" => Ok(b"weekvisit"),
        b"/top/monthvote/%d.html" => Ok(b"monthvote"),
        b"/top/weekvote/%d.html" => Ok(b"weekvote"),
        b"/top/monthflower/%d.html" => Ok(b"monthflower"),
        b"/top/weekflower/%d.html" => Ok(b"weekflower"),
        b"/top/monthegg/%d.html" => Ok(b"monthegg"),
        b"/top/weekegg/%d.html" => Ok(b"weekegg"),
        b"/top/postdate/%d.html" => Ok(b"postdate"),
        b"/top/goodnum/%d.html" => Ok(b"goodnum"),
        b"/top/newhot/%d.html" => Ok(b"newhot"),
        _ => Err("invalid_request"),
    }
}
