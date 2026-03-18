use serde::Serialize;

#[derive(Debug)]
pub struct ClipboardService {
    poll_interval_ms: u64,
    max_text_bytes: usize,
}

impl ClipboardService {
    pub fn new(poll_interval_ms: u64, max_text_bytes: usize) -> Self {
        Self {
            poll_interval_ms,
            max_text_bytes,
        }
    }

    pub fn status(&self) -> ClipboardStatus {
        ClipboardStatus {
            mode: "polling".to_string(),
            poll_interval_ms: self.poll_interval_ms,
            max_text_bytes: self.max_text_bytes,
            current_item_tracking: true,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardStatus {
    pub mode: String,
    pub poll_interval_ms: u64,
    pub max_text_bytes: usize,
    pub current_item_tracking: bool,
}
