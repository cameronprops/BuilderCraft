# BuilderCraft editing interface with Unreal visualization

Accepted direction: BuilderCraft owns the editing/control interface; Unreal is an optional visualization engine beneath it. Users must be able to edit, not only watch an exported movie. This interface is a product requirement, not a completed custom UI in the first feed adapter.

The existing native CAD UI remains authoritative for exact geometry. CAD/Graph/Show commands and identity/production records are shared with a dedicated previs workspace. Backend rendering and bridge transport must remain replaceable, so the free/open-source suite runs without Unreal. An optional Unreal-host UI may use original Slate/UMG/editor-extension code; it must route edits to native services rather than create a second authoritative CAD or show database.

## Editable workspaces

- Geometry, assemblies, placement, constraints and procedural parameters; layers, bodies and production assignments.
- Lighting fixtures, addresses/groups, virtual console/cues and looks; automation axes, limits, motion intent and timelines.
- Ride vehicle assemblies, paths/speed/banking, rider/camera points, swept envelopes, clearance and sightline overlays.
- Cameras, script scenes/beats, shots/storyboards, presentation modes and timeline scrubbing.
- Audio zones/listeners, projection surfaces/frusta/media, scenery/effect relationships and show-control signals.
- Audience movement/accessibility routes, weather/environment, animated props, particles, water/effects and immersive walkthroughs as backends become available.

Controls for virtual devices drive previs. Configured Show adapters map the same identities/signals to independently installed controller programs or hardware. Lighting appearance, sound propagation and engineering calculations need their own verified models; rendering alone is not measurement.

## Edit authority and feedback

Selection and gizmos return stable project/object IDs. Transform, parameter and cue edits are revision-checked commands with undo and source/procedural relationships. The native owner validates and publishes a new revision; an Unreal preview cannot silently replace exact NURBS/Brep with triangles. Native command errors restore the last valid model. Asset/material/display overrides and engineering edits have distinct ownership. Script reconciliation flags affected shots/effects instead of losing assignments.

Current direction is one-way visualization via complete snapshots. Bidirectional selection/transform proposals, edit acknowledgements/conflicts, undo synchronization, custom controls and asynchronous/coalesced jobs are next bridge/UI increments. The current Unreal component actor exposes local feed settings/material/collision options, not the full BuilderCraft editing interface.
