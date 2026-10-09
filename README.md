# Worldwright: open-source design environment

Worldwright combines independent CAD, Scan, Graph and Show apps around shared geometry, scene data, commands and versioned bridges. The current alpha is the CAD desktop; the other executable apps and their planned capabilities are not implemented yet.

| App | Product direction |
|---|---|
| CAD | Rhino-style 3D CAD, NURBS/Brep modeling, full command coverage, layers and body browser |
| Scan | GOM/PolyWorks/DesignX-inspired mesh repair, best fit, inspection and scan reconstruction |
| Orb Weaver (Graph) | Native typed procedural graph for Grasshopper-style workflows, embedded in CAD and independently usable; Houdini-style/VFX extensions later |
| Show | Lighting, AV and automation organization, paperwork, patch, pre-cueing and previs |

Every app should operate independently, with optional file/live bridges to BuilderCraft or proprietary tools. Early immersive walkthroughs from massing models are a core requirement.

Start with [suite architecture](docs/architecture/SUITE.md), [delivery roadmap](docs/roadmap/SUITE_ROADMAP.md), [Rhino command inventory](docs/commands/README.md), [show/previs](docs/architecture/SHOW_AND_PREVIS.md), [memory policy](docs/architecture/MEMORY_AND_JOBS.md) and [StructureGraph reuse](docs/architecture/STRUCTUREGRAPH_REUSE.md).

## Current CAD alpha 0.1

A standalone Rust CAD application based on CADCraft, adding a modeling workspace, named model organization, and initial 3D NURBS tools. No AI, account, Rhino license, or internet connection is required to use the application after installation.

## Run from source
Requires Rust 1.95 or newer and the normal system dependencies for eframe/wgpu.

```sh
cargo run -p cadcraft -- --sample
```

The package name remains `cadcraft` to keep upstream integration simple; the desktop workspace is Worldwright. The `--sample` option opens an inherited drafting sample. Choose **3D** to create a 3D curve or control surface. Drag the viewport to orbit and scroll to zoom. Expand **Control points** in the Model Browser to edit XYZ coordinates. Save as `.dftba` to preserve 3D objects and named organization.

Switch to **2D / Drafting** for existing CADCraft drafting. The workspace selector restores the CADCraft-style layout. This alpha's 3D view is orthographic with an orbit camera; perspective projection, four viewports and 3D snapping are next steps.

## Alpha tools
- Original Rust rational 3D B-spline curves and tensor-product control surfaces.
- Surface wireframe display, orbit and zoom; numerical control-point editing.
- Undoable curve/surface creation, editing, naming and visibility.
- Named assemblies, components and bodies independent of layers; body creation from selected objects; descendant selection and visibility.
- Versioned native `.dftba` project envelope preserving supported drafting data, model organization and exact 3D control data.
- Shared command API via the inherited CLI, loopback JSON control channel and MCP.

Bodies in this alpha are named owners of geometry, not a claim of watertight solid topology. Trimming, booleans, solid modeling, mesh processing and scan metrology are not implemented yet. Original CADCraft drafting tools are inherited and must be evaluated against the files you use; this is not full AutoCAD or Rhino parity.

## API
```sh
cargo run -p cadcraft -- --sample --control 7979
```

See docs/BUILDERCRAFT_API.md and docs/FORMAT_SUPPORT.md. The control endpoint is optional and local. Every modeling mutation runs through the command engine and its undo transactions.

## Git
Branch: `buildercraft/alpha-foundation`. Upstream: https://github.com/storytold/cadcraft . User-owned repository: https://github.com/cameronprops/BuilderCraft . Source imported from local commit `8509232c17f4763137d8714946f48d9cc0511863`; `history/BuilderCraft.bundle` preserves the original Git history. Keep upstream updates separate from BuilderCraft feature branches.

## Procedural modeling
The first headless **Orb Weaver** typed graph evaluator and ten paired CAD/Graph numeric operations are authored (compilation pending), with no graphical node canvas or Grasshopper data-tree parity yet. See [procedural modeling](docs/PROCEDURAL_MODELING.md) and the [dependency plan](docs/dependencies/README.md).

## License and attribution
MIT OR Apache-2.0, retaining CADCraft's copyright notices and third-party attribution. BuilderCraft is an independent fork, not an ArtCraft product. Upstream trademark assets have been removed from the current source tree; upstream history remains intact.

The first shared scene/geometry kernel is implemented in `crates/kernel`; see [kernel usage and limits](docs/architecture/KERNEL.md). Embedded CAD parametric modeling and native Kangaroo-style solving are accepted roadmap items.

A local saved-project visualization feed and GLB export are available; see [Unreal bridge setup and validation status](bridges/unreal/README.md). [Production organization](docs/architecture/PRODUCTION_ORGANIZATION.md) now persists scene/effect and department relationships.

## Native CAD and parametric coverage

Current priority is the CAD workspace with native Rhino-like tools, a full embedded Grasshopper-style workspace and SolidWorks-style sketch/feature workflows. The component-by-component build register is [Grasshopper 1 and Kangaroo 2](docs/components/README.md), containing 817 and 110 public-index entries respectively. None is currently marked implemented; runtime/version reconciliation and full port/tree contracts remain pending.

The 3D viewport offers Top, Front, Right, Isometric and Fit; drag to orbit,
Shift-drag to pan, and scroll to zoom. Fit frames visible control hulls with
a bounded traversal. Native polygon meshes can be clicked to select individual
triangle or quad faces, highlighted in the wireframe, then edited using the
existing undoable Delete Face command. The selection is tied to the document
revision. This first picker only considers polygon meshes; edge/vertex picking,
shaded occlusion, perspective and full parametric authoring remain future work.

The new native filename is `.dftba`; existing `.bcraft` projects remain readable.
For local source validation (no GitHub Actions), run
`bash tools/verify-worldwright-kernel.sh` on Unix/macOS or
`powershell -ExecutionPolicy Bypass -File tools/verify-worldwright-kernel.ps1`
on Windows. The mesh UI changes are still undergoing local build validation.

## Building native CAD and Orb Weaver nodes together

The [dependency hierarchy](docs/dependencies/README.md) routes all 4,356
catalogued references to provisional dependency groups, with 2,347 still
unclassified pending manual review. The [shared typed dispatcher](crates/kernel/src/shared_tools.rs)
owns the implementation for ten native point, vector and polyline tools; the
CAD commands and the Orb Weaver graph engine call it rather than duplicate
algorithms. These new files and authored tests await local Rust compilation.
Run `python3 tools/build_dependency_index.py --check` to verify catalog
coverage and `python3 tools/check_paired_tools.py` to catch drift between
the reference plan, kernel, CAD commands and Orb Weaver node identities.

