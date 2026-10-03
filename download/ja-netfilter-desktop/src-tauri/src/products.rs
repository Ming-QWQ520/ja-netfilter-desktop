//! JetBrains product definitions and runtime detection.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::platform::{self};
use crate::workspace::WorkspaceState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductInfo {
    pub id: String,
    pub name: String,
    pub env_var: String,
    pub vmoptions_path: Option<String>,
    pub vmoptions_source: VmoptionsSource,
    pub javaagent_installed: bool,
    pub javaagent_target: Option<String>,
    pub vmoptions_preview: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VmoptionsSource {
    /// Resolved via env var (`*_VM_OPTIONS`).
    Env,
    /// Found at the JetBrains per-user default location.
    User,
    /// Using the workspace template (no per-user file exists yet).
    Template,
    /// No file exists anywhere.
    Missing,
}

impl VmoptionsSource {
    pub fn label(self) -> &'static str {
        match self {
            VmoptionsSource::Env => "Env var",
            VmoptionsSource::User => "User config",
            VmoptionsSource::Template => "Template",
            VmoptionsSource::Missing => "Missing",
        }
    }
}

/// The full product list as known by ja-netfilter, with friendly display names.
pub fn list_known_products() -> Vec<(&'static str, &'static str)> {
    crate::workspace::JB_PRODUCT_LABELS.clone()
}

/// Detect the current state of every known product for this user.
pub fn detect_all(state: &WorkspaceState) -> Vec<ProductInfo> {
    let workdir = state.get();
    list_known_products()
        .into_iter()
        .map(|(id, name)| detect_one(id, name, &workdir))
        .collect()
}

pub fn detect_one(id: &str, name: &str, workdir: &std::path::Path) -> ProductInfo {
    let env_var = platform::env_var_name(id);

    let (path, source) = match platform::find_vmoptions_path(id, workdir) {
        Some(p) => {
            let is_env = std::env::var(&env_var).map(|v| PathBuf::from(v) == p).unwrap_or(false);
            let is_template = p.starts_with(workdir);
            let source = if is_env {
                VmoptionsSource::Env
            } else if is_template {
                VmoptionsSource::Template
            } else {
                VmoptionsSource::User
            };
            (Some(p), source)
        }
        None => (None, VmoptionsSource::Missing),
    };

    let mut vmoptions_preview: Option<String> = None;
    let mut javaagent_installed = false;
    let mut javaagent_target: Option<String> = None;

    if let Some(ref p) = path {
        if let Ok(content) = std::fs::read_to_string(p) {
            vmoptions_preview = Some(content.clone());
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("-javaagent:") && trimmed.contains("ja-netfilter") {
                    javaagent_installed = true;
                    javaagent_target = Some(trimmed.to_string());
                    break;
                }
                // Also accept the generic `-javaagent:...=jetbrains` form
                // which the bundled lib.jar uses by default.
                if trimmed.starts_with("-javaagent:") && trimmed.ends_with("=jetbrains") {
                    javaagent_installed = true;
                    javaagent_target = Some(trimmed.to_string());
                    break;
                }
            }
        }
    }

    ProductInfo {
        id: id.to_string(),
        name: name.to_string(),
        env_var,
        vmoptions_path: path.map(|p| p.display().to_string()),
        vmoptions_source: source,
        javaagent_installed,
        javaagent_target,
        vmoptions_preview,
    }
}

/// Where would the bundled ja-netfilter jar live inside the user workspace?
pub fn workspace_jar_path(workdir: &std::path::Path) -> PathBuf {
    workdir.join("lib.jar")
}
