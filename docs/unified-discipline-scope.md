# BuilderCraft: Unified Designer-Led Themed Entertainment Platform

Status: long-term product scope and architectural requirements. These are NOT
claims that all listed capabilities are already implemented.

## Mission

A single designer-first project environment spans CAD, geometry processing,
scanning, digital fabrication, scenic and environment design, ride vehicles,
show systems, previsualization, BIM coordination and production paperwork.
Each discipline contributes to one spatially coherent project, using
shared object identity, transforms, typed records, revision history, material
properties, assemblies, constraints, dependencies and publishing services.

The experience should support blue sky through detailed design and execution,
with a flexible scratchpad beside rigorous geometry, simulation and production
deliverables. No discipline needs to surrender its specialized methodology
to participate in the same project.

## Neuroinclusive workflow architecture

The entire platform is adjustable for different attention, sensory,
communication and task-sequencing preferences. A user may choose a minimal
canvas, guided step sequence, dense professional workspace or custom mix,
without fragmenting the shared project. Interruption recovery, pinned notes,
visible command state, configurable toolbars and predictable undo support
CAD, mesh repair, scan alignment, StructureGraph, show systems, data and
publishing alike. See
[Neuroinclusive and Customizable Workflows](neuroinclusive-customizable-workflows.md).

## Capability matrix

| Discipline / workflow | Native authoring and analysis goals | Shared project outputs |
| --- | --- | --- |
| CAD and surfacing | Curves, NURBS, solid BReps, feature/history modeling, loft/sweep/trim/Boolean, drafting, assemblies | Exact CAD, drawings, BIM reference, manufacturing |
| Mesh editing and repair | PolyWorks-like manual vertex/edge/face selection, triangle/quad creation, cleaning, welding, hole filling, remesh, thickness | Editable polygon topology, printable/exportable meshes |
| Scan and metrology | Point clouds, mesh and scan cleanup, best-fit and registration/alignment, target-based and datum alignment, measurement/deviation | Registered as-built references, reconstruction and QA |
| Synthesized / parametric design | Grasshopper-style visual graphs; integrate StructureGraph as a structural/procedural subsystem, with nodes, constraints, parameter lineage and graph-driven fabrication | Rebuildable scene objects, truss/framing and model assemblies |
| Scenic, rockwork and environments | Sculpting, procedural rock/wood/brick, texture, panelization, landforms, terrain/heightfield arrays, vegetation, GIS/site layout | Scenic assets, installation zones, reportable surface/material quantities |
| Ride vehicle design | Cabin/hull and seating layout, restraints/reference geometry, envelope and clearance modeling, vehicle assemblies, track/path curves and rail profiles, bank and kinematic sampling | CAD deliverables, clearance views, swept geometry, previs vehicles |
| Animatronics and creatures | Character/creature surface and internal mechanism design, joints, linkages, actuators, motion envelopes, skins and collision constraints | Editable creature assemblies, motion previews, fabrication components |
| Automation and motion | Paths, pivots, axes, constraints, limit data, equipment mounting, mechanical simulation and scripted cue intentions | Time-based animation and clearance analysis, engineering handoff |
| Lighting, AV and controls | Fixtures, beams/focus, speaker positioning, coverage, video/projectors, cables, embedded sensors, patch and device records | Show-system diagrams, patch schedules, previs, device paperwork |
| Previsualization | Coordinated scene/timeline, fixture and audio states, vehicle/animatronic simulation, materials, cameras, trigger/cue playback | Engine-ready assets, Unreal interoperability, evaluated show sequence |
| Spreadsheet/database | Native grids, formulas, typed records, height-map arrays, relation queries, value-bound CAD properties | Dynamic geometry, schedules, reports, cue data |
| Designer-led BIM | Semantic assemblies and design intent first; gradual classification and documentation; IFC/Revit-targeted exports when validated | Coordination with architecture/structure/MEP and construction |
| Model shop / fabrication | Segmentation, keyed joints, watertightness, print tolerances, nesting, CNC-ready sheets, shop work orders | 3MF/STL/mesh, CAM handoff, patterns, labels and fabrication drawings |
| Paperwork / publishing | Scratchpad/flowcharts, sheet layouts, title blocks, plotting, label/sticker batches, quantities and device/part reports | PDF/plotter pages, asset stickers, QR labels, schedules |

