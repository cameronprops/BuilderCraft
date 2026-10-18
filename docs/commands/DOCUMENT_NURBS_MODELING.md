# Update — bounded adaptive rail tolerance

The object-selected BRep commands now use adaptive **exact rational knot insertion** followed by a conservative, **one-sided curve-to-chord geometric deviation bound**. Instead of setting a fixed number of samples, pass `chord_tolerance` (default `0.01` document units) and optional `max_segments` (default 127, maximum 127). A legacy `segments` field is accepted as a **budget** for compatibility, no longer as a fixed count. Providing both fields is an error.

For every accepted clamped rational NURBS segment, its positive weights guarantee the curve lies inside its control-point convex hull. Each control point is measured to its corresponding chord. The maximum control-to-chord distance provides a conservative bound for all curve points, rather than just checking a few sample points. Both rails of Sweep2 are subdivided at **identical normalized parameter values** for correct paired station correspondence.

Results report `construction_spine:"adaptive_exact_knot_insertion_polyline"`, `spine_deviation_certified:true`, `spine_bound_kind:"one_sided_curve_to_chord_convex_hull"`, `max_curve_to_chord_bound`, `chord_tolerance`, `sample_segments`, and `max_segments`. The guarantee applies **only to source-curve-to-polyline deviation**, not inverse polyline-to-curve Hausdorff distance, surface deviation, twist/collision, or a sweeping manufacturing-tolerance certificate. `sweep_surface_deviation_certified:false` is explicit. Reject, without document mutation, if the tolerance cannot be met inside the budget, if the input is not a clamped positive-weight rational curve, or if the output has degenerate edges.

**Example:**

```json
{"command":"brep.pipe_from_curve","params":{
  "name":"Scenic rail","curve_id":123,"radius":0.125,"up":[0,0,1],
  "chord_tolerance":0.002,"max_segments":127
}}
```

This section supersedes the older *uniform-parameter sampling* notes further below. Document and OCCT worker behavior outside the rail sampling stage is unchanged.

---

# WorldWriter document-native NURBS editing and exact BRep construction

This iteration connects the previously implemented **pure geometric kernel** to the **persistent, undoable CAD document**. The kernel's rational NURBS math remains authoritative; no mathematical operation is reimplemented in the document layer.

## Exact rational NURBS object editing

| Command | Parameters | Result |
|---|---|---|
| `nurbs.split_object` | `id`, `t`, optional `selected_revision` | Two newly owned, editable NURBS objects with separate IDs; source preserved; one undo transaction |
| `nurbs.trim_object` | `id`, `start`, `end`, optional `copy`, `selected_revision` | An exact rational trimmed curve, copied by default or replacing the original while retaining its ID |

The trim and split operations use **homogeneous knot insertion**, preserving the shape of degree-1..5 rational NURBS curves, including multispan curves. Parameters are normalized to 0..1 and are **not** distances or stations. Both commands validate all geometry, selected revision, capacity, and names before making document changes. Copy operations preserve layer and visibility. Project save/load, undo and redo are covered in tests.

Example:
```json
{"command":"nurbs.split_object","params":{"id":123,"t":0.45,"selected_revision":99}}
```

## Object-selected exact BRep construction

| Command | Object inputs | Outcome |
|---|---|---|
| `brep.pipe_from_curve` | `curve_id`, `radius`, `up`, optional `segments` | Native OpenCascade pipe solid |
| `brep.sweep1_from_curve` | `curve_id`, `section`, `up`, optional `segments` | Native OpenCascade swept solid |
| `brep.sweep2_from_curves` | `curve_a_id`, `curve_b_id`, `section`, optional `segments` | Native OpenCascade two-rail ruled loft solid |

The workflow reads the **selected NURBS object** from the document, samples its points uniformly by normalized parameter, and then runs the native OCCT worker on the resulting **piecewise-linear spine**. The resulting shape is an **actual exact BRep for the sampled rail**, but **NOT** an exact analytic sweep of the original rational NURBS. This distinction is surfaced with explicit result fields:

- `construction_spine = "uniform_parameter_polyline_approximation"`
- `spine_deviation_certified = false`
- `sample_segments` records sampling density
- `associative = false`; editing the source curve does **not** rebuild the BRep
- `source_curve_id` (and `source_curve_b_id` for Sweep2) identify the selected inputs at construction time; they do not establish feature-history dependency links.

The default rail is 64 sample points for general curves, and a pair of exact endpoints for a two-control degree-1 line. Curved/multispan rail approximations must use 8..127 segments. Inputs are bounded by the existing OCCT worker. Degenerate samples, invalid orientations/sections, stale selection revisions or unavailable native worker return an error **without document mutation**.

Example:
```json
{"command":"brep.pipe_from_curve","params":{
  "name":"Scenic handrail",
  "curve_id":123,
  "radius":0.125,
  "up":[0,1,0],
  "segments":96
}}
```

Results go into the existing document `exact_breps` collection with undo support, native solid topology, inspect/STEP export and derived display previews. The shared memory and native-worker safety limits remain in effect. Polygon display geometry is never substituted for BRep source geometry.

## Qualification and future stages

- No adaptive error-bounded approximation or true OCCT weighted NURBS-edge input yet; the `segments` setting changes approximation density, not a certified chord error.
- No Rhino-style closed-curve join/trim by intersections; no topological naming history across successive edits.
- No automatic wall-thickness, structural, fabrication, ride-envelope or clearance guarantees.
- This increment is the first CAD **object integration** of the modeling functions; additional viewport grips and parametric feature relationships remain separate milestones.

Acceptance: the Rust project/engine tests cover NURBS creation, exact split/trim, save/load/undo/redo, malformed or stale selections; the OCCT worker-installed integration job additionally checks all three object-driven BRep commands and undo.
