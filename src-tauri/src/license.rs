//! 自定义授权 —— 仿 `ckey_script.ps1` 的 `Read_Host_License_Info` / `Create_Key`。
//!
//! ckey_script.ps1 的做法：
//!   1. 提示输入授权名称（回车默认）与到期时间（回车默认 `2099-12-31`，
//!      格式 `yyyy-MM-dd`，`Read-Valid_Date` 校验）；
//!   2. 对每个检测到的产品，POST JSON
//!      `{ assigneeName, expiryDate, licenseName, productCode }`
//!      到 `https://ckey.run/generateLicense/file`；
//!   3. 把响应字节写入 IDE 用户配置目录的 `<prd>.key`
//!      （`%APPDATA%\JetBrains\<Product><ver>\<prd>.key`）。
//!
//! IDE 启动后由 power 插件按 `power.conf` 的规则认可该授权文件，
//! 授权信息即显示为自定义的名称与到期时间。单个产品失败仅告警不中断
//! （与 ckey_script 的 manual_activation_required 语义一致）。

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::Duration;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::locate;
use crate::logger::{self, Level};
use crate::workspace::WorkspaceState;

/// 授权名称默认值（回车默认）。
pub const DEFAULT_LICENSE_NAME: &str = "Ming";
/// 到期时间默认值（回车默认）。
pub const DEFAULT_EXPIRY_DATE: &str = "2099-12-31";

const LICENSE_URL: &str = "https://ckey.run/generateLicense/file";

/// ckey_script.ps1 的产品码表；不在表中的产品跳过授权文件生成。
const PRODUCT_CODES: &[(&str, &str)] = &[
    ("idea", "II,PCWMP,PSI"),
    ("clion", "CL,PSI,PCWMP"),
    ("phpstorm", "PS,PCWMP,PSI"),
    ("goland", "GO,PSI,PCWMP"),
    ("pycharm", "PC,PSI,PCWMP"),
    ("webstorm", "WS,PCWMP,PSI"),
    ("rider", "RD,PDB,PSI,PCWMP"),
    ("datagrip", "DB,PSI,PDB"),
    ("rubymine", "RM,PCWMP,PSI"),
    ("appcode", "AC,PCWMP,PSI"),
    ("dataspell", "DS,PSI,PDB,PCWMP"),
    ("rustrover", "RR,PSI,PCWP"),
];

fn product_code(id: &str) -> Option<&'static str> {
    PRODUCT_CODES
        .iter()
        .find(|(pid, _)| *pid == id)
        .map(|(_, code)| *code)
}

/// 到期时间格式校验（等价 Read-Valid_Date 的 yyyy-MM-dd 严格校验）。
pub fn is_valid_expiry(s: &str) -> bool {
    NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").is_ok()
}

/// 归一化授权名称：空白 → 默认值。
pub fn normalize_license_name(name: Option<&str>) -> String {
    match name.map(str::trim) {
        Some(n) if !n.is_empty() => n.to_string(),
        _ => DEFAULT_LICENSE_NAME.to_string(),
    }
}

