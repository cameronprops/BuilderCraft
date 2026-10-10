#!/usr/bin/env python3
"""Fail closed on broken WorldWright Rhino-parity and OrbWeaver alpha manifests."""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
catalog = json.loads((ROOT / "docs/benchmarks/orbweaver-alpha-demo-catalog.json").read_text())
parity = json.loads((ROOT / "docs/parity/rhino-object-and-brep-acceptance.json").read_text())
reuse = json.loads((ROOT / "docs/dependencies/BREP_AND_DEMO_BACKEND_CANDIDATES.json").read_text())

EXPECTED_DEMOS = {
    "basketweaver-lite", "basketweaver-full-datadam",
    "primer-point-attractor", "primer-weaving-data-trees", "primer-surface-morph",
    "wallabee-tree-builder", "engagement-ring",
}
EXPECTED_GEOMETRY = {
    "point", "line", "polyline", "curve", "spline", "nurbs_curve",
    "nurbs_surface", "polysurface", "brep", "mesh", "groups", "layers",
}

def unique(records, key):
    values = [item[key] for item in records]
    assert len(values) == len(set(values)), f"duplicate catalog {key}"
    return set(values)

assert catalog["schema_version"] == 1
assert catalog["status"] == "specification_only_not_executable", "do not claim executable demos without tests"
assert unique(catalog["demos"], "id") == EXPECTED_DEMOS
assert unique(parity["geometry_types"], "id") == EXPECTED_GEOMETRY
assert len(catalog["runner"]["all_gates"]) >= 8
assert all(item["required_components"] and item["inputs"] and item["outputs"] and item["gates"] for item in catalog["demos"])
assert len([d for d in catalog["demos"] if d["source_type"] == "independent_recreation_from_published_example"]) == 3
full = next(d for d in catalog["demos"] if d["id"] == "basketweaver-full-datadam")
assert "data_dam" in full["required_components"]
assert "manual_release_updates_only_dirty_subgraph" in full["gates"]
assert "brep_boolean" in next(d for d in catalog["demos"] if d["id"] == "engagement-ring")["required_components"]
assert "particle_solver" in next(d for d in catalog["demos"] if d["id"] == "wallabee-tree-builder")["required_components"]
assert all(t["status"] == "audit_pending" for t in parity["geometry_types"])
assert reuse["canonical_backend_decision"] == "UNDECIDED_PENDING_ACCURACY_TESTS"
assert len(unique(reuse["alternatives"], "name")) >= 8
assert all(candidate["url"].startswith("https://") and candidate["license"] and candidate["gate"] for candidate in reuse["alternatives"])
print("Demo spec gate: 7 prewired alpha webs, 12 Rhino object families, upstream candidate audit coherent.")
print("Spec check only: graph execution, BRep parity, GUI and compiled performance benchmarks remain pending.")
