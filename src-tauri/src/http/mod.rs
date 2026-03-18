use serde::Serialize;

#[derive(Debug)]
pub struct HttpServer {
    bind_host: String,
    preferred_port: u16,
    mobile_base_path: String,
}

impl HttpServer {
    pub fn new(bind_host: String, preferred_port: u16, mobile_base_path: String) -> Self {
        Self {
            bind_host,
            preferred_port,
            mobile_base_path,
        }
    }

    pub fn status(&self) -> HttpServerStatus {
        HttpServerStatus {
            bind_host: self.bind_host.clone(),
            preferred_port: self.preferred_port,
            health_endpoint: "/api/v1/health".to_string(),
            mobile_base_path: self.mobile_base_path.clone(),
            sse_endpoint: "/api/v1/events".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpServerStatus {
    pub bind_host: String,
    pub preferred_port: u16,
    pub health_endpoint: String,
    pub mobile_base_path: String,
    pub sse_endpoint: String,
}
