use std::convert::Infallible;
use std::net::SocketAddr;
use std::time::Duration;

use async_stream::stream;
use axum::extract::{ConnectInfo, Path, Query, State};
use axum::http::{header, HeaderMap, Method, StatusCode, Uri};
use axum::middleware::Next;
use axum::response::sse::{Event, KeepAlive};
use axum::response::{Html, IntoResponse, Response, Sse};
use axum::routing::{delete, get, patch, post};
use axum::{middleware, Json, Router};
use tracing::{info, warn};

use crate::auth::{build_access_url, normalize_mobile_base_path};
use crate::error::AppError;
use crate::persistence::{
    AuditAction, ClipboardListQuery, CreateAuditLogInput, PersistenceRepository,
    SaveClipboardItemInput, SourceKind,
};

use super::state::HttpRuntimeContext;
use super::types::{
    ApiEnvelope, ClipboardClearResponse, ClipboardDeleteResponse, ClipboardItemDetail,
    ClipboardItemSummary, ClipboardListResponse, ClipboardPinRequest, ClipboardWriteRequest,
    ClipboardWriteResponse, HealthResponse, ServerEvent, SessionResponse,
};

pub fn build_router(context: HttpRuntimeContext) -> Router {
    let mobile_route = normalize_mobile_base_path(&context.runtime_config.mobile_route);
    let mobile_fallback = format!("{mobile_route}/*path");

    Router::new()
        .route("/", get(root_handler))
        .route(&mobile_route, get(mobile_handler))
        .route(&mobile_fallback, get(mobile_handler))
        .route("/api/v1/health", get(health_handler))
        .route("/api/v1/session", get(session_handler))
        .route("/api/v1/session/rotate-token", post(rotate_token_handler))
        .route("/api/v1/clipboard-items", get(list_clipboard_items_handler))
        .route(
            "/api/v1/clipboard-items",
            post(create_clipboard_item_handler),
        )
        .route(
            "/api/v1/clipboard-items/:id",
            get(get_clipboard_item_handler),
        )
        .route(
            "/api/v1/clipboard-items/:id",
            delete(delete_clipboard_item_handler),
        )
        .route(
            "/api/v1/clipboard-items/:id/activate",
            post(activate_clipboard_item_handler),
        )
        .route(
            "/api/v1/clipboard-items/:id",
            patch(update_clipboard_pin_handler),
        )
        .route(
            "/api/v1/clipboard-items/clear",
            post(clear_clipboard_history_handler),
        )
        .route("/api/v1/events", get(events_handler))
        .layer(middleware::from_fn_with_state(
            context.clone(),
            request_guard,
        ))
        .with_state(context)
}

async fn request_guard(
    State(state): State<HttpRuntimeContext>,
    req: axum::http::Request<axum::body::Body>,
    next: Next,
) -> Response {
    let remote_key = remote_key_from_request(&req).unwrap_or_else(|| "unknown".to_string());
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let classification = classify_request(&method, &path);

    if let Some(kind) = classification {
        let rate_result = match kind {
            RequestKind::Read => state.http_server.allow_read_request(&remote_key),
            RequestKind::Write => state.http_server.allow_write_request(&remote_key),
            RequestKind::Sse => state.http_server.allow_sse_request(&remote_key),
        };

        if let Err(error) = rate_result {
            warn!(
                method = %method,
                path = %path,
                remote = %remote_key,
                error = %error,
                "request rejected by rate limiter"
            );
            return api_error(error);
        }

        info!(
            method = %method,
            path = %path,
            remote = %remote_key,
            kind = ?kind,
            "request accepted"
        );
    }

    next.run(req).await
}

