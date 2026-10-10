//! 跨平台辅助函数。
//!
//! 集中处理 Linux/macOS/Windows 之间的差异，使其他后端模块保持平台无关。
//!
//! 路径规范化策略参考 `ckey_script.ps1`：
//!   - 写入 vmoptions 的路径必须剥离 `\\?\` / `\\?\UNC\` 前缀（JVM 无法处理）。
//!   - 统一使用正斜杠（JVM 在所有平台都接受）。
//!   - 路径含空格时（如安装到 `C:\Program Files\`），JDK/JetBrains 启动器会按
//!     空白切分 vmoptions 行导致 `-javaagent` 被截断，因此优先通过安全目录部署
//!     规避；若最终路径仍含空白，则用双引号包起来作为兜底。

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
        #[cfg(any(
            target_os = "linux",
            target_os = "freebsd",
            target_os = "openbsd"
        ))]
        {
            Os::Linux
        }
        #[cfg(not(any(
            target_os = "windows",
            target_os = "macos",
            target_os = "linux",
            target_os = "freebsd",
            target_os = "openbsd"
        )))]
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

/// 判断路径字符串是否"干净"——可直接写进 vmoptions 而不需要任何转义。
///
/// 干净 = 只含可见 ASCII、不含空白 / 引号 / 反斜杠。
/// ckey_script.ps1 通过把 agent 放到 `%PUBLIC%\.jb_run\` 来保证这一点；
/// 本应用通过 `agent_home::ensure_deployed` 把 agent 部署到
/// `%LOCALAPPDATA%\ja-netfilter-desktop\agent\` 达到同样效果。
pub fn is_clean_agent_path(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_graphic() && c != '"' && c != '\\' && c != '\'')
}

/// 剥离 Windows verbatim 前缀，返回可直接参与路径拼接/展示/传参的 PathBuf。
///
/// 背景（v0.1.0 现场缺陷）：Tauri 的 `path().resolve(Resource)` 在部分
/// Windows 环境返回 `\\?\E:\...` verbatim 路径。该前缀仅在**纯反斜杠**形式下
/// 对 Win32 API 有效；一旦把反斜杠替换成正斜杠（JVM vmoptions 的惯例写法），
/// 就变成 `//?/E:/...`，JVM 打不开该 jar，IDE 报
/// "FATAL ERROR in native method: processing of -javaagent failed" 且拒绝启动。
/// 因此必须在路径进入 WorkspaceState **之前**剥离前缀，让 `-javaagent`
/// 拿到的是普通 drive 路径（JVM 对 `E:/foo/bar.jar` 完全兼容）。
pub fn simplify_path(path: PathBuf) -> PathBuf {
    let s = path.to_string_lossy().to_string();
    // 顺序无关紧要：两个前缀互不为前缀，但先剥 UNC 更易读
    let stripped = if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{rest}")
    } else if let Some(rest) = s.strip_prefix(r"\\?\") {
        rest.to_string()
    } else {
        return path; // 普通路径，原样返回
    };
    PathBuf::from(stripped)
}

/// 规范化路径，用于写入 vmoptions 的 `-javaagent:` 行和日志展示。
///
/// 修复点（相对 v0.1.6）：
///   1. **先**检查 `\\?\UNC\` 再检查 `\\?\`——旧实现顺序反了，UNC 路径会被
///      错误地剥成 `UNC/server/share`。
///   2. 反斜杠统一转正斜杠。
pub fn normalize_path_for_output(path: &Path) -> String {
    let s = path.to_string_lossy().to_string();

    // 注意顺序：必须先剥离更长的 `\\?\UNC\` 前缀
    let stripped = if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{}", rest)
    } else if let Some(rest) = s.strip_prefix(r"\\?\") {
        rest.to_string()
    } else {
        s
    };

    stripped.replace('\\', "/")
}

/// 为 vmoptions 文件构建 `-javaagent:` 行。
///
/// - 路径先经 `normalize_path_for_output` 规范化（剥离 `\\?\` 前缀、正斜杠）。
/// - 若路径仍含空白（如 `C:/Program Files/...`），用双引号包住整个路径段，
///   与 JDK argfile / JetBrains 启动器的引号语义兼容：
///   `-javaagent:"C:/Program Files/x/lib.jar"=jetbrains`。
///
/// 正常情况下配合 `agent_home::ensure_deployed`，路径永远是干净的，
/// 引号兜底极少触发。
pub fn javaagent_line(jar_path: &Path) -> String {
    let normalized = normalize_path_for_output(jar_path);
    if is_clean_agent_path(&normalized) {
        format!("-javaagent:{}=jetbrains", normalized)
    } else {
        format!("-javaagent:\"{}\"=jetbrains", normalized)
    }
}

/// 计算产品 ID 对应的环境变量名（`<PRODUCT>_VM_OPTIONS`）。
///
/// 与 ckey_script.ps1 一致，本版本安装时**删除**该变量（User + Machine 两个
/// 作用域），避免旧安装残留的环境变量指向失效的 vmoptions 文件。
pub fn env_var_name(product_id: &str) -> String {
    format!("{}_VM_OPTIONS", product_id.to_uppercase())
}

/// 返回在系统文件管理器中打开路径的默认 shell 命令。
///
/// Windows 上若目标是文件则使用 `explorer /select,` 定位到文件。
pub fn open_in_explorer_cmd(path: &Path) -> Option<(String, Vec<String>)> {
    match Os::current() {
        Os::Macos => Some(("open".into(), vec![path.display().to_string()])),
        Os::Windows => {
            if path.is_file() {
                Some((
                    "explorer".into(),
                    vec![format!("/select,{}", path.display())],
                ))
            } else {
                Some(("explorer".into(), vec![path.display().to_string()]))
            }
        }
        Os::Linux => Some(("xdg-open".into(), vec![path.display().to_string()])),
    }
}

/// 各平台上 JetBrains 产品配置/缓存根目录（ckey_script.ps1 的
/// `AppData\Local\JetBrains` 对应物）：
///   - Windows: `%LOCALAPPDATA%\JetBrains`
///   - macOS:   `~/Library/Application Support/JetBrains`
///   - Linux:   `~/.local/share/JetBrains`
pub fn jetbrains_local_root() -> Option<PathBuf> {
    match Os::current() {
        Os::Windows => dirs::data_local_dir().map(|d| d.join("JetBrains")),
        Os::Macos => dirs::home_dir()
            .map(|h| h.join("Library").join("Application Support").join("JetBrains")),
        Os::Linux => dirs::data_dir()
            .or_else(|| dirs::home_dir().map(|h| h.join(".local").join("share")))
            .map(|d| d.join("JetBrains")),
    }
}

/// 各平台上 JetBrains 用户配置根目录（ckey_script.ps1 的
/// `AppData\Roaming\JetBrains` 对应物）：
///   - Windows: `%APPDATA%\JetBrains`
///   - macOS:   `~/Library/Application Support/JetBrains`
///   - Linux:   `~/.config/JetBrains`
pub fn jetbrains_roaming_root() -> Option<PathBuf> {
    match Os::current() {
        Os::Windows => dirs::config_dir().map(|d| d.join("JetBrains")),
        Os::Macos => dirs::home_dir()
            .map(|h| h.join("Library").join("Application Support").join("JetBrains")),
        Os::Linux => dirs::config_dir().map(|d| d.join("JetBrains")),
    }
}
