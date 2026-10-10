# Dedicated Mesh Repair workbench (alpha slice)

Purpose: a scan/surface-oriented workbench, visually and behaviorally distinct
from precision CAD and OrbWeaver. Inspired by PolyWorks IMEdit's interaction
model, with inspection workflow influences from GOM/ZEISS INSPECT. This is
original egui/Rust UI. No commercial UI art or source was copied.

## Entry and visuals

Choose Window → Workspace → Mesh Repair or run
`ui.workspace.mesh_repair`. Return via "Return to CAD" or Modeling.
It has a graphite/teal chrome, dark slate mesh viewport with shaded native
polygon facets, a left Project Explorer with measured mesh and nominal
reference sections, and a right Surface Tools panel with quality checks and
revision-safe mesh repair commands. The current viewport is painter-based:
flat polygon fills are **not occlusion-correct shaded rendering** yet.

The CAD editor retains its original toolbars, shortcuts, drafting grids,
direct modeling and OrbWeaver. All operations share one document and one
Rust topology/repair backend, without importing duplicated math.

## Scoped keymap (first pass)

- **Spacebar** toggles Navigation ↔ Selection while in Mesh Repair, matching
  PolyWorks selection activation conventions. Navigation is the entry default.
  Selection mode allows the existing mouse-driven mesh-face picker; dragging
  orbits and Shift+drag pans in both modes.
- **F** fit visible geometry.
- **I** run mesh topology inspection, using headless `mesh3d.topology` and
  `mesh3d.boundaries`. Defects appear in the reused vertex overlay.
- **Esc** clears transient face pick and returns to navigation mode.
- Typing in fields and the Command Search dialog takes precedence. Space
  retains all prior CAD behaviors outside Mesh Repair.

Source for PolyWorks Space convention:
https://gmv.cast.uark.edu/scanning/software/polyworks/shortcut-guide-polyworks/polyworks-interface-basics-2/

## Live operations

The right panel offers revision-tagged boundary hole filling, picked-face
deletion, and a manual index-based quad-strip split wired through
`mesh3d.edit` (inherited from prior PR #40). All are undoable. Inspection
and its local report are transient, keyed to document UID and revision.
Nominal alignment, surface deviations/color legend, live point-cloud selection,
polygonal/volume selection and high-throughput scan rendering are future
dependencies, not represented as operational features.

## Acceptance

UI tests cover the per-workspace Space mode switch, non-destructive/revision
inspection and undoable repair. Workspace layout serde remains backward
compatible with old profiles through defaulted optional fields. Cross-platform
native CI required before merge.
