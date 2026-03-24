use std::sync::Arc;

use serde::Serialize;

use crate::auth::{AuthService, AuthStatus, SessionSnapshot};
use crate::clipboard::{ClipboardService, ClipboardStatus};
use crate::config::{AppPaths, RuntimeConfig};
use crate::error::AppError;
use crate::http::{HttpRuntimeContext, HttpServer, HttpServerStatus};
use crate::network::{NetworkService, NetworkStatus};
use crate::persistence::{PersistenceLayer, PersistenceRepository, PersistenceStatus};

pub struct AppState {
    pub runtime_config: RuntimeConfig,
    pub paths: AppPaths,
    pub device_id: String,
    pub clipboard: Arc<ClipboardService>,
    pub http_server: Arc<HttpServer>,
    pub auth: Arc<AuthService>,
    pub persistence: Arc<PersistenceLayer>,
    pub network: Arc<NetworkService>,
}

impl AppState {
    pub fn new(runtime_config: RuntimeConfig, paths: AppPaths) -> Result<Self, AppError> {
        let network = Arc::new(NetworkService::new());
        let persistence = Arc::new(PersistenceLayer::new(paths.database_path.clone())?);
        let device = persistence.upsert_device(network.device_name())?;

        let clipboard = Arc::new(ClipboardService::new(
            runtime_config.clipboard_poll_interval_ms,
            runtime_config.max_text_bytes,
        ));
        let http_server = Arc::new(HttpServer::new(
            runtime_config.lan_host.clone(),
            runtime_config.preferred_port,
            runtime_config.mobile_route.clone(),
        ));
        let auth = Arc::new(AuthService::new(runtime_config.token_ttl_minutes));

        auth.ensure_session(persistence.as_ref(), now_ms())?;

        Ok(Self {
            runtime_config,
            paths,
            device_id: device.id,
            clipboard,
            http_server,
            auth,
            persistence,
            network,
        })
    }

    pub fn bootstrap(&self) -> Result<AppBootstrap, AppError> {
        let session = self
            .auth
            .ensure_session(self.persistence.as_ref(), now_ms())?;
        let http_status = self.http_server.status();
        let effective_port = http_status
            .effective_port
            .unwrap_or(http_status.preferred_port);
        let session_snapshot = self.auth.current_session_snapshot(
            &session.session,
            self.network.access_host(),
            effective_port,
            &self.runtime_config.mobile_route,
        )?;

        Ok(AppBootstrap {
            app_name: "LocalShare".to_string(),
            routes: RouteOverview {
                desktop: "/".to_string(),
                mobile: self.runtime_config.mobile_route.clone(),
            },
            runtime_config: self.runtime_config.clone(),
            paths: self.paths.clone(),
            services: ServiceOverview {
                clipboard: self.clipboard.status(),
                http_server: http_status,
                auth: self.auth.status(),
                session: session_snapshot,
                persistence: self.persistence.status(),
                network: self.network.status(),
            },
        })
    }

    pub fn http_context(&self) -> HttpRuntimeContext {
        HttpRuntimeContext {
            runtime_config: self.runtime_config.clone(),
            http_server: self.http_server.clone(),
            auth: self.auth.clone(),
            clipboard: self.clipboard.clone(),
            persistence: self.persistence.clone(),
            network: self.network.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteOverview {
    pub desktop: String,
    pub mobile: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceOverview {
    pub clipboard: ClipboardStatus,
    #[serde(rename = "httpServer")]
    pub http_server: HttpServerStatus,
    pub auth: AuthStatus,
    pub session: SessionSnapshot,
    pub persistence: PersistenceStatus,
    pub network: NetworkStatus,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppBootstrap {
    pub app_name: String,
    pub routes: RouteOverview,
    pub runtime_config: RuntimeConfig,
    pub paths: AppPaths,
    pub services: ServiceOverview,
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}
