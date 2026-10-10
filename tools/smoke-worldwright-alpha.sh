#!/usr/bin/env bash
# Alpha smoke gate for the ACTUAL compiled desktop and headless applications.
# Runs on Linux CI after cargo build, or locally with Rust 1.95.
# Does not claim GUI rendering, mouse picking or graphical interaction passed.
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ "${1:-}" != "--already-built" ]]; then
  cargo build --locked -p cadcraft -p cadcraft-cli
fi
cli="./target/debug/cadcraft-cli"
desktop="./target/debug/cadcraft"
[[ -x "$cli" ]] || { echo "Missing compiled CAD CLI: $cli" >&2; exit 1; }
[[ -x "$desktop" ]] || { echo "Missing compiled native CAD: $desktop" >&2; exit 1; }

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

echo "[alpha] Compiled binaries and registry"
"$cli" --version > "$tmp/version.txt"
"$cli" commands > "$tmp/commands.json"
python3 - "$tmp/commands.json" <<'PY'
import json,sys
with open(sys.argv[1],encoding="utf-8") as f:
    rows=json.load(f)
assert isinstance(rows,list) and len(rows)>25, "missing executable command registry"
ids={item.get("id") for item in rows if isinstance(item,dict)}
required={"line","circle","undo","redo","move"}
missing=required-ids
assert not missing, f"missing alpha commands: {sorted(missing)}"
print(f"  executable registry: {len(ids)} commands")
PY

echo "[alpha] Native .dftba save and reopen, DXF conversion"
"$cli" sample floorplan "$tmp/floorplan.dftba"
test -s "$tmp/floorplan.dftba"
"$cli" info "$tmp/floorplan.dftba" > "$tmp/initial.json"
"$cli" run "$tmp/floorplan.dftba" --cmd 'drawing.inspect {"entities":false}' --save "$tmp/roundtrip.dftba" > "$tmp/inspect.json"
test -s "$tmp/roundtrip.dftba"
"$cli" info "$tmp/roundtrip.dftba" > "$tmp/reopen.json"
"$cli" convert "$tmp/roundtrip.dftba" "$tmp/floorplan.dxf"
test -s "$tmp/floorplan.dxf"
"$cli" convert "$tmp/floorplan.dxf" "$tmp/dxf-reopened.dftba"
test -s "$tmp/dxf-reopened.dftba"
"$cli" info "$tmp/dxf-reopened.dftba" > "$tmp/dxf-info.json"
python3 - "$tmp/initial.json" "$tmp/reopen.json" "$tmp/dxf-info.json" <<'PY'
import json,sys
docs=[]
for path in sys.argv[1:]:
    with open(path,encoding="utf-8") as f: docs.append(json.load(f))
assert all(isinstance(doc,dict) for doc in docs), "invalid inspect result"
print("  .dftba and DXF reopen: decoded JSON metadata")
PY

echo "[alpha] UI workspace transition and headless kernel integration"
cargo test --locked -p cadcraft-ui-egui workspace::tests
cargo test --locked -p buildercraft-kernel --test mesh_scene
cargo test --locked -p orbweaver
echo "[alpha] Compiled headless smoke passed; graphical usability remains a separate manual acceptance gate."
