# Shared geometry dependencies across CAD, OrbWeaver and metrology

Machine-readable contract: [CROSS_DOMAIN_KERNEL_DEPENDENCIES.json](../roadmap/CROSS_DOMAIN_KERNEL_DEPENDENCIES.json).

Worldwright is one geometry kernel with independently runnable CAD, OrbWeaver,
Scan/Metrology, reverse-engineering and fabrication applications. This is not a
copy of GOM, PolyWorks or Design X. Their public workflows motivate requirements,
not source reuse, binary dependencies or unverified format compatibility.

## Reuse before duplication

| Canonical kernel service | CAD / OrbWeaver use | Scan / metrology / reverse-engineering use |
|---|---|---|
| 3D vectors, units, datum frames, transformations | CPlanes, gizmos, orientation | Scan frame calibration, datum alignment and instrument units |
| Orthographic ray/plane and grid snapping | Interactive geometry placement | Datum/inspection-plane placement |
| Curve/surface evaluation | Sketches, surface editing and graph preview | Nominal reference and reconstruction |
| 3D nearest-neighbor, mesh closest point | Object snapping, repair and surface joining | Scan-to-CAD correspondence, deviation fields |
| Robust rigid best fit, ICP | Align command and parametric transforms | Scan registration, part-to-part inspection |
| Normals, topology and point-region segmentation | Modeling, selection and mesh cleanup | Scan groups by height, curvature, normal or region |
| Thickness/deviation/tolerance | Fabrication and manufacturability | Inspection maps, limits and reporting |

Current `crates/engine/src/spatial.rs` serves **2D drafting**. It is not
a 3D nearest-neighbor index and must not be stretched into a misleading
metrology feature. The 3D index is planned with a revision-bound, memory-capped
cache and source-identity preservation.

## Current 3D command increment

The new shared `crates/geom/src/snap3d.rs` supplies finite, rigid
construction planes, grid projection, screen-to-world rays and projected
pixel-distance evaluation. Three headless CAD commands use that math:

- `geometry3d.cplane.point`: project a pixel to a configurable construction
  plane; optional grid spacing
- `geometry3d.snap`: visible/unlocked exact NURBS endpoints,
  parameter midpoints and surface corners/edge sample candidates before a
  construction-plane or grid fallback
- `geometry3d.line`: resolve two queries from a single source revision and
  commit one undoable 3D line

The command wrappers are independent of the GUI and callable through the
normal API. Their Rust tests are authored but **not yet verified**. This
increment does **not** implement dense point-cloud snapping, a general
spatial index, exact nearest point on NURBS, signed scan deviations, or ICP.
A snap to a parameter midpoint is **not** an arc-length midpoint.

OrbWeaver's equivalent point-placement/snapping nodes should call the same
Rust geometry operations once its typed geometry-reference interface is
merged, not copy the CAD adapter. A metrology datum wizard can use this
construction-plane math now, but an inspected/registered datum needs
residuals, orientation confidence and calibration metadata that this
increment does not provide.

## Measured data contracts

Metrology results need explicit source identity and revision, coordinate
frame, units, tolerance values, provenance, confidence, and residual
statistics. Keep source scan and nominal CAD geometry immutable for
inspection; use separate derived geometry and atomically update
registrations. Bound processing work, avoid duplicating full point clouds
across UI/graph/report consumers and release revision-stale caches.

Regenerate/validate this register with:

```sh
python3 tools/check_cross_domain_dependencies.py
```

It checks tiers, unique IDs, resolved source paths, statuses, consumers, and
cycles. Rust compilation, geometry fixtures and CAD/Graph/Scan equivalence
are separate release gates. Future integration with the pre-existing
`docs/dependencies/kernel-operation-deps.json` index must reference its
canonical operation IDs rather than author another implementation.
