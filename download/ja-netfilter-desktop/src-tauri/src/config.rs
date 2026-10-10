//! ja-netfilter 插件配置文件（`dns.conf`、`power.conf`、`url.conf` 等）。
//!
//! v0.1.0：读写目标为应用自带资源目录内的 `config-jetbrains/`
//! （agent 就地引用，JVM 实际加载的就是这份，无需任何复制）。

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::workspace::{self, WorkspaceState};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigFile {
    pub name: String,
    pub relative_path: String,
    pub size: u64,
}

/// 配置子目录名（与 `-javaagent:...=jetbrains` 参数对应）。
pub const CONFIG_SUBDIR: &str = "config-jetbrains";

/// 枚举配置文件：以 agent_root 内实际存在的 *.conf 为准
/// （dns/power/url 是内置项，env/native 等由 ckey 流程下载的也可显示）。
pub fn list(state: &WorkspaceState) -> Result<Vec<ConfigFile>> {
    let dir = state.agent_root.join(CONFIG_SUBDIR);
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("conf") {
                let name = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default()
                    .to_string();
                let size = path.metadata().map(|m| m.len()).unwrap_or(0);
                out.push(ConfigFile {
                    name: name.clone(),
                    relative_path: format!("{}/{}", CONFIG_SUBDIR, name),
                    size,
                });
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// 读取配置文本。
pub fn read_text(state: &WorkspaceState, relative_path: &str) -> Result<String> {
    workspace::read_resource(state, relative_path)
}

/// 写入配置文本。
pub fn write_text(
    state: &WorkspaceState,
    relative_path: &str,
    content: &str,
) -> Result<std::path::PathBuf> {
    workspace::write_resource(state, relative_path, content)
}
