# Worldwright fabrication: OrcaSlicer source audit and shared tooling boundary

Reviewed 2026-10-10 against [OrcaSlicer main](https://github.com/OrcaSlicer/OrcaSlicer/tree/07a12f5fe46ea450975c4cb65a653f1bb94eaaa4), the [Orca CLI wiki](https://github.com/OrcaSlicer/OrcaSlicer/wiki/cli_mode), and the Worldwright `main` baseline `618d6d4`. **All features in this document are candidates unless marked as already implemented elsewhere. No slicer, CNC CAM, or laser toolpath integration is claimed.**

## Decision

Add fabrication as a **workspace and headless workflow over existing shared geometry**, not as another canonical geometry/kernel implementation. CAD, OrbWeaver, Mesh Repair and fabrication use identical mathematical operations and immutable source IDs/revisions. Manufacturing output is derived and must never silently replace exact curves/BRep, native triangle meshes, units, parametric data, or document history.

For alpha, **prefer file-based OrcaSlicer handoff, then an optional local out-of-process CLI adapter**. Orca is a substantial C++/wxWidgets/CGAL/libigl slicer application, not a small Rust geometry crate. Do not vendor its whole source tree or link it into `buildercraft-kernel` to gain isolated mesh operations. Orca's internal algorithms remain relevant reference or later selective AGPL integration, subject to cross-platform build and licensing gates.

### Orca source-level findings

| Source / system | Actual role | Worldwright decision |
|---|---|---|
| [libslic3r/MeshBoolean.cpp](https://github.com/OrcaSlicer/OrcaSlicer/blob/main/src/libslic3r/MeshBoolean.cpp) | Includes libigl+CGAL mesh booleans; also CGAL polygon soup orientation, mesh repair, remeshing, manifoldness and MCUT operations | **Do not duplicate kernel**; compare with existing Worldwright topology/repair work and independent `manifold-rust`; require manifold/degenerate fixtures before considering any foreign backend |
| [libslic3r/TriangleMesh.cpp](https://github.com/OrcaSlicer/OrcaSlicer/blob/main/src/libslic3r/TriangleMesh.cpp) and [QuadricEdgeCollapse.cpp](https://github.com/OrcaSlicer/OrcaSlicer/blob/main/src/libslic3r/QuadricEdgeCollapse.cpp) | Mesh storage, statistics and decimation | Benchmark and reuse approaches where better; preserve authored mesh attributes/IDs and bounded triangle budgets |
| [libslic3r/CutUtils.cpp](https://github.com/OrcaSlicer/OrcaSlicer/blob/main/src/libslic3r/CutUtils.cpp) | Part division and connector operations, including clearance/tolerance geometry | High-value behavior reference for print-size segmentation and connectors, not a copy of generic mesh Boolean code |
| [libslic3r/Arrange.cpp](https://github.com/OrcaSlicer/OrcaSlicer/blob/main/src/libslic3r/Arrange.cpp), [Orient.cpp](https://github.com/OrcaSlicer/OrcaSlicer/blob/main/src/libslic3r/Orient.cpp) | Build plate placement and orientation | Reuse through external slicer initially; later shared bounded nesting/placement primitives with domain-specific objectives |
| `libslic3r` slicing, support, infill and G-code generation | Mature FDM-specific manufacturing logic | Keep in Orca adapter first. CNC G-code and laser motion/power are **different** postprocessing/safety domains |
| [libslic3r/CAD/GeometryEngine.cpp](https://github.com/OrcaSlicer/OrcaSlicer/blob/main/src/libslic3r/CAD/GeometryEngine.cpp), [SketchSolver.cpp](https://github.com/OrcaSlicer/OrcaSlicer/blob/main/src/libslic3r/CAD/SketchSolver.cpp) | Experimental Orca Design CAD uses OpenCascade and SolveSpace constraint solver | Confirms useful common backend choices; **does not replace** Worldwright's exact BRep P0 acceptance or justify a second document/kernel |
| [STL Transformation wiki](https://github.com/OrcaSlicer/OrcaSlicer/wiki/prepare_stl_transformation) | "Fix Model" uses **Windows Netfabb API**; simplify is a separate tool | **Not** a portable repair backend. Worldwright's native Scan/Mesh Repair must work on macOS, Linux, Windows and Haiku |
| [CLI actions](https://github.com/OrcaSlicer/OrcaSlicer/wiki/cli_actions) / [CLI misc](https://github.com/OrcaSlicer/OrcaSlicer/wiki/cli_misc) | Headless slice, export, orient/arrange, load presets, bounded slice parameters | Implement optional local process adapter only after exchange and profile validation; output errors not success-by-exit-code alone |

Orca upstream states [AGPL-3.0](https://github.com/OrcaSlicer/OrcaSlicer/blob/main/LICENSE.txt); copied/linked material requires a detailed combined-work review, source/attribution and third-party license audit. A separately installed executable receiving user-directed files may have a different compliance posture, **not a blanket licensing exemption**. The optional Bambu network plugin contains non-free dependencies: do not make it a required Worldwright component or silently bundle it. The Orca **Design** tab and Python plugin system are experimental/nightly or post-2.4.2 features as of review; no stability or API contract is assumed.

### Complementary sources to test, not blindly import

- [manifold-rust](https://github.com/larsbrubaker/manifold-rust), Apache-2.0: pure Rust manifold mesh Boolean candidate, compare against current kernel and BRep boundary. Preserve failure and source topology; a valid manifold mesh is **not** an exact NURBS/BRep.
- [clipper2-rust](https://github.com/larsbrubaker/clipper2-rust), Boost-1.0: portable 2D path clipping/offset candidate for laser kerf, CNC profiles, sliced contours and nesting; independently validate scale and integer overflow. Prefer assessing this pure Rust port before duplicating geometry or introducing a C++ build solely for 2D clipping.
- [OpenCAMStudio](https://github.com/pr0m1th3as/OpenCAMStudio/tree/5c510b504fe39f4e40700ff3b1e331d0309fcad3), GPL-3.0-only: early Rust 2.5D CNC CAM reference. Its claimed working DXF/DWG to profile/pocket, simulation and G-code path needs independent machine/post tests; it is not an established 3/5-axis replacement.

## Manufacturing domain boundary

All domains share `source_document_id`, `source_revision`, stable object/part IDs, millimetre-normalized dimensions, original units/frame, source geometry handles, material/operation tags, derived geometry tolerance, revision hash, provenance, resource/cancellation policy, and conversion-loss report.

1. **Shared geometry / analysis**: topology diagnostics, normals/winding, weld, holes, shells, source-preserving triangulation, plane section, polygon Boolean/offset, trim, split, connectors and thickness/clearance checks. Add a function only when another existing operation cannot do it. Dispatch through CAD commands, OrbWeaver nodes, headless API and fabrication UI.
2. **FDM 3D print**: derive watertight manufacturing mesh, orientation, plate packing, print-segment/connector kits, profile selection, supports, infill, slicer simulation and external output. Choose `3MF` (parts/instances/materials/settings when supported) and `STL` as controlled exchange. Do not claim generic 3MF exchange is equivalent to a printer-vendor-specific project 3MF.
3. **Resin / SLS / MJF**: shared derived mesh and packing/QC, then separate technology-specific supports/arrangement, exposure/build planning and vendor export adapters. An FDM G-code engine does not implement those processes.
4. **CNC**: use exact curves/BRep or verified mesh depending on operation; build stock/tool/work coordinate definitions, 2D profiles/pockets/drilling, cutter compensation and collision/stock verification, then explicit machine/controller-specific postprocessors. Never use slicer G-code as milling G-code.
5. **Laser**: flatten/project with explicit plane and source revision; planar paths, kerf offset, closed-loop/path ordering, nesting, layers mapped to cut/score/engrave, material/power/speed and operation-specific safe postprocessors. A 3D manifold requirement does not apply to open-vector laser artwork.

**Machine safety:** generating files must not automatically arm or run equipment. Require machine profile, tool/material units, bounds and travel limits, operator review, collision/remaining-stock checks for CNC, and laser power/focus/speed checks. Postprocessor and actual device/firmware version are part of the validated output claim.

## Initial implementation gates (do not divert exact BRep P0)

- **F0: geometry contract**. Use the existing native mesh repair and bounded STL/OBJ exchange. Add manufacturing mesh derivation from exact CAD after P0 is validated; assert watertightness/winding/scale and preserve source. Integrate with active Scan UI branches instead of reimplementing their edits.
- **F1: print handoff**. A button or API action produces a validated temporary STL, with explicit units and print tolerances, then hands it to a user-installed OrcaSlicer. Move to a **tested 3MF** writer/reader to preserve assemblies/instances/profiles. External host absence must not break CAD, Mesh Repair or OrbWeaver.
- **F2: Orca CLI**. Optional discovered local executable, bounded child job, temporary isolated folder, sanitized flags/filenames, version check, tested machine/process/filament preset loading, cancellation, progress, output hash, stderr/result diagnostics and typed estimated material/print time. Use the documented `--slice`, `--load-settings`, `--load-filaments`, `--export-3mf` flags; validate on real Windows/macOS/Linux builds. Default no cloud upload, no bundled proprietary networking. On Haiku, portable file handoff remains functional even when the Orca binary is unavailable.
- **F3: shared cut/connector**. Use existing mesh Boolean/topology/planar math after fixture qualification; implement reversible plane cutting, oriented split, pegged and dovetail joint kits with anisotropic clearance, minimum wall, magnet-pocket and print-volume validation. Same typed operation through CAD/OrbWeaver/Scan/Fabrication.
- **F4: 2.5D CNC and vector laser vertical slices**. Source-preserving projected sketch => validated polygon offsets => travel/tool simulation => postprocessor-specific file; demonstrate with one contour/pocket and one closed laser cut/engrave pair before advanced CAM or nesting.
- **F5: advanced tooling**. Multi-plate nesting, print farm, resin/SLS, 3-axis and 5-axis CNC, sheet forming, relief carving, adaptive feeds and kerf calibration. These require independent tests and profiles.

## Non-negotiable acceptance fixtures

- 10 mm closed cube, open cube, reversed winding, self-intersection, duplicate faces and deliberately non-manifold edges: report rather than silently alter source.
- Millimetre/inch round-trip, scale-model miniature, asymmetric multi-part object and unchanged object IDs after export/reimport; no accidental 25.4x resize.
- Known plane-split volume, female/male connector clearance in XYZ, nominal-vs-result wall thickness, cancellation and failed Boolean leaving the design intact.
- FDM sample from existing **BasketWeaver** and **engagement ring** OrbWeaver demos; keep the graphs authoritative, freeze manufacturing snapshot by revision and record actual printed derivative.
- CNC rectangle pocket with tool radius and clearance, plus open/closed vector laser paths with kerf applied to the correct side. Compare simulator envelope and exported instructions; **machine execution requires operator validation**.
- Malformed/bomb 3MF/ZIP, oversized polygon input, invalid/ambiguous preset, missing external host and cancelled job: bounded failure with no document mutation.

## Verified implementation status

- Worldwright current `main`: shared mesh topology/repair primitives and bounded STL/OBJ adapters exist. Editable mesh repair GUI and exact BRep integrations are under active draft PR review. See [open PRs](https://github.com/cameronprops/BuilderCraft/pulls) and [mesh alpha](SCENE_MESH_ALPHA.md).
- **Not implemented or validated here**: manufacturing workspace, 3MF exchange, Orca adapter, FDM slicing, CNC CAM, laser path postprocessing, native connector library or any physical machine trial.
- Environment in this audit: Rust/Cargo unavailable, so no new Rust compile/test result. Documentation-only change; no native performance numbers or printer compatibility implied.
