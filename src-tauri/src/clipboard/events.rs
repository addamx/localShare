use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Mutex;

use serde::Serialize;

use crate::persistence::SourceKind;

pub const CLIPBOARD_REFRESH_EVENT_NAME: &str = "localshare://clipboard/refresh";

pub type ClipboardEventReceiver = Receiver<ClipboardRefreshEvent>;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardRefreshEvent {
    pub item_id: String,
    pub created: bool,
    pub reused_existing: bool,
    pub is_current: bool,
    pub source_kind: SourceKind,
    pub observed_at_ms: i64,
}

#[derive(Debug, Default)]
pub struct ClipboardEventHub {
    subscribers: Mutex<Vec<Sender<ClipboardRefreshEvent>>>,
}

impl ClipboardEventHub {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn subscribe(&self) -> ClipboardEventReceiver {
        let (sender, receiver) = channel();
        if let Ok(mut subscribers) = self.subscribers.lock() {
            subscribers.push(sender);
        }

        receiver
    }

    pub fn publish(&self, event: ClipboardRefreshEvent) {
        let mut subscribers = match self.subscribers.lock() {
            Ok(subscribers) => subscribers,
            Err(_) => return,
        };

        subscribers.retain(|subscriber| subscriber.send(event.clone()).is_ok());
    }

    pub fn subscriber_count(&self) -> usize {
        self.subscribers
            .lock()
            .map(|subscribers| subscribers.len())
            .unwrap_or(0)
    }
}
