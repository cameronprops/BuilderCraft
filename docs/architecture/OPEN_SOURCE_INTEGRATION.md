# Open-source integration register

Reviewed 2026-10-09 against upstream manifests, README scope and license files.
This follows Cameron's direction to use compatible open-source implementations
when they improve a concrete shared service. Non-MIT candidates are included.
Upstream performance/coverage claims remain claims until native fixtures and
benchmarks verify them. Adding a dependency alone does not count as integration.

## Existing main integrations extended in this increment

| Repository | Pinned version | Placement | Working subset |
|---|---|---|---|
| [robust](https://github.com/georust/robust) | 1.2.0 | L0 geometry, reused by existing mesh closest-point kernel | Adaptive orientation on projected triangles; floating-point cancellation regression |
| [stl_io](https://github.com/hmeyer/stl_io) | 0.11.0 | L3 IO through L4 command adapters | Bounded ASCII/binary STL decoding and binary STL encoding; explicit f32 error/loss reporting and collapse/reversal rejection |
| [tobj](https://github.com/Twinklebear/tobj) | 4.0.5, `use_f64`, default features disabled | L3 IO through L4 commands | Triangular OBJ decode without external material loading; native f64 OBJ encode; source geometry is never mutated |

These are Cargo dependencies, not copied branches or parallel geometry engines.
Cargo.lock pins checksums; published package VCS commits are recorded in the JSON
register. MIT notices are in `docs/licenses` and attribution in `ATTRIBUTION.md`.
Base64 0.22.1 is the transport helper, with its own MIT notice.

`worldwright.mesh.decode` / `worldwright.mesh.encode` return the existing kernel
`TriangleMesh`. Decoded geometry feeds `worldwright.mesh.closest_point` and the
existing mesh diagnostics/repair services. Mesh document editing already exists independently through `mesh3d.*`; this
exchange facade does not persist imported objects or provide a full Scan UI. Limits: 8 MiB input/output, 65,536 vertices/faces,
256 OBJ object/group records. Unknown/non-triangle geometry and external
material libraries reject explicitly. Units are caller-owned; metadata/index
losses are reported. This is per-call admission, not measured aggregate RAM.

The viewport's control-curve tool uses the construction-plane/snap commands
already merged on main. Snap and preview now share exact-evaluation cost
admission. Snapped line and control-curve creation share one native constructor,
layer/identity preflight and transaction path. No duplicate point solver remains.

## Existing indirect use

[Vello](https://github.com/linebender/vello) already supplies `vello_cpu` 0.1.0
through epaint 0.36.2. This is indirect rendering support, not a new Vello GPU
canvas integration or a measured speed improvement.

## Additional integrations already on main

`las` 0.11.1 supplies bounded uncompressed LAS previews through `las_preview`.
`fastnoise-lite` 1.1.1 supplies shared deterministic procedural noise. LAZ,
classification-preserving Scan resources and full terrain authoring remain deferred.
The existing dependency status register is `../dependencies/OPEN_SOURCE_REUSE_CANDIDATES.json`;
this register supplements it with source evidence and precise adapter scope.

## Not yet integrated

| Repository | Reviewed license | Next owner / reason it remains separate |
|---|---|---|
| [Curvo](https://github.com/mattatz/curvo) | MIT | Next shared surface-construction adapter. Compare rational weights, knot domains, continuity, degeneracies, tolerance, source preservation and cancellation before extrude/loft/revolve adoption. Keep native stored geometry authoritative. |
| [egui-snarl](https://github.com/zakarumych/egui-snarl) | MIT OR Apache-2.0 | OrbWeaver canvas after typed evaluator and graph persistence. Current manifest uses egui 0.36, matching the UI. Canvas actions must delegate to shared operations/bake transactions. |
| [Spade](https://github.com/Stoeoef/spade) | MIT OR Apache-2.0 | Constrained planar/UV triangulation, feeding existing mesh validation/topology. |
| [geo](https://github.com/georust/geo) | MIT OR Apache-2.0 | Planar polygon/offset/trim adapters with native tolerances and identity. Does not supply 3D solid booleans. |
| [geo-index](https://github.com/georust/geo-index) | MIT OR Apache-2.0 | Packed planar/GIS indexing. Its indexes are 2D; it must not replace a 3D scan nearest-point index. Existing rstar is available for 3D work. |
| [cadcore](https://github.com/YATSKOVSKYI/cadcore) | MIT | Evaluate complementary analytic sweeps/topology/STEP on independent fixtures. README claims do not establish native acceptance. Avoid creating a second document/identity engine. |
| [meshoptimizer](https://github.com/zeux/meshoptimizer) | MIT | C++ upstream. Portable bounded Rust adapter, source-index remapping and native/WASM/Haiku checks before previs optimization. |
| [xatlas](https://github.com/jpcy/xatlas) | MIT | C++ upstream. Isolated portable UV-atlas adapter with memory and UV/source-index fidelity checks. |
| [Truck](https://github.com/ricosjp/truck) | Apache-2.0 | Reopened under broader license preference; candidate complementary trimmed topology/STEP, not an unchecked wholesale kernel replacement. |
| [Parry](https://github.com/dimforge/parry) | Apache-2.0 | Reopened for 3D proximity/collision acceleration. Compare f64 configuration, resource/cancellation contracts and existing rstar options. |
| [Fidget](https://github.com/mkeeter/fidget) | MPL-2.0 | Reopened for bounded implicit fields/rockwork. Preserve its notices and source license; bytecode path for WASM, optional native JIT only after resource/teardown checks. |

Nothing in the deferred list is silently discarded. No newly enabled UI/tool
capability is claimed for these repositories. Their manifests and license-file
blob IDs are tracked in `OPEN_SOURCE_INTEGRATION.json` where available.

## Next dependency order

1. Shared curve derivatives, adaptive arc-length stations and rail frames, then Curvo/native exact surface construction from drafted curves.
2. Existing shared graph evaluator: document-scoped geometry references, project persistence and bake transactions, then egui-snarl canvas.
3. Planar triangulation/polygon adapters and appropriate 3D spatial acceleration.
4. Extend existing LAS previews into streaming attribute/provenance-preserving resources.
5. Terrain fields, implicit meshing and separately bounded visualization/UV work.

Main already uses `.dftba` with legacy `.bcraft` readability. This increment
preserves that project schema and format selection.

## Reconciliation with recent modeling work

Main `f9ba908` includes headless CAD/OrbWeaver arrays, planar PushPull and
projection/flow subsets, plus undoable `mesh3d.project` and
`mesh3d.flow_along_srf` document edits. `worldwright.tool.*` typed outputs remain
separate from implicit document insertion. General trimmed BRep/solid operations
and complete Rhino option parity remain separate acceptance work.

The authoritative next-work queue is [issue #26](https://github.com/cameronprops/BuilderCraft/issues/26)
and `../roadmap/IMMEDIATE_MODELING_OPENSCAD_AND_ALPHA_UI.md`: shared curve/rail
evaluation before Pipe/Sweep and BRep work, with command-line and customizable
UI acceptance. Document-scoped graph references and bake remain graph needs;
existing mesh Project/Flow persistence must not be reimplemented.
