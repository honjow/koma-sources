use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

use crate::host;

pub struct AppState {
    pub wasm_path: PathBuf,
}

#[derive(Deserialize)]
pub struct RunRequest {
    pub op: String,
    pub request: serde_json::Value,
}

#[derive(Deserialize)]
pub struct ProxyQuery {
    pub url: String,
}

pub async fn start_server(wasm_path: PathBuf, port: u16) -> anyhow::Result<()> {
    let state = Arc::new(AppState { wasm_path });

    let app = Router::new()
        .route("/api/info", get(api_info))
        .route("/api/run", post(api_run))
        .route("/api/test-all", get(api_test_all))
        .route("/api/proxy", get(api_proxy))
        .route("/", get(index_html))
        .route("/{*path}", get(static_files))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    eprintln!("🚀 koma-source-dev serve → http://localhost:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn api_info(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match host::run_source_info(&state.wasm_path) {
        Ok(v) => Json(v).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn api_run(
    State(state): State<Arc<AppState>>,
    Json(body): Json<RunRequest>,
) -> impl IntoResponse {
    let request_str = serde_json::to_string(&body.request).unwrap_or_default();
    match host::run_operation(&state.wasm_path, &body.op, &request_str) {
        Ok(v) => Json(v).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn api_test_all(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // Run test-all and capture results as JSON
    let ops = vec![
        ("search", r#"{"query":"test"}"#),
        ("get_listings", "{}"),
        ("get_manga_list", r#"{"page":"1"}"#),
        ("get_filters", "{}"),
        ("get_settings", "{}"),
        ("get_home", "{}"),
        ("get_image_request", r#"{"url":"https://example.com/img.jpg"}"#),
    ];
    let mut results = Vec::new();
    for (op, req) in ops {
        let status = match host::run_operation(&state.wasm_path, op, req) {
            Ok(v) => {
                let ok = v.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
                if ok { "pass".to_string() } else { "fail".to_string() }
            }
            Err(e) => format!("error: {}", e),
        };
        results.push(serde_json::json!({"op": op, "status": status}));
    }
    Json(serde_json::json!({"results": results}))
}

async fn api_proxy(Query(q): Query<ProxyQuery>) -> impl IntoResponse {
    // Fetch the image URL and proxy it back
    match ureq::get(&q.url)
        .set("Referer", &q.url)
        .call()
    {
        Ok(resp) => {
            let content_type = resp.content_type().to_string();
            let mut bytes = Vec::new();
            if resp.into_reader().read_to_end(&mut bytes).is_ok() {
                Response::builder()
                    .header("Content-Type", content_type)
                    .header("Cache-Control", "public, max-age=3600")
                    .body(axum::body::Body::from(bytes))
                    .unwrap()
                    .into_response()
            } else {
                StatusCode::BAD_GATEWAY.into_response()
            }
        }
        Err(_) => StatusCode::BAD_GATEWAY.into_response(),
    }
}

async fn index_html() -> impl IntoResponse {
    Html(include_str!("../static/index.html"))
}

async fn static_files(axum::extract::Path(path): axum::extract::Path<String>) -> impl IntoResponse {
    // Serve embedded static files
    match path.as_str() {
        "app.js" => (
            StatusCode::OK,
            [("Content-Type", "application/javascript")],
            include_str!("../static/app.js"),
        ).into_response(),
        "style.css" => (
            StatusCode::OK,
            [("Content-Type", "text/css")],
            include_str!("../static/style.css"),
        ).into_response(),
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}
