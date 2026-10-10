//! JetBrains 产品定位 —— 与 ckey_script.ps1 / win_core.ps1 同一套算法的
//! Rust 实现（用于状态检测与 macOS/Linux 安装路径；Windows 安装走 PS）。
//!
//! 发现链（对每个产品）：
//!   1. 扫描 `%LOCALAPPDATA%\JetBrains\*`（macOS:
//!      `~/Library/Application Support/JetBrains/*`，Linux:
//!      `~/.local/share/JetBrains/*`）下目录名包含产品 id 的目录
//!      （"studio" 会额外扫描 `%LOCALAPPDATA%\Google\*AndroidStudio*`，
//!      "devecostudio" 扫描 `%LOCALAPPDATA%\Huawei\*`）。
//!   2. 读取目录内 `.home` 文件 → IDE 真实安装路径 → `bin/` 下递归
//!      搜寻 `*.vmoptions`。
//!   3. 用户配置目录（Roaming）：`%APPDATA%\JetBrains\<同名目录>\<prd>64.exe.vmoptions`
//!      等候选。

use std::fs;
use std::path::{Path, PathBuf};

use crate::platform::{self, Os};

/// 单个产品在某配置目录下的发现结果。
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ProductLocation {
    /// 缓存/配置目录名，如 `IntelliJIdea2024.2`。
    pub dir_name: String,
    /// local 侧目录（含 .home 的那个）。
    pub cache_dir: PathBuf,
    /// 用户配置（Roaming）侧目录 —— 授权文件 `<prd>.key` 写在这里。
    pub roaming_dir: Option<PathBuf>,
    /// `.home` 指向的 IDE 安装目录（可能为空）。
    pub install_dir: Option<PathBuf>,
    /// 安装目录 bin/ 下递归找到的 vmoptions。
    pub bin_vmoptions: Vec<PathBuf>,
    /// 用户配置目录下的 vmoptions（IDE 优先读取）。
    pub roaming_vmoptions: Vec<PathBuf>,
}

impl ProductLocation {
    /// 所有可编辑的 vmoptions（roaming 优先）。
    pub fn all_vmoptions(&self) -> Vec<PathBuf> {
        let mut v = self.roaming_vmoptions.clone();
        v.extend(self.bin_vmoptions.iter().cloned());
        v
    }

    /// IDE 实际读取的主 vmoptions（Roaming > bin）。
    pub fn primary_vmoptions(&self) -> Option<PathBuf> {
        self.roaming_vmoptions
            .first()
            .or_else(|| self.bin_vmoptions.first())
            .cloned()
    }
}

/// 目录名 → 是否匹配产品（含 studio/devecostudio 消歧规则）。
fn dir_matches(product_id: &str, dir_name: &str) -> bool {
    let n = dir_name.to_lowercase();
    match product_id {
        // Android Studio 的缓存目录是 AndroidStudio2024.x（Google 目录下）
        "studio" => n.contains("androidstudio") && !n.contains("devecostudio"),
        "devecostudio" => n.contains("devecostudio"),
        "webide" => false, // 老产品，目录名是 WebStorm*，交给 webstorm
        id => {
            let id = id.to_lowercase();
            // 长名优先消歧：jetbrains_client 等不会误吞
            n.contains(&id) && !(id == "studio" && n.contains("devecostudio"))
        }
    }
}

/// 产品的 vmoptions 文件名候选（Roaming 配置目录）。
fn roaming_candidates(product_id: &str, os: Os) -> Vec<String> {
    match os {
        Os::Windows => vec![
            format!("{}64.exe.vmoptions", product_id),
            format!("{}.exe.vmoptions", product_id),
            format!("{}.vmoptions", product_id),
        ],
        _ => vec![format!("{}.vmoptions", product_id)],
    }
}

