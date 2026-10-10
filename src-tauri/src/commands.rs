//! Tauri command surface — 前端与 Rust 后端的桥梁。
//!
//! v0.1.0：安装/卸载命令异步化（spawn_blocking），避免 PowerShell 调用
//! 期间冻结 UI；install_all 一次调用覆盖全部产品（单次 PS 进程）；
//! 自定义授权（名称/到期时间）随安装命令贯通，安装成功后自动生成
//! `<prd>.key` 授权文件（仿 ckey_script.ps1）。

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use crate::agent_sync::{self, SyncSummary};
use crate::config::{self, ConfigFile};
use crate::installer::{self, InstallResult};
use crate::license::{self, LicenseResult};
use crate::logger::{self, LogEntry};
use crate::platform::Os;
use crate::products::{self, ProductInfo};
use crate::workspace::{self, WorkspaceState};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    /// 杂项数据目录。
    pub workdir: String,
    /// 应用自带资源目录（agent 就地引用的根）。
    pub bundle_root: String,
    /// agent 运行时目录（正常 == bundle_root；极端情况指向兜底副本）。
    pub agent_root: String,
    /// agent 路径是否"干净"（无空格/非 ASCII）。
    pub agent_clean: bool,
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
    let name = products::list_known_products()
        .into_iter()
        .find(|(id, _)| *id == product_id)
        .map(|(_, n)| n)
        .unwrap_or(product_id.as_str());
    Ok(products::detect_one(&product_id, name, &state))
}

// -------- Install / Uninstall（异步，避免阻塞 UI） --------------------------

#[tauri::command]
pub async fn install_product(
    app: AppHandle,
    product_id: String,
    license_name: Option<String>,
    license_expiry: Option<String>,
) -> Result<InstallResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<WorkspaceState>();
        let results = installer::install_batch(&state, &[product_id]);
        // 授权文件生成（best-effort：失败仅记录日志，不影响安装结果）
        for r in &results {
            if r.success {
                let lr = license::generate_keys(
                    &state,
                    std::slice::from_ref(&r.product_id),
                    license_name.as_deref(),
                    license_expiry.as_deref(),
                );
                report_license_results(&lr);
            }
        }
        results
            .into_iter()
            .next()
            .ok_or_else(|| "无结果".to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn uninstall_product(
    app: AppHandle,
    product_id: String,
) -> Result<InstallResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<WorkspaceState>();
        installer::uninstall_batch(&state, &[product_id])
            .into_iter()
            .next()
            .ok_or_else(|| "无结果".to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn install_all_products(
    app: AppHandle,
    license_name: Option<String>,
    license_expiry: Option<String>,
) -> Result<Vec<InstallResult>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<WorkspaceState>();
        let ids: Vec<String> = products::list_known_products()
            .into_iter()
            .map(|(id, _)| id.to_string())
            .collect();
        let results = installer::install_batch(&state, &ids);
        // 为安装成功且有产品码的产品生成授权文件（ckey_script 的一次性流程）
        let ok_ids: Vec<String> = results
            .iter()
            .filter(|r| r.success)
            .map(|r| r.product_id.clone())
            .collect();
        if !ok_ids.is_empty() {
            let lr = license::generate_keys(
                &state,
                &ok_ids,
                license_name.as_deref(),
                license_expiry.as_deref(),
            );
            report_license_results(&lr);
        }
        Ok(results)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn uninstall_all_products(app: AppHandle) -> Result<Vec<InstallResult>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<WorkspaceState>();
        let ids: Vec<String> = products::list_known_products()
            .into_iter()
            .map(|(id, _)| id.to_string())
            .collect();
        Ok(installer::uninstall_batch(&state, &ids))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn cleanup_env_vars(app: AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<WorkspaceState>();
        let ids: Vec<String> = products::list_known_products()
            .into_iter()
            .map(|(id, _)| id.to_string())
            .collect();
        installer::cleanup_env(&state, &ids);
        logger::append(
            logger::Level::Success,
            "已清理全部 <PRODUCT>_VM_OPTIONS 环境变量（User + Machine）",
        );
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

// -------- License（自定义授权） ---------------------------------------------

/// 把授权生成结果写入日志。
fn report_license_results(results: &[LicenseResult]) {
    let ok = results.iter().filter(|r| r.ok).count();
    let skipped = results.len() - ok;
    if ok > 0 {
        logger::append(
            logger::Level::Success,
            &format!("授权文件生成完成：{} 个成功，{} 个跳过/失败", ok, skipped),
        );
    }
}

#[tauri::command]
pub async fn generate_license_keys(
    app: AppHandle,
    license_name: Option<String>,
    license_expiry: Option<String>,
) -> Result<Vec<LicenseResult>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<WorkspaceState>();
        // 检测到的产品全部生成（未检测到配置目录的产品会被跳过并说明原因）
        let ids: Vec<String> = products::list_known_products()
            .into_iter()
            .map(|(id, _)| id.to_string())
            .collect();
        let results = license::generate_keys(
            &state,
            &ids,
            license_name.as_deref(),
            license_expiry.as_deref(),
        );
        report_license_results(&results);
        Ok(results)
    })
    .await
    .map_err(|e| e.to_string())?
}

