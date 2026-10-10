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

CAD-first alpha dependency order, reviewed against source on 2026-10-09:

1. Close the viewport selection loop with the existing transactional gizmo.
2. Add construction-plane point input and endpoint snapping; use it for interactive curve creation and transform reference input.
3. Add shared exact surface construction (extrude/loft/revolve) through CAD/API and the upcoming node evaluator.
4. Add the shared typed graph evaluator and persistent parameters/data trees, then embed the OrbWeaver component canvas inside CAD.
5. Extend the inherited drafting constraint service into mechanical sketches and feature-history blocks through the same geometry operations; bounded relaxation/form-finding follows the graph contracts.

Preview tessellation, GLB export and the local visualization feed are already
implemented. Unreal host acceptance remains outstanding and does not block CAD
interaction work. Scan/Show expansion remains deferred under the CAD-first
priority. Robust trimming/topology/solids depend on curves, surfaces and
intersections; additional standalone transform names do not close those gaps.
See `CAD_ALPHA_DEPENDENCIES.json` for the machine-readable sequence.

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


## OrbWeaver and dependency-first paired tools (partial native acceptance)

Worldwright's native Grasshopper-style graph core is now called **OrbWeaver**
(working name, after orb-weaving spiders). It will run
inside CAD and headlessly/independently through the same underlying kernel.
A separate visual canvas and executable remain later deliverables.

The group-level prerequisite DAG is `../dependencies/tool-groups.json` and
the preliminary reference mapping is
`../dependencies/reference-index.json`. All 1,072 Rhino commands, 817
Grasshopper entries, 110 Kangaroo entries and 2,357 manual topics are accounted
for, but initial classification is heuristic and 2,347 references remain
unclassified. **No per-command or per-component verified dependency hierarchy
or conformance claim is inferred from these labels.**

Twenty-five CAD/Graph pairs now route point, vector, polyline, native data-tree
and supported modeling functions through
one typed kernel dispatcher (`crates/kernel/src/shared_tools.rs`). The newly
authored `crates/orbweaver` evaluator supports typed ports, literal/linked
values, versioned serializable graphs, dependency scheduling, cycle/type
checks, graph limits and atomic error propagation. The CAD/API command
adapter in `crates/engine/src/cmd/worldwright_tools.rs` uses those same
validated implementations. Vector length is an additional base primitive,
reused by normalize. The interpolation fraction, division count and division
spacing are named modifier ports.

These native code paths passed the local Rust validation recorded below.
Native graph data-tree structure operations are implemented
with strict branch paths, explicit matching policies, flatten, graft and simplify.
A component canvas, exact Grasshopper implicit
path matching, preview/bake, solver, expressions and exact reference GH port
matching are not implemented. Do not promote any public catalog entry to working parity without
a local compilation/test and reference conformance fixture.

The next dependency-respecting build steps are: versioned geometry reference
ports and their typed graph bindings, graph persistence in `.dftba`, preview/bake
transactions, exact curve operations paired in CAD/OrbWeaver, then surface,
intersection/solid and physics forms. Each algorithm is implemented once,
and CAD options / OrbWeaver settings are modifiers or thin adapters.

### OrbWeaver native data-tree operation increment

The Rust graph crate is now `crates/orbweaver` (package `orbweaver`,
public nodes `orbweaver.*`). The dependency map now contains **57** registered
kernel operations, of which **25** have shared CAD/OrbWeaver typed ports.
The five new `kernel.tree.*` paired operations validate canonical branch paths,
flatten, graft, simplify and match with explicit Shortest, Longest and
CrossReference modifiers. The existing OrbWeaver DAG can link tagged tree
values through these nodes. A headless `paired_tree` example checks that
CAD and graph entry points invoke one dispatcher. Grasshopper tree-path
matching, graph UI and
`.dftba` graph persistence are still future work. Local compiled tests passed in this increment; hosted
CI conclusions and Grasshopper conformance are separate evidence.

## History-driven mechanical modeling track

