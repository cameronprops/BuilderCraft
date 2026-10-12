# Worldwright open-source fusion: one core, optional modules

Status: architectural acceptance direction, **not** a claim that every upstream
has been integrated. Reviewed against Worldwright main and selected FreeCAD
source on 2026-10-11. Product: Worldwright; repository: BuilderCraft.

## Goal

Build one usable free/open-source CAD, procedural modeling, scan, fabrication
and themed-entertainment environment. Start from proven upstream math when it
is demonstrably better. Independently implement genuinely missing tools. Do
not conflate a collection of binaries with one coherent data model.

The desired user experience is a unified creative shell, not a forced
FreeCAD/Rhino/Blender/PolyWorks look-alike. Commands, modes and palettes should
adapt to the task while objects and revisions remain intact.

## Where code belongs

| Owner | Current truth | Fusion policy |
|---|---|---|
| Exact trim/topology/Boolean | Current OCCT-backed BRep worker and native document envelope | One OCCT implementation. Borrow narrowly scoped FreeCAD/OCCT algorithms only when they improve robustness; never run competing BRep kernels silently |
| Rust geometry/NURBS | Existing rational curve/surface, shared numerical primitives, mesh/point representations | Reuse native code; compare Curvo, Truck, cadcore and other legal sources with fixtures before replacement |
| Sketches/features/assembly | Existing CAD drafting constraints and optional scoped feature history | Borrow isolated FreeCAD Sketcher/PartDesign techniques; reuse current constraint and history services or replace measured weak spots |
| Dependency traversal | **Experimental FreeCAD-derived LGPL crate** `crates/freecad-graph` wired to native history impact | Numeric, iterative, bounded traversal used through CAD API; no alternate graph solver |
| Parametric/procedural | OrbWeaver Rust evaluator and shared command registry | Add typed data trees, solvers, preview/bake, graph history. Do not package a second math implementation for each node |
| Mesh/scan | Native polygon tools and in-progress repair PRs | Evaluate Blender/OrcaSlicer/MeshLab/FreeCAD Mesh for topology repair and sampling; promote only benchmark winners |
| Fab/CAM | Native surface/mesh output and limited format adapters | Selective slicer/3MF/arrangement/laser/CNC modules; machine execution and postprocessors have separate safety gates |
| Document/UI | CADCraft-derived Rust egui shell, editable .dftba and command API | One identity model, one command service, familiar task workspaces and customizable palettes |
| Previs/show/ride | Unreal bridge prototype, planned controller/rail/frame tools | Keep engine/render/IO workers optional; host-specific adapters do not own core algorithms |

**No automatic bulk import** of Qt, Coin3D, FreeCAD Python runtime,
Blender's entire editor, or OrcaSlicer's application shell. Reuse source at
the smallest stable boundary. Copyleft modules remain properly attributed and
license-identifiable; distribution must meet each license's conditions.

## Scheduling and smart memory management

The scheduler belongs in one shared Worldwright runtime, not inside each
workspace. Later host modules must observe these invariants:

1. An object has a stable project/document/object identity and an authoritative
   geometry representation. Display/repair/toolpath proxies carry source
   revision, tolerance and provenance.
2. Immutable geometry and reference-counted leases support copy-on-write
   document/undo snapshots. Avoid repeated serializations and duplicate
   CPU/GPU buffers when the current backend supports sharing.
3. Each job declares estimated peak input, working, output, cache and native
   process bytes before admission; each workspace has a budget and cancellation
   token. A 3D viewport repaint must never trigger an unbounded Boolean or
   full-scene export.
4. Admit jobs within an aggregate RAM/VRAM cap. Expensive OpenCascade C++
   work stays isolated out of process with timeouts and resource ceilings.
   A worker crash cannot commit incomplete geometry.
5. Cache only derived assets keyed by identity, source revision, numerical
   settings, format and backend version. Evict disposable display caches before
   authored objects or undo state. Keep the last valid preview on failure.
6. Probe hardware capabilities locally, with user-visible overrides.
   Prefer CPU for exact topology/robust predicates and GPU for rendering and
   measured batch kernels; dispatch by benchmarks, not a fixed GPU-first rule.
   Concurrency and thresholds must adapt to cores, available RAM/VRAM,
   drivers and installed backends.
7. Preserve interactive input responsiveness and prioritize visible-scene
   tasks; enqueue exports, simulations and scan processing cooperatively.
   Never run heavy workloads on UI repaint.
8. Validate memory limits under real long-lived sessions, pathological models,
   history churn, open/close cycles, and hardware scarcity. Rust ownership
   alone does not prove bounded total memory.

**Current implementation is partial:** the repository has retained-geometry
budgets, job cancellation in selected paths and bounded BRep IPC/viewport
proxies. It does **not** yet have a complete machine-profiler, cross-module
RAM/VRAM allocator, adaptive scheduler or full GPU/CPU optimization. Those
remain acceptance work, not badges granted by documentation.

## License/provenance separation

- Original inherited CADCraft source remains under its original headers.
- External GPL/LGPL/MPL code enters only with exact source path, pinned
  upstream revision, file-specific license, contributor credits, a retained
  license copy, tests and distribution compliance review.
- The first code-derived FreeCAD subsystem is `crates/freecad-graph`, a
  small LGPL-2.1-or-later Rust port of the iterative incoming-dependency walk
  in FreeCAD `src/App/DocumentObject.cpp` at
  `ec6852e6caadf55045881a6003cc7ad038c8d2a7`. It computes graph
  reachability for Worldwright's history-impact query. This is not FreeCAD
  Sketcher, BRep, UI or full parametric engine.
- LGPL in a linked product can require notices, source and a means to use a
  modified version of the library. Source-only review branches must not be
  promoted into downloadable releases before packaging compliance is checked.
- A donation-supported free product is the current target. Commercial
  flexibility is not a criterion for rejecting otherwise legal reusable code.

## Focused adoption sequence

1. Validate current OCCT exact BRep/roundtrips and close UI picking,
   exact edit, save/reopen and failure-preservation tests.
2. Land the FreeCAD-derived dependency walker with native Rust CI, then build
   selective revisioned feature recomputation using it, without double work.
3. Compare FreeCAD Sketcher constraints and datum-plane/PartDesign operations
   against the existing Worldwright solver and OCCT worker with matched
   tolerance/DOF fixtures. Only replace weaker implementations.
4. Execute CAD and OrbWeaver operations through the same typed port registry;
   integrate an interactive component canvas and revision-safe preview/bake.
5. Finish Scan repair/align, fabrication validation, source-aware export,
   memory profiling and model-conformance fixtures.
6. Use the resulting stable services for optional host plugins later. Host
   integration is intentionally paused until the shared core passes its gates.

**Alpha stop condition:** a user can launch the CAD app, model and edit in a 3D
viewport, use solids and editable curves/meshes, select, undo, save/reopen
.dftba, exchange supported formats, and evaluate a linked OrbWeaver graph with
passing automated tests. No full Rhino or FreeCAD command parity is needed
to call that particular vertical slice an alpha.

See `docs/dependencies/FREECAD_WORKFLOW_REUSE.md`,
`docs/dependencies/OPEN_SOURCE_REUSE_CANDIDATES.json`,
`docs/architecture/MEMORY_AND_JOBS.md`, and
`docs/architecture/COPYLEFT_AND_DONATIONWARE.md`.
