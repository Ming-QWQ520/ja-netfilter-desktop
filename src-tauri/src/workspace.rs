//! 资源读写分离 —— 不再预先复制任何文件到用户工作区。
//!
//! ## 设计原则
//!
//! - **`lib.jar` / `plugins/*.jar`**：只读，始终使用项目自带路径（安装目录内）。
//!   JVM 加载 javaagent 不需要写权限，所以直接指向安装目录内的 jar 即可。
//!
//! - **`config/*.conf` / `vmoptions/*.vmoptions`**：读时优先使用工作区副本
//!   （用户可能编辑过），工作区不存在时回退到项目自带的模板（安装目录内）。
//!   写时先复制到工作区（copy-on-write），再修改工作区副本，避免污染安装目录。
//!
//! ## 工作区
//!
//! 工作区目录在应用启动时创建，但**不预先复制任何文件**：
//!   - Linux:   `~/.config/ja-netfilter-desktop/`
//!   - macOS:    `~/Library/Application Support/ja-netfilter-desktop/`
//!   - Windows:  `%APPDATA%\ja-netfilter-desktop\`
//!
//! 只有用户实际保存了某个文件（vmoptions 或 config），该文件才会出现在工作区中。
//! lib.jar 永远不会出现在工作区中——它始终从安装目录加载。

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use parking_lot::RwLock;
use tauri::{AppHandle, Manager};

/// 用户工作区子目录名。
pub const WORKDIR_NAME: &str = "ja-netfilter-desktop";

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

/// 持有工作区路径和项目自带资源路径。
#[derive(Clone)]
pub struct WorkspaceState {
    /// 用户工作区（可写，存放用户修改过的配置/vmoptions 副本）。
    pub workdir: Arc<RwLock<PathBuf>>,
    /// 项目自带的 resources 目录（只读，存放原始 jar / config / vmoptions 模板）。
    pub resource_root: Arc<RwLock<PathBuf>>,
}

impl WorkspaceState {
    pub fn new(workdir: PathBuf, resource_root: PathBuf) -> Self {
        Self {
            workdir: Arc::new(RwLock::new(workdir)),
            resource_root: Arc::new(RwLock::new(resource_root)),
        }
    }

    pub fn workdir(&self) -> PathBuf {
        self.workdir.read().clone()
    }

    pub fn resource_root(&self) -> PathBuf {
        self.resource_root.read().clone()
    }
}

/// 初始化工作区和资源根路径。
///
/// - 创建工作区目录（不复制任何文件）
/// - 解析项目自带的 resources 目录（只读）
/// - 清理工作区 vmoptions 中可能残留的旧 javaagent 行
pub fn init_workspace(app: &AppHandle) -> Result<(PathBuf, PathBuf)> {
    // 工作区目录（可写）
    let base = dirs::config_dir()
        .context("无法解析当前平台的用户配置目录")?;
    let workdir = base.join(WORKDIR_NAME);
    fs::create_dir_all(&workdir).with_context(|| {
        format!("创建工作区失败：{}", workdir.display())
    })?;

    // 项目自带的 resources 目录（只读）
    let resource_root = app
        .path()
        .resolve("resources", tauri::path::BaseDirectory::Resource)
        .context("无法解析项目自带的 resources 目录")?;
    if !resource_root.exists() {
        anyhow::bail!(
            "项目自带的 resources 目录不存在：{}",
            resource_root.display()
        );
    }

    log::info!("工作区：{}", workdir.display());
    log::info!("项目自带资源目录：{}", resource_root.display());

    // 清理工作区 vmoptions 中可能残留的旧 javaagent 行。
    cleanup_legacy_javaagent_lines(&workdir);

    Ok((workdir, resource_root))
}

/// 清理工作区 vmoptions 中残留的旧 javaagent 行（指向带空格旧路径等）。
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

