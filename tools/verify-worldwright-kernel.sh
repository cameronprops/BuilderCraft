#!/usr/bin/env bash
# Portable native compiler / workspace smoke gate (no GitHub Actions needed).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
for tool in rustc cargo; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "error: $tool unavailable; install native Rust toolchain first" >&2
    exit 2
  fi
done
rustc --version
cargo --version
cargo fmt --all -- --check
cargo test --locked -p buildercraft-kernel
cargo test --locked -p cadcraft-cli
if [[ "${1:-}" == "--desktop" ]]; then
  cargo build --locked -p cadcraft
fi
echo "Worldwright native smoke gate passed (GUI not exercised)."
