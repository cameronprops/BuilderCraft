# Worldwright: continuous world-building pipeline

Decision date: 2026-10-13. **Accepted product direction, not implemented functionality.**

Worldwright should let a designer move organically from blue-sky references and notes through spatial planning, massing, artistic and technical design, construction definition, and two coordinated outputs: an immersive rendered experience and production-ready information. The workflow is **nonlinear**: users can return to concept exploration after technical development, while already approved work remains protected.

**Product promise: one world, one evolving project, many valid ways to work and publish.**

This expands [the suite architecture](SUITE.md), [the unified-workspace UX](UNIFIED_WORKSPACE_EXPERIENCE.md), [the reference-library plan](../reference-library-roadmap.md), [feature history](FEATURE_HISTORY.md), [production organization](PRODUCTION_ORGANIZATION.md) and [the design environment](../unified-design-environment.md). It does not replace their shared-kernel, independent-module, versioned-bridge or CAD-first decisions.

## Experience, not a rigid wizard

| Mode of work | Designer actions | Persistent output |
|---|---|---|
| 1. Blue sky | Collect images, sketches, manuals, stories, mood boards, annotations and research; compare directions | Reference records, spatial notebook entries, design intent and provenance |
| 2. World planning | Define scenes, zones, narrative beats, guest paths, ride envelopes, sightlines, constraints and scale | Linked spatial layouts, paths, envelopes and relationships |
| 3. Massing | Sketch in 2D/3D, block architecture, sculpt terrain, place scenic proxies and establish compositions | Editable volumes, proxy assemblies and spatial relationships |
| 4. Design development | Mix direct CAD, NURBS/BRep, subdivision/mesh, scan, terrain, OrbWeaver and history-driven features | Authoritative editable geometry, recipes, materials and assemblies |
| 5. Construction design | Resolve structure, connections, tolerances, access, MEP/AV/show equipment, fabrication partitions and checks | Validated assemblies, quantities, attributes and technical views |
| 6A. Experience publication | Render/walk the scene with lighting, media, audio, motion, animation, timing and user navigation | Derived scene assets, engine manifest, reproducible walkthrough revision |
| 6B. Production publication | Generate plans, elevations, sections, details, schedules, shop packages, machine handoffs and BIM | Revisioned drawings, fabrication files, IFC/adapter deliverables and loss reports |

These rows are **activities, not mandatory gates or separate file formats**. A project may contain concept props, dimensionally developed architecture and reviewed construction assemblies at the same time. Workspaces are views into shared project data, and CAD, Scan, Graph/OrbWeaver and Show remain independently runnable and testable modules.

## WorldGraph: logical coordination layer

Use **WorldGraph** as the proposed name for the typed design-intent and relationship layer over the existing persistent project/scene services. This is **not a second scene graph, second geometry kernel or new monolithic application**. Reuse stable IDs, unit/frame contracts, typed commands, transactions, provenance, revision tracking and content-addressed resources already specified in the shared architecture.

Conceptual records:

- **World / scene / zone / experience:** hierarchy, bounds, coordinate frames, narrative intent, routes, sightlines and scene membership.
- **Object / assembly / instance:** stable project identity, owners, authored representation, revision, materials, classification and spatial transforms.
- **Reference / notebook entry:** image, PDF, sketch, research URL, measurement, note or annotation, linked to an object, zone, location or design question; retain source, rights, date and calibration.
- **Design intent / decision:** what an object must achieve visually, narratively, dimensionally or operationally, including assumptions, confidence, constraints and rationale.
- **Representation:** authoritative source and explicitly derived or cached variants, with origin revision, units, transform, fidelity, error bounds where available and supported editing operations.
- **Typed relationship:** e.g. references, occupies, depends on, constrains, assembled into, supports, clears, drives cue, documented on, derived from or published as.
- **Milestone / review / deliverable:** maturity, checks, approvals, branch/variant membership, publication manifest and exported artifact identity.

