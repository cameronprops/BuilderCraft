# BuilderCraft: Holistic Spatial Assemblies

Status: planned architecture; this is a product design specification, not a completed implementation.

## Design goal
One real-world assembly, such as a themed rock wall, is one coordinated project object with nested disciplinary components, shared coordinates and revisions. The wall may contain scenic surfaces, framing, embedded luminaires, speakers, sensors, access doors, mounting brackets, cable paths and fabrication parts.

It must be possible to work by discipline or to see the entire assembly together. Workspaces are views and tool presets, never separate disconnected file systems.

## Core assembly hierarchy
Assembly: RW-017
- Scenic: mesh/NURBS rock surface, thickness, finish, seams and panel IDs.
- Structure: steel/wood framing, supports, attachment points, envelopes.
- Lighting: luminaires, beams, focus targets, aiming angles, fixture IDs.
- Audio: speakers, orientation, coverage previews, mounting and access.
- Controls: sensors, devices, network/power connections, field IDs.
- Motion: simulated mechanisms, positions, clearances and swept volumes.
- Fabrication: segmented pieces, hardware, mass estimates, lifting attributes.
- Documents: plans, sections, bills of materials, installation schedules, stickers, reports.

Every component has a persistent ID. Assembly membership and spatial relationships are independently queryable. Nested parts can be selected separately, but the parent assembly is the coordination unit.

## Linked views and workflows
- 3D scene: whole wall or isolated systems; transparent/cutaway views.
- CAD: exact models, mesh sculpting, snapping, dimensioning and constraints.
- Graph: procedural rock geometry and rule-based population of components.
- Data: rows for each device/part, typed fields, formulas and relationships.
- Timeline: simulated states such as aim, intensity and actuator position.
- Diagrams: freeform notes, org charts, signal paths and connection schematics.
- Paperwork: sections, construction sets, fixture schedules, patch sheets, part labels and QR stickers.
- Output: BIM or CAD exchange, simulation/previs geometry, manufacturable mesh, CNC drawings, printer-ready sheets.

Picking any nested object should reveal its related records, schematic symbols, reports and other linked assets. Picking a row or diagram symbol should select or highlight the corresponding 3D object.

## Spatial relationships
Model typed relationships rather than only layer names:
- mounted_on, embedded_in, illuminates, aimed_at, audible_in, senses
- connected_to, controlled_by, constrained_by, part_of, installed_at
- supported_by, fabricated_as, represented_by, documented_on

Each relationship has typed parameters, transforms, optional clearances, revisions and provenance. These relationships enable cross-discipline questions without requiring all objects to be converted to one geometry type.

## Unified data binding
Authoritative project fields describe the design. A workbook or schematic edits authorized fields via revisioned transactions. The viewport updates its preview; dragging an editable design target can update its corresponding values. Keep design values separate from simulated, observed and externally commanded device values. Spreadsheet edits must not actuate physical systems.

## Reports and output
One shared page/template/render engine serves architectural plotting, title blocks, labels, stickers and repeatable reports. A report may query the full assembly or filtered nested components.

Wall report examples:
- bounding dimensions, face area, volume if actually closed
- mass/weight: validated volume and material density, or clearly labeled estimate
- panel count, materials, fastening schedule and part IDs
- embedded equipment inventory, position and access requirements
- mounting/coordination checks and unresolved collision issues
- cable and network schedules; patch and focus paperwork
- fabrication drawings, assembly instructions and print labels

For layered or open rockwork, do not present inferred mass as a verified weight. Preserve assumptions, units and source revisions. Issued sheets are fixed snapshots while live sheets track changes.

## Flexible canvas
CAD can also be used as a blank technical notebook: diagrams, flowcharts, org charts, annotations, block diagrams and embedded live tables. Freeform marks need not be made into BIM objects. Users can promote a sketch into structured geometry or attach it to an assembly when useful.

## Incremental implementation
1. Stable assembly/component IDs and typed relationships in the shared kernel.
2. Basic record bindings between an assembly and its spreadsheet rows.
3. Query/filter selection synchronized between the data grid and viewport.
4. Reusable diagram/sheet primitives and titleblock field bindings.
5. Repeatable report rows, label templates and PDF/print output.
6. Geometry-derived quantities and mass estimation with provenance.
7. Fixture/speaker/sensor components with simulated spatial previews.
8. Cross-discipline coordination and interference/clearance reporting.
9. Vendor-specific paperwork, patch/cue export and previs interoperability.

Acceptance: edit fixture aim within an assembly, observe the simulated beam in 3D, see its focus record update, and generate a fixture label and wall-system schedule referring to the same stable ID, without triggering any real-world hardware action.