The optional scoped feature-history foundation now has a machine-readable
[60-item dependency catalog](../dependencies/feature-history.json):
eight native infrastructure contracts with local test evidence and 52 planned
sketch, dimension, solid, assembly and fabrication operations. Histories can
belong to a document, a component/body node or a reusable block definition,
without imposing a timeline on direct CAD modeling.

The initial code stores stable step IDs, typed local parameters, chronological
dependencies, suppression, rollback, revision-checked edits and CAD undo.
It evaluates already-implemented shared kernel operations and saves recipes
inside `.dftba`. An **initial 3D workspace timeline panel** now exposes enable, inspect, reorder, suppression, rollback and editing of basic local values. This is **not** a full visual feature-authoring or functioning sketch-to-solid mechanical modeler. Next: stable versioned geometry
references, dimension expressions, constraint-driven sketch profiles and
shared extrude/revolve/hole/fillet kernels, then previews and UI/bake.
## Construction-plane drafting increment

Shared orthographic plane inversion and exact curve endpoint queries now drive
Draw control curve. XY/XZ/YZ, origin, degree and snap controls feed a transient
preview. Enter/Finish creates one undoable curve; Escape and stale document or
plane changes cancel. API controllers use `geometry3d.snap` and
`nurbs.controlcurve3d`, sharing the same geometry and transaction services.
Arbitrary planes are supported by the query API. Surface corners are available; intersections,
transform reference input and interpolated curves remain planned. Next in the
CAD hierarchy is shared exact surface construction from these authored curves.
See `BUILDERCRAFT_API.md` for limits and `CAD_ALPHA_DEPENDENCIES.json` for scope.

## Repository integration increment

GeoRust robust 1.2 supplies adaptive projected-triangle orientation through the
shared closest-point kernel. stl_io 0.11 and tobj 4.0.5 supply bounded position-
only STL/triangular OBJ exchange through L3 IO and L4 commands. These results
reuse the existing TriangleMesh, validation, face-normal and closest-point
services. Source meshes remain unchanged; f32 and attribute/index losses are
explicit. Persistent mesh editing already exists through `mesh3d.*`; this exchange
facade adds no implicit persistence or full Scan UI.

The reviewed used/deferred list is in
`../architecture/OPEN_SOURCE_INTEGRATION.md` and its machine-readable register.
Curvo is the next surface-construction candidate. geo-index is 2D, while
3D acceleration must use suitable shared services. Vello CPU is already
indirectly present through epaint. Non-MIT Truck/Parry/Fidget candidates are
reopened under the broader open-source preference, pending their feature gates.

## Latest-main reconciliation

The drafting increment was reconciled with main `f9ba908`: mesh-scene editing,
optional histories, OrbWeaver typed evaluation, `.dftba` persistence, LAS/noise
adapters and shared numeric PushPull/Project/Flow/array subsets are retained.
Earlier uncompiled notes above describe their original authoring checkpoints;
they are not current validation evidence. See the final validation record below
and `../ALPHA_SMOKE_TEST.md` for the compiled-program/UI acceptance sequence.

Next work follows [issue #26](https://github.com/cameronprops/BuilderCraft/issues/26)
and `IMMEDIATE_MODELING_OPENSCAD_AND_ALPHA_UI.md`: shared curve derivatives,
adaptive arc-length stations and rail frames before Pipe/Sweep and BRep work.
Native mesh Project/Flow already has undoable document adapters. Geometry
references/project persistence/bake remain graph-specific needs; preserve the
existing document operations and one common kernel.

### Reconciled validation result

Linux Rust 1.95.0: 646 workspace tests and all six CI gates passed, including
13 configured WASM libraries. The required kernel wrapper and native CAD/CLI
build passed. Compiled CLI curve save/reopen and PushPull smoke checks passed;
headless viewport rendering was inspected. OrbWeaver is registered at L2.
See `../architecture/KERNEL_VALIDATION.md` for scope and platform limits.
