use serde::Serialize;

#[derive(Debug)]
pub struct NetworkService {
    device_name: String,
    access_host: String,
    access_hosts: Vec<String>,
}

impl NetworkService {
    pub fn new() -> Self {
        let device_name = std::env::var("HOSTNAME")
            .or_else(|_| std::env::var("COMPUTERNAME"))
            .unwrap_or_else(|_| "desktop-host".to_string());
        let access_hosts = resolve_access_hosts();
        let access_host = select_primary_access_host(&access_hosts);

        Self {
            device_name,
            access_host,
            access_hosts,
        }
    }

    pub fn device_name(&self) -> &str {
        &self.device_name
    }

    pub fn access_host(&self) -> &str {
        &self.access_host
    }

    pub fn access_hosts(&self) -> &[String] {
        &self.access_hosts
    }

    pub fn status(&self) -> NetworkStatus {
        NetworkStatus {
            device_name: self.device_name.clone(),
            access_host: self.access_host.clone(),
            access_hosts: self.access_hosts.clone(),
            lan_discovery_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkStatus {
    pub device_name: String,
    pub access_host: String,
    pub access_hosts: Vec<String>,
    pub lan_discovery_enabled: bool,
}

fn resolve_access_hosts() -> Vec<String> {
    let mut preferred = Vec::new();
    let mut fallback = Vec::new();

    if let Ok(addresses) = if_addrs::get_if_addrs() {
        for address in addresses {
            let ip = match address.ip() {
                std::net::IpAddr::V4(ip) => ip,
                std::net::IpAddr::V6(_) => continue,
            };

            if ip.is_loopback() || ip.is_link_local() || ip.is_broadcast() {
                continue;
            }

            let ip_text = ip.to_string();
            if is_private_ipv4(ip) {
                preferred.push(ip_text);
            } else {
                fallback.push(ip_text);
            }
        }
    }

    preferred.extend(fallback);
    preferred.sort();
    preferred.dedup();

    if preferred.is_empty() {
        preferred.push("127.0.0.1".to_string());
    }

    preferred
}

fn select_primary_access_host(candidates: &[String]) -> String {
    let detected = std::net::UdpSocket::bind("0.0.0.0:0")
        .and_then(|socket| {
            socket.connect("8.8.8.8:80")?;
            socket.local_addr()
        })
        .map(|addr| addr.ip().to_string())
        .ok();

    if let Some(detected) = detected {
        if candidates.iter().any(|candidate| candidate == &detected) {
            return detected;
        }
    }

    candidates
        .first()
        .cloned()
        .unwrap_or_else(|| "127.0.0.1".to_string())
}

fn is_private_ipv4(ip: std::net::Ipv4Addr) -> bool {
    let [a, b, ..] = ip.octets();
    a == 10 || (a == 172 && (16..=31).contains(&b)) || (a == 192 && b == 168)
}
