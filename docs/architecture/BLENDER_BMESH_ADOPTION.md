# Blender BMesh-inspired mesh topology adoption — first slice

WorldWright already provides persisted native triangles and quads (`PolygonMesh`),
derived `PolygonTopology` halfedges, boundary-loop diagnostics, triangle exchange,
revision-bound edit operations and separate exact NURBS geometry. Do **not** replace
those with Blender's mesh object model or conflate polygon B-reps with exact CAD
B-reps.

## Proven upstream reference

Blender's [BMesh developer design](https://developer.blender.org/docs/features/objects/mesh/bmesh/)
describes *disk cycles* (edges around vertices), *loop cycles* (face corners),
*radial cycles* (faces around edges), and low-level invertible Euler edits.
[Blender BMesh header](https://github.com/blender/blender/blob/main/source/blender/bmesh/bmesh.hh)
is GPL-2.0-or-later. **No Blender source code was copied or translated in this
change.** This change independently implements a read-only algorithm based on
well-known indexed graph adjacency. If code from upstream is imported later,
preserve its license and copyright notices and recheck project-wide obligations.

## Implemented in this branch

`polygon_mesh_vertex_fans`: a deterministic, read-only diagnostic across
native polygon halfedges. Reports sorted edges, faces, and connected face-fans
for every referenced vertex; labels isolated vertices and non-manifold vertex
fans (including bow-tie/pinched vertices with entirely manifold edges).
Invalid edge multiplicity and winding are reported as non-manifold around their
endpoints. A single fan with either zero or two boundary spokes is accepted.
No source mesh mutation, triangulation, generated persistent selection IDs or
claims of geometric self-intersection testing.

Fixtures cover open/closed manifolds, joined quads, disconnected fans, invalid
edge multiplicity, inconsistent winding, deterministic results and invalid IDs.

## Logical next borrowing sequence

1. Feed vertex-fan failures into mesh diagnostics, repair previews, UI selection
   overlays and OrbWeaver's topology inspection block.
2. Make transactional BMesh-like edge split / inverse join primitives for native
   tri/quad storage. Return old-to-new ID maps and validate manifold policies;
   never quietly destroy quad identity or change persisted mesh schema.
3. Expand topology to n-gons and explicit per-corner UV/material attributes when
   serialization migration and format round-trips have acceptance tests.
4. Adapt viewport modal transforms and outliner interaction *behavior* through
   our existing egui, GPU, scene IDs, and transaction systems.
5. Audit Geometry Nodes instancing and field evaluation against OrbWeaver's
   typed engine; use exact NURBS/B-rep types as first-class inputs.
6. Evaluate Cycles as an optional renderer; do not block alpha wireframe/shaded.

No direct Blender dependency or GPL-source incorporation is implied by this
increment. The production OpenCascade B-rep evaluation is independent.
