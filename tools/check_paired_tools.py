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
    graph = (ROOT / "crates/orbweaver/src/lib.rs").read_text()
    lock = (ROOT / "Cargo.lock").read_text()

    implementation = re.findall(
        r'SharedToolContract\s*\{\s*'
        r'operation:\s*"([^"]+)",\s*'
        r'cad_command:\s*"([^"]+)",\s*'
        r'orbweaver_node:\s*"([^"]+)",\s*'
        r'dependency_group:\s*"([^"]+)"',
        rust,
    )
    docs = [
        (p["kernel_operation"], p["cad_command"], p["orbweaver_node"], p["group"])
        for p in pairs
    ]
    if len(implementation) != len(pairs) or implementation != docs:
        raise SystemExit("Shared CAD/OrbWeaver Rust contracts differ from the dependency register.")
    for kernel, cad, node, _ in implementation:
        if f'id: "{kernel}"' not in registry:
            raise SystemExit(f"Kernel registry is missing {kernel}")
        if f'CommandSpec::new("{cad}"' not in engine:
            raise SystemExit(f"CAD command not registered: {cad}")
        if not node.startswith("orbweaver."):
            raise SystemExit(f"Invalid OrbWeaver node namespace: {node}")
    if not all(s in graph for s in ("execute_shared_tool", "GRAPH_SCHEMA_VERSION", "InputBinding")):
        raise SystemExit("OrbWeaver does not delegate to the shared typed dispatcher.")
    if 'name = "orbweaver"' not in lock:
        raise SystemExit("OrbWeaver workspace package missing from Cargo.lock.")
    if len(set(implementation)) != len(implementation):
        raise SystemExit("Duplicated CAD/OrbWeaver binding.")

    # Validate every registered kernel operation and its lower-level tool DAG,
    # not just the ten paired entrypoints.
    index = json.loads((ROOT / "docs/dependencies/kernel-operation-deps.json").read_text())
    entries = index["entries"]
    indexed = {entry["id"]: entry for entry in entries}
    registered = set(re.findall(r'^\s*id:\s*"(kernel\.[^"]+)"',
                                registry, re.MULTILINE))
    if len(indexed) != len(entries) or set(indexed) != registered:
        raise SystemExit("Kernel operation DAG differs from registered source operations.")
    group_defs = {entry["id"]: entry for entry in plan["groups"]}
    stages = {}

    def inspect(operation: str) -> None:
        if stages.get(operation) == 1:
            raise SystemExit(f"Cyclic native tool prerequisite: {operation}")
        if stages.get(operation) == 2:
            return
        stages[operation] = 1
        current = indexed[operation]
        current_group = group_defs.get(current["group"])
        if current_group is None:
            raise SystemExit(f"Unknown dependency group: {current['group']}")
        for lower in current["depends_on"]:
            if lower not in indexed:
                raise SystemExit(f"Unknown native operation prerequisite: {lower}")
            lower_group = group_defs[indexed[lower]["group"]]
            if lower_group["tier"] is not None and current_group["tier"] is not None:
                if lower_group["tier"] > current_group["tier"]:
                    raise SystemExit(f"Prerequisite tier order is reversed: {lower} -> {operation}")
            inspect(lower)
        stages[operation] = 2

    for name in indexed:
        inspect(name)

    native_deps = re.findall(
        r'SharedToolContract\s*\{\s*operation:\s*"(kernel\.[^"]+)"'
        r'[\s\S]*?prerequisites:\s*&\[([^\]]*)\]',
        rust,
    )
    if len(native_deps) != len(pairs):
        raise SystemExit("Shared kernel tool dependency metadata is incomplete.")
    for operation, items in native_deps:
        declared = re.findall(r'"(kernel\.[^"]+)"', items)
        if declared != indexed[operation]["depends_on"]:
            raise SystemExit(f"Tool prerequisites differ from kernel DAG: {operation}")

    # Specialized Scan/CAD/OrbWeaver reuse must not introduce duplicate math engines.
    from check_metrology_reuse import validate_metrology
    proposed, adapters = validate_metrology(ROOT, registered)
    print(f"Metrology reuse dependencies coherent: {proposed} planned shared kernels; {adapters} planned app adapters.")

    print(f"Native kernel dependency graph coherent: {len(entries)} operations.")
    print(f"Paired tool contracts coherent: {len(pairs)} kernel ops, "
          f"{len(pairs)} CAD commands, {len(pairs)} OrbWeaver node IDs.")
    print("Static check only: Rust compilation, UI behavior and Grasshopper parity still pending.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
