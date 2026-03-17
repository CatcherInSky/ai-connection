#!/usr/bin/env bash
# 在 WSL 中运行此脚本以创建 Tauri 所需的 icons/icon.png（1x1 占位图）
set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
mkdir -p "$ROOT/icons"
echo 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADBgGApJ5P7wAAAABJRU5ErkJggg==' | base64 -d > "$ROOT/icons/icon.png"
echo "Created $ROOT/icons/icon.png"
