//! HTTP server: POST /v1/chat/completions, GET /v1/models, GET /v1/health.

use crate::config::{default_session_file_path, Config};
use crate::openai::{build_completion_response, sse_chunk, sse_done, ChatCompletionRequest};
use crate::service::{CompletionError, CompletionInput, CompletionService};
use crate::session::{PersistentSessionStore, SessionStore};
use axum::{
    body::Body,
    extract::State,
    http::HeaderMap,
    response::Response,
    routing::{get, post},
    Json, Router,
};
use bytes::Bytes;
use std::num::NonZeroUsize;
use std::sync::Arc;

/// App state: config, session store, and completion service (injectable for tests).
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub completion_service: Arc<CompletionService>,
}

/// OpenAI-style error body.
#[derive(serde::Serialize)]
struct ErrorBody {
    error: ErrorDetail,
}

#[derive(serde::Serialize)]
struct ErrorDetail {
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<String>,
}

fn err_response(
    status: axum::http::StatusCode,
    message: &str,
    code: Option<&str>,
) -> (axum::http::StatusCode, Json<ErrorBody>) {
    (
        status,
        Json(ErrorBody {
            error: ErrorDetail {
                message: message.to_string(),
                code: code.map(String::from),
            },
        }),
    )
}

fn completion_error_to_http(e: CompletionError) -> (axum::http::StatusCode, Json<ErrorBody>) {
    match e {
        CompletionError::CursorNotFound => err_response(
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            "cursor-agent not found. Set CURSOR_PATH or ensure Cursor is installed.",
            Some("cursor_not_found"),
        ),
        CompletionError::InvalidRequest(msg) => err_response(
            axum::http::StatusCode::BAD_REQUEST,
            msg,
            Some("invalid_request"),
        ),
        CompletionError::NoContent => err_response(
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            "cursor-agent returned no content. Check plugin/server logs for cursor-agent stderr; consider increasing request_timeout_sec in cursor-brain.json.",
            Some("no_response"),
        ),
        CompletionError::SpawnFailed(io) => err_response(
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            &format!("Failed to start cursor-agent: {}", io),
            Some("spawn_failed"),
        ),
        CompletionError::JoinFailed(msg) => {
            err_response(axum::http::StatusCode::INTERNAL_SERVER_ERROR, &msg, None)
        }
    }
}

/// POST /v1/chat/completions: thin handler — build input, call service, map response.
async fn chat_completions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ChatCompletionRequest>,
) -> Result<axum::response::Response, (axum::http::StatusCode, Json<ErrorBody>)> {
    let default_model = state.config.default_model.as_deref().unwrap_or("auto");
    let input = CompletionInput::from_request(
        &body,
        &headers,
        &state.config.session_header_name,
        default_model,
    )
    .map_err(completion_error_to_http)?;

    if input.stream {
        let (id, model_owned, mut rx) = state
            .completion_service
            .complete_stream(input)
            .await
            .map_err(completion_error_to_http)?;
        let stream = async_stream::stream! {
            while let Some(delta) = rx.recv().await {
                match delta {
                    crate::cursor::StreamDelta::Content(s) => {
                        let chunk = sse_chunk(&id, &model_owned, Some(&s), None);
                        yield Ok::<_, std::convert::Infallible>(Bytes::from(chunk));
                    }
                    crate::cursor::StreamDelta::Done { finish_reason } => {
                        let chunk = sse_chunk(&id, &model_owned, None, Some(&finish_reason));
                        yield Ok(Bytes::from(chunk));
                        yield Ok(Bytes::from(sse_done()));
                        break;
                    }
                }
            }
        };
        return Ok(Response::builder()
            .status(axum::http::StatusCode::OK)
            .header("Content-Type", "text/event-stream")
            .header("Cache-Control", "no-cache")
            .header("Connection", "keep-alive")
            .body(Body::from_stream(stream))
            .unwrap());
    }

    let (out, model_owned, id) = state
        .completion_service
        .complete(input)
        .await
        .map_err(completion_error_to_http)?;
    let resp = build_completion_response(&id, &model_owned, &out, true);
    Ok(Response::builder()
        .status(axum::http::StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&resp).unwrap_or_default()))
        .unwrap())
}

/// GET /v1/models — list from cursor-agent --list-models; fallback to default when agent unavailable or timeout.
async fn list_models(State(state): State<AppState>) -> Json<serde_json::Value> {
    let config = state.config.clone();
    const LIST_MODELS_TIMEOUT_SECS: u64 = 15;
    let ids: Vec<String> = match config.resolve_cursor_path() {
        Some(cursor_path) => {
            let list = tokio::time::timeout(
                std::time::Duration::from_secs(LIST_MODELS_TIMEOUT_SECS),
                tokio::task::spawn_blocking(move || {
                    crate::cursor::list_models_via_agent(&cursor_path)
                }),
            )
            .await
            .ok()
            .and_then(|r| r.ok())
            .unwrap_or_default();
            if list.is_empty() {
                crate::config::DEFAULT_MODELS_LIST
                    .iter()
                    .map(|s| (*s).to_string())
                    .collect()
            } else {
                list
            }
        }
        None => crate::config::DEFAULT_MODELS_LIST
            .iter()
            .map(|s| (*s).to_string())
            .collect(),
    };
    let data: Vec<serde_json::Value> = ids
        .iter()
        .map(|id| {
            serde_json::json!({
                "id": id,
                "object": "model",
                "created": 0
            })
        })
        .collect();
    Json(serde_json::json!({
        "object": "list",
        "data": data
    }))
}

/// GET /v1/health
async fn health(State(state): State<AppState>) -> Json<serde_json::Value> {
    let cursor_ok = state.config.resolve_cursor_path().is_some();
    Json(serde_json::json!({
        "status": if cursor_ok { "ok" } else { "degraded" },
        "cursor": cursor_ok,
        "port": state.config.port,
        "session_storage": "file"
    }))
}

pub fn app(config: Arc<Config>) -> Router {
    let cap = NonZeroUsize::new(config.session_cache_max as usize).unwrap_or(NonZeroUsize::MIN);
    let session_store: Arc<dyn SessionStore> = Arc::new(PersistentSessionStore::new(
        default_session_file_path(),
        cap,
    ));
    let completion_service = Arc::new(CompletionService::new(config.clone(), session_store));
    let state = AppState {
        config,
        completion_service,
    };
    Router::new()
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/models", get(list_models))
        .route("/v1/health", get(health))
        .with_state(state)
}
