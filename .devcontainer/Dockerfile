# Reproducible Rust/Cargo host for the Worldwright workspace.
# Matches rust-toolchain.toml (Rust 1.95.0).
# This builds locally; no GitHub Actions service is required.
FROM rust:1.95.0-bookworm

ENV DEBIAN_FRONTEND=noninteractive

# Linux build prerequisites for CADCraft/Worldwright eframe/wgpu, native file
# dialogs, audio/USB backends and headless unit/integration test compilation.
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        git \
        curl \
        pkg-config \
        cmake \
        clang \
        libclang-dev \
        libasound2-dev \
        libudev-dev \
        libdbus-1-dev \
        libgtk-3-dev \
        libxkbcommon-dev \
        libwayland-dev \
        libx11-dev \
        libxi-dev \
        libxcursor-dev \
        libxrandr-dev \
        libgl1-mesa-dev \
        libvulkan-dev \
    && rm -rf /var/lib/apt/lists/*

# Install these into the image rather than in a one-off shell session.
RUN rustup component add rustfmt clippy \
    && rustup target add wasm32-unknown-unknown \
    && rustc --version \
    && cargo --version \
    && cargo fmt --version \
    && cargo clippy --version

ENV DEBIAN_FRONTEND=
WORKDIR /workspaces/worldwright
