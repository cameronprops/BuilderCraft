#!/usr/bin/env python3
"""Validate cross-domain operation ordering and reuse contracts with the stdlib."""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PATH = ROOT / "docs/roadmap/CROSS_DOMAIN_KERNEL_DEPENDENCIES.json"
STATUSES = {"implemented_source", "partial_source", "authored_uncompiled", "planned"}
CONSUMERS = {"cad", "orbweaver", "metrology", "reverse_engineering", "fabrication"}


def check() -> None:
    data = json.loads(PATH.read_text(encoding="utf-8"))
    assert data["schema_version"] == 1, "unsupported registry schema"
    operations = data["operations"]
    assert operations, "dependency registry must be nonempty"
    entries: dict[str, dict] = {}
    for item in operations:
        ident = item["id"]
        assert isinstance(ident, str) and ident and ident not in entries, f"duplicate or empty operation ID {ident}"
        assert isinstance(item["tier"], int) and item["tier"] >= 0, ident
        assert item["status"] in STATUSES, ident
        assert item["consumers"] and set(item["consumers"]) <= CONSUMERS, ident
        assert len(item["requires"]) == len(set(item["requires"])), f"duplicate dependencies: {ident}"
        source = item["source"]
        if item["status"] == "planned":
            assert source is None, f"planned operation falsely claims source: {ident}"
        else:
            assert source and (ROOT / source).is_file(), f"missing claimed source for {ident}: {source}"
        entries[ident] = item

    for ident, item in entries.items():
        for dep in item["requires"]:
            assert dep in entries, f"{ident} requires unknown {dep}"
            assert entries[dep]["tier"] < item["tier"], f"{ident} has a non-ascending dependency on {dep}"

    marks: dict[str, int] = {}

    def visit(ident: str) -> None:
        state = marks.get(ident, 0)
        assert state != 1, f"dependency cycle at {ident}"
        if state == 2:
            return
        marks[ident] = 1
        for dep in entries[ident]["requires"]:
            visit(dep)
        marks[ident] = 2

    for ident in entries:
        visit(ident)

    sequence = data["next_validated_sequence"]
    assert len(sequence) == len(set(sequence)), "duplicate priority queue item"
    assert set(sequence) <= entries.keys(), "priority queue references unknown ID"
    print(f"Cross-domain dependency DAG valid: {len(entries)} operations, "
          f"{sum(len(x['requires']) for x in operations)} edges, {len(sequence)} queued.")


if __name__ == "__main__":
    check()
