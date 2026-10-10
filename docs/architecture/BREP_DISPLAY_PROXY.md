# Exact BRep to shaded/wireframe display proxy

**Status:** experimental headless display proxy on top of the standalone exact BRep worker. It has no CAD viewport binding yet; see [alpha interaction acceptance](../roadmap/ALPHA_INTERACTION.md).

The source of truth is a persisted OCCT binary BRep. The optional `tessellate` JSON request returns a separately derived **non-exact** viewport mesh. It never overwrites exact topology, face/trim definitions, analytic cylinders/spheres, or STEP exchange data.

```json
{"op":"tessellate","brep":"<base64 exact BRep>","linear_deflection":0.1,"angular_deflection":0.4}
```

Response: `ok=true`, `mesh={vertices:[[x,y,z],...],normals:[[nx,ny,nz],...],indices:[i0,i1,i2,...],face_ids_session_hex:["...",...],edge_chains:[[[x,y,z],...],...],exact:false}`.

- Normals come from exact underlying BRep surface derivatives; each BRep face remains a distinct smoothing group.
- `indices` are packed triangles for a depth-buffered shaded viewport, `edge_chains` are world-space polylines for a wire overlay. The mesh is **not** a valid substitute for NURBS/STEP/persistent BRep geometry.
- The Face ID strings are transient native-session OCCT IDs, **not** stable document face IDs. The CAD host must create revision-aware persistent mapping when caching a render proxy or handling subobject picks.
- Rejects bad deflections, nonfinite/degenerate normals, unsupported/budget-overrun meshes and invalid indices. A rejected display meshing request must not invalidate the persisted source BRep.
- Meshing uses absolute chord/angular deflection; source tolerance remains a separate native-kernel issue. This proxy is a development milestone, **not** alpha shaded-mode signoff until GPU depth, selection and mixed-geometry viewport integration pass.

Test fixtures include a closed six-face box with triangulated surface/edge chains, sphere radial normals, refusal of invalid deflection, native BRep roundtrip, and exact Boolean volume preservation.
