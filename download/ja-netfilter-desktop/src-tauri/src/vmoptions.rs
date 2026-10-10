//! vmoptions 文件读写。
//!
//! v0.1.0 语义（零复制，最小写入）：
//!   - 编辑目标有两类：
//!     1) **真实 IDE vmoptions**（绝对路径，来自 locate::find_product_locations）
//!     2) **应用自带的模板**（产品 id → agent_root/vmoptions/<id>.vmoptions），
//!        仅供无 IDE 时查看/编辑参考，不参与激活。
//!   - `append_lines` 供安装器追加 `-javaagent:` 行（仅此一行，与 ckey_script
//!     一致；授权信息走 `<prd>.key` 文件而非 vmoptions）。
//!   - `strip_managed` 供"清除 agent 配置"动作使用（等价 ckey 的 Revert），
//!     同时清理历史版本遗留的 `-Dja.netfilter.name=` 行。

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::locate;
use crate::workspace::WorkspaceState;

/// 解析 `path_or_id`：
///   - 绝对路径 → 原样
///   - 产品 id（如 `idea`）→ 该产品检测到的主 vmoptions（Roaming > bin），
///     未检测到 IDE 时回退应用自带模板
pub fn resolve_target(state: &WorkspaceState, path_or_id: &str) -> Result<PathBuf> {
    let p = Path::new(path_or_id);
    if p.is_absolute() {
        return Ok(p.to_path_buf());
    }
    let id = path_or_id;
    if id.contains('/') || id.contains('\\') || id.ends_with(".vmoptions") {
        // 视为 agent_root 下的相对路径
        return Ok(state.agent_root.join(id));
    }
    // 产品 id → 检测
    let locations = locate::find_product_locations(id);
    for loc in &locations {
        if let Some(p) = loc.primary_vmoptions() {
            return Ok(p);
        }
    }
    let tpl = state.agent_root.join("vmoptions").join(format!("{}.vmoptions", id));
    if tpl.exists() {
        return Ok(tpl);
    }
    anyhow::bail!("未找到 {} 的 vmoptions 文件（未检测到 IDE 且模板缺失）", id)
}

/// 读取 vmoptions 文本。
pub fn read_text(state: &WorkspaceState, path_or_id: &str) -> Result<String> {
    let p = resolve_target(state, path_or_id)?;
    fs::read_to_string(&p).with_context(|| format!("读取 vmoptions 失败：{}", p.display()))
}

/// 写回 vmoptions 文本（直接作用于目标文件；应用资源目录随 per-user 安装可写）。
pub fn write_text(state: &WorkspaceState, path_or_id: &str, content: &str) -> Result<()> {
    let p = resolve_target(state, path_or_id)?;
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(&p, content).with_context(|| format!("写入 vmoptions 失败：{}", p.display()))
}

/// 清除文件中的受管行（-javaagent / -Dja.netfilter.name）。
pub fn strip_managed(path: &Path) -> Result<()> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("读取 vmoptions 失败：{}", path.display()))?;
    let kept: Vec<&str> = content
        .lines()
        .filter(|l| !locate::is_agent_line(l) && !locate::is_name_line(l))
        .collect();
    let mut out = kept.join("\n");
    if !out.is_empty() {
        out.push('\n');
    }
    fs::write(path, out).with_context(|| format!("写入 vmoptions 失败：{}", path.display()))
}

/// 追加 `-javaagent:` 行（安装器使用；ckey_script 仅写这一行）。
pub fn append_lines(path: &Path, agent_line: &str) -> Result<()> {
    let content = fs::read_to_string(path).unwrap_or_default();
    let mut out = content.trim_end_matches('\n').to_string();
    if !out.is_empty() {
        out.push('\n');
    }
    out.push_str(agent_line);
    out.push('\n');
    fs::write(path, out).with_context(|| format!("写入 vmoptions 失败：{}", path.display()))
}
