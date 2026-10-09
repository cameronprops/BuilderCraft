# BuilderCraft delivery roadmap

Accepted direction: CAD for themed entertainment professionals, 2026-10-07. This roadmap supersedes inherited CADCraft product priorities for BuilderCraft. Upstream roadmap/history are preserved separately. No speculative hour estimates or parity percentages.

## Current baseline

One CADCraft-derived executable with initial rational 3D curves/control surfaces, numerical control editing, named assemblies/components/bodies, native `.bcraft` v1, undoable commands and local command API. Inherited drafting exists; full Rhino equivalence is unverified. Separate Scan/Graph/Show apps, mesh repair/metrology and console exporters are not implemented. Native bounded GLB export and saved/unsaved local scene feeds now exist; the optional Unreal adapter is source-only and awaits host validation.

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

## Tessellation increment

Implemented bounded uniform-parameter curve polylines and untrimmed surface triangle previews in the shared kernel, plus the non-mutating `geometry3d.preview` API. Resolution, sample, output-capacity and complexity limits are checked before sampling; jobs support cancellation and final retained-byte admission. Exact CAD data is preserved. This is preview sampling, not adaptive/tolerance-certified meshing, trimmed Brep meshing, a new viewport renderer or a GLB exporter.

Required native command/component inventories and viewport work are tracked in `../commands/NATIVE_COVERAGE.md`. All native code must be original or verified open source; the native suite must remain free to run without paid hosts.

## Visualization feed and production organization increment

Implemented: native bounded 3D scene snapshot, portable GLB, explicit coordinate/winding conversion, persisted feed sequence, single-writer local publication, saved-project watch CLI, and stable IDs for recurring scene updates. Production records, multiple object assignments and relationships persist in `.bcraft`, are undoable through API and travel with the scene. Original Unreal polling adapter source is provided; compilation and editor/runtime walkthrough acceptance are still pending on an Unreal host. Automatic publication of every unsaved edit, asynchronous/coalesced jobs, richer rendering and delta transfer remain work items.

Added required native ride-path assembly motion, swept envelopes, clearances and sightlines; 2–5 rail Track generation with maintained gauges, banking and smooth solids; script/storyboard references and optional Scripto discovery; BIM/cross-discipline smart connectors. See `../architecture/PRODUCTION_ORGANIZATION.md`. These geometry/adapters/UI capabilities are planned, not implemented by metadata alone.

BuilderCraft-owned editable previs UI over the Unreal backend is required, including geometry/placement, lighting, automation, vehicles, cameras, audio/projection, effects and production organization. Current bridge is one-way visualization; bidirectional editing and the custom workspace remain pending. See `../architecture/PREVIS_INTERFACE.md`.

### Native unsaved visualization feed

Implemented a document-bound background publisher with one replaceable pending drawing,
cooperative cancellation, start/status/stop API, committed interactive edit polling and
undo/redo submission. Validation covers unsaved metadata bursts and tab isolation.
Unreal host validation and bidirectional editing remain subsequent milestones.

### CAD-first priority and component inventory

Current user priority is the native CAD app: Rhino-like modeling, embedded full Grasshopper-style parametric authoring, and SolidWorks-style sketch/feature workflows. Scan, Show and Unreal interface expansion are deferred; existing bridges remain optional. Grasshopper 2 is a separate reference generation.

The public reference register contains 817 built-in Grasshopper 1 entries and 110 Kangaroo 2 entries, all not implemented. Purpose summaries and links are recorded; typed ports, runtime GUIDs, tree matching and exact-version reconciliation are pending. Three pairs of 1D/2D domain entries share source URLs and are explicitly flagged.

Added orthographic Top, Front, Right and Isometric camera commands, Fit of visible exact-geometry control hulls, and Shift-drag pan. Fit bounds work to 100,000 controls and rejects invalid/empty hulls without changing the camera. It conservatively frames the control hull, not tight trimmed geometry bounds. No perspective, geometry picking, multiple viewports or full Grasshopper canvas is claimed.

The component register explicitly tracks known public-feed omissions: Point/Curve and other basic typed parameters require a separate reconciliation queue in `docs/components/GAP_REVIEW.md`. Public API documentation currently identifying a Rhino 9 build must not certify Rhino 8 coverage.

### Manual-driven CAD kernel development

Target is systematic feature-by-feature reconstruction of the latest released Rhino
manual, with platform/options/type coverage tracked separately. Rhino 8 stable help
is the current baseline; WIP and exact-build reconciliation remain separate. Added
manual topic seeds and review gates in `docs/commands/MANUAL_REBUILD.md`.

First kernel/API/browser increment implements exact 3D translation, axis rotation,
positive uniform scale and plane reflection, with atomic batch validation, optional
fresh-ID copies and undo. Move/Rotate3D/Scale/Mirror are partial, limited to native
exact curves/control surfaces. Numeric editing is implemented; viewport gumball,
other representations, richer options and associative history remain pending.

### Directional scaling increment

Added original native Scale1D/Scale2D API operations for exact curves and control
surfaces, including explicit arbitrary directions/planes, zero-factor flattening,
copy and undo. Coverage remains partial; interactive CPlane/reference workflows
and other geometry types are pending. See `../commands/MANUAL_REBUILD.md`.

### Nonuniform scaling increment

Added original native ScaleNU world-axis and ScaleByPlane explicit-frame API
operations for exact curves/control surfaces. Parameterization, atomic batch
behavior, copying, undo and persistence are preserved. Reference/CPlane/preset
UI, Rigid/history and other representations remain pending. Next: ScalePositions.

## ScalePositions increment

Native exact curves/control surfaces now support 1D, 2D and 3D spacing through
`geometry3d.transform`, with Space buttons in the transform panel. Bounds come
from rational span subdivision; objects retain their sizes and parameterization.
Shared batch work limits, copy, undo/redo, failed-operation preservation and
`.bcraft` round trips have regression coverage. See
[command contract](../commands/MANUAL_REBUILD.md) for tolerance, scope and pending options.

## Shear increment

Added original bounded affine shear to the shared exact transform service and
the numeric transform panel. Copy, undo/redo, rational parameterization and
project persistence use the existing transaction path. Scope and pending options
are in `docs/commands/MANUAL_REBUILD.md`. Hosted workflows are now manual-only;
ordinary source pushes do not schedule GitHub Actions.

## Orient3Pt increment

Added original three-point frame orientation to the shared exact transform
service and numeric UI, with optional first-edge uniform scaling. Existing copy,
undo/redo, resource limits and project persistence apply. Exact rational curves
and control surfaces only; reference picking and other representations remain
pending. See `docs/commands/MANUAL_REBUILD.md` for numerical limits and evidence.
