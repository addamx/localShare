mod events;
mod worker;

use std::sync::Arc;

use serde::Serialize;

use crate::error::AppError;
use crate::persistence::PersistenceRepository;

#[allow(unused_imports)]
pub use events::{
    ClipboardEventHub, ClipboardEventReceiver, ClipboardRefreshEvent, CLIPBOARD_REFRESH_EVENT_NAME,
};

#[derive(Debug)]
pub struct ClipboardService {
    poll_interval_ms: u64,
    max_text_bytes: usize,
    dedup_window_ms: u64,
    runtime: std::sync::Mutex<Option<worker::ClipboardRuntime>>,
    events: Arc<ClipboardEventHub>,
}

impl ClipboardService {
    pub fn new(poll_interval_ms: u64, max_text_bytes: usize) -> Self {
        Self {
            poll_interval_ms,
            max_text_bytes,
            dedup_window_ms: Self::derive_dedup_window_ms(poll_interval_ms),
            runtime: std::sync::Mutex::new(None),
            events: Arc::new(ClipboardEventHub::new()),
        }
    }

    pub fn status(&self) -> ClipboardStatus {
        let running = self
            .runtime
            .lock()
            .map(|runtime| runtime.is_some())
            .unwrap_or(false);
        let subscriber_count = self.events.subscriber_count();

        ClipboardStatus {
            mode: "polling".to_string(),
            poll_interval_ms: self.poll_interval_ms,
            dedup_window_ms: self.dedup_window_ms,
            max_text_bytes: self.max_text_bytes,
            current_item_tracking: true,
            running,
            subscriber_count,
            refresh_event_topic: CLIPBOARD_REFRESH_EVENT_NAME.to_string(),
        }
    }

    pub fn subscribe(&self) -> ClipboardEventReceiver {
        self.events.subscribe()
    }

    pub fn is_running(&self) -> bool {
        self.runtime
            .lock()
            .map(|runtime| runtime.is_some())
            .unwrap_or(false)
    }

    pub fn write_text(&self, text: &str) -> Result<(), AppError> {
        worker::write_text(text)
    }

    pub fn start<R>(
        &self,
        persistence: Arc<R>,
        source_device_id: Option<String>,
    ) -> Result<(), AppError>
    where
        R: PersistenceRepository + Send + Sync + 'static,
    {
        let mut runtime = self
            .runtime
            .lock()
            .map_err(|_| AppError::State("clipboard runtime lock poisoned".to_string()))?;

        if runtime.is_some() {
            return Err(AppError::State(
                "clipboard listener already running".to_string(),
            ));
        }

        let worker = worker::spawn(worker::ClipboardWorkerConfig {
            poll_interval_ms: self.poll_interval_ms,
            max_text_bytes: self.max_text_bytes,
            dedup_window_ms: self.dedup_window_ms,
            persistence,
            source_device_id,
            events: Arc::clone(&self.events),
        })?;

        *runtime = Some(worker);
        Ok(())
    }

    pub fn stop(&self) -> bool {
        let runtime = self.runtime.lock().ok().and_then(|mut guard| guard.take());
        if let Some(runtime) = runtime {
            runtime.stop();
            return true;
        }

        false
    }

    fn derive_dedup_window_ms(poll_interval_ms: u64) -> u64 {
        poll_interval_ms.saturating_mul(2).max(1_500).min(10_000)
    }
}

impl Drop for ClipboardService {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardStatus {
    pub mode: String,
    pub poll_interval_ms: u64,
    pub dedup_window_ms: u64,
    pub max_text_bytes: usize,
    pub current_item_tracking: bool,
    pub running: bool,
    pub subscriber_count: usize,
    pub refresh_event_topic: String,
}
