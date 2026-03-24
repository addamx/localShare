use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use arboard::Clipboard;
use tracing::{debug, warn};

use crate::error::AppError;
use crate::persistence::{
    PersistenceLayer, PersistenceRepository, SaveClipboardItemInput, SourceKind,
};

use super::events::{ClipboardEventHub, ClipboardRefreshEvent};

pub(crate) struct ClipboardWorkerConfig<R>
where
    R: PersistenceRepository + Send + Sync + 'static,
{
    pub poll_interval_ms: u64,
    pub max_text_bytes: usize,
    pub dedup_window_ms: u64,
    pub persistence: Arc<R>,
    pub source_device_id: Option<String>,
    pub events: Arc<ClipboardEventHub>,
}

#[derive(Debug)]
pub(crate) struct ClipboardRuntime {
    stop_flag: Arc<AtomicBool>,
    join_handle: Option<JoinHandle<()>>,
}

impl ClipboardRuntime {
    pub fn stop(mut self) {
        self.stop_flag.store(true, Ordering::Relaxed);
        if let Some(join_handle) = self.join_handle.take() {
            let _ = join_handle.join();
        }
    }
}

pub(crate) fn spawn<R>(config: ClipboardWorkerConfig<R>) -> Result<ClipboardRuntime, AppError>
where
    R: PersistenceRepository + Send + Sync + 'static,
{
    let stop_flag = Arc::new(AtomicBool::new(false));
    let thread_stop_flag = Arc::clone(&stop_flag);

    let join_handle = thread::Builder::new()
        .name("localshare-clipboard".to_string())
        .spawn(move || run_polling_loop(config, thread_stop_flag))
        .map_err(AppError::Io)?;

    Ok(ClipboardRuntime {
        stop_flag,
        join_handle: Some(join_handle),
    })
}

pub(crate) fn write_text(text: &str) -> Result<(), AppError> {
    if text.trim().is_empty() {
        return Err(AppError::Validation(
            "clipboard content cannot be empty".to_string(),
        ));
    }

    let mut clipboard = Clipboard::new()
        .map_err(|error| AppError::State(format!("failed to access system clipboard: {error}")))?;
    clipboard
        .set_text(text.to_string())
        .map_err(|error| AppError::State(format!("failed to write system clipboard: {error}")))?;

    Ok(())
}

fn run_polling_loop<R>(config: ClipboardWorkerConfig<R>, stop_flag: Arc<AtomicBool>)
where
    R: PersistenceRepository + Send + Sync + 'static,
{
    let mut clipboard: Option<Clipboard> = None;
    let mut state = ClipboardWorkerState::default();
    let poll_interval = Duration::from_millis(config.poll_interval_ms.max(100));
    let failure_retry_ms = config
        .dedup_window_ms
        .saturating_mul(2)
        .max(3_000)
        .min(30_000);

    while !stop_flag.load(Ordering::Relaxed) {
        if let Some(text) = read_current_text(&mut clipboard) {
            let observed_hash = PersistenceLayer::hash_text(&text);
            let now = now_ms();

            if state.should_skip(&observed_hash, now, failure_retry_ms) {
                sleep_until_next_tick(&stop_flag, poll_interval);
                continue;
            }

            if text.trim().is_empty() {
                debug!(event = "clipboard_ignored", reason = "empty_text");
                sleep_until_next_tick(&stop_flag, poll_interval);
                continue;
            }

            if text.as_bytes().len() > config.max_text_bytes {
                debug!(
                    event = "clipboard_ignored",
                    reason = "content_too_large",
                    bytes = text.as_bytes().len(),
                    max_bytes = config.max_text_bytes,
                );
                state.mark_failure(observed_hash, now);
                sleep_until_next_tick(&stop_flag, poll_interval);
                continue;
            }

            let save_result = config.persistence.save_clipboard_item(
                SaveClipboardItemInput {
                    content: text,
                    source_kind: SourceKind::DesktopLocal,
                    source_device_id: config.source_device_id.clone(),
                    pinned: false,
                    mark_current: true,
                },
                config.dedup_window_ms as i64,
                config.max_text_bytes,
            );

            match save_result {
                Ok(saved) => {
                    state.mark_success(observed_hash);
                    let event = ClipboardRefreshEvent {
                        item_id: saved.item.id,
                        created: saved.created,
                        reused_existing: saved.reused_existing,
                        is_current: saved.item.is_current,
                        source_kind: saved.item.source_kind,
                        observed_at_ms: now_ms(),
                    };

                    config.events.publish(event);
                }
                Err(error) => {
                    state.mark_failure(observed_hash, now);
                    warn!(event = "clipboard_save_failed", error = %error);
                }
            }
        }

        sleep_until_next_tick(&stop_flag, poll_interval);
    }
}

fn read_current_text(clipboard: &mut Option<Clipboard>) -> Option<String> {
    if clipboard.is_none() {
        *clipboard = Clipboard::new().ok();
    }

    let clipboard_instance = clipboard.as_mut()?;
    match clipboard_instance.get_text() {
        Ok(text) => Some(text),
        Err(_) => {
            *clipboard = None;
            None
        }
    }
}

fn sleep_until_next_tick(stop_flag: &AtomicBool, interval: Duration) {
    let mut remaining = interval;

    while !remaining.is_zero() && !stop_flag.load(Ordering::Relaxed) {
        let chunk = remaining.min(Duration::from_millis(100));
        thread::sleep(chunk);
        remaining = remaining.saturating_sub(chunk);
    }
}

fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

#[derive(Debug, Default)]
struct ClipboardWorkerState {
    last_processed_hash: Option<String>,
    last_failed_hash: Option<String>,
    last_failed_at_ms: i64,
}

impl ClipboardWorkerState {
    fn should_skip(&self, current_hash: &str, now_ms: i64, failure_retry_ms: u64) -> bool {
        if self
            .last_processed_hash
            .as_ref()
            .is_some_and(|hash| hash == current_hash)
        {
            return true;
        }

        if self
            .last_failed_hash
            .as_ref()
            .is_some_and(|hash| hash == current_hash)
            && now_ms.saturating_sub(self.last_failed_at_ms) < failure_retry_ms as i64
        {
            return true;
        }

        false
    }

    fn mark_success(&mut self, current_hash: String) {
        self.last_processed_hash = Some(current_hash);
        self.last_failed_hash = None;
        self.last_failed_at_ms = 0;
    }

    fn mark_failure(&mut self, current_hash: String, now_ms: i64) {
        self.last_failed_hash = Some(current_hash);
        self.last_failed_at_ms = now_ms;
    }
}
