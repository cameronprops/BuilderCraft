# BuilderCraft: Unified Design Environment Architecture

Status: product architecture decision and implementation roadmap, not a claim of existing functionality.

## Mission

One collaborative design environment for scenic, rockwork, AV, automation,
carpentry, model shop, architectural, industrial, and themed-entertainment work.
At its core: exact solid CAD, surface modeling, direct polygon modeling,
procedural environment generation, and manufacturable output.

A single cohesive project should serve design, digital fabrication,
construction coordination, BIM, previs, visualization, engineering, and
asset handoff, and project paperwork without forcing everyone into the same editing methodology.

## Fundamental principle: one project, multiple methodologies

Three distinct kinds of switches:

1. **Workflow**: change tools, picking style, automation, and UI without
   changing the authoritative geometry (manual vs guided vs procedural).
2. **Representation**: NURBS BRep, parametric feature history, subdiv,
   polygon mesh, point cloud, SDF/voxel, paths or semantic BIM objects.
   Conversions MAY be lossy, approximate, slow, or one-way. Keep authoritative
   source and explicit derived/cached variants rather than pretending these
   are reversible display toggles.
3. **Display**: shaded, wireframe, X-ray, exploded, analysis overlays,
   drawings, semantic BIM view, real-time renderer/previs, section/clipping.

An object has **one persistent identity** even when it has several linked
representations. A project can contain heterogeneous object types together.

## Document / scene graph

The common project file tracks:
- Persistent stable object IDs and revision IDs
- Hierarchical assemblies, layers, groups, blocks/instances, references,
  components and relationships
- Units, tolerances, local coordinate frames, global coordinate/georeferencing
- Authoritative source geometry and its editable parameters/history
- Derived geometry representations with source links and accuracy metadata
- Materials, texture coordinates, finishes, color, visibility and LODs
- User-defined typed properties, asset tags and BIM/IFC classifications
- Human-oriented names, asset IDs, schedules and fabrication attributes
- Dimensions, annotations, reference measurements, issue/approval status
- Cross-discipline relationships, constraints, connection points and ports
- Provenance, timestamps, file dependencies and non-destructive revisions

Avoid replacing the entire scene graph whenever a discipline changes.
Provenance and mapping must survive tessellation, retopology, export and
reimport when the target format permits.

## Core engines: shared services, not isolated apps

1. Exact CAD: curves, NURBS/BReps, surface joins, solid modeling, robust
   booleans, intersections, transforms and precision.
2. Polygon editor: native triangles, quads and mixed n-gons, half-edge
   connectivity, manual patching, interactive mesh repair, retopology.
3. Procedural graph: Grasshopper-like typed node system, lazy recomputation,
   preview, baking and parameter history; same scene object IDs.
4. Scan / metrology: point clouds, alignment, mesh reconstruction, deviation,
   registration, feature extraction and reference overlays.
5. Environment: terrains/heightfields, erosion, vegetation, landscapes,
   paths, hardscape, rockwork and generated scenery.
6. Visualization/previs: materials, lights, cameras, animation, simulation,
   tracked design and interchangeable real-time backends.
7. Fabrication: part splitting, wall thickness, watertightness, print
   orientation, tolerances, supports, nests, sheet goods, CAM/CNC handoff,
   machine-specific profiles and manufacturing annotations.
8. BIM/coordination: IFC-semantic export/import, Revit-compatible exchange
   where available, references, properties, clash review and schedules.
9. Systems: AV, lighting, automation, ride/show control geometry and metadata,
   routing/pathways, kinematic envelopes and interfaces. Simulation must
   be distinguished from certified real-world machine control.
10. Paperwork / Documentation: live drawing sheets, elevations, details,
    cut lists, schedules, bills of materials, equipment and cable schedules,
    labels, shop tickets, submittals, issue logs, revision packages, field
    reports, fabrication instructions, print sheets and PDFs.

The user may install/expose only tools relevant to a task, but all tools
operate on the same project and kernel services.

## Interface principles

- A neutral default 'Design' workspace rather than a wall of discipline tabs.
- Context-sensitive tools: selecting a curve suggests sweep, trim, extrude;
  selecting mesh boundaries suggests bridge/fill/repair; selecting solids
  suggests manufacturing/assembly; equipment objects expose routing metadata.
- Manual, guided wizard, scripted, and node-based ways to run the *same
  underlying command* when feasible. Commands expose typed input/output
  contracts, preview results, transactions, undo/redo, and predictable errors.
- Workspaces are optional, customizable presets: CAD/Surfacing, Sculpt/Scan,
  Rockwork/Scenic, Architecture/BIM, AV/Lighting, Automation/Kinematics,
  Carpentry/Fabrication, Model Shop/3D Printing, Environment, Previs,
  Paperwork/Documentation.
- Customizable shortcuts, toolbar sets, radial/context menus, saved presets,
  macros, palettes and nodes; UI layouts are user preferences, not data silos.
- Non-destructive default, and explicit confirmation for precision-losing
  operations such as tessellation, voxelization, remeshing, triangulation.
- Show source-vs-derived state clearly. Never silently replace exact geometry
  with approximation.
- Selection grammar consistent across CAD, scan, mesh, graphs and assemblies.
- Immediate preview, clear constraints and repair suggestions, units-aware
  settings, accessible discoverability, no modal dead ends.

## Discipline workflows, same project

- **Rockwork:** modeled/scanned geology, zones, panel seams, steel and
  framing interfaces, FRP/chip thickness, elevations, installation groups.
