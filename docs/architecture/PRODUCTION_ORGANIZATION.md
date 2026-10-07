# Production organization and smart relationships

Native production metadata is a separate many-to-many relationship model, alongside layers/sublayers and assembly/component/body ownership. The first implementation persists validated records, bindings and links in `.bcraft` v1 (older files default to an empty model), supports undoable `production.set` and read-only `production.model`, and carries the model in visualization JSON and GLB extras. It is not a finished production-management UI or BIM translator.

Record kinds: show, act, scene, beat, effect, department, system, equipment, zone, package, cue, asset, script, storyboard, camera and ride_path. Each has a stable 128-bit string ID, optional parent, name and bounded attributes. Attributes can carry script source/revision/scene ID, asset source, external BIM ID, package/status/owner or shot reference. Bindings assign a CAD object to multiple records with explicit roles; links connect records using roles such as triggered_by, synchronized_with, projects_onto, illuminates, heard_in, rides_on and depicts. These relationships describe intent, not executable device logic.

Production record hierarchy does not force geometry to change layers or bodies. The same scenic piece can appear in multiple script scenes and effects and be related to lighting, projection, audio, automation and a vehicle. IDs survive rename/reorder. A binding to missing/nonexported geometry remains unresolved and gets an export diagnostic, preserving references rather than deleting intent. Smart connectors and report/query UI are subsequent work.

## Script and storyboard direction

Scripto is an optional discovery target: https://scripto.live/about . Available API permissions and exchange routes still need verification; no Scripto connector is implemented or required. Use a host-independent import boundary first, with external script IDs/revisions mapped to stable native scene/beat records. Script revisions must produce a reviewable reconciliation rather than renumbering every scene and breaking assignments. Cameras/shots/storyboards reference the same records and model IDs; asset changes should flag affected shots. Review FDX/Fountain and other format routes individually before adoption. Never automatically upload models or scripts to a cloud host.

## Ride path, envelope and sightline acceptance queue

Required native workflow: place a complete vehicle assembly along an arc-length-parameterized ride snake, with reproducible tangent/frame orientation, explicit banking, local assembly transforms and rider eye/camera points. Sweep conservative vehicle/rider envelopes and test clearance against selected objects, zones and structures; report limiting locations, assumptions and tolerances. Articulation and multiple-vehicle spacing need explicit models. Sightline tests must identify viewer pose, target/occluders, field of view and inclusion masks. Unreal displays poses, envelope results and sightline overlays; native services own analytical results. These geometry operations are planned, not implemented by the production records.

## Track operation acceptance queue

Required native `Track` operation: 2–5 rails on a ride path, with user-defined cross-section offsets/gauges, transported frames and controlled banking. Preserve specified pairwise rail distances in each track section. Avoid frame flips at vertical/tangent transitions. Smooth the path with explicit positional/tangent/curvature quality, then sweep rail profiles into closed smooth solids with end-cap and join policies. Check curvature limits, rail/self intersections, topology, gauge and fabrication tolerances. Optional supports/ties and output as separate rail bodies come later. Shares pipe/sweep primitives, ride-path frames and collision/envelope services. Native Brep sweep/solid prerequisites are still pending.

## BIM and cross-discipline connectors

Keep asset identity, type/instance distinction, coordinate/units, provenance and property namespaces explicit. IFC/BIM property mapping and animation/projection/audio/lighting/scenic/vehicle connectors require individually tested adapters and loss reports. A metadata relationship alone does not establish IFC compatibility, physical clearance certification or show-control execution.
