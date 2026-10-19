# Worldwright command needs: Retopology, Surfacing and Form Editing

**Status:** proposed command inventory and acceptance specification, not a claim of implementation or successful validation.
**Owner directive:** 2026-10-18.
**Tracking:** [Issue #99](https://github.com/cameronprops/BuilderCraft/issues/99).
**Dependencies:** [Mesh editor roadmap](../interactive-mesh-editor-roadmap.md), [BRep/exact solid tracker #26](https://github.com/cameronprops/BuilderCraft/issues/26), [shared CAD and OrbWeaver tool policy](../dependencies/README.md).

## Intended experience

Worldwright needs a precise, fluid hybrid of the surface-snapped quad creation of ZBrush's newer Retopo Brush and Maya Quad Draw; artist-friendly polygon/SubD construction; Rhino-style rational NURBS control-point modeling; and trustworthy BRep face edits. Editing modes are interchangeable in one scene. Reference scans/meshes remain untouched and are independently visible, frozen, selectable and measurable. Every result preserves its actual representation rather than silently converting precise geometry to display triangles.

Reference functionality only: [Maxon ZBrush Retopo Brush](https://help.maxon.net/zbr/en-us/Content/html/user-guide/3d-modeling/topology/Retopo/01-retopo.html), [Maya Quad Draw](https://help.autodesk.com/cloudhelp/2026/ENU/Maya-Modeling/files/GUID-35D650EE-C840-4E4B-8E5C-DC5FFE4DA53A.htm). No proprietary source, documentation text, icons or shortcuts should be copied.

### Proposed command groups

Identifiers below are **candidate stable IDs** to be reconciled against the existing registry. All are **NEEDS**; related legacy commands do not count as tested parity. Keep each command's alias, typed schema, CAD UI, headless API, OrbWeaver node, limitations, fixtures and evidence in the eventual registry.

| Group | Candidate commands | Required behavior |
| --- | --- | --- |
| Retopology drawing | `retopo.draw_quads`, `retopo.draw_strokes`, `retopo.place_vertices`, `retopo.preview_faces` | Place/snapped dots, sketch connected edges across a high-resolution live reference surface, preview and explicitly commit tri/quad faces without destructive triangulation |
| Retopology construction | `retopo.extend`, `retopo.bridge`, `retopo.stitch`, `retopo.patch`, `retopo.grid_fill` | Extend existing border loops, weld with guarded tolerance, stamp adjustable topology patch templates, fill orderly quad grids; preserve source sculpt |
| Retopology correction | `retopo.project`, `retopo.relax`, `retopo.guide_flow`, `retopo.symmetry`, `retopo.quad_remesh` | Closest/ray/UV projection policy, controlled signed normal offset, feature-curve/symmetry guides, local/full constrained remesh and deviation report |
| Mesh subelements | `mesh.vertex_move`, `mesh.vertex_slide`, `mesh.edge_slide`, `mesh.edge_loop_insert`, `mesh.edge_dissolve`, `mesh.face_inset`, `mesh.face_extrude` | Typed vertex/edge/face handles, numerical XYZ/UVN constraints, configurable weld rules, selection across strips/loops, procedural preview and stable topology remap |
| Mesh form/surface | `mesh.face_pushpull`, `mesh.edge_bevel`, `mesh.edge_crease`, `mesh.bridge_loops`, `mesh.subdivide`, `mesh.smooth_region` | Edge/face normal-aware edits, face extrusion and thickness, bevel width/segments/miter, SubD creases, quad-preserving subdivision, boundary-aware smoothing |
| SubD design | `subd.from_quads`, `subd.control_cage`, `subd.crease`, `subd.refine`, `subd.to_nurbs_fit` | Live subdivision/control-cage levels and creases; accurate surface fitting across patches with continuity/gap/tolerance reports. Reject unsafe exact-surface claims |
| NURBS surfacing | `surface.point_edit`, `surface.patch`, `surface.network`, `surface.rebuild`, `surface.refit`, `surface.match`, `surface.blend` | Move individual rational control points without losing weights/knots; patch/network surfacing; parameterization, degree, G0/G1/G2 matching and quantified fit error |
| BRep face modeling | `brep.face_pushpull`, `brep.face_offset`, `brep.edge_fillet`, `brep.edge_chamfer`, `brep.fillet_feasibility` | Exact trimmed BRep preflight and rebuild, constant/variable edge radii, tangent chains/corner transitions, offending feature diagnostics, source-preserving failures |
| Form deformation | `deform.soft_select`, `deform.proportional_edit`, `deform.cage`, `deform.lattice`, `deform.bend`, `deform.twist`, `deform.taper`, `deform.shear`, `deform.conform` | Handle-based move/rotate/scale with influence radius/curve, symmetry, volume/feature preservation options, local/world/camera normals, non-destructive history where supported |
| Precision/inspection | `surface.deviation`, `mesh.quad_quality`, `topology.inspect` | Compare rebuilt surface to reference using sampled/max/RMS deviation, face skew/aspect/planarity, watertight/normal/manifold and seam/feature continuity reports |

### Cross-cutting interaction requirements

1. **Live reference drawing:** high-poly scan, editable mesh, reference NURBS and trimmed BRep surface as independently selectable targets. X-ray wireframe/reference opacity, front/back hit selection and orthographic/perspective ray behavior are explicit; projected hits return face/surface ID, normal and UV as applicable.
2. **Accurate quad placement:** true editable quads; free point plot and click-to-complete faces; projected point/edge drag; border extrusion; edge loop insertion/deletion, slide and relax; snap to existing vertices/edges, projected guides, grid, world-axis and exact numerical distances/angles.
3. **Control-point manipulation:** edit individual NURBS CVs, weights and hulls, SubD cage points and polygon vertices with common gizmo/box/lasso/brush selections. Whole-object and sub-object modes are distinct. Respect locked axes, custom construction plane and numeric input.
4. **Soft and compound deformation:** configurable influence falloff (linear/smooth/gaussian or user curve), distance in Euclidean or geodesic space, symmetry plane and mirrored seam policy; numerical and sculpt-style handles use identical shared math.
5. **Push/Pull semantics:** distinguish polygon face normal extrusion, face region offset, curve/profile extrusion and exact BRep-face push/pull. For BRep, adjacent faces must extend/trim/intersect and repair as valid topology; no fake mesh-only BRep edit.
6. **Fillet transparency:** interactive edge-chain preview; accept constant and variable radii, corner solutions, continuity, selected edges, optional setbacks. Feasibility preflight reports safe interval or proven lower/upper search bracket, offending geometry, tolerance, self-intersection and clearance. **Never state a global maximum radius without actually establishing it.** The user can request a conservative auto-fit radius.
7. **Representation-aware conversion:** mesh/quad/SubD/BRep/NURBS and downstream print/realtime mesh outputs are separate with named conversion steps and measured fit. Native quad/SubD should not degrade to triangles until preview/required exchange. Reconstructed NURBS requires error and continuity validation.
8. **Uniform command execution:** same kernel work under direct CAD, API/CLI, headless tests and OrbWeaver; all mutations revision-checked, cancelable, undoable, validated and bounded in memory, with non-destructive previews and source-safe failures.

## Priority implementation sequence

- **P0 foundation:** editable `PolygonMesh` halfedge with 3+/quad face loops and stable IDs; reversible selection/remap; reference hit-testing, spatial indexes, cached preview, bounded undo, accurate constraints and mesh snapshots. Reuse the existing mesh editor roadmap and never duplicate kernel work.
- **P1 useful tool:** `retopo.place_vertices` -> `retopo.draw_quads` -> `retopo.extend` -> `mesh.vertex_move` -> `mesh.edge_slide` -> `retopo.project`. Deliver direct viewport gestures plus reproducible typed APIs.
- **P2 fast mesh/organic editing:** `retopo.patch`, `retopo.bridge`, `retopo.relax`, loop insertion/slide/dissolve, creases, subdivision, soft selection, proportional/cage deformation, local quad remeshing.
- **P3 precision surfaces:** rational NURBS CV edit; native SubD control cage and conversion with measurable fit; surface patch/network/rebuild/match, guided continuity and error heatmaps.
- **P4 exact solids:** complete BRep prerequisites of issue #26, then genuine BRep PushPull/FilletEdge/ChamferEdge with feasibility estimation and atomic topology repair. Mesh bevel/push-pull can ship earlier, **clearly separated**.
- **P5 mature editing:** variable radius corner fillets, region-to-region SubD/BRep transitions, advanced deformation, history edits, very large scan streaming and performance QA.

## Acceptance scenes and gates

- Scan-derived irregular rock surface: lay down a 4x5 quad patch with exact point snapping, bridge a boundary, locally relax while maintaining ridge tags and check deviation to the scan.
- Organic face/creature sculpt: place and revise edge loops around a feature, mirror across a plane, edit multiple vertices with falloff, subdivide, preserve quad topology and export editable geometry.
- Manufactured panel: create a planar face, numerically push/pull a specified distance, inset an opening, bevel mesh edges, separately fillet exact BRep edges when the kernel allows.
- Fillet-limiting test: radii below/near/above a geometry-specific transition; report last verified valid radius, invalid bracket, confidence and reason. Reject topological failure without changing the source.
- NURBS patch control: alter selected CVs and weights, preserve valid knots, test G0/G1/G2 edge joins within unit-scaled tolerances, and quantify fitted patch error.
- Degenerate/non-manifold/reversed faces, nonplanar/bowtie quads, cut UV seams, mirrored symmetry with centerline, disjoint scan targets, million-polygon references, memory pressure and cancellation.
- Tests require actual Rust compilation plus JSON and UI command flows, undo/redo and save/reopen on supported platforms before any entry changes from NEEDS to working/partial.

**Scope boundary:** this document records needed operations; it neither increments operational-command counts nor asserts automated certification. Keep existing verified features and their recorded status untouched.
