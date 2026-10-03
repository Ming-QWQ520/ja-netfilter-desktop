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
    /// 用户默认配置目录中找到。
    User,
    /// 使用项目自带模板（用户目录下尚无文件）。
    Template,
    /// 任何地方都不存在。
    Missing,
}

impl VmoptionsSource {
    #[allow(dead_code)]
    pub fn label(self) -> &'static str {
        match self {
            VmoptionsSource::Env => "环境变量",
            VmoptionsSource::User => "用户配置",
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
    let root = state.get();
    list_known_products()
        .into_iter()
        .map(|(id, name)| detect_one(id, name, &root))
        .collect()
}

pub fn detect_one(id: &str, name: &str, resource_root: &std::path::Path) -> ProductInfo {
    let env_var = platform::env_var_name(id);

    let (path, source) = match platform::find_vmoptions_path(id, resource_root) {
        Some(p) => {
            let is_env = std::env::var(&env_var)
                .map(|v| std::path::Path::new(&v) == p.as_path())
                .unwrap_or(false);
            let is_template = p.starts_with(resource_root);
            let source = if is_env {
                VmoptionsSource::Env
            } else if is_template {
                VmoptionsSource::Template
            } else {
                VmoptionsSource::User
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

/// 项目自带 jar 在 resource_root 下的路径。
pub fn resource_jar_path(resource_root: &std::path::Path) -> PathBuf {
    resource_root.join("lib.jar")
}
