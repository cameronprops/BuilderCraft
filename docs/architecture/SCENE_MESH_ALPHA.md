# Worldwright mesh-scene vertical slice

This increment connects native triangle/quad editing to the existing **headless**
shared Scene, not yet to the graphical CAD document or viewport.

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

- CAD document `.bcraft` persistence of polygons, UI viewport drawing,
  picking, selection feedback, toolbar commands, undo through the CAD
  document (as opposed to shared scene snapshots), and export of editable
  quads through any specific file adapter.
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

1. First run the complete kernel tests. Resolve any compile, format, lint
   or geometric correctness failures locally.
2. Add `PolygonMesh` as a versioned `.bcraft` document shape with an
   explicit migration and round-trip test. Avoid breaking existing files.
3. Expose one CAD API edit transaction for a mesh object, carrying document
   identity, selection revision and provenance.
4. Render the selected mesh and boundary edges in the existing viewport;
   wire one selection mode and a Delete Faces action with undo and save/reopen.
5. Extend to interactive boundary-loop selection, a hole-fill preview/commit,
   mesh import and geometry diagnostics, then publish a packaged preview.

Do not treat a headless kernel operation as a completed viewport command.
