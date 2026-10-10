//! 跨平台安装 / 卸载。
//!
//! v0.1.0 按 ckey_script.ps1 的语义实现，且 **零复制**：
//!   1. agent 就地引用：`-javaagent` 指向应用自带资源目录内的 lib.jar
//!      （项目自带的文件不复制到 C 盘或其它位置）。
//!   2. 安装前剥离目标 vmoptions 中**全部**旧 `-javaagent:` 行
//!      （ckey 的 Revert_Vm_Options，regex `^-javaagent:.*\.jar.*`），
//!      再追加新行 —— 从根上治愈 "processing of -javaagent failed"。
//!      同时剥离历史版本遗留的 `-Dja.netfilter.name=` 行（不再写入）。
//!   3. **删除**（而非设置）`<PRODUCT>_VM_OPTIONS` 环境变量（User + Machine
//!      两个作用域），并在 Windows 上广播 WM_SETTINGCHANGE。
//!   4. 目标文件 = IDE 真实读取的位置：Roaming 配置目录 vmoptions +
//!      `.home` 指向的安装目录 `bin/*.vmoptions`（递归）。
//!   5. 路径含空格时：Windows 由 win_core.ps1 解析 8.3 短路径；若仍不安全
//!      （卷禁用 8.3），PS 上报 `unsafe-agent-path`，本模块才把 agent 兜底
//!      复制到无空格安全目录并重试一次 —— 这是唯一会发生复制的情况。
//!   6. 卸载时一并移除自定义授权生成的 `<prd>.key` 文件。
//!
//! Windows 上核心逻辑由内嵌的 `win_core.ps1` 完成（以 JSON 契约通信）；
//! macOS / Linux 由本模块以相同语义用 Rust 原生实现。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::license;
use crate::locate;
use crate::logger::{self, Level};
use crate::platform::{self, Os};
use crate::vmoptions;
use crate::workspace::WorkspaceState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallResult {
    pub product_id: String,
    pub success: bool,
    /// 第一个编辑过的 vmoptions 文件（兼容旧字段）。
    pub vmoptions_path: Option<String>,
    /// 全部编辑/清理过的文件。
    pub edited_paths: Vec<String>,
    pub jar_path: String,
    pub message: String,
}

impl InstallResult {
    fn skip(product_id: &str, jar: &Path, message: String) -> Self {
        Self {
            product_id: product_id.to_string(),
            success: false,
            vmoptions_path: None,
            edited_paths: Vec::new(),
            jar_path: platform::normalize_path_for_output(jar),
            message,
        }
    }
}

fn result_from(product_id: &str, ok: bool, edited: Vec<PathBuf>, jar: &Path, message: &str) -> InstallResult {
    let disp: Vec<String> = edited
        .iter()
        .map(|p| platform::normalize_path_for_output(p))
        .collect();
    InstallResult {
        product_id: product_id.to_string(),
        success: ok,
        vmoptions_path: disp.first().cloned(),
        edited_paths: disp,
        jar_path: platform::normalize_path_for_output(jar),
        message: message.to_string(),
    }
}

// ---------------------------------------------------------------------------
// PowerShell 核心（Windows）
// ---------------------------------------------------------------------------

const WIN_CORE_PS1: &str = include_str!("scripts/win_core.ps1");

#[derive(Debug, Deserialize)]
struct PsLogEntry {
    level: String,
    message: String,
}

