//! 跨平台安装 / 卸载逻辑。
//!
//! 安装流程（直接编辑 IDE 自带的 vmoptions 文件，不依赖环境变量）：
//!   1. 查找 IDE 自带的 vmoptions 文件：
//!      - Windows: %APPDATA%\JetBrains\<product>*\<product>64.exe.vmoptions
//!                 C:\Program Files\JetBrains\<Product>*\bin\<product>64.exe.vmoptions
//!      - macOS: ~/Library/Application Support/JetBrains/<product>/*.vmoptions
//!      - Linux: ~/.config/JetBrains/<product>/*.vmoptions
//!   2. 若找到 IDE 自带的 vmoptions，直接编辑该文件（追加 -javaagent 行）。
//!   3. 若未找到，回退到工作区模板 + 环境变量方式。
//!   4. lib.jar 始终使用项目自带路径（不复制到工作区）。
//!   5. 同时设置 <PRODUCT>_VM_OPTIONS 环境变量（作为双保险）。

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::logger;
use crate::platform::{self, Os};
use crate::products::bundled_jar_path;
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

/// 为单个产品安装 javaagent。`license_name` 为可选的自定义授权名称。
pub fn install(
    state: &WorkspaceState,
    product_id: &str,
    license_name: Option<&str>,
) -> Result<InstallResult> {
    let resource_root = state.resource_root();
    let workdir = state.workdir();
    let jar = bundled_jar_path(&resource_root);

    if !jar.exists() {
        return Ok(InstallResult {
            product_id: product_id.to_string(),
            success: false,
            vmoptions_path: None,
            jar_path: platform::normalize_path_for_output(&jar),
            message: format!("未找到项目自带的 lib.jar：{}", jar.display()),
        });
    }

    // 优先查找 IDE 自带的 vmoptions 文件（最可靠的方式）
    let ide_vmoptions = find_ide_vmoptions(product_id);

    let (vm_path_to_edit, used_ide_file) = if let Some(ref ide_path) = ide_vmoptions {
        // 找到了 IDE 自带的 vmoptions 文件，直接编辑它
        (ide_path.clone(), true)
    } else {
        // 未找到 IDE 自带 vmoptions，回退到工作区模板
        let template = resource_root
            .join("vmoptions")
            .join(format!("{}.vmoptions", product_id));
        if !template.exists() {
            return Ok(InstallResult {
                product_id: product_id.to_string(),
                success: false,
                vmoptions_path: None,
                jar_path: platform::normalize_path_for_output(&jar),
                message: format!(
                    "未找到 {} 的 vmoptions 文件，请先安装对应 IDE。",
                    product_id
                ),
            });
        }
        // copy-on-write 到工作区
        match copy_on_write(&template, &workdir, product_id) {
            Ok(p) => (p, false),
            Err(e) => {
                return Ok(InstallResult {
                    product_id: product_id.to_string(),
                    success: false,
                    vmoptions_path: None,
                    jar_path: platform::normalize_path_for_output(&jar),
                    message: format!("复制 vmoptions 到工作区失败：{}", e),
                })
            }
        }
    };

    // 编辑 vmoptions：先剥离旧的 javaagent 行，再追加新的
    vmoptions::ensure_javaagent(&vm_path_to_edit, &jar, license_name)?;

    // 同时设置环境变量（双保险）
    let env_var = platform::env_var_name(product_id);
    let vm_path_native = vm_path_to_edit.to_string_lossy().to_string();
    match Os::current() {
        Os::Macos => {
            std::process::Command::new("launchctl")
                .args(["setenv", &env_var, &vm_path_native])
                .status()
                .ok();
            write_shell_rc(&env_var, &vm_path_native)?;
        }
        Os::Linux => {
            write_shell_rc(&env_var, &vm_path_native)?;
        }
        Os::Windows => {
            // setx 使用原生路径（反斜杠）
            std::process::Command::new("setx")
                .args([&env_var, &vm_path_native])
                .status()
                .ok();
        }
    }

    let vm_path_display = platform::normalize_path_for_output(&vm_path_to_edit);
    let method = if used_ide_file { "IDE 自带 vmoptions" } else { "工作区模板" };

    logger::append(
        logger::Level::Info,
        &format!(
            "[{}] 已安装 javaagent -> {}（{}）{}",
            product_id,
            vm_path_display,
            method,
            if let Some(n) = license_name {
                format!("（自定义授权名称：{}）", n)
            } else {
                String::new()
            }
        ),
    );

    Ok(InstallResult {
        product_id: product_id.to_string(),
        success: true,
        vmoptions_path: Some(vm_path_display),
        jar_path: platform::normalize_path_for_output(&jar),
        message: format!("javaagent 已写入{}（{}）", method, vm_path_display),
    })
}