Persistent logical object IDs must survive a move from proxy to developed geometry and remain traceable through scene, documents and exports where the target format allows it. When topology or semantic identity changes too much to preserve safely, require an explicit mapping or replacement decision. No invented correspondence.

Graph mutations go through the existing transactional command engine. Dependencies invalidate only affected downstream derived data; jobs must be cancellable, bounded and revision-checked. Shared geometric algorithms are invoked by CAD, OrbWeaver and APIs, not reimplemented in WorldGraph. Simulation feedback requires explicit state/time evaluation rather than uncontrolled dependency cycles.

## Spatial notebook and blue-sky references

Make references first-class editable project resources. A user can put a photo on a board, pin a research note in 3D at a location, attach a source drawing to a facade, or associate a story beat with a route segment. Notes and source material survive as the associated object moves through massing and construction.

Required behaviors:
1. Drag and drop images, PDFs, sketches, links and technical specifications into collections and boards with searchable tags and source attribution.
2. Spatially anchor annotations to objects, local coordinates, scene zones or camera views, not merely transient screen pixels.
3. Distinguish **inspiration**, **assumption**, **measured reference**, **design decision** and **approved requirement** so a mood-board image never silently becomes a dimension.
4. Preserve a versioned trail from reference to sketch, modeled feature, fabrication decision and published deliverable.
5. Work offline by default; remote lookup, AI analysis and cloud publication are explicit opt-ins. Respect source rights and the project's confidentiality rules.

## One object, multiple valid representations

A scenic rock wall illustrates the model:
- conceptual sketch, reference photographs and story notes;
- artist-authored high-detail mesh or procedural surface;
- dimensioned CAD envelope and mounting interfaces;
- structural frame, panel seams, thickness and hardware;
- optimized LOD/collision/material assets for realtime preview;
- BIM classification, location and construction metadata;
- sheets, cut lists, print/CNC files and inspection reports.

These are **linked representations of one logical assembly**, not necessarily interchangeable geometry. The artistic mesh is not silently an engineered BRep; a closed mesh is not evidence of structural adequacy. Allow each representation to have an explicit authoring source and ownership; preserve original data when generating derived versions. If multiple authoritative discipline sources conflict, require a review and resolution rather than automatic last-writer-wins.

Representations can be independently regenerated, frozen, reviewed or intentionally superseded. Display and engine LOD meshes must not replace exact CAD geometry. Every conversion reports fidelity, unsupported fields, approximation and identity mapping.

## Design maturity and controlled change

Track readiness **per object/assembly/deliverable**, not only per whole project.

| Maturity | Meaning | What it does not imply |
|---|---|---|
| Concept | Artistic intent and references; approximate form acceptable | Reliable dimensions or quantities |
| Massing | Spatial scale, adjacency, paths and bounding volumes established | Solved construction |
| Design development | Editable geometry, intent and principal interfaces defined | Engineering sign-off |
| Construction candidate | Detailed parts, properties and declared validation checks exist | Automatically approved for fabrication |
| Validated / approved for a named use | Specified checks passed and required reviewers signed off against a revision | Universal safety certification or compliance |

Show status, responsible discipline, reviewer, evidence, revision and outstanding exceptions. Approval belongs to a **specific version and intended use**. A late creative change should show impacted dependent geometry, cue placements, sheets, quantities and exports, then regenerate or request reapproval selectively. A locked deliverable remains reproducible while a new creative branch is explored.

## Two synchronized publication tracks

### Immersive experience track

Publish a selected project revision to a navigable preview, initially through the existing GLB plus semantic manifest / Unreal bridge strategy. Include units, frames, object IDs, hierarchy, instances, visual materials, lights, cameras, collision, spawn point and publication hashes when supported. Extend toward scene composition, lighting looks, show timing, spatial audio, animation, rides and scenery motion as separate validated capabilities. A scene can be walked in massing before final assets exist.