- **AV:** equipment proxies, mounts, speaker sightlines, projection cones,
  cable paths, power/heat/network allocations, coordination locations.
- **Automation:** tracks, ride vehicles, motion envelopes, constraints,
  collision/swept volumes, sensors and subsystem interfaces.
- **Carpentry:** 3D solids, cut lists, material grain, joints, sheet layouts,
  tolerances, assembly drawings, CNC output.
- **Model shop:** scale representations, slicing, print segmentation,
  print materials, connectors, watertight mesh, resin/FDM/SLS profiles.
- **Architecture:** exact CAD, documentation, semantic building elements,
  coordination and IFC/BIM delivery.
- **Previs:** linked cameras/materials/rigs/timelines, geometry LODs,
  Unreal and other engine exports, upstream source IDs.
- **Documentation:** model-linked plans/sections/details, quantity takeoffs,
  rockwork panel schedules, cable/equipment inventories, carpentry cut lists,
  drawing sets, build tickets, change logs and review/submittal packages.

## Paperwork is a first-class workspace

Documentation is part of the project, not a downstream disconnected export.
A rockwork grouping, rail, lighting fixture, shop assembly, or model-shop
segment should be queryable in the drawing, in a schedule, and in a build
ticket using the SAME persistent object/assembly identifier.

Planned workflows:
- **2D drafting:** model-derived plans, elevations, sections, details, notes,
  dimensions, symbols, tables, title blocks and revision clouds; allow manual
  drafting overlays that do not silently alter 3D geometry.
- **Live schedules:** BOMs, material and cut lists, fabrication quantities,
  weight/dimensions, group and zone reports, equipment/cable schedules, costs
  and purchasing fields with provenance and unit handling.
- **Operational paperwork:** job travelers, shop tickets, RFI and submittal
  registers, punch lists, QA/inspection records, install packages, redlines
  and approval workflows.
- **Templates:** customizable company and discipline-specific sheet sizes,
  title blocks, legends, fields, numbering systems, signoffs and revision
  conventions. Maintain a usable neutral default.
- **Linked edits:** updates to model geometry/metadata mark affected documents
  dirty, offer controlled regeneration/diff and preserve historical issued
  versions. Document-only annotations remain stable when regenerating.
- **Output:** PDF drawing packages, print-ready sheets, CSV/XLSX schedules,
  DXF/DWG 2D exchange and other supported document exports. Separate PDF
  vector fidelity from raster 3D previews and report unsupported mappings.

Never make paperwork depend on a separate discipline project or a one-way
export. Manual override fields require ownership/provenance and should not
be overwritten silently by regeneration. Issued revisions are immutable
snapshots with references back to source model revisions.

## Interoperability contracts

- Native project format is a versioned archive with a structured manifest,
  per-object payloads, source/derived relationships and checksums.
- Define explicit import/export adapters for appropriate combinations of
  3DM, STEP, IGES, OBJ, STL, 3MF, glTF, USD, FBX, DXF/DWG and IFC.
  Support depends on licensing, available libraries, and proven round trips.
- STL/OBJ/glTF and many renderer exports do not preserve CAD history or
  semantic BIM properties. State exactly what is retained or lost.
- Native editing format MUST NOT be constrained by the lowest-fidelity export.
- Coordinate systems, units, object origin, material semantics, names,
  instancing and winding conventions are mapping requirements.
- Provide per-export purpose presets: BIM, previs, construction documents,
  milling/CNC, 3D print, game engine, render, metrology.
- Export validation should report closed/open mesh, face count, units,
  nonmanifold geometry, accuracy from exact source, and lost attributes.

## API / architecture

UI -> Command Registry -> Typed Kernel Transactions -> Scene/Geometry
     -> Representation Service -> Export/Analysis Adapters

Each operation advertises:
- Stable ID, input/output schemas, units, domain and supported geometry
- Read-only/mutating flag; selection contract and revision preconditions
- Preview/dry-run, undo/redo support, cancellation and memory budgets
- Resulting object IDs, mapping/provenance, warnings and accuracy information
- Multi-user conflict handling where collaboration is implemented

The registry currently holds kernel operation metadata, NOT this full schema.
Extend in stages, with backward-compatible contract changes.

## Staged delivery

0. Protect existing headless kernel and its cargo CI.
1. Editable topology (tri/quad/mixed), stable selections, topology transactions.
2. UI picking and preview wired to kernel commands; object selection grammar.
3. Parametric and direct edit operations over shared object IDs.
4. Source/derived geometry links, explicit conversion policies and versioning.
5. Discipline-specific workspaces as *presets* over the same commands.
6. Export adapters, round-trip checks and unit/coordinate guarantees.
7. Fabrication, BIM, previs, paperwork and systems integration using shared
   object attributes; introduce model-linked sheets and schedules early.
8. Collaborative references, permissioning, approval/review and large-scene
   performance.

## Acceptance tests

- One scenic rockwork object can be scanned, modeled, patched, assigned
  framing/interfaces, classified for construction, exported as a printable
  mesh, and rendered for previs without losing original source geometry.
- An AV mount and carpentry support refer to the same assembly coordinates.
- A track/ride vehicle design, its motion envelope and exported Unreal
  representation share IDs and an auditable source revision.
- Every conversion reports accuracy, expected data loss and unit mapping.
- An export never silently rewrites native editable geometry.
- A team member with a different workspace sees the same project state.
- Changing a referenced rockwork panel updates its linked quantity schedule
  and flags affected drawing sheets for regeneration without rewriting an
  already-issued construction package.
- Large projects remain selectable and editable with bounded operations.

A user should think about the design rather than the application's modes.
