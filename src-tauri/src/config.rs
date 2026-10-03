//! ja-netfilter 插件配置文件（`dns.conf`、`power.conf`、`url.conf`）。
//! 直接读写项目自带 `resources/config/` 目录下的文件。

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::workspace::{resolve_under_root, WorkspaceState};

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

/// 枚举项目自带的配置文件（含文件大小）。
pub fn list(state: &WorkspaceState) -> Result<Vec<ConfigFile>> {
    let mut out = Vec::new();
    for rel in CONFIG_FILES {
        let path = resolve_under_root(state, rel)?;
        let size = path.metadata().map(|m| m.len()).unwrap_or(0);
        out.push(ConfigFile {
            name: rel.rsplit('/').next().unwrap_or(rel).to_string(),
            relative_path: rel.to_string(),
            size,
        });
    }
    Ok(out)
}

pub fn read_text(state: &WorkspaceState, relative_path: &str) -> Result<String> {
    let path = resolve_under_root(state, relative_path)?;
    fs::read_to_string(&path)
        .with_context(|| format!("读取配置失败：{}", path.display()))
}

pub fn write_text(
    state: &WorkspaceState,
    relative_path: &str,
    content: &str,
) -> Result<PathBuf> {
    let path = resolve_under_root(state, relative_path)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(&path, content)
        .with_context(|| format!("写入配置失败：{}", path.display()))?;
    Ok(path)
}
