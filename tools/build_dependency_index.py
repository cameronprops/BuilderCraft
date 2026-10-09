#!/usr/bin/env python3
"""Regenerate/verify Worldwright's preliminary dependency coverage index.

No third-party packages or network requests. This maps reference *metadata*
to broad dependency groups; it does not infer exact feature contracts or mark
any Rhino/Grasshopper/Kangaroo feature implemented.

Usage:
    python3 tools/build_dependency_index.py --check
    python3 tools/build_dependency_index.py --write
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
GROUPS = ROOT / "docs/dependencies/tool-groups.json"
INDEX = ROOT / "docs/dependencies/reference-index.json"


def read(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def validate_groups(plan: dict) -> None:
    groups = {entry["id"]: entry for entry in plan["groups"]}
    if len(groups) != len(plan["groups"]):
        raise ValueError("duplicate dependency-group ID")
    stages: dict[str, int] = {}

    def visit(key: str) -> None:
        if stages.get(key) == 1:
            raise ValueError(f"cyclic dependency group: {key}")
        if stages.get(key) == 2:
            return
        group = groups.get(key)
        if group is None:
            raise ValueError(f"missing dependency group: {key}")
        stages[key] = 1
        for dep in group["requires"]:
            dependency = groups.get(dep)
            if dependency is None:
                raise ValueError(f"unregistered prerequisite: {dep}")
            if group["tier"] is not None and dependency["tier"] is not None:
                if dependency["tier"] > group["tier"]:
                    raise ValueError(f"invalid tier: {dep} -> {key}")
            visit(dep)
        stages[key] = 2

    for key in groups:
        visit(key)
    for rule in plan["classification_rules"]:
        if rule["group"] not in groups:
            raise ValueError(f"unknown classification group: {rule['group']}")
    for pair in plan["paired_operations"]:
        if pair["group"] not in groups:
            raise ValueError(f"unknown tool dependency group: {pair['group']}")
    for key in ("kernel_operation", "cad_command", "calisoga_node"):
        values = [p[key] for p in plan["paired_operations"]]
        if len(set(values)) != len(values):
            raise ValueError(f"duplicate paired operation identity: {key}")


def infer(plan: dict, source: str, values: dict) -> str:
    for rule in plan["classification_rules"]:
        if rule["source"] != source:
            continue
        value = values.get(rule["field"])
        if not isinstance(value, str):
            continue
        subject = value.lower()
        target = rule["value"].lower()
        mode = rule["match"]
        matched = (
            subject == target if mode == "exact"
            else subject.startswith(target) if mode == "prefix"
            else target in subject if mode == "contains"
            else False
        )
        if not matched:
            continue
        if "category" in rule and values.get("category") != rule["category"]:
            continue
        if "name" in rule and values.get("name") != rule["name"]:
            continue
        return rule["group"]
    return "unclassified"


def build() -> dict:
    plan = read(GROUPS)
    validate_groups(plan)
    rhino = read(ROOT / "docs/commands/rhino8.json")
    manual = read(ROOT / "docs/commands/manual_inventory.json")
    components = read(ROOT / "docs/components/grasshopper1-kangaroo2.json")
    pairs = plan["paired_operations"]
    ref_pairs = {pair["reference_inventory_id"]: pair for pair in pairs
                 if pair["reference_inventory_id"]}
    rows: list[list[str | None]] = []

    for item in rhino["commands"]:
        paired = next((p for p in pairs if p["reference_name"]
                       and p["reference_name"].lower() == item["name"].lower()), None)
        group = paired["group"] if paired else infer(plan, "rhino_command", item)
        basis = ("paired_name_candidate" if paired else
                 "unclassified" if group == "unclassified" else "name_inferred")
        rows.append(["rhino_command", item["name"], group, basis,
                     paired["kernel_operation"] if paired else None])

    for item in components["entries"]:
        paired = ref_pairs.get(item["inventory_id"])
        group = paired["group"] if paired else infer(plan, "component", item)
        basis = ("paired_subset_candidate" if paired else
                 "unclassified" if group == "unclassified" else "category_inferred")
        rows.append([item["reference_family"], item["inventory_id"], group, basis,
                     paired["kernel_operation"] if paired else None])

    for item in manual["topics"]:
        filename = item["path"].rsplit("/", 1)[-1]
        topic_name = ""
        if item["topic_family"] == "commands":
            topic_name = filename.removesuffix(".html").removesuffix(".htm")
        group = infer(plan, "rhino_command", {"name": topic_name}) if topic_name else "unclassified"
        basis = "unclassified" if group == "unclassified" else "topic_name_inferred"
        rows.append(["rhino_manual_topic",
                     f"{item['platform']}:{item['path']}", group, basis, None])

    identities = [(row[0], row[1]) for row in rows]
    if len(set(identities)) != len(identities):
        raise ValueError("duplicate reference-index identity")
    group_counts = {g["id"]: 0 for g in plan["groups"]}
    for row in rows:
        group_counts[row[2]] += 1
    counts = {
        "rhino_commands": len(rhino["commands"]),
        "grasshopper1": sum(x["reference_family"] == "grasshopper1"
                            for x in components["entries"]),
        "kangaroo2": sum(x["reference_family"] == "kangaroo2"
                         for x in components["entries"]),
        "rhino_manual_topics": len(manual["topics"]),
        "total": len(rows),
        "unclassified": group_counts["unclassified"],
    }
    return {
        "schema_version": 1,
        "group_plan": "tool-groups.json",
        "caveat": ("Every category/name mapping is provisional; 0 entries are "
                   "dependency-verified by this heuristic index. Parent reference "
                   "files retain actual implementation status and behavior-review evidence."),
        "columns": [
            "source", "reference_id_or_key", "preliminary_group",
            "classification", "paired_kernel_candidate",
        ],
        "counts": counts,
        "preliminary_group_counts": group_counts,
        "rows": rows,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--check", action="store_true")
    mode.add_argument("--write", action="store_true")
    args = parser.parse_args()

    expected = build()
    if args.write:
        INDEX.write_text(json.dumps(expected, ensure_ascii=False, separators=(",", ":"))
                         + "\n", encoding="utf-8")
        print(f"Wrote {len(expected['rows'])} provisional reference entries to {INDEX}")
        return 0
    actual = read(INDEX)
    if actual != expected:
        print("Dependency reference index differs from its inventories/rules. "
              "Run with --write and review the resulting diff.")
        return 1
    print(f"Dependency index verified: {len(actual['rows'])} entries, "
          f"{actual['counts']['unclassified']} awaiting classification. "
          "All assignments remain provisional.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
