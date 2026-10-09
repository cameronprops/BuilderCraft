# Worldwright + Orb Weaver: dependency-first tool building

**Orb Weaver** is the planned, native and independently runnable Grasshopper-style
parametric graph environment in Worldwright. The name is taken from the familiar orb-weaving spiders. It is a working
product name; trademark clearance has not been performed.

## Source registers

| File | Meaning |
|---|---|
| `tool-groups.json` | Reviewed **group-level** dependency DAG, build tiers, modifier-first operation pair plan and provisional classification rules |
| `kernel-operation-deps.json` | Prerequisite graph for **all 41** registered kernel operations; 23 have an explicit source/dataflow dependency review, 18 await it |
| `reference-index.json` | All 1,072 Rhino command refs, 817 Grasshopper refs, 110 Kangaroo refs and 2,357 Rhino manual topics given preliminary groups; 4,356 total source rows |
| `../commands/rhino8.json` | Reference command behavior review and status; authoritative for command parity status |
| `../commands/manual_inventory.json` | Manual topic review and acceptance evidence; authoritative for reviewed manuals |
| `../components/grasshopper1-kangaroo2.json` | Graph reference components and parity contracts; authoritative for native component status |
| `crates/kernel/src/shared_tools.rs` | **Executable** typed operation contracts and dispatcher for paired CAD and Orb Weaver nodes; new source of truth for the initial tool pairs |
| `crates/orb-weaver/src/lib.rs` | Native graph execution with typed inputs and deterministic topological evaluation |
| `crates/engine/src/cmd/worldwright_tools.rs` | Worldwright CAD/API commands, calling the **same** kernel dispatcher |

The dependency index is a **build-routing aid, not a verified full hierarchy of
4,356 individual tool contracts**. By contrast, all 41 currently registered
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
| 1 | Scalar/vector math and typed Orb Weaver ports | Dot, cross, normalize, amplitude |
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
Orb Weaver UI code.

## One kernel, two interfaces

A shared native operation has:

1. One **kernel algorithm**, valid without a CAD file or paid Rhino host.
2. One **typed port contract** (validated kinds, semantics and modifier ports).
3. One **CAD command adapter** and one **Orb Weaver graph node adapter**, both
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

| Shared operation | Worldwright CAD/API command | Orb Weaver component | Notes |
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

This is **not** ten verified Grasshopper feature-parity ports. Initial
Orb Weaver graph values are individual typed values, no implicit conversion or
list/data-tree matching. Graph work is pure/headless and can run without the
CAD application. The new code has unit tests authored but not yet executed
against Cargo/Rust; do not mark official catalog entries working yet.

## Subsequent implementation order

1. Validate this layer with local Cargo: shared dispatcher, Orb Weaver graph,
   engine commands, reference-index checks, and `.dftba` persistence tests.
2. Introduce typed **data trees** with branch paths, graft/flatten, list
   matching, and graph evaluation that handles a branch of items.
3. Support **geometry references** as immutable versioned handles (including
   exact curves, surfaces, meshes) and preview/bake transactions.
4. Then pair curve evaluation, division, length and transforms with CAD
   commands and Orb Weaver nodes using shared exact geometry.
5. Add surfaces, intersections, topology, meshing, solids and form-finding
   once their prerequisite algorithms and validation are established.
6. Keep every new tool paired by default. Exceptions must document why the
   operation is UI-only, graph-only, or host-specific; none may create a
   second independent geometry implementation.

No compiler build or full-reference conformance is implied until local
validation is actually recorded.