// -------- Agent 资源同步 ----------------------------------------------------

/// 手动同步最新 agent 资源（设置页按钮；安装流程内也会自动执行一次）。
#[tauri::command]
pub async fn sync_agent_resources(app: AppHandle) -> Result<SyncSummary, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<WorkspaceState>();
        Ok(agent_sync::sync_latest(&state))
    })
    .await
    .map_err(|e| e.to_string())?
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

/// 清除目标 vmoptions 中的受管行（-javaagent / 遗留 -Dja.netfilter.name）。
#[tauri::command]
pub fn strip_vmoptions(
    state: State<'_, WorkspaceState>,
    path_or_id: String,
) -> Result<(), String> {
    let p = crate::vmoptions::resolve_target(&state, &path_or_id).map_err(|e| e.to_string())?;
    crate::vmoptions::strip_managed(&p).map_err(|e| e.to_string())
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
    // 应用自带资源目录内的 plugins-jetbrains/（JVM 实际加载的插件）
    let plugins_dir = state.agent_root.join("plugins-jetbrains");
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
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

// -------- Workspace --------------------------------------------------------

#[tauri::command]
pub fn get_workspace_info(
    state: State<'_, WorkspaceState>,
    app: AppHandle,
) -> Result<WorkspaceInfo, String> {
    let jar_path = state.agent_jar.clone();
    let plugins_dir = state.agent_root.join("plugins-jetbrains");

    let mut plugin_jars = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&plugins_dir) {
        for entry in entries.flatten() {
            if entry.path().extension().and_then(|s| s.to_str()) == Some("jar") {
                plugin_jars.push(crate::platform::normalize_path_for_output(&entry.path()));
            }
        }
    }
    plugin_jars.sort();

    Ok(WorkspaceInfo {
        workdir: crate::platform::normalize_path_for_output(&state.workdir),
        bundle_root: crate::platform::normalize_path_for_output(&state.bundle_root),
        agent_root: crate::platform::normalize_path_for_output(&state.agent_root),
        agent_clean: crate::platform::is_clean_agent_path(
            &crate::platform::normalize_path_for_output(&jar_path),
        ),
        jar_path: crate::platform::normalize_path_for_output(&jar_path),
        jar_exists: jar_path.exists(),
        plugin_jars,
        vmoptions_dir: crate::platform::normalize_path_for_output(
            &state.agent_root.join("vmoptions"),
        ),
        config_dir: crate::platform::normalize_path_for_output(
            &state.agent_root.join("config-jetbrains"),
        ),
        os: crate::platform::Os::current(),
        app_version: app.package_info().version.to_string(),
    })
}

// -------- Misc -------------------------------------------------------------

#[tauri::command]
pub fn reveal_in_finder(path: String) -> Result<(), String> {
    let p = std::path::PathBuf::from(&path);
    // 文件可能不存在（占位路径），退回到父目录
    let target = if p.exists() { p.clone() } else { p.parent().map(|x| x.to_path_buf()).unwrap_or(p) };
    let (cmd, args) = crate::platform::open_in_explorer_cmd(&target)
        .ok_or_else(|| "no explorer command for this OS".to_string())?;
    std::process::Command::new(cmd)
        .args(args)
        .spawn()
        .map_err(|e| e.to_string())?;
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

/// 当前磁盘日志文件路径（前端展示 + 一键打开目录）。
#[tauri::command]
pub fn get_log_file_path() -> Result<Option<String>, String> {
    Ok(logger::log_file_path())
}

#[tauri::command]
pub fn app_version(app: AppHandle) -> Result<String, String> {
    Ok(app.package_info().version.to_string())
}

// 保留 workspace 引用避免 unused import
#[allow(unused)]
fn _touch_workspace() {
    let _ = workspace::WORKDIR_NAME;
}
