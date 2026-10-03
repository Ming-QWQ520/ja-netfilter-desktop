//! vmoptions 文件读写，包含感知 javaagent 的编辑逻辑。
//!
//! 读时优先工作区副本，回退项目自带模板。写时 copy-on-write 到工作区。
//! lib.jar 始终使用项目自带路径（不复制到工作区）。

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::platform::javaagent_line;
use crate::workspace::{self, WorkspaceState};

/// 读取 vmoptions 文件：优先工作区副本，回退项目自带模板。
/// `path_or_id` 可以是绝对路径、工作区/项目根目录下的相对路径、或纯产品 id。
pub fn read_text(state: &WorkspaceState, path_or_id: &str) -> Result<String> {
    // 绝对路径直接读
    let p = Path::new(path_or_id);
    if p.is_absolute() {
        return fs::read_to_string(p)
            .with_context(|| format!("读取 vmoptions 失败：{}", p.display()));
    }

    // 展开 id 为相对路径
    let rel = expand_id_to_rel(path_or_id);

    // 通过 workspace::read_resource 读取（优先工作区，回退项目自带）
    workspace::read_resource(state, &rel)
}

/// 将文本写回 vmoptions 文件：copy-on-write 到工作区。
pub fn write_text(state: &WorkspaceState, path_or_id: &str, content: &str) -> Result<()> {
    // 绝对路径直接写
    let p = Path::new(path_or_id);
    if p.is_absolute() {
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).ok();
        }
        return fs::write(p, content)
            .with_context(|| format!("写入 vmoptions 失败：{}", p.display()));
    }

    let rel = expand_id_to_rel(path_or_id);
    workspace::write_resource(state, &rel, content)?;
    Ok(())
}

/// 重置 vmoptions：删除工作区副本，使后续读取回退到项目自带模板。
pub fn reset_to_template(state: &WorkspaceState, path_or_id: &str) -> Result<()> {
    // 绝对路径：若是工作区副本则删除；否则不做处理
    let p = Path::new(path_or_id);
    if p.is_absolute() {
        if p.starts_with(state.workdir()) && p.exists() {
            fs::remove_file(p)?;
        }
        return Ok(());
    }

    let rel = expand_id_to_rel(path_or_id);
    workspace::reset_resource(state, &rel)
}

/// 追加或刷新 vmoptions 中的 `-javaagent:` 行，使其指向项目自带的 `lib.jar`。
/// 若提供了 `license_name`，则同时追加 `-Dja.netfilter.name=<license_name>` 行。
pub fn ensure_javaagent(
    vmoptions_path: &Path,
    jar_path: &Path,
    license_name: Option<&str>,
) -> Result<()> {
    let content = fs::read_to_string(vmoptions_path).unwrap_or_default();
    let line = javaagent_line(jar_path);

    let cleaned = strip_managed_lines(&content);
    let mut out = cleaned.trim_end_matches('\n').to_string();
    if !out.is_empty() {
        out.push('\n');
    }
    out.push_str(&line);
    out.push('\n');
    if let Some(name) = license_name {
        if !name.trim().is_empty() {
            out.push_str(&format!("-Dja.netfilter.name={}", name.trim()));
            out.push('\n');
        }
    }

    fs::write(vmoptions_path, out)?;
    Ok(())
}

/// 从 vmoptions 文件中移除所有指向 ja-netfilter 的 `-javaagent:` 行
/// 以及自定义授权名称行。
pub fn strip_javaagent(vmoptions_path: &Path) -> Result<()> {
    let content = fs::read_to_string(vmoptions_path).unwrap_or_default();
    let cleaned = strip_managed_lines(&content);
    fs::write(vmoptions_path, cleaned)?;
    Ok(())
}

/// 移除以下两类行：
///   - `-javaagent:...ja-netfilter...` 或 `-javaagent:...=jetbrains`
///   - `-Dja.netfilter.name=...`
fn strip_managed_lines(content: &str) -> String {
    content
        .lines()
        .filter(|l| {
            let t = l.trim();
            if t.starts_with("-javaagent:")
                && (t.contains("ja-netfilter") || t.ends_with("=jetbrains"))
            {
                return false;
            }
            if t.starts_with("-Dja.netfilter.name=") {
                return false;
            }
            true
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// 将纯产品 id（如 `idea`）展开为 `vmoptions/idea.vmoptions`。
/// 若已包含路径分隔符或 `.vmoptions` 后缀，则原样返回。
fn expand_id_to_rel(path_or_id: &str) -> String {
    if path_or_id.contains('/')
        || path_or_id.contains('\\')
        || path_or_id.ends_with(".vmoptions")
    {
        path_or_id.to_string()
    } else {
        format!("vmoptions/{}.vmoptions", path_or_id)
    }
}
