# Quad-preserving strip subdivision

Native Rust mesh edit: `polygon_mesh_split_quad_strip` / document operation
`mesh3d.edit` with `{"kind":"split_quad_strip","selected_revision":N,
"edge_vertices":[a,b],"fraction":0.5}`.

The operation traces the **entire quad strip** by following the opposite edge
of each encountered quad. The propagation reaches every face incident to every
split edge (including cyclic strips), therefore does not introduce T-junctions.
It interpolates one new indexed vertex per cut edge and divides every affected
quad into two quads. Original vertex IDs and old face IDs remain stable; new
faces are appended, not triangles. Deterministic sorted lists map the old
split-edge IDs to new vertex IDs and affected old face IDs to appended faces.

The propagator rejects invalid/nonmanifold source geometry, inconsistent
winding, mixed triangle/quad incidence along the affected strip, incompatible
fraction propagation through a closed cycle, overflow and stale pick revision.
It never partially mutates a document. Existing CAD scene undo and file roundtrip
preserve the native quads. Other isolated triangle components are untouched.

This is not Blender source-code reuse and does not replace exact CAD B-rep.
Not covered: corner-to-corner splits, multiple simultaneous perpendicular
strip cuts, cross-patch interpolation, true curved quad subdivision, certified
self-intersection detection or GPU-accelerated mesh editing.

Tests: standalone quad, two adjacent quads, cyclic strip, interpolation
direction, mixed topology rejection, stale selections, undo and persistence.
