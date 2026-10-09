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

## Manual command continuation: directional scaling

Added original Scale1D and Scale2D kernel/API operations for native exact curves
and control surfaces, with arbitrary explicit axes/plane normals and origins,
nonnegative numeric factors, copying and undo. Existing document persistence and
preview consume the edited exact shape. Interactive aliases, CPlane/reference
picking, Rigid, SubCrv and additional representations remain pending. See
`docs/commands/MANUAL_REBUILD.md` for contracts and acceptance evidence.

Validation: all six `cargo xtask ci` gates passed with 366 workspace tests.
Local development validation disables incremental compilation, uses opt-level 0,
and one codegen unit for naga/egui to avoid invalid dependency object artifacts.
Release profiles and shipped source configuration are unchanged.

## Manual command continuation: nonuniform scaling

ScaleNU world-axis numeric scaling and ScaleByPlane explicit perpendicular-frame
scaling are implemented in the same bounded shared service. Coverage remains
partial for native exact curves/control surfaces. Known-coordinate, rational
curve/surface, failure preservation, copy/undo and project round-trip evidence
is in the transform tests. Interactive and additional geometry options remain
tracked in `docs/commands/MANUAL_REBUILD.md`.

Validation: all six `cargo xtask ci` gates passed with 368 workspace tests,
including the final tilted-plane and rounding-drift acceptance cases.

## ScalePositions increment

Native exact curves/control surfaces now support 1D, 2D and 3D spacing through
`geometry3d.transform`, with Space buttons in the transform panel. Bounds come
from rational span subdivision; objects retain their sizes and parameterization.
Shared batch work limits, copy, undo/redo, failed-operation preservation and
`.bcraft` round trips have regression coverage. See
[command contract](commands/MANUAL_REBUILD.md) for tolerance, scope and pending options.

## Shear increment

Added original bounded affine shear to the shared exact transform service and
the numeric transform panel. Copy, undo/redo, rational parameterization and
project persistence use the existing transaction path. Scope and pending options
are in `docs/commands/MANUAL_REBUILD.md`. Hosted workflows are now manual-only;
ordinary source pushes do not schedule GitHub Actions.

Validation: 378 workspace tests and all six `cargo xtask ci` gates passed on
Rust 1.95. Headless clicks and the rendered transform panel were verified.
Local validation used dependency opt-level 0, one codegen unit and disabled
incremental compilation; shipped build profiles remain unchanged.

## Orient3Pt increment

Added original three-point frame orientation to the shared exact transform
service and numeric UI, with optional first-edge uniform scaling. Existing copy,
undo/redo, resource limits and project persistence apply. Exact rational curves
and control surfaces only; reference picking and other representations remain
pending. See `docs/commands/MANUAL_REBUILD.md` for numerical limits and evidence.

Validation: 381 workspace tests and all six local `cargo xtask ci` gates passed
on Rust 1.95, including the final overflow regression. Headless clicks and the
rendered numeric panel were inspected. Local validation uses opt-level 0, one
codegen unit and disabled incremental compilation; shipped profiles are unchanged.