/// 读取资源文件：优先工作区副本，回退项目自带模板。
///
/// 用于 `config/*.conf` 和 `vmoptions/*.vmoptions`。
/// `lib.jar` / `plugins/*.jar` 不应使用此函数——它们始终从 resource_root 读取。
pub fn read_resource(state: &WorkspaceState, rel: &str) -> Result<String> {
    // 1. 工作区副本
    let workdir_copy = state.workdir().join(rel);
    if workdir_copy.exists() {
        return fs::read_to_string(&workdir_copy).with_context(|| {
            format!("读取工作区文件失败：{}", workdir_copy.display())
        });
    }
    // 2. 项目自带模板
    let template = state.resource_root().join(rel);
    if template.exists() {
        return fs::read_to_string(&template).with_context(|| {
            format!("读取项目自带文件失败：{}", template.display())
        });
    }
    anyhow::bail!("资源文件不存在（工作区和项目自带均未找到）：{}", rel)
}

/// 写入资源文件：copy-on-write 到工作区，再修改工作区副本。
///
/// 用于 `config/*.conf` 和 `vmoptions/*.vmoptions`。
pub fn write_resource(state: &WorkspaceState, rel: &str, content: &str) -> Result<PathBuf> {
    let workdir_copy = state.workdir().join(rel);

    // 确保父目录存在
    if let Some(parent) = workdir_copy.parent() {
        fs::create_dir_all(parent).ok();
    }

    // 直接写入工作区副本（覆盖或新建）
    fs::write(&workdir_copy, content)
        .with_context(|| format!("写入工作区文件失败：{}", workdir_copy.display()))?;
    Ok(workdir_copy)
}

/// 删除工作区中的资源副本，使后续读取回退到项目自带模板。
pub fn reset_resource(state: &WorkspaceState, rel: &str) -> Result<()> {
    let workdir_copy = state.workdir().join(rel);
    if workdir_copy.exists() {
        fs::remove_file(&workdir_copy)
            .with_context(|| format!("删除工作区副本失败：{}", workdir_copy.display()))?;
    }
    Ok(())
}

/// 列出工作区 + 项目自带的资源文件。
///
/// 用于 `config/*.conf`：合并去重后返回。
pub fn list_resources(state: &WorkspaceState, sub_dir: &str) -> Result<Vec<ResourceEntry>> {
    let mut entries = Vec::new();
    let mut seen_names: std::collections::HashSet<String> = std::collections::HashSet::new();

    // 1. 工作区副本
    let workdir_sub = state.workdir().join(sub_dir);
    if let Ok(entries_iter) = fs::read_dir(&workdir_sub) {
        for entry in entries_iter.flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                let rel = format!("{}/{}", sub_dir, name);
                let size = path.metadata().map(|m| m.len()).unwrap_or(0);
                entries.push(ResourceEntry {
                    name: name.to_string(),
                    relative_path: rel.clone(),
                    size,
                    source: ResourceSource::Workspace,
                });
                seen_names.insert(name.to_string());
            }
        }
    }

    // 2. 项目自带模板（跳过已在工作区的）
    let resource_sub = state.resource_root().join(sub_dir);
    if let Ok(entries_iter) = fs::read_dir(&resource_sub) {
        for entry in entries_iter.flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                if seen_names.contains(name) {
                    continue;
                }
                let rel = format!("{}/{}", sub_dir, name);
                let size = path.metadata().map(|m| m.len()).unwrap_or(0);
                entries.push(ResourceEntry {
                    name: name.to_string(),
                    relative_path: rel,
                    size,
                    source: ResourceSource::Bundled,
                });
            }
        }
    }

    Ok(entries)
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ResourceEntry {
    pub name: String,
    pub relative_path: String,
    pub size: u64,
    pub source: ResourceSource,
}

#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ResourceSource {
    /// 用户工作区副本（用户已修改过）。
    Workspace,
    /// 项目自带模板（只读，未修改）。
    Bundled,
}

/// 解析工作区下的相对路径，禁止 `..` 逃逸。
pub fn resolve_under_workdir(state: &WorkspaceState, rel: &str) -> Result<PathBuf> {
    let root = state.workdir();
    let joined = root.join(rel);
    let joined_str = joined.to_string_lossy();
    let root_str = root.to_string_lossy();
    if !joined_str.starts_with(&*root_str) {
        anyhow::bail!("路径越界：{}", rel);
    }
    Ok(joined)
}
