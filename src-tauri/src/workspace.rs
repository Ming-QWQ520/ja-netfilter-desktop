//! 工作区状态与应用内路径根。
//!
//! v0.1.0 设计：**项目自带的文件不复制**。
//!   - `bundle_root`：安装目录内的应用资源（lib.jar / config-jetbrains /
//!     plugins-jetbrains / vmoptions 模板），agent **就地引用**该目录。
//!   - `agent_root`：JVM 实际加载的运行时目录 —— 默认等于 `bundle_root`；
//!     仅当资源路径含空格且 8.3 短路径不可用时，installer 才会把它切到
//!     agent_home::deploy_fallback 的兜底副本。
//!   - `workdir`：杂项数据目录（%APPDATA%\ja-netfilter-desktop）。

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tauri::{AppHandle, Manager};

/// 用户数据目录名（与 tauri.conf.json 的 identifier 对应）。
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

/// 全局路径状态。
#[derive(Clone)]
pub struct WorkspaceState {
    /// 杂项数据目录。
    pub workdir: PathBuf,
    /// 安装目录内的应用资源（agent 就地引用的根）。
    pub bundle_root: PathBuf,
    /// agent 运行时目录（默认 == bundle_root；极端情况指向兜底副本）。
    pub agent_root: PathBuf,
    /// 写入 vmoptions 的 lib.jar 路径。
    pub agent_jar: PathBuf,
}

impl WorkspaceState {
    pub fn new(
        workdir: PathBuf,
        bundle_root: PathBuf,
        agent_root: PathBuf,
        agent_jar: PathBuf,
    ) -> Self {
        Self {
            workdir,
            bundle_root,
            agent_root,
            agent_jar,
        }
    }
}

/// 初始化工作区与 bundle 资源根（不复制任何文件 —— agent 就地引用）。
pub fn init_workspace(app: &AppHandle) -> Result<(PathBuf, PathBuf)> {
    let base = dirs::config_dir().context("无法解析当前平台的用户配置目录")?;
    let workdir = base.join(WORKDIR_NAME);
    fs::create_dir_all(&workdir)
        .with_context(|| format!("创建工作区失败：{}", workdir.display()))?;

    let bundle_root = app
        .path()
        .resolve("resources", tauri::path::BaseDirectory::Resource)
        .context("无法解析项目自带的 resources 目录")?;
    if !bundle_root.exists() {
        anyhow::bail!(
            "项目自带的 resources 目录不存在：{}",
            bundle_root.display()
        );
    }

    log::info!("工作区：{}", workdir.display());
    log::info!("bundle 资源目录：{}", bundle_root.display());

    Ok((workdir, bundle_root))
}

// ---------------------------------------------------------------------------
// agent_root 上的资源读写（配置 / vmoptions 模板）
// ---------------------------------------------------------------------------

/// 防止 `..` 逃逸出 agent_root。
fn guard_rel(agent_root: &Path, rel: &str) -> Result<PathBuf> {
    if rel.contains("..") {
        anyhow::bail!("非法路径：{}", rel);
    }
    Ok(agent_root.join(rel))
}

/// 读取 agent_root 下的资源文本（config/*.conf、vmoptions/*.vmoptions）。
/// agent_root 与 bundle_root 同目录（就地引用）；若分裂到兜底目录则回退 bundle。
pub fn read_resource(state: &WorkspaceState, rel: &str) -> Result<String> {
    let path = guard_rel(&state.agent_root, rel)?;
    if path.exists() {
        return fs::read_to_string(&path)
            .with_context(|| format!("读取文件失败：{}", path.display()));
    }
    let fallback = state.bundle_root.join(rel);
    if fallback.exists() {
        return fs::read_to_string(&fallback)
            .with_context(|| format!("读取文件失败：{}", fallback.display()));
    }
    anyhow::bail!("资源文件不存在：{}", rel)
}

/// 写入 agent_root 下的资源文本。
pub fn write_resource(state: &WorkspaceState, rel: &str, content: &str) -> Result<PathBuf> {
    let path = guard_rel(&state.agent_root, rel)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(&path, content)
        .with_context(|| format!("写入文件失败：{}", path.display()))?;
    Ok(path)
}
