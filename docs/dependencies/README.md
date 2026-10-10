# Worldwright + OrbWeaver: dependency-first tool building

**OrbWeaver** is the planned, native and independently runnable Grasshopper-style
parametric graph environment in Worldwright. The name is taken from the familiar orb-weaving spiders. It is a working
product name; trademark clearance has not been performed.

## Source registers

UI source provenance and copy permissions are now tracked by
[`UI_CODE_REUSE.json`](UI_CODE_REUSE.json) and checked with
`python3 tools/check_ui_reuse_manifest.py`. See the
[open-source donationware and copyleft policy](../architecture/COPYLEFT_AND_DONATIONWARE.md)
before importing GPL/LGPL/MPL code or asset packs. The broad upstream
candidate evaluation is in [`OPEN_SOURCE_REUSE_CANDIDATES.json`](OPEN_SOURCE_REUSE_CANDIDATES.json).

| File | Meaning |
|---|---|
| `tool-groups.json` | Reviewed **group-level** dependency DAG, build tiers, modifier-first operation pair plan and provisional classification rules |
| `kernel-operation-deps.json` | Prerequisite graph for **all 47** registered kernel operations; 29 have an explicit source/dataflow dependency review, 18 await it |
| `metrology-reuse.json` | Source-checked existing kernel reuse plus **16 proposed shared primitives** and **17 planned Scan/CAD/OrbWeaver/fabrication adapters** for alignment, mesh repair, measurements, deviation and reconstruction |
| `reference-index.json` | All 1,072 Rhino command refs, 817 Grasshopper refs, 110 Kangaroo refs and 2,357 Rhino manual topics given preliminary groups; 4,356 total source rows |
| `../commands/rhino8.json` | Reference command behavior review and status; authoritative for command parity status |
| `../commands/manual_inventory.json` | Manual topic review and acceptance evidence; authoritative for reviewed manuals |
| `../components/grasshopper1-kangaroo2.json` | Graph reference components and parity contracts; authoritative for native component status |
| `crates/kernel/src/shared_tools.rs` | **Executable** typed operation contracts and dispatcher for paired CAD and OrbWeaver nodes; new source of truth for the initial tool pairs |
| `crates/orbweaver/src/lib.rs` | Native graph execution with typed inputs and deterministic topological evaluation |
| `crates/engine/src/cmd/worldwright_tools.rs` | Worldwright CAD/API commands, calling the **same** kernel dispatcher |

The dependency index is a **build-routing aid, not a verified full hierarchy of
4,356 individual tool contracts**. By contrast, all 47 currently registered
kernel operations have explicit DAG entries, with 18 marked source-review
pending. The dependency graph is validated for cycles and tier ordering. It contains classification candidates and
more than two thousand unclassified references. Every individual dependency
must be verified against actual behavior and a test fixture before promotion
to a reviewed contract. Unclassified items remain explicitly visible rather
than being silently put in a default "implemented" category.

Regenerate or verify the index after updating reference inventories:

```sh
python3 tools/build_dependency_index.py --check
python3 tools/check_paired_tools.py
python3 tools/build_dependency_index.py --write
```

The check uses no network calls or paid GitHub Actions.

## Dependency ladder (topologically ordered)

| Tier | What we build | Examples |
|---|---|---|
| 0 | Typed values, frames/units, IDs/revisions | Numbers, points, validators, units |
| 1 | Scalar/vector math and typed OrbWeaver ports | Dot, cross, normalize, amplitude |
| 2 | Point geometry and graph DAG execution | Distance, midpoint, interpolation |
| 3 | Polyline geometry, transforms, graph list/tree skeleton | Segment length, divide by count/distance |
| 4 | Curves, triangle/quad mesh, selection | Curve evaluation and mesh construction |
| 5 | Surfaces, mesh topology, preview | Surface frames, boundary loops, repair |
| 6 | Intersections, tree/bake, constraints | Project, Trim, Kangaroo-like goals |
| 7 | BRep solids, scan registration, I/O | Sweep/loft-to-solid, best fit, shell |
| 8 | Scenic and show/previs systems | Rockwork, track/rails, assets and fixtures |

The tier labels are coarse execution ordering. The actual validated DAG is
`tool-groups.json`, where each group lists its prerequisite IDs. Independent
branches can proceed in parallel; lower-layer code must not import CAD or
OrbWeaver UI code.

## Shared metrology operations and app boundaries

Scan workflows inspired by PolyWorks, GOM, and Geomagic Design X reuse the
existing numerical kernel instead of growing independent CAD and Scan engines.
The tracked dependency plan is [metrology-reuse.json](metrology-reuse.json),
with **13 existing registered kernel foundations**, **16 proposed
shared kernel operations** and **17 planned UI/command adapters**.

The plan explicitly distinguishes three states: an **existing kernel operation**
(some services already implemented but not evidence of a finished metrology tool),
a **planned shared numerical primitive** (no Rust implementation yet), and a
**planned app adapter** (Scan/CAD/OrbWeaver/fabrication interface not yet authored).
No new planned IDs are added to the runtime kernel registry until implementation,
unit tests, interface integration and native CI validation establish working code.

