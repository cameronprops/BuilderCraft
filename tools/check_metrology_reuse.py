#!/usr/bin/env python3
"""Validate planned metrology dependencies against the implemented kernel register.

Pure planning validation. Passing does NOT indicate compilation, functionality,
precision, or any proprietary package's behavioral equivalence.
"""
from __future__ import annotations

import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def validate_metrology(root: Path = ROOT, registered: set[str] | None = None) -> tuple[int, int]:
    catalog = json.loads((root / "docs/dependencies/metrology-reuse.json").read_text(encoding="utf-8"))
    if catalog["schema_version"] != 1:
        raise ValueError("unsupported metrology dependency schema")
    if registered is None:
        source = (root / "crates/kernel/src/registry.rs").read_text(encoding="utf-8")
        registered = set(re.findall(r'^\s*id:\s*"(kernel\.[^"]+)"', source, re.MULTILINE))
    groups_data = json.loads((root / "docs/dependencies/tool-groups.json").read_text(encoding="utf-8"))
    groups = {entry["id"]: entry for entry in groups_data["groups"]}
    existing = catalog["existing_kernel_reuse"]
    seen_existing = [entry["id"] for entry in existing]
    if len(seen_existing) != len(set(seen_existing)) or set(seen_existing) - registered:
        raise ValueError("metrology existing-reuse IDs missing or duplicated in kernel registry")

    primitives = catalog["planned_shared_primitives"]
    primitive_map = {entry["id"]: entry for entry in primitives}
    if len(primitive_map) != len(primitives) or set(primitive_map) & registered:
        raise ValueError("planned primitives collide with registered kernel operations")
    consumers = {"cad", "orbweaver", "scan", "fab"}
    for entry in primitives:
        if not entry["id"].startswith("kernel.") or entry["status"] != "planned":
            raise ValueError("unimplemented metrology primitives must remain planned and namespaced")
        if entry["group"] not in groups or not entry["inputs"] or not entry["outputs"]:
            raise ValueError("missing metrology primitive group or typed I/O description")
        if not entry["acceptance"] or not {"scan", "cad", "orbweaver"}.issubset(set(entry["consumers"])):
            raise ValueError("metrology primitive lacks test criteria or shared consumers")
        if set(entry["consumers"]) - consumers:
            raise ValueError("unknown metrology primitive consumer")
    known_kernel = registered | set(primitive_map)
    for entry in primitives:
        for dependency in entry["depends_on"]:
            if dependency not in known_kernel:
                raise ValueError(f"unknown metrology kernel dependency: {dependency}")
            dep = primitive_map.get(dependency)
            if dep is not None:
                here, lower = groups[entry["group"]]["tier"], groups[dep["group"]]["tier"]
                if here is not None and lower is not None and lower > here:
                    raise ValueError(f"reversed tier dependency: {dependency} -> {entry['id']}")

    states: dict[str, int] = {}

    def visit_primitive(key: str) -> None:
        if states.get(key) == 1:
            raise ValueError(f"cyclic metrology kernel dependency: {key}")
        if states.get(key) == 2:
            return
        states[key] = 1
        for dep in primitive_map[key]["depends_on"]:
            if dep in primitive_map:
                visit_primitive(dep)
        states[key] = 2

    for key in primitive_map:
        visit_primitive(key)

    tools = catalog["planned_tool_adapters"]
    tool_map = {entry["id"]: entry for entry in tools}
    if len(tool_map) != len(tools):
        raise ValueError("duplicate metrology app adapter identity")
    for entry in tools:
        if entry["status"] != "planned" or entry["kind"] not in {"shared_adapter", "host_adapter"}:
            raise ValueError("unimplemented metrology tools must be marked planned")
        if not entry["acceptance"] or not entry["kernel_dependencies"]:
            raise ValueError("metrology tool is missing contract acceptance or shared dependency")
        if set(entry["kernel_dependencies"]) - known_kernel:
            raise ValueError(f"unregistered planned tool dependency: {entry['id']}")
        if set(entry["tool_dependencies"]) - set(tool_map):
            raise ValueError(f"unknown metrology tool adapter prerequisite: {entry['id']}")
        if set(entry["frontends"]) - consumers or not entry["frontends"]:
            raise ValueError(f"unknown metrology app interface: {entry['id']}")
    states.clear()

    def visit_tool(key: str) -> None:
        if states.get(key) == 1:
            raise ValueError(f"cyclic metrology app tool dependency: {key}")
        if states.get(key) == 2:
            return
        states[key] = 1
        for dep in tool_map[key]["tool_dependencies"]:
            visit_tool(dep)
        states[key] = 2

    for key in tool_map:
        visit_tool(key)

    return len(primitives), len(tools)


if __name__ == "__main__":
    primitives, adapters = validate_metrology()
    print(f"Metrology dependency planning coherent: {primitives} proposed shared kernels; {adapters} planned app adapters.")
    print("Planning contract only: no metrology implementations or vendor parity claimed.")
