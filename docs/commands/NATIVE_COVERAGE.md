# Required native, free and open-source coverage

Accepted scope: every documented Rhino command and every Grasshopper/Kangaroo component in the declared reference versions needs an original native implementation or verified open-source equivalent. No proprietary source may be copied, linked into the native app or required to run it. Public availability of code alone does not establish an open-source license. Verify licenses before reuse. Keep optional external bridges separate from native coverage.

## Current facts

- Rhino 8 Windows/Mac public command headings are inventoried in `rhino8.json`. Candidate mappings are not verified native parity. Options, platform differences, UI commands and aliases need separate review.
- Public-index inventories now contain 817 Grasshopper 1 built-in entries and 110 Kangaroo 2 entries in `docs/components/grasshopper1-kangaroo2.json`, with purpose summaries and reference links. Exact installed-version completeness, runtime GUIDs, ports and matching/tree contracts remain unverified. Their native engines/canvases/solvers are not implemented. A native tessellation function or a similarly named node is not evidence of component parity.
- Current CAD viewport is an orbitable orthographic projection with curve/control-surface previews. Named top/front/right/isometric views, bounded control-hull fitting and Shift-drag panning are now implemented. Tested viewport object selection now uses shared sampled-wire hit queries and transient selection commands. Perspective, multiple viewports, subobject/window selection, snaps and robust shaded scene rendering remain work items.

## Inventory and rebuild tasks

1. Declare exact reference versions: Rhino 8, Grasshopper 1 as shipped with that Rhino version, and Kangaroo 2. Record later Grasshopper 2 separately, rather than silently replacing the baseline.
2. Enumerate every built-in component, parameter type and solver goal by stable ID/name/category, inputs/outputs, matching/list/tree rules and public reference. Compare an authorized runtime catalog where available with public indexes to find missing entries. Record obsolete and platform-specific elements explicitly.
3. Inventory interaction behavior too: component canvas, wires, previews, clusters, graft/flatten/simplify, bake, undo, persistent parameters and viewport tools.
4. Implement native contracts, original algorithms and open-source UI; test degenerate inputs, geometry/data-tree semantics, units, determinism, failure preservation, cancellation and memory limits.
5. A working entry requires the native implementation path and acceptance fixtures. Partial entries list unsupported options. Proprietary host results cannot be counted as native working coverage.
6. Optional third-party plug-ins require separate versioned inventories. No fixed catalog can truthfully establish coverage of every third-party or future component.

## Public discovery starting points

- Rhino inventory references are stored per command in `rhino8.json`.
- McNeel Grasshopper 2 documentation: https://www.rhino3d.com/docs/grasshopper2/ (separate reference generation).
- Daniel Piker's public discussion of Kangaroo goals: https://www.grasshopper3d.com/group/kangaroo/forum/topics/goal-source-code-examples
- Public goal-example repository: https://github.com/Dan-Piker/K2Goals . Treat as a discovery lead only; verify applicable licenses before any code reuse. No source from it was copied for this increment.

The suite remains a native free/open-source implementation even when a user optionally connects independently installed proprietary software through an open-source bridge. Such bridges must not substitute for the required native functions.
