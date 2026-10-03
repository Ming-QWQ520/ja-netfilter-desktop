//! ja-netfilter plugin config files (`dns.conf`, `power.conf`, `url.conf`).

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::workspace::{resolve_under_workdir, WorkspaceState};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigFile {
    pub name: String,
    pub relative_path: String,
    pub size: u64,
}

/// The list of plugin config files we ship with the app.
pub const CONFIG_FILES: &[&str] = &[
    "config/dns.conf",
    "config/power.conf",
    "config/url.conf",
];

/// Enumerate the workspace config files (with size).
pub fn list(state: &WorkspaceState) -> Result<Vec<ConfigFile>> {
    let mut out = Vec::new();
    for rel in CONFIG_FILES {
        let path = resolve_under_workdir(state, rel)?;
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
    let path = resolve_under_workdir(state, relative_path)?;
    fs::read_to_string(&path)
        .with_context(|| format!("failed to read config at {}", path.display()))
}

pub fn write_text(state: &WorkspaceState, relative_path: &str, content: &str) -> Result<PathBuf> {
    let path = resolve_under_workdir(state, relative_path)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(&path, content)
        .with_context(|| format!("failed to write config at {}", path.display()))?;
    Ok(path)
}
