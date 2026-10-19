# Geometry constraint intelligence — first native CAD implementation

WorldWriter must **explain modeling limits before attempting destructive changes**. A Rhino-style tool should not merely show "Fillet failed": it should offer a bounded, reproducible feasibility search over user-specified geometric parameters.

This initial slice is an **actual OpenCascade feasibility probe**, not a heuristic guessed from nearest edge lengths. It calls the same existing isolated native worker used by production BRep construction. Each parameter candidate runs separately, allowing worker crashes or timeouts to be reported without modifying the document.

## New commands

| Command | Investigated parameter | Inputs |
| --- | --- | --- |
| `brep.constraints.fillet` | Selected-edge fillet radius | `id`, `edge_indices` (empty means all), `radius_min`, `radius_max` |
| `brep.constraints.pipe` | Pipe radius | `rail`, `up`, `radius_min`, `radius_max` |
| `brep.constraints.sweep1` | Convex swept section uniform scale | `rail`, `section`, `up`, `scale_min`, `scale_max` |
| `brep.constraints.sweep2` | Cross-section height scale (Y only) | `rail_a`, `rail_b`, `section`, `scale_min`, `scale_max` |
| `brep.fillet` | Actual exact-solid edge fillet | `name`, `id`, `radius`, `edge_indices` |

Optional arguments for all probes: `probe_count` (3..12; default 6), `refine_count` (0..6; default 3), `selected_revision` (CAD revision consistency), and `tolerance`. Fillet and pipe radii, and sweep multipliers, must be positive and finite in the requested window.

**Important:** for two-rail Sweep2 the section's X coordinate already anchors between rails at x=0 and x=1. Only its local *height* (Y) is scaled. Scaling X would detach sections from the rails.

### What it reports

Every probe returns an ordered record of tested parameters, success/failure, and the backend's rejection reason:

- `highest_tested_feasible`: greatest **actually tested** successful candidate, never a guessed radius.
- `suggested_value`: a value that passed an actual OpenCascade trial, not an extrapolated threshold.
- `nearest_tested_failure_above`: smallest failing value that was actually tested above the best passing value, if any.
- `status`: `no_success_in_tested_range`, `sampled_transition_only`, or `range_cap_reached`.
- `nonmonotonic_samples_detected`: whether any sampled failure was followed by a higher successful probe.
- `maximality_proven:false`, `advisory_only:true`, and `geometry_modified:false`.

**No global maximum is claimed.** CAD kernels may have disconnected feasible parameter intervals, tolerance-dependent failures, and topological jumps. A bounded search does not mathematically establish the absolute maximum admissible fillet radius. Even if a selected radius succeeds, it is still subject to modeling tolerance, changed geometry, edge selection, native worker version, and assembly clearances. The report intentionally records trial evidence instead of creating false certainty.

## Example: ask the tool before filleting

```json
{
  "command": "brep.constraints.fillet",
  "params": {
    "id": 123,
    "edge_indices": [],
    "radius_min": 0.1,
    "radius_max": 2.5,
    "probe_count": 8,
    "refine_count": 4
  }
}
```

An empty `edge_indices` selects all edges. Nonempty selections refer to current native BRep edge enumeration indices and are *transient*, not persistent topological names. They must be rechecked after any edit that changes the solid.

Then pass the returned `suggested_value` to the real exact BRep `brep.fillet` command:

```json
{
  "command": "brep.fillet",
  "params": {
    "name": "Rounded scenic enclosure",
    "id": 123,
    "radius": 0.35,
    "edge_indices": []
  }
}
```

The source BRep is preserved and the successful result is a new native exact BRep document object, with existing undo and save/STEP workflows.

## Error taxonomy: current and next milestones

**Now:** actual native feasibility pass/fail observations and the underlying OpenCascade error text. Technical failures (unavailable worker, process termination, timeouts or malformed output) are not silently classified as geometric impossibility.

**Next:** structured diagnostics for local thickness, short neighboring edges, offset self-intersection, profile twist, inconsistent rail alignment, concave section and tolerance. The diagnostic engine should ultimately estimate and display a **safe band** as distinct from the last passing tested value, visualize the offending region, and let the user choose an alternative approach.

These tests only establish constructibility **for a specific exact solid operation**. They do not check show-set fabrication tolerance, mechanical clearances, ride-envelope safety, structural load, or accessibility.

### Engine architecture

- A single OpenCascade worker performs all native geometric operations. No second geometry kernel is introduced.
- Each candidate is probed through the established one-shot worker invocation; it cannot mutate the document.
- Search budgets are strict: no more than 12 initial candidates and 6 transition refinements. All results are sorted.
- Invalid candidate inputs, stale object selection, unavailable worker and unrecoverable processing errors fail closed.
- The exact fillet implementation shares the same source BRep as the preflight operation, without copying its algorithms into Rust.
