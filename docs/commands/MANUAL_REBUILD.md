# Rhino manual to native kernel

Accepted target: systematically rebuild every feature in the latest released Rhino
manual, alongside the full embedded Grasshopper workspace. Current official download
page lists Rhino 8 as released and the next version as WIP. Treat Rhino 8 Windows/Mac
rolling help as the stable baseline; exact installed minor/build remains unverified.
Keep WIP differences separately. Recheck release status when refreshing references.

This is original implementation from public behavior and documented mathematics.
Authorized black-box observation may resolve ambiguity. Never read, disassemble or
copy proprietary implementation code. No paid host is required by native tools.

## Registers and gates

- `rhino8.json`: documented command headings and option-level native acceptance.
- `manual_inventory.json`: topic URLs, platform, review status and native service contracts.
  Its initial topic set is seeded from command indexes; full manual traversal is pending.
- `../components/`: Grasshopper/Kangaroo component inventory and known gaps.
- Git stores contracts, algorithms, tests, coverage and developer instructions.
  Google Drive stores original operator cheat sheets and workflow notes.

For each topic: review both platform references, record options and supported types,
define units/tolerance/degeneracy rules, identify shared kernel dependencies, implement
an original bounded service, expose CAD commands and component-ready contracts, test
known and hostile cases, inspect the UI, then update status. A numerical API does not
establish interactive prompt, selection or all-object-type parity. Unimplemented
options must be rejected or clearly unavailable. Failures preserve source and undo.

Order by dependency rather than alphabetical spelling: geometry types and transforms;
selection/snaps/CPlanes; curve creation/edit/evaluation; surface construction; trimming,
intersection and topology; solid operations; meshing/SubD; dimensions/annotation;
materials/display/rendering; file formats/automation and platform integration.
Feature coverage includes non-command UI/settings/workflows and manual subsections.
An index count is never a full-manual completion claim.

## First reviewed increment: exact 3D affine transforms

Native service: `buildercraft_kernel::transform_exact`, command `geometry3d.transform`.
Supports explicit world-space translation, rotation about origin/axis with numeric
angle, positive uniform scaling about origin, and reflection about an explicit plane.
Curves and control surfaces retain rational weights, knots and degrees. Batch edits
are atomic before document mutation, undoable, identity-preserving when edited, with
optional geometry copies using fresh IDs. Copies keep object name/layer/visibility;
organization membership, production assignments and history are not cloned.

Partial Rhino coverage only: meshes/Breps/SubD, gumball/drag/nudge, CPlane/normal-derived
movement, subcurve selection, last-axis memory, reference-point angle/scale inputs,
rigid scale, zero/negative scale factors, mirror CPlane/three-point/object inputs,
mirror axis presets and preview, remembered copy choices and associative
feature history are pending. Existing legacy 2D commands are not automatically 3D.

Bounds: 128 IDs, 100,000 aggregate controls, 32 MiB estimated output shape retention,
16 MiB per exact shape, finite coordinates within 1e12 native units, scale 1e-9 to 1e9.
Kernel cancellation is checked before allocation and per control. CAD synchronous
commands do not yet expose a cancellable background transform job. These bounds do
not account for total process RAM or all CAD undo retention.

Evidence: `crates/kernel/tests/transforms.rs`, `crates/engine/tests/transforms3d.rs`.
Source: official Move, Rotate3D, Scale and Mirror command pages under
https://docs.mcneel.com/rhino/8/help/en-us/commands/ . Behavior descriptions here are
original summaries, not copied help text.

## Second reviewed increment: Scale1D and Scale2D

Reviewed the combined Scale command topic on both Rhino 8 Windows and Mac help.
The per-command redirect pages do not carry the complete options. Original native
`geometry3d.transform` operations `scale1d` and `scale2d` take explicit origin,
axis (1D) or plane normal (2D), and numeric factor from 0 through 1e9. Directions
are normalized; zero/nonfinite directions and out-of-range factors are rejected.
1D scales the axial component and leaves perpendicular components unchanged;
2D uniformly scales the plane and leaves the normal component unchanged.
Zero allows flattening, which may create degenerate geometry; it is not evidence
of valid solid topology. Rational weights, knots and degrees are retained.

