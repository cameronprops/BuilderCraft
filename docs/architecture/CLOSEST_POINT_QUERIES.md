# Native 3D closest-point queries

The first shared closest-point command set supplies original Rust geometry
operations usable by Worldwright CAD, future OrbWeaver nodes and
scan/metrology/reverse-engineering workflows. No licensed GOM, PolyWorks or
Design X runtime is required.

## Commands

Both commands run through the regular headless `Session::execute` API.

```json
{
  "command": "worldwright.mesh.closest_point",
  "params": {
    "mesh": {
      "vertices": [
        {"x": 0, "y": 0, "z": 0},
        {"x": 2, "y": 0, "z": 0},
        {"x": 0, "y": 2, "z": 0}
      ],
      "triangles": [[0, 1, 2]]
    },
    "query": [0.5, 0.5, 3],
    "max_distance": 5
  }
}
```

It returns `hit.point`, unsigned Euclidean `hit.distance`,
`hit.triangle_index`, `hit.barycentric` and `hit.degenerate`.
A miss beyond `max_distance` returns `hit:null` rather than a misleading
nearest-face assertion.

`worldwright.cloud.closest_point` accepts `points:[{x,y,z},...]` and a
`query:[x,y,z]`, and returns the nearest sample's point, distance and
original `point_index`.

## Shared mathematical ownership

- `crates/geom/src/closest3d.rs`: nearest position on a finite triangle,
  with barycentric weights; degeneracies fall back to edges/vertices.
- `crates/kernel/src/closest3d.rs`: triangle-mesh and point-sample
  queries; stable original indexes, validation, resource budgets and
  cooperative cancellation; source data remains immutable.
- `crates/engine/src/cmd/closest3d.rs`: JSON API/command adapters with no
  new numerical solver.

This is an intentionally bounded **linear scan**. The 3D spatial
acceleration index is still a separate planned dependency. Per call,
there are at most 65,536 vertices and 65,536 triangle faces, or 65,536
point samples. Larger files need a streaming/validated import, revision-
bound cached BVH or KD-tree, and cancellable background queries before
full-resolution scan inspection can be supported.

Query results are unsigned geometric distances. They are **not**
inspection deviations, do not establish inside/outside classification or
metrology accuracy, and do not compute nearest points on exact NURBS.
Absolute units and coordinate frames remain caller-owned; metrology
adapters must preserve tolerance, source revision, datum calibration,
measuring-device uncertainty and data provenance.

## Integration gates

Authored tests cover interior barycentric projection, edge/vertex/degenerate
triangles, finite-input rejection, stable equal-distance ties, radius misses,
invalid topology, cancellation and non-mutating API commands. They require
native Cargo verification before claims of passing tests. The future
OrbWeaver component must delegate to this same kernel operation rather
than implement a second closest-point routine.

Cross-domain dependency catalog (built in parallel):
`docs/roadmap/CROSS_DOMAIN_KERNEL_DEPENDENCIES.json` on
`feature/3d-construction-plane-snaps` / PR #9.
