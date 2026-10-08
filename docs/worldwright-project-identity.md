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