The existing bounded atomic batch, new-ID copy and undo service applies. API only:
active viewport CPlane, reference picking, default bounding-box center, Rigid,
SubCrv, history, remembered choices and native interactive aliases remain pending.
Negative factors are rejected. Existing UI uniform Scale is unchanged.

Evidence: rational evaluation at oblique axes, off-origin known-coordinate cases,
zero-factor flattening, bad factor/axis rejection, copy identity, native project save/reopen and undo/source
preservation in `crates/kernel/tests/transforms.rs` and
`crates/engine/tests/transforms3d.rs`. Next: ScaleNU and explicit frame contracts.
References: https://docs.mcneel.com/rhino/8/help/en-us/commands/scale.htm and
https://docs.mcneel.com/rhino/8mac/help/en-us/commands/scale.htm .

Numeric API examples (IDs must identify native exact geometry):

```json
{"command":"geometry3d.transform","params":{"ids":[42],"operation":{"kind":"scale1d","origin":[0,0,0],"axis":[1,0,0],"factor":2},"copy":false}}
```

```json
{"command":"geometry3d.transform","params":{"ids":[42],"operation":{"kind":"scale2d","origin":[0,0,0],"normal":[0,0,1],"factor":2},"copy":true}}
```

## Third reviewed increment: ScaleNU and ScaleByPlane

Original `geometry3d.transform` operations `scale_nu` and `scale_by_plane` extend
nonuniform scaling through the same exact geometry, atomic batch, copy and undo
services. ScaleNU accepts explicit origin and world XYZ `factors:[sx,sy,sz]`.
ScaleByPlane accepts origin, `x_axis`, `y_axis` and `factors:[sx,sy]`, scales each
plane direction independently and preserves displacement normal to the plane.
Factors are finite from 0 through 1e9; negative factors are rejected. Flattening
can create degenerate geometry and is not a solid-topology validity guarantee.

Plane directions are normalized and must be perpendicular within a normalized
dot tolerance of 1e-9. Accepted numerical drift is orthogonalized before scaling;
nonperpendicular, parallel and zero directions are rejected before mutation.
This explicit-frame service does not infer a CPlane or a plane from an object.

Both Windows/Mac combined Scale topics were reviewed. WorldCoordinates numerical
behavior is implemented for ScaleNU. ActiveCPlane/3Point/Object/FromView and named
plane presets, reference inputs, interactive aliases, Rigid, history, remembered
choices, SubCrv and non-exact representations remain pending. Preset planes can
be expressed by explicit axes; no native preset selector is claimed.

Evidence includes off-origin world XYZ coordinates, a known oblique-plane result,
rational curve/surface evaluation, invalid frame/factor/unsupported-field rejection,
atomic failure preservation, copy/undo and `.bcraft` save/reopen in the shared
transform tests. Next: ScalePositions (spacing without deforming each object).

## ScalePositions increment

`geometry3d.transform` now accepts `scale_positions` for native exact curves and
control surfaces. Each object is translated from its world-axis bounding-box
center; its control-point differences, knots, weights and degree are preserved.
Modes: `{"kind":"one_d","axis":[1,0,0]}`,
`{"kind":"two_d","normal":[0,0,1]}`, or `{"kind":"three_d"}`.
Supply `origin`, finite `factor` from 0 to 1e9, and absolute bounds `tolerance`
from 1e-9 to 1 model units. Existing copy/undo/batch semantics apply.

Bounds use original homogeneous Bezier span extraction and adaptive convex-hull
subdivision, including rational weights and multiple knot spans. Subdivision
stops at the requested bounds tolerance plus a scale-dependent f64 rounding
guard. Translation error can amplify with the scale factor. The batch shares
a 200,000-work limit and each patch has depth limit 48; exceeding either rejects
the operation rather than silently substituting a control-cage center. Kernel
cancellation is cooperative; the current desktop command remains synchronous.

The transform panel has Space 1D / 2D / 3D buttons, using existing origin,
axis/normal, factor and copy controls; UI bounds tolerance is 1e-6 model units.
Reference-point picking, active-CPlane inference, mesh/solid input, group-level
centers, remembered Rhino options and associative history remain pending.
Graph canvas integration is not implemented. Numerical API coverage is partial.

Evidence: `crates/kernel/tests/spacing.rs` and
`crates/engine/tests/transforms3d.rs`.

## Shear increment

