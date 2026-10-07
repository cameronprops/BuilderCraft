# BuilderCraft delivery roadmap

Accepted direction: CAD for themed entertainment professionals, 2026-10-07. This roadmap supersedes inherited CADCraft product priorities for BuilderCraft. Upstream roadmap/history are preserved separately. No speculative hour estimates or parity percentages.

## Current baseline

One CADCraft-derived executable with initial rational 3D curves/control surfaces, numerical control editing, named assemblies/components/bodies, native `.bcraft` v1, undoable commands and local command API. Inherited drafting exists; full Rhino equivalence is unverified. Separate Scan/Graph/Show apps, mesh repair/metrology, live bridges, engine scene export and console exporters are not implemented.

## Sequence and completion gates

| Milestone | Deliver | Evidence required to finish |
|---|---|---|
| S0: contracts and honest coverage | Four-app boundaries, shared identity/units/revisions, documented Rhino inventory, StructureGraph reuse register and resource policy | Scope in repo; command counts reproducible; no false working statuses |
| S1: useful CAD and quick walkthrough | Modeling viewport/snaps/transforms, curve/surface foundations, controlled tessellation, GLB + scene manifest and Unreal helper | Massing scene imports at correct scale with instances/materials/collision/spawn; update preserves IDs; native exact model survives |
| S2: shared geometry and independent Scan alpha | Mesh/point cloud model, repair diagnostics, hole fill, smoothing, orientation, rigid best fit and deviation report | Scan app runs without CAD; known alignment fixture converges with residuals; repair preserves sources and attributes; cancellation/budgets measured |
| S3: independent Graph alpha | Typed node graph, lists/data trees, attributes, parameters/subgraphs, preview/bake, staged cache and time/state nodes | Same core commands through direct UI/API/nodes; save/reopen graph; deterministic recipe and cancellation; independent headless/app execution |
| S4: independent Show alpha and integrated previs | Equipment database, unit organization, DMX patch validation, CSV reports, fixture placement, basic cue intent, lighting and kinematic preview | Same UUID across record/scene/patch; collision/range validation; works without geometry; combined walkthrough with light and moving scenery |
| S5: validated exchange and console adapters | GDTF/MVR; first tested Eos and grandMA3 routes; Rhino/Autodesk interoperability tiers; versioned live bridge | Each direction/version explicitly tested; unsupported data reported; offline file fallback; patch and pre-cue tests separate |
| S6: deep modeling, scan reconstruction and VFX | Prioritized Rhino options/commands, robust Brep solids/booleans, offset/shell, scan reconstruction, procedural effects and richer simulations | Geometry/topology fixtures and memory gate; progressively broaden validated command and adapter catalogs |

S1 walkthrough does not wait for S6 VFX or complete Rhino parity. Show schema and bridge identity are defined early even while the independent app executable lands later. Progress is by tested vertical workflows, while the full command inventory prevents forgotten scope.

## Priority work queues

- CAD: embedded Grasshopper-style component canvas, live parametric modeling and preview/bake through Graph's shared evaluator; Kangaroo-style constraints, relaxation and form-finding; snaps, 3D transform/selection, curve creation/editing, projection/intersection, surface creation (loft/revolve/sweep), trim/join, topology/solids, offset/fillet, Flow/FlowAlongSrf and fabrication tools. `Project` and `FlowAlongSrf` have explicit acceptance cases.
- Scan: diagnose manifoldness/degenerates/self-intersections; preserve colors/normals/tags; smooth with feature boundaries; fill holes; offset/shell with thickness validation; rigid landmarks + ICP best fit with masks/weights/outlier rejection; residual/deviation reporting and scan-to-CAD later.
- Graph: exact CAD and mesh ports first, Grasshopper data-tree semantics and Houdini attributes, immutable recipes, incremental jobs, bake, kinematics, then particles/volumes/VFX.
- Show: lighting/AV/automation equipment schema, patch and reports, cue intent, portable rig exchange, then individually validated console exporters.
- StructureGraph: recipe stages/rails/provenance/QA first; deterministic scenic generators and connectors after source/fixture review.
- Engineering: geometry resource handles, aggregate RAM/undo/cache budgets, bounded IPC/import, worker cancellation, GPU teardown and reproducible benchmarks.

## Status rules

Working: implemented and acceptance fixtures pass in a stated representation/scope. Partial: implemented subset with explicit options/limitations. Unvalidated: candidate implementation exists but reference acceptance is pending. Not implemented: absent native equivalent. External adapter: separately labeled; host-supported behavior is not counted as native parity.

Track import and export separately for every format; track patch and cue features separately for every console. Update status only alongside implementation evidence. No claiming full Rhino/Houdini/metrology/Lightwright parity from menu names or dependency availability.

## First kernel increment

Implemented: dependency-light `buildercraft-kernel`, immutable shared exact geometry, bounded mesh/point-cloud resources, unit/axis point conversion, revision-checked atomic scene batches, hierarchy validation, cancellation, snapshots and metadata manifest. Existing CAD now uses shared exact buffers and copy-on-write control edits; `kernel.manifest` projects current 3D organization into these contracts. See `../architecture/KERNEL.md` for usage and limitations. Graph evaluation, embedded CAD canvas, Kangaroo solver, independent Scan/Graph/Show executables and engine export remain planned.

## Next concrete tasks

1. Tessellate exact curves/surfaces into bounded preview resources, preserving exact source and revision.
2. Export a massing scene to GLB with the semantic manifest; validate units, identity and hierarchy in an engine import fixture.
3. Add the shared typed graph evaluator and persistent parameters/data trees, then embed its component canvas inside CAD.
4. Implement original native constraint goals and bounded iterative relaxation for Kangaroo-style form-finding; verify convergence, anchors, units and cancellation.
5. Reuse reviewed StructureGraph recipes through these shared geometry/graph services.

The native solver is required; a licensed-host adapter cannot substitute for it.
