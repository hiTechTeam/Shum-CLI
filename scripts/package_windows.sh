#!/bin/sh
# Cross-build shum.exe for Windows x64 and ARM64 on macOS and pack them into
# dist/windows/ as the zip archives install.ps1 expects.
# Needs: cargo-xwin, Homebrew llvm and lld (clang-cl, lld-link), cmake, nasm.
set -eu
cd "$(dirname "$0")/.."
root=$(pwd)
export PATH="/opt/homebrew/opt/llvm/bin:/opt/homebrew/opt/lld/bin:$PATH"
out=$root/dist/windows
rm -rf "$out"
mkdir -p "$out"
for pair in x64:x86_64-pc-windows-msvc arm64:aarch64-pc-windows-msvc; do
    arch=${pair%%:*}
    target=${pair#*:}
    cargo xwin build --release --locked --target "$target"
    stage=$(mktemp -d)
    cp "target/$target/release/shum.exe" LICENSE "$stage/"
    (cd "$stage" && zip -q -X "$out/shum-windows-$arch.zip" LICENSE shum.exe)
    rm -rf "$stage"
    (cd "$out" && shasum -a 256 "shum-windows-$arch.zip" > "shum-windows-$arch.zip.sha256")
done
ls -l "$out"
