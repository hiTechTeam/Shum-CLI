#!/bin/sh
# Build shum for Linux ARM64 and x64 in one ARM64 Docker container (x64 is
# cross-compiled, not emulated) and pack them into dist/linux/ as the
# archives install-linux.sh expects.
set -eu
cd "$(dirname "$0")/.."
out=$(pwd)/dist/linux
rm -rf "$out"
mkdir -p "$out"
image=shum-cli-builder:cross
# The builder image is made once; SHUM_REBUILD_IMAGE=1 forces a rebuild.
# SHUM_BUILDER_BASE starts it from an image that already has Rust.
if [ "${SHUM_REBUILD_IMAGE-0}" = 1 ] || ! docker image inspect "$image" >/dev/null 2>&1; then
    docker build --platform linux/arm64 --build-arg "BASE=${SHUM_BUILDER_BASE:-ubuntu:22.04}" -t "$image" scripts/linux
fi
# COPYFILE_DISABLE keeps macOS metadata files (._*) out of the copy.
COPYFILE_DISABLE=1 tar --no-xattrs --exclude=./target --exclude=./dist --exclude=./.git -cf - . |
docker run --rm -i --platform linux/arm64 \
    -v shum-cli-cargo:/root/.cargo/registry \
    -v shum-cli-git:/root/.cargo/git \
    -v shum-cli-target:/work/target \
    -v "$out:/out" \
    -e CC_x86_64_unknown_linux_gnu=x86_64-linux-gnu-gcc \
    -e AR_x86_64_unknown_linux_gnu=x86_64-linux-gnu-ar \
    -e CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-linux-gnu-gcc \
    "$image" sh -euc '
        mkdir -p /work && tar -xf - -C /work && cd /work
        for pair in arm64:aarch64-unknown-linux-gnu x64:x86_64-unknown-linux-gnu; do
            arch=${pair%%:*}; target=${pair#*:}
            cargo build --release --locked --target "$target"
            stage=$(mktemp -d)
            cp "target/$target/release/shum" LICENSE "$stage/"
            tar -C "$stage" --owner=0 --group=0 --numeric-owner -czf "/out/shum-linux-$arch.tar.gz" LICENSE shum
            rm -rf "$stage"
        done'
for arch in arm64 x64; do
    (cd "$out" && shasum -a 256 "shum-linux-$arch.tar.gz" > "shum-linux-$arch.tar.gz.sha256")
done
ls -l "$out"
