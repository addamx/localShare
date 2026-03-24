use std::sync::Arc;

use crate::auth::AuthService;
use crate::clipboard::ClipboardService;
use crate::config::RuntimeConfig;
use crate::network::NetworkService;
use crate::persistence::PersistenceLayer;

use super::server::HttpServer;

#[derive(Clone)]
pub struct HttpRuntimeContext {
    pub runtime_config: RuntimeConfig,
    pub http_server: Arc<HttpServer>,
    pub auth: Arc<AuthService>,
    pub clipboard: Arc<ClipboardService>,
    pub persistence: Arc<PersistenceLayer>,
    pub network: Arc<NetworkService>,
}