MeliesFX / Show relationships supply cue intent, equipment and motion metadata without confusing simulation with actual controller operation. Render previews are interpretations, not guaranteed photometric or acoustic measurements. Realtime asset overrides need explicit source ownership and a deliberate round-trip policy; a one-way visualization feed is not bidirectional CAD editing.

### Production and BIM track

Generate linked plans, sections, elevations, dimensions, details, assemblies, bills of materials, fabrication/print/CNC handoffs, equipment and cable schedules, issue/revision packages and BIM-coordination records. Use the same object identities and approved revision as the immersive track.

Prefer open IFC exchange with explicit semantic classifications, units, mappings and loss diagnostics. Revit-family/native editability is adapter- and capability-specific; never advertise a generic mesh export as fully editable BIM. Flag missing material properties, tolerances, unresolved clashes, changed quantities and unapproved engineering assumptions. A construction-ready designation requires named tests and the appropriate human discipline reviews.

Both branches may be previewed continuously; **release** is a controlled revisioned action. A change in one branch should trigger impact analysis, not blind overwrite of the other.

## UI contract

Worldwright's shell should feel like a single design environment, with:
- reference board, spatial notebook, 2D sketch and 3D scene available without document conversion;
- fluid switching between blue-sky, planning, CAD, sculpt/scan, terrain, procedural graph, fabrication, show, rendering and documentation views;
- a consistent object browser, search/command line, selection grammar, inspector, undo, revision and approval indicators;
- side-by-side concept/reference/technical/experience views, optionally following the same selected world object;
- on-demand context tools and an optional guided GO sequence, never an obligatory linear wizard;
- timeline/diff and dependency-impact views that answer "why is this here?", "what derives from it?" and "what will this change break?".

Design mode must remain approachable without mandatory early BIM classification, engineering fields, or forced parametric histories. Advanced data and verification appear progressively as the work needs them.

## Implementation order and acceptance

This is a **dependency-aware roadmap decision**, not permission to interrupt current CAD alpha work.

1. **Contract first:** document stable cross-domain identity, reference/notebook records, typed relationship schema, revision/ownership, representation lineage, readiness states and conversion diagnostics. Extend existing scene/document services; avoid a duplicate graph engine.
2. **Minimum creative loop:** import/tag/pin a reference; create spatial note; author a massing proxy; link the note and proxy; save/reopen without losing IDs or provenance.
3. **Representation and dependency loop:** advance one object from proxy through editable geometry to a derived preview, with bounded invalidation, undo, deterministic caches and explicit conversion warnings.
4. **Dual-output vertical slice:** publish a massing walkthrough and one linked technical sheet/schedule from the same selected revision. Test units, IDs, camera position and revision mapping.
5. **Construction and BIM slice:** add a validated assembly, material/part quantities and IFC classification/export with explicit fidelity and semantic reports.
6. **Show and complex environments:** layer lighting, media, ride envelopes, cue intent, terrain, vegetation, rockwork and procedural generation over the same service contracts.

**Golden acceptance scenario:** A small themed scene starts with photos and notes, gains a guest path and building/rockwork massing, becomes a detailed facade plus supported assembly, then produces a navigable walkthrough and a drawing/BIM package. Change the facade location and prove that spatial notes remain associated, impacted outputs become stale, approved revisions remain immutable, selective regeneration works, exported units and IDs stay correct, and no unsupported BIM or structural claim is made.

Test save/reopen; undo/redo; editing in different workspaces; branching and review locks; external import/export; stale-update conflict rejection; cancellation; cross-platform resource budgets; and the difference between authoritative versus derived geometry.

**Current implementation statement:** The repository has a CAD-first alpha, shared services and partial visualization/production foundations. The full WorldGraph, creative notebook, maturity gates, production sheet pipeline, IFC roundtrip and complete realtime scene authoring described here are **not implemented by this documentation**. Do not mark them working without executable features and passing acceptance evidence. Keep existing BRep, selection, OrbWeaver and native CAD validation priorities ahead of broad UI work.
