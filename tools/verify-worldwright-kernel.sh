#!/usr/bin/env bash
# Local/offline validation only. No GitHub Actions or hosted runners.
set -euo pipefail
cd "$(dirname "$0")/.."

if ! command -v cargo >/dev/null 2>&1; then
  echo "Rust/Cargo not found. Install the Rust 1.95 toolchain locally, then rerun." >&2
  exit 127
fi

echo "[1/4] Rust format check"
cargo fmt --all -- --check

echo "[2/4] Worldwright mesh-scene integration tests"
cargo test --locked -p buildercraft-kernel --test mesh_scene

echo "[3/4] Complete shared kernel test suite"
cargo test --locked -p buildercraft-kernel

echo "[4/4] Kernel Clippy warning gate"
cargo clippy --locked -p buildercraft-kernel --all-targets -- -D warnings

echo "Worldwright local kernel validation passed."
