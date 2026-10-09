#!/usr/bin/env bash
# Native Haiku technology preview. Intentionally does not claim a GUI build.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
# shellcheck source=packaging/env.sh
. "$ROOT/packaging/env.sh"

for tool in rustc cargo tar mktemp; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "error: missing $tool; install a Haiku-native Rust/Cargo toolchain and build utilities" >&2
    exit 2
  fi
done

if ! rustc --print cfg | grep -Fxq 'target_os="haiku"'; then
  echo "error: this release gate must execute with a Haiku-native Rust host" >&2
  echo "A Linux cross-compile is not a substitute for native test execution." >&2
  exit 2
fi
if ! rustc --print cfg | grep -Fxq 'target_arch="x86_64"'; then
  echo "error: currently supported technology-preview architecture is x86_64 Haiku" >&2
  exit 2
fi

cd "$ROOT"
echo "Building Haiku-native Worldwright CLI preview with $(rustc --version)"
cargo test --locked -p buildercraft-kernel
cargo test --locked -p cadcraft-cli
cargo build --locked --release -p cadcraft-cli

BINARY="$CARGO_TARGET_DIR/release/cadcraft-cli"
if [ ! -x "$BINARY" ]; then
  echo "error: expected native CLI binary not found: $BINARY" >&2
  exit 1
fi

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT
"$BINARY" --version
"$BINARY" sample bracket "$TMP_DIR/sample.dxf"
"$BINARY" info "$TMP_DIR/sample.dxf" > "$TMP_DIR/sample-info.json"
"$BINARY" convert "$TMP_DIR/sample.dxf" "$TMP_DIR/roundtrip.dxf"
"$BINARY" info "$TMP_DIR/roundtrip.dxf" > "$TMP_DIR/roundtrip-info.json"
test -s "$TMP_DIR/sample-info.json"
test -s "$TMP_DIR/roundtrip-info.json"

PACKAGE_NAME="Worldwright-Haiku-x86_64-v$VERSION-cli-preview"
STAGE="$TMP_DIR/$PACKAGE_NAME"
mkdir -p "$STAGE/bin"
cp "$BINARY" "$STAGE/bin/"
copy_docs "$STAGE"
if [ -f "$ROOT/NOTICE" ]; then cp "$ROOT/NOTICE" "$STAGE/"; fi
cp "$ROOT/packaging/haiku/README.md" "$STAGE/HAIKU.md"

ARCHIVE="$DIST/$PACKAGE_NAME.tar.gz"
tar -czf "$ARCHIVE" -C "$TMP_DIR" "$PACKAGE_NAME"
printf '%s  %s\n' "$(sha256 "$ARCHIVE")" "$(basename "$ARCHIVE")" > "$ARCHIVE.sha256"
echo "Validated Haiku-native CLI preview: $ARCHIVE"
echo "IMPORTANT: no Haiku CAD graphical desktop is included in this archive."
