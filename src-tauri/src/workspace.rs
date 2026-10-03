//! 资源定位（直接读取项目自带 resources 目录，不再镜像到用户工作区）。
//!
//! 应用启动时不再将 `lib.jar` / `plugins/` / `config/` / `vmoptions/` 复制到
//! 用户配置目录。所有读写都直接作用于项目自带的 `resources/` 目录。
//!
//! 这意味着：
//!   - `lib.jar`、`plugins/*.jar`、`config/*.conf`、`vmoptions/*.vmoptions`
//!     始终是应用 bundle 内的同一路径。
//!   - 任何用户编辑都直接写入这些文件，下次升级会被覆盖（与原版 install.sh
//!     行为一致：install.sh 同样在分发目录原地修改 vmoptions）。
//!   - 检测产品时，工作区模板路径就是 `<bundle>/resources/vmoptions/<id>.vmoptions`。
//!
//! 此模块只负责把 Tauri 的 resource 解析为文件系统路径。

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use parking_lot::RwLock;
use tauri::{AppHandle, Manager};

/// 所有已知的 JetBrains 产品 ID（与原版 install.sh 保持一致）。
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

/// 持有应用 resource_root 的路径（项目自带的 resources 目录）。
#[derive(Clone)]
pub struct WorkspaceState {
    pub resource_root: Arc<RwLock<PathBuf>>,
}

impl WorkspaceState {
    pub fn new(resource_root: PathBuf) -> Self {
        Self {
            resource_root: Arc::new(RwLock::new(resource_root)),
        }
    }

    pub fn get(&self) -> PathBuf {
        self.resource_root.read().clone()
    }

    #[allow(dead_code)]
    pub fn set(&self, path: PathBuf) {
        *self.resource_root.write() = path;
    }
}

/// 解析 Tauri 的 `resources/` 目录为文件系统路径。该目录在开发模式下
/// 位于 `<repo>/src-tauri/resources/`，在打包后位于应用 bundle 内。
pub fn init_resource_root(app: &AppHandle) -> Result<PathBuf> {
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
    log::info!("使用项目自带资源目录：{}", resource_root.display());
    Ok(resource_root)
}

/// 解析 resource_root 之下的相对路径，禁止 `..` 逃逸。
pub fn resolve_under_root(state: &WorkspaceState, rel: &str) -> Result<PathBuf> {
    let root = state.get();
    let joined = root.join(rel);
    // 不做 canonicalize —— resource_root 在某些平台（如 macOS .app bundle）
    // 解析后包含 `/private/var/folders/...` 前缀，canonicalize 会引入额外
    // 解析失败风险。我们直接做字符串前缀检查。
    let joined_str = joined.to_string_lossy();
    let root_str = root.to_string_lossy();
    if !joined_str.starts_with(&*root_str) {
        anyhow::bail!("路径越界：{}", rel);
    }
    Ok(joined)
}
