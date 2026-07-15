#!/usr/bin/env bash
# 清理构建缓存脚本
# 用法:
#   ./scripts/clean.sh          清理本项目缓存（前端产物 + Vite 缓存 + Rust 本 crate 产物）
#   ./scripts/clean.sh --all    连同 node_modules 和整个 Rust target/ 一起删除
#   ./scripts/clean.sh --deps   在 --all 基础上再清理 pnpm 全局 store

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

MODE="${1:-}"

rm_path() {
  if [ -e "$1" ]; then
    echo "  删除 $1"
    rm -rf "$1"
  fi
}

echo "==> 清理前端产物"
rm_path "dist"
rm_path "node_modules/.vite"
rm_path ".DS_Store"

echo "==> 清理 Rust 产物"
if [ "$MODE" = "--all" ] || [ "$MODE" = "--deps" ]; then
  echo "  cargo clean（整个 target/）"
  (cd src-tauri && cargo clean)
else
  # 只删本 crate（包名 app，见 src-tauri/Cargo.toml），保留依赖编译缓存，下次更快
  echo "  cargo clean -p app（仅本 crate，保留依赖缓存）"
  (cd src-tauri && cargo clean -p app) || (cd src-tauri && cargo clean)
fi

if [ "$MODE" = "--all" ] || [ "$MODE" = "--deps" ]; then
  echo "==> 删除 node_modules"
  rm_path "node_modules"
fi

if [ "$MODE" = "--deps" ]; then
  echo "==> 清理 pnpm 全局 store"
  pnpm store prune || true
fi

echo "✅ 清理完成"
