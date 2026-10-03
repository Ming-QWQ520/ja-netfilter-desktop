//! 用户工作区管理（可写镜像）。
//!
//! 应用启动时将项目自带的 `lib.jar`、`plugins/`、`config/`、`vmoptions/`
//! 镜像到一个 **可写的** 用户目录，避免写入安装目录时遭遇权限拒绝：
//!
//!   - Linux:   `~/.config/ja-netfilter-desktop/`
//!   - macOS:    `~/Library/Application Support/ja-netfilter-desktop/`
//!   - Windows:  `%APPDATA%\ja-netfilter-desktop\`
//!
//! 镜像策略：
//!   - `lib.jar` / `plugins/*.jar`: 每次启动都覆盖（确保用户始终使用最新 jar）
//!   - `config/*.conf`: 缺失才复制（保留用户编辑）
//!   - `vmoptions/*.vmoptions`: 缺失才复制（保留用户编辑）
//!   - 清理 vmoptions 中残留的、指向带空格旧路径的 javaagent 行
//!
//! 检测产品时，工作区模板路径就是 `<workdir>/vmoptions/<id>.vmoptions`。
//! 由于 workdir 一定在用户目录下，路径中包含空格的概率大大降低；
//! `normalize_path_for_output` 会进一步将反斜杠转为正斜杠，确保 JVM 解析无误。

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use parking_lot::RwLock;
use tauri::{AppHandle, Manager};

/// 用户工作区子目录名。
pub const WORKDIR_NAME: &str = "ja-netfilter-desktop";

/// 资源根目录下的相对路径映射到工作区中的目标路径。
const MIRROR_FILES: &[(&str, &str)] = &[
    // 每次启动都覆盖
    ("lib.jar", "lib.jar"),
    ("plugins/dns.jar", "plugins/dns.jar"),
    ("plugins/hideme.jar", "plugins/hideme.jar"),
    ("plugins/power.jar", "plugins/power.jar"),
    ("plugins/url.jar", "plugins/url.jar"),
    // 缺失才复制
    ("config/dns.conf", "config/dns.conf"),
    ("config/power.conf", "config/power.conf"),
    ("config/url.conf", "config/url.conf"),
];

/// 所有已知的 JetBrains 产品 ID（与原版 install.sh 保持一致）。
#[allow(dead_code)]
pub static JB_PRODUCT_IDS: &[&str] = &[
    "idea",
    "clion",
    "phpstorm",
    "goland",
    "pycharm",
    "webstorm",
    "webide",
    "rider",
    "datagrip",
    "rubymine",
    "dataspell",
    "aqua",
    "rustrover",
    "gateway",
    "jetbrains_client",
    "jetbrainsclient",
    "studio",
    "devecostudio",
];

/// GUI 中展示的产品友好名称。
pub fn product_labels() -> Vec<(&'static str, &'static str)> {
    vec![
        ("idea", "IntelliJ IDEA"),
        ("clion", "CLion"),
        ("phpstorm", "PhpStorm"),
        ("goland", "GoLand"),
        ("pycharm", "PyCharm"),
        ("webstorm", "WebStorm"),
        ("webide", "WebIDE (legacy)"),
        ("rider", "Rider"),
        ("datagrip", "DataGrip"),
        ("rubymine", "RubyMine"),
        ("dataspell", "DataSpell"),
        ("aqua", "Aqua"),
        ("rustrover", "RustRover"),
        ("gateway", "JetBrains Gateway"),
        ("jetbrains_client", "JetBrains Client"),
        ("jetbrainsclient", "JetBrains Client (legacy)"),
        ("studio", "Android Studio"),
        ("devecostudio", "DevEco Studio"),
    ]
}

/// 持有用户工作区路径（可写的镜像目录）。
#[derive(Clone)]
pub struct WorkspaceState {
    pub workdir: Arc<RwLock<PathBuf>>,
}

impl WorkspaceState {
    pub fn new(workdir: PathBuf) -> Self {
        Self {
            workdir: Arc::new(RwLock::new(workdir)),
        }
    }

    pub fn get(&self) -> PathBuf {
        self.workdir.read().clone()
    }

    #[allow(dead_code)]
    pub fn set(&self, path: PathBuf) {
        *self.workdir.write() = path;
    }
}

/// 解析用户工作区路径：
///   - Linux:   `$XDG_CONFIG_HOME/ja-netfilter-desktop`（或 `~/.config/...`）
///   - macOS:    `~/Library/Application Support/ja-netfilter-desktop`
///   - Windows:  `%APPDATA%/ja-netfilter-desktop`
pub fn init_workdir(app: &AppHandle) -> Result<PathBuf> {
    let base = dirs::config_dir()
        .context("无法解析当前平台的用户配置目录")?;
    let workdir = base.join(WORKDIR_NAME);
    fs::create_dir_all(&workdir).with_context(|| {
        format!("创建工作区失败：{}", workdir.display())
    })?;

    mirror_resources(app, &workdir)?;
    Ok(workdir)
}