`geometry3d.transform` accepts `shear` on native exact curves/control surfaces,
with explicit world-space `origin`, `direction`, `normal` and `angle_degrees`.
The fixed plane passes through origin with the given normal. A point moves by
`direction * tan(angle) * dot(point - origin, normal)` after normalization.
Directions must be perpendicular within 1e-9; accepted drift is orthogonalized.
Angles are strictly between -89 and 89 degrees, including zero and negative
angles. Invalid frames, nonfinite parameters and coordinate overflow reject
before document mutation. Existing copy, budgets, cancellation and undo apply.

The transform panel exposes a Shear button with independent direction and angle,
using the existing origin and axis/normal controls. Rational weights, knots,
degrees and parameterization remain intact. Source geometry stays authoritative.
Reference picking, active CPlane, Rigid, SubCrv, remembered choices, history,
meshes and solid representations remain pending. Numeric subset only.

Windows and Mac official Shear pages reviewed; their option lists differ.
Original numerical fixtures cover fixed-plane points, signed inverse, oblique
frames, rational evaluation, surface edits, hostile inputs, copies, undo/redo,
project round trips and actual headless button clicks.

## Orient3Pt increment

`geometry3d.transform` accepts `orient3pt` with three world-space `source` and
three `target` points plus optional `scale` (boolean, default false). The first
point is the origin, the first-to-second direction defines X, and the third
point determines the right-handed plane orientation. Without Scale, dimensions
are preserved. With Scale, uniform size follows the ratio of target/source
first-edge lengths; the third point never introduces nonuniform scaling or
shear. The full third source point need not land on the third target point.

Both point triples must have first/third edges at least 1e-9 model units and
angular sine at least 1e-9. All points must be finite within 1e12 units; scale
ratios must be 1e-9 through 1e9. Degenerate frames, unsupported fields and output
coordinate overflow reject before mutation. Existing batch, copy, cancellation,
retained-byte limits, undo/redo and persistence apply. Weights, degrees and knots
are preserved for exact rational curves and control surfaces.

Numeric UI includes all six reference points, a Scale checkbox and Orient3Pt
button. Viewport reference picking, remembered Copy and meshes/Breps/SubD remain
pending. Windows and Mac reference pages were reviewed. Evidence covers known
3D placements, handedness, inverse transforms, third-point scale independence,
rational evaluation, surface transforms, hostile frames, copy, undo, persistence
and actual headless button clicks.

### Viewport transform gizmo

Initial world-axis UI: move along projected axes, rotate in projected axis planes,
and uniformly scale about the selected control-hull center. Hidden/locked
selections are rejected. Preview transforms rendered samples only and never
modifies authoritative geometry. Release dispatches `geometry3d.transform`,
including its copy and undo semantics. Escape or a changed selection, document
identity/revision, camera or viewport cancels. Edge-on move handles are hidden;
edge-on rotation is unavailable. This is a limited native gadget, not complete
Rhino Gumball parity. Numerical and pointer-event tests cover all three modes,
preview preservation, single-release undo, and Escape/camera/revision cancellation.
Full workspace: 483 tests and all six `cargo xtask ci` gates passed on Rust 1.95.0.
Rendered handle/ring inspection completed. The inherited `plan/` and sibling
`craftrules` references and the documented `tools/verify-worldwright-kernel.sh`
wrapper are absent in this checkout; the checked-in suite rules and `cargo xtask ci`
were used directly.

### Viewport object selection

`geometry3d.pick` exposes toolkit-independent orthographic pixel/depth hit
queries, reusing the kernel preview-wire visitor used by rendering. Click and
Shift-click dispatch `geometry3d.select` for replace/toggle selection without
changing document geometry, revision or undo history. Hidden objects/layers and
locked layers are excluded. Gizmo handles consume clicks, and drag gestures do
not click-select. Selected exact curve/surface wires highlight orange.

Fixed preview sampling and wire-only selection are explicit limits: no exact
curve hit certification, surface-interior occlusion, subobjects, windows or
snapping. Object/evaluation budgets reject queries without a partial hit;
rendering signals an incomplete preview if its budget is reached. Tests live in
`geom/src/picking.rs`, `kernel/src/wireframe.rs`, `engine/src/cmd/picking3d.rs` and
`ui-egui/src/buildercraft.rs`. See the API and alpha dependency register.
