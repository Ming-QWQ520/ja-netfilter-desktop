/**
 * 轻量 i18n —— 参考 ckey_script.ps1 的 Get-i18nString 结构（key -> {zh, en}）。
 * 语言状态由 stores/ui.ts 持久化到 localStorage，默认跟随系统。
 */

import { computed } from "vue";
import { ui } from "@/stores/ui";

export type Locale = "zh" | "en";

type Dict = Record<string, { zh: string; en: string }>;

export const dict: Dict = {
  // App bar
  app_name: { zh: "ja-netfilter 桌面版", en: "ja-netfilter Desktop" },
  app_subtitle: { zh: "JetBrains Java agent 管理器", en: "JetBrains Java agent manager" },

  // Tabs
  tab_overview: { zh: "总览", en: "Overview" },
  tab_config: { zh: "配置", en: "Config" },
  tab_logs: { zh: "日志", en: "Logs" },
  tab_settings: { zh: "设置", en: "Settings" },

  // Overview
  overview_desc: {
    zh: "检测本机 JetBrains IDE，一键写入/移除 javaagent。",
    en: "Detect local JetBrains IDEs and toggle the javaagent in one click.",
  },
  stat_installed: { zh: "已安装", en: "Installed" },
  stat_detected: { zh: "检测到产品", en: "Detected" },
  stat_agent_ready: { zh: "Agent 就绪", en: "Agent ready" },
  install_all: { zh: "全部安装", en: "Install all" },
  uninstall_all: { zh: "全部卸载", en: "Uninstall all" },
  clean_env: { zh: "清理环境变量", en: "Clean env vars" },
  refresh: { zh: "刷新", en: "Refresh" },
  installed_badge: { zh: "已安装", en: "Installed" },
  not_installed_badge: { zh: "未安装", en: "Not installed" },
  btn_install: { zh: "安装", en: "Install" },
  btn_reinstall: { zh: "重装", en: "Reinstall" },
  btn_uninstall: { zh: "卸载", en: "Uninstall" },
  license_name_label: {
    zh: "自定义授权（名称 + 到期时间，随安装写入授权文件）",
    en: "Custom license (name + expiry, written into license files on install)",
  },
  license_hint: {
    zh: "留空/回车默认：名称 Ming，到期 2099-12-31；授权文件由 ckey.run 生成并写入 IDE 配置目录。",
    en: "Leave empty / press Enter for defaults: name Ming, expiry 2099-12-31. License files are generated via ckey.run into each IDE config dir.",
  },
  no_products: { zh: "未检测到任何 JetBrains IDE。", en: "No JetBrains IDE detected." },
  detecting: { zh: "正在检测已安装的 JetBrains 产品…", en: "Detecting installed JetBrains products…" },
  vmoptions_label: { zh: "vmoptions", en: "vmoptions" },
  source_user: { zh: "用户配置", en: "User config" },
  source_ide: { zh: "IDE 安装目录", en: "IDE install dir" },
  source_template: { zh: "内置模板", en: "Bundled template" },
  source_missing: { zh: "缺失", en: "Missing" },
  ide_not_found: { zh: "未检测到该 IDE，安装时将自动跳过。", en: "IDE not detected; it will be skipped during install." },
  agent_line_label: { zh: "当前 javaagent 行", en: "Current javaagent line" },
  plugins_title: { zh: "插件", en: "Plugins" },
  plugins_desc: {
    zh: "IDE 启动时由 lib.jar 从应用资源目录就地加载。",
    en: "Loaded by lib.jar directly from the app's bundled resources at IDE startup.",
  },
  no_plugins: { zh: "未找到插件 jar。", en: "No plugin jars found." },

  // Confirm dialog
  confirm_install_all: {
    zh: "将为所有检测到的 IDE 写入 javaagent，并按当前授权信息生成授权文件（自动清理旧的 -javaagent 行），继续？",
    en: "Write the javaagent into every detected IDE and generate license files with the current license info (stale -javaagent lines are cleaned first)?",
  },
  confirm_uninstall_all: {
    zh: "将从所有 IDE 的 vmoptions 中移除 javaagent 行并删除授权文件，继续？",
    en: "Remove javaagent lines and license files from all IDEs?",
  },
  confirm_clean_env: {
    zh: "将删除所有 <PRODUCT>_VM_OPTIONS 环境变量（含 Machine 作用域），旧版安装的残留配置会被清除，继续？",
    en: "Delete every <PRODUCT>_VM_OPTIONS env var (User + Machine)? Legacy leftovers will be removed.",
  },
  confirm_strip_vmoptions: {
    zh: "清除该 vmoptions 中的全部受管行（-javaagent / 旧名称行）？",
    en: "Strip all managed lines (-javaagent / legacy name line) from this vmoptions file?",
  },
  action_continue: { zh: "继续", en: "Continue" },
  action_cancel: { zh: "取消", en: "Cancel" },

  // Config tab
  config_files: { zh: "插件配置", en: "Plugin configs" },
  config_files_desc: {
    zh: "直接编辑应用自带的规则文件（agent 就地加载，无需复制）。",
    en: "Edit the bundled rule files in place (loaded directly by the agent, no copies).",
  },
  vmoptions_editor: { zh: "vmoptions", en: "vmoptions" },
  vmoptions_desc: {
    zh: "直接查看/编辑各产品真实的 vmoptions 文件。",
    en: "View/edit the real per-product vmoptions files.",
  },
  save: { zh: "保存", en: "Save" },
  saving: { zh: "保存中…", en: "Saving…" },
  revert: { zh: "撤销", en: "Revert" },
  strip_agents: { zh: "清除 agent 行", en: "Strip agent lines" },
  dirty: { zh: "未保存", en: "Unsaved" },
  synced: { zh: "已同步", en: "Synced" },
  no_file_selected: { zh: "请选择左侧文件", en: "Pick a file on the left" },
  select_product: { zh: "选择产品", en: "Select product" },

  // Logs
  logs_desc: {
    zh: "安装/卸载/文件读写操作实时同步（Rust 事件推送），并持久化到磁盘日志文件。",
    en: "Install/uninstall/IO operations streamed live via Rust events and persisted to a log file.",
  },
  log_all: { zh: "全部", en: "All" },
  log_debug: { zh: "调试", en: "Debug" },
  log_info: { zh: "信息", en: "Info" },
  log_success: { zh: "成功", en: "Success" },
  log_warn: { zh: "警告", en: "Warn" },
  log_error: { zh: "错误", en: "Error" },
  autoscroll: { zh: "自动滚动", en: "Auto scroll" },
  copy: { zh: "复制", en: "Copy" },
  clear: { zh: "清空", en: "Clear" },
  no_logs: { zh: "暂无日志", en: "No logs yet" },
  log_file: { zh: "日志文件", en: "Log file" },
  log_open_dir: { zh: "打开目录", en: "Open folder" },
  log_file_missing: { zh: "（尚未生成）", en: "(not created yet)" },

  // Settings
  appearance: { zh: "外观", en: "Appearance" },
  theme_label: { zh: "主题", en: "Theme" },
  theme_system: { zh: "跟随系统", en: "System" },
  theme_light: { zh: "浅色", en: "Light" },
  theme_dark: { zh: "深色", en: "Dark" },
  language_label: { zh: "语言 / Language", en: "Language / Language" },
  behavior: { zh: "行为", en: "Behavior" },
  paths: { zh: "路径", en: "Paths" },
  path_agent_root: { zh: "Agent 资源目录（就地引用）", en: "Agent resources (in-place)" },
  path_agent_root_desc: {
    zh: "lib.jar / 插件 / 配置随应用安装包自带，agent 就地引用、零复制；vmoptions 中的 -javaagent 指向这里。",
    en: "lib.jar / plugins / configs ship with the app and are referenced in place (zero copy); -javaagent lines point here.",
  },
  path_config: { zh: "插件配置目录", en: "Plugin config directory" },
  path_plugins: { zh: "插件目录", en: "Plugins directory" },
  path_vmoptions_tpl: { zh: "vmoptions 模板目录", en: "vmoptions templates" },
  path_workdir: { zh: "应用数据目录", en: "App data directory" },
  reveal: { zh: "打开所在位置", en: "Reveal" },
  about: { zh: "关于", en: "About" },
  version: { zh: "版本", en: "Version" },
  platform: { zh: "平台", en: "Platform" },
  disclaimer: {
    zh: "ja-netfilter 是通用 Java agent 框架，可用于调试、监控和扩展基于 JVM 的应用程序。请仅在您拥有合法授权的软件上使用；绕过付费授权可能违反软件服务条款及当地法律法规。",
    en: "ja-netfilter is a general-purpose Java agent framework for debugging, monitoring and extending JVM applications. Use only on software you are licensed for; bypassing paid licensing may violate terms of service and local law.",
  },
  license_saved: { zh: "授权信息已保存，下次安装时生效。", en: "License info saved; applies on next install." },
  agent_sync_title: { zh: "Agent 资源同步", en: "Agent resources sync" },
  agent_sync_desc: {
    zh: "从 ckey.run 拉取当前最新的 lib.jar / 配置 / 插件并就地更新（与 ckey_script.ps1 同源同版本）。授权 key 必须与当前版 power.conf 配对，安装前也会自动同步。",
    en: "Fetch the latest lib.jar / configs / plugins from ckey.run and update them in place (same source as ckey_script.ps1). License keys must match the current power.conf; sync also runs automatically before installs.",
  },
  agent_sync_btn: { zh: "同步最新 Agent", en: "Sync latest agent" },
  agent_sync_running: { zh: "正在同步……", en: "Syncing…" },
  agent_sync_done: { zh: "同步完成：{msg}", en: "Sync finished: {msg}" },
  agent_sync_fail: { zh: "同步失败", en: "Sync failed" },
  saved_ok: { zh: "已保存", en: "Saved" },
  copied: { zh: "已复制到剪贴板", en: "Copied to clipboard" },
  logs_cleared: { zh: "日志已清空", en: "Logs cleared" },
  env_cleaned: { zh: "环境变量已清理", en: "Env vars cleaned" },
  agents_stripped: { zh: "已清除受管行", en: "Managed lines stripped" },

  // toasts
  install_done: { zh: "javaagent 与授权文件已就绪", en: "javaagent & license file ready" },
  install_failed: { zh: "安装失败", en: "Install failed" },
  uninstall_done: { zh: "javaagent 与授权文件已移除", en: "javaagent & license file removed" },
  install_all_done: {
    zh: "已为检测到的产品安装 javaagent",
    en: "javaagent installed for detected products",
  },
  uninstall_all_done: { zh: "已移除全部 javaagent 配置", en: "All javaagent entries removed" },
  load_failed: { zh: "加载失败", en: "Load failed" },

  plugin_docs: {
    zh: "dns：拦截/改写域名解析；hideme：对 IDE 探测隐藏 agent；power：RSA 密钥变换引擎；url：拦截 URL 请求返回本地响应。",
    en: "dns: hook name resolution; hideme: hide agent from probes; power: RSA transform engine; url: intercept URLs with local responses.",
  },
};

/** 当前语言的翻译函数（响应式；无命中时回退 key）。 */
export function useT() {
  return computed(
    () =>
      (key: string): string => {
        const entry = dict[key];
        if (!entry) return key;
        const uiStore = ui();
        return entry[uiStore.locale as Locale] ?? entry.zh;
      },
  ).value;
}

/** 非响应式场景直接取值。 */
export function t(key: string): string {
  const entry = dict[key];
  if (!entry) return key;
  const uiStore = ui();
  return entry[uiStore.locale as Locale] ?? entry.zh;
}
