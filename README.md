# ja-netfilter-desktop

JetBrains ja-netfilter 桌面管理器 —— Tauri 2 + Vue 3 + Material Design 3。

检测本机 JetBrains IDE，一键写入/移除 `-javaagent`、清理环境变量，并按**自定义授权信息**生成授权文件。核心安装逻辑与 `ckey_script.ps1`（CodeKey Run）语义一致，并以跨平台 GUI 呈现。

## 特性

- **就地引用，零复制**：`lib.jar`、插件、配置随安装包自带，`-javaagent` 直接指向应用资源目录，不向 C 盘或其它位置复制任何文件（仅当资源路径含空格且 8.3 短路径不可用时，才兜底复制到无空格安全目录）。
- **安全安装**：安装前剥离 vmoptions 中全部旧 `-javaagent:` 行（含历史遗留 `-Dja.netfilter.name=` 行）；删除（而非设置）User + Machine 两作用域的 `<PRODUCT>_VM_OPTIONS` 环境变量并广播 `WM_SETTINGCHANGE` —— 从根上治愈 `processing of -javaagent failed` 崩溃。
- **自定义授权**：授权名称（回车/留空默认 `Ming`）与到期时间（回车/留空默认 `2099-12-31`，`yyyy-MM-dd`）随安装生成 `<prd>.key` 授权文件，写入各 IDE 用户配置目录（`%APPDATA%\JetBrains\<Product><ver>\` 等），单个产品失败仅告警不中断。
- **真实定位**：经 `%LOCALAPPDATA%\JetBrains\*\.home` 发现链定位 IDE 真实 vmoptions（bin 递归 + Roaming 配置目录），检测状态与 IDE 实际读取的文件一致。
- **MD3 单页 UI**：总览 / 配置 / 日志 / 设置四个 Tab，明暗双主题（跟随系统）、中英双语、实时日志事件流。

## 触发 Actions 编译发布

仓库内置 `.github/workflows/release.yml`（Windows / macOS / Linux 全平台构建），两种触发方式任选：

```bash
# 方式一：推送 tag 自动发布（推荐）
git tag v0.1.0
git push origin v0.1.0

# 方式二：GitHub 页面 → Actions → Release → Run workflow（tag 留空自动取 app 版本）
```

构建完成后自动在 Releases 发布 **Latest** 版本，包含：

- Windows：`*.msi` / `*-setup.exe`（NSIS，默认按当前用户安装，无需管理员）
- macOS：`*.dmg`（Apple Silicon / Intel）
- Linux：`*.deb` / `*.AppImage`

> 首次运行需在仓库 Settings → Actions → General → Workflow permissions 勾选
> "Read and write permissions"（或依赖 workflow 内已声明的 `contents: write`）。

## 本地开发

```bash
pnpm install        # 安装前端依赖
pnpm tauri:dev      # 开发模式
pnpm tauri:build    # 产出安装包（src-tauri/target/release/bundle/）
```

环境要求：Node ≥ 20 + pnpm ≥ 9、Rust stable、Tauri 2 系统依赖（Linux 需 webkit2gtk-4.1 等，Windows/macOS 无额外要求）。

## 目录结构

```
├── .github/workflows/     # ci.yml（推送检查）+ release.yml（tag 发布）
├── src/                   # Vue 3 前端（MD3 单页 + Pinia + i18n）
│   ├── components/        # Overview / Config / Logs / Settings 四个 Tab
│   ├── stores/            # products / license / ui / logs / toast / confirm
│   ├── i18n/              # zh/en 双语字典（ckey_script 同款 key 结构）
│   └── styles/md3.css     # Material Design 3 设计系统
└── src-tauri/
    ├── src/
    │   ├── installer.rs   # 安装/卸载编排（Windows→PS，Unix→原生）
    │   ├── license.rs     # 自定义授权：ckey.run 生成 <prd>.key
    │   ├── scripts/win_core.ps1  # Windows 核心（剥离旧行/删环境变量/广播/定位）
    │   ├── locate.rs      # .home 发现链（检测 + Unix 安装共用）
    │   ├── agent_home.rs  # 就地引用解析 + 极端情况兜底
    │   └── ...            # products / vmoptions / config / platform / logger
    └── resources/         # 项目自带：lib.jar + config-jetbrains + plugins-jetbrains + vmoptions
```

## 安装语义（与 ckey_script.ps1 对照）

| ckey_script.ps1 | ja-netfilter-desktop |
|---|---|
| 下载 agent 到 `%PUBLIC%\.jb_run\` | **不下载不复制**：引用安装包自带资源目录 |
| `Revert_Vm_Options` 剥离旧 `-javaagent` 行 | 同款正则 `^-javaagent:.*\.jar.*`，另剥离遗留 `-Dja.netfilter.name=` |
| `Remove_Env`（User + Machine）删 `<PRODUCT>_VM_OPTIONS` | 相同，另加 `WM_SETTINGCHANGE` 广播 |
| `.home` → `bin\*.vmoptions` + Roaming 定位 | 同款发现链（检测与安装共用一套） |
| 输入授权名称（回车默认）/ 到期时间（回车默认 2099-12-31） | GUI 双输入框，留空/回车即默认（Ming / 2099-12-31），持久化 |
| `Create_Key` POST `ckey.run/generateLicense/file` → 写 `<prd>.key` | 相同 JSON 契约（licenseName / expiryDate / productCode），单产品失败仅告警 |

## PowerShell 调试

`win_core.ps1` 以 JSON 契约通信（stdout 单行 JSON，诊断走 stderr）：

```powershell
powershell -File src-tauri/src/scripts/win_core.ps1 `
  -Mode install -Products "idea,pycharm" `
  -AgentJar "C:\path\to\lib.jar"
```

> ⚠️ PS 5.1 类型陷阱：`[string]$Products` 参数与大小写不敏感的同名变量冲突
> 会把数组静默空格 join 成字符串。脚本内已用独立变量 `$productList` 规避，
> 修改时请勿引入与参数同名（任意大小写）的数组变量。

## 许可与声明

ja-netfilter 是通用 Java agent 框架，可用于调试、监控和扩展基于 JVM 的应用程序。请仅在您拥有合法授权的软件上使用；绕过付费授权可能违反软件服务条款及当地法律法规。
