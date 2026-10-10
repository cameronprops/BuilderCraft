# Selection and model transforms

**Status:** Existing foundation for NURBS preview-wire picking, polygon mesh-face picking, Shift-toggle, visibility/lock exclusions and world-axis gizmos; comprehensive object and subobject selection is an **alpha gate**.

## Current interaction

Click a visible supported exact curve/surface preview wire or native mesh face. Shift-click toggles. The 3D **Model Browser** can select objects and model nodes. The gizmo can preview a move, rotation or scale; the engine owns the undoable transaction. Current mesh face pick is revision-bound. Preview-wire picking is sampled, and polygon picking is not full NURBS/mesh GPU depth occlusion.

## Alpha selection contract

| Mode | Hit target | How selection works |
| --- | --- | --- |
| Object | Point, line, polyline, polycurve, curve/spline, mesh, NURBS surface, trimmed face, polysurface/BRep, group, block instance | Click replace; Shift-toggle; empty click clears unless modal operation owns pointer |
| Area | Rectangular window and crossing, with optional lasso later | Window = fully enclosed; crossing = touched; apply visibility/lock/type filters |
| Subobject | Edit point/control point, mesh vertex/edge/face, BRep vertex/edge/face | Explicit mode or modifier, shows owning path and highlighted subobject, returns revisioned element identity |
| Ambiguous | Overlapping candidates | Cycle a hit stack; present object names/layers and instance path; never choose randomly |
| Filter | Per object kind / layer / group / instance path | Find/select matching supported elements without changing underlying geometry |

A group can select as a unit; a modifier or explicit Edit Group isolates members. A block instance selects as one object; Edit Definition changes shared geometry, Edit Instance changes instance-specific transform and supported overrides. Objects on hidden, frozen or locked layers are not editable through regular picking.

## Transform controls

The manipulator uses Move, Rotate and Scale with world, local/object, construction plane and parent coordinate options. Provide a pivot/axis handle, typed distance/angle/scale, copy mode, orthogonal constraints, snapping, numeric entry, Cancel and single Undo/Redo transaction. Geometry remains exact when the canonical operation supports it. Selection itself is transient and must not pollute the geometry undo stack.

**Release test:** create point/line/polyline, rational spline, trimmed face and quad mesh fixture; select each in wireframe and shaded views; window select; Shift-toggle; locked exclusion; apply and cancel transform; undo/redo; save/reopen preserving geometry and refs. If a type does not yet have a working picker, the alpha feature matrix must mark it unsupported and block signoff.

See [native smoke tests](alpha-acceptance.md) and [geometry parity acceptance](../parity/rhino-object-and-brep-acceptance.json).
