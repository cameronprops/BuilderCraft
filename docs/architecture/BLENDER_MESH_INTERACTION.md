# Next BMesh-style mesh interaction slice

This change extends the original Rust vertex-fan diagnostic from
[PR #37](https://github.com/cameronprops/BuilderCraft/pull/37), without
importing GPL Blender code.

## User-facing native commands

- `mesh3d.topology {id, include_all_vertices?:false}` returns the document's
  `source_revision`, a `selected_vertices` list of non-manifold indexed
  vertex IDs, detailed non-manifold face-fan reports, and anomalous edge IDs.
  When `include_all_vertices` is false, only faulty referenced vertex entries
  are expanded. Isolated vertices are reported separately.
- `mesh3d.edit {id, edit:{kind:"split_edge", selected_revision, edge_vertices:[a,b], fraction:0.5}}`
  splits an existing **triangle** boundary or interior edge, updating every
  incident triangle to avoid a T-junction. The original polygons are replaced
  in place; new triangle IDs and the appended vertex ID are available from the
  kernel's `PolygonEdgeSplitResult`. `fraction` is measured from the
  caller-supplied first endpoint. Rejects stale picks, invalid topology and
  incident quads. The document command commits one validated change so existing
  undo/redo works.

## Scope boundaries

- This is indexed polygon topology, not exact NURBS/B-rep topology.
- No Blender source was copied. Code is an independent Rust implementation of
  conventional triangle subdivision and graph adjacency.
- Vertex diagnostic selection is a headless command output; viewport
  highlighting is **not yet wired**.
- Quad subdivision must choose a deliberate quad-preserving policy rather than
  silently triangulating. This slice rejects quad edges until that policy exists.
- There is no geometric self-intersection test, edge-slide, bevel or certified
  watertightness guarantee. Source polygon file schema remains unchanged.

## Acceptance tests

Kernel: boundary/interior edge split, reversed endpoint interpolation, index
stability, no T-junction, manifoldness, invalid fractions, bad/stale picks,
quad refusal, atomic failures.

CAD engine: `mesh3d.topology` returns pickable revision-bound defects without
changing the document; `mesh3d.edit` split persists and supports native undo.

The next step is viewport inspection overlays, selection-handle handoff, and
native quad-edge subdivision with explicit per-face ID maps.
