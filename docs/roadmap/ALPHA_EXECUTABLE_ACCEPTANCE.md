# Alpha executable acceptance gates

Status: **implementation underway**, not an alpha release certification.
This checklist is for a **compiled desktop application**, not a paper command
inventory. The mandatory gate is a real Windows/macOS/Linux executable plus
an actual, inspectable 2D and 3D UI; Haiku is an experimental port target.
The first automated slice runs `tools/smoke-worldwright-alpha.sh` in Linux
native CI **after** the desktop and CLI binaries have been compiled.

## First automated executable gate

| ID | Minimum user-visible result | Verification | Release implication |
|---|---|---|---|
| A01 | CAD desktop binary exists after building | `cargo build --locked -p cadcraft -p cadcraft-cli`, assert both binaries | Necessary, not proof it opens |
| A02 | Global command catalog is discoverable and includes Line, Circle, Move, Undo, Redo | Run `cadcraft-cli commands` and parse JSON | Registry only, not behavior |
| A03 | Create genuine model and save native document | Compiled CLI writes `.dftba` floorplan fixture | Persistence first pass |
| A04 | Reopen native file and inspect, then save again | Compiled CLI `info` / `run` / `--save` | File decoder/encoder smoke |
| A05 | Translate DXF, reopen and inspect | Compiled CLI `convert` in both directions | Format path smoke, not fidelity |
| A06 | Switch between 2D and 3D without altering revision, selection or command history | egui workspace command and serialization unit/integration tests | Testable UI-state invariant |
| A07 | Shared mesh-scene operations and OrbWeaver graph executor work | Existing kernel mesh_scene and OrbWeaver Rust tests | Kernel integration, not GUI parity |
| A08 | Same code compiles and tests on three native operating systems | Existing GitHub native CI matrix | Portability; runtime GUI still untested |
| A09 | Command palette searches actual engine/UI registries with bounded results, stable ranking and no synthetic commands | Run `cargo test --locked -p cadcraft-ui-egui command_palette::tests` | Search logic gate; keyboard/mouse acceptance remains manual |

The executable gate is **fail-closed**: scripts use `set -euo pipefail`,
parse returned JSON, require nonempty artifacts, and reject unsupported
operations. Source compile, registry membership and roundtrip success are
distinct checks.

## Mandatory hands-on alpha walkthrough

A tester must perform the following with a real Worldwright desktop window.
Until run and recorded on each platform these are **not verified**.

1. Open the app to a usable blank 2D page. Create a line, polyline, circle
   and dimension through keyboard commands and palette buttons.
2. Open the Command Palette with Cmd/Ctrl+K. Search by ID/alias, navigate with arrow keys, submit with Enter, dismiss with Escape, and verify Window-menu access.
3. Select with mouse, toggle snap/grid/ortho, inspect precise properties,
   change a layer, hide/show, and verify contextual right-click actions.
4. Undo, redo, copy, move, rotate, scale and delete; confirm exact geometry
   and the selection remain correct, including after zoom/pan.
5. Switch **2D Drawing -> 3D Modeling -> Back** via top bar, Window menu,
   typed `ui.workspace.*` commands and Alt shortcuts. Verify no source
   geometry, revision, selection or history changes merely from switching.
6. Create/edit an exact NURBS curve, surface and polygon mesh; orbit, pan,
   zoom, select, pick mesh faces and use gizmo controls. Test degeneracies.
7. Save native `.dftba`, close and reopen; check object IDs, units,
   hierarchy, material assignments (when supported), and available history.
   Open an older `.bcraft` and check compatibility.
8. Import and export one actually supported fixture per format. Check
   geometric fidelity, coordinate systems, materials and unsupported fields.
   STL/OBJ/LAS adapters don't count as desktop import until wired through
   document transactions and file dialogs.
9. Run OrbWeaver over the **same** geometry operation as the CAD command;
   edit an input, recompute and verify deterministic output and undo.
10. Inspect the UI with keyboard only, high text zoom, reduced motion,
   focus visibility, and obvious Cancel/Back paths. Verify no modal traps.
11. Relaunch on each claimed host (Windows, macOS, Linux), repeat save/reopen,
    and record screenshots, crashes and platform-specific input issues.

## Next command groups to close alpha gaps

| Group | First-pass commands and features | Priority |
|---|---|---|
| Document | New, Open, Save, Save As, Undo, Redo, Units | P0 |
| Drafting | Line, Polyline, Rectangle, Circle, Arc, Trim, Extend, Offset, dimensions | P0 |
| Editing | Select, Move, Copy, Rotate, Scale, Erase, control points, basic gizmo | P0 |
| Organization | Layers, visibility/lock, object names, groups/blocks, hierarchy | P0 |
| 3D | Curve, surface, mesh, primitive solids, extrusion, transformations | P0 |
| 3D navigation | Top/Front/Right/Iso, Fit, Orbit, Pan, Zoom, picking | P0 |
| Interchange | Native `.dftba`, legacy `.bcraft`, DXF and one tested mesh format | P0 |
| OrbWeaver | Input parameter, value, arithmetic, shared geometry node, graph execution | P0 |
| Secondary | Full layer style workflows, constraints, complex booleans, drawings | P1 |
| Beyond alpha | Advanced mesh reconstruction, terrain, tooling, fabrication, simulations | P2 |

Do **not** equate command registration with implemented/correct command
behavior. Existing feature parity registers remain authoritative for
individual Rhino and Grasshopper tool equivalence.

## Where test evidence lives

- `.github/workflows/worldwright-native-ci.yml`: three-platform CI and
  Linux compiled binary smoke
- `tools/smoke-worldwright-alpha.sh`: actual process calls to compiled CLI
- `crates/ui-egui/src/workspace.rs`: reversible UI-only transitions
- `crates/ui-egui/src/buildercraft.rs`: actual egui 3D viewport
- `crates/ui-egui/src/cmdline.rs`: global command line
- `crates/ui-egui/src/menus.rs`: keyboard/menu workspace commands
- `docs/architecture/UNIFIED_WORKSPACE_EXPERIENCE.md`: interaction contract

Desktop screenshot/manual acceptance remains a separate gate until an
instrumented display runner and tester reports provide evidence.
