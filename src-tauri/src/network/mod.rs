use serde::Serialize;

#[derive(Debug)]
pub struct NetworkService {
    device_name: String,
}

impl NetworkService {
    pub fn new() -> Self {
        let device_name = std::env::var("HOSTNAME")
            .or_else(|_| std::env::var("COMPUTERNAME"))
            .unwrap_or_else(|_| "desktop-host".to_string());

        Self { device_name }
    }

    pub fn status(&self) -> NetworkStatus {
        NetworkStatus {
            device_name: self.device_name.clone(),
            lan_discovery_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkStatus {
    pub device_name: String,
    pub lan_discovery_enabled: bool,
}
