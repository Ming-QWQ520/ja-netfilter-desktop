//! ja-netfilter 插件配置文件（`dns.conf`、`power.conf`、`url.conf`）。
//!
//! 读时优先工作区副本，回退项目自带模板。写时 copy-on-write 到工作区。
//! 不再预先复制任何文件到工作区。

use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::workspace::{self, WorkspaceState};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigFile {
    pub name: String,
    pub relative_path: String,
    pub size: u64,
}

/// 项目自带的插件配置文件清单。
pub const CONFIG_FILES: &[&str] = &[
    "config/dns.conf",
    "config/power.conf",
    "config/url.conf",
];

/// 枚举配置文件。优先工作区副本，回退项目自带模板。
pub fn list(state: &WorkspaceState) -> Result<Vec<ConfigFile>> {
    let mut out = Vec::new();
    for rel in CONFIG_FILES {
        let workdir_copy = state.workdir().join(rel);
        let bundled = state.resource_root().join(rel);
        let (path, size) = if workdir_copy.exists() {
            let size = workdir_copy.metadata().map(|m| m.len()).unwrap_or(0);
            (workdir_copy, size)
        } else if bundled.exists() {
            let size = bundled.metadata().map(|m| m.len()).unwrap_or(0);
            (bundled, size)
        } else {
            continue;
        };
        out.push(ConfigFile {
            name: rel.rsplit('/').next().unwrap_or(rel).to_string(),
            relative_path: rel.to_string(),
            size,
        });
    }
    Ok(out)
}

/// 读取配置文件：优先工作区副本，回退项目自带模板。
pub fn read_text(state: &WorkspaceState, relative_path: &str) -> Result<String> {
    workspace::read_resource(state, relative_path)
}

/// 写入配置文件：copy-on-write 到工作区，再修改工作区副本。
pub fn write_text(
    state: &WorkspaceState,
    relative_path: &str,
    content: &str,
) -> Result<PathBuf> {
    workspace::write_resource(state, relative_path, content)
}
