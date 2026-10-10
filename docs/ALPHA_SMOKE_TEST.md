# Worldwright alpha acceptance walkthrough

This is an acceptance sequence for the compiled native program, not a claim of
Rhino parity. Use synthetic geometry. Save to a new `.dftba` file and verify
legacy `.bcraft` files still reopen. Keep Windows, macOS, Linux and Haiku results
separate; a Linux test result does not establish another platform's support.

## Compile and automated checks

```sh
bash tools/check-worldwright-rust.sh
bash tools/verify-worldwright-kernel.sh
cargo xtask ci
cargo build --locked -p cadcraft -p cadcraft-cli
cargo run --locked -p orbweaver --example paired_tree
```

The kernel wrapper includes mesh scene, document/I/O/engine/UI and OrbWeaver
tests. The six CI gates add workspace Clippy, assets, layering and WASM checks.
Record the observed result and commit, including failures.

## Native viewport and persistence

Start `cargo run --locked -p cadcraft -- --sample --control 39137` with a free
local port. Open the 3D workspace and perform these steps:

| Step | Action | Expected acceptance |
|---|---|---|
| 1 | Select exact curve wires and a mesh face; Shift-click to toggle | Shared selection; hidden/locked layers excluded; selection adds no undo entry |
| 2 | Move/rotate/scale a selected exact shape with the gizmo | One undoable operation; source geometry and rational parameters preserved |
| 3 | Choose XY, XZ and YZ construction planes and edit their origin | Grid follows the chosen plane; edge-on unsnapped input reports an error |
| 4 | Draw control curve; choose at least two points; finish with Enter or Finish | Exact native control curve, one undo step; degree adapts to point count |
| 5 | Draw with End/corner snap enabled near an off-plane endpoint | Endpoint stays at its exact world coordinate; disabling snap uses the plane |
| 6 | Begin drafting, then press Esc or change the document/plane | Draft cancels; no geometry or undo mutation |
| 7 | Undo and redo the finished curve and a mesh edit | Geometry/revisions update coherently; stale mesh picks cannot edit a new revision |
| 8 | Save `.dftba`, close, reopen; reopen a synthetic legacy `.bcraft` | Exact curves, meshes and optional histories persist |

The drawing tool creates a **control curve**, not an interpolated curve through
every clicked point. Wire selection uses sampled previews. Mesh/NURBS selection
and drafting share pointer consumption, so drafting clicks must not change
selection underneath the active tool.

## Command/API smoke list

Send JSON lines to the local control channel using `engine.execute`. For example:

```json
{"id":1,"method":"engine.execute","params":{"command":"nurbs.controlcurve3d","params":{"name":"Alpha curve","points":[[0,0,0],[2,1,0],[4,0,0]],"degree":2}}}
{"id":2,"method":"engine.execute","params":{"command":"worldwright.tool.list","params":{}}}
{"id":3,"method":"engine.execute","params":{"command":"worldwright.pushpull","params":{"inputs":{"face":{"kind":"polyline","value":[{"x":0,"y":0,"z":0},{"x":2,"y":0,"z":0},{"x":2,"y":2,"z":0},{"x":0,"y":2,"z":0}]},"distance":{"kind":"number","value":2}}}}}
```

| Command family | What to check | Current limit |
|---|---|---|
| `geometry3d.cplane.point`, `geometry3d.snap`, `geometry3d.line` | Plane queries, snap identity, one-transaction line construction | Orthographic input; no general intersection snap |
| `nurbs.controlcurve3d`, `geometry3d.controlpoint`, `geometry3d.transform` | Creation/edit, invalid input, undo and save/reopen | Native exact curves/control surfaces; not all Rhino options |
| `mesh3d.create/list/preview/boundaries/edit/set/array/pushpull/project/flow_along_srf` | Native polygon meshes, revision-safe edits, face mapping | Supported repair, arrays, PushPull and Project/Flow subsets; not general BRep modeling |
| `worldwright.mesh.decode/encode` | ASCII/binary STL and triangular OBJ round trips; explicit losses | Read-only 8 MiB/65,536-element exchange; no automatic file or document import |
| `worldwright.mesh.closest_point`, `worldwright.cloud.closest_point` | Known closest-point fixture and empty/invalid cases | Bounded shared queries; not full inspection/ICP |
| `worldwright.array.linear/rectangular/polar/path` | Counts, axes, order and deterministic result | Shared typed point/polyline outputs; `mesh3d.array` separately creates undoable mesh copies |
| `worldwright.pushpull` | Signed distance and invalid face rejection | Planar quad output; not general BRep solids |
| `worldwright.project`, `.project.mesh`, `.project.nurbs` | Explicit direction and missed-target rejection | Numeric/sampled supported target subsets |
| `worldwright.flow_along_srf`, `.flow_along_nurbs` | Source/target parameterization and offset fixtures | Patch/untrimmed NURBS subset, not complete Rhino FlowAlongSrf |
| `worldwright.tool.run/list` and paired OrbWeaver nodes | Same inputs produce the same shared-kernel outputs | Graph canvas and document bake remain separate work |

Use the precise payloads in `BUILDERCRAFT_API.md` and the paired tool list. Do not
assume legacy 2D command aliases are 3D equivalents. LAS preview and procedural
noise are currently shared library APIs, not complete Scan/Terrain interfaces.

## Observed local checkpoint

The reconciled increment passed 646 workspace tests, all six CI gates, the
kernel validation wrapper and native CAD/CLI compilation on Linux Rust 1.95.0.
CLI control-curve `.dftba` save/reopen and the PushPull example above ran
successfully. The drafting viewport was rendered and inspected headlessly.
This is not a completed interactive desktop or four-platform release sign-off.
