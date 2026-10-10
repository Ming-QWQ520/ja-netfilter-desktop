//! Agent 资源联网同步 —— 对齐 `ckey_script.ps1` 的 `File_Download` 语义。
//!
//! 排查结论（v0.1.0 现场缺陷）：应用自带的 agent 资源是历史快照
//! （power.conf 标注 `Suit 230914`，2023-09 时代），且缺 `env.conf` /
//! `native.conf` / `env.jar` / `native.jar` / `privacy.jar`；而
//! ckey_script.ps1 **每次运行**都从 `https://ckey.run/ja-netfilter/` 下载
//! 最新的 1 个 agent + 5 份配置 + 7 个插件。ckey.run 生成的 `<prd>.key`
//! 必须与**当前版** power.conf 的 RSA 规则配对 —— 旧 power.conf 验不了
//! 新 key，这正是「用 ps1 装好的激活，被本应用重装后失效」的根因。
//!
//! 修复策略（与 ps1 保持同源同版本）：
//!   1. 安装前自动把 `agent_root` 内的资源同步到 ckey.run 当前版本。
//!      就地更新、**零复制语义不变**（-javaagent 行无需任何改动）；
//!   2. 任一文件失败或 ckey.run 不可达时：保留自带资源（回退），
//!      仅告警不中断安装 —— 与授权生成（license.rs）的 best-effort 一致；
//!   3. 另暴露独立命令 `sync_agent_resources` 供设置页手动触发。
//!
//! 同步采用「内容一致即跳过」策略：未变化的文件不重写，日志只报告
//! 真正发生更新的文件，便于回查。

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::logger::{self, Level};
use crate::workspace::WorkspaceState;

/// 与 ckey_script.ps1 的 `$script:url_download` 完全一致的下载根。
const SYNC_BASE: &str = "https://ckey.run/ja-netfilter";