## Architecture invariants

1. **One identity, multiple representations.** Keep authoritative source
   geometry, linked derived forms and explicit accuracy/loss metadata.
   NURBS, meshes, scan points and kinematic states are different types;
   switching editing methods is not always a reversible format conversion.
2. **One shared spatial context.** Coordinated units, datum, tolerances,
   assembly transforms, instances and scene origin.
3. **One data context.** Scene objects, worksheet records, structure graphs,
   timelines, devices, labels and paperwork reference stable IDs.
4. **One dependency model.** Editing a scene object can invalidate a geometry
   derivative, computed spreadsheet field, assembly report, drawing, printed
   label or simulation. Only dependent artifacts recompute, with provenance.
5. **One transaction/history model.** Preview, undo/redo, cancellation,
   versioned changes, revision/merge and validation of assumptions.
6. **Multiple interchangeable UIs.** Direct modeling, sculpting, wizard,
   node graph, data table, timeline, diagram/scratchpad, drafting sheet and
   discipline-specific workspaces expose the same project, not siloed files.
7. **One exchange and publishing service.** Purpose-specific deliverables
   preserve as much geometry, semantics, transforms and identifiers as target
   formats support; every lossy conversion reports what changed.
8. **Safety separation.** Authored design, evaluated simulation, observed
   telemetry and commanded real-world equipment are distinct. BuilderCraft
   previews are not certified ride/automation control, and spreadsheet edits
   cannot implicitly actuate physical devices.

## StructureGraph integration

Treat StructureGraph as a first-class procedural/structural plugin with a
stable typed adapter into the scene graph and common graph runtime. Source
graphs remain editable and reproducible, generating assemblies, steel/framing,
connectors, zones, metadata, sheets and fabrication representations. Reusing
StructureGraph's existing structures requires an actual API/format audit;
do not claim upstream compatibility until integration tests pass.

Examples: Trylon/Perisphere historical reconstruction, scenic steel backing,
ride track supports, scaffold or built-up structure, model-shop segment
assembly with printable joints. Geometry needs explicit source graph version,
seed, parameters and material/tolerance settings.

## Example: holistic rockwork-and-ride scene

An experience scene contains a rock wall plus structure, sculpted cladding,
embedded fixtures/speakers/sensors, wiring, doors and show effects. Nearby
track and ride vehicle are coordinated to the same frame and boundaries.
Animatronic assemblies, control axes and cues produce deterministic previews.
The data workspace tracks position, patch, materials, mass estimates and
clearance limits. A report queries ALL related parts; a sheet set prints
assembly drawings, pick lists, stickers and title blocks. Revit receives
verified discipline-specific exchange geometry/semantics; Unreal receives
the evaluated viewport/previs representation. The original scene stays
editable even after generating derivative deliverables.

## Delivery strategy

Do not build isolated clones per domain. Prioritize enabling primitives:

1. Stable scene object/assembly ID system and typed relationships.
2. Editable polygon topology, scan alignment, exact CAD surface/solid operators.
3. Data engine, spreadsheet bindings and scalar/vector/time-series values.
4. Procedural graph + StructureGraph adapter over the same object IDs.
5. Versioned transforms/constraints, motion timeline and simulation state.
6. Native 2D vector layout, drawings, stickers and report templates.
7. Display/picking tools and customizable workspaces.
8. Validated import/export adapters for Revit/IFC, Unreal and show vendors.
9. Automated interdisciplinary consistency checks and large-scene scaling.

Every item needs an implemented-vs-planned status, reproducible tests and
round-trip/accuracy checks. Building a domain UI is not proof that underlying
geometry and data functions work.
