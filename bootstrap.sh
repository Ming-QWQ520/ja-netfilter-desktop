#!/usr/bin/env bash
# 引导脚本：安装依赖、类型检查、构建 ja-netfilter-desktop 项目。
# 首次检出时运行一次以验证工具链可用。

set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"

echo "==> 0. 确保 pnpm 可用"
if ! command -v pnpm >/dev/null 2>&1; then
  echo "    未找到 pnpm —— 通过 corepack 安装"
  corepack enable
  corepack prepare pnpm@9 --activate
fi
pnpm --version

echo "==> 1. 安装依赖"
if [ ! -d node_modules ]; then
  pnpm install --frozen-lockfile=false
else
  echo "    node_modules 已存在，跳过安装"
fi

echo "==> 2. 类型检查 Vue 3 前端"
pnpm lint

echo "==> 3. 构建 Vite 产物"
pnpm build

echo
echo "==> 前端 OK。"
echo
echo "==> 后续步骤："
echo "    - 安装 Rust 工具链：https://rustup.rs"
echo "    - 安装 Tauri 2 系统依赖：https://v2.tauri.app/start/prerequisites/"
echo "    - 开发模式运行桌面应用：   pnpm tauri:dev"
echo "    - 构建可分发包：           pnpm tauri:build"
echo
echo "==> 产物位于：src-tauri/target/release/bundle/"
