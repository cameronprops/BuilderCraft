# Exact NURBS + OpenCascade construction: acceptance slice

**Status:** native integration in PR #69, pending CI and geometry validation. This slice upgrades WorldBuilder's core in two distinct representations, without promoting display meshes to BReps.

## Native rational NURBS curve operations

Three shared kernel operations now accept tagged `kind:"curve"` values with `degree`, `control`, `weights`, and `knots`. CAD command `worldwright.nurbs.*` and OrbWeaver `orbweaver.nurbs.*` dispatch the **same mathematical Rust code**.

| Command | Output | Representation and guarantee |
|---|---|---|
| `worldwright.nurbs.evaluate` | point | evaluates original weighted NURBS at normalized parameter t |
| `worldwright.nurbs.split` | two rational curves in a data tree | **shape-preserving** homogeneous knot insertion; works for multispan clamped degree-1..5 NURBS |
| `worldwright.nurbs.trim` | rational curve | two shape-preserving rational knot-insertion splits |

These are **parameter-based**, not length-based operations. The input curve must be valid under the existing native NURBS representation. Split requires an interior parameter, trim requires `0 <= start < end <= 1`. Knot insertion preserves the underlying curve up to floating-point rounding and does not approximate geometry by rebuilding control points.

Example NURBS request:

```json
{"command":"worldwright.nurbs.split","params":{"inputs":{
  "curve":{"kind":"curve","value":{
    "degree":2,
    "control":[{"x":0,"y":0,"z":0},{"x":2,"y":4,"z":0},{"x":4,"y":0,"z":0}],
    "weights":[1,0.5,1],
    "knots":[0,0,0,1,1,1]
  }},
  "t":{"kind":"number","value":0.35}
}}}
```

This does not yet perform generic NURBS join, tangent extrapolation, refit/rebuild with deviation bound, offset or surface conversion. The pre-existing polyline commands remain available.

## Exact OpenCascade solid operations

The same existing native OpenCascade worker now accepts **three new typed, bounded requests**. They produce authoritative serialized native BRep solids, held in document-owned `exact_breps` and handled by the established revision/undo, `.dftba`, STEP export and shaded-proxy workflows.

| Command | Algorithm | Guarantee and limits |
|---|---|---|
| `brep.pipe` | OCCT `Solid::sweep` of analytic circle | exact BRep tube along piecewise-linear rail; fixed radius |
| `brep.sweep1` | OCCT `Solid::sweep` of polygon profile | exact BRep solid; single constant, convex cross-section |
| `brep.sweep2` | OCCT `Solid::loft` with ruled station sections | exact BRep loft through sections constructed from two indexed rails |

**Exact BRep** describes the output surface topology and the OCCT construction. The input spines are still *piecewise linear*; Sweep2 is a ruled loft through sampled sections, not a general nonlinear two-rail Sweep2 with exact rail interpolation. The worker fails closed if native geometry is invalid and does not fall back to converting triangulated meshes into nominal solids.

### Example: exact Pipe

```json
{"command":"brep.pipe","params":{
  "name":"Show rail protective tube",
  "rail":[[0,0,0],[0,0,5]],
  "radius":0.25,
  "up":[0,1,0],
  "tolerance":0.000001
}}
```

`up` must be nonparallel to every rail segment tangent; abrupt near-reversals, zero-length segments, giant coordinates, invalid profiles and rails exceeding 128 stations are rejected. Output is **solid**, not hollow tubing. You can export the returned object ID through `brep.to_step` and inspect topology through `brep.inspect`.

### Fabrication and ride-design qualification

Neither the exact NURBS subset nor exact BRep outputs currently validate whole-model self-intersections, reliable topological naming after editing, geometric continuity through acute rail corners, vehicle sweep envelope, profile collision, drainage, structural performance or code compliance. These remain explicit acceptance gates, not silently assumed capabilities.

Run `cargo test --locked -p buildercraft-kernel`, `cargo test --locked -p cadcraft-engine`, and the isolated worker suite with `cargo test --manifest-path evaluations/cadrum/Cargo.toml -- --test-threads=1`. The worker tests depend on OpenCascade native libraries and run in a separate CI configuration.
