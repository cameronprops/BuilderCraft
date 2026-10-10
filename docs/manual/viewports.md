# Viewport navigation and camera views

**Status:** Existing foundation for single 3D orthographic viewer, camera orbit/pan/zoom and Top/Front/Right/Iso/Fit. **Alpha gate** for reliable full navigation, multiple synchronized views and optional perspective. **Beta** for camera walkthrough/ride-through.

## Current 3D development interaction

- Drag the blank viewport to orbit, Shift-drag to pan, scroll to zoom. The native UI has **Top, Front, Right, Isometric, Fit** controls.
- The viewer is currently **orthographic**, and current wire/face visibility is not a production depth-buffer representation.
- The 2D Drafting workspace has its own inherited canvas controls. Do not assume the current 2D input shortcuts match every new 3D tool.
- Camera state is UI state, not a geometry transaction. Changing a view must never create an undo entry or change curve/control-point data.

## Required alpha controls

| Camera/view | Required behavior |
| --- | --- |
| Single | Toggle Top, Bottom, Front, Back, Left, Right, Isometric and Perspective; resize without drifting focal point |
| Four-view | Four independently controlled camera orientations, one shared model and unified selection; expand/restore any viewport |
| Navigation | Orbit, pan, zoom-to-cursor, mouse wheel/pinch where supported, Fit All, Fit Selected, center target, camera reset |
| View state | Stable view name/ID, projection type, position/target, near/far policy, orientation, clipping, display mode and grid/CPlane |
| UI mapping | Documented input bindings, remapping and one alternate accessible preset, keyboard-only camera controls where practical |
| Workspaces | 2D drafting, 3D modeling and OrbWeaver switch without exporting or converting the scene |

**Implementation contract:** cameras and input events are per viewport, while selection, document geometry, layers, visibility, objects and command history are shared. Mouse gestures resolve through one input priority: modal tool > gizmo drag > selection gesture > camera gesture. Escape cancels current interaction and discards only transient preview state.

**Acceptance:** all viewports show the same object IDs and selection; drag on a gizmo cannot unexpectedly orbit; zoom does not blow up on very small/large model scales; hidden/locked objects are excluded from normal picking; display modes can differ between panes without mutating the document; layouts save/reopen as preferences.

**Beta/extended:** human eye-height walkthrough, fly mode, recorded navigation paths, ride-through camera along a spline with banking, later vehicle motion/envelope and optional Unreal visualization. A flythrough image is not proof of physically correct ride movement or clearance.
