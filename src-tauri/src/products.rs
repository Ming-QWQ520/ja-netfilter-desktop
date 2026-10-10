//! JetBrains 产品定义与运行时检测。
//!
//! v0.2.0 修复：旧版 `detect_one` 走的是一条从未生效的路径链
//! （`JetBrains/<id>/idea.vmoptions` 这种不存在的目录），导致界面显示的
//! 安装状态与真实 IDE vmoptions 脱节。现在检测走与安装器完全相同的
//! `.home` 发现链（locate::find_product_locations）：
//!   Roaming 配置目录 vmoptions > IDE 安装目录 bin/*.vmoptions。

use serde::{Deserialize, Serialize};

use crate::locate;
use crate::platform;
use crate::workspace::WorkspaceState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductInfo {
    pub id: String,
    pub name: String,
    pub env_var: String,
    /// IDE 实际读取的主 vmoptions（Roaming > bin），未检测到为 null。
    pub vmoptions_path: Option<String>,
    /// 全部发现的 vmoptions 文件。
    pub vmoptions_paths: Vec<String>,
    pub vmoptions_source: VmoptionsSource,
    pub javaagent_installed: bool,
    pub javaagent_target: Option<String>,
    pub vmoptions_preview: Option<String>,
    /// 是否检测到 IDE（有 .home / 配置目录）。
    pub ide_found: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VmoptionsSource {
    /// 用户配置目录（Roaming，IDE 优先读取）。
    User,
    /// IDE 安装目录 bin 下的 vmoptions。
    Ide,
    /// agent 安全目录中的模板。
    Template,
    /// 任何地方都不存在。
    Missing,
}

impl VmoptionsSource {
    #[allow(dead_code)]
    pub fn label(self) -> &'static str {
        match self {
            VmoptionsSource::User => "用户配置",
            VmoptionsSource::Ide => "IDE 安装目录",
            VmoptionsSource::Template => "内置模板",
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
    let locations = locate::find_product_locations(id);

    // 汇总所有 vmoptions（Roaming 优先）
    let mut ordered: Vec<std::path::PathBuf> = Vec::new();
    let mut ide_found = !locations.is_empty();
    for loc in &locations {
        for p in loc.roaming_vmoptions.iter().chain(loc.bin_vmoptions.iter()) {
            if !ordered.contains(p) {
                ordered.push(p.clone());
            }
        }
    }

    // 确定来源
    let primary = ordered.first().cloned();
    let source = if let Some(ref p) = primary {
        let in_roaming = locations
            .iter()
            .any(|l| l.roaming_vmoptions.contains(p));
        if in_roaming {
            VmoptionsSource::User
        } else {
            VmoptionsSource::Ide
        }
    } else {
        // 未检测到 IDE：回退 agent 安全目录里的模板（供编辑参考）
        ide_found = false;
        let tpl = state.agent_root.join("vmoptions").join(format!("{}.vmoptions", id));
        if tpl.exists() {
            ordered.push(tpl);
            VmoptionsSource::Template
        } else {
            VmoptionsSource::Missing
        }
    };

    // 状态读取：任何一个文件含 agent 行即视为已安装；
    // 预览优先展示 Roaming/主文件内容。
    let mut javaagent_installed = false;
    let mut javaagent_target: Option<String> = None;
    let mut vmoptions_preview: Option<String> = None;

    for p in &ordered {
        let Ok(content) = std::fs::read_to_string(p) else {
            continue;
        };
        if vmoptions_preview.is_none() {
            vmoptions_preview = Some(content.clone());
        }
        for line in content.lines() {
            let t = line.trim();
            if t.starts_with("-javaagent:") && (t.contains("ja-netfilter") || t.ends_with("=jetbrains")) {
                javaagent_installed = true;
                javaagent_target = Some(t.to_string());
                break;
            }
        }
        if javaagent_installed {
            break;
        }
    }

    let paths_display: Vec<String> = ordered
        .iter()
        .map(|p| platform::normalize_path_for_output(p))
        .collect();

    ProductInfo {
        id: id.to_string(),
        name: name.to_string(),
        env_var,
        vmoptions_path: paths_display.first().cloned(),
        vmoptions_paths: paths_display,
        vmoptions_source: source,
        javaagent_installed,
        javaagent_target,
        vmoptions_preview,
        ide_found,
    }
}
