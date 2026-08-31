#!/usr/bin/env bash
# 多平台交叉编译(产物输出 dist/<os>-<arch>/)
# 平台: windows/amd64 · windows/arm64 · darwin/arm64 · ubuntu(linux)/amd64 · ubuntu(linux)/arm64
set -euo pipefail
cd "$(dirname "$0")"

echo "== 静态检查 =="
if [ -n "$(gofmt -l .)" ]; then
    echo "gofmt 存在未格式化文件:"; gofmt -l .
    exit 1
fi
GOOS=windows GOARCH=arm64 go vet ./... && echo "vet 通过"

OUT=dist
rm -rf "$OUT"
mkdir -p "$OUT"
TARGETS=("windows amd64" "windows arm64" "darwin arm64" "ubuntu amd64" "ubuntu arm64")

for t in "${TARGETS[@]}"; do
    os="${t% *}"; arch="${t#* }"
    goos=$os; [ "$os" = "ubuntu" ] && goos=linux
    dir="$OUT/$os-$arch"; mkdir -p "$dir"
    ext=""; [ "$os" = "windows" ] && ext=".exe"
    extra=""; [ "$os" = "windows" ] && extra="-H=windowsgui"

    echo "== 编译 $os/$arch($goos) =="
    CGO_ENABLED=0 GOOS=$goos GOARCH=$arch go build -trimpath -ldflags "-s -w $extra" -o "$dir/ransom_drill$ext" .
    cp "$dir/ransom_drill$ext" "$dir/restore_drill$ext"   # 同一二进制,按文件名区分(restore = 还原)
    [ "$os" = "windows" ] && CGO_ENABLED=0 GOOS=$goos GOARCH=$arch go build -trimpath -ldflags "-s -w" -o "$dir/ransom_drill_console$ext" .
done

echo "== 产物 =="
for f in "$OUT"/*/*; do
    file "$f" | sed "s|^$OUT/|  dist/|"
done
echo ""
echo "== 说明 =="
echo "  每个平台目录包含: ransom_drill(勒索)+ restore_drill(还原,同一二进制按文件名区分);"
echo "  windows 目录另有 ransom_drill_console(.exe 控制台教学版)。"
