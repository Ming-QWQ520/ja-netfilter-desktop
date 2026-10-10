//! In-memory log buffer + 前端事件推送 + 磁盘持久化。
//!
//! v0.1.0 诊断增强：
//!   1. `append` 同时把每条日志追加写入磁盘文件
//!      `%APPDATA%\ja-netfilter-desktop\logs\ja-netfilter-YYYYMMDD.log`，
//!      应用关闭后日志仍可回查/提交 issue。
//!   2. 初始化时自动清理 14 天前的旧日志文件。
//!   3. 新增便捷函数 debug!/info!/...（减少调用端样板）。
//!
//! v0.2.0 修复：v0.1.x 前端监听的 `log://entry` 事件在 Rust 端从未发出，
//! 日志只能靠 2 秒轮询。现在 `append` 会同时通过 Tauri 事件推送到前端，
//! 并新增 SUCCESS 级别（对齐 ckey_script.ps1 的日志分级）。

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use once_cell::sync::Lazy;
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

pub const MAX_ENTRIES: usize = 1024;
pub const LOG_EVENT: &str = "log://entry";
/// 保留最近 N 天的日志文件。
const RETENTION_DAYS: i64 = 14;

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

impl Level {
    fn as_str(self) -> &'static str {
        match self {
            Level::Debug => "DEBUG",
            Level::Info => "INFO",
            Level::Success => "SUCCESS",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
        }
    }
}

static LOGS: Lazy<Arc<RwLock<Vec<LogEntry>>>> =
    Lazy::new(|| Arc::new(RwLock::new(Vec::with_capacity(MAX_ENTRIES))));

static EMITTER: Lazy<RwLock<Option<AppHandle>>> = Lazy::new(|| RwLock::new(None));

static LOG_DIR: Lazy<Mutex<Option<PathBuf>>> = Lazy::new(|| Mutex::new(None));

/// 注入事件发送器（setup 阶段调用一次）。
pub fn set_emitter(app: AppHandle) {
    *EMITTER.write() = Some(app);
}

/// 设置日志目录并清理过期文件；返回当前日志文件的完整路径。
pub fn set_log_dir(dir: &Path) -> Option<PathBuf> {
    *LOG_DIR.lock() = Some(dir.to_path_buf());
    prune_old_logs(dir);
    current_log_file()
}

/// 当前日志文件路径（未初始化目录时为 None）。
pub fn log_file_path() -> Option<String> {
    current_log_file().map(|p| p.display().to_string())
}

fn current_log_file() -> Option<PathBuf> {
    let dir = LOG_DIR.lock().clone()?;
    let name = format!("ja-netfilter-{}.log", chrono::Local::now().format("%Y%m%d"));
    Some(dir.join(name))
}

/// 删除超过保留期的日志文件。
fn prune_old_logs(dir: &Path) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let cutoff_secs = (chrono::Local::now() - chrono::Duration::days(RETENTION_DAYS)).timestamp();
    for entry in entries.flatten() {
        let path = entry.path();
        let is_log = path.extension().and_then(|s| s.to_str()) == Some("log");
        if !is_log {
            continue;
        }
        let stale = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| (d.as_secs() as i64) < cutoff_secs)
            .unwrap_or(false);
        if stale {
            let _ = fs::remove_file(&path);
        }
    }
}

/// 追加一条日志：写入环形缓冲 + 推送 `log://entry` 事件 + 追加写磁盘文件。
pub fn append(level: Level, message: &str) {
    let entry = LogEntry {
        ts: chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string(),
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
    write_to_file(&entry);
}

fn write_to_file(entry: &LogEntry) {
    let Some(path) = current_log_file() else {
        return;
    };
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    let line = format!("{} [{}] {}\n", entry.ts, entry.level.as_str(), entry.message);
    let _ = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .and_then(|mut f| f.write_all(line.as_bytes()));
}

// ---------------------------------------------------------------------------
// 便捷函数
// ---------------------------------------------------------------------------

pub fn debug(message: &str) {
    append(Level::Debug, message);
}

pub fn info(message: &str) {
    append(Level::Info, message);
}

pub fn success(message: &str) {
    append(Level::Success, message);
}

pub fn warn(message: &str) {
    append(Level::Warn, message);
}

pub fn error(message: &str) {
    append(Level::Error, message);
}

/// 读取缓冲历史。
pub fn history() -> Vec<LogEntry> {
    LOGS.read().clone()
}

/// 清空缓冲。
pub fn clear() {
    LOGS.write().clear();
}
