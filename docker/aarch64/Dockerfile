# Cross-compiler image for building dirtbike_dash for a 64-bit Raspberry Pi
# (aarch64-unknown-linux-gnu). Debian bookworm matches Raspberry Pi OS Bookworm's
# glibc ABI. Slint's default backend (winit + GL/femtovg) needs its native deps
# cross-installed for arm64 too, hence the long apt list below.

FROM rust:1-bookworm

RUN dpkg --add-architecture arm64 \
    && apt-get update \
    && apt-get install -y --no-install-recommends \
        pkg-config \
        gcc-aarch64-linux-gnu \
        g++-aarch64-linux-gnu \
        libfontconfig1-dev:arm64 \
        libfreetype-dev:arm64 \
        libxkbcommon-dev:arm64 \
        libx11-dev:arm64 \
        libxcursor-dev:arm64 \
        libxrandr-dev:arm64 \
        libxi-dev:arm64 \
        libxext-dev:arm64 \
        libwayland-dev:arm64 \
        libegl1-mesa-dev:arm64 \
        libgl1-mesa-dev:arm64 \
        libgbm-dev:arm64 \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add aarch64-unknown-linux-gnu

ENV PKG_CONFIG_ALLOW_CROSS=1 \
    PKG_CONFIG_PATH=/usr/lib/aarch64-linux-gnu/pkgconfig \
    PKG_CONFIG_LIBDIR=/usr/lib/aarch64-linux-gnu/pkgconfig \
    CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc

WORKDIR /work
