//! vmoptions file IO with javaagent-aware editing.
//!
//! Mirrors the semantics of the upstream shell/VBS install scripts: when
//! installing, the existing `-javaagent:...ja-netfilter.jar...` line is
//! removed and a fresh line pointing at the workspace jar is appended.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::platform::javaagent_line;
use crate::workspace::{resolve_under_workdir, WorkspaceState};

/// Read the raw text content of a vmoptions file identified either by an
/// absolute path (preferred) or a workspace-relative id.
pub fn read_text(state: &WorkspaceState, path_or_id: &str) -> Result<String> {
    let path = resolve_path(state, path_or_id)?;
    fs::read_to_string(&path)
        .with_context(|| format!("failed to read vmoptions at {}", path.display()))
}

/// Write text back to a vmoptions file. Refuses to write outside the
/// workspace unless the path is an absolute, existing file.
pub fn write_text(state: &WorkspaceState, path_or_id: &str, content: &str) -> Result<()> {
    let path = resolve_path(state, path_or_id)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(&path, content)
        .with_context(|| format!("failed to write vmoptions at {}", path.display()))
}

/// Reset a vmoptions file back to the workspace template (or empty out the
/// javaagent line if the file was user-supplied).
pub fn reset_to_template(state: &WorkspaceState, path_or_id: &str) -> Result<()> {
    // If the requested id maps to a workspace vmoptions template, overwrite it
    // from the bundled resource again. Otherwise, simply strip the javaagent.
    let workdir = state.get();
    let template_path = workdir
        .join("vmoptions")
        .join(format!("{}.vmoptions", path_or_id));

    if template_path.exists() {
        // Find the bundled resource for this id.
        let resource_rel = format!("resources/vmoptions/{}.vmoptions", path_or_id);
        // We don't have an AppHandle here, so do a best-effort filesystem lookup
        // in known sibling locations.
        if let Some(restored) = try_restore_template_from_dist(&resource_rel) {
            fs::write(&template_path, &restored)?;
            return Ok(());
        }
    }

    // Fall back: strip any existing javaagent lines.
    let path = resolve_path(state, path_or_id)?;
    let content = fs::read_to_string(&path).unwrap_or_default();
    let stripped: String = content
        .lines()
        .filter(|l| {
            let t = l.trim();
            !(t.starts_with("-javaagent:") && (t.contains("ja-netfilter") || t.ends_with("=jetbrains")))
        })
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&path, stripped)?;
    Ok(())
}

/// Append or refresh the `-javaagent:` line in a vmoptions file so it points
/// at the workspace `lib.jar`.
pub fn ensure_javaagent(vmoptions_path: &Path, jar_path: &Path) -> Result<()> {
    let content = fs::read_to_string(vmoptions_path).unwrap_or_default();
    let line = javaagent_line(jar_path);

    let cleaned: String = content
        .lines()
        .filter(|l| {
            let t = l.trim();
            !(t.starts_with("-javaagent:") && (t.contains("ja-netfilter") || t.ends_with("=jetbrains")))
        })
        .collect::<Vec<_>>()
        .join("\n");

    let mut out = cleaned.trim_end_matches('\n').to_string();
    if !out.is_empty() {
        out.push('\n');
    }
    out.push_str(&line);
    out.push('\n');

    fs::write(vmoptions_path, out)?;
    Ok(())
}

/// Strip every `-javaagent:` line that targets ja-netfilter from a file.
pub fn strip_javaagent(vmoptions_path: &Path) -> Result<()> {
    let content = fs::read_to_string(vmoptions_path).unwrap_or_default();
    let cleaned: String = content
        .lines()
        .filter(|l| {
            let t = l.trim();
            !(t.starts_with("-javaagent:") && (t.contains("ja-netfilter") || t.ends_with("=jetbrains")))
        })
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(vmoptions_path, cleaned)?;
    Ok(())
}

/// Best-effort: try to restore a vmoptions template from the app bundle. The
/// bundled resource lives under `<app>/resources/vmoptions/<id>.vmoptions`,
/// which is reachable from the workspace via a sibling directory.
fn try_restore_template_from_dist(resource_rel: &str) -> Option<String> {
    // Walk up from CARGO_MANIFEST_DIR (dev) / executable dir (release) to
    // find the resource. This is best-effort and may return None in odd
    // setups — the GUI still functions correctly without restoration.
    let manifest_dir = option_env!("CARGO_MANIFEST_DIR").map(PathBuf::from);
    let exe_dir = std::env::current_exe().ok().and_then(|p| p.parent().map(PathBuf::from));

    for base in [manifest_dir.clone(), exe_dir].into_iter().flatten() {
        let candidate = base.join(resource_rel);
        if candidate.exists() {
            return fs::read_to_string(&candidate).ok();
        }
        // Also try with `src-tauri/` prefix in case we're running from a dev shell.
        let candidate2 = base.join("src-tauri").join(resource_rel);
        if candidate2.exists() {
            return fs::read_to_string(&candidate2).ok();
        }
    }
    None
}

/// Resolve a vmoptions path. Accepts:
///   - absolute path
///   - workspace-relative path (e.g. `vmoptions/idea.vmoptions`)
///   - bare product id (e.g. `idea`) — expanded to `vmoptions/idea.vmoptions`
fn resolve_path(state: &WorkspaceState, path_or_id: &str) -> Result<PathBuf> {
    let p = Path::new(path_or_id);
    if p.is_absolute() {
        return Ok(p.to_path_buf());
    }
    // Bare product id?
    let expanded = if !path_or_id.contains('/') && !path_or_id.contains('\\') && !path_or_id.ends_with(".vmoptions") {
        format!("vmoptions/{}.vmoptions", path_or_id)
    } else {
        path_or_id.to_string()
    };
    resolve_under_workdir(state, &expanded)
}
