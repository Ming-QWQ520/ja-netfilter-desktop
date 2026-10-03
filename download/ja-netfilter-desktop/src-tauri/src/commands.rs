//! Tauri command surface — the bridge between the Vue frontend and the Rust
//! backend modules.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

use crate::config::{self, ConfigFile};
use crate::installer::{self, InstallResult};
use crate::logger::{self, LogEntry};
use crate::platform::Os;
use crate::products::{self, ProductInfo};
use crate::workspace::WorkspaceState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    pub workdir: String,
    pub jar_path: String,
    pub jar_exists: bool,
    pub plugin_jars: Vec<String>,
    pub vmoptions_dir: String,
    pub config_dir: String,
    pub os: Os,
    pub app_version: String,
}

// -------- Products ---------------------------------------------------------

#[tauri::command]
pub fn list_products(state: State<'_, WorkspaceState>) -> Result<Vec<ProductInfo>, String> {
    Ok(products::detect_all(&state))
}

#[tauri::command]
pub fn refresh_product_status(
    state: State<'_, WorkspaceState>,
    product_id: String,
) -> Result<ProductInfo, String> {
    let resource_root = state.get();
    let name = products::list_known_products()
        .into_iter()
        .find(|(id, _)| *id == product_id)
        .map(|(_, n)| n)
        .unwrap_or(product_id.as_str());
    Ok(products::detect_one(&product_id, name, &resource_root))
}

// -------- Install / Uninstall ----------------------------------------------

