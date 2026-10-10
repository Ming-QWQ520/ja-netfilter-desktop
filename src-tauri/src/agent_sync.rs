//! Agent 资源联网同步 —— 对齐 `ckey_script.ps1` 的 `File_Download` 语义。
//!
//! 背景（v0.1.0 现场缺陷）：应用自带的 agent 资源是历史快照（power.conf
//! 标注 `Suit 230914`），而 ckey.run 生成的 `<prd>.key` 必须与**当前版**
//! power.conf 的 RSA 规则配对 —— 旧 power.conf 验不了新 key，这正是
//! 「用 ps1 装好的激活，被本应用重装后失效」的根因。
//!
//! 同步策略（v0.1.1 简化：「匹配一致则不下载」）：
//!   1. 全量同步成功后，在 workdir 写入 `sync-manifest.json`（13 个文件的
//!      SHA-256 + 时间戳）；
//!   2. 后续同步先校验本地清单（文件齐全 + 哈希吻合 + 24h 内），
//!      再用一个极小的 `power.conf` 探测请求确认激活关键文件未变 ——
//!      一致则**零下载**直接返回（ckey.run 不支持 ETag/If-None-Match，
//!      这是无校验头服务端下唯一可靠的「不下载」判定）；
//!   3. 清单缺失/过期/哈希不符/探测到变化 → 走全量下载（内容一致仍跳过
//!      写入，未变化文件不落盘）；网络层不可达时快速终止并沿用本地资源；
//!   4. `force=true`（设置页手动按钮）跳过快速路径，强制核对全部文件。
//!
//! 授权 key 与 power.conf 的配对不变量由此保持：只要 power.conf 变了，
//! 下一次安装前的自动同步必然能感知（探测哈希不符）。

use std::collections::BTreeMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

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

/// 激活关键文件（授权 key 必须与其 RSA 规则配对）——探测用它代表
/// 远端资源版本；它不变，key 配对就不会被破坏。
const PROBE_REMOTE: &str = "config/power.conf";
const PROBE_LOCAL: &str = "config-jetbrains/power.conf";

/// 快速路径的清单保鲜期：超过则视为过期，走全量同步。
const FRESH_SECS: u64 = 24 * 60 * 60;

/// workdir 内的清单文件名。
const MANIFEST_NAME: &str = "sync-manifest.json";

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

