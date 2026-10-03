# ja-netfilter Desktop

A cross-platform **Tauri + Vue 3** desktop GUI for the [ja-netfilter](https://github.com/ja-netfilter/ja-netfilter) Java agent framework. It replaces the bundled shell / VBS install scripts with a single, friendly interface that:

- 🔍 **Auto-detects** every JetBrains IDE installed for the current user.
- ⚡ **Installs / uninstalls** the `-javaagent` line into each product's vmoptions with one click.
- 📝 **Edits** the `dns.conf` / `power.conf` / `url.conf` plugin configs in-app.
- 🛠 **Inspects** the bundled vmoptions templates per product.
- 🧩 **Lists** the loaded plugin jars (`dns.jar`, `hideme.jar`, `power.jar`, `url.jar`).
- 📜 **Streams** every install / uninstall / IO operation through a live log console.
- 📦 **Bundles** the original `lib.jar`, `plugins/`, `config/`, `vmoptions/` and `scripts/` as Tauri resources.

> ⚠️ **Responsible-use notice.** ja-netfilter is a generic Java agent framework. It can be used to debug, monitor and extend JVM-based applications. Only attach it to software you are licensed to run; circumventing paid license checks may violate the software's terms of service and local law.

---

## Architecture

```
ja-netfilter-desktop/
├── src/                          # Vue 3 + Pinia frontend
│   ├── components/               # Sidebar, Dashboard, ProductCard, ConfigEditor,
│   │                             # VmoptionsViewer, PluginsPanel, SettingsPanel,
│   │                             # LogConsole, ToastHost
│   ├── stores/                   # products / settings / logs / toast
│   ├── api/                      # typed wrappers around `invoke`
│   ├── types/                    # shared types mirrored from the Rust side
│   └── styles/main.css           # JetBrains-inspired dark theme
├── src-tauri/                    # Rust backend
│   ├── src/
│   │   ├── main.rs               # binary entry
│   │   ├── lib.rs                # Tauri builder + command registration
│   │   ├── commands.rs            # #[tauri::command] surface
│   │   ├── products.rs            # JetBrains product detection
│   │   ├── vmoptions.rs           # javaagent-aware vmoptions editing
│   │   ├── config.rs              # dns/power/url config IO
│   │   ├── installer.rs           # cross-platform install / uninstall
│   │   ├── platform.rs            # OS abstraction
│   │   ├── workspace.rs           # per-user writable workspace
│   │   └── logger.rs              # in-memory log buffer
│   ├── resources/                # bundled lib.jar / plugins / configs / scripts
│   ├── capabilities/default.json  # Tauri 2 capability manifest
│   ├── icons/                    # placeholder app icons
│   └── tauri.conf.json           # Tauri 2 config
└── package.json                  # Node toolchain (Vite + Tauri CLI)
```

### Install / uninstall semantics

The Rust installer in `src-tauri/src/installer.rs` performs the **same steps** as the upstream `scripts/install.sh` and `scripts/install-current-user.vbs`:

1. Locate the per-product vmoptions file in this order:
   - `$<PRODUCT>_VM_OPTIONS` env var
   - Default per-user location (`~/.config/JetBrains/<product>/idea.vmoptions` on Linux, `~/Library/Application Support/JetBrains/...` on macOS, `%APPDATA%\JetBrains\<product>\idea.vmoptions` on Windows)
   - Bundled workspace template (`<workdir>/vmoptions/<product>.vmoptions`)
2. Strip any existing `-javaagent:...ja-netfilter.jar...` or `-javaagent:...=jetbrains` line.
3. Append `-javaagent:<workdir>/lib.jar=jetbrains`.
4. Persist `<PRODUCT>_VM_OPTIONS=<vmoptions_path>`:
   - **macOS** — `launchctl setenv` + write to `~/.profile` / `~/.bashrc` / `~/.zshrc`.
   - **Linux** — write to `~/.profile` / `~/.bashrc` / `~/.zshrc`.
   - **Windows** — `setx` (persists for future sessions).

### Per-user workspace

On first launch the app mirrors the bundled `lib.jar`, `plugins/`, `config/`, `vmoptions/` into a writable per-user directory:

- **Linux** — `~/.config/ja-netfilter-desktop/`
- **macOS** — `~/Library/Application Support/ja-netfilter-desktop/`
- **Windows** — `%APPDATA%\ja-netfilter-desktop\`

All edits the user makes inside the GUI (config edits, vmoptions tweaks, jar swaps) land in that workspace and never touch the read-only app bundle. Existing files are preserved across app upgrades.

---

## Prerequisites

You need the standard Tauri 2 toolchain installed on the build machine:

| Tool | Version | Notes |
|------|---------|-------|
| Node.js | ≥ 20 LTS | for the Vue 3 frontend |
| Rust | ≥ 1.77 (stable) | for the Tauri backend |
| Tauri CLI 2 system dependencies | — | see the [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/) for your OS |

On Linux you'll additionally need `webkit2gtk-4.1`, `librsvg`, `libgtk-3`, `libayatana-appindicator3-1` and friends — the Tauri docs list them per distro.

---

## Development

```bash
# 1. Install JS deps
npm install

# 2. Run the dev build (starts Vite + Tauri together, hot reload on the frontend)
npm run tauri:dev
```

The first run will compile the entire Rust backend (≈ 2-3 minutes) and launch the desktop window.

## Production build

```bash
npm run tauri:build
```

Output artifacts land under `src-tauri/target/release/bundle/`:

- **Linux** — `.deb`, `.rpm`, `.AppImage`
- **macOS** — `.dmg`, `.app`
- **Windows** — `.msi`, `.exe` (NSIS)

---

## Tauri commands (Rust ↔ Vue bridge)

The frontend talks to the backend exclusively through `@tauri-apps/api`'s `invoke`. The full command surface:

| Command | Purpose |
|---------|---------|
| `list_products` | Detect all known JetBrains products and return their state. |
| `refresh_product_status` | Re-detect a single product. |
| `install_product` / `uninstall_product` | Edit a single product's vmoptions + env var. |
| `install_all_products` / `uninstall_all_products` | Bulk version of the above. |
| `read_vmoptions` / `write_vmoptions` / `reset_vmoptions` | Per-product vmoptions IO. |
| `list_configs` / `read_config` / `write_config` | dns / power / url config IO. |
| `read_plugin_jars` | Enumerate the plugin jars in the workspace. |
| `get_workspace_info` | Return paths, OS, app version, jar presence. |
| `reveal_in_finder` | Open a path in the native file explorer. |
| `pick_jar_file` | Native file picker (filtered to `*.jar`). |
| `set_active_jar_path` | Stub for future "swap lib.jar" feature. |
| `get_log_history` / `clear_log_history` | Manage the in-memory log buffer. |
| `app_version` | Return the app version string. |

The TypeScript wrappers live in [`src/api/index.ts`](src/api/index.ts).

---

## Customising the bundled lib.jar

If you have your own build of `ja-netfilter.jar` (or just want to swap the jar in place), you can either:

- Replace the file at `<workdir>/lib.jar` after first launch (the workspace takes precedence), or
- Replace `src-tauri/resources/lib.jar` *before* running `npm run tauri:build` so the bundle ships with your version.

The vmoptions files always reference `<workdir>/lib.jar`, so the swap is transparent to the install logic.

---

## Regenerating icons

The placeholder icons under `src-tauri/icons/` were generated with:

```bash
python3 /home/z/my-project/scripts/make_icons.py
```

(re-run from the repo root after editing that script). For production-grade artwork, drop a 1024×1024 source PNG at `src-tauri/icons/icon.png` and use `tauri icon` from the Tauri CLI to regenerate the full set.

---

## License

This desktop wrapper is released under the MIT license. The bundled `lib.jar`, `plugins/`, `config/`, `vmoptions/` and `scripts/` files are the work of the upstream ja-netfilter project — please consult their respective licenses.
