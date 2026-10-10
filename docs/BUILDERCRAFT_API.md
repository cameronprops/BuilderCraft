# BuilderCraft alpha API

The inherited JSON-line control channel dispatches the same commands as the desktop and CLI. Protocol details: control-protocol.md. The native app enables it explicitly with `--control PORT`, on loopback only. No public HTTP endpoint or network collaboration is included.

Example request, one JSON object per line:

```json
{"id":1,"method":"engine.execute","params":{"command":"model.create","params":{"kind":"assembly","name":"Fountain"}}}
```

Use the returned model ID as `parent` when creating a component or body.

| Command | Parameters |
|---|---|
| buildercraft.capabilities | API/schema version and implemented geometry capabilities |
| visualization.publish | project_id, directory; native bounded GLB + full scene snapshot, persistent publication sequence; no drawing mutation |
| production.model | returns native production records, bindings and links |
| production.set | records, bindings, links; validated, undoable replacement of production organization |
| geometry3d.preview | id; optional curve_segments (1–4096), surface_u and surface_v (1–128); bounded uniform polyline/triangle data plus source revision, no edit |
| kernel.manifest | project_id (32 hex digits), optional geometry_budget_bytes; metadata-only 3D scene projection, no edit |
| model.create | name, kind (assembly/component/body), optional parent ID and hex entity handles; bodies use current selection if handles are absent |
| model.list | returns organization nodes |
| model.rename | id, name |
| model.select | id; selects descendant geometry |
| model.visible | id, visible |
| nurbs.curve | XY control points, degree, optional positive weights |
| nurbs.curve3d | name, curve with degree, XYZ control objects, weights and knot vector |
| nurbs.surface | name, surface with compatible curve rows, degree_v and knots_v |
| geometry3d.list | returns exact 3D control geometry |
| geometry3d.set | id, optional name and visibility |
| geometry3d.controlpoint | id, optional row, index, point [x,y,z] |
| mesh3d.create | name, mesh with vertices and native triangle/quad faces |
| mesh3d.list | current document revision and native mesh objects |
| mesh3d.boundaries | id; revision-stamped closed loops and ambiguous edges |
| mesh3d.preview | id; non-mutating triangle view and original polygon-face mapping |
| mesh3d.edit | id, edit with kind, selected_revision and selected indices/loop |
| mesh3d.set | id, optional name and/or visible |

3D curve example:
```json
{"id":2,"method":"engine.execute","params":{"command":"nurbs.curve3d","params":{"name":"Arch","curve":{"degree":2,"control":[{"x":0,"y":0,"z":0},{"x":5,"y":0,"z":10},{"x":10,"y":0,"z":0}],"weights":[1,1,1],"knots":[0,0,0,1,1,1]}}}}
```

Commands are discoverable through the inherited command catalog. IDs remain stable within this alpha; a formal version-negotiated integration SDK, change subscriptions and structured transfer diagnostics are planned. Scripts should use command results and query the document after mutations rather than assuming success. API integration does not automatically synchronize external Rhino/Autodesk documents.

Shared kernel usage, identity mapping and current bridge limitations: [KERNEL.md](architecture/KERNEL.md).

Native live visualization commands:

- `visualization.start {project_id,directory}` starts a background feed for the current document, immediately submitting its unsaved drawing.
- `visualization.status {}` reports running/pending/busy, document UID, published sequence and source revision, and error.
- `visualization.stop {}` cancels and joins the worker. Stop the previous feed before starting another.

Edits coalesce into one pending drawing. Native command completion and undo/redo submit automatically; direct document-mutating integrations call `Session::poll_visualization`. The desktop also polls after interactive commands finish. The feed follows its starting document rather than the active tab. These commands do not provide bidirectional engine edits or show control yet.

Desktop camera commands: `ui.buildercraft.top`, `ui.buildercraft.front`, `ui.buildercraft.right`, `ui.buildercraft.iso`, and `ui.buildercraft.fit`. All use orthographic projection and return camera center and scale through the UI command API. Fit targets visible 3D control hulls, excludes hidden layers, and rejects empty/nonfinite/over-budget hulls without changing the camera. Shift-drag pans in the camera plane. Camera changes do not modify CAD geometry or document undo history.

`geometry3d.transform {ids:[id,...],operation:{kind,...},copy?:false}` applies the shared exact-transform kernel to at most 128 native curves/control surfaces. Operations: `move {delta:[x,y,z]}`, `rotate {origin:[x,y,z],axis:[x,y,z],angle_degrees}`, `scale {origin:[x,y,z],factor}`, `mirror {origin:[x,y,z],normal:[x,y,z]}`. All points are world coordinates in document units. Unknown options, missing IDs, duplicate IDs, invalid axes, unsupported scale factors and budget/coordinate overflow fail before changing the drawing. Copies get new IDs but no organization or production assignment. Edits retain IDs and are undoable. This is partial Rhino transform coverage, not full command parity.

