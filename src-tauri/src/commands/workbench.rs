use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

use serde::Serialize;
use tauri::State;

use crate::app_state::AppState;
use crate::auth::SessionSnapshot;
use crate::error::AppError;
use crate::persistence::{
    ClipboardItemRecord, ClipboardListQuery, PersistenceRepository,
};

#[tauri::command]
pub fn list_clipboard_items(
    state: State<'_, AppState>,
    query: Option<ClipboardListQuery>,
) -> Result<Vec<ClipboardItemRecord>, AppError> {
    state
        .persistence
        .list_clipboard_items(&query.unwrap_or_default())
}

#[tauri::command]
pub fn get_clipboard_item(
    state: State<'_, AppState>,
    item_id: String,
) -> Result<ClipboardItemRecord, AppError> {
    state
        .persistence
        .get_clipboard_item(&item_id)?
        .ok_or_else(|| AppError::NotFound(format!("clipboard item `{item_id}` not found")))
}

#[tauri::command]
pub fn activate_clipboard_item(
    state: State<'_, AppState>,
    item_id: String,
) -> Result<ClipboardItemRecord, AppError> {
    let existing = state
        .persistence
        .get_clipboard_item(&item_id)?
        .ok_or_else(|| AppError::NotFound(format!("clipboard item `{item_id}` not found")))?;
    state.clipboard.write_text(&existing.content)?;
    let item = state.persistence.activate_clipboard_item(&item_id)?;
    state
        .http_server
        .publish_refresh("clipboard", Some(item_id.clone()));
    Ok(item)
}

#[tauri::command]
pub fn update_clipboard_item_pin(
    state: State<'_, AppState>,
    item_id: String,
    pinned: bool,
) -> Result<ClipboardItemRecord, AppError> {
    let item = state.persistence.update_clipboard_item_pin(&item_id, pinned)?;
    state
        .http_server
        .publish_refresh("clipboard", Some(item_id.clone()));
    Ok(item)
}

#[tauri::command]
pub fn delete_clipboard_item(state: State<'_, AppState>, item_id: String) -> Result<(), AppError> {
    state.persistence.soft_delete_clipboard_item(&item_id)?;
    state
        .http_server
        .publish_refresh("clipboard", Some(item_id.clone()));
    Ok(())
}

#[tauri::command]
pub fn clear_clipboard_history(state: State<'_, AppState>) -> Result<usize, AppError> {
    let cleared = state.persistence.clear_clipboard_history()?;
    state.http_server.publish_refresh("clipboard", None);
    Ok(cleared)
}

#[tauri::command]
pub fn rotate_session_token(state: State<'_, AppState>) -> Result<SessionSnapshot, AppError> {
    let session = state
        .auth
        .rotate_session(state.persistence.as_ref(), now_ms())?;
    let http_status = state.http_server.status();
    let effective_port = http_status
        .effective_port
        .unwrap_or(http_status.preferred_port);

    let snapshot = state.auth.current_session_snapshot(
        &session.session,
        state.network.access_host(),
        effective_port,
        &state.runtime_config.mobile_route,
    )?;
    state.http_server.publish_refresh("session", None);
    Ok(snapshot)
}

#[tauri::command]
pub fn get_connectivity_report(state: State<'_, AppState>) -> Result<ConnectivityReport, AppError> {
    let http_status = state.http_server.status();
    let port = http_status
        .effective_port
        .unwrap_or(http_status.preferred_port);
    let access_url = state.bootstrap()?.services.session.access_url;

    let mut hosts = vec!["127.0.0.1".to_string(), "localhost".to_string()];
    hosts.push(state.network.access_host().to_string());
    hosts.extend(state.network.access_hosts().iter().cloned());
    hosts.sort();
    hosts.dedup();

    let checks = hosts
        .into_iter()
        .map(|host| probe_host(&host, port))
        .collect::<Vec<_>>();

    Ok(ConnectivityReport {
        bind_host: http_status.bind_host,
        preferred_port: http_status.preferred_port,
        effective_port: port,
        server_state: format!("{:?}", http_status.state).to_lowercase(),
        server_error: http_status.last_error,
        access_url,
        checks,
    })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectivityReport {
    pub bind_host: String,
    pub preferred_port: u16,
    pub effective_port: u16,
    pub server_state: String,
    pub server_error: Option<String>,
    pub access_url: String,
    pub checks: Vec<HostCheck>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostCheck {
    pub host: String,
    pub url: String,
    pub tcp_ok: bool,
    pub http_ok: bool,
    pub http_status_line: Option<String>,
    pub error: Option<String>,
}

fn probe_host(host: &str, port: u16) -> HostCheck {
    let url = format!("http://{host}:{port}/api/v1/health");
    let timeout = Duration::from_millis(900);

    let socket_addr = match resolve_socket_addr(host, port) {
        Ok(addr) => addr,
        Err(error) => {
            return HostCheck {
                host: host.to_string(),
                url,
                tcp_ok: false,
                http_ok: false,
                http_status_line: None,
                error: Some(error),
            }
        }
    };

    let mut stream = match TcpStream::connect_timeout(&socket_addr, timeout) {
        Ok(stream) => stream,
        Err(error) => {
            return HostCheck {
                host: host.to_string(),
                url,
                tcp_ok: false,
                http_ok: false,
                http_status_line: None,
                error: Some(format!("tcp connect failed: {error}")),
            }
        }
    };

    let _ = stream.set_read_timeout(Some(timeout));
    let _ = stream.set_write_timeout(Some(timeout));
    let request = format!(
        "GET /api/v1/health HTTP/1.1\r\nHost: {host}:{port}\r\nConnection: close\r\n\r\n"
    );

    if let Err(error) = stream.write_all(request.as_bytes()) {
        return HostCheck {
            host: host.to_string(),
            url,
            tcp_ok: true,
            http_ok: false,
            http_status_line: None,
            error: Some(format!("http write failed: {error}")),
        };
    }

    let mut response = String::new();
    if let Err(error) = stream.read_to_string(&mut response) {
        return HostCheck {
            host: host.to_string(),
            url,
            tcp_ok: true,
            http_ok: false,
            http_status_line: None,
            error: Some(format!("http read failed: {error}")),
        };
    }

    let status_line = response.lines().next().map(str::to_string);
    let http_ok = status_line
        .as_deref()
        .is_some_and(|line| line.contains(" 200 "));

    HostCheck {
        host: host.to_string(),
        url,
        tcp_ok: true,
        http_ok,
        http_status_line: status_line,
        error: if http_ok {
            None
        } else {
            Some("health endpoint did not return 200".to_string())
        },
    }
}

fn resolve_socket_addr(host: &str, port: u16) -> Result<SocketAddr, String> {
    (host, port)
        .to_socket_addrs()
        .map_err(|error| format!("resolve failed: {error}"))?
        .find(|addr| addr.is_ipv4())
        .ok_or_else(|| "no ipv4 address resolved".to_string())
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}
