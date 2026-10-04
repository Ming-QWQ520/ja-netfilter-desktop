//! 跨平台安装 / 卸载逻辑。
//!
//! 安装时直接查找并编辑 IDE 自带的 vmoptions 文件（追加 -javaagent 行），
//! 不再依赖环境变量。若未找到 IDE 自带 vmoptions，回退到工作区模板。
//! lib.jar 始终使用项目自带路径（不复制到工作区）。
//! Windows 上 setx 后广播 WM_SETTINGCHANGE，确保 IDE 能立即读取新环境变量。

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
            // setx 持久化环境变量到注册表
            std::process::Command::new("setx")
                .args([&env_var, &vm_path_native])
                .status()
                .ok();
            // 广播 WM_SETTINGCHANGE，让 Windows Explorer 和其他进程立即感知
            // 环境变量变更。否则用户需要注销/重启才能生效。
            broadcast_env_change();
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
        vmoptions_path: Some(vm_path_display.clone()),
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

    // Windows 上删除环境变量并广播
    if Os::current() == Os::Windows {
        std::process::Command::new("reg")
            .args(["delete", "HKCU\\Environment", "/v", &env_var, "/f"])
            .status()
            .ok();
        broadcast_env_change();
    }

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

/// 查找 IDE 自带的 vmoptions 文件。Windows 搜索 %APPDATA%\JetBrains\ 和
/// C:\Program Files\JetBrains\ 等目录。macOS 搜索 ~/Library/Application Support/JetBrains/。
/// Linux 搜索 ~/.config/Jetbrains/。使用前缀匹配查找版本号目录。
pub fn find_ide_vmoptions(product_id: &str) -> Option<PathBuf> {
    let os = Os::current();

    // vmoptions 文件名候选（Windows 用 .exe.vmoptions，其他平台用 .vmoptions）
    let vmoptions_names: Vec<String> = match os {
        Os::Windows => vec![
            format!("{}64.exe.vmoptions", product_id),
            format!("{}.exe.vmoptions", product_id),
            format!("{}.vmoptions", product_id),
        ],
        _ => vec![format!("{}.vmoptions", product_id)],
    };

    // 产品目录名前缀映射（AppData 中的目录名前缀）
    // 例如 IntelliJ IDEA 在 AppData 中是 IntelliJIdea2024.2
    let dir_prefix = product_dir_prefix(product_id);

    // 搜索目录列表
    let search_roots: Vec<PathBuf> = match os {
        Os::Windows => {
            let mut roots = Vec::new();
            // %APPDATA%\JetBrains\
            if let Some(appdata) = dirs::config_dir() {
                roots.push(appdata.join("JetBrains"));
            }
            // C:\Program Files\JetBrains\ (try common drives)
            for drive in &["C:", "D:", "E:"] {
                roots.push(PathBuf::from(format!("{}\\Program Files\\JetBrains", drive)));
                roots.push(PathBuf::from(format!("{}\\Program Files (x86)\\JetBrains", drive)));
            }
            // %LOCALAPPDATA%\JetBrains\Toolbox\apps\ (Toolbox installation)
            if let Some(local) = dirs::data_local_dir() {
                roots.push(local.join("JetBrains").join("Toolbox").join("apps"));
            }
            // %LOCALAPPDATA%\Programs\
            if let Some(local) = dirs::data_local_dir() {
                roots.push(local.join("Programs"));
            }
            roots
        }
        Os::Macos => {
            let mut roots = Vec::new();
            if let Some(home) = dirs::home_dir() {
                roots.push(home.join("Library").join("Application Support").join("JetBrains"));
            }
            // /Applications/ for IDE install dirs
            roots.push(PathBuf::from("/Applications"));
            roots
        }
        Os::Linux => {
            let mut roots = Vec::new();
            if let Some(config) = dirs::config_dir() {
                roots.push(config.join("JetBrains"));
            }
            // /opt/ for IDE install dirs
            roots.push(PathBuf::from("/opt"));
            // ~/.local/share/JetBrains/
            if let Some(home) = dirs::home_dir() {
                roots.push(home.join(".local").join("share").join("JetBrains"));
            }
            roots
        }
    };

    // 在每个搜索根目录中查找 vmoptions 文件
    for search_root in &search_roots {
        if !search_root.exists() {
            continue;
        }
        if let Ok(entries) = fs::read_dir(search_root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let dir_name = path.file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("");

                    // 检查目录名是否匹配产品前缀（前缀匹配，忽略大小写）
                    let dir_matches = dir_name.to_lowercase().starts_with(&dir_prefix.to_lowercase());

                    if dir_matches {
                        // 在匹配的目录中查找 vmoptions 文件
                        for name in &vmoptions_names {
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
                    }

                    // Toolbox 安装：在 apps/<product>/ch-0/<version>/bin/ 下
                    // 搜索更深一层
                    if search_root.ends_with("apps") {
                        if let Ok(sub_entries) = fs::read_dir(&path) {
                            for sub_entry in sub_entries.flatten() {
                                let sub_path = sub_entry.path();
                                if sub_path.is_dir() {
                                    // ch-0 目录
                                    let ch_dir = sub_path.join("ch-0");
                                    if ch_dir.exists() {
                                        if let Ok(ch_entries) = fs::read_dir(&ch_dir) {
                                            for ch_entry in ch_entries.flatten() {
                                                let ver_path = ch_entry.path();
                                                if ver_path.is_dir() {
                                                    for name in &vmoptions_names {
                                                        let vmoptions = ver_path.join("bin").join(name);
                                                        if vmoptions.exists() {
                                                            return Some(vmoptions);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

/// 产品 ID 到 JetBrains AppData 目录名前缀的映射。
/// 例如 idea -> IntelliJIdea, pycharm -> PyCharm, etc.
fn product_dir_prefix(product_id: &str) -> String {
    match product_id {
        "idea" => "IntelliJIdea".to_string(),
        "clion" => "CLion".to_string(),
        "phpstorm" => "PhpStorm".to_string(),
        "goland" => "GoLand".to_string(),
        "pycharm" => "PyCharm".to_string(),
        "webstorm" => "WebStorm".to_string(),
        "webide" => "WebStorm".to_string(),
        "rider" => "Rider".to_string(),
        "datagrip" => "DataGrip".to_string(),
        "rubymine" => "RubyMine".to_string(),
        "dataspell" => "DataSpell".to_string(),
        "aqua" => "Aqua".to_string(),
        "rustrover" => "RustRover".to_string(),
        "gateway" => "JetBrainsGateway".to_string(),
        "jetbrains_client" => "JetBrainsClient".to_string(),
        "jetbrainsclient" => "JetBrainsClient".to_string(),
        "studio" => "AndroidStudio".to_string(),
        "devecostudio" => "DevEcoStudio".to_string(),
        _ => product_id.to_string(),
    }
}

/// Windows 上广播 WM_SETTINGCHANGE 消息，让所有顶级窗口（包括 Explorer）
/// 感知环境变量变更。使用 PowerShell 调用 Win32 API。
fn broadcast_env_change() {
    #[cfg(target_os = "windows")]
    {
        let ps_script = r#"
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public class Win32Env {
    [DllImport("user32.dll", SetLastError = true, CharSet = CharSet.Auto)]
    public static extern IntPtr SendMessageTimeout(
        IntPtr hWnd, uint Msg, UIntPtr wParam, string lParam,
        uint fuFlags, uint uTimeout, out UIntPtr lpdwResult);
}
"@ -ErrorAction SilentlyContinue
[Win32Env]::SendMessageTimeout([IntPtr]0xffff, 0x1a, [UIntPtr]::Zero, 'Environment', 2, 5000, [ref]([UIntPtr]::Zero)) | Out-Null
"#;
        std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", ps_script])
            .status()
            .ok();
    }
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
