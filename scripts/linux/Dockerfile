# Build environment for the Linux CLI, ARM64 natively and x64 by cross-
# compiling, so nothing runs under emulation. Ubuntu 22.04 (glibc 2.35)
# keeps the binaries working on newer distributions. libdbus is built from
# source (keyring's vendored feature), so no x64 system libraries are needed.
ARG BASE=ubuntu:22.04
FROM $BASE
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
        build-essential ca-certificates curl git pkg-config libdbus-1-dev \
        gcc-x86-64-linux-gnu libc6-dev-amd64-cross \
    && rm -rf /var/lib/apt/lists/*
ENV PATH=/root/.cargo/bin:$PATH
RUN command -v cargo >/dev/null || curl -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable
RUN rustup target add x86_64-unknown-linux-gnu
