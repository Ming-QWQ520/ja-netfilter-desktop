// Thin wrappers around the `invoke` calls that talk to the Rust backend.

import { invoke } from "@tauri-apps/api/core";
import type {
  ConfigFile,
  InstallResult,
  LicenseResult,
  LogEntry,
  PluginJar,
  ProductInfo,
  SyncSummary,
  WorkspaceInfo,
} from "@/types";

export const api = {
  // Products
  listProducts: () => invoke<ProductInfo[]>("list_products"),
  refreshProductStatus: (id: string) =>
    invoke<ProductInfo>("refresh_product_status", { productId: id }),

  // Install / Uninstall（异步命令；install/uninstall all 一次调用覆盖全部产品；
  // 安装成功后自动生成 <prd>.key 授权文件）
  installProduct: (id: string, licenseName?: string, licenseExpiry?: string) =>
    invoke<InstallResult>("install_product", { productId: id, licenseName, licenseExpiry }),
  uninstallProduct: (id: string) =>
    invoke<InstallResult>("uninstall_product", { productId: id }),
  installAllProducts: (licenseName?: string, licenseExpiry?: string) =>
    invoke<InstallResult[]>("install_all_products", { licenseName, licenseExpiry }),
  uninstallAllProducts: () =>
    invoke<InstallResult[]>("uninstall_all_products"),
  cleanupEnvVars: () => invoke<void>("cleanup_env_vars"),

  // 自定义授权（重新生成 <prd>.key，无需重装）
  generateLicenseKeys: (licenseName?: string, licenseExpiry?: string) =>
    invoke<LicenseResult[]>("generate_license_keys", { licenseName, licenseExpiry }),

  // 同步最新 agent 资源（ckey.run 当前版本；安装流程内也会自动执行）
  syncAgentResources: () => invoke<SyncSummary>("sync_agent_resources"),

  // vmoptions（path_or_id：绝对路径或产品 id）
  readVmoptions: (pathOrId: string) =>
    invoke<string>("read_vmoptions", { pathOrId }),
  writeVmoptions: (pathOrId: string, content: string) =>
    invoke<void>("write_vmoptions", { pathOrId, content }),
  stripVmoptions: (pathOrId: string) =>
    invoke<void>("strip_vmoptions", { pathOrId }),

  // Configs
  listConfigs: () => invoke<ConfigFile[]>("list_configs"),
  readConfig: (relativePath: string) =>
    invoke<string>("read_config", { relativePath }),
  writeConfig: (relativePath: string, content: string) =>
    invoke<string>("write_config", { relativePath, content }),

  // Plugins
  readPluginJars: () => invoke<PluginJar[]>("read_plugin_jars"),

  // Workspace
  getWorkspaceInfo: () => invoke<WorkspaceInfo>("get_workspace_info"),
  revealInFinder: (path: string) =>
    invoke<void>("reveal_in_finder", { path }),

  // Logs
  getLogHistory: () => invoke<LogEntry[]>("get_log_history"),
  clearLogHistory: () => invoke<void>("clear_log_history"),
  getLogFilePath: () => invoke<string | null>("get_log_file_path"),

  // Misc
  appVersion: () => invoke<string>("app_version"),
};