/// 为单个产品卸载 javaagent。
pub fn uninstall(state: &WorkspaceState, product_id: &str) -> Result<InstallResult> {
    let resource_root = state.resource_root();
    let jar = bundled_jar_path(&resource_root);

    // 查找所有可能的 vmoptions 文件并清理
    let mut cleaned_paths: Vec<PathBuf> = Vec::new();

    // 1. IDE 自带的 vmoptions
    if let Some(ide_path) = find_ide_vmoptions(product_id) {
        if ide_path.exists() {
            vmoptions::strip_javaagent(&ide_path)?;
            cleaned_paths.push(ide_path);
        }
    }

    // 2. 工作区副本
    let workdir = state.workdir();
    let workspace_copy = workdir
        .join("vmoptions")
        .join(format!("{}.vmoptions", product_id));
    if workspace_copy.exists() {
        vmoptions::strip_javaagent(&workspace_copy)?;
        cleaned_paths.push(workspace_copy);
    }

    // 清理环境变量
    let env_var = platform::env_var_name(product_id);
    if Os::current() == Os::Macos {
        std::process::Command::new("launchctl")
            .args(["unsetenv", &env_var])
            .status()
            .ok();
    }
    remove_shell_rc(&env_var)?;

    let paths_display: Vec<String> = cleaned_paths
        .iter()
        .map(|p| platform::normalize_path_for_output(p))
        .collect();

    logger::append(
        logger::Level::Info,
        &format!(
            "[{}] 已从 {} 处移除 javaagent",
            product_id,
            if paths_display.is_empty() {
                "无".to_string()
            } else {
                paths_display.join(", ")
            }
        ),
    );

    Ok(InstallResult {
        product_id: product_id.to_string(),
        success: true,
        vmoptions_path: cleaned_paths.first().map(|p| platform::normalize_path_for_output(p)),
        jar_path: platform::normalize_path_for_output(&jar),
        message: format!("已从 {} 处移除 javaagent 行", paths_display.len()),
    })
}

/// 查找 IDE 自带的 vmoptions 文件。
///
/// Windows:
///   - %APPDATA%\JetBrains\<product>*\<product>64.exe.vmoptions
///   - C:\Program Files\JetBrains\<Product>*\bin\<product>64.exe.vmoptions
///   - %LOCALAPPDATA%\Programs\<Product>\bin\<product>64.exe.vmoptions
///
/// macOS:
///   - ~/Library/Application Support/JetBrains/<product>*/<product>.vmoptions
///
/// Linux:
///   - ~/.config/JetBrains/<product>*/<product>.vmoptions
///   - ~/.<product><version>/<product>.vmoptions
pub fn find_ide_vmoptions(product_id: &str) -> Option<PathBuf> {
    let os = Os::current();

    // vmoptions 文件名候选
    let vmoptions_names: Vec<String> = match os {
        Os::Windows => vec![
            format!("{}64.exe.vmoptions", product_id),
            format!("{}.exe.vmoptions", product_id),
            format!("{}.vmoptions", product_id),
        ],
        _ => vec![format!("{}.vmoptions", product_id)],
    };

    // 搜索目录列表
    let search_dirs: Vec<PathBuf> = match os {
        Os::Windows => {
            let mut dirs = Vec::new();
            // %APPDATA%\JetBrains\
            if let Some(appdata) = dirs::config_dir() {
                dirs.push(appdata.join("JetBrains"));
            }
            // C:\Program Files\JetBrains\
            if let Some(pf) = dirs::home_dir() {
                // Try common Program Files paths
                for drive in &["C:", "D:", "E:"] {
                    dirs.push(PathBuf::from(format!("{}\\Program Files\\JetBrains", drive)));
                    dirs.push(PathBuf::from(format!(
                        "{}\\Program Files (x86)\\JetBrains",
                        drive
                    )));
                }
            }
            // %LOCALAPPDATA%\Programs\
            if let Some(local) = dirs::data_local_dir() {
                dirs.push(local.join("Programs"));
            }
            dirs
        }
        Os::Macos => {
            let mut dirs = Vec::new();
            if let Some(home) = dirs::home_dir() {
                dirs.push(home.join("Library").join("Application Support").join("JetBrains"));
            }
            dirs
        }
        Os::Linux => {
            let mut dirs = Vec::new();
            if let Some(config) = dirs::config_dir() {
                dirs.push(config.join("JetBrains"));
            }
            // 旧版本可能在 ~/.<product><version>/
            if let Some(home) = dirs::home_dir() {
                dirs.push(home);
            }
            dirs
        }
    };

    // 在每个搜索目录中查找 vmoptions 文件
    for search_dir in &search_dirs {
        if !search_dir.exists() {
            continue;
        }
        if let Ok(entries) = fs::read_dir(search_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    // 在子目录中查找 vmoptions 文件
                    for name in &vmoptions_names {
                        // 直接在子目录中
                        let vmoptions = path.join(name);
                        if vmoptions.exists() {
                            return Some(vmoptions);
                        }
                        // 在 bin 子目录中（Windows IDE 安装目录结构）
                        let vmoptions_bin = path.join("bin").join(name);
                        if vmoptions_bin.exists() {
                            return Some(vmoptions_bin);
                        }
                    }
                } else if path.is_file() {
                    // 直接是 vmoptions 文件（如 ~/.<product><version>.vmoptions）
                    if let Some(fname) = path.file_name().and_then(|s| s.to_str()) {
                        for name in &vmoptions_names {
                            if fname == name {
                                return Some(path);
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

/// Copy-on-write：将项目自带的 vmoptions 模板复制到工作区，返回工作区副本路径。
fn copy_on_write(template: &Path, workdir: &Path, product_id: &str) -> Result<PathBuf> {
    let target_dir = workdir.join("vmoptions");
    fs::create_dir_all(&target_dir)?;
    let target = target_dir.join(format!("{}.vmoptions", product_id));
    fs::copy(template, &target)?;
    log::info!(
        "copy-on-write：{} -> {}",
        template.display(),
        target.display()
    );
    Ok(target)
}

/// 在 ~/.profile、~/.bashrc、~/.zshrc 中追加 `export <ENV>=<path>`。幂等。
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
        log::warn!("无法将环境变量 {} 写入任何 shell rc 文件", env_var);
    }
    Ok(())
}

/// 从 ~/.profile、~/.bashrc、~/.zshrc 中移除指定环境变量的 export 行。
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