#[tauri::command]
pub fn install_product(
    state: State<'_, WorkspaceState>,
    product_id: String,
    license_name: Option<String>,
) -> Result<InstallResult, String> {
    installer::install(&state, &product_id, license_name.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn uninstall_product(
    state: State<'_, WorkspaceState>,
    product_id: String,
) -> Result<InstallResult, String> {
    installer::uninstall(&state, &product_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn install_all_products(
    state: State<'_, WorkspaceState>,
    license_name: Option<String>,
) -> Result<Vec<InstallResult>, String> {
    let mut results = Vec::new();
    for (id, _) in products::list_known_products() {
        match installer::install(&state, id, license_name.as_deref()) {
            Ok(r) => results.push(r),
            Err(e) => {
                logger::append(
                    logger::Level::Error,
                    &format!("[{}] install error: {}", id, e),
                );
                results.push(InstallResult {
                    product_id: id.to_string(),
                    success: false,
                    vmoptions_path: None,
                    jar_path: String::new(),
                    message: format!("{}", e),
                });
            }
        }
    }
    Ok(results)
}

#[tauri::command]
pub fn uninstall_all_products(
    state: State<'_, WorkspaceState>,
) -> Result<Vec<InstallResult>, String> {
    let mut results = Vec::new();
    for (id, _) in products::list_known_products() {
        match installer::uninstall(&state, id) {
            Ok(r) => results.push(r),
            Err(e) => {
                logger::append(
                    logger::Level::Error,
                    &format!("[{}] uninstall error: {}", id, e),
                );
                results.push(InstallResult {
                    product_id: id.to_string(),
                    success: false,
                    vmoptions_path: None,
                    jar_path: String::new(),
                    message: format!("{}", e),
                });
            }
        }
    }
    Ok(results)
}

// -------- vmoptions --------------------------------------------------------

#[tauri::command]
pub fn read_vmoptions(
    state: State<'_, WorkspaceState>,
    path_or_id: String,
) -> Result<String, String> {
    crate::vmoptions::read_text(&state, &path_or_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn write_vmoptions(
    state: State<'_, WorkspaceState>,
    path_or_id: String,
    content: String,
) -> Result<(), String> {
    crate::vmoptions::write_text(&state, &path_or_id, &content).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reset_vmoptions(
    state: State<'_, WorkspaceState>,
    path_or_id: String,
) -> Result<(), String> {
    crate::vmoptions::reset_to_template(&state, &path_or_id).map_err(|e| e.to_string())
}

// -------- Configs ----------------------------------------------------------

#[tauri::command]
pub fn list_configs(state: State<'_, WorkspaceState>) -> Result<Vec<ConfigFile>, String> {
    config::list(&state).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn read_config(
    state: State<'_, WorkspaceState>,
    relative_path: String,
) -> Result<String, String> {
    config::read_text(&state, &relative_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn write_config(
    state: State<'_, WorkspaceState>,
    relative_path: String,
    content: String,
) -> Result<String, String> {
    config::write_text(&state, &relative_path, &content)
        .map(|p| p.display().to_string())
        .map_err(|e| e.to_string())
}

// -------- Plugins ----------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginJar {
    pub name: String,
    pub path: String,
    pub size: u64,
}

#[tauri::command]
pub fn read_plugin_jars(state: State<'_, WorkspaceState>) -> Result<Vec<PluginJar>, String> {
    let resource_root = state.get();
    let plugins_dir = resource_root.join("plugins");
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&plugins_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("jar") {
                let size = path.metadata().map(|m| m.len()).unwrap_or(0);
                out.push(PluginJar {
                    name: path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or_default()
                        .to_string(),
                    path: crate::platform::normalize_path_for_output(&path),
                    size,
                });
            }
        }
    }
    Ok(out)
}

// -------- Workspace --------------------------------------------------------

#[tauri::command]
pub fn get_workspace_info(
    state: State<'_, WorkspaceState>,
    app: AppHandle,
) -> Result<WorkspaceInfo, String> {
    let resource_root = state.get();
    let jar_path = resource_root.join("lib.jar");
    let vmoptions_dir = resource_root.join("vmoptions");
    let config_dir = resource_root.join("config");
    let plugins_dir = resource_root.join("plugins");

    let mut plugin_jars = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&plugins_dir) {
        for entry in entries.flatten() {
            if entry
                .path()
                .extension()
                .and_then(|s| s.to_str())
                == Some("jar")
            {
                plugin_jars.push(entry.path().display().to_string());
            }
        }
    }

    Ok(WorkspaceInfo {
        workdir: crate::platform::normalize_path_for_output(&resource_root),
        jar_path: crate::platform::normalize_path_for_output(&jar_path),
        jar_exists: jar_path.exists(),
        plugin_jars: plugin_jars
            .into_iter()
            .map(|p| crate::platform::normalize_path_for_output(std::path::Path::new(&p)))
            .collect(),
        vmoptions_dir: crate::platform::normalize_path_for_output(&vmoptions_dir),
        config_dir: crate::platform::normalize_path_for_output(&config_dir),
        os: crate::platform::Os::current(),
        app_version: app.package_info().version.to_string(),
    })
}

// -------- Misc -------------------------------------------------------------

#[tauri::command]
pub fn reveal_in_finder(path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    let (cmd, args) = crate::platform::open_in_explorer_cmd(&p)
        .ok_or_else(|| "no explorer command for this OS".to_string())?;
    std::process::Command::new(cmd)
        .args(args)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn pick_jar_file(app: AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    use std::sync::mpsc;
    let (tx, rx) = mpsc::channel();
    app.dialog()
        .file()
        .add_filter("Java archive", &["jar"])
        .pick_file(move |result| {
            let _ = tx.send(result);
        });
    let result = rx.recv().map_err(|e| e.to_string())?;
    Ok(result.and_then(|f| f.into_path().ok().map(|p| p.display().to_string())))
}

#[tauri::command]
pub fn set_active_jar_path(_state: State<'_, WorkspaceState>, _path: String) -> Result<(), String> {
    // Future hook: allow the user to swap the active lib.jar path. For now we
    // always use the workspace's `lib.jar`, so this is a stub.
    Ok(())
}

#[tauri::command]
pub fn get_log_history() -> Result<Vec<LogEntry>, String> {
    Ok(logger::history())
}

#[tauri::command]
pub fn clear_log_history() -> Result<(), String> {
    logger::clear();
    Ok(())
}

#[tauri::command]
pub fn app_version(app: AppHandle) -> Result<String, String> {
    Ok(app.package_info().version.to_string())
}
