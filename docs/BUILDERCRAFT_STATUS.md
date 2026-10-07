# BuilderCraft development status

## Current milestone
First source alpha: standalone Rust workspace and original 3D NURBS evaluation, with named model organization and native persistence.

## Next milestone
- Connect browser actions to one unified selected 3D/drafting object model; handle delete/reparent/instances and orphan cleanup comprehensively.
- Add 3D snapping, construction planes, typed XYZ input, move/rotate/scale and four-view layouts.
- Add NURBS weight/knot editing and construction workflows for extrusion, revolve, loft and sweep.
- Establish a Rust-compatible B-rep topology/kernel choice before trimming or solids.
- Add a mesh geometry representation, OBJ/PLY/STL adapters, diagnosis, bounded smoothing, hole filling, offset and shell pipelines.
- Add API version/capabilities negotiation, document change subscriptions and per-adapter conversion reports.

All work remains within the same project, document, commands, body organization and persistence architecture. Scan & Mesh is a workspace, not a duplicate application.

## Completion standard
Each tool must create/modify real geometry, preview or clearly show its result, cancel safely, undo, persist correctly and have meaningful numerical or round-trip checks. Wired menu items are not feature completion. The alpha does not yet meet full manufacturing CAD or inspection accuracy requirements.

## Verified alpha 0.1
- Desktop and CLI built and desktop run on Linux with software graphics.
- All six repository gates passed: formatting, clippy, 326 workspace tests, asset attribution, crate layering, and WebAssembly checks.
- Live desktop API smoke test passed: create curve/surface, assembly-component-body ownership, hide/undo, control-point edit/undo/redo, save/reopen, and rejection of lossy export without overwriting an existing file.
- NURBS palette button was clicked through the live UI; it created real geometry and undo removed it.
- Windows/macOS unsigned build workflow is configured but has not been run. No installer for those systems has been produced here.
