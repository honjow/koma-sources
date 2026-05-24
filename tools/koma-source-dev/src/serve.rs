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
use std::sync::{Arc, RwLock};
use tower_http::cors::CorsLayer;

use crate::host;

pub struct AppState {
    pub wasm_dir: PathBuf,
    pub current_wasm: RwLock<PathBuf>,
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

#[derive(Deserialize)]
pub struct SwitchRequest {
    pub name: String,
}

#[derive(Serialize)]
pub struct SourceEntry {
    pub name: String,
    pub file: String,
    pub active: bool,
}

pub async fn start_server(wasm_dir: PathBuf, port: u16) -> anyhow::Result<()> {
    // Find first .wasm file as default
    let first_wasm = find_wasm_files(&wasm_dir)
        .into_iter()
        .next()
        .unwrap_or_default();

    let state = Arc::new(AppState {
        wasm_dir: wasm_dir.clone(),
        current_wasm: RwLock::new(first_wasm),
    });

    let app = Router::new()
        .route("/api/sources", get(api_sources))
        .route("/api/switch", post(api_switch))
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
    eprintln!("   wasm dir: {}", wasm_dir.display());
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

fn find_wasm_files(dir: &PathBuf) -> Vec<PathBuf> {
    let mut results = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "wasm").unwrap_or(false) {
                results.push(path);
            }
        }
    }
    results.sort();
    results
}

async fn api_sources(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let current = state.current_wasm.read().unwrap().clone();
    let sources: Vec<SourceEntry> = find_wasm_files(&state.wasm_dir)
        .into_iter()
        .map(|p| {
            let file = p.file_name().unwrap_or_default().to_string_lossy().to_string();
            let name = file.trim_end_matches(".wasm").replace('_', " ");
            let active = p == current;
            SourceEntry { name, file, active }
        })
        .collect();
    Json(sources)
}

async fn api_switch(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SwitchRequest>,
) -> impl IntoResponse {
    let target = state.wasm_dir.join(&body.name);
    if target.exists() {
        *state.current_wasm.write().unwrap() = target;
        (StatusCode::OK, "switched").into_response()
    } else {
        (StatusCode::NOT_FOUND, "wasm not found").into_response()
    }
}

async fn api_info(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let wasm = state.current_wasm.read().unwrap().clone();
    match host::run_source_info(&wasm) {
        Ok(v) => Json(v).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn api_run(
    State(state): State<Arc<AppState>>,
    Json(body): Json<RunRequest>,
) -> impl IntoResponse {
    let wasm = state.current_wasm.read().unwrap().clone();
    let request_str = serde_json::to_string(&body.request).unwrap_or_default();
    eprintln!("[api_run] op={} request={}", body.op, request_str);
    match host::run_operation(&wasm, &body.op, &request_str) {
        Ok(v) => Json(v).into_response(),
        Err(e) => {
            eprintln!("[api_run] ERROR op={}: {}", body.op, e);
            Json(serde_json::json!({
                "ok": false,
                "error": e.to_string(),
                "operation": body.op
            })).into_response()
        }
    }
}

async fn api_test_all(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let wasm = state.current_wasm.read().unwrap().clone();
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
        let status = match host::run_operation(&wasm, op, req) {
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
    (
        [(
            "Cache-Control",
            "no-cache, no-store, must-revalidate",
        )],
        Html(include_str!("../static/index.html")),
    )
}

async fn static_files(axum::extract::Path(path): axum::extract::Path<String>) -> impl IntoResponse {
    // All-in-one index.html now, no separate static files needed
    StatusCode::NOT_FOUND.into_response()
}