#[derive(Debug, Deserialize)]
struct PsProductResult {
    product: String,
    ok: bool,
    #[serde(default)]
    edited: Vec<String>,
    #[serde(default)]
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PsCoreOutput {
    ok: bool,
    #[serde(default)]
    results: Vec<PsProductResult>,
    #[serde(default)]
    logs: Vec<PsLogEntry>,
}

/// 把内嵌脚本写到临时文件（UTF-8 BOM 前缀，PS 5.1 才能正确解析 Unicode）。
fn materialize_ps_script() -> Result<PathBuf> {
    let dir = std::env::temp_dir().join("ja-netfilter-desktop");
    fs::create_dir_all(&dir)?;
    let path = dir.join("win_core.ps1");
    fs::write(&path, format!("\u{feff}{}", WIN_CORE_PS1))?;
    Ok(path)
}

/// 运行 PowerShell 核心，解析 JSON 输出，并把日志灌入应用日志。
fn run_ps_core(mode: &str, products: &[&str], agent_jar: &Path) -> Result<PsCoreOutput> {
    let script = materialize_ps_script()?;
    logger::debug(&format!(
        "PowerShell 启动：mode={}，products=[{}]，agent={}，script={}",
        mode,
        products.join(","),
        platform::normalize_path_for_output(agent_jar),
        platform::normalize_path_for_output(&script),
    ));
    let output = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(&script)
        .arg("-Mode")
        .arg(mode)
        .arg("-Products")
        .arg(products.join(","))
        .arg("-AgentJar")
        .arg(agent_jar)
        .output()
        .context("无法启动 powershell.exe")?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    logger::debug(&format!(
        "PowerShell 退出：code={}，stdout {} 字节",
        output.status.code().unwrap_or(-1),
        stdout.len()
    ));
    // stderr 在成功时也可能是诊断输出（PS 的 Write-Error / 告警），必须完整透传
    if !stderr.trim().is_empty() {
        logger::warn(&format!(
            "PowerShell stderr：{}",
            truncate_str(stderr.trim(), 2000)
        ));
    }

    // stdout 必须是纯 JSON；PS 把诊断信息写到 stderr
    let parsed: PsCoreOutput = serde_json::from_str(stdout.trim()).map_err(|e| {
        logger::error(&format!(
            "PowerShell 输出解析失败：{}；stdout 尾部：{}",
            e,
            truncate_str(stdout.trim(), 1000)
        ));
        anyhow::anyhow!(
            "PowerShell 输出解析失败：{}；stderr: {}",
            e,
            stderr.trim()
        )
    })?;

    for log in &parsed.logs {
        let level = match log.level.as_str() {
            "error" => Level::Error,
            "warn" => Level::Warn,
            "success" => Level::Success,
            "debug" => Level::Debug,
            _ => Level::Info,
        };
        logger::append(level, &log.message);
    }

    Ok(parsed)
}

/// 截断过长文本（日志友好）。
fn truncate_str(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let cut: String = s.chars().take(max).collect();
    format!("{}…（已截断）", cut)
}

/// PS 输出是否报告了"agent 路径不安全（8.3 短路径也救不了）"。
fn ps_reports_unsafe_path(out: &PsCoreOutput) -> bool {
    out.logs
        .iter()
        .any(|l| l.message.contains("unsafe-agent-path"))
}

/// 从 PS 日志中提取最后一条 error 级消息（作为整体失败的可读原因）。
fn ps_failure_reason(out: &PsCoreOutput) -> String {
    out.logs
        .iter()
        .rev()
        .find(|l| l.level == "error" || l.level == "warn")
        .map(|l| l.message.clone())
        .unwrap_or_else(|| "未知原因（无 error 日志）".to_string())
}

/// Windows 安装：先用就地资源路径跑 PS；若路径不安全（8.3 不可用），
/// 兕底复制到无空格目录后重试一次。
fn windows_install(refs: &[&str], state: &WorkspaceState) -> Vec<InstallResult> {
    let jar = state.agent_jar.clone();

    let run = |jar: &Path, unsafe_flag: &mut bool, reason: &mut Option<String>| -> Option<Vec<InstallResult>> {
        match run_ps_core("install", refs, jar) {
            Ok(out) if out.ok && !out.results.is_empty() => Some(
                out.results
                    .into_iter()
                    .map(|r| InstallResult {
                        product_id: r.product,
                        success: r.ok,
                        vmoptions_path: r.edited.first().cloned(),
                        edited_paths: r.edited,
                        jar_path: platform::normalize_path_for_output(jar),
                        message: r.message.unwrap_or_default(),
                    })
                    .collect::<Vec<_>>(),
            ),
            Ok(out) if !out.results.is_empty() => {
                // 整体 ok=false 但已有逐产品结果（部分成功）：保留结果而非全部报失败
                *unsafe_flag = ps_reports_unsafe_path(&out);
                Some(
                    out.results
                        .into_iter()
                        .map(|r| InstallResult {
                            product_id: r.product,
                            success: r.ok,
                            vmoptions_path: r.edited.first().cloned(),
                            edited_paths: r.edited,
                            jar_path: platform::normalize_path_for_output(jar),
                            message: r.message.unwrap_or_default(),
                        })
                        .collect::<Vec<_>>(),
                )
            }
            Ok(out) => {
                // 无逐产品结果的整体失败（agent 未找到 / unsafe-agent-path / 未知模式）
                *unsafe_flag = ps_reports_unsafe_path(&out);
                let detail = ps_failure_reason(&out);
                logger::error(&format!("Windows 安装核心失败：{}", detail));
                *reason = Some(detail);
                None
            }
            Err(e) => {
                logger::error(&format!("PowerShell 核心执行失败：{:#}", e));
                *reason = Some(e.to_string());
                None
            }
        }
    };

    logger::info(&format!(
        "开始安装 {} 个产品（Windows/PowerShell 核心）",
        refs.len()
    ));
    logger::info(&format!(
        "javaagent 就地引用：{}",
        platform::normalize_path_for_output(&jar)
    ));

    let mut unsafe_path_failure = false;
    let mut failure_reason: Option<String> = None;
    if let Some(results) = run(&jar, &mut unsafe_path_failure, &mut failure_reason) {
        log_install_summary(&results);
        return results;
    }

    // 兜底：仅在 PS 上报 unsafe-agent-path（就地路径无法变成无空格路径）时发生。
    if unsafe_path_failure && ps_failed_unsafe(&jar) {
        logger::warn(
            "就地路径无法写入 vmoptions（8.3 短路径不可用），兜底复制 agent 到安全目录（唯一复制场景）",
        );
        match crate::agent_home::deploy_fallback(&state.bundle_root) {
            Ok((safe_root, clean)) => {
                logger::info(&format!(
                    "兜底部署完成：{}（clean={})",
                    platform::normalize_path_for_output(&safe_root),
                    clean
                ));
                let safe_jar = safe_root.join("lib.jar");
                if clean {
                    let mut retry_unsafe = false;
                    let mut retry_reason = None;
                    if let Some(results) = run(&safe_jar, &mut retry_unsafe, &mut retry_reason) {
                        log_install_summary(&results);
                        return results;
                    }
                }
                logger::error("兜底目录路径仍不安全，安装失败");
            }
            Err(e) => logger::error(&format!("兜底部署失败：{:#}", e)),
        }
    }

    let reason = failure_reason.unwrap_or_else(|| "PowerShell 执行失败".to_string());
    refs.iter()
        .map(|id| InstallResult::skip(id, &jar, reason.clone()))
        .collect()
}

/// 安装摘要（成功/跳过计数 + 产品明细）。
fn log_install_summary(results: &[InstallResult]) {
    let ok = results.iter().filter(|r| r.success).count();
    logger::info(&format!(
        "安装结束：{} 成功 / {} 未检测到或失败",
        ok,
        results.len() - ok
    ));
    for r in results.iter().filter(|r| !r.success) {
        logger::warn(&format!(
            "[{}] 未安装：{}",
            r.product_id,
            if r.message.is_empty() { "未检测到 IDE 的 vmoptions" } else { &r.message }
        ));
    }
}

/// 判定上一次 PS 运行是否因 unsafe-agent-path 失败（查看日志缓冲不可靠，
/// 这里直接检查路径是否不干净 + PS 失败由调用方路径控制）。
fn ps_failed_unsafe(jar: &Path) -> bool {
    // 路径本身干净则不可能是不安全路径问题，避免无谓兜底复制
    !platform::is_clean_agent_path(&platform::normalize_path_for_output(jar))
}

// ---------------------------------------------------------------------------
// macOS / Linux 原生实现（与 PS 同语义）
// ---------------------------------------------------------------------------

/// 剥离文件中全部受管行（-javaagent:*jar* / -Dja.netfilter.name=*），
/// 返回移除的行数。
fn strip_managed_lines(path: &Path) -> Result<usize> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("读取 vmoptions 失败：{}", path.display()))?;
    let kept: Vec<&str> = content
        .lines()
        .filter(|l| !locate::is_agent_line(l) && !locate::is_name_line(l))
        .collect();
    let removed = content.lines().count() - kept.len();
    if removed > 0 {
        let mut out = kept.join("\n");
        if !out.is_empty() {
            out.push('\n');
        }
        fs::write(path, out)
            .with_context(|| format!("写入 vmoptions 失败：{}", path.display()))?;
        logger::append(
            Level::Info,
            &format!("已清理 {} 行旧 agent 配置：{}", removed, path.display()),
        );
    }
    Ok(removed)
}

