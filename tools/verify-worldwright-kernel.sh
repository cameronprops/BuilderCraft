#!/usr/bin/env bash
# Local/offline validation only. No GitHub Actions or hosted runners.
set -euo pipefail
cd "$(dirname "$0")/.."

if ! command -v cargo >/dev/null 2>&1; then
  echo "Rust/Cargo not found. Install the Rust 1.95 toolchain locally, then rerun." >&2
  exit 127
fi

echo "[1/8] Dependency catalog/DAG check"
python3 tools/build_dependency_index.py --check

echo "[2/8] Paired-tool consistency check"
python3 tools/check_paired_tools.py

echo "[3/8] Rust format check"
cargo fmt --all -- --check

echo "[4/8] Worldwright mesh-scene integration tests"
cargo test --locked -p buildercraft-kernel --test mesh_scene

echo "[5/8] Complete shared kernel test suite"
cargo test --locked -p buildercraft-kernel

echo "[6/8] CAD document, file I/O, engine and UI integration tests"
cargo test --locked -p cadcraft-doc -p cadcraft-io -p cadcraft-engine -p cadcraft-ui-egui

echo "[7/8] OrbWeaver shared-tool and DAG evaluator tests"
cargo test --locked -p orbweaver

echo "[8/8] Kernel Clippy warning gate"
cargo clippy --locked -p buildercraft-kernel --all-targets -- -D warnings

echo "Worldwright local kernel validation passed."
