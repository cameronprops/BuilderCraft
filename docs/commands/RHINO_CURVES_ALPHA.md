# Rhino-style curve modeling: nine native alpha commands

This change extends the existing WorldWriter / WorldBuilder / SolidCraft shared Rust kernel, not a separate CAD engine. All nine operations have a single typed native implementation callable as a `worldwright.curve.*` command, `orbweaver.curve.*` node, or `worldwright.tool.run` generic request.

**IMPORTANT:** All nine operations are flagged **Partial** in the kernel registry because they do not yet match Rhino's full command contracts. No proprietary Rhino/Fusion code has been included. This package does not add toolbar buttons, bake geometry into the document, or make exact BRep solids. Outputs are pure data.

| Command | Inputs | Output | Limit |
|---|---|---|---|
| `join` | two polylines, tolerance | joined polyline | endpoint join only, optional reversal |
| `split` | polyline, absolute station | two polylines in a two-branch Tree | single interior split, not surface/object intersection |
| `trim` | polyline, begin/end stations | polyline | station interval only |
| `extend` | polyline, length, start flag 0/1 | polyline | tangent-line extension only |
| `rebuild` | polyline, controls, degree | genuine clamped NURBS Curve | control-fit approximation, not deviation-minimized Rhino Rebuild |
| `offset` | polyline, signed XY distance, miter limit | polyline | open XY miter offset, no self-intersection repair |
| `pipe` | polyline rail, radius, sides | PolygonMesh | capped faceted tube, not OCCT BRep or hollow pipe |
| `sweep1` | polyline rail, strictly convex planar cross section | PolygonMesh | one section fixed in transported local frame, end caps |
| `sweep2` | paired polyline rails, normalized convex planar section | PolygonMesh | equally indexed rail stations, cap mesh, no rail reparameterization |

All measurements use document units. The program explicitly fails on illegal parameter ranges, degenerate path geometry, incompatible rail station counts, or invalid/concave sections. Mesh outputs use native polygon topology and validation; no global self-intersection/clearance test is claimed. Path and section budgets bound allocations.

### Example input

```json
{
  "command": "worldwright.curve.pipe",
  "params": {
    "inputs": {
      "rail": {"kind":"polyline","value":[
        {"x":0,"y":0,"z":0},
        {"x":5,"y":0,"z":0},
        {"x":10,"y":2,"z":1}
      ]},
      "radius": {"kind":"number","value":0.25},
      "sides": {"kind":"count","value":24}
    }
  }
}
```

For `rebuild`, use `{"kind":"count","value":8}` for `controls`, and `{"kind":"count","value":3}` for `degree`. Result has `kind:"curve"` and NURBS control/weight/knot data.

For `sweep2`, the supplied section lives in local XY with **x=0 at rail A** and **x=1 at rail B**; local y is the vertical offset perpendicular to the rail midpoint direction. The cross-section's z must be zero. It is a coarse sectional preview and can fold or self-intersect at severely bending or reversing rails.

### Acceptance

- Rust unit tests for joins, splits, trims, extends, rebuilt NURBS control properties, XY offset, and topology of capped mesh sweeps.
- Engine tests compare direct CAD command and generic tool output for each of the nine commands, asserting non-mutating scene revision.
- Static dependency DAG and paired command catalog are updated.
- Native CI formats, checks, tests and lints the connected Rust workspace.

### Next gates to actual Rhino / themed-entertainment parity

1. Exact NURBS-aware Join, Split, Trim, Extend, fitting/rebuild and offset with error/tolerance reporting.
2. OpenCascade-based exact BRep Sweep1 / Sweep2 / Pipe with joins, rail section transitions, cap/joint management, consistent normals and stable topology.
3. Document-owned geometry bake, stable object IDs, undo, UI handles, construction tolerances.
4. Ride clearance envelopes, superelevation and curvature constraints, terrain-following rockwork and architectural scenic tooling.
