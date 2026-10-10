# Invited/open CAD alpha user acceptance sheet

**These are test cases, not existing test results.** Sign off against a particular commit, operating system, driver/GPU and real mouse/keyboard. Automated Rust tests alone do not certify an interactive viewport.

| Priority | Manual action | Pass criteria |
| --- | --- | --- |
| P0 | Start native CAD, open sample, switch drafting / modeling | One model, no data loss or view-switch undo |
| P0 | Navigate Top/Bottom/Front/Back/Left/Right/Iso/Perspective, fit all/selected | Stable camera, no NaN/out-of-range zoom; named views and four-view panes share selection |
| P0 | Orbit/pan/zoom while a command or gizmo is active | No competing gesture or geometry mutation; Esc cancels |
| P0 | Create point, line, polyline, spline/NURBS, trimmed surface/BRep and mixed triangle/quad mesh | Native types/IDs survive save/reopen; unavailable exact types reject explicitly |
| P0 | Select each object, shift-toggle, window/crossing, filter, overlapping candidates | Same chosen ID in all panes and browser; hidden/locked excluded |
| P0 | Select vertex/edge/face/control point; inspect identity and move/rotate/scale | Revision-safe subobject pick; precision transform, cancel and undo preserve source |
| P0 | Toggle Wireframe and Shaded per pane on a mixed scene | Actual front/back/depth/selection behavior and reliable display-proxy cache; not falsely exact |
| P0 | Create material, change base color and opacity; assign to two objects | Same material ID and visible style; save/reopen and assign without changing topology |
| P0 | Add layers/sub-layers, move object, hide/lock/isolate, use scene browser | Consistent viewport selection/visibility, editor and retained hierarchy |
| P0 | Define block, insert two instances, edit shared definition, undo | Both instances update from one definition; instance transforms remain distinct; recursive cycle rejected |
| P0 | Group two mixed objects, edit group, add/remove, ungroup | Original IDs/geometry/layers intact; select group/member mode works |
| P0 | Place reference image, calibrate width, lock, opacity, linked-file relink | Stable scale and placement; image does not steal geometry selection |
| P0 | Large scene and exact/mesh combination, save/open/export | Bounded latency/memory, diagnostics, honest unsupported-format warnings |
| P0 | F1 Help, open manual locally, find and follow current command | Docs distinguish current and future functions, available without internet |

**Test log template:** commit SHA; OS and version; GPU/driver; resolution/scaling; input method; test step; PASS/FAIL/BLOCKED; reproduction, screenshot/log link; persisted fixture path. Repeat on Windows, macOS and Linux; Haiku native in its own separately maintained gate. Failure of any P0 requirement blocks claiming the corresponding complete alpha capability.

Additional existing technical gates: [repository-wide smoke test](../ALPHA_SMOKE_TEST.md), [BRep acceptance](../roadmap/IMMEDIATE_MODELING_OPENSCAD_AND_ALPHA_UI.md) and [paired operation validation](../dependencies/README.md).