/// 本地同步清单（全量成功后写入 workdir）。
#[derive(Debug, Serialize, Deserialize)]
struct SyncManifest {
    /// 结构版本（将来字段变更时用于失效旧清单）。
    version: u32,
    /// 全量成功的 Unix 时间戳（秒）。
    synced_at: u64,
    /// 本地相对路径 -> SHA-256（hex）。
    files: BTreeMap<String, String>,
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    let out = h.finalize();
    let mut s = String::with_capacity(out.len() * 2);
    for b in out {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

fn manifest_path(state: &WorkspaceState) -> std::path::PathBuf {
    state.workdir.join(MANIFEST_NAME)
}

fn load_manifest(state: &WorkspaceState) -> Option<SyncManifest> {
    let bytes = std::fs::read(manifest_path(state)).ok()?;
    match serde_json::from_slice::<SyncManifest>(&bytes) {
        Ok(m) if m.version == 1 => Some(m),
        _ => None,
    }
}

fn save_manifest(state: &WorkspaceState, m: &SyncManifest) {
    let path = manifest_path(state);
    let tmp = path.with_extension("json.tmp");
    match serde_json::to_vec_pretty(m)
        .map_err(anyhow::Error::from)
        .and_then(|b| {
            std::fs::write(&tmp, b)?;
            std::fs::rename(&tmp, &path)
                .map_err(|e| {
                    let _ = std::fs::remove_file(&tmp);
                    e
                })
                .map_err(anyhow::Error::from)
        }) {
        Ok(()) => {}
        Err(e) => logger::append(
            Level::Warn,
            &format!("[agent 同步] 同步清单写入失败（不影响资源）：{e}"),
        ),
    }
}

/// 校验本地文件与清单完全吻合（全部存在 + SHA-256 一致）。
fn local_matches(state: &WorkspaceState, m: &SyncManifest) -> bool {
    FILES.iter().all(|(_, rel)| {
        let expected = match m.files.get(*rel) {
            Some(h) => h,
            None => return false,
        };
        match std::fs::read(state.agent_root.join(rel)) {
            Ok(bytes) => sha256_hex(&bytes) == *expected,
            Err(_) => false,
        }
    })
}

/// 把秒差转成「x 小时前 / x 分钟前」。
fn age_text(secs: u64) -> String {
    if secs < 3600 {
        format!("{} 分钟前", (secs / 60).max(1))
    } else if secs < 86400 {
        format!("{} 小时前", secs / 3600)
    } else {
        format!("{} 天前", secs / 86400)
    }
}

fn http_client() -> Option<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        .user_agent(format!("ja-netfilter-desktop/{}", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(8))
        .timeout(Duration::from_secs(20))
        .build()
        .ok()
}

/// 探测远端 power.conf 是否与清单一致。
/// 返回：Ok(Some(一致)) / Ok(Some(不一致)) / Ok(None)（远端异常，需全量核对）
/// / Err(())（网络不可达）。
fn probe_remote(
    client: &reqwest::blocking::Client,
    m: &SyncManifest,
) -> Result<Option<bool>, ()> {
    let url = format!("{SYNC_BASE}/{PROBE_REMOTE}");
    let resp = client
        .get(&url)
        .send()
        .map_err(|_| ())?;
    if !resp.status().is_success() {
        return Ok(None);
    }
    let bytes = resp.bytes().map_err(|_| ())?;
    if bytes.is_empty() {
        return Ok(None);
    }
    let expected = match m.files.get(PROBE_LOCAL) {
        Some(h) => h,
        None => return Ok(None),
    };
    Ok(Some(sha256_hex(bytes.as_ref()) == *expected))
}

/// 把 agent_root 内的 agent / 配置 / 插件同步到 ckey.run 当前版本。
/// 网络异常与单文件失败均不致命；详细过程写入应用日志。
///
/// `force=false`（安装前自动同步）：本地清单新鲜且完整时仅做一次
/// power.conf 探测，一致则零下载返回；`force=true`（手动）：全量核对。
pub fn sync_latest(state: &WorkspaceState, force: bool) -> SyncSummary {
    // ---- 快速路径：清单新鲜 + 本地完整 + 探测一致 => 零下载 ----
    if !force {
        if let Some(m) = load_manifest(state) {
            let age = now_secs().saturating_sub(m.synced_at);
            if age < FRESH_SECS && local_matches(state, &m) {
                match http_client() {
                    None => {}
                    Some(client) => match probe_remote(&client, &m) {
                        Ok(Some(true)) => {
                            let msg = format!(
                                "agent 资源已是最新（{}同步，power.conf 校验一致），跳过下载",
                                age_text(age)
                            );
                            logger::info(&format!("[agent 同步] {msg}"));
                            return SyncSummary {
                                ok: true,
                                updated: Vec::new(),
                                unchanged: FILES.len(),
                                failed: Vec::new(),
                                message: msg,
                            };
                        }
                        Ok(Some(false)) => {
                            logger::info(
                                "[agent 同步] 探测到远端 power.conf 已更新，开始全量同步……",
                            );
                        }
                        Ok(None) => {
                            logger::warn(
                                "[agent 同步] 远端探测异常，回退全量核对……",
                            );
                        }
                        Err(()) => {
                            let msg = "ckey.run 不可达（离线/代理？），本地资源校验完整，沿用上次同步结果";
                            logger::warn(&format!("[agent 同步] {msg}"));
                            return SyncSummary {
                                ok: true,
                                updated: Vec::new(),
                                unchanged: FILES.len(),
                                failed: Vec::new(),
                                message: msg.to_string(),
                            };
                        }
                    },
                }
            }
        }
    }

    let client = match http_client() {
        Some(c) => c,
        None => {
            let msg = "agent 同步跳过：HTTP 客户端创建失败，沿用自带资源";
            logger::warn(&msg);
            return SyncSummary::skipped(&msg);
        }
    };

    if force {
        logger::info("开始核对 agent 资源（手动强制，逐文件比对 ckey.run 当前版本）……");
    } else {
        logger::info("开始同步 agent 资源（对齐 ckey.run 当前版本）……");
    }

    let mut updated = Vec::new();
    let mut unchanged = 0usize;
    let mut failed = Vec::new();
    let mut hashes: BTreeMap<String, String> = BTreeMap::new();
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

        let digest = sha256_hex(bytes.as_ref());

        // 内容一致 -> 跳过写入
        if let Ok(existing) = std::fs::read(&dest) {
            if existing.as_slice() == bytes.as_ref() {
                unchanged += 1;
                hashes.insert(local_rel.to_string(), digest);
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
                hashes.insert(local_rel.to_string(), digest);
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

    // 全部成功才刷新清单（部分失败时保留旧清单，下次继续核对）
    if failed.is_empty() && !net_down {
        save_manifest(
            state,
            &SyncManifest {
                version: 1,
                synced_at: now_secs(),
                files: hashes,
            },
        );
    }

    let summary = if failed.is_empty() {
        let msg = if updated.is_empty() {
            format!("agent 资源核对完成：{unchanged} 个文件与 ckey.run 最新版一致，无需更新")
        } else {
            format!(
                "agent 资源同步完成：更新 {} 个 / 一致 {unchanged} 个 / 失败 0 个",
                updated.len()
            )
        };
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
            "agent 资源部分同步：更新 {} 个 / 一致 {unchanged} 个 / 失败 {} 个（失败项沿用自带资源，不影响安装继续）",
            updated.len(),
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
