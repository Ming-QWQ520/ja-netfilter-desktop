//! 跨平台辅助函数。
//!
//! 集中处理 Linux/macOS/Windows 之间的差异，使其他后端模块保持平台无关。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Os {
    Windows,
    Macos,
    Linux,
}

impl Os {
    pub fn current() -> Self {
        #[cfg(target_os = "windows")]
        {
            Os::Windows
        }
        #[cfg(target_os = "macos")]
        {
            Os::Macos
        }
        #[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "openbsd"))]
        {
            Os::Linux
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux", target_os = "freebsd", target_os = "openbsd")))]
        {
            Os::Linux
        }
    }

    #[allow(dead_code)]
    pub fn is_windows(self) -> bool {
        matches!(self, Os::Windows)
    }

    #[allow(dead_code)]
    pub fn is_macos(self) -> bool {
        matches!(self, Os::Macos)
    }
}

/// 规范化路径，用于写入 vmoptions 的 `-javaagent:` 行和设置环境变量。
///
/// 关键修复：
///   1. 剥离 Windows `\\?\` 长路径前缀 —— Tauri 在 Windows 上解析 resource 路径时
///      可能返回带有 `\\?\` 前缀的路径，JVM 的 `-javaagent:` 解析器无法处理该
///      前缀，导致 "processing of -javaagent failed" 致命错误。
///   2. 剥离 UNC 前缀 `\\?\UNC\`。
///   3. 将反斜杠转换为正斜杠（仅用于 javaagent 行）—— JVM 在所有平台上都
///      接受正斜杠，避免了 vmoptions 解析器在空格处分割参数的问题。
///
/// 注意：此函数仅用于生成写入 vmoptions 文件和设置环境变量的路径字符串，
/// 不影响 Rust 内部的文件 I/O（文件 I/O 仍使用原始 PathBuf）。
pub fn normalize_path_for_output(path: &Path) -> String {
    let s = path.to_string_lossy().to_string();

    // 剥离 Windows 长路径前缀
    let stripped = s
        .strip_prefix(r"\\?\")
        .or_else(|| s.strip_prefix(r"\\?\UNC\"))
        .unwrap_or(&s)
        .to_string();

    // 将反斜杠转换为正斜杠 —— JVM 在所有平台上都接受正斜杠，
    // 且正斜杠不会被 vmoptions 解析器误认为是转义字符或参数分隔符。
    stripped.replace('\\', "/")
}

/// 为 vmoptions 文件构建 `-javaagent:` 行。
///
/// 路径经过 `normalize_path_for_output` 规范化：
///   - 剥离 `\\?\` 前缀
///   - 使用正斜杠
///
/// 这样可以确保 JVM 在 Windows 上能正确解析 javaagent 路径，
/// 避免 "processing of -javaagent failed" 致命错误。
pub fn javaagent_line(jar_path: &Path) -> String {
    let normalized = normalize_path_for_output(jar_path);
    format!("-javaagent:{}=jetbrains", normalized)
}

/// 定位 JetBrains 启动器实际读取的 vmoptions 文件。查找顺序与上游
/// install 脚本一致：
///   1. `<PRODUCT>_VM_OPTIONS` 环境变量（如果已显式设置）
///   2. 用户默认 vmoptions 路径（JetBrains 配置目录下）
///   3. 工作区中镜像的模板（`<workdir>/vmoptions/<id>.vmoptions`）
///
/// 注意：此函数已被 installer.rs 和 products.rs 中的本地 find_vmoptions 替代，
/// 保留是为了未来可能的复用。
#[allow(dead_code)]
pub fn find_vmoptions_path(product_id: &str, workdir: &Path) -> Option<PathBuf> {
    let env_key = format!("{}_VM_OPTIONS", product_id.to_uppercase());
    if let Ok(val) = std::env::var(&env_key) {
        let p = PathBuf::from(&val);
        if p.exists() {
            return Some(p);
        }
    }

    if let Some(path) = user_default_vmoptions(product_id) {
        if path.exists() {
            return Some(path);
        }
    }

    let template = workdir
        .join("vmoptions")
        .join(format!("{}.vmoptions", product_id));
    if template.exists() {
        return Some(template);
    }

    None
}

/// 各操作系统下用户默认的 vmoptions 路径。与上游 install 脚本假设的一致。
pub fn user_default_vmoptions(product_id: &str) -> Option<PathBuf> {
    let os = Os::current();
    let config_root = match os {
        Os::Windows => dirs::config_dir()?,
        Os::Macos => dirs::home_dir()?.join("Library").join("Application Support"),
        Os::Linux => dirs::config_dir()?,
    };

    let vendor_dir = match os {
        Os::Windows => "JetBrains",
        _ => "JetBrains",
    };

    Some(
        config_root
            .join(vendor_dir)
            .join(product_id)
            .join("idea.vmoptions"),
    )
}

/// 计算产品 ID 对应的环境变量名。
pub fn env_var_name(product_id: &str) -> String {
    format!("{}_VM_OPTIONS", product_id.to_uppercase())
}

/// 返回在系统文件管理器中打开路径的默认 shell 命令。
pub fn open_in_explorer_cmd(path: &Path) -> Option<(String, Vec<String>)> {
    match Os::current() {
        Os::Macos => Some(("open".into(), vec![path.display().to_string()])),
        Os::Windows => Some(("explorer".into(), vec![path.display().to_string()])),
        Os::Linux => Some(("xdg-open".into(), vec![path.display().to_string()])),
    }
}