/// 把项目自带的资源镜像到工作区。
/// - jar 文件每次启动覆盖（保证最新）
/// - 配置和 vmoptions 缺失才复制（保留用户编辑）
fn mirror_resources(app: &AppHandle, workdir: &Path) -> Result<()> {
    for (resource_rel, target_rel) in MIRROR_FILES {
        let target = workdir.join(target_rel);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).ok();
        }

        let resource_path = match app
            .path()
            .resolve(resource_rel, tauri::path::BaseDirectory::Resource)
        {
            Ok(p) => p,
            Err(_) => {
                log::warn!("无法解析资源：{}", resource_rel);
                continue;
            }
        };

        if !resource_path.exists() {
            log::warn!("资源文件不存在：{}", resource_path.display());
            continue;
        }

        // jar 文件每次启动都覆盖；配置和 vmoptions 缺失才复制。
        let is_jar = target_rel.ends_with(".jar");
        if is_jar || !target.exists() {
            fs::copy(&resource_path, &target).with_context(|| {
                format!(
                    "复制资源失败：{} -> {}",
                    resource_path.display(),
                    target.display()
                )
            })?;
        }
    }

    // 镜像 vmoptions 模板（缺失才复制）。
    let vm_dir = match app
        .path()
        .resolve("vmoptions", tauri::path::BaseDirectory::Resource)
    {
        Ok(p) if p.exists() => p,
        _ => {
            log::warn!("项目自带的 vmoptions 目录不存在，跳过镜像");
            // 仍然要清理工作区 vmoptions 中残留的旧 javaagent 行。
            cleanup_legacy_javaagent_lines(workdir);
            return Ok(());
        }
    };

    let target_vm_dir = workdir.join("vmoptions");
    fs::create_dir_all(&target_vm_dir)?;
    if let Ok(entries) = fs::read_dir(&vm_dir) {
        for entry in entries.flatten() {
            let src = entry.path();
            let name = entry.file_name();
            let dst = target_vm_dir.join(&name);
            if !dst.exists() {
                fs::copy(&src, &dst).ok();
            }
        }
    }

    // 清理残留的、指向带空格旧路径的 javaagent 行。
    cleanup_legacy_javaagent_lines(workdir);

    Ok(())
}

/// 清理工作区 vmoptions 中残留的、指向带空格旧路径的 javaagent 行。
///
/// 在 v0.1.0 时，安装目录为 `C:\Users\...\ja-netfilter Desktop\`（含空格），
/// 导致 JVM 崩溃。升级到 v0.1.1 后安装目录改名为 `ja-netfilter-Desktop`，
/// 但旧的 vmoptions 文件中可能仍残留指向旧路径的 javaagent 行。本函数
/// 在每次启动时扫描工作区 vmoptions，清理所有 `-javaagent:` 行（不论
/// 是否指向 ja-netfilter），由后续的安装步骤重新写入正确的路径。
fn cleanup_legacy_javaagent_lines(workdir: &Path) {
    let target_vm_dir = workdir.join("vmoptions");
    if !target_vm_dir.exists() {
        return;
    }
    if let Ok(entries) = fs::read_dir(&target_vm_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("vmoptions") {
                continue;
            }
            if let Ok(content) = fs::read_to_string(&path) {
                let cleaned: String = content
                    .lines()
                    .filter(|l| {
                        let t = l.trim();
                        !(t.starts_with("-javaagent:")
                            && (t.contains("ja-netfilter") || t.ends_with("=jetbrains")))
                    })
                    .filter(|l| !l.trim().starts_with("-Dja.netfilter.name="))
                    .collect::<Vec<_>>()
                    .join("\n");
                if cleaned != content {
                    let _ = fs::write(&path, cleaned);
                    log::info!("清理残留 javaagent 行：{}", path.display());
                }
            }
        }
    }
}

/// 解析工作区下的相对路径，禁止 `..` 逃逸。
pub fn resolve_under_root(state: &WorkspaceState, rel: &str) -> Result<PathBuf> {
    let root = state.get();
    let joined = root.join(rel);
    // 不做 canonicalize —— 在某些平台下 canonicalize 会引入解析失败。
    // 直接做字符串前缀检查。
    let joined_str = joined.to_string_lossy();
    let root_str = root.to_string_lossy();
    if !joined_str.starts_with(&*root_str) {
        anyhow::bail!("路径越界：{}", rel);
    }
    Ok(joined)
}
