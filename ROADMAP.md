# BuilderCraft roadmap

BuilderCraft is a suite for themed entertainment professionals. The accepted scope and delivery gates are maintained in [the suite roadmap](docs/roadmap/SUITE_ROADMAP.md) and [architecture](docs/architecture/SUITE.md).

The CAD alpha exists; independent Scan/Graph/Show apps and new bridge/export features are planned. See [Rhino command coverage](docs/commands/README.md) for documented command inventory and verified-status rules.

The inherited CADCraft roadmap is retained at [docs/upstream/CADCRAFT_ROADMAP.md](docs/upstream/CADCRAFT_ROADMAP.md) for upstream context; its percentages and estimates do not represent BuilderCraft suite completion.

Current native CAD increment: exact 3D Move, Rotate3D, Scale and Mirror services, numeric browser controls and bounded undoable batch edits. Rhino equivalence is partial. The [manual topic register](docs/commands/manual_inventory.json) and [rebuild gates](docs/commands/MANUAL_REBUILD.md) now guide systematic feature work.

### Alpha workspace slice (2026-10-10)

Native desktop: registry-backed keyboard command search; Modeling/Drafting/Focus
presets; one saved custom layout; versioned desktop layout persistence; resizable
Tool Sets/Inspector; compact modeling toolbar and bottom command line.
Acceptance: `docs/ALPHA_SMOKE_TEST.md`. Free docking, shortcut/toolbar editing,
light theme, shaded/material viewport and exact-solid/demo gates remain open.

Tool shelf restoration: registered command pinning, bounded persistent preferences, reorder/remove, and disabled missing entries are source-authored on current main. Restored source passed eight workbench tests, rendered inspection and all six local CI gates on Rust 1.95.

### Exact-solid UI restoration

Inspector primitive and Boolean controls restored against main 8615c7f. Uses existing transactional OCCT engine commands and bounded shaded display service; no new runtime dependency. Document-scoped operands, invalid-input regression and native-worker result/undo acceptance authored. Current-session validation passed: 695 workspace tests, all six gates, required kernel script, native CAD/CLI builds, 21 OCCT tests and four panel tests including real pointer clicks. Rendered panel inspected. Next dependency: exact-solid transforms, followed by OrbWeaver reference resolution and Bake.

### Exact-solid transforms checkpoint

Source-authored `brep.transform` and Inspector Move/Rotate/uniform Scale use the
existing OCCT worker, bounded decoder and document transaction. Preserve object
identity/organization. Native and UI regressions authored; validation pending.
Gizmo/multi-selection transforms and OrbWeaver exact-solid reference/Bake remain
separate dependencies. Missing inherited local plan/craftrules sources were not
used as evidence.
