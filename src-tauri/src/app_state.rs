use serde::Serialize;

use crate::auth::{AuthService, AuthStatus};
use crate::clipboard::{ClipboardService, ClipboardStatus};
use crate::config::{AppPaths, RuntimeConfig};
use crate::http::{HttpServer, HttpServerStatus};
use crate::network::{NetworkService, NetworkStatus};
use crate::persistence::{PersistenceLayer, PersistenceStatus};

pub struct AppState {
    pub runtime_config: RuntimeConfig,
    pub paths: AppPaths,
    pub clipboard: ClipboardService,
    pub http_server: HttpServer,
    pub auth: AuthService,
    pub persistence: PersistenceLayer,
    pub network: NetworkService,
}

impl AppState {
    pub fn new(runtime_config: RuntimeConfig, paths: AppPaths) -> Self {
        let persistence = PersistenceLayer::new(paths.database_path.clone());
        let clipboard = ClipboardService::new(
            runtime_config.clipboard_poll_interval_ms,
            runtime_config.max_text_bytes,
        );
        let http_server =
            HttpServer::new(runtime_config.lan_host.clone(), runtime_config.preferred_port, runtime_config.mobile_route.clone());
        let auth = AuthService::new(runtime_config.token_ttl_minutes);
        let network = NetworkService::new();

        Self {
            runtime_config,
            paths,
            clipboard,
            http_server,
            auth,
            persistence,
            network,
        }
    }

    pub fn bootstrap(&self) -> AppBootstrap {
        AppBootstrap {
            app_name: "LocalShare".to_string(),
            routes: RouteOverview {
                desktop: "/".to_string(),
                mobile: self.runtime_config.mobile_route.clone(),
            },
            runtime_config: self.runtime_config.clone(),
            paths: self.paths.clone(),
            services: ServiceOverview {
                clipboard: self.clipboard.status(),
                http_server: self.http_server.status(),
                auth: self.auth.status(),
                persistence: self.persistence.status(),
                network: self.network.status(),
            },
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