/// 原生安装（Unix）：环境变量清理 + 定位 + 剥离 + 追加。
fn native_install(product_id: &str, agent_line: &str) -> (bool, Vec<PathBuf>) {
    // 1. 删除旧环境变量（当前进程 + launchctl + shell rc）
    let env_var = platform::env_var_name(product_id);
    std::env::remove_var(&env_var);
    if Os::current() == Os::Macos {
        let _ = Command::new("launchctl").args(["unsetenv", &env_var]).status();
    }
    remove_from_shell_rc(&env_var);

    // 2. 定位
    let locations = locate::find_product_locations(product_id);
    let mut edited = Vec::new();

    for loc in &locations {
        for vm in loc.all_vmoptions() {
            if !vm.exists() {
                continue;
            }
            if let Err(e) = strip_managed_lines(&vm) {
                logger::append(Level::Error, &format!("{}：{}", vm.display(), e));
                continue;
            }
            match vmoptions::append_lines(&vm, agent_line) {
                Ok(()) => edited.push(vm),
                Err(e) => logger::append(Level::Error, &format!("{}：{}", vm.display(), e)),
            }
        }
    }

    (!edited.is_empty(), edited)
}

/// 原生卸载（Unix）。
fn native_uninstall(product_id: &str) -> Vec<PathBuf> {
    let env_var = platform::env_var_name(product_id);
    std::env::remove_var(&env_var);
    if Os::current() == Os::Macos {
        let _ = Command::new("launchctl").args(["unsetenv", &env_var]).status();
    }
    remove_from_shell_rc(&env_var);

    let mut cleaned = Vec::new();
    for loc in locate::find_product_locations(product_id) {
        for vm in loc.all_vmoptions() {
            if !vm.exists() {
                continue;
            }
            if let Ok(n) = strip_managed_lines(&vm) {
                if n > 0 {
                    cleaned.push(vm);
                }
            }
        }
    }
    cleaned
}

