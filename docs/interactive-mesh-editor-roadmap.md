# BuilderCraft Interactive Mesh Editor: CAD + Scan + Graph

## Product direction

Build an interactive polygon editor drawing on PolyWorks|Modeler workflows, with
first-class triangles, quads, and mixed polygon topology. This document is a
design target, not a claim that the commands below are already implemented.

Current kernel storage: `TriangleMesh { vertices: Vec<Vec3>, triangles: Vec<[u32;3]> }`.
Keep backward compatibility with this exchange/rendering representation. Do
not force all user editing through destructive triangulation.

## Core representation

Plan a new *editable* `PolygonMesh` built around indexed vertices, oriented
halfedges, edges, and faces with 3+ vertices per face. Maintain stable typed
selection handles, mesh revision IDs, and an old-to-new remap after topology
changes. Store explicit crease, sharp-edge, boundary, and feature-curve tags.
Normals, face adjacency, boundary loops and selection overlays are derived or
cached. A direct triangle view is generated for render/export and existing
kernel APIs, with source face mapping. Quads remain quads in edit documents.

Any topology mutation is a validated transaction with preview, undo/redo,
cancellation, change report, and error if manifold/winding requirements are
not met. Preserve the original until an edit commits.

## Selection

- Vertex, edge, face, boundary-loop and connected-component picking
- Point/edge/face cycling, additive/subtractive/toggle selections
- Rectangle, lasso, brush, grow/shrink, feature-angle and height-range selection
- Selection IDs resolved against mesh revision; never silently reuse stale IDs
- Snapping to vertices, edges, closest points, curves and reference planes
- Picking UI is an app-layer concern; kernel consumes validated selection IDs

## Interactive geometry creation

First slice: add oriented triangle from three existing vertex indices.
Next: edge + point, chained triangles, bridge two boundaries, split face,
cut edge, add point on mesh, delete face, retriangulate selection.
Every operation should surface a proposed preview, reject zero-area and
duplicate faces, enforce configurable manifold/winding policy and return
new-face IDs.

## Quad editing

- Create native quad from 4 corners, including nonplanar quads
- Bridge two ordered edge loops into quad strips (matching or resampled counts)
- Merge two triangles into a quad, with convexity/angle and shape tests
- Split quad into triangles only for export/preview, preserving native face
- Grid fill holes, loop cuts, edge slide, inset, extrude, quad remeshing
- Retopology drawing on scan surface: vertices snapped to high-density scan
- Subdivision: Catmull-Clark on quad/mixed polygon faces
- Surface correspondence and error heatmap relative to source scan
- Keep hard creases and feature curves through remeshing

## Manual vs automated repair

Existing: edge/face diagnostics, tolerance weld map, welding, duplicate
face detection, unused vertex compaction, basic repair orchestration.
Planned: manual vertex/edge/face actions, interactive hole patch preview,
curvature-constrained reconstruction, boundary smoothing, edge sharpness,
quad-preserving remeshing, normal orientation, non-manifold vertex repair,
self-intersection tests, thickness/watertight validation, scan fit deviation.

Automatic operations should support selected regions, not only whole meshes,
and support before/after reports.

## Implementation order

1. Triangle-add kernel primitive and tests on legacy TriangleMesh
2. Transaction/command result and typed selection/revision protocol
3. Edit PolygonMesh halfedge + face-loop topology (mixed tri/quad/n-gon)
4. Selection/hover overlay and preview in CAD/Scan UI
5. Triangle chain and manual patching based on picked vertices/edges
6. Native quad creation/merge and quad strips
7. Interactive fill + projected retopology, quad grid fill
8. Brush/region editing, subdivisions, remeshing and metrology metrics

## Quality gates

Check arbitrary winding, duplicate triangles, non-manifold edge sharing,
non-manifold vertices, repeated indices, collinear and tiny-area triangles,
nonplanar quads, bow-tie quads, holes with branches, large/scan-sized meshes,
undo/redo consistency, stable selection remapping, format round trips.
Benchmarks must cover millions of triangles and bounded job memory.

Never state feature parity with proprietary products without functional tests.
See PolyWorks public documentation and OpenMesh halfedge design for reference.
