# BuilderCraft: themed entertainment CAD suite

BuilderCraft combines independent CAD, Scan, Graph and Show apps around shared geometry, scene data, commands and versioned bridges. The current alpha is the CAD desktop; the other executable apps and their planned capabilities are not implemented yet.

| App | Product direction |
|---|---|
| CAD | Rhino-style 3D CAD, NURBS/Brep modeling, full command coverage, layers and body browser |
| Scan | GOM/PolyWorks/DesignX-inspired mesh repair, best fit, inspection and scan reconstruction |
| Graph | Houdini-style procedural modeling/VFX with Grasshopper data flow and engine pipelines |
| Show | Lighting, AV and automation organization, paperwork, patch, pre-cueing and previs |

Every app should operate independently, with optional file/live bridges to BuilderCraft or proprietary tools. Early immersive walkthroughs from massing models are a core requirement.

Start with [suite architecture](docs/architecture/SUITE.md), [delivery roadmap](docs/roadmap/SUITE_ROADMAP.md), [Rhino command inventory](docs/commands/README.md), [show/previs](docs/architecture/SHOW_AND_PREVIS.md), [memory policy](docs/architecture/MEMORY_AND_JOBS.md) and [StructureGraph reuse](docs/architecture/STRUCTUREGRAPH_REUSE.md).

## Current CAD alpha 0.1

A standalone Rust CAD application based on CADCraft, adding a modeling workspace, named model organization, and initial 3D NURBS tools. No AI, account, Rhino license, or internet connection is required to use the application after installation.

## Run from source
Requires Rust 1.90 or newer and the normal system dependencies for eframe/wgpu.

```sh
cargo run -p cadcraft -- --sample
```

The package name remains `cadcraft` to keep upstream integration simple; the desktop workspace is BuilderCraft. The `--sample` option opens an inherited drafting sample. Choose **3D** to create a 3D curve or control surface. Drag the viewport to orbit and scroll to zoom. Expand **Control points** in the Model Browser to edit XYZ coordinates. Save as `.bcraft` to preserve 3D objects and named organization.

Switch to **2D / Drafting** for existing CADCraft drafting. The workspace selector restores the CADCraft-style layout. This alpha's 3D view is orthographic with an orbit camera; perspective projection, four viewports and 3D snapping are next steps.

## Alpha tools
- Original Rust rational 3D B-spline curves and tensor-product control surfaces.
- Surface wireframe display, orbit and zoom; numerical control-point editing.
- Undoable curve/surface creation, editing, naming and visibility.
- Named assemblies, components and bodies independent of layers; body creation from selected objects; descendant selection and visibility.
- Versioned native `.bcraft` project envelope preserving supported drafting data, model organization and exact 3D control data.
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
The Houdini-like component with Grasshopper functionality is requested but not implemented. See [the shared procedural modeling scope](docs/PROCEDURAL_MODELING.md).

## License and attribution
MIT OR Apache-2.0, retaining CADCraft's copyright notices and third-party attribution. BuilderCraft is an independent fork, not an ArtCraft product. Upstream trademark assets have been removed from the current source tree; upstream history remains intact.

The first shared scene/geometry kernel is implemented in `crates/kernel`; see [kernel usage and limits](docs/architecture/KERNEL.md). Embedded CAD parametric modeling and native Kangaroo-style solving are accepted roadmap items.

A local saved-project visualization feed and GLB export are available; see [Unreal bridge setup and validation status](bridges/unreal/README.md). [Production organization](docs/architecture/PRODUCTION_ORGANIZATION.md) now persists scene/effect and department relationships.
