//! ja-netfilter-desktop — Tauri application library entry point.
//!
//! v0.1.0 启动流程（**零复制**）：
//!   1. env_logger 初始化
//!   2. 注入前端日志事件发送器（log://entry）
//!   3. 初始化工作区 + 定位应用自带资源目录
//!   4. agent 就地引用：`-javaagent` 直接指向资源目录内的 lib.jar，
//!      项目自带的文件不再复制到 C 盘或其它位置
//!      （仅在路径含空格且 8.3 短路径不可用时，安装阶段才兜底复制）
//!   5. 注册全部命令（含自定义授权生成 generate_license_keys）

mod agent_home;
mod commands;
mod config;
mod installer;
mod license;
mod locate;
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
            // 1. 前端实时日志事件
            logger::set_emitter(app.handle().clone());

            // 2. 工作区 + 应用自带资源目录
            let (workdir, bundle_root) = workspace::init_workspace(app.handle())?;

            // 3. agent 就地引用（零复制）
            let agent_root = bundle_root.clone();
            let agent_jar = agent_root.join("lib.jar");

            log::info!("bundle 资源目录：{}", bundle_root.display());
            log::info!("agent lib.jar（就地引用）：{}", agent_jar.display());
            if !platform::is_clean_agent_path(&platform::normalize_path_for_output(&agent_jar)) {
                // 含空格/非 ASCII：Windows 安装时由 PS 解析 8.3 短路径，
                // 仍失败才走 deploy_fallback 兜底（见 installer.rs）。
                log::warn!(
                    "资源路径含空格或非 ASCII 字符：{}（安装时将解析短路径，必要时兜底复制）",
                    agent_jar.display()
                );
                logger::append(
                    logger::Level::Warn,
                    &format!(
                        "资源路径含空格/非 ASCII：{}（安装时解析 8.3 短路径）",
                        agent_jar.display()
                    ),
                );
            }
            if agent_jar.exists() {
                logger::append(
                    logger::Level::Success,
                    &format!(
                        "agent 就地引用就绪（零复制）：{}",
                        platform::normalize_path_for_output(&agent_jar)
                    ),
                );
            } else {
                logger::append(
                    logger::Level::Warn,
                    &format!("未找到自带 lib.jar：{}", agent_jar.display()),
                );
            }

            app.manage(workspace::WorkspaceState::new(
                workdir,
                bundle_root,
                agent_root,
                agent_jar,
            ));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_products,
            refresh_product_status,
            install_product,
            uninstall_product,
            install_all_products,
            uninstall_all_products,
            cleanup_env_vars,
            generate_license_keys,
            read_vmoptions,
            write_vmoptions,
            strip_vmoptions,
            list_configs,
            read_config,
            write_config,
            read_plugin_jars,
            get_workspace_info,
            reveal_in_finder,
            get_log_history,
            clear_log_history,
            app_version,
        ])
        .run(tauri::generate_context!())
        .expect("error while running ja-netfilter-desktop application");
}
