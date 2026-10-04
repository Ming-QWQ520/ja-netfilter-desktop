// Shared types mirrored from the Rust backend.

export type Os = "windows" | "macos" | "linux";

export type VmoptionsSource = "env" | "ide" | "user" | "template" | "missing";

export interface ProductInfo {
  id: string;
  name: string;
  env_var: string;
  vmoptions_path: string | null;
  vmoptions_source: VmoptionsSource;
  javaagent_installed: boolean;
  javaagent_target: string | null;
  vmoptions_preview: string | null;
}

export interface InstallResult {
  product_id: string;
  success: boolean;
  vmoptions_path: string | null;
  jar_path: string;
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
  jar_path: string;
  jar_exists: boolean;
  plugin_jars: string[];
  vmoptions_dir: string;
  config_dir: string;
  os: Os;
  app_version: string;
}

export type LogLevel = "debug" | "info" | "warn" | "error";

export interface LogEntry {
  ts: string;
  level: LogLevel;
  message: string;
}
