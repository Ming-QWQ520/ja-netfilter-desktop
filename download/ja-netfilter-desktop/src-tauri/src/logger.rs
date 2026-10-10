//! In-memory log buffer + 前端事件推送。
//!
//! v0.2.0 修复：v0.1.x 前端监听的 `log://entry` 事件在 Rust 端从未发出，
//! 日志只能靠 2 秒轮询。现在 `append` 会同时通过 Tauri 事件推送到前端，
//! 并新增 SUCCESS 级别（对齐 ckey_script.ps1 的日志分级）。

use std::sync::Arc;

use once_cell::sync::Lazy;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

pub const MAX_ENTRIES: usize = 1024;
pub const LOG_EVENT: &str = "log://entry";

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
    Success,
    Warn,
    Error,
}

static LOGS: Lazy<Arc<RwLock<Vec<LogEntry>>>> =
    Lazy::new(|| Arc::new(RwLock::new(Vec::with_capacity(MAX_ENTRIES))));

static EMITTER: Lazy<RwLock<Option<AppHandle>>> = Lazy::new(|| RwLock::new(None));

/// 注入事件发送器（setup 阶段调用一次）。
pub fn set_emitter(app: AppHandle) {
    *EMITTER.write() = Some(app);
}

/// 追加一条日志：写入环形缓冲 + 推送 `log://entry` 事件。
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
        buf.push(entry.clone());
    }
    // 事件推送失败（如 webview 尚未就绪）不影响日志缓冲
    if let Some(app) = EMITTER.read().as_ref() {
        let _ = app.emit(LOG_EVENT, &entry);
    }
}

/// 读取缓冲历史。
pub fn history() -> Vec<LogEntry> {
    LOGS.read().clone()
}

/// 清空缓冲。
pub fn clear() {
    LOGS.write().clear();
}
