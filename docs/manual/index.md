# WorldWright manual

**Version:** early development / October 2026. **The manual is part of alpha**, but is not a claim that every described feature works yet. Features are tagged **Existing foundation**, **Alpha gate**, or **Beta/extended**. Exact capabilities come from the compiled release notes and [alpha smoke checklist](alpha-acceptance.md), not from an unverified design sketch.

WorldWright is an open-source, donationware design suite. CAD is the first native application; OrbWeaver is the parametric graph system. Drawing, meshes, NURBS, assemblies, layers and reference materials are intended to live together with one document model. A display tessellation is not a replacement for exact source geometry.

## Start using the current development build

1. Install Rust 1.95+ and native dependencies listed in [development environment](../architecture/DEVELOPMENT_ENVIRONMENT.md).
2. At the source repository root run `cargo run --locked -p cadcraft -- --sample`.
3. Choose the **3D** workspace for the current orthographic model viewer. **2D / Drafting** exposes CADCraft drafting.
4. Orbit by dragging, Shift-drag to pan, wheel to zoom. The source includes Top, Front, Right, Isometric and Fit controls. Click a supported visible model wire or mesh face, Shift-click to toggle. Exact and mesh interaction remains incomplete, so verify against [selection behavior](selection.md).
5. Save a native `.dftba` test file and reopen it. Never rely on exported formats retaining unsupported 3D or organizational metadata.

## Find a topic

| Topic | What you will learn |
| --- | --- |
| [Viewports](viewports.md) | Cameras, construction planes, viewport layouts, navigation |
| [Selection and transforms](selection.md) | Object types, selection filters, subobjects and gizmos |
| [Scene organization](scene-organization.md) | Layers vs groups vs blocks vs assemblies, scene browser and ownership |
| [Display and materials](display-materials.md) | Wireframe, shaded, alpha material model and rendered beta |
| [Reference artwork](references.md) | Image planes, calibration, links, locking and source rights |
| [Alpha acceptance](alpha-acceptance.md) | User-facing test run and feature coverage matrix |

**Documentation location:** these Markdown files ship with the repository and are readable offline. Optional local website: install MkDocs separately, then `mkdocs serve` at repository root. Do not require internet access for editing or consulting this manual. Proposed GitHub Pages hosting is a convenience, not the master copy.

**Terminology:** *existing foundation* means there is source code for some behavior, **not** that a distributable build was manually certified across OS targets. *Alpha gate* means required before a usable invited/open alpha. *Beta* means intentionally deferred. Errors and unsupported operations must be visible rather than silently approximate geometry.
