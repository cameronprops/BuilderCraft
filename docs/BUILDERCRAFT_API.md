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

3D curve example:
```json
{"id":2,"method":"engine.execute","params":{"command":"nurbs.curve3d","params":{"name":"Arch","curve":{"degree":2,"control":[{"x":0,"y":0,"z":0},{"x":5,"y":0,"z":10},{"x":10,"y":0,"z":0}],"weights":[1,1,1],"knots":[0,0,0,1,1,1]}}}}
```

Commands are discoverable through the inherited command catalog. IDs remain stable within this alpha; a formal version-negotiated integration SDK, change subscriptions and structured transfer diagnostics are planned. Scripts should use command results and query the document after mutations rather than assuming success. API integration does not automatically synchronize external Rhino/Autodesk documents.

Shared kernel usage, identity mapping and current bridge limitations: [KERNEL.md](architecture/KERNEL.md).
