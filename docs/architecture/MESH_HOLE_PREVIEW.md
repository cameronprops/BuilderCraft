# Interactive planar mesh hole preview

This is a **real repair-tool vertical slice** built on the existing Rust kernel,
not a separate triangulator or a decorative mockup.

1. Open WorldWright Mesh Repair and select a measured polygon mesh.
2. Choose **Pick hole boundary in viewport** and click an oriented boundary
   edge within ten screen points. Alternatively enter a loop index and click
   **Preview** for large/off-budget scenes.
3. The `mesh3d.hole_preview` headless command calls the existing
   `polygon_mesh_fill_hole` kernel with a required, revision-bound selection.
   The operation validates the proposed topology but never changes the document.
4. Preview triangles and their boundary appear in teal. **Commit fill**
   delegates to the existing transaction-based `mesh3d.edit`
   `fill_planar_hole` operation. Undo restores the original mesh.
5. Cancel or Escape clears transient picks. Document UID and revision guards
   prevent stale previews from being committed or rendered.

The picker reuses `mesh3d.boundaries` for the exact loop list; only a
screen-space point-to-segment query lives in UI. Selection respects mesh
visibility/locked state and conservative renderer/picker face budgets.
The preview is computed on demand, never every frame, and only returns new
triangle IDs rather than transferring a replacement mesh.

**Current kernel limit:** simple planar **convex or concave** inner boundaries
with at most 256 vertices. The shared kernel uses orientation-checked, quality-aware
bounded ear clipping, rejects self-intersecting/overlapping boundary edges and
collapsed edges, and checks area conservation and resulting halfedge winding.
Existing faces and vertices are preserved, and the same triangulation is used
in the Mesh Repair preview and the committed edit.

Curved/nonplanar or intersecting mesh surfaces, bridged openings, noisy scans,
and smoothing or unconstrained triangle propagation still require additional
algorithms and verified tests. Global 3D self-intersection certification is not
provided by the planar patch validator. GPU remains optional for display;
topology validation and mesh repair remain deterministic CPU operations.

Regression tests cover read-only preview/commit consistency, stale selection,
undo, and nearest-boundary picking. No new external dependency was introduced.