### Scale object positions

This translates each exact curve/control surface from its geometric bounding-box
center without resizing it. Example (replace 42 with an existing object ID):

```json
{"command":"geometry3d.transform","params":{"ids":[42],"operation":{"kind":"scale_positions","origin":[0,0,0],"factor":2,"mode":{"kind":"three_d"},"tolerance":0.000001},"copy":false}}
```

For 1D use `mode:{"kind":"one_d","axis":[1,0,0]}`; for 2D use
`mode:{"kind":"two_d","normal":[0,0,1]}`. Tolerance is an absolute bounds
accuracy request in model units, with a floating-point rounding allowance.
The shared work budget rejects cases that do not converge. See
[the full contract](commands/MANUAL_REBUILD.md#scalepositions-increment).

### Exact shear

```json
{"command":"geometry3d.transform","params":{"ids":[42],"operation":{"kind":"shear","origin":[0,0,0],"direction":[1,0,0],"normal":[0,0,1],"angle_degrees":45},"copy":false}}
```

The normal defines the fixed plane; direction lies in that plane. Signed angle
is strictly between -89 and 89 degrees. Exact curves/control surfaces only.
Unsupported options are rejected. Copy, atomic undo/redo and persistence apply.

### Three-point orientation

```json
{"command":"geometry3d.transform","params":{"ids":[42],"operation":{"kind":"orient3pt","source":[[0,0,0],[1,0,0],[0,1,0]],"target":[[10,20,30],[10,22,30],[9,20,30]],"scale":true},"copy":false}}
```

Scale defaults to false; when true it uses only the first-edge length ratio.
The third point defines plane orientation. Exact curves/control surfaces only.
Invalid/near-collinear triples and unsupported options reject atomically.

## Worldwright editable polygon mesh API

New native polygons are retained as triangle or quad faces. The document stores
them inside the existing versioned `.dftba` project (legacy `.bcraft` supported) (optional `mesh3d` field).
Triangle conversion for preview and GLB export does not overwrite native quads.

Example creation:

```json
{"command":"mesh3d.create","params":{"name":"Floor patch","mesh":{"vertices":[{"x":0,"y":0,"z":0},{"x":1,"y":0,"z":0},{"x":1,"y":1,"z":0},{"x":0,"y":1,"z":0}],"faces":[{"quad":[0,1,2,3]}]}}}
```

Query `mesh3d.boundaries {"id":123}` to retrieve `source_revision`
and `report.closed_loops`. For numeric face deletion:

```json
{"command":"mesh3d.edit","params":{"id":123,"edit":{"kind":"delete_faces","selected_revision":4,"selected_faces":[0]}}}
```

The `123` ID and `4` revision above are illustrative: use the response
from create, and the current revision from list or boundary analysis. Other
edit kinds are `add_triangle_from_edge` (edge_vertices, point_vertex) and
`fill_planar_hole` (loop_index). Invalid or stale edits do not modify the
document or consume an undo step. The initial UI offers numeric face indices,
click-to-select polygon faces with revision-aware highlighting, and listed
boundary loops. Viewport edge and vertex picking are still pending.

Native meshes are not silently exported to unsupported 2D file formats. Use
`.dftba` for editable persistence (legacy `.bcraft` reads still work); GLB is a derived visualization, not an
editable quad-mesh interchange format. All new integration tests require local
Rust execution before the implementation can be claimed verified.


## Paired native CAD/OrbWeaver tool API (source authored)

The new numeric CAD/API commands and OrbWeaver nodes both delegate to
`buildercraft_kernel::execute_shared_tool`. No per-interface geometry
algorithm is duplicated. No active drawing or undo transaction is needed to
calculate a pure point/vector/polyline result.

`worldwright.tool.list {}` enumerates the first ten shared native tool
contracts, including typed ports, modifier flags and kernel prerequisites.

Direct CAD distance command:

```json
{
  "command": "worldwright.point.distance",
  "params": {
    "inputs": {
      "a": {"kind":"point","value":{"x":0,"y":0,"z":0}},
      "b": {"kind":"point","value":{"x":3,"y":4,"z":12}}
    }
  }
}
```

The equivalent generic command:

```json
{
  "command": "worldwright.tool.run",
  "params": {
    "operation": "kernel.point.distance",
    "inputs": {
      "a": {"kind":"point","value":{"x":0,"y":0,"z":0}},
      "b": {"kind":"point","value":{"x":3,"y":4,"z":12}}
    }
  }
}
```

Both use the **same** typed dispatcher and return `output` as a tagged
`ToolValue` (`number`, `count`, `point`, `vector`, or `polyline`).
An OrbWeaver node has the component ID `orbweaver.point.distance`; node ports
accept `{"source":"constant","value":{...}}` literals or
`{"source":"output","node":<upstream node ID>}` links. The graph
schema is version 1 and deterministic for supported scalar-valued nodes.

The initial modifier ports are `t` for interpolation, `count` for
polyline division by count, and `spacing` for division by distance. These
are named typed settings, not duplicate geometry solvers. Exact graph
list/tree matching, reference Grasshopper option equivalence, and graph
document bake/persistence are future milestones. Inputs reject unknown ports,
wrong kinds, nonfinite values, and invalid domain/spacing policies.

See `docs/dependencies/` for the group-level hierarchy and all 46 native
kernel operation DAG nodes; `crates/orbweaver/examples/paired_distance.rs`
for an executable headless equivalence demo. Run the local validation scripts
before marking any new code tested.


## OrbWeaver tree operations (source authored; Rust validation pending)

The same native data-tree services can be called as pure Worldwright CAD/API
commands or wired as OrbWeaver graph nodes. These five new paired commands
extend the prior ten numeric operations:

| Worldwright command | OrbWeaver node | Inputs | Output |
|---|---|---|---|
| `worldwright.tree.validate` | `orbweaver.tree.validate` | tree | Top-level item count |
| `worldwright.tree.flatten` | `orbweaver.tree.flatten` | tree | One ordered branch |
| `worldwright.tree.graft` | `orbweaver.tree.graft` | tree | Item-indexed child branches |
| `worldwright.tree.simplify` | `orbweaver.tree.simplify` | tree | Simplified branch paths |
| `worldwright.tree.match` | `orbweaver.tree.match` | a, b, mode | Tree of pairs |

Typed tree values are tagged `{"kind":"tree","value":{"branches":[...]}}`.
Branches contain path arrays and tagged items. Matching needs identical
branch paths, and a typed `match_mode` modifier of `shortest`, `longest`,
or `cross_reference`. Longest repeats the last item; CrossReference produces
a Cartesian product. These are explicitly defined native rules, **not a
claim of exact implicit Grasshopper tree-matching behavior**.

Example direct CAD/API flatten request:

```json
{
  "command": "worldwright.tree.flatten",
  "params": {
    "inputs": {
      "tree": {
        "kind": "tree",
        "value": {
          "branches": [
            {"path": [0, 1], "items": [{"kind": "number", "value": 2}]},
            {"path": [0, 2], "items": [{"kind": "number", "value": 4}]}
          ]
        }
      }
    }
  }
}
```

The equivalent OrbWeaver graph uses node ID `orbweaver.tree.flatten`
with a `Constant` binding carrying the same tagged tree, or an `Output`
binding connecting it to another tree node. Results use `ToolValue::Tree`.
Generated paired values use `ToolValue::Pair`, and output/error checks are
shared by both interfaces.

Native trees preserve empty branches, require ordered unique paths of depth
1–16, and enforce count/clone limits. Graph-level `.dftba` persistence,
geometry handle ports, exact Grasshopper implicit path matching and graphical
editing are still pending. Native tree-item broadcasting is available for the
ten point/vector/polyline operations; use the top-level optional `matching`
modifier (`shortest`, `longest`, `cross_reference`) on CAD commands, or the
per-node `matching` field on OrbWeaver graph nodes. See `crates/orbweaver/examples/paired_tree.rs`. Local compilation
and runtime tests have not yet been performed.

## Optional scoped parametric feature histories

The command engine now has source-authored `worldwright.history.create`,
`worldwright.history.edit`, `worldwright.history.list`,
`worldwright.history.inspect` and `worldwright.history.evaluate`.

A scope can be `{"kind":"document"}`,
`{"kind":"model_node","id":42}` or
`{"kind":"block_definition","id":"Bracket"}`. History edits carry the
scope, `expected_revision` and a tagged `change`:
`append`, `set_parameter`, `set_input`, `set_suppressed`,
`reorder` or `set_rollback`. The evaluator returns typed kernel values
but does **not** bake geometry. Undo uses the normal CAD snapshot mechanism.

The native `.dftba` envelope has an optional `feature_timelines` field.
Existing version-1 documents without the field still load. Unsupported
operations such as a future `kernel.solid.extrude` are rejected, not
silently recorded as functional features. See
[feature-history examples and rules](architecture/FEATURE_HISTORY.md).

## Viewport picking scope

CAD click and Shift-click use the same `geometry3d.pick` and `geometry3d.select`
commands available to headless controllers. A plain empty click clears selection;
Shift-click toggles a hit and preserves selection on a miss. Picking uses the
same 96 curve segments and 13-by-24 surface isocurve wires as display, not exact
curve intersections or filled surface interiors. Within a pixel tolerance, the
nearest projected wire wins; coincident wires prefer camera-facing depth, then
the lowest stable object ID. No occlusion, face/edge/vertex subobject selection,
window selection, or snapping is claimed.

Hidden objects/layers and locked layers are excluded. Queries admit at most 4096
objects and 50 million conservative evaluation work units. Exceeding the scene
budget returns an error without applying a partial selection. Rendering also
uses this aggregate work limit and can stop before drawing the complete scene.