High-leverage shared sequences:

- Point-cloud spatial indexes → stable closest-point queries → rigid best-fit
  and ICP → nominal-to-actual deviation with signed/unsigned policies.
- Mesh topology and weld/repair → nearest triangle queries → wall thickness,
  clearance, hole diagnostics and downstream fabrication QA.
- Frames/units and world coordinates → height-band grouping, reports and
  cross-application model synchronization (especially scenic rockwork).
- Fit planes, primitives and mesh sections → editable NURBS reconstruction.

Spatial indexes are immutable/revision-keyed and budgeted. Apps share source
IDs, tolerances, coordinate frames and cancellation contracts without loading
all scan datasets into CAD document entities. Metrology-specific user
interfaces and optional commercial host API connectors contain **no separate
geometry algorithms**. A native Scan app remains independent of CAD; pending
Scan work never blocks the CAD alpha dependency chain.

Validation: `python3 tools/check_metrology_reuse.py` is integrated into
`tools/check_paired_tools.py` and the native CI validation suite. This checks
referential integrity, tier ordering, cycles, typed port descriptions and
truthful planned states, not geometry accuracy. No proprietary implementation
or format reverse engineering is assumed.

## One kernel, two interfaces

A shared native operation has:

1. One **kernel algorithm**, valid without a CAD file or paid Rhino host.
2. One **typed port contract** (validated kinds, semantics and modifier ports).
3. One **CAD command adapter** and one **OrbWeaver graph node adapter**, both
   delegating to the kernel dispatcher.
4. Optional **document wrappers** for object selection, drawing unit policies,
   undo/copy, history, preview/bake, and interactive options.
5. Independent acceptance gates: native algorithm tests, paired-interface
   equivalence, document transaction/undo, graph determinism, UI behavior,
   reference option parity.

### Base + modifier policy

Modifiers are named typed ports, never a separate copy of the algorithm.
Examples:

- `kernel.point.interpolate` has base points **a, b** and modifier **t**
  (fraction from 0 to 1).
- `kernel.polyline.divide_count` and
  `kernel.polyline.divide_distance` use the same shared arc-length
  sampling implementation, with modifiers **count** or **spacing**.
- `kernel.vector.normalize` reuses the validated **length** base operation.
- Future scale, scale1d, scale2d, scaleNU and scale-positions variants should
  share transformation policy and coordinate-frame operations.
- Future offset, shell, thickness and connectors should share distance,
  surface-normal and collision validation (not duplicate mesh offset solvers).

Modifier support can be a literal setting or a dynamically wired graph port.
More complex modifiers (tree matching, tolerance, periodic/wrap, reference
geometry, masks) will have separate **contracts**, but continue to compose
shared kernel services.

## First paired implementation, currently source-only

| Shared operation | Worldwright CAD/API command | OrbWeaver component | Notes |
|---|---|---|---|
| Point distance | `worldwright.point.distance` | `orbweaver.point.distance` | GH *Distance* reference behavior not fully verified |
| Point midpoint | `worldwright.point.midpoint` | `orbweaver.point.midpoint` | Native point primitive |
| Point interpolation | `worldwright.point.interpolate` | `orbweaver.point.interpolate` | Modifier `t` |
| Vector length | `worldwright.vector.length` | `orbweaver.vector.length` | GH *Vector Length* candidate |
| Vector normalize | `worldwright.vector.normalize` | `orbweaver.vector.normalize` | GH *Unit Vector* candidate |
| Vector dot | `worldwright.vector.dot` | `orbweaver.vector.dot` | GH *Dot Product* candidate |
| Vector cross | `worldwright.vector.cross` | `orbweaver.vector.cross` | GH *Cross Product* candidate |
| Polyline length | `worldwright.polyline.length` | `orbweaver.polyline.length` | Polyline-only native behavior |
| Divide by count | `worldwright.polyline.divide_count` | `orbweaver.polyline.divide_count` | Modifier `count`, polyline subset of GH *Divide Curve* |
| Divide by distance | `worldwright.polyline.divide_distance` | `orbweaver.polyline.divide_distance` | Modifier `spacing`, polyline subset of GH *Divide Distance* |
| Validate tree | `worldwright.tree.validate` | `orbweaver.tree.validate` | Base dependency; checks paths and item limits |
| Flatten tree | `worldwright.tree.flatten` | `orbweaver.tree.flatten` | Single path `{0}`, ordered items |
| Graft tree | `worldwright.tree.graft` | `orbweaver.tree.graft` | Item-indexed child paths; empty branches preserved |
| Simplify tree | `worldwright.tree.simplify` | `orbweaver.tree.simplify` | Removes only common leading path prefix |
| Match tree | `worldwright.tree.match` | `orbweaver.tree.match` | Modifier `mode`: shortest, longest, cross-reference; identical branch paths required |

