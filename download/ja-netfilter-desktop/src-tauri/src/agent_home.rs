//! agent 运行时路径解析。
//!
//! v0.1.0 设计原则：**项目自带的文件不复制**。
//! lib.jar / config-jetbrains/ / plugins-jetbrains/ 随应用安装包分发，
//! `-javaagent` 直接指向应用资源目录内的 lib.jar（就地引用，零复制）：
//!
//!   - Windows（NSIS currentUser）: `%LOCALAPPDATA%\Programs\ja-netfilter-Desktop\resources\`
//!   - macOS: `/Applications/ja-netfilter-Desktop.app/Contents/Resources/resources/`
//!   - Linux: `/usr/lib/ja-netfilter-desktop/resources/`（deb）/ AppImage 挂载点
//!
//! 路径含空白时的处理顺序（见 installer.rs）：
//!   1. Windows 交由 win_core.ps1 的 Get-ShortPath 解析 8.3 短路径；
//!   2. 短路径仍不可用（卷禁用 8.3）时，Rust 侧才把 agent **兜底复制**
//!      到无空格安全目录（唯一会发生复制的情况）。
//!
//! 兜底目录与 ckey_script.ps1 的 `%PUBLIC%\.jb_run\` 同思路：
//!   - Windows: `%LOCALAPPDATA%\ja-netfilter-desktop\agent\`
//!   - macOS:   `~/Library/ja-netfilter-desktop/agent/`（避开含空格的
//!              "Application Support"）
//!   - Linux:   `~/.local/share/ja-netfilter-desktop/agent/`

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::platform;

/// agent 安全目录内的相对布局（目录名与 `-javaagent:...=jetbrains` 参数对应，
/// ja-netfilter 会从 lib.jar 同级的 `config-jetbrains/`、`plugins-jetbrains/`
/// 读取配置与插件）。
#[allow(dead_code)]
pub const AGENT_ITEMS: &[&str] = &["lib.jar", "config-jetbrains", "plugins-jetbrains", "vmoptions"];

/// 计算兜底安全目录（仅在就地路径不可用且 8.3 短路径失败时使用）。
pub fn fallback_home() -> Result<PathBuf> {
    let base = match platform::Os::current() {
        platform::Os::Windows => dirs::data_local_dir()
            .context("无法解析 %LOCALAPPDATA% 目录")?,
        platform::Os::Macos => dirs::home_dir()
            .context("无法解析用户主目录")?
            .join("Library"),
        platform::Os::Linux => dirs::data_dir()
            .or_else(|| dirs::home_dir().map(|h| h.join(".local").join("share")))
            .context("无法解析 XDG data 目录")?,
    };
    Ok(base.join("ja-netfilter-desktop").join("agent"))
}

/// 兜底安全目录是否"干净"（不含空白/非 ASCII——可直接写入 vmoptions）。
pub fn home_is_clean(home: &Path) -> bool {
    platform::is_clean_agent_path(&platform::normalize_path_for_output(&home.join("lib.jar")))
}

/// 极端情况兜底：把 bundle 内的 agent 运行时镜像到无空格安全目录。
/// 正常情况下（per-user 安装 + 8.3 可用）不会走到这里。
/// 返回 `(agent_root, is_clean)`。
pub fn deploy_fallback(resource_root: &Path) -> Result<(PathBuf, bool)> {
    let home = fallback_home()?;
    fs::create_dir_all(&home)
        .with_context(|| format!("创建兜底 agent 目录失败：{}", home.display()))?;

    let src_jar = resource_root.join("lib.jar");
    if src_jar.exists() {
        fs::copy(&src_jar, home.join("lib.jar")).with_context(|| {
            format!(
                "复制 lib.jar 失败：{} -> {}",
                src_jar.display(),
                home.join("lib.jar").display()
            )
        })?;
    }
    for dir in ["config-jetbrains", "plugins-jetbrains", "vmoptions"] {
        let src_dir = resource_root.join(dir);
        if src_dir.is_dir() {
            mirror_dir(&src_dir, &home.join(dir))?;
        }
    }

    let clean = home_is_clean(&home);
    Ok((home, clean))
}

/// 递归镜像目录：源侧存在则覆盖到目标（兜底场景，保持与 bundle 一致）。
fn mirror_dir(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)?.flatten() {
        let sp = entry.path();
        let dp = dst.join(entry.file_name());
        if sp.is_dir() {
            mirror_dir(&sp, &dp)?;
        } else {
            fs::copy(&sp, &dp).with_context(|| {
                format!("复制资源失败：{} -> {}", sp.display(), dp.display())
            })?;
        }
    }
    Ok(())
}
