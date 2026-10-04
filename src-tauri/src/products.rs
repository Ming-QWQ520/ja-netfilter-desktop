//! JetBrains 产品定义与运行时检测。

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::platform::{self};
use crate::workspace::WorkspaceState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductInfo {
    pub id: String,
    pub name: String,
    pub env_var: String,
    pub vmoptions_path: Option<String>,
    pub vmoptions_source: VmoptionsSource,
    pub javaagent_installed: bool,
    pub javaagent_target: Option<String>,
    pub vmoptions_preview: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VmoptionsSource {
    /// 通过环境变量（`*_VM_OPTIONS`）解析。
    Env,
    /// IDE 自带的 vmoptions 文件（安装目录或用户配置目录中）。
    Ide,
    /// 用户默认配置目录中找到。
    User,
    /// 工作区副本（用户修改过的 vmoptions）。
    Workspace,
    /// 项目自带模板（只读，未修改）。
    Template,
    /// 任何地方都不存在。
    Missing,
}

impl VmoptionsSource {
    #[allow(dead_code)]
    pub fn label(self) -> &'static str {
        match self {
            VmoptionsSource::Env => "环境变量",
            VmoptionsSource::Ide => "IDE 自带",
            VmoptionsSource::User => "用户配置",
            VmoptionsSource::Workspace => "工作区副本",
            VmoptionsSource::Template => "项目模板",
            VmoptionsSource::Missing => "缺失",
        }
    }
}

/// 列出 ja-netfilter 已知的全部产品（id, 友好名称）。
pub fn list_known_products() -> Vec<(&'static str, &'static str)> {
    crate::workspace::product_labels()
}

/// 检测当前用户全部已知产品的状态。
pub fn detect_all(state: &WorkspaceState) -> Vec<ProductInfo> {
    list_known_products()
        .into_iter()
        .map(|(id, name)| detect_one(id, name, state))
        .collect()
}

pub fn detect_one(id: &str, name: &str, state: &WorkspaceState) -> ProductInfo {
    let env_var = platform::env_var_name(id);
    let workdir = state.workdir();
    let resource_root = state.resource_root();

    // 查找 vmoptions 文件：环境变量 → IDE 自带 → 工作区副本 → 项目自带模板
    let (path, source) = match find_vmoptions(id, &workdir, &resource_root) {
        Some(p) => {
            let is_env = std::env::var(&env_var)
                .map(|v| std::path::Path::new(&v) == p.as_path())
                .unwrap_or(false);
            let is_workspace = p.starts_with(&workdir);
            let is_bundled = p.starts_with(&resource_root);
            let source = if is_env {
                VmoptionsSource::Env
            } else if is_workspace {
                VmoptionsSource::Workspace
            } else if is_bundled {
                VmoptionsSource::Template
            } else {
                // 不是环境变量、工作区、项目自带 —— 说明是 IDE 自带的
                VmoptionsSource::Ide
            };
            (Some(p), source)
        }
        None => (None, VmoptionsSource::Missing),
    };

    let mut vmoptions_preview: Option<String> = None;
    let mut javaagent_installed = false;
    let mut javaagent_target: Option<String> = None;

    if let Some(ref p) = path {
        if let Ok(content) = std::fs::read_to_string(p) {
            vmoptions_preview = Some(content.clone());
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("-javaagent:") && trimmed.contains("ja-netfilter") {
                    javaagent_installed = true;
                    javaagent_target = Some(trimmed.to_string());
                    break;
                }
                if trimmed.starts_with("-javaagent:") && trimmed.ends_with("=jetbrains") {
                    javaagent_installed = true;
                    javaagent_target = Some(trimmed.to_string());
                    break;
                }
            }
        }
    }

    ProductInfo {
        id: id.to_string(),
        name: name.to_string(),
        env_var,
        vmoptions_path: path.map(|p| platform::normalize_path_for_output(&p)),
        vmoptions_source: source,
        javaagent_installed,
        javaagent_target,
        vmoptions_preview,
    }
}

/// 查找 vmoptions 文件：环境变量 → IDE 自带 → 工作区副本 → 项目自带模板。
fn find_vmoptions(
    product_id: &str,
    workdir: &std::path::Path,
    resource_root: &std::path::Path,
) -> Option<PathBuf> {
    let env_key = format!("{}_VM_OPTIONS", product_id.to_uppercase());
    if let Ok(val) = std::env::var(&env_key) {
        let p = PathBuf::from(&val);
        if p.exists() {
            return Some(p);
        }
    }

    // IDE 自带的 vmoptions 文件（最可靠）
    if let Some(path) = crate::installer::find_ide_vmoptions(product_id) {
        if path.exists() {
            return Some(path);
        }
    }

    // 工作区副本（用户修改过的）
    let workspace_copy = workdir
        .join("vmoptions")
        .join(format!("{}.vmoptions", product_id));
    if workspace_copy.exists() {
        return Some(workspace_copy);
    }

    // 项目自带模板（只读）
    let template = resource_root
        .join("vmoptions")
        .join(format!("{}.vmoptions", product_id));
    if template.exists() {
        return Some(template);
    }

    None
}

/// 项目自带 lib.jar 的路径（始终从安装目录加载，不复制到工作区）。
pub fn bundled_jar_path(resource_root: &std::path::Path) -> PathBuf {
    resource_root.join("lib.jar")
}
