//! 跨平台安装 / 卸载逻辑。
//!
//! 本模块替代原版的 shell / VBS 脚本（`scripts/install.sh`、
//! `scripts/install-current-user.vbs` 等），用一个 Rust 实现完成完全相同的步骤：
//!
//!   1. 找到对应产品的 vmoptions 文件（用户配置目录优先，回退到项目自带模板）。
//!   2. 移除任何已有的指向 ja-netfilter 的 `-javaagent:` 行。
//!   3. 追加 `-javaagent:<resource_root>/lib.jar=jetbrains`。
//!   4. 若提供了 `license_name`，则同时追加 `-Dja.netfilter.name=<license_name>`。
//!   5. 持久化 `<PRODUCT>_VM_OPTIONS` 环境变量，使 IDE 下次启动时读取该文件。
//!      - Linux：写入 `~/.profile`、`~/.bashrc`、`~/.zshrc`
//!      - macOS：执行 `launchctl setenv` 并写入 shell rc 文件
//!      - Windows：执行 `setx`
//!
//! 所有副作用都会通过 `logger.rs` 写入内存日志缓冲，供前端日志控制台读取。

use std::fs;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::logger;
use crate::platform::{self, Os};
use crate::products::resource_jar_path;
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
pub fn install(state: &WorkspaceState, product_id: &str, license_name: Option<&str>) -> Result<InstallResult> {
    let resource_root = state.get();
    let jar = resource_jar_path(&resource_root);
    if !jar.exists() {
        return Ok(InstallResult {
            product_id: product_id.to_string(),
            success: false,
            vmoptions_path: None,
            jar_path: jar.display().to_string(),
            message: format!("未找到 ja-netfilter jar：{}", jar.display()),
        });
    }

    let vm_path = match platform::find_vmoptions_path(product_id, &resource_root) {
        Some(p) => p,
        None => {
            return Ok(InstallResult {
                product_id: product_id.to_string(),
                success: false,
                vmoptions_path: None,
                jar_path: jar.display().to_string(),
                message: format!(
                    "未找到 {} 的 vmoptions 文件 —— 请先安装对应 IDE，或使用项目自带模板。",
                    product_id
                ),
            })
        }
    };

    // 原地编辑 vmoptions：先剥离旧的 javaagent 行，再追加新的；若有自定义授权
    // 名称，则同时追加 -Dja.netfilter.name=<value>。
    vmoptions::ensure_javaagent(&vm_path, &jar, license_name)?;

    // 持久化环境变量，使 IDE 下次启动时使用该 vmoptions。
    let env_var = platform::env_var_name(product_id);
    let vm_path_str = vm_path.display().to_string();
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
            vm_path.display(),
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
        jar_path: jar.display().to_string(),
        message: "javaagent 已写入 vmoptions 文件".into(),
    })
}

/// 为单个产品卸载 javaagent。
pub fn uninstall(state: &WorkspaceState, product_id: &str) -> Result<InstallResult> {
    let resource_root = state.get();
    let jar = resource_jar_path(&resource_root);
    let vm_path = match platform::find_vmoptions_path(product_id, &resource_root) {
        Some(p) => p,
        None => {
            return Ok(InstallResult {
                product_id: product_id.to_string(),
                success: false,
                vmoptions_path: None,
                jar_path: jar.display().to_string(),
                message: format!("未找到 {} 的 vmoptions 文件，无需卸载。", product_id),
            })
        }
    };

    vmoptions::strip_javaagent(&vm_path)?;

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
        &format!("[{}] 已从 {} 移除 javaagent", product_id, vm_path.display()),
    );

    Ok(InstallResult {
        product_id: product_id.to_string(),
        success: true,
        vmoptions_path: Some(vm_path.display().to_string()),
        jar_path: jar.display().to_string(),
        message: "已移除 javaagent 行".into(),
    })
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
