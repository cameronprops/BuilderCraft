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


## Paired native CAD/Orb Weaver tool API (source authored)

The new numeric CAD/API commands and Orb Weaver nodes both delegate to
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
A Orb Weaver node has the component ID `orbweaver.point.distance`; node ports
accept `{"source":"constant","value":{...}}` literals or
`{"source":"output","node":<upstream node ID>}` links. The graph
schema is version 1 and deterministic for supported scalar-valued nodes.

The initial modifier ports are `t` for interpolation, `count` for
polyline division by count, and `spacing` for division by distance. These
are named typed settings, not duplicate geometry solvers. Exact graph
list/tree matching, reference Grasshopper option equivalence, and graph
document bake/persistence are future milestones. Inputs reject unknown ports,
wrong kinds, nonfinite values, and invalid domain/spacing policies.

See `docs/dependencies/` for the group-level hierarchy and all 41 native
kernel operation DAG nodes; `crates/orb-weaver/examples/paired_distance.rs`
for an executable headless equivalence demo. Run the local validation scripts
before marking any new code tested.
