# WorldWright mesh topology viewport overlay

This increment adds an **actual native egui viewport overlay** for the
original-Rust BMesh-inspired vertex-fan diagnostics and the headless
`mesh3d.topology` command introduced in draft PRs #37 and #38.

## Workflow

1. Select an editable polygon mesh in Model Browser.
2. Expand **Polygon mesh repair** and click **Inspect topology**.
3. The editor calls `mesh3d.topology` and stores its reported non-manifold
   vertex indices with exact drawing UID and document revision.
4. The orthographic 3D wireframe viewport shows magenta markers around those
   indexed vertices. A marker is drawn **only if its vertex is referenced by
   a face within the global visible-face budget**. The overlay is capped at
   2048 markers per mesh, and the panel warns if more are flagged.
5. Modify the drawing, undo, switch documents or re-open and the old markers
   stop appearing automatically because the UID/revision guard fails.
   **Inspect topology** recomputes them. **Clear highlights** removes them.

The overlay is UI-only `#[serde(skip)]`; it never changes model geometry,
`.dftba` document persistence, undo/redo or actual selection. Locked/hidden
mesh visibility continues to follow the viewport's existing drawing rules.
This is wireframe emphasis, not an occlusion-aware depth-buffer picker and
not a repair operation. Future work: click to subselect an individual
diagnostic vertex; edge/face defect overlays; transaction-aware repair
operations; explicit quad-preserving subdivision. Source license: no GPL
Blender source reused or translated.

## Acceptance

Tests cover defect highlight retrieval through the real CAD engine, source
revision and UID guards, no document mutation, serialization omission,
missing object failure preservation and no highlighted off-budget faces.
Cross-platform root workspace CI is required before merge.
