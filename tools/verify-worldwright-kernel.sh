#!/usr/bin/env bash
# Local/offline validation only. No GitHub Actions or hosted runners.
set -euo pipefail
cd "$(dirname "$0")/.."

if ! command -v cargo >/dev/null 2>&1; then
  echo "Rust/Cargo not found. Install the Rust 1.95 toolchain locally, then rerun." >&2
  exit 127
fi

echo "[1/6] Rust format check"
cargo fmt --all -- --check

echo "[2/6] Worldwright mesh-scene integration tests"
cargo test --locked -p buildercraft-kernel --test mesh_scene

echo "[3/6] Complete shared kernel test suite"
cargo test --locked -p buildercraft-kernel

echo "[4/6] CAD document, file I/O, engine and UI integration tests"
cargo test --locked -p cadcraft-doc -p cadcraft-io -p cadcraft-engine -p cadcraft-ui-egui

echo "[5/6] Calisoga shared-tool and DAG evaluator tests"
cargo test --locked -p calisoga

echo "[6/6] Kernel Clippy warning gate"
cargo clippy --locked -p buildercraft-kernel --all-targets -- -D warnings

echo "Worldwright local kernel validation passed."
