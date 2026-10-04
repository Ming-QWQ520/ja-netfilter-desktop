# ja-netfilter 桌面版

基于 **Tauri + Vue 3** 的跨平台桌面 GUI，用于管理 [ja-netfilter](https://github.com/ja-netfilter/ja-netfilter) Java agent 框架。它将原版附带的 shell / VBS 安装脚本替换为一个友好、可视化的界面，提供：

- 🔍 **自动检测**当前用户已安装的全部 JetBrains IDE。
- ⚡ **一键安装 / 卸载** 各产品的 `-javaagent` 行到对应 vmoptions 文件。
- 📝 **在线编辑** `dns.conf` / `power.conf` / `url.conf` 插件配置文件。
- 🛠 **查看** 各产品自带的 vmoptions 模板。
- 🧩 **列出** 加载的插件 jar（`dns.jar`、`hideme.jar`、`power.jar`、`url.jar`）。
- 📜 **实时日志** 同步显示每一次安装 / 卸载 / 读写操作。
- 🪪 **自定义授权名称**，安装时写入 `-Dja.netfilter.name=<value>`。
- 📦 **直接调用** 项目自带的 `lib.jar`、`plugins/`、`config/`、`vmoptions/`、`scripts/`，不再复制到用户工作区。

> ⚠️ **使用须知**：ja-netfilter 是一个通用的 Java agent 框架，可用于调试、监控和扩展基于 JVM 的应用程序。请仅在您拥有合法授权的软件上使用；绕过付费授权可能违反软件服务条款及当地法律法规。

---

## 目录结构

```
ja-netfilter-desktop/
├── src/                          # Vue 3 + Pinia 前端
│   ├── components/               # Sidebar、Dashboard、ProductCard、ConfigEditor、
│   │                             # VmoptionsViewer、PluginsPanel、SettingsPanel、
│   │                             # LogConsole、ToastHost
│   ├── stores/                   # products / settings / logs / toast / license
│   ├── api/                      # invoke 调用的类型化封装
│   ├── types/                    # 与 Rust 端对应的共享类型
│   └── styles/main.css           # JetBrains 风格的深色主题
├── src-tauri/                    # Rust 后端
│   ├── src/
│   │   ├── main.rs               # 二进制入口
│   │   ├── lib.rs                # Tauri builder 与命令注册
│   │   ├── commands.rs            # #[tauri::command] 命令表面
│   │   ├── products.rs            # JetBrains 产品检测
│   │   ├── vmoptions.rs           # 感知 javaagent 的 vmoptions 编辑
│   │   ├── config.rs              # dns/power/url 配置读写
│   │   ├── installer.rs           # 跨平台安装 / 卸载
│   │   ├── platform.rs            # 操作系统抽象
│   │   ├── workspace.rs           # 项目自带 resources 目录定位
│   │   └── logger.rs              # 内存日志缓冲
│   ├── resources/                # 自带的 lib.jar / plugins / configs / scripts
│   ├── capabilities/default.json  # Tauri 2 capability 清单
│   ├── icons/                    # 占位应用图标
│   └── tauri.conf.json           # Tauri 2 配置
└── package.json                  # Node 工具链（Vite + Tauri CLI，pnpm）
```

### 安装 / 卸载语义

Rust 安装器（`src-tauri/src/installer.rs`）执行的步骤与原版 `scripts/install.sh` 和 `scripts/install-current-user.vbs` **完全一致**：

1. 按以下顺序查找各产品的 vmoptions 文件：
   - `$<PRODUCT>_VM_OPTIONS` 环境变量
   - 用户默认位置（Linux：`~/.config/JetBrains/<product>/idea.vmoptions`；macOS：`~/Library/Application Support/JetBrains/...`；Windows：`%APPDATA%\JetBrains\<product>\idea.vmoptions`）
   - 项目自带模板（`<resource_root>/vmoptions/<product>.vmoptions`）
2. 移除任何已存在的 `-javaagent:...ja-netfilter.jar...` 或 `-javaagent:...=jetbrains` 行。
3. 追加 `-javaagent:<resource_root>/lib.jar=jetbrains`。
4. 若提供了自定义授权名称，则追加 `-Dja.netfilter.name=<value>`。
5. 持久化 `<PRODUCT>_VM_OPTIONS=<vmoptions_path>`：
   - **macOS** —— `launchctl setenv` + 写入 `~/.profile` / `~/.bashrc` / `~/.zshrc`
   - **Linux** —— 写入 `~/.profile` / `~/.bashrc` / `~/.zshrc`
   - **Windows** —— `setx`（持久化到后续会话）

### 项目自带资源

应用直接读取 bundle 内的 `resources/` 目录，**不再镜像到用户工作区**：

- **lib.jar**、`plugins/*.jar`、`config/*.conf`、`vmoptions/*.vmoptions` 始终位于应用 bundle 内的同一路径。
- 用户编辑（配置改动、vmoptions 调整、jar 替换）直接作用于这些文件，与原版 install.sh 的行为一致：原版 install.sh 同样在分发目录原地修改 vmoptions。
- 检测产品时，工作区模板路径就是 `<bundle>/resources/vmoptions/<id>.vmoptions`。

---

## 环境要求

构建机需要标准的 Tauri 2 工具链：

| 工具 | 版本 | 说明 |
|------|------|------|
| Node.js | ≥ 20 LTS | 用于 Vue 3 前端 |
| Rust | ≥ 1.77（stable） | 用于 Tauri 后端 |
| Tauri CLI 2 系统依赖 | — | 参见 [Tauri 2 前置条件](https://v2.tauri.app/start/prerequisites/) |

Linux 上还需要 `webkit2gtk-4.1`、`librsvg`、`libgtk-3`、`libayatana-appindicator3-1` 等 —— Tauri 文档按发行版列出了完整清单。

---

## 开发

```bash
# 0. 启用 pnpm（Node ≥ 20 自带 corepack）
corepack enable
corepack prepare pnpm@9 --activate

# 1. 安装 JS 依赖
pnpm install

# 2. 运行开发版（同时启动 Vite + Tauri，前端热更新）
pnpm tauri:dev
```

首次运行会编译整个 Rust 后端（约 2-3 分钟），然后启动桌面窗口。

## 生产构建

```bash
pnpm tauri:build
```

产物位于 `src-tauri/target/release/bundle/`：

- **Linux** —— `.deb`、`.rpm`、`.AppImage`
- **macOS** —— `.dmg`、`.app`
- **Windows** —— `.msi`、`.exe`（NSIS）

## CI / GitHub Actions

仓库内置两个工作流（位于 `.github/workflows/`）：

| 工作流 | 触发条件 | 作用 |
|--------|----------|------|
| `ci.yml` | push / PR | 安装 pnpm + Rust，前端类型检查，执行 `pnpm build`，运行 `cargo check` + `cargo clippy` |
| `release.yml` | tag `v*` | 跨平台编译 Windows / macOS（Intel + ARM）/ Linux 安装包，自动上传到 GitHub Release |

发布新版本：

```bash
git tag v0.1.0
git push origin v0.1.0
```

release 工作流会并行构建三个平台，并将 `.msi`、`.dmg`、`.AppImage`、`.deb`、`.rpm` 等产物上传到 Release 页面。

---

## Tauri 命令（Rust ↔ Vue 桥接）

前端通过 `@tauri-apps/api` 的 `invoke` 与后端通信。完整命令清单：

| 命令 | 作用 |
|------|------|
| `list_products` | 检测全部已知的 JetBrains 产品并返回其状态 |
| `refresh_product_status` | 重新检测单个产品 |
| `install_product` / `uninstall_product` | 编辑单个产品的 vmoptions + 环境变量；`install_product` 接受可选的 `license_name` |
| `install_all_products` / `uninstall_all_products` | 批量版本；`install_all_products` 接受可选的 `license_name` |
| `read_vmoptions` / `write_vmoptions` / `reset_vmoptions` | 单产品 vmoptions 读写 |
| `list_configs` / `read_config` / `write_config` | dns / power / url 配置读写 |
| `read_plugin_jars` | 枚举 resource_root 中的插件 jar |
| `get_workspace_info` | 返回路径、OS、应用版本、jar 是否存在 |
| `reveal_in_finder` | 在系统文件管理器中打开路径 |
| `pick_jar_file` | 原生文件选择器（过滤 `*.jar`） |
| `set_active_jar_path` | 预留：未来用于切换 lib.jar 路径 |
| `get_log_history` / `clear_log_history` | 管理内存日志缓冲 |
| `app_version` | 返回应用版本号 |

TypeScript 封装位于 [`src/api/index.ts`](src/api/index.ts)。

---

## 自定义授权名称

在「设置」页面可填写自定义授权名称。保存后，**下次安装** javaagent 时，会同时在 vmoptions 文件中追加：

```
-Dja.netfilter.name=<value>
```

留空则不写入该参数，使用 ja-netfilter 默认行为。该值保存在浏览器 `localStorage` 中，下次启动应用自动恢复。

---

## 替换 lib.jar

若您有自编译的 `ja-netfilter.jar`（或仅想替换默认 jar），可直接：

- 替换 `src-tauri/resources/lib.jar`，然后执行 `pnpm tauri:build`，新版本将包含您的 jar。

vmoptions 文件始终引用 `<resource_root>/lib.jar`，因此替换对安装逻辑透明。

---

## 重新生成图标

`src-tauri/icons/` 下的占位图标由以下脚本生成：

```bash
python3 /home/z/my-project/scripts/make_icons.py
```

如需生产级图标，请将 1024×1024 源 PNG 放在 `src-tauri/icons/icon.png`，然后使用 Tauri CLI 的 `tauri icon` 命令重新生成完整图标集。

---

## 许可证

本桌面封装基于 MIT 协议发布。附带的 `lib.jar`、`plugins/`、`config/`、`vmoptions/`、`scripts/` 为上游 ja-netfilter 项目所有，请参阅其各自的许可证。
