#!/usr/bin/env bash
# Bootstrap, type-check and build the ja-netfilter-desktop project.
# Run this once on a fresh checkout to verify the toolchain works.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"

echo "==> 1. Installing npm dependencies"
if [ ! -d node_modules ]; then
  npm install --no-audit --no-fund
else
  echo "    node_modules already exists, skipping install"
fi

echo "==> 2. Type-checking the Vue 3 frontend"
npx vue-tsc --noEmit

echo "==> 3. Building the Vite bundle"
npx vite build

echo
echo "==> Frontend OK."
echo
echo "==> Next steps:"
echo "    - Install Rust toolchain: https://rustup.rs"
echo "    - Install Tauri 2 system deps: https://v2.tauri.app/start/prerequisites/"
echo "    - Run the desktop app in dev mode:   npm run tauri:dev"
echo "    - Build a distributable bundle:      npm run tauri:build"
echo
echo "==> Output bundles land in: src-tauri/target/release/bundle/"
