# Worldwright mesh-scene vertical slice

This development branch connects native triangle/quad editing to the shared
headless Scene **and** the CAD document, command API and early desktop viewport.
The headless Scene and CAD document still have separate storage systems.

## Implemented in this increment

- `GeometryData::PolygonMesh` preserves editable native triangles/quads
  without destructive triangulation. The normal scene `GeometryBudget::retain`
  validates indices, finite coordinates and element counts, and counts vertex
  and polygon-face capacity before admitting a retained lease.
- `SceneCommand::EditPolygon(id, PolygonSceneEdit)` applies a selected polygon
  change in the existing `Scene::apply` transaction. It can delete selected
  faces, construct a triangle from a boundary edge and vertex, or fill one
  planar convex inner loop. Picks carry the source scene revision.
- Existing scene IDs, layers, hierarchy, snapshot restore, revision checks
  and shared geometry budgets are used. Invalid edits do not publish the
  partially edited scene. Retaining source and output simultaneously can
  legitimately reject an edit when the geometry budget is exhausted.
- The operation is synchronous; until cancellation is checked within the
  individual mesh algorithm, edited polygon meshes are capped at 100,000
  vertices and 100,000 faces inside `Scene::apply`.
- The scene manifest identifies these objects as `polygonMesh`.

## Not yet implemented

- Vertex and edge picking, NURBS surface occlusion and full depth-buffer
  picking, shaded mesh drawing, and robust large-scan viewport acceleration.
  Polygon **face** click selection and highlighting now exist on this branch.
- Editable quad export to independently validated OBJ/PLY or CAD interchange
  adapters is still pending; `.dftba` retains native quads internally.
- Advanced concave/nonplanar fills, self-intersection detection, large-scan
  streaming, preservation of material/UV/per-face attributes, and robust
  external OBJ/PLY/STL import.
- The shared Scene and current CAD Document are not yet one synchronized
  authoritative editable model. Do not claim an end-to-end mesh editor alpha.

## Acceptance commands (local, no paid GitHub Actions)

```sh
cargo fmt --all -- --check
cargo test --locked -p buildercraft-kernel --test mesh_scene
cargo test --locked -p buildercraft-kernel
cargo clippy --locked -p buildercraft-kernel --all-targets -- -D warnings
```

The above commands **must be executed on a machine with Rust 1.95 and the
project workspace available** before this new code is considered verified.
At time of authoring, repository writes were performed via GitHub's source
API, and no Rust compiler was available in the authoring container.

## Next vertical slice, in order

1. Run full local Rust 1.95 validation and resolve any compilation, format,
   lint, headless UI or geometry correctness failures.
2. Validate clicking a mesh face, deleting it, undoing, and saving/reopening
   `.dftba` in the actual desktop application (not just tests).
3. Add viewport edge/vertex picking and interactive boundary-loop selection,
   keeping source-document revision checks and geometric selection feedback.
4. Add robust OBJ/PLY/STL import, face-attribute preservation, mesh diagnostics
   and a hole-fill preview/commit flow.
5. Test/export the first complete mesh-repair fixture and package a desktop alpha.

Do not treat a headless kernel operation as a completed viewport command.

## CAD document integration increment

The same branch now adds native `Drawing.mesh3d` entries (copy-on-write
`PolygonGeometryObject` retaining triangles/quads). The optional `mesh3d`
array is persisted inside the existing version 1 `.dftba` JSON envelope
(with `.bcraft` still accepted as a legacy filename extension).
Old v1 projects without the key continue to deserialize. Invalid geometry,
duplicate IDs and over-budget meshes are rejected at load time; source meshes
are not silently dropped by DXF/DWG/PDF/PNG save or plot operations.

The shared CAD scene manifest now includes polygon objects and their existing
model-body ownership. GLB visualization derives a triangulated copy without
altering the saved polygon mesh.

### CAD API integration

The same desktop/CLI/API command engine now exposes:
- `mesh3d.create` with a name and native `PolygonMesh` payload
- `mesh3d.list` returning persistent mesh objects and revision
- `mesh3d.boundaries` returning revision-stamped diagnostic loops
- `mesh3d.preview` returning triangles and source polygon-face mappings
- `mesh3d.edit` routing an explicit `PolygonSceneEdit` through the shared
  pure geometry operation and native document undo
- `mesh3d.set` for object name/visibility

All three editing variants use the source document revision to reject stale
picks. No mesh edit is published until validation is complete.

### Early desktop controls

The modeling viewport now displays a bounded polygon wireframe. The Model
Browser includes mesh objects, their visibility/name, numeric face deletion,
and selectable planar hole-loop fill buttons. A built-in editable sample ring
can be created for an immediate smoke test. The wireframe viewport now supports
**face click selection**, nearest-depth selection among polygon meshes, revision-
bound selection highlights and a Delete Picked Face button. Edge and vertex
picking are still pending. The user controls
name/visibility and can undo saved edits with the existing CAD undo command.

### New acceptance path

Run with an installed local Rust toolchain:

```sh
bash tools/verify-worldwright-kernel.sh
cargo test --locked -p cadcraft-io -p cadcraft-engine -p cadcraft-ui-egui
cargo run -p cadcraft -- --sample
```

Then click **3D → New editable mesh**, select its entry in **Model Browser**,
open **Polygon mesh repair**, fill its inner loop, undo the operation, save a
`.dftba` file, reopen it, and use **Fit** to frame its vertices. Repeat a
second save after face deletion. Confirm the GLB visualization exporter shows
the same mesh and stable object ID.

No local compiler or desktop runtime was available to execute these gates
during this repository-editing session; keep the PR unmerged until they pass.

### Viewport face-picking increment

The 3D viewport uses `OrthoFrame` projection and tests native polygon triangles
and quads in screen space. It computes barycentric depth at the clicked point
and chooses the nearest polygon face; deterministic object/face IDs resolve
coplanar ties. The quad remains a quad in the document. Visible layer and
object flags are respected, and the same 15,000-face cap controls both drawing
and hit testing. Empty clicks clear the picked face; orbit/pan drag remains
separate from clicking. Selected native face edges are highlighted without
changing the underlying geometry.

The pick stores both the source document UUID-like session UID and its revision;
stale picks cannot silently mutate a newer scene or an unrelated open document
that happens to have the same geometry IDs and revision. The existing `mesh3d.edit` command handles Delete Face,
undo, and later `.dftba` persistence. Eight pure picking tests plus headless
viewport click, undo, and cross-document isolation tests are authored,
**not yet executed**. The picker
currently considers polygon meshes only: a NURBS surface in front will not
occlude a mesh, and shaded depth-buffer picking is future work.

Windows local validation: `powershell -ExecutionPolicy Bypass -File tools/verify-worldwright-kernel.ps1`.
Linux/macOS: `bash tools/verify-worldwright-kernel.sh`. Neither invokes
GitHub Actions; the PR remains draft until the tests pass.
