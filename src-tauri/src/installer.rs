//! Cross-platform install / uninstall logic.
//!
//! This module replaces the upstream shell/VBS scripts (`scripts/install.sh`,
//! `scripts/install-current-user.vbs`, etc.) with a single Rust implementation
//! that performs exactly the same steps:
//!
//!   1. Ensure the workspace `lib.jar` exists (it is mirrored at app start by
//!      `workspace::init_workdir`, so this is normally a no-op).
//!   2. For each requested product, find the per-user vmoptions file (or fall
//!      back to the bundled template).
//!   3. Strip any existing `-javaagent:` line that targets ja-netfilter.
//!   4. Append `-javaagent:<workspace>/lib.jar=jetbrains`.
//!   5. Persist the `<PRODUCT>_VM_OPTIONS` env var so the IDE picks it up.
//!      On Linux we additionally write to `~/.profile`, `~/.bashrc` and
//!      `~/.zshrc` (mirroring install.sh). On macOS we use `launchctl setenv`.
//!      On Windows we update the user environment block via `setx`.
//!
//! All side-effects are reported through the structured return values and
//! appended to the in-memory log buffer (see `logger.rs`).

use std::fs;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::logger;
use crate::platform::{self, Os};
use crate::products::workspace_jar_path;
use crate::vmoptions;
use crate::workspace::WorkspaceState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallResult {
    pub product_id: String,
    pub success: bool,
    pub vmoptions_path: Option<String>,
    pub jar_path: String,
    pub message: String,
}

/// Install the javaagent for a single product.
pub fn install(state: &WorkspaceState, product_id: &str) -> Result<InstallResult> {
    let workdir = state.get();
    let jar = workspace_jar_path(&workdir);
    if !jar.exists() {
        return Ok(InstallResult {
            product_id: product_id.to_string(),
            success: false,
            vmoptions_path: None,
            jar_path: jar.display().to_string(),
            message: format!("ja-netfilter jar not found at {}", jar.display()),
        });
    }

    let vm_path = match platform::find_vmoptions_path(product_id, &workdir) {
        Some(p) => p,
        None => {
            return Ok(InstallResult {
                product_id: product_id.to_string(),
                success: false,
                vmoptions_path: None,
                jar_path: jar.display().to_string(),
                message: format!(
                    "No vmoptions file found for {} — install the IDE first or use the bundled template.",
                    product_id
                ),
            })
        }
    };

    // Edit the vmoptions in place — strip old javaagent lines and append fresh.
    vmoptions::ensure_javaagent(&vm_path, &jar)?;

    // Persist the env var so the IDE picks up the vmoptions path.
    let env_var = platform::env_var_name(product_id);
    let vm_path_str = vm_path.display().to_string();
    match Os::current() {
        Os::Macos => {
            // Best-effort launchctl, then write to shell rc files.
            std::process::Command::new("launchctl")
                .args(["setenv", &env_var, &vm_path_str])
                .status()
                .ok();
            write_shell_rc(&env_var, &vm_path_str)?;
        }
        Os::Linux => {
            write_shell_rc(&env_var, &vm_path_str)?;
        }
        Os::Windows => {
            // setx persists for future sessions. We can't use std::env::set_var
            // persistently across reboots, so setx is what we want here.
            std::process::Command::new("setx")
                .args([&env_var, &vm_path_str])
                .status()
                .ok();
        }
    }

    logger::append(
        logger::Level::Info,
        &format!(
            "[{}] installed javaagent -> {}",
            product_id,
            vm_path.display()
        ),
    );

    Ok(InstallResult {
        product_id: product_id.to_string(),
        success: true,
        vmoptions_path: Some(vm_path_str),
        jar_path: jar.display().to_string(),
        message: format!("Installed javaagent into {}", "vmoptions file"),
    })
}

/// Uninstall the javaagent from a single product.
pub fn uninstall(state: &WorkspaceState, product_id: &str) -> Result<InstallResult> {
    let workdir = state.get();
    let jar = workspace_jar_path(&workdir);
    let vm_path = match platform::find_vmoptions_path(product_id, &workdir) {
        Some(p) => p,
        None => {
            return Ok(InstallResult {
                product_id: product_id.to_string(),
                success: false,
                vmoptions_path: None,
                jar_path: jar.display().to_string(),
                message: format!("No vmoptions file found for {} — nothing to uninstall.", product_id),
            })
        }
    };

    vmoptions::strip_javaagent(&vm_path)?;

    // On macOS, also unset the env var via launchctl; on Windows, leave the
    // var set (it points at a file that simply no longer has the agent).
    let env_var = platform::env_var_name(product_id);
    if Os::current() == Os::Macos {
        std::process::Command::new("launchctl")
            .args(["unsetenv", &env_var])
            .status()
            .ok();
    }
    remove_shell_rc(&env_var)?;

    logger::append(
        logger::Level::Info,
        &format!("[{}] removed javaagent from {}", product_id, vm_path.display()),
    );

    Ok(InstallResult {
        product_id: product_id.to_string(),
        success: true,
        vmoptions_path: Some(vm_path.display().to_string()),
        jar_path: jar.display().to_string(),
        message: "Removed javaagent line".into(),
    })
}

/// Append `export <ENV>=<path>` to ~/.profile, ~/.bashrc, ~/.zshrc. Idempotent.
fn write_shell_rc(env_var: &str, value: &str) -> Result<()> {
    let line = format!("export {}=\"{}\"", env_var, value);
    let mut added = false;

    for path_str in [".profile", ".bashrc", ".zshrc"] {
        if let Some(home) = dirs::home_dir() {
            let rc = home.join(path_str);
            if let Ok(existing) = fs::read_to_string(&rc) {
                if existing.contains(&line) {
                    continue;
                }
                let mut new = existing.trim_end_matches('\n').to_string();
                if !new.is_empty() {
                    new.push('\n');
                }
                new.push_str(&line);
                new.push('\n');
                fs::write(&rc, new).ok();
                added = true;
            } else {
                fs::write(&rc, format!("{}\n", line)).ok();
                added = true;
            }
        }
    }

    if !added {
        log::warn!("could not persist env var {} to any shell rc file", env_var);
    }
    Ok(())
}

/// Remove any line that exports the named env var from ~/.profile, ~/.bashrc, ~/.zshrc.
fn remove_shell_rc(env_var: &str) -> Result<()> {
    let pattern = format!("export {}=", env_var);
    for path_str in [".profile", ".bashrc", ".zshrc"] {
        if let Some(home) = dirs::home_dir() {
            let rc = home.join(path_str);
            if let Ok(existing) = fs::read_to_string(&rc) {
                let filtered: String = existing
                    .lines()
                    .filter(|l| !l.contains(&pattern))
                    .collect::<Vec<_>>()
                    .join("\n");
                if filtered != existing {
                    fs::write(&rc, filtered).ok();
                }
            }
        }
    }
    Ok(())
}
