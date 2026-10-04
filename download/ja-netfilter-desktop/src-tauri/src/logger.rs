//! In-memory log buffer used for the live log console in the frontend.
//!
//! We keep the most recent `MAX_ENTRIES` log lines in a global ring buffer so
//! the GUI can fetch history on demand and tail new entries via events.

use std::sync::Arc;

use once_cell::sync::Lazy;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

pub const MAX_ENTRIES: usize = 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub ts: String,
    pub level: Level,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Debug,
    Info,
    Warn,
    Error,
}

static LOGS: Lazy<Arc<RwLock<Vec<LogEntry>>>> =
    Lazy::new(|| Arc::new(RwLock::new(Vec::with_capacity(MAX_ENTRIES))));

/// Append a new log entry.
pub fn append(level: Level, message: &str) {
    let entry = LogEntry {
        ts: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        level,
        message: message.to_string(),
    };
    {
        let mut buf = LOGS.write();
        if buf.len() >= MAX_ENTRIES {
            buf.remove(0);
        }
        buf.push(entry);
    }
}

/// Read out the buffered history.
pub fn history() -> Vec<LogEntry> {
    LOGS.read().clone()
}

/// Clear the buffered history.
pub fn clear() {
    LOGS.write().clear();
}
