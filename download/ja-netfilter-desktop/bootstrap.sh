#!/usr/bin/env bash
# Bootstrap, type-check and build the ja-netfilter-desktop project.
# Run this once on a fresh checkout to verify the toolchain works.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"

echo "==> 0. Ensuring pnpm is available"
if ! command -v pnpm >/dev/null 2>&1; then
  echo "    pnpm not found — installing via corepack"
  corepack enable
  corepack prepare pnpm@9 --activate
fi
pnpm --version

echo "==> 1. Installing dependencies"
if [ ! -d node_modules ]; then
  pnpm install --frozen-lockfile=false
else
  echo "    node_modules already exists, skipping install"
fi

echo "==> 2. Type-checking the Vue 3 frontend"
pnpm lint

echo "==> 3. Building the Vite bundle"
pnpm build

echo
echo "==> Frontend OK."
echo
echo "==> Next steps:"
echo "    - Install Rust toolchain: https://rustup.rs"
echo "    - Install Tauri 2 system deps: https://v2.tauri.app/start/prerequisites/"
echo "    - Run the desktop app in dev mode:   pnpm tauri:dev"
echo "    - Build a distributable bundle:      pnpm tauri:build"
echo
echo "==> Output bundles land in: src-tauri/target/release/bundle/"
