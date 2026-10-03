//! vmoptions 文件读写，包含感知 javaagent 的编辑逻辑。
//!
//! 与原版 shell / VBS 安装脚本语义一致：安装时移除旧的
//! `-javaagent:...ja-netfilter.jar...` 行，然后追加一行指向项目自带 jar 的新
//! javaagent 行。若提供了自定义授权名称，则额外追加一行
//! `-Dja.netfilter.name=<value>`。

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::platform::javaagent_line;
use crate::workspace::{resolve_under_root, WorkspaceState};

/// 读取 vmoptions 文件的文本内容。`path_or_id` 可以是绝对路径，也可以是
/// 资源根目录下的相对路径或纯产品 id。
pub fn read_text(state: &WorkspaceState, path_or_id: &str) -> Result<String> {
    let path = resolve_path(state, path_or_id)?;
    fs::read_to_string(&path)
        .with_context(|| format!("读取 vmoptions 失败：{}", path.display()))
}

/// 将文本写回 vmoptions 文件。仅允许写入资源根目录之下，或绝对路径的现存文件。
pub fn write_text(state: &WorkspaceState, path_or_id: &str, content: &str) -> Result<()> {
    let path = resolve_path(state, path_or_id)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(&path, content)
        .with_context(|| format!("写入 vmoptions 失败：{}", path.display()))
}

/// 将 vmoptions 重置为项目自带模板。
pub fn reset_to_template(state: &WorkspaceState, path_or_id: &str) -> Result<()> {
    // 直接清空 javaagent 行 + 自定义授权名称行；下次调用 install 时会重新写入。
    let path = resolve_path(state, path_or_id)?;
    let content = fs::read_to_string(&path).unwrap_or_default();
    let stripped = strip_managed_lines(&content);
    fs::write(&path, stripped)?;
    Ok(())
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

/// 解析 vmoptions 路径。接受：
///   - 绝对路径
///   - 资源根目录下的相对路径（如 `vmoptions/idea.vmoptions`）
///   - 纯产品 id（如 `idea`）—— 展开为 `vmoptions/idea.vmoptions`
fn resolve_path(state: &WorkspaceState, path_or_id: &str) -> Result<PathBuf> {
    let p = Path::new(path_or_id);
    if p.is_absolute() {
        return Ok(p.to_path_buf());
    }
    let expanded = if !path_or_id.contains('/')
        && !path_or_id.contains('\\')
        && !path_or_id.ends_with(".vmoptions")
    {
        format!("vmoptions/{}.vmoptions", path_or_id)
    } else {
        path_or_id.to_string()
    };
    resolve_under_root(state, &expanded)
}