/// 从 ~/.profile、~/.bashrc、~/.zshrc 中移除 `<PRODUCT>_VM_OPTIONS=` 行。
fn remove_from_shell_rc(env_var: &str) {
    let marker = format!("{}_VM_OPTIONS=", env_var);
    for name in [".profile", ".bashrc", ".zshrc"] {
        let Some(home) = dirs::home_dir() else {
            continue;
        };
        let rc = home.join(name);
        let Ok(existing) = fs::read_to_string(&rc) else {
            continue;
        };
        let filtered: String = existing
            .lines()
            .filter(|l| !l.contains(&marker))
            .collect::<Vec<_>>()
            .join("\n");
        if filtered != existing {
            let _ = fs::write(&rc, filtered + "\n");
        }
    }
}

// ---------------------------------------------------------------------------
// 对外批量接口（命令层调用）
// ---------------------------------------------------------------------------

/// 批量安装（单产品或全部产品一次调用）。
/// Windows 上是一次 PowerShell 进程；Unix 上是原生循环。
/// 授权文件（`<prd>.key`）由命令层在安装成功后另行生成（license::generate_keys）。
pub fn install_batch(state: &WorkspaceState, product_ids: &[String]) -> Vec<InstallResult> {
    if product_ids.is_empty() {
        return Vec::new();
    }
    let refs: Vec<&str> = product_ids.iter().map(|s| s.as_str()).collect();

    // 安装前同步最新 agent 资源（对齐 ckey_script.ps1 的每次下载语义）。
    // 老打包的 power.conf 验不了 ckey.run 新生成的授权 key（v0.1.0 现场缺陷），
    // 必须先刷新到当前版本；失败仅告警，安装继续使用自带资源。
    let _ = crate::agent_sync::sync_latest(state);

    match Os::current() {
        Os::Windows => windows_install(&refs, state),
        _ => {
            let jar = state.agent_jar.clone();
            let agent_line = platform::javaagent_line(&jar);

            logger::append(
                Level::Info,
                &format!(
                    "javaagent 就地引用：{}",
                    platform::normalize_path_for_output(&jar)
                ),
            );

            let mut results = Vec::new();
            for id in &refs {
                let (ok, edited) = native_install(id, &agent_line);
                if ok {
                    logger::append(
                        Level::Success,
                        &format!(
                            "[{}] 已安装 -> {}",
                            id,
                            edited
                                .iter()
                                .map(|p| platform::normalize_path_for_output(p))
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                    );
                } else {
                    logger::append(
                        Level::Warn,
                        &format!("[{}] 未找到 IDE 的 vmoptions，已跳过", id),
                    );
                }
                results.push(result_from(id, ok, edited, &jar, ""));
            }
            results
        }
    }
}

/// 批量卸载（同时移除授权文件）。
pub fn uninstall_batch(state: &WorkspaceState, product_ids: &[String]) -> Vec<InstallResult> {
    if product_ids.is_empty() {
        return Vec::new();
    }
    let refs: Vec<&str> = product_ids.iter().map(|s| s.as_str()).collect();
    let jar = state.agent_jar.clone();

    let results = match Os::current() {
        Os::Windows => match run_ps_core("uninstall", &refs, &jar) {
            Ok(out) if out.ok => out
                .results
                .into_iter()
                .map(|r| InstallResult {
                    product_id: r.product,
                    success: true,
                    vmoptions_path: r.edited.first().cloned(),
                    edited_paths: r.edited,
                    jar_path: platform::normalize_path_for_output(&jar),
                    message: r.message.unwrap_or_default(),
                })
                .collect(),
            _ => refs
                .iter()
                .map(|id| InstallResult::skip(id, &jar, "PowerShell 执行失败".into()))
                .collect(),
        },
        _ => {
            let mut results = Vec::new();
            for id in &refs {
                let cleaned = native_uninstall(id);
                logger::append(
                    Level::Success,
                    &format!("[{}] 已移除 javaagent（{} 处）", id, cleaned.len()),
                );
                results.push(result_from(id, true, cleaned, &jar, ""));
            }
            results
        }
    };

    // 移除自定义授权生成的 <prd>.key（恢复干净状态）
    license::remove_keys(product_ids);

    results
}

/// 仅清理环境变量。
pub fn cleanup_env(state: &WorkspaceState, product_ids: &[String]) {
    let refs: Vec<&str> = product_ids.iter().map(|s| s.as_str()).collect();
    let jar = state.agent_jar.clone();
    match Os::current() {
        Os::Windows => {
            let _ = run_ps_core("cleanup-env", &refs, &jar);
        }
        _ => {
            for id in refs {
                let env_var = platform::env_var_name(id);
                std::env::remove_var(&env_var);
                if Os::current() == Os::Macos {
                    let _ = Command::new("launchctl")
                        .args(["unsetenv", &env_var])
                        .status();
                }
                remove_from_shell_rc(&env_var);
            }
        }
    }
}
