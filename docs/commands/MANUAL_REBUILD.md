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