/// local 侧扫描根 + 对应的 roaming 根。
fn scan_roots(product_id: &str) -> Vec<(PathBuf, Option<PathBuf>)> {
    let _os = Os::current();
    let mut roots: Vec<(PathBuf, Option<PathBuf>)> = Vec::new();

    if let Some(local) = platform::jetbrains_local_root() {
        let roaming = platform::jetbrains_roaming_root();
        roots.push((local, roaming));
    }

    match product_id {
        "studio" => {
            if let Some(local) = dirs::data_local_dir() {
                let g = local.join("Google");
                let roaming = dirs::config_dir().map(|c| c.join("Google"));
                roots.push((g, roaming));
            }
        }
        "devecostudio" => {
            if let Some(local) = dirs::data_local_dir() {
                let h = local.join("Huawei");
                let roaming = dirs::config_dir().map(|c| c.join("Huawei"));
                roots.push((h, roaming));
            }
        }
        _ => {}
    }
    roots
}

/// 发现产品的全部位置。找不到时返回空数组。
pub fn find_product_locations(product_id: &str) -> Vec<ProductLocation> {
    let os = Os::current();
    let _ = os; // 平台差异通过 scan_roots / roaming_candidates 体现
    let mut out: Vec<ProductLocation> = Vec::new();

    for (local_root, roaming_root) in scan_roots(product_id) {
        let entries = match fs::read_dir(&local_root) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let cache_dir = entry.path();
            if !cache_dir.is_dir() {
                continue;
            }
            let dir_name = entry
                .file_name()
                .to_string_lossy()
                .to_string();
            if !dir_matches(product_id, &dir_name) {
                continue;
            }

            let mut loc = ProductLocation {
                dir_name: dir_name.clone(),
                cache_dir: cache_dir.clone(),
                roaming_dir: None,
                install_dir: None,
                bin_vmoptions: Vec::new(),
                roaming_vmoptions: Vec::new(),
            };

            // .home -> 安装目录 -> bin/*.vmoptions
            let home_file = cache_dir.join(".home");
            if home_file.exists() {
                if let Ok(content) = fs::read_to_string(&home_file) {
                    let install = content.lines().next().unwrap_or("").trim().to_string();
                    if !install.is_empty() {
                        let install_dir = PathBuf::from(&install);
                        if install_dir.exists() {
                            let bin = install_dir.join("bin");
                            if bin.is_dir() {
                                loc.bin_vmoptions = find_vmoptions_in(&bin);
                            }
                            loc.install_dir = Some(install_dir);
                        }
                    }
                }
            }

            // Roaming 配置目录候选
            if let Some(ref roam_root) = roaming_root {
                let roam_dir = roam_root.join(&dir_name);
                if roam_dir.is_dir() {
                    for cand in roaming_candidates(product_id, os) {
                        let p = roam_dir.join(&cand);
                        if p.is_file() {
                            loc.roaming_vmoptions.push(p);
                        }
                    }
                    loc.roaming_dir = Some(roam_dir);
                }
            }

            out.push(loc);
        }
    }

    out
}

/// 在 bin 目录递归查找 *.vmoptions（深度限制 2，避免大目录拖慢）。
fn find_vmoptions_in(bin: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    visit(bin, 0, &mut out);
    out.sort();
    out
}

fn visit(dir: &Path, depth: u32, out: &mut Vec<PathBuf>) {
    if depth > 2 {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            visit(&p, depth + 1, out);
        } else if p
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.ends_with(".vmoptions"))
            .unwrap_or(false)
        {
            out.push(p);
        }
    }
}

/// 判断一行是否为受管 javaagent 行（ckey_script.ps1 的
/// `^-javaagent:.*\.jar.*` 等价实现，忽略大小写）。
pub fn is_agent_line(line: &str) -> bool {
    let t = line.trim().to_lowercase();
    t.starts_with("-javaagent:") && t.contains(".jar")
}

/// 判断一行是否为授权名称行。
pub fn is_name_line(line: &str) -> bool {
    let t = line.trim().to_lowercase();
    t.starts_with("-dja.netfilter.name=")
}

/// 文件内容中是否已存在受管 javaagent 行。
#[allow(dead_code)]
pub fn contains_agent_line(content: &str) -> bool {
    content.lines().any(is_agent_line)
}