/// 归一化到期时间：空白/非法 → 默认值。
pub fn normalize_expiry(expiry: Option<&str>) -> String {
    match expiry.map(str::trim) {
        Some(e) if !e.is_empty() && is_valid_expiry(e) => e.to_string(),
        _ => DEFAULT_EXPIRY_DATE.to_string(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseResult {
    pub product_id: String,
    pub ok: bool,
    pub key_path: Option<String>,
    pub message: String,
}

/// 为产品列表生成授权文件（`<prd>.key`）。返回逐产品结果。
pub fn generate_keys(
    state: &WorkspaceState,
    product_ids: &[String],
    license_name: Option<&str>,
    expiry: Option<&str>,
) -> Vec<LicenseResult> {
    let name = normalize_license_name(license_name);
    let date = normalize_expiry(expiry);
    let _ = &state.workdir; // workdir 仅用于日志上下文

    logger::append(
        Level::Info,
        &format!("开始生成授权文件：名称「{}」，到期「{}」", name, date),
    );

    let client = match reqwest::blocking::Client::builder()
        .user_agent(format!("ja-netfilter-desktop/{}", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(30))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            let msg = format!("HTTP 客户端创建失败：{}", e);
            logger::append(Level::Error, &msg);
            return product_ids
                .iter()
                .map(|id| LicenseResult {
                    product_id: id.clone(),
                    ok: false,
                    key_path: None,
                    message: msg.clone(),
                })
                .collect();
        }
    };

    let mut out = Vec::new();
    for id in product_ids {
        out.push(generate_one(&client, state, id, &name, &date));
    }
    out
}

fn generate_one(
    client: &reqwest::blocking::Client,
    state: &WorkspaceState,
    product_id: &str,
    license_name: &str,
    expiry: &str,
) -> LicenseResult {
    let _ = state;
    let skip = |msg: String| {
        // 跳过/失败原因必须落到日志（否则前端 Snackbar 消失后就无法回查）
        logger::warn(&msg);
        LicenseResult {
            product_id: product_id.to_string(),
            ok: false,
            key_path: None,
            message: msg,
        }
    };

    // 1. 产品码（ckey_script.ps1 的 $product.product_code）
    let Some(code) = product_code(product_id) else {
        return skip(format!(
            "[{}] 无已知产品码，跳过授权文件生成",
            product_id
        ));
    };

    // 2. 目标配置目录（ckey：Roaming\JetBrains\<dir>，即 roaming 侧）
    let mut config_dirs: BTreeSet<PathBuf> = BTreeSet::new();
    for loc in locate::find_product_locations(product_id) {
        if let Some(d) = loc.roaming_dir {
            config_dirs.insert(d);
        }
    }
    if config_dirs.is_empty() {
        return skip(format!(
            "[{}] 未检测到用户配置目录，无法写入授权文件（需先启动一次该 IDE）",
            product_id
        ));
    }
    for dir in &config_dirs {
        logger::debug(&format!(
            "[{}] 授权文件目标目录：{}",
            product_id,
            dir.display()
        ));
    }

    // 3. POST 生成授权（Create_Key 的 JSON 契约）
    let body = serde_json::json!({
        "assigneeName": "",
        "expiryDate": expiry,
        "licenseName": license_name,
        "productCode": code,
    });
    logger::debug(&format!(
        "[{}] 请求授权服务：POST {}（productCode={}，name={}，expiry={}）",
        product_id, LICENSE_URL, code, license_name, expiry
    ));
    let resp = client.post(LICENSE_URL).json(&body).send();
    let bytes = match resp {
        Ok(r) => {
            let status = r.status();
            logger::debug(&format!(
                "[{}] 授权服务响应：HTTP {}",
                product_id, status
            ));
            match r.error_for_status() {
                Ok(r) => match r.bytes() {
                    Ok(b) => {
                        logger::debug(&format!(
                            "[{}] 授权响应 {} 字节",
                            product_id,
                            b.len()
                        ));
                        if b.is_empty() {
                            return skip(format!(
                                "[{}] 授权服务返回空响应（服务端异常？请稍后重试）",
                                product_id
                            ));
                        }
                        b
                    }
                    Err(e) => {
                        return skip(format!(
                            "[{}] 读取授权响应失败：{}",
                            product_id, e
                        ))
                    }
                },
                Err(e) => {
                    return skip(format!(
                        "[{}] 授权服务返回错误：{}（含状态码与 URL，请检查网络或稍后重试）",
                        product_id, e
                    ))
                }
            }
        }
        Err(e) => {
            let kind = if e.is_timeout() {
                "请求超时（30s）"
            } else if e.is_connect() {
                "无法连接（网络/代理/防火墙？）"
            } else {
                "请求异常"
            };
            return skip(format!(
                "[{}] 请求授权服务失败——{}：{}",
                product_id, kind, e
            ));
        }
    };

    // 4. 写入 <config_dir>/<prd>.key
    let mut written = Vec::new();
    for dir in &config_dirs {
        let key_path = dir.join(format!("{}.key", product_id));
        match std::fs::write(&key_path, &bytes) {
            Ok(()) => {
                written.push(key_path.clone());
                logger::debug(&format!(
                    "[{}] 已写入 {}（{} 字节）",
                    product_id,
                    key_path.display(),
                    bytes.len()
                ));
            }
            Err(e) => logger::append(
                Level::Warn,
                &format!(
                    "[{}] 写入授权文件失败 {}: {}",
                    product_id,
                    key_path.display(),
                    e
                ),
            ),
        }
    }
    if written.is_empty() {
        return skip(format!("{}：授权文件写入失败（目录不可写？）", product_id));
    }

    let display: Vec<String> = written
        .iter()
        .map(|p| crate::platform::normalize_path_for_output(p))
        .collect();
    logger::success(&format!(
        "[{}] 授权文件已生成：{}（{}，到期 {}）",
        product_id,
        display.join(", "),
        license_name,
        expiry
    ));
    LicenseResult {
        product_id: product_id.to_string(),
        ok: true,
        key_path: display.first().cloned(),
        message: String::new(),
    }
}

/// 卸载时移除 `<prd>.key` 授权文件，恢复干净状态。返回删除数量。
pub fn remove_keys(product_ids: &[String]) -> usize {
    let mut removed = 0usize;
    for id in product_ids {
        for loc in locate::find_product_locations(id) {
            if let Some(dir) = loc.roaming_dir {
                let key_path = dir.join(format!("{}.key", id));
                if key_path.is_file() {
                    match std::fs::remove_file(&key_path) {
                        Ok(()) => {
                            removed += 1;
                            logger::append(
                                Level::Info,
                                &format!("已移除授权文件：{}", key_path.display()),
                            );
                        }
                        Err(e) => logger::append(
                            Level::Warn,
                            &format!("移除授权文件失败 {}: {}", key_path.display(), e),
                        ),
                    }
                }
            }
        }
    }
    removed
}