These are **15 native paired operation contracts**, not 15 verified
Grasshopper-equivalent components. OrbWeaver now supports tagged tree values,
structural tree nodes, explicit list matching and branch-preserving graph links.
It supports explicit Shortest/Longest/CrossReference broadcasting for
the first ten native point/vector/polyline operations, but **not** Grasshopper's
implicit branch-path matching or full component-option parity. Graph work is pure/headless and can run without the
CAD application. The new code has unit tests authored but not yet executed
against Cargo/Rust; do not mark official catalog entries working yet.

## Subsequent implementation order

1. Validate this layer with local Cargo: shared dispatcher, OrbWeaver graph,
   engine commands, reference-index checks, and `.dftba` persistence tests.
2. Extend **typed geometry references and document-scoped handles** beyond the current paired
   primitives when each new geometry operation is validated; never presume
   Grasshopper's implicit path alignment.
3. Support **geometry references** as immutable versioned handles (including
   exact curves, surfaces, meshes) and preview/bake transactions.
4. Then pair curve evaluation, division, length and transforms with CAD
   commands and OrbWeaver nodes using shared exact geometry.
5. Add surfaces, intersections, topology, meshing, solids and form-finding
   once their prerequisite algorithms and validation are established.
6. Keep every new tool paired by default. Exceptions must document why the
   operation is UI-only, graph-only, or host-specific; none may create a
   second independent geometry implementation.

No compiler build or full-reference conformance is implied until local
validation is actually recorded.

## Tree foundation increment (source authored, uncompiled)

The common kernel now owns `DataTree<T>`, `TreePath`, and operations in
`crates/kernel/src/data_tree.rs`. A validated tree has 0–4,096 canonical,
unique, lexicographically sorted branch paths, maximum path depth 16, and
up to 250,000 top-level items. Empty branches are meaningful; absent trees
are distinct. Grafting each item creates a child path; an empty branch creates
one empty child. Simplification keeps at least one path coordinate. List
matching pairs only **identical branch paths**; Longest extends by repeating
the final item and CrossReference forms the explicit Cartesian product.

All five tools are **paired** between Worldwright CAD's `worldwright.tree.*`
command family and OrbWeaver's `orbweaver.tree.*` node family, calling one
kernel dispatcher. Typed `ToolValue::Tree` and `ToolValue::Pair` values can
flow through linked OrbWeaver graph nodes. Resource budgets include nested
polyline/tree/pair values and preflight cross-reference cloning. The previous
scalar evaluator is extended, not replaced or duplicated.

The known GH public-index candidates for Flatten Tree, Graft Tree and Simplify
Tree are mapped provisionally. This is not Grasshopper path-matching parity,
and these references retain `not_implemented` status until a runtime build,
fixtures and option review establish verified coverage. Native `.dftba`
graph persistence and the graphical component canvas are not yet wired.

Run `python3 tools/build_dependency_index.py --check` and
`python3 tools/check_paired_tools.py` for static consistency; run the
local Rust test/lint scripts in a Rust-capable environment before merging.

## Tree broadcasting increment (source authored, compilation pending)

`crates/kernel/src/tool_broadcast.rs` is a policy adapter, **not** an
additional geometry engine. It accepts native `ToolValue::Tree` inputs on
the first ten CAD/OrbWeaver point, vector and polyline operations, validates
every leaf type, and delegates each matched item back to the same scalar
kernel operation. It preserves strict canonical branch paths and empty
branches. The policy is selected via `matching` on a CAD command or on
an OrbWeaver node, and defaults to Shortest for old version-1 graphs.

Shortest truncates to the shortest list, Longest repeats the last item,
and CrossReference applies a Cartesian product in port order with the
rightmost varying fastest. Preflight checks limit branch count, output
count, repeated input allocations and nested value costs. Mixed branch
paths, invalid leaf types and runaway Cartesian products fail atomically.
These behaviors are native contracts and do **not** claim complete
Grasshopper implicit path alignment.

Regression tests cover linked graph nodes, all three policies, three-port
Cartesian ordering, stale/incompatible linked types, preserved empty
branches, invalid paths and resource limits. Tests are committed but **not
compiled or executed yet**. The next reusable infrastructure layer is
versioned geometry references, shared with CAD document identity and undo.

## Scoped history-driven modeling dependencies

The separate [history feature register](feature-history.json) contains **60**
contracts and tool dependencies: eight source-authored, uncompiled history
primitives and 52 planned capabilities. These cover typed dimensions, sketch
constraints and solving, profiles, BRep solids, extrude/revolve, holes, fillets,
patterns, and mechanical assemblies.

They are not 60 new geometry engines. The kernel registry remains at 47
operations, including 15 existing CAD/OrbWeaver paired contracts. Histories
execute those operations through the same dispatcher.

A feature history is optional and belongs to a document, a model node, or
a reusable block definition. Direct-modeled objects remain independent.
See [feature-history rules](../architecture/FEATURE_HISTORY.md). Native
`.dftba` persists source-authored recipes; visual timelines, constrained
sketches, solids and history bake are still planned and unverified.
