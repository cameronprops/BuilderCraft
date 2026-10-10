# Adaptive hole filling: nonplanar, averaged and directional planar caps

WorldWright's Mesh Repair UI and headless API use one shared kernel,
`polygon_mesh_fill_hole_with_mode`, for interactive preview and undoable
commit. The existing `fill_planar_hole` command remains backward compatible.

## Modes

- **Planar only**: original conservative operation, requires coplanar
  vertices (also supports simple concave boundaries via ear clipping).
- **Faceted/nonplanar**: triangulates in a stable oriented projection, using
  original 3D boundary vertices. Result is a stitched triangular patch whose
  facets need not share one plane; no scan vertex is moved.
- **Average best-fit plane**: computes a shared total-least-squares plane using
  the 3x3 symmetric covariance eigensolver in `cadcraft_geom::best_fit_plane`.
  Reports RMS and maximum boundary departure.
- **Automatic boundary direction**: plane through the arithmetic mean of the
  boundary positions, normal from their oriented area.
- **Custom plane direction**: caller supplies a normal; the operation aligns
  its winding with the hole and rejects near-edge-on normals.

The three flat-cap modes project an **inset ring** of new vertices to their
chosen plane and bridge the original 3D hole boundary to this new ring with
triangles. Only the inner cap is strictly planar. Its rim transition is
allowed to be nonplanar, preserving every source vertex and original face.

## API

Preview: `mesh3d.hole_preview` with `id`, `loop_index`,
`selected_revision`, and `mode`, e.g.:

```json
{"id":5,"loop_index":1,"selected_revision":42,
 "mode":{"mode":"best_fit_planar"}}
```

Commit: `mesh3d.edit` with `{"id":5,"edit":{"kind":"fill_hole",
"selected_revision":42,"loop_index":1,"mode":{"mode":"faceted"}}}`.

Allowed mode tags: `planar_only`, `faceted`, `best_fit_planar`,
`boundary_normal_planar`, `direction_planar`. The last requires
`"direction":{"x":0.0,"y":0.0,"z":1.0}`. The dry-run response includes
`new_vertices`, `new_triangles`, `source_vertex_count`, and
`cap_plane` diagnostics. There are no duplicate triangulators or extra
dependencies. All repair remains CPU and transactionally validated.

## Safety and next dependencies

Simple closed boundary only, currently at most 256 vertices. The kernel
rejects bad/stale picks, folded insets, degenerate projected triangles,
incorrect halfedge winding and certain invalid surrounding faces. In
particular **not all concave holes can safely accept an inset planar cap**;
the tool rejects these rather than promising repair. True nonplanar
surface fairing, curvature continuation, guaranteed 3D collision
detection, bridged/branched/open loops, large scan acceleration and
triangle propagation remain subsequent dependencies.

Cross-platform Rust CI (1.95) is required before merge.
