use std::collections::HashMap;
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::http::Method;
use serde::Serialize;
use tokio::runtime::Builder as TokioRuntimeBuilder;
use tokio::sync::broadcast;
use tracing::{error, info, warn};
use tower_http::cors::{Any, CorsLayer};

use crate::error::AppError;

use super::handlers::build_router;
use super::state::HttpRuntimeContext;
use super::types::ServerEvent;

const MAX_PORT_ATTEMPTS: u16 = 8;
const READ_LIMIT_PER_MINUTE: u32 = 240;
const WRITE_LIMIT_PER_MINUTE: u32 = 60;
const SSE_LIMIT_PER_MINUTE: u32 = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HttpServerState {
    Stopped,
    Starting,
    Running,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpServerStatus {
    pub bind_host: String,
    pub preferred_port: u16,
    pub effective_port: Option<u16>,
    pub state: HttpServerState,
    pub last_error: Option<String>,
    pub health_endpoint: String,
    pub mobile_base_path: String,
    pub sse_endpoint: String,
}

#[derive(Debug)]
pub struct HttpServer {
    bind_host: String,
    preferred_port: u16,
    mobile_base_path: String,
    runtime: Arc<Mutex<HttpServerRuntime>>,
    events: broadcast::Sender<ServerEvent>,
    limiter: Arc<RequestLimiter>,
}

#[derive(Debug)]
struct HttpServerRuntime {
    state: HttpServerState,
    effective_port: Option<u16>,
    last_error: Option<String>,
    started_at: Option<i64>,
}

#[derive(Debug)]
struct RequestLimiter {
    buckets: Mutex<HashMap<String, RateBucket>>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum RequestClass {
    Read,
    Write,
    Sse,
}

#[derive(Debug, Clone, Copy)]
struct RateBucket {
    window_start_ms: i64,
    read_count: u32,
    write_count: u32,
    sse_count: u32,
}

impl Default for HttpServerRuntime {
    fn default() -> Self {
        Self {
            state: HttpServerState::Stopped,
            effective_port: None,
            last_error: None,
            started_at: None,
        }
    }
}

impl Default for RateBucket {
    fn default() -> Self {
        Self {
            window_start_ms: 0,
            read_count: 0,
            write_count: 0,
            sse_count: 0,
        }
    }
}

impl RequestLimiter {
    fn new() -> Self {
        Self {
            buckets: Mutex::new(HashMap::new()),
        }
    }

    fn allow(&self, key: &str, class: RequestClass, now_ms: i64) -> Result<(), AppError> {
        let mut buckets = self
            .buckets
            .lock()
            .map_err(|_| AppError::State("http rate limiter lock poisoned".to_string()))?;
        let bucket = buckets.entry(key.to_string()).or_default();
        if bucket.window_start_ms == 0 || now_ms.saturating_sub(bucket.window_start_ms) >= 60_000 {
            *bucket = RateBucket {
                window_start_ms: now_ms,
                ..RateBucket::default()
            };
        }

        let limit = match class {
            RequestClass::Read => {
                bucket.read_count += 1;
                READ_LIMIT_PER_MINUTE
            }
            RequestClass::Write => {
                bucket.write_count += 1;
                WRITE_LIMIT_PER_MINUTE
            }
            RequestClass::Sse => {
                bucket.sse_count += 1;
                SSE_LIMIT_PER_MINUTE
            }
        };

        let count = match class {
            RequestClass::Read => bucket.read_count,
            RequestClass::Write => bucket.write_count,
            RequestClass::Sse => bucket.sse_count,
        };

        if count > limit {
            return Err(AppError::RateLimited(format!(
                "request limit exceeded for `{key}`"
            )));
        }

        buckets.retain(|_, entry| now_ms.saturating_sub(entry.window_start_ms) < 120_000);
        Ok(())
    }
}

impl HttpServer {
    pub fn new(bind_host: String, preferred_port: u16, mobile_base_path: String) -> Self {
        Self {
            bind_host,
            preferred_port,
            mobile_base_path,
            runtime: Arc::new(Mutex::new(HttpServerRuntime::default())),
            events: broadcast::channel(64).0,
            limiter: Arc::new(RequestLimiter::new()),
        }
    }

    pub fn status(&self) -> HttpServerStatus {
        let runtime = self.runtime.lock().ok();
        let runtime = runtime.as_ref();
        HttpServerStatus {
            bind_host: self.bind_host.clone(),
            preferred_port: self.preferred_port,
            effective_port: runtime.and_then(|runtime| runtime.effective_port),
            state: runtime
                .map(|runtime| runtime.state)
                .unwrap_or(HttpServerState::Stopped),
            last_error: runtime.and_then(|runtime| runtime.last_error.clone()),
            health_endpoint: "/api/v1/health".to_string(),
            mobile_base_path: self.mobile_base_path.clone(),
            sse_endpoint: "/api/v1/events".to_string(),
        }
    }

    pub fn start(&self, context: HttpRuntimeContext) -> Result<(), AppError> {
        {
            let mut runtime = self
                .runtime
                .lock()
                .map_err(|_| AppError::State("http runtime lock poisoned".to_string()))?;
            if matches!(
                runtime.state,
                HttpServerState::Running | HttpServerState::Starting
            ) {
                return Ok(());
            }
            runtime.state = HttpServerState::Starting;
            runtime.last_error = None;
        }

        let server = self.clone();
        thread::Builder::new()
            .name("localshare-http".to_string())
            .spawn(move || {
                let runtime = TokioRuntimeBuilder::new_multi_thread().enable_all().build();
                let runtime = match runtime {
                    Ok(runtime) => runtime,
                    Err(error) => {
                        server.mark_failed(format!("failed to build runtime: {error}"));
                        return;
                    }
                };

                runtime.block_on(async move {
                    if let Err(error) = server.run(context).await {
                        server.mark_failed(error.to_string());
                        error!(%error, "localshare http server stopped");
                    }
                });
            })
            .map_err(|error| AppError::State(format!("failed to spawn http thread: {error}")))?;

        Ok(())
    }

    pub fn wait_until_ready(&self, timeout: Duration) -> Result<HttpServerStatus, AppError> {
        let start = std::time::Instant::now();

        loop {
            let status = self.status();
            match status.state {
                HttpServerState::Running => return Ok(status),
                HttpServerState::Failed => {
                    return Err(AppError::State(
                        status
                            .last_error
                            .unwrap_or_else(|| "http server failed to start".to_string()),
                    ))
                }
                HttpServerState::Stopped | HttpServerState::Starting => {}
            }

            if start.elapsed() >= timeout {
                return Err(AppError::State(
                    "http server did not become ready before timeout".to_string(),
                ));
            }

            thread::sleep(Duration::from_millis(50));
        }
    }

    pub fn publish_refresh(&self, scope: impl Into<String>, item_id: Option<String>) {
        let event = ServerEvent::refresh(scope.into(), item_id, now_ms());
        if let Err(error) = self.events.send(event) {
            warn!(%error, "failed to broadcast http event");
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ServerEvent> {
        self.events.subscribe()
    }

    pub fn allow_request(&self, remote_key: &str, class: RequestClass) -> Result<(), AppError> {
        self.limiter.allow(remote_key, class, now_ms())
    }

    pub fn allow_read_request(&self, remote_key: &str) -> Result<(), AppError> {
        self.allow_request(remote_key, RequestClass::Read)
    }

    pub fn allow_write_request(&self, remote_key: &str) -> Result<(), AppError> {
        self.allow_request(remote_key, RequestClass::Write)
    }

    pub fn allow_sse_request(&self, remote_key: &str) -> Result<(), AppError> {
        self.allow_request(remote_key, RequestClass::Sse)
    }

    async fn run(&self, context: HttpRuntimeContext) -> Result<(), AppError> {
        let mut last_error = None;
        for offset in 0..=MAX_PORT_ATTEMPTS {
            let port = context.runtime_config.preferred_port.saturating_add(offset);
            let addr = format!("{}:{}", self.bind_host, port);
            match TcpListener::bind(&addr) {
                Ok(listener) => {
                    listener.set_nonblocking(true).map_err(|error| {
                        AppError::State(format!("failed to set nonblocking listener: {error}"))
                    })?;
                    let tokio_listener = tokio::net::TcpListener::from_std(listener)?;
                    self.mark_running(port);
                    info!(
                        bind_host = %self.bind_host,
                        effective_port = port,
                        mobile_base_path = %self.mobile_base_path,
                        "localshare http server bound"
                    );
                    let router = build_router(context.clone());
                    axum::serve(
                        tokio_listener,
                        router
                            .layer(cors_layer())
                            .into_make_service_with_connect_info::<std::net::SocketAddr>(),
                    )
                    .await
                    .map_err(AppError::from)?;
                    return Ok(());
                }
                Err(error) => {
                    last_error = Some(error.to_string());
                    warn!(
                        bind_host = %self.bind_host,
                        candidate_port = port,
                        error = %error,
                        "localshare http port bind failed"
                    );
                }
            }
        }

        let error = last_error.unwrap_or_else(|| "failed to bind any HTTP port".to_string());
        self.mark_failed(error.clone());
        Err(AppError::State(error))
    }

    fn mark_running(&self, port: u16) {
        if let Ok(mut runtime) = self.runtime.lock() {
            runtime.state = HttpServerState::Running;
            runtime.effective_port = Some(port);
            runtime.last_error = None;
            runtime.started_at = Some(now_ms());
        }
    }

    fn mark_failed(&self, error: String) {
        if let Ok(mut runtime) = self.runtime.lock() {
            runtime.state = HttpServerState::Failed;
            runtime.last_error = Some(error.clone());
        }
        error!(%error, "localshare http server failed");
    }
}

impl Clone for HttpServer {
    fn clone(&self) -> Self {
        Self {
            bind_host: self.bind_host.clone(),
            preferred_port: self.preferred_port,
            mobile_base_path: self.mobile_base_path.clone(),
            runtime: self.runtime.clone(),
            events: self.events.clone(),
            limiter: self.limiter.clone(),
        }
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any)
        .allow_headers([axum::http::header::AUTHORIZATION, axum::http::header::CONTENT_TYPE])
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
}
