#!/usr/bin/env bash
# Quick local check for toolchain availability. Used by the dev container at
# creation time and useful to developers/agents in any checkout.
set -euo pipefail
cd "$(dirname "$0")/.."

for tool in rustup rustc cargo; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        printf 'Worldwright needs %s.\n' "$tool" >&2
        printf 'Use a VS Code Dev Container, or install rustup: https://rustup.rs/\n' >&2
        exit 127
    fi
done

rustup show active-toolchain
rustc --version
cargo --version
cargo fmt --version
cargo clippy --version

if ! rustc --version | grep -q '^rustc 1\.95\.0 '; then
    echo 'Expected Rust 1.95.0 from rust-toolchain.toml.' >&2
    echo 'Run: rustup toolchain install 1.95.0 --profile minimal --component rustfmt --component clippy --target wasm32-unknown-unknown' >&2
    exit 1
fi

if ! rustup target list --installed | grep -qx 'wasm32-unknown-unknown'; then
    echo 'Missing WebAssembly target. Run: rustup target add wasm32-unknown-unknown' >&2
    exit 1
fi

echo 'Worldwright Rust/Cargo toolchain is available.'
echo 'To validate source: bash tools/verify-worldwright-kernel.sh'