async fn root_handler(State(state): State<HttpRuntimeContext>) -> Html<String> {
    let http_status = state.http_server.status();
    let body = format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>LocalShare</title>
  <style>
    body {{ font-family: system-ui, sans-serif; margin: 0; padding: 32px; background: #0f172a; color: #e2e8f0; }}
    .card {{ max-width: 720px; margin: 0 auto; background: rgba(15, 23, 42, 0.88); border: 1px solid #334155; border-radius: 20px; padding: 28px; }}
    code {{ background: #1e293b; padding: 2px 6px; border-radius: 6px; }}
  </style>
</head>
<body>
  <main class="card">
    <h1>LocalShare</h1>
    <p>LAN service is available on <code>{bind_host}</code>.</p>
    <p>Status: <code>{state:?}</code></p>
    <p>API: <code>/api/v1/health</code>, <code>/api/v1/session</code>, <code>/api/v1/clipboard-items</code></p>
  </main>
</body>
</html>"#,
        bind_host = http_status.bind_host,
        state = http_status.state,
    );

    Html(body)
}

async fn mobile_handler(State(state): State<HttpRuntimeContext>) -> Html<String> {
    let http_status = state.http_server.status();
    let body = format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>LocalShare Mobile</title>
  <style>
    body {{ font-family: system-ui, sans-serif; margin: 0; padding: 24px; background: #111827; color: #f3f4f6; }}
    .card {{ max-width: 640px; margin: 0 auto; background: rgba(17, 24, 39, 0.96); border: 1px solid #374151; border-radius: 18px; padding: 24px; }}
    code {{ background: #1f2937; padding: 2px 6px; border-radius: 6px; }}
  </style>
</head>
<body>
  <main class="card">
    <h1>LocalShare Mobile</h1>
    <p>Use your session token to call the LAN API.</p>
    <p>Mobile path: <code>{mobile_path}</code></p>
    <p>Health endpoint: <code>{health}</code></p>
    <p>Events endpoint: <code>{events}</code></p>
  </main>
</body>
</html>"#,
        mobile_path = http_status.mobile_base_path,
        health = http_status.health_endpoint,
        events = http_status.sse_endpoint,
    );

    Html(body)
}

async fn health_handler(State(state): State<HttpRuntimeContext>) -> Response {
    let http_status = state.http_server.status();
    let data = HealthResponse {
        service: "LocalShare".to_string(),
        status: format!("{:?}", http_status.state).to_lowercase(),
        bind_host: http_status.bind_host,
        preferred_port: http_status.preferred_port,
        effective_port: http_status.effective_port,
        database_ready: state.persistence.status().ready,
        session_ready: state.auth.current_token().ok().flatten().is_some(),
        mobile_base_path: http_status.mobile_base_path,
        health_endpoint: http_status.health_endpoint,
        sse_endpoint: http_status.sse_endpoint,
    };

    api_ok(data)
}

async fn session_handler(
    State(state): State<HttpRuntimeContext>,
    headers: HeaderMap,
    uri: Uri,
    ConnectInfo(_remote_addr): ConnectInfo<SocketAddr>,
) -> Response {
    let token = match extract_token(&headers, &uri) {
        Some(token) => token,
        None => return api_error(AppError::Unauthorized("missing bearer token".to_string())),
    };

    let session = match state.auth.validate_bearer_token(
        state.persistence.as_ref(),
        Some(token.as_str()),
        now_ms(),
    ) {
        Ok(session) => session,
        Err(error) => return api_error(error),
    };

    let response = build_session_response(
        &state,
        &session,
        &token,
        state.network.access_host().to_string(),
    );
    api_ok(response)
}

async fn rotate_token_handler(
    State(state): State<HttpRuntimeContext>,
    headers: HeaderMap,
    uri: Uri,
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
) -> Response {
    if let Err(error) = authorize_request(&state, &headers, &uri) {
        return api_error(error);
    }

    let session = match state
        .auth
        .rotate_session(state.persistence.as_ref(), now_ms())
    {
        Ok(session) => session,
        Err(error) => return api_error(error),
    };

    if let Err(error) = state.persistence.append_audit_log(CreateAuditLogInput {
        action: AuditAction::RotateToken,
        item_id: None,
        ip: Some(remote_addr.ip().to_string()),
        user_agent: None,
        reason: Some("rotate token".to_string()),
    }) {
        warn!(%error, "failed to append session rotate audit log");
    }

    let response = build_session_response(
        &state,
        &session.session,
        &session.token,
        state.network.access_host().to_string(),
    );
    state.http_server.publish_refresh("session", None);
    api_ok(response)
}

async fn list_clipboard_items_handler(
    State(state): State<HttpRuntimeContext>,
    headers: HeaderMap,
    uri: Uri,
    query: Query<ClipboardListQuery>,
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
) -> Response {
    if let Err(error) = authorize_request(&state, &headers, &uri) {
        return api_error(error);
    }

    let query = query.0;
    let items = match state.persistence.list_clipboard_items(&query) {
        Ok(items) => items.into_iter().map(ClipboardItemSummary::from).collect(),
        Err(error) => return api_error(error),
    };

    trace_access("clipboard.list", &remote_addr, None);
    api_ok(ClipboardListResponse { items })
}

async fn get_clipboard_item_handler(
    State(state): State<HttpRuntimeContext>,
    headers: HeaderMap,
    uri: Uri,
    Path(item_id): Path<String>,
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
) -> Response {
    if let Err(error) = authorize_request(&state, &headers, &uri) {
        return api_error(error);
    }

    let item = match state.persistence.get_clipboard_item(&item_id) {
        Ok(Some(item)) => item,
        Ok(None) => {
            return api_error(AppError::NotFound(format!(
                "clipboard item `{item_id}` not found"
            )))
        }
        Err(error) => return api_error(error),
    };

    trace_access("clipboard.detail", &remote_addr, Some(item_id.clone()));
    api_ok(ClipboardItemDetail::from(item))
}

async fn create_clipboard_item_handler(
    State(state): State<HttpRuntimeContext>,
    headers: HeaderMap,
    uri: Uri,
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<ClipboardWriteRequest>,
) -> Response {
    if let Err(error) = authorize_request(&state, &headers, &uri) {
        return api_error(error);
    }

    let input = SaveClipboardItemInput {
        content: payload.content,
        source_kind: SourceKind::MobileWeb,
        source_device_id: None,
        pinned: payload.pinned,
        mark_current: false,
    };

    let mut result = match state.persistence.save_clipboard_item(
        input,
        state.runtime_config.clipboard_poll_interval_ms as i64,
        state.runtime_config.max_text_bytes,
    ) {
        Ok(result) => result,
        Err(error) => return api_error(error),
    };

    if payload.activate {
        if let Err(error) = state.clipboard.write_text(&result.item.content) {
            return api_error(error);
        }

        result.item = match state.persistence.activate_clipboard_item(&result.item.id) {
            Ok(item) => item,
            Err(error) => return api_error(error),
        };
    }

    if let Err(error) = state.persistence.append_audit_log(CreateAuditLogInput {
        action: AuditAction::Create,
        item_id: Some(result.item.id.clone()),
        ip: Some(remote_addr.ip().to_string()),
        user_agent: None,
        reason: Some("mobile submit".to_string()),
    }) {
        warn!(%error, "failed to append clipboard create audit log");
    }

    state
        .http_server
        .publish_refresh("clipboard", Some(result.item.id.clone()));
    trace_access(
        "clipboard.create",
        &remote_addr,
        Some(result.item.id.clone()),
    );
    api_ok(ClipboardWriteResponse {
        item: ClipboardItemDetail::from(result.item),
        created: result.created,
        reused_existing: result.reused_existing,
    })
}

async fn activate_clipboard_item_handler(
    State(state): State<HttpRuntimeContext>,
    headers: HeaderMap,
    uri: Uri,
    Path(item_id): Path<String>,
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
) -> Response {
    if let Err(error) = authorize_request(&state, &headers, &uri) {
        return api_error(error);
    }

    let existing = match state.persistence.get_clipboard_item(&item_id) {
        Ok(Some(item)) => item,
        Ok(None) => {
            return api_error(AppError::NotFound(format!(
                "clipboard item `{item_id}` not found"
            )))
        }
        Err(error) => return api_error(error),
    };

    if let Err(error) = state.clipboard.write_text(&existing.content) {
        return api_error(error);
    }

    let item = match state.persistence.activate_clipboard_item(&item_id) {
        Ok(item) => item,
        Err(error) => return api_error(error),
    };

    if let Err(error) = state.persistence.append_audit_log(CreateAuditLogInput {
        action: AuditAction::Activate,
        item_id: Some(item_id.clone()),
        ip: Some(remote_addr.ip().to_string()),
        user_agent: None,
        reason: Some("api activate".to_string()),
    }) {
        warn!(%error, "failed to append clipboard activate audit log");
    }

    state
        .http_server
        .publish_refresh("clipboard", Some(item_id.clone()));
    trace_access("clipboard.activate", &remote_addr, Some(item_id));
    api_ok(ClipboardItemDetail::from(item))
}

async fn update_clipboard_pin_handler(
    State(state): State<HttpRuntimeContext>,
    headers: HeaderMap,
    uri: Uri,
    Path(item_id): Path<String>,
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<ClipboardPinRequest>,
) -> Response {
    if let Err(error) = authorize_request(&state, &headers, &uri) {
        return api_error(error);
    }

    let item = match state
        .persistence
        .update_clipboard_item_pin(&item_id, payload.pinned)
    {
        Ok(item) => item,
        Err(error) => return api_error(error),
    };

    state
        .http_server
        .publish_refresh("clipboard", Some(item_id.clone()));
    trace_access("clipboard.pin", &remote_addr, Some(item_id));
    api_ok(ClipboardItemDetail::from(item))
}

async fn delete_clipboard_item_handler(
    State(state): State<HttpRuntimeContext>,
    headers: HeaderMap,
    uri: Uri,
    Path(item_id): Path<String>,
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
) -> Response {
    if let Err(error) = authorize_request(&state, &headers, &uri) {
        return api_error(error);
    }

    if let Err(error) = state.persistence.soft_delete_clipboard_item(&item_id) {
        return api_error(error);
    }

    if let Err(error) = state.persistence.append_audit_log(CreateAuditLogInput {
        action: AuditAction::Delete,
        item_id: Some(item_id.clone()),
        ip: Some(remote_addr.ip().to_string()),
        user_agent: None,
        reason: Some("api delete".to_string()),
    }) {
        warn!(%error, "failed to append clipboard delete audit log");
    }

    state
        .http_server
        .publish_refresh("clipboard", Some(item_id.clone()));
    trace_access("clipboard.delete", &remote_addr, Some(item_id.clone()));
    api_ok(ClipboardDeleteResponse { item_id })
}

async fn clear_clipboard_history_handler(
    State(state): State<HttpRuntimeContext>,
    headers: HeaderMap,
    uri: Uri,
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
) -> Response {
    if let Err(error) = authorize_request(&state, &headers, &uri) {
        return api_error(error);
    }

    let cleared = match state.persistence.clear_clipboard_history() {
        Ok(count) => count,
        Err(error) => return api_error(error),
    };

    if let Err(error) = state.persistence.append_audit_log(CreateAuditLogInput {
        action: AuditAction::Delete,
        item_id: None,
        ip: Some(remote_addr.ip().to_string()),
        user_agent: None,
        reason: Some("api clear history".to_string()),
    }) {
        warn!(%error, "failed to append clipboard clear audit log");
    }

    state.http_server.publish_refresh("clipboard", None);
    trace_access("clipboard.clear", &remote_addr, None);
    api_ok(ClipboardClearResponse {
        cleared_count: cleared,
    })
}

async fn events_handler(
    State(state): State<HttpRuntimeContext>,
    headers: HeaderMap,
    uri: Uri,
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
) -> Response {
    if let Err(error) = authorize_request(&state, &headers, &uri) {
        return api_error(error);
    }

    let mut receiver = state.http_server.subscribe();
    let stream = stream! {
        yield Ok::<Event, Infallible>(Event::default().event("ready").data("connected"));
        let mut heartbeat = tokio::time::interval(Duration::from_secs(15));
        loop {
            tokio::select! {
                _ = heartbeat.tick() => {
                    yield Ok(Event::default().comment("keepalive"));
                }
                message = receiver.recv() => {
                    match message {
                        Ok(event) => {
                            match Event::default().event(event.kind.clone()).json_data(&event) {
                                Ok(serialized) => yield Ok(serialized),
                                Err(error) => {
                                    warn!(%error, "failed to serialize sse event");
                                }
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                            let event = ServerEvent::refresh("resync", None, now_ms());
                            if let Ok(serialized) = Event::default().event("refresh").json_data(&event) {
                                yield Ok(serialized);
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    }
                }
            }
        }
    };

    trace_access("events.subscribe", &remote_addr, None);
    Sse::new(stream)
        .keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(15))
                .text("keepalive"),
        )
        .into_response()
}

fn build_session_response(
    state: &HttpRuntimeContext,
    session: &crate::persistence::SessionRecord,
    token: &str,
    public_host: String,
) -> SessionResponse {
    let http_status = state.http_server.status();
    let public_port = http_status
        .effective_port
        .unwrap_or(http_status.preferred_port);
    let access_url = build_access_url(
        &public_host,
        public_port,
        &state.runtime_config.mobile_route,
        token,
    );

    SessionResponse {
        device_name: state.network.device_name().to_string(),
        public_host,
        public_port,
        access_url,
        health_endpoint: http_status.health_endpoint,
        sse_endpoint: http_status.sse_endpoint,
        mobile_base_path: http_status.mobile_base_path,
        session_id: session.id.clone(),
        session_status: session.status,
        expires_at: session.expires_at,
        token_ttl_minutes: state.auth.token_ttl_minutes(),
        bearer_header_name: "Authorization".to_string(),
        token_query_key: "token".to_string(),
        rotation_enabled: true,
        max_text_bytes: state.runtime_config.max_text_bytes,
        read_only: false,
    }
}

fn authorize_request(
    state: &HttpRuntimeContext,
    headers: &HeaderMap,
    uri: &Uri,
) -> Result<crate::persistence::SessionRecord, AppError> {
    let token = extract_token(headers, uri);
    state
        .auth
        .validate_bearer_token(state.persistence.as_ref(), token.as_deref(), now_ms())
}

fn extract_token(headers: &HeaderMap, uri: &Uri) -> Option<String> {
    if let Some(value) = headers.get(header::AUTHORIZATION) {
        if let Ok(raw) = value.to_str() {
            if let Some(token) = raw
                .strip_prefix("Bearer ")
                .or_else(|| raw.strip_prefix("bearer "))
            {
                let token = token.trim();
                if !token.is_empty() {
                    return Some(token.to_string());
                }
            }
        }
    }

    uri.query().and_then(|query| {
        query.split('&').find_map(|pair| {
            let (key, value) = pair.split_once('=')?;
            if key == "token" {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            } else {
                None
            }
        })
    })
}

fn trace_access(action: &str, remote_addr: &SocketAddr, item_id: Option<String>) {
    info!(
        action = %action,
        remote = %remote_addr,
        item_id = ?item_id,
        "lan api request processed"
    );
}

fn api_ok<T: serde::Serialize>(data: T) -> Response {
    let ts = now_ms();
    (StatusCode::OK, Json(ApiEnvelope::ok(data, ts))).into_response()
}

fn api_error(error: AppError) -> Response {
    let ts = now_ms();
    let status =
        StatusCode::from_u16(error.http_status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    (
        status,
        Json(ApiEnvelope::<serde_json::Value>::err(error, ts)),
    )
        .into_response()
}

fn remote_key_from_request(req: &axum::http::Request<axum::body::Body>) -> Option<String> {
    req.extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|connect_info| connect_info.0.ip().to_string())
}

#[derive(Debug, Clone, Copy)]
enum RequestKind {
    Read,
    Write,
    Sse,
}

fn classify_request(method: &Method, path: &str) -> Option<RequestKind> {
    if path == "/api/v1/health" {
        return None;
    }

    if path == "/api/v1/events" {
        return Some(RequestKind::Sse);
    }

    if !path.starts_with("/api/v1/") {
        return None;
    }

    match method {
        &Method::GET => Some(RequestKind::Read),
        _ => Some(RequestKind::Write),
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}
