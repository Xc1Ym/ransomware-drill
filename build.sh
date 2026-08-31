#!/usr/bin/env bash
# 在 macOS 上交叉编译 Windows ARM64 演练样本
set -euo pipefail
cd "$(dirname "$0")"

echo "== 静态检查 =="
if [ -n "$(gofmt -l .)" ]; then
    echo "gofmt 存在未格式化文件:"; gofmt -l .
    exit 1
fi
GOOS=windows GOARCH=arm64 go vet ./... && echo "vet 通过"

echo "== 编译 GUI 勒索样本(双击运行,无控制台) =="
CGO_ENABLED=0 GOOS=windows GOARCH=arm64 go build -trimpath -ldflags "-s -w -H=windowsgui" -o ransom_drill.exe .

echo "== GUI 还原器:同一二进制,按文件名 restore_drill.exe 自动进入还原 =="
cp ransom_drill.exe restore_drill.exe

echo "== 编译 CUI 教学版(控制台逐步日志,供演示者观察) =="
CGO_ENABLED=0 GOOS=windows GOARCH=arm64 go build -trimpath -ldflags "-s -w" -o ransom_drill_console.exe .

echo "== 产物 =="
file ransom_drill.exe restore_drill.exe ransom_drill_console.exe
ls -lh ransom_drill.exe restore_drill.exe ransom_drill_console.exe
