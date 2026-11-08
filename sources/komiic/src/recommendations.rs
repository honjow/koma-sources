//! Komiic's public reader rankings and account-specific AI recommendations.
use alloc::{format, string::ToString, vec::Vec};
use serde_json::{json, Value};

use super::{
    fetch_error_code, post_graphql, session, write_error, write_manga_list_page, FetchError,
    PAGE_SIZE,
};

const OPERATION: &str = "get_manga_list";
const CARD_FIELDS: &str = "id title status imageUrl authors { id name }";
const AI_LIMIT: usize = 25;

pub fn run(request: &[u8], listing: &[u8]) -> Option<u32> {
    let period = match listing {
        b"reader-recommended-week" => "WEEK",
        b"reader-recommended-month" => "MONTH",
        b"reader-recommended-year" => "YEAR",
        b"ai-recommended" => return Some(finish(ai(request))),
        _ => return None,
    };
    Some(finish(reader(request, period)))
}

fn finish(result: Result<u32, FetchError>) -> u32 {
    match result {
        Ok(output) => output,
        Err(error) => {
            let (code, message) = fetch_error_code(error);
            write_error(OPERATION, code, message)
        }
    }
}

fn reader(request: &[u8], period: &str) -> Result<u32, FetchError> {
    let page = koma_source_sdk::json_utils::request_page_number(request);
    let offset = page
        .checked_sub(1)
        .and_then(|page| page.checked_mul(PAGE_SIZE))
        .filter(|offset| *offset <= i32::MAX as usize)
        .ok_or(FetchError::ClientError)?;
    // Keep the site's 30-card offset and use one lookahead card to distinguish
    // a full last page from a real continuation. The host's page limit is a hint.
    let query = format!("query readerRecommendations($period: RecommendationPeriod!, $pagination: Pagination!) {{ comics: topRecommendedComics(period: $period, pagination: $pagination, adult: false, contentType: REGULAR) {{ {CARD_FIELDS} }} }}");
    let body = json!({"query":query,"variables":{"period":period,"pagination":{
        "offset":offset,"limit":PAGE_SIZE+1,"orderBy":"VIEWS","asc":false
    }}})
    .to_string();
    let response = post_graphql(request, body.as_bytes())?;
    Ok(write_manga_list_page(
        OPERATION,
        response,
        PAGE_SIZE,
        Some(page),
        true,
        request,
    ))
}

fn authenticated_query(request: &[u8], body: &Value) -> Result<Value, FetchError> {
    let body = body.to_string();
    // Use the selected mirror's private session and its existing refresh logic.
    // A guest suggestion response is an empty list, so never fall back to it.
    let response =
        session::graphql(request, body.as_bytes(), true).ok_or(FetchError::AuthRequired)??;
    serde_json::from_slice(response).map_err(|_| FetchError::Graphql)
}

fn ai(request: &[u8]) -> Result<u32, FetchError> {
    let result = authenticated_query(
        request,
        &json!({
            "query":"query personalizedSuggestions($limit: Int, $excludeRead: Boolean, $excludeFavorites: Boolean) { personalizedSuggestions(limit: $limit, excludeRead: $excludeRead, excludeFavorites: $excludeFavorites) { comicId } }",
            "variables":{"limit":AI_LIMIT,"excludeRead":true,"excludeFavorites":true}
        }),
    )?;
    let suggestions = result["data"]["personalizedSuggestions"]
        .as_array()
        .ok_or(FetchError::Graphql)?;
    if suggestions.len() > AI_LIMIT {
        return Err(FetchError::Graphql);
    }
    let mut ids: Vec<&str> = Vec::new();
    for suggestion in suggestions.iter().filter(|row| !row.is_null()) {
        let id = suggestion["comicId"]
            .as_str()
            .filter(|id| {
                !id.is_empty() && id.len() <= 16 && id.as_bytes().iter().all(u8::is_ascii_digit)
            })
            .ok_or(FetchError::Graphql)?;
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    let mut ordered = Vec::new();
    if !ids.is_empty() {
        let query = format!("query comicByIds($comicIds: [ID]!) {{ comicByIds(comicIds: $comicIds) {{ {CARD_FIELDS} }} }}");
        let details = authenticated_query(
            request,
            &json!({"query":query,"variables":{"comicIds":ids}}),
        )?;
        let comics = details["data"]["comicByIds"]
            .as_array()
            .ok_or(FetchError::Graphql)?;
        // The metadata response need not preserve the recommendation order.
        for id in ids {
            if let Some(comic) = comics.iter().find(|comic| comic["id"].as_str() == Some(id)) {
                ordered.push(comic.clone());
            }
        }
    }
    let cards = json!({"comics":ordered}).to_string();
    Ok(write_manga_list_page(
        OPERATION,
        cards.as_bytes(),
        AI_LIMIT,
        None,
        true,
        request,
    ))
}
