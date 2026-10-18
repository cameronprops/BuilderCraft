# WorldBuilder path commands: Rhino-style alpha slice

This incremental implementation adds three **native, non-mutating, headless** shared operations. The same Rust functions are used by WorldBuilder CAD commands, OrbWeaver nodes and the generic `worldwright.tool.run` endpoint.

| Kernel operation | WorldBuilder command | OrbWeaver node | Native status |
| --- | --- | --- | --- |
| `kernel.polyline.point_at_station` | `worldwright.polyline.point_at_station` | `orbweaver.polyline.point_at_station` | Implemented for polylines |
| `kernel.polyline.tangent_at_station` | `worldwright.polyline.tangent_at_station` | `orbweaver.polyline.tangent_at_station` | Implemented for polylines |
| `kernel.polyline.offset_xy` | `worldwright.polyline.offset_xy` | `orbweaver.polyline.offset_xy` | Partial, XY open-line miter only |

All distance inputs use the current document's linear unit. These functions do not create, bake, modify or duplicate the source geometry; their return values can be passed to downstream commands/nodes.

## Direct CAD command example

For a spline-derived polyline containing points `(0,0,0)`, `(10,0,0)`, `(10,10,0)`, station 15 evaluates to `(10,5,0)`. Invoke:

```json
{
  "command": "worldwright.polyline.point_at_station",
  "params": {
    "inputs": {
      "points": {"kind":"polyline","value":[{"x":0,"y":0,"z":0},{"x":10,"y":0,"z":0},{"x":10,"y":10,"z":0}]},
      "station": {"kind":"number","value":15}
    }
  }
}
```

Swap the command for `worldwright.polyline.tangent_at_station` to obtain a normalized directional vector. At exact internal vertices the tangent uses the incoming segment; this is not a continuously varying frame.

Offset by 1 unit to the left:

```json
{
  "command": "worldwright.polyline.offset_xy",
  "params": {
    "inputs": {
      "points": {"kind":"polyline","value":[{"x":0,"y":0,"z":0},{"x":10,"y":0,"z":0},{"x":10,"y":10,"z":0}]},
      "distance": {"kind":"number","value":1},
      "miter_limit": {"kind":"number","value":4}
    }
  }
}
```

The resulting polyline has coordinates `(0,1,0)`, `(9,1,0)`, `(9,10,0)`.

## Theme-park modeling uses

- **Ride / guided path stationing:** place a proxy vehicle or show marker at a known route distance.
- **Scenic walls and paths:** generate a rough parallel alignment from a centerline.
- **Set dressing:** provide station points and headings for iterative placement along a scenic path.
- **Railing / track preliminary layout:** inspect a simple planar offset, then replace with certified geometry and clearance validation.

## Explicit limitations / correctness boundary

- These functions only evaluate *linear 3D polylines*. Smooth splines or NURBS must use an explicit, tolerance-controlled approximation before these commands. Do not imply Rhino's full `Offset`/`EvaluateCrv` functionality.
- `offset_xy` offsets in the **XY plane**, preserving per-vertex Z. It requires an **open** polyline with nonzero XY-length segments. Curves with vertical plan segments, hairpin reversals or excessive miters return errors.
- Offset output is **untrimmed**, and global self-intersections are not detected or resolved. It is not a region operation, true variable-distance offset, wall footprint, clearance envelope or engineered track.
- Offset is capped at 10,000 vertices; input coordinate and station checks use existing kernel bounds. `miter_limit` is a ratio in [1,20].
- No NURBS offset, terrain draping, catenary/path curvature, superelevation, vehicle kinematics, stone packing or physical construction verification has been implemented here.
- `tangent_at_station` is discontinuous at corners and cannot independently establish a ride-safe dynamic orientation.
- UI affordances, interactive preview and document-baked geometry are **future integration work**, not part of these numeric commands.

## Next operations for useful Rhino-like themed entertainment parity

1. Native NURBS arc-length evaluation with tolerance guarantees, stable curve frames and nearest-point queries.
2. Curve/rail editing: join, split, trim, extend, rebuild, planar offset with intersection resolution, `Pipe`, `Sweep1` and `Sweep2` production qualification.
3. Editable object creation and undoable geometry bake from native operation outputs.
4. Ride envelope and scenic clearance tools with reliable obstacle/collision math; support designed path speed/turning/superelevation later.
5. Procedural rock/stone wall tool using splines, sampled terrain and validated stone placement, downstream of the shared path services.

Never copy proprietary Rhino/Fusion code. Port only appropriately licensed open-source algorithms, or write original implementations.
