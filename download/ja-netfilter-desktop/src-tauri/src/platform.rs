//! Cross-platform helpers.
//!
//! Centralises the differences between Linux/macOS/Windows so the rest of the
//! backend can stay platform-agnostic.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Os {
    Windows,
    Macos,
    Linux,
}

impl Os {
    pub fn current() -> Self {
        #[cfg(target_os = "windows")]
        {
            Os::Windows
        }
        #[cfg(target_os = "macos")]
        {
            Os::Macos
        }
        #[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "openbsd"))]
        {
            Os::Linux
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux", target_os = "freebsd", target_os = "openbsd")))]
        {
            Os::Linux
        }
    }

    #[allow(dead_code)]
    pub fn is_windows(self) -> bool {
        matches!(self, Os::Windows)
    }

    #[allow(dead_code)]
    pub fn is_macos(self) -> bool {
        matches!(self, Os::Macos)
    }
}

/// Locate the file that the JetBrains launcher actually reads for a given
/// product. The discovery order matches the upstream install scripts:
///   1. `<PRODUCT>_VM_OPTIONS` env var (if set explicitly)
///   2. Per-user default vmoptions path under the JetBrains config dir
///   3. The bundled template shipped with the app (under `resources/vmoptions`)
pub fn find_vmoptions_path(product_id: &str, resource_root: &Path) -> Option<PathBuf> {
    let env_key = format!("{}_VM_OPTIONS", product_id.to_uppercase());
    if let Ok(val) = std::env::var(&env_key) {
        let p = PathBuf::from(val);
        if p.exists() {
            return Some(p);
        }
    }

    if let Some(path) = user_default_vmoptions(product_id) {
        if path.exists() {
            return Some(path);
        }
    }

    let template = resource_root
        .join("vmoptions")
        .join(format!("{}.vmoptions", product_id));
    if template.exists() {
        return Some(template);
    }

    None
}

/// Default per-user vmoptions location for each product on each OS.
/// These match what the upstream install scripts assume.
pub fn user_default_vmoptions(product_id: &str) -> Option<PathBuf> {
    let os = Os::current();
    let config_root = match os {
        Os::Windows => dirs::config_dir()?,
        Os::Macos => dirs::home_dir()?.join("Library").join("Application Support"),
        Os::Linux => dirs::config_dir()?,
    };

    let vendor_dir = match os {
        Os::Windows => "JetBrains",
        _ => "JetBrains",
    };

    Some(
        config_root
            .join(vendor_dir)
            .join(product_id)
            .join("idea.vmoptions"),
    )
}

/// Compute the env var name used by JetBrains for a given product id.
pub fn env_var_name(product_id: &str) -> String {
    format!("{}_VM_OPTIONS", product_id.to_uppercase())
}

/// Build the `-javaagent:` line that the install scripts append to vmoptions.
pub fn javaagent_line(jar_path: &Path) -> String {
    format!("-javaagent:{}=jetbrains", jar_path.display())
}

/// Return a default shell command that opens a path in the native file
/// explorer. Returns None if we don't know how.
pub fn open_in_explorer_cmd(path: &Path) -> Option<(String, Vec<String>)> {
    match Os::current() {
        Os::Macos => Some(("open".into(), vec![path.display().to_string()])),
        Os::Windows => Some(("explorer".into(), vec![path.display().to_string()])),
        Os::Linux => Some(("xdg-open".into(), vec![path.display().to_string()])),
    }
}
