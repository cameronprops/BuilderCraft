# FreeCAD systems considered for Worldwright

Review: 2026-10-11. Behavioral references: [FreeCAD source](https://github.com/FreeCAD/FreeCAD), [topological naming guidance](https://github.com/FreeCAD/FreeCAD-documentation/blob/main/wiki/Topological_naming_problem.md), [sketch placement](https://github.com/FreeCAD/FreeCAD-documentation/blob/main/wiki/PartDesign_NewSketch.md), and [spreadsheet workbench](https://github.com/FreeCAD/FreeCAD-documentation/blob/main/wiki/Spreadsheet_Workbench.md).

This is **selective workflow adaptation, not a fork of FreeCAD**. No FreeCAD source, icons, models, Qt/Coin3D frameworks, or documentation text was copied into this increment. FreeCAD is predominantly LGPL-2.1-or-later; source reuse requires a file-level license audit, appropriate notices, platform tests and an objective benefit over existing Worldwright code. The existing OCCT worker remains the only authoritative exact BRep backend.

## Workflow decisions and dependencies

| FreeCAD pattern | Worldwright destination | Current evidence | Next acceptance gate |
|---|---|---|---|
| Parametric document recompute / dependency graph | Scoped feature histories, optional per body, block or document | Native history recipes, revisions, suppression and recomputation exist. This branch adds a **read-only transitive impact query** in the kernel and CAD API, still uncompiled | Rust 1.95 tests, native CI, large-history timing, then revision-keyed selective recompute |
| Sketcher dimensional constraints | Mechanical sketch workbench using existing `cadcraft-constraints` solver | The native drafting solver exists, but a sketch-plane/closed-profile mechanical contract does not | Planes, degrees-of-freedom diagnostics, fully/under/over-constrained fixtures, a sketch-to-solid vertical slice |
| PartDesign Body, Pad, Pocket, Revolution | Body-local history + shared OCCT exact BRep worker | Body organization and scoped history exist. Exact worker is separate from history; feature bindings are missing | Attach a constrained profile to a datum plane, extrude and subtract exact solids with stable references |
| Datum planes and attachment modes | Shared construction frames, local placement and offset attachment | CPlane point input exists; persistent parametric datums do not | Reference identity, offset rules, orientation, units, detach/reattach and undo roundtrip |
| Topological naming and error recovery | Stable typed edges/faces across OCCT operations, explicit remap/error diagnostics | Exact BRep worker exists. History steps must not store raw face indices as stable references | Cut/fillet/rebuild fixtures including topology changes and ambiguous remaps |
| Property editor / expression bindings | Scoped typed parameters and expressions, compact inspector | Local typed history parameters and inherited drafting expressions exist separately | Unified unit-aware expressions, cross-object dependency checks, cycle rejection, no full-document recompute for unrelated edits |
| Spreadsheet-controlled designs and configurations | Named parameter tables and configuration variants | No spreadsheet-model binding validated; do not add a second formula engine | Bounded named cells and configuration-switch tests across CAD and OrbWeaver |
| App Link / reusable parts | Instances referencing shared block/part definitions | Organization and block semantics partially exist | Stable source definition, instance transforms/configs and reference invalidation |
| TechDraw | Associative 2D detail views, fabrication documentation | Inherited 2D drafting exists; 3D-to-2D associative views not validated | Revisioned orthographic section/detail output, scale and dimension roundtrip |
| Path CAM | Fabrication workspace over source-aware geometry | STL/OBJ and some mesh operations exist; no verified machine-ready toolpath kernel | Profiles, offsets, safe entry moves, machine limits, G-code postprocessor acceptance |
| Mesh workbench | Dedicated Scan/Repair workspace | Worldwright has separate native mesh work under active PRs | Repair/undo benchmarks, topology preservation, inspection metadata |
| Workbench and task panel model | Worldwright selectable CAD/Scan/Fabrication/Show modes with user's mixed command shelf | Native UI already has a customizable shelf; specialized mode completeness varies | Per-workspace settings without changing authoritative geometry or requiring a second UI stack |

## Dependency-impact increment

`worldwright.history.impact` reads a validated, already-ordered timeline, takes a source parameter or step ID and returns **potentially affected** step IDs in timeline order, including currently suppressed or rolled-back descendants. The algorithm makes one forward pass over at most 512 steps and reuses the existing timeline validation. It does **not** recompute, cache, edit, bake, or change the document.

Sample headless command:

```json
{"command":"worldwright.history.impact","params":{"scope":{"kind":"document"},"source":{"kind":"step","id":12}}}
```

For a parameter, supply `"source":{"kind":"parameter","id":"width"}`. Unknown IDs and names return errors. Report carries source, scope and timeline revision. Repeated calls are read-only and cannot add undo records. The tests exercise chain/branch isolation, suppression/rollback, unknown sources and command state immutability.

This increment is **source-authored on a review branch, not confirmed buildable** until Rust 1.95 and cross-platform CI have run. It is the prerequisite for skipping unaffected work during future interactive editing, not proof that incremental recomputation already exists.

## Rejected bulk imports

- FreeCAD GUI/Qt/Coin3D: duplicates the active Rust/egui application shell.
- Another OpenCascade or mesh Boolean engine: violates one operation/multiple interfaces.
- All FreeCAD workbenches as bundled runtime dependencies: substantial startup, packaging and licensing surface with no guaranteed correctness gain.
- Copying FreeCAD assets or examples without examining their exact licenses.
