//! ja-netfilter-desktop — Tauri application library entry point.
//!
//! Wires all Rust modules (products, vmoptions, config, installer, platform,
//! logger) into a single Tauri command surface, registers the event channel
//! used for live log streaming, and serves the Vue3 frontend.

mod commands;
mod config;
mod installer;
mod logger;
mod platform;
mod products;
mod vmoptions;
mod workspace;

use commands::*;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_secs()
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // 初始化工作区（可写，但不预先复制任何文件）和项目自带资源根目录（只读）。
            let (workdir, resource_root) = workspace::init_workspace(app.handle())?;
            log::info!("工作区：{}", workdir.display());
            log::info!("项目自带资源目录：{}", resource_root.display());

            app.manage(workspace::WorkspaceState::new(workdir, resource_root));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_products,
            refresh_product_status,
            install_product,
            uninstall_product,
            install_all_products,
            uninstall_all_products,
            read_vmoptions,
            write_vmoptions,
            reset_vmoptions,
            list_configs,
            read_config,
            write_config,
            read_plugin_jars,
            get_workspace_info,
            reveal_in_finder,
            pick_jar_file,
            set_active_jar_path,
            get_log_history,
            clear_log_history,
            app_version,
        ])
        .run(tauri::generate_context!())
        .expect("error while running ja-netfilter-desktop application");
}
