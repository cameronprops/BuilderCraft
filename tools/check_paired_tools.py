#!/usr/bin/env python3
"""Static paired-operation consistency gate; no Rust compiler or paid CI required.

This catches missing CAD adapters, graph node IDs, registry entries and
dependency grouping drift before local compilation. It does NOT prove an
algorithm or a Grasshopper component passes runtime acceptance.
"""
from __future__ import annotations

import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def main() -> int:
    plan = json.loads((ROOT / "docs/dependencies/tool-groups.json").read_text())
    pairs = plan["paired_operations"]
    rust = (ROOT / "crates/kernel/src/shared_tools.rs").read_text()
    registry = (ROOT / "crates/kernel/src/registry.rs").read_text()
    engine = (ROOT / "crates/engine/src/cmd/worldwright_tools.rs").read_text()
    graph = (ROOT / "crates/calisoga/src/lib.rs").read_text()
    lock = (ROOT / "Cargo.lock").read_text()

    implementation = re.findall(
        r'SharedToolContract\s*\{\s*'
        r'operation:\s*"([^"]+)",\s*'
        r'cad_command:\s*"([^"]+)",\s*'
        r'calisoga_node:\s*"([^"]+)",\s*'
        r'dependency_group:\s*"([^"]+)"',
        rust,
    )
    docs = [
        (p["kernel_operation"], p["cad_command"], p["calisoga_node"], p["group"])
        for p in pairs
    ]
    if len(implementation) != len(pairs) or implementation != docs:
        raise SystemExit("Shared CAD/Calisoga Rust contracts differ from the dependency register.")
    for kernel, cad, node, _ in implementation:
        if f'id: "{kernel}"' not in registry:
            raise SystemExit(f"Kernel registry is missing {kernel}")
        if f'CommandSpec::new("{cad}"' not in engine:
            raise SystemExit(f"CAD command not registered: {cad}")
        if not node.startswith("calisoga."):
            raise SystemExit(f"Invalid Calisoga node namespace: {node}")
    if not all(s in graph for s in ("execute_shared_tool", "GRAPH_SCHEMA_VERSION", "InputBinding")):
        raise SystemExit("Calisoga does not delegate to the shared typed dispatcher.")
    if 'name = "calisoga"' not in lock:
        raise SystemExit("Calisoga workspace package missing from Cargo.lock.")
    if len(set(implementation)) != len(implementation):
        raise SystemExit("Duplicated CAD/Calisoga binding.")

    print(f"Paired tool contracts coherent: {len(pairs)} kernel ops, "
          f"{len(pairs)} CAD commands, {len(pairs)} Calisoga node IDs.")
    print("Static check only: Rust compilation, UI behavior and Grasshopper parity still pending.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
