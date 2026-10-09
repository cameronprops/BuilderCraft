# Worldwright: Project Identity

Working product name: **Worldwright**.
Legacy repository and internal crate identifiers may still use BuilderCraft or
CADCraft during migration; do not break build paths and serialization simply to
change visible branding.

Worldwright is an open-source, designer-led, integrated world/experience design
environment: CAD/NURBS, mesh/scan repair, parametric and StructureGraph workflows,
scenic/environment, AV/lighting, ride and animatronics, previs, designer-led BIM,
data/spreadsheets, show paperwork and fabrication.

Brand/visual identity is user-owned; do not commit an auto-generated logo.
Naming migration should separately audit:
- Github repository and remote names (manual repo owner administration)
- Package/application title, installer identity, menus/icons
- Crate and code namespace renaming (backwards compatibility)
- Native project file extensions and user data migrations
- Docs, CLI commands, API identifiers and legacy compatibility aliases

Source control only: GitHub Actions are not required for everyday commits.
Kernel and packaging lint workflows have been switched to manual dispatch
on the development branch. Repository administrators should disable Actions
at repository settings if *all* workflows (including alpha and release) must
be prevented from running automatically. Build/test locally with Rust when
available; do not claim unexecuted tests passed.

## Native Worldwright project file

**.dftba** is the preferred Worldwright project extension. This is a
file-name/format routing change, not a change to the version-1 JSON payload
schema. Existing `.bcraft` projects remain readable and writable as legacy
aliases. Save As in the Worldwright workspace proposes `.dftba`, and desktop
open filters accept both extensions. Opening an old `.bcraft` file and using
Save retains its existing path; use Save As to migrate its filename to
`.dftba`. Do not claim backwards compatibility with future file schema
changes until separate migration tests exist.

Both extensions are project containers; generic DXF/DWG/SVG/PDF export cannot
silently discard 3D geometry or mesh objects.

## Calisoga graph identity

**Calisoga** is the working name for Worldwright's open-source Grasshopper-style
parametric graph engine, from the *Calisoga* spider genus. The native graph
engine is independently usable and intended to appear embedded in the CAD
workspace; it executes shared kernel operations with direct CAD commands.
UI/data-tree parity and a standalone executable remain in development.
The dependency-first tool hierarchy and mapping register live under
`docs/dependencies/`. Native Worldwright files remain `.dftba`.
