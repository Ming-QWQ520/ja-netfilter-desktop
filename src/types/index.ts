// Shared types mirrored from the Rust backend.

export type Os = "windows" | "macos" | "linux";

export type VmoptionsSource = "user" | "ide" | "template" | "missing";

export interface ProductInfo {
  id: string;
  name: string;
  env_var: string;
  /** IDE 实际读取的主 vmoptions（Roaming > bin），未检测到为 null。 */
  vmoptions_path: string | null;
  /** 全部发现的 vmoptions 文件。 */
  vmoptions_paths: string[];
  vmoptions_source: VmoptionsSource;
  javaagent_installed: boolean;
  javaagent_target: string | null;
  vmoptions_preview: string | null;
  /** 是否检测到 IDE（.home / 配置目录存在）。 */
  ide_found: boolean;
}

export interface InstallResult {
  product_id: string;
  success: boolean;
  vmoptions_path: string | null;
  edited_paths: string[];
  jar_path: string;
  message: string;
}

/** 自定义授权文件（<prd>.key）生成结果。 */
export interface LicenseResult {
  product_id: string;
  ok: boolean;
  key_path: string | null;
  message: string;
}

/** agent 资源联网同步结果（对齐 ckey.run 当前版本）。 */
export interface SyncSummary {
  ok: boolean;
  updated: string[];
  unchanged: number;
  failed: string[];
  message: string;
}

export interface ConfigFile {
  name: string;
  relative_path: string;
  size: number;
}

export interface PluginJar {
  name: string;
  path: string;
  size: number;
}

export interface WorkspaceInfo {
  workdir: string;
  bundle_root: string;
  agent_root: string;
  agent_clean: boolean;
  jar_path: string;
  jar_exists: boolean;
  plugin_jars: string[];
  vmoptions_dir: string;
  config_dir: string;
  os: Os;
  app_version: string;
}

export type LogLevel = "debug" | "info" | "success" | "warn" | "error";

export interface LogEntry {
  ts: string;
  level: LogLevel;
  message: string;
}
