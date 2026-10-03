// Thin wrappers around the `invoke` calls that talk to the Rust backend.
// All functions return Promises so the UI can `await` them inside event
// handlers without throwing raw errors — failures are converted to
// Error objects with the backend's string reason as the message.

import { invoke } from "@tauri-apps/api/core";
import type {
  ConfigFile,
  InstallResult,
  LogEntry,
  PluginJar,
  ProductInfo,
  WorkspaceInfo,
} from "@/types";

export const api = {
  // Products
  listProducts: () => invoke<ProductInfo[]>("list_products"),
  refreshProductStatus: (id: string) =>
    invoke<ProductInfo>("refresh_product_status", { productId: id }),

  // Install / Uninstall
  // `licenseName` is optional. When provided, the backend appends an extra
  //   -Dja.netfilter.name=<licenseName>
  // line to the vmoptions file alongside the -javaagent line.
  installProduct: (id: string, licenseName?: string) =>
    invoke<InstallResult>("install_product", { productId: id, licenseName }),
  uninstallProduct: (id: string) =>
    invoke<InstallResult>("uninstall_product", { productId: id }),
  installAllProducts: (licenseName?: string) =>
    invoke<InstallResult[]>("install_all_products", { licenseName }),
  uninstallAllProducts: () => invoke<InstallResult[]>("uninstall_all_products"),

  // vmoptions
  readVmoptions: (pathOrId: string) =>
    invoke<string>("read_vmoptions", { pathOrId }),
  writeVmoptions: (pathOrId: string, content: string) =>
    invoke<void>("write_vmoptions", { pathOrId, content }),
  resetVmoptions: (pathOrId: string) =>
    invoke<void>("reset_vmoptions", { pathOrId }),

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
  revealInFinder: (path: string) => invoke<void>("reveal_in_finder", { path }),
  pickJarFile: () => invoke<string | null>("pick_jar_file"),

  // Logs
  getLogHistory: () => invoke<LogEntry[]>("get_log_history"),
  clearLogHistory: () => invoke<void>("clear_log_history"),

  // Misc
  appVersion: () => invoke<string>("app_version"),
};
