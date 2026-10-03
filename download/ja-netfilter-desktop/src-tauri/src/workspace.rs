//! Per-user workspace management.
//!
//! ja-netfilter ships with a set of files (lib.jar, plugins, configs,
//! vmoptions) that the bundled app exposes as Tauri resources. Because the
//! install location is itself writable (the user edits vmoptions, swaps
//! lib.jar for a custom build, etc.), we mirror those resources into a
//! writable per-user directory on first launch and treat *that* as the
//! authoritative source for all subsequent reads/writes.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use tauri::{AppHandle, Manager};

/// Sub-directory name used inside the user-config dir for the workspace.
pub const WORKDIR_NAME: &str = "ja-netfilter-desktop";

/// Map of bundled resource path -> relative target inside the workdir.
const RESOURCE_FILES: &[(&str, &str)] = &[
    ("resources/lib.jar", "lib.jar"),
    ("resources/config/dns.conf", "config/dns.conf"),
    ("resources/config/power.conf", "config/power.conf"),
    ("resources/config/url.conf", "config/url.conf"),
    ("resources/plugins/dns.jar", "plugins/dns.jar"),
    ("resources/plugins/hideme.jar", "plugins/hideme.jar"),
    ("resources/plugins/power.jar", "plugins/power.jar"),
    ("resources/plugins/url.jar", "plugins/url.jar"),
];

/// All known vmoptions product identifiers (matches the upstream install.sh).
pub static JB_PRODUCT_IDS: &[&str] = &[
    "idea",
    "clion",
    "phpstorm",
    "goland",
    "pycharm",
    "webstorm",
    "webide",
    "rider",
    "datagrip",
    "rubymine",
    "dataspell",
    "aqua",
    "rustrover",
    "gateway",
    "jetbrains_client",
    "jetbrainsclient",
    "studio",
    "devecostudio",
];

/// Display names shown in the GUI for each product id.
pub static JB_PRODUCT_LABELS: Lazy<Vec<(&'static str, &'static str)>> = Lazy::new(|| {
    vec![
        ("idea", "IntelliJ IDEA"),
        ("clion", "CLion"),
        ("phpstorm", "PhpStorm"),
        ("goland", "GoLand"),
        ("pycharm", "PyCharm"),
        ("webstorm", "WebStorm"),
        ("webide", "WebIDE (legacy)"),
        ("rider", "Rider"),
        ("datagrip", "DataGrip"),
        ("rubymine", "RubyMine"),
        ("dataspell", "DataSpell"),
        ("aqua", "Aqua"),
        ("rustrover", "RustRover"),
        ("gateway", "JetBrains Gateway"),
        ("jetbrains_client", "JetBrains Client"),
        ("jetbrainsclient", "JetBrains Client (legacy)"),
        ("studio", "Android Studio"),
        ("devecostudio", "DevEco Studio"),
    ]
});

/// Process-wide hold on the workdir path so commands can read it cheaply.
#[derive(Clone)]
pub struct WorkspaceState {
    pub workdir: Arc<RwLock<PathBuf>>,
}

impl WorkspaceState {
    pub fn new(workdir: PathBuf) -> Self {
        Self {
            workdir: Arc::new(RwLock::new(workdir)),
        }
    }

    pub fn get(&self) -> PathBuf {
        self.workdir.read().clone()
    }

    pub fn set(&self, path: PathBuf) {
        *self.workdir.write() = path;
    }
}

/// Resolve the per-user workspace directory and ensure it exists with all
/// bundled resources mirrored into it. The path is:
///   - Linux:   `$XDG_CONFIG_HOME/ja-netfilter-desktop` (or `~/.config/...`)
///   - macOS:    `~/Library/Application Support/ja-netfilter-desktop`
///   - Windows:  `%APPDATA%/ja-netfilter-desktop`
pub fn init_workdir(app: &AppHandle) -> Result<PathBuf> {
    let base = dirs::config_dir()
        .context("could not resolve user config dir for the current platform")?;
    let workdir = base.join(WORKDIR_NAME);
    fs::create_dir_all(&workdir).with_context(|| {
        format!(
            "failed to create workspace at {}",
            workdir.display()
        )
    })?;

    mirror_resources(app, &workdir)?;
    Ok(workdir)
}

/// Mirror every bundled resource file into the workspace. Existing files are
/// preserved so user edits survive app upgrades; missing files are copied.
fn mirror_resources(app: &AppHandle, workdir: &Path) -> Result<()> {
    for (resource_rel, target_rel) in RESOURCE_FILES {
        let target = workdir.join(target_rel);
        if target.exists() {
            continue;
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).ok();
        }
        if let Ok(resource_path) = app
            .path()
            .resolve(resource_rel, tauri::path::BaseDirectory::Resource)
        {
            if resource_path.exists() {
                fs::copy(&resource_path, &target).with_context(|| {
                    format!(
                        "failed to copy bundled resource {} -> {}",
                        resource_path.display(),
                        target.display()
                    )
                })?;
                continue;
            }
        }
        log::warn!(
            "bundled resource not found at build time: {} (target {})",
            resource_rel,
            target.display()
        );
    }

    // Mirror vmoptions templates — they are not pre-listed because there
    // are many of them; iterate the resources/vmoptions dir at runtime.
    if let Ok(vm_dir) = app
        .path()
        .resolve("resources/vmoptions", tauri::path::BaseDirectory::Resource)
    {
        if vm_dir.exists() {
            let target_vm_dir = workdir.join("vmoptions");
            fs::create_dir_all(&target_vm_dir)?;
            if let Ok(entries) = fs::read_dir(&vm_dir) {
                for entry in entries.flatten() {
                    let src = entry.path();
                    let name = entry.file_name();
                    let dst = target_vm_dir.join(&name);
                    if !dst.exists() {
                        fs::copy(&src, &dst).ok();
                    }
                }
            }
        }
    }

    Ok(())
}

/// Return the workspace directory currently in use.
pub fn current_workdir(state: &WorkspaceState) -> PathBuf {
    state.get()
}

/// Resolve a workspace-relative path safely (no escape via `..`).
pub fn resolve_under_workdir(state: &WorkspaceState, rel: &str) -> Result<PathBuf> {
    let root = state.get();
    let joined = root.join(rel);
    let canonical = joined.canonicalize().unwrap_or_else(|_| joined.clone());
    if !canonical.starts_with(&root) {
        anyhow::bail!("path escapes the workspace: {}", rel);
    }
    Ok(canonical)
}
