#!/usr/bin/env bash
# 一键更新版本号脚本
# 同步四处版本号：package.json / src-tauri/tauri.conf.json / src-tauri/Cargo.toml / src-tauri/Cargo.lock(app 条目)
# 用法:
#   ./scripts/bump-version.sh 2.1.0            仅更新四处版本号
#   ./scripts/bump-version.sh 2.1.0 --tag      更新后自动 git commit 并打 v2.1.0 tag
#   ./scripts/bump-version.sh patch|minor|major       基于当前版本自动递增
#   ./scripts/bump-version.sh minor --tag

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

ARG="${1:-}"
DO_TAG=""
[ "${2:-}" = "--tag" ] && DO_TAG=1

if [ -z "$ARG" ]; then
  echo "用法: ./scripts/bump-version.sh <版本号|patch|minor|major> [--tag]" >&2
  exit 1
fi

# 读取当前版本（以 tauri.conf.json 为准）
OLD="$(grep -m1 '"version"' src-tauri/tauri.conf.json | sed -E 's/.*"version":[[:space:]]*"([^"]+)".*/\1/')"
if [ -z "$OLD" ]; then
  echo "✗ 无法从 src-tauri/tauri.conf.json 读取当前版本" >&2
  exit 1
fi

# 计算新版本
case "$ARG" in
  major|minor|patch)
    IFS='.' read -r MA MI PA <<< "$OLD"
    case "$ARG" in
      major) MA=$((MA + 1)); MI=0; PA=0 ;;
      minor) MI=$((MI + 1)); PA=0 ;;
      patch) PA=$((PA + 1)) ;;
    esac
    NEW="$MA.$MI.$PA"
    ;;
  *)
    if ! echo "$ARG" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$'; then
      echo "✗ 版本号格式应为 x.y.z（或使用 patch/minor/major）: $ARG" >&2
      exit 1
    fi
    NEW="$ARG"
    ;;
esac

if [ "$NEW" = "$OLD" ]; then
  echo "✗ 新版本与当前版本相同（${OLD}）" >&2
  exit 1
fi

if git rev-parse "v$NEW" >/dev/null 2>&1; then
  echo "✗ tag v$NEW 已存在" >&2
  exit 1
fi

echo "==> 版本号 $OLD -> $NEW"

# 1) package.json（顶层 version 字段）
sed -i '' "s/\"version\": \"$OLD\"/\"version\": \"$NEW\"/" package.json

# 2) src-tauri/tauri.conf.json
sed -i '' "s/\"version\": \"$OLD\"/\"version\": \"$NEW\"/" src-tauri/tauri.conf.json

# 3) src-tauri/Cargo.toml（仅顶层 [package].version，^version 锚定排除依赖项）
sed -i '' "s/^version = \"$OLD\"/version = \"$NEW\"/" src-tauri/Cargo.toml

# 4) src-tauri/Cargo.lock（仅 name = "app" 紧随的 version 行）
awk -v old="$OLD" -v new="$NEW" '
  prev == "name = \"app\"" && $0 == "version = \"" old "\"" { sub("\"" old "\"", "\"" new "\"") }
  { print; prev = $0 }
' src-tauri/Cargo.lock > src-tauri/Cargo.lock.tmp && mv src-tauri/Cargo.lock.tmp src-tauri/Cargo.lock

echo "  ✓ package.json           $(grep -m1 '"version"' package.json | tr -d ' ')"
echo "  ✓ tauri.conf.json        $(grep -m1 '"version"' src-tauri/tauri.conf.json | tr -d ' ')"
echo "  ✓ Cargo.toml             $(grep -m1 '^version' src-tauri/Cargo.toml)"
echo "  ✓ Cargo.lock (app)       version = \"$(awk '/name = "app"/{getline; print; exit}' src-tauri/Cargo.lock | sed -E 's/.*"([^"]+)".*/\1/')\""

if [ -n "$DO_TAG" ]; then
  echo "==> 提交并打 tag v$NEW"
  git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock
  git commit -m "chore(release): v$NEW"
  git tag "v$NEW"
  echo "  ✓ 已创建 commit 与 tag v${NEW}（推送：git push origin main --follow-tags）"
else
  echo "==> 已更新版本号（未提交）。如需发版：git commit + git tag v${NEW}，或重跑加 --tag"
fi
