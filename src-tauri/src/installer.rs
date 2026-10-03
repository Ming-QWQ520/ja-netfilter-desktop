//! 跨平台安装 / 卸载逻辑。
//!
//! 本模块替代原版的 shell / VBS 脚本（`scripts/install.sh`、
//! `scripts/install-current-user.vbs` 等），用一个 Rust 实现完成完全相同的步骤：
//!
//! 安装流程：
//!   1. 查找 vmoptions 文件（环境变量 → 用户默认 → 工作区副本 → 项目自带模板）。
//!   2. 若找到的是项目自带模板（只读），先 copy-on-write 到工作区再编辑。
//!   3. lib.jar 始终使用项目自带路径（不复制到工作区）—— JVM 加载 javaagent
//!      不需要写权限。
//!   4. 移除任何已有的指向 ja-netfilter 的 `-javaagent:` 行。
//!   5. 追加 `-javaagent:<resource_root>/lib.jar=jetbrains`。
//!   6. 若提供了 `license_name`，则同时追加 `-Dja.netfilter.name=<license_name>`。
//!   7. 持久化 `<PRODUCT>_VM_OPTIONS` 环境变量，使 IDE 下次启动时读取该文件。
//!
//! 卸载流程：
//!   1. 查找 vmoptions 文件（同上）。
//!   2. 若是项目自带模板，说明从未安装过，直接返回成功。
//!   3. 否则在工作区副本或用户文件中移除 javaagent 行。
//!   4. 清理环境变量。

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

    // 查找 vmoptions 文件
    let (vm_path, is_bundled) = match find_vmoptions(product_id, &workdir, &resource_root) {
        Some((p, bundled)) => (p, bundled),
        None => {
            return Ok(InstallResult {
                product_id: product_id.to_string(),
                success: false,
                vmoptions_path: None,
                jar_path: platform::normalize_path_for_output(&jar),
                message: format!(
                    "未找到 {} 的 vmoptions 文件 —— 请先安装对应 IDE，或使用项目自带模板。",
                    product_id
                ),
            })
        }
    };

    // 若是项目自带模板（只读），先 copy-on-write 到工作区再编辑。
    let vm_path_to_edit = if is_bundled {
        match copy_on_write(&vm_path, &workdir, product_id) {
            Ok(p) => p,
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
    } else {
        vm_path
    };

    // 编辑 vmoptions：先剥离旧的 javaagent 行，再追加新的；若有自定义授权
    // 名称，则同时追加 -Dja.netfilter.name=<value>。
    vmoptions::ensure_javaagent(&vm_path_to_edit, &jar, license_name)?;

    // 持久化环境变量
    let env_var = platform::env_var_name(product_id);
    let vm_path_str = platform::normalize_path_for_output(&vm_path_to_edit);
    match Os::current() {
        Os::Macos => {
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
            std::process::Command::new("setx")
                .args([&env_var, &vm_path_str])
                .status()
                .ok();
        }
    }

    logger::append(
        logger::Level::Info,
        &format!(
            "[{}] 已安装 javaagent -> {}{}",
            product_id,
            vm_path_str,
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
        vmoptions_path: Some(vm_path_str),
        jar_path: platform::normalize_path_for_output(&jar),
        message: "javaagent 已写入 vmoptions 文件".into(),
    })
}

/// 为单个产品卸载 javaagent。
pub fn uninstall(state: &WorkspaceState, product_id: &str) -> Result<InstallResult> {
    let resource_root = state.resource_root();
    let workdir = state.workdir();
    let jar = bundled_jar_path(&resource_root);

    let (vm_path, is_bundled) = match find_vmoptions(product_id, &workdir, &resource_root) {
        Some((p, bundled)) => (p, bundled),
        None => {
            return Ok(InstallResult {
                product_id: product_id.to_string(),
                success: false,
                vmoptions_path: None,
                jar_path: platform::normalize_path_for_output(&jar),
                message: format!("未找到 {} 的 vmoptions 文件，无需卸载。", product_id),
            })
        }
    };

    // 若是项目自带模板，说明从未安装过，直接返回成功。
    if is_bundled {
        return Ok(InstallResult {
            product_id: product_id.to_string(),
            success: true,
            vmoptions_path: Some(platform::normalize_path_for_output(&vm_path)),
            jar_path: platform::normalize_path_for_output(&jar),
            message: "vmoptions 为项目自带模板（只读），未安装过，无需卸载".into(),
        });
    }

    vmoptions::strip_javaagent(&vm_path)?;

    let env_var = platform::env_var_name(product_id);
    if Os::current() == Os::Macos {
        std::process::Command::new("launchctl")
            .args(["unsetenv", &env_var])
            .status()
            .ok();
    }
    remove_shell_rc(&env_var)?;

    let vm_path_str = platform::normalize_path_for_output(&vm_path);
    logger::append(
        logger::Level::Info,
        &format!("[{}] 已从 {} 移除 javaagent", product_id, vm_path_str),
    );

    Ok(InstallResult {
        product_id: product_id.to_string(),
        success: true,
        vmoptions_path: Some(vm_path_str),
        jar_path: platform::normalize_path_for_output(&jar),
        message: "已移除 javaagent 行".into(),
    })
}

/// 查找 vmoptions 文件。返回 (path, is_bundled) —— is_bundled 表示是否为
/// 项目自带模板（只读）。
fn find_vmoptions(
    product_id: &str,
    workdir: &Path,
    resource_root: &Path,
) -> Option<(PathBuf, bool)> {
    let env_key = format!("{}_VM_OPTIONS", product_id.to_uppercase());
    if let Ok(val) = std::env::var(&env_key) {
        let p = PathBuf::from(&val);
        if p.exists() {
            return Some((p, false));
        }
    }

    if let Some(path) = platform::user_default_vmoptions(product_id) {
        if path.exists() {
            return Some((path, false));
        }
    }

    // 工作区副本（用户修改过的）
    let workspace_copy = workdir
        .join("vmoptions")
        .join(format!("{}.vmoptions", product_id));
    if workspace_copy.exists() {
        return Some((workspace_copy, false));
    }

    // 项目自带模板（只读）
    let template = resource_root
        .join("vmoptions")
        .join(format!("{}.vmoptions", product_id));
    if template.exists() {
        return Some((template, true));
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
