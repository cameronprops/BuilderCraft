# WorldWright — immediate geometry, OpenSCAD and alpha UI delivery agenda

**Authoritative implementation tracker:** [GitHub issue #26](https://github.com/cameronprops/BuilderCraft/issues/26). Follow its seven ordered stages: (1) punch list, (2) implement, (3) compile and merge, (4) command-line parity, (5) debug, (6) benchmark/deduplicate/prune and migrate all consumers, (7) customizable first-build UI.

## Dependency-first execution queue

| Phase | Shared service prerequisite | CAD / OrbWeaver commands and acceptance | Gate |
| --- | --- | --- | --- |
| 0A | unit/tolerance contract, robust predicates, entity IDs & topology integrity | Validated manifold, trim loop, seam/corner and shape/domain diagnostics | Exact versus approximate status always explicit |
| 0B | NURBS derivatives, adaptive curve sampling, arc-length inversion, analytic surface frames | `EvaluateCrv`, `Divide`, `FrameAt`, `ClosestPoint`, `Rebuild`, `Refit` | Curve domain, knot, inflection, discontinuity tests |
| 0C | shared rail station + orthonormal orientation service | Freeform / roadlike / parallel-transport frames; guide-up, banking, closed-loop twist | Use by ArrayCrv, Pipe, Sweep1, Sweep2, Track |
| 1A | indexed surface grid construction, cross-section interpolation, caps & topology | **Pipe**, variable radius/thick/cap; **Sweep1**, loft, extrude, revolve as composition | Mesh prototype and exact-NURBS tracks labeled distinctly |
| 1B | paired rail synchronisation, cross-section seam registration, continuity fit | **Sweep2** and Track 2–5 rails, height preservation, closed seam/trim | Reject mismatched rails and self-crossing sections |
| 1C | BRep faces/edges/trims, robust intersections, splitting, join/heal, cap | **BooleanUnion / Difference / Intersection / Split**; Trim, Split, Join, Cap | Contact/tangent/coincident/non-manifold golden fixtures |
| 1D | trimmed UV projection, Newton refinement, surface-to-surface UVN mapping | **FlowAlongSrf** AutoAdjust/Rigid/Copy/PreserveStructure, Project/Pull to BRep | Output topology + seam/normal/tolerance tests |
| 1E | face selection + intersections + boolean rebuild on watertight BRep | **PushPull** native BRep faces, shell/offset and local solid edit | Revisioned undo/redo, no broken solids |
| 2A | sealed dependency adapter + source mapping, file bridge | **OpenSCAD** `.scad` source editor; user-confirmed rendering to STL/3MF | CLI-only subprocess first; explicit safety/license checks |
| 2B | history dependency graph and scene handles | Associative parametric sweep/pipe/flow + editable Rhino-style history | Same kernel invoked by CAD/OrbWeaver |
| 3 | performance/identity/provenance register and UI shell | All CLI aliases + interactive prompts + JSON API; customizable dockable UI, palettes, toolbar/presets, shortcuts and persistent layouts | GUI, CLI and platform testing |

**Implementation discipline.** One canonical geometry algorithm per operator. The CAD command, OrbWeaver node, feature timeline, exported API and future visual tools must call it directly. Compare alternative algorithms using repeatable accuracy/performance/memory fixtures; remove weaker duplication only *after* all consumers are migrated and green. No licensing shortcuts. No merged source without real CI evidence. Nonimplemented scopes stay PENDING rather than masquerading as Rhino parity.

**Current verified baseline:** PR #23 merged after full native CI; PR #24 merged after full native CI. PR #25 oriented path arrays remains open and independently gated. Existing NURBS support is not a topology-certified exact BRep kernel. Do not present mesh-polygon sweeps as full exact Rhino Sweep/Pipe.

**OpenSCAD integration architecture:** Use OpenSCAD's documented CLI to launch *explicit user-requested* script evaluation with path allowlisting, timeout/cancellation, output caps, diagnostic capture, and isolated working folder. Never execute on file-open. `.scad` stays authoritative text alongside parameter history and imported mesh. A future linked engine requires separate GPL-2.0-or-later compatibility, dependency/source/notice and distribution audit. Maintain source attribution. No paid host requirement.

**First GUI customizability is part of acceptance**, not a late enhancement: dock/move/resize/hide tools, serialized workspaces, custom keybindings, palette locations, user-defined toolbars, keyboard-accessible command search, discoverable panels, startup reset-to-safe profile, accessible type and reduced motion. UI spirit: Rhino/AutoCAD modeling workflow; Photoshop/Inkscape panel editing; Houdini nodes; ZBrush sculpt tool organization. All visual elements must be original or lawfully reused.

**References:** [Rhino Sweep1](https://docs.mcneel.com/rhino/8/help/en-us/commands/sweep1.htm), [Sweep2](https://docs.mcneel.com/rhino/8/help/en-us/commands/sweep2.htm), [Pipe](https://docs.mcneel.com/rhino/8/help/en-us/commands/pipe.htm), [FlowAlongSrf](https://docs.mcneel.com/rhino/8/help/en-us/commands/flowalongsrf.htm), [BooleanUnion](https://docs.mcneel.com/rhino/8/help/en-us/commands/booleanunion.htm), [OpenSCAD CLI](https://files.openscad.org/documentation/manual/Using_OpenSCAD_in_a_command_line_environment.html).

**Last edited:** 2026-10-09. Maintain the GitHub issue for checkboxes, PR links, blocking failures, and status updates; avoid duplicate source-of-truth statuses in docs.