/// 远端相对路径 -> agent_root 内的本地相对路径。
/// 清单与 ckey_script.ps1 的 `File_Download` 一致（1 + 5 + 7 个文件）。
const FILES: &[(&str, &str)] = &[
    ("ja-netfilter.jar", "lib.jar"),
    ("config/dns.conf", "config-jetbrains/dns.conf"),
    ("config/env.conf", "config-jetbrains/env.conf"),
    ("config/native.conf", "config-jetbrains/native.conf"),
    ("config/power.conf", "config-jetbrains/power.conf"),
    ("config/url.conf", "config-jetbrains/url.conf"),
    ("plugins/dns.jar", "plugins-jetbrains/dns.jar"),
    ("plugins/env.jar", "plugins-jetbrains/env.jar"),
    ("plugins/native.jar", "plugins-jetbrains/native.jar"),
    ("plugins/power.jar", "plugins-jetbrains/power.jar"),
    ("plugins/url.jar", "plugins-jetbrains/url.jar"),
    ("plugins/hideme.jar", "plugins-jetbrains/hideme.jar"),
    ("plugins/privacy.jar", "plugins-jetbrains/privacy.jar"),
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncSummary {
    /// true = 同步流程本身顺利（含"不可达时回退自带资源"，不算失败）。
    pub ok: bool,
    /// 发生更新的本地文件（相对 agent_root）。
    pub updated: Vec<String>,
    /// 内容一致而跳过的文件数。
    pub unchanged: usize,
    /// 下载失败的文件（相对 agent_root）。
    pub failed: Vec<String>,
    /// 人类可读的一句话结论（已写入日志）。
    pub message: String,
}

impl SyncSummary {
    fn skipped(msg: &str) -> Self {
        Self {
            ok: true,
            updated: Vec::new(),
            unchanged: 0,
            failed: Vec::new(),
            message: msg.to_string(),
        }
    }
}

/// 把 agent_root 内的 agent / 配置 / 插件同步到 ckey.run 当前版本。
/// 网络异常与单文件失败均不致命；详细过程写入应用日志。
pub fn sync_latest(state: &WorkspaceState) -> SyncSummary {
    let client = match reqwest::blocking::Client::builder()
        .user_agent(format!("ja-netfilter-desktop/{}", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(8))
        .timeout(Duration::from_secs(20))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            let msg = format!("agent 同步跳过：HTTP 客户端创建失败（{e}），沿用自带资源");
            logger::warn(&msg);
            return SyncSummary::skipped(&msg);
        }
    };

    logger::info("开始同步最新 agent 资源（对齐 ckey.run 当前版本，与 ckey_script.ps1 同源）……");

    let mut updated = Vec::new();
    let mut unchanged = 0usize;
    let mut failed = Vec::new();
    let mut net_down = false;

    for (remote, local_rel) in FILES {
        if net_down {
            // 连通性已判定失败：跳过剩余文件，避免逐个等超时
            failed.push(local_rel.to_string());
            continue;
        }
        let url = format!("{SYNC_BASE}/{remote}");
        let dest = state.agent_root.join(local_rel);

        // 下载
        let bytes = match client.get(&url).send() {
            Ok(resp) => {
                let status = resp.status();
                match resp.error_for_status() {
                    Ok(r) => match r.bytes() {
                        Ok(b) => b,
                        Err(e) => {
                            failed.push(local_rel.to_string());
                            logger::append(
                                Level::Warn,
                                &format!("[agent 同步] 读取响应失败 {local_rel}：{e}"),
                            );
                            continue;
                        }
                    },
                    Err(e) => {
                        failed.push(local_rel.to_string());
                        logger::append(
                            Level::Warn,
                            &format!("[agent 同步] {remote} 返回 {status}：{e}"),
                        );
                        continue;
                    }
                }
            }
            Err(e) => {
                failed.push(local_rel.to_string());
                if e.is_timeout() || e.is_connect() {
                    // 连接层失败：判定网络不可达，终止剩余下载（快速回退自带资源）
                    let kind = if e.is_timeout() { "超时" } else { "无法连接（网络/代理/防火墙？）" };
                    logger::append(
                        Level::Warn,
                        &format!("[agent 同步] {remote} {kind}：{e}；判定 ckey.run 不可达，跳过剩余文件（沿用自带资源）"),
                    );
                    net_down = true;
                } else {
                    logger::append(
                        Level::Warn,
                        &format!("[agent 同步] {remote} 请求异常：{e}"),
                    );
                }
                continue;
            }
        };

        if bytes.is_empty() {
            failed.push(local_rel.to_string());
            logger::append(
                Level::Warn,
                &format!("[agent 同步] {remote} 返回空响应，跳过（保留现有文件）"),
            );
            continue;
        }

        // 内容一致 -> 跳过写入
        if let Ok(existing) = std::fs::read(&dest) {
            if existing.as_slice() == bytes.as_ref() {
                unchanged += 1;
                continue;
            }
        }

        // 原子写入：先写临时文件再改名（避免中断留下半截 jar）
        let parent = dest.parent().unwrap_or(&state.agent_root).to_path_buf();
        if let Err(e) = std::fs::create_dir_all(&parent) {
            failed.push(local_rel.to_string());
            logger::append(
                Level::Warn,
                &format!("[agent 同步] 创建目录失败 {}：{e}", parent.display()),
            );
            continue;
        }
        let tmp = dest.with_extension("tmp-dl");
        match std::fs::write(&tmp, &bytes).and_then(|_| std::fs::rename(&tmp, &dest)) {
            Ok(()) => {
                logger::append(
                    Level::Info,
                    &format!(
                        "[agent 同步] 已更新：{local_rel}（{} 字节）",
                        bytes.len()
                    ),
                );
                updated.push(local_rel.to_string());
            }
            Err(e) => {
                let _ = std::fs::remove_file(&tmp);
                failed.push(local_rel.to_string());
                logger::append(
                    Level::Warn,
                    &format!("[agent 同步] 写入失败 {local_rel}：{e}"),
                );
            }
        }
    }

    let summary = if failed.is_empty() {
        let msg = format!(
            "agent 资源同步完成：更新 {} 个 / 一致 {} 个 / 失败 0 个",
            updated.len(),
            unchanged
        );
        logger::success(&msg);
        SyncSummary {
            ok: true,
            updated,
            unchanged,
            failed,
            message: msg,
        }
    } else {
        let msg = format!(
            "agent 资源部分同步：更新 {} 个 / 一致 {} 个 / 失败 {} 个（失败项沿用自带资源，不影响安装继续）",
            updated.len(),
            unchanged,
            failed.len()
        );
        logger::append(Level::Warn, &msg);
        SyncSummary {
            ok: !updated.is_empty() || unchanged > 0,
            updated,
            unchanged,
            failed,
            message: msg,
        }
    };
    summary
}
