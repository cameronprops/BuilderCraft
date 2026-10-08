# BuilderCraft Reference Library and Materials Browser

Status: feature request and design direction. Detailed design deferred.

## Purpose

A native, searchable reference library within the same unified design
environment. References must be usable for discovery, visual comparison,
dimension checking, material selection, modeling, fabrication, previs and
production paperwork. It is a shared resource, not another isolated app.

## Reference types

- Photographs, scans, drawings, sketches, annotated screenshots, image planes
- PDFs, manuals, manufacturer cutsheets, codes, specifications, articles
- CAD objects, 3D scans/point clouds, sample meshes and digital maquettes
- Material/finish samples, textures, normal/displacement/height maps
- Fixture/speaker/sensor/automation equipment specifications and profiles
- Historical and architectural references, GIS/topographic datasets
- Show and fabrication examples, report templates, symbols and title blocks
- Links to approved web references and internally shared team resources

## User experience

- Drag a reference onto a 2D scratchpad, pin it to a viewport/image plane,
  place it in a 3D reference assembly or attach it to an object/scene zone.
- Search by name, typed metadata, tags, dimensions, discipline, manufacturer,
  license, source, project and revision. Optional OCR/semantic indexing may
  be explored later, with user control and privacy safeguards.
- Collections and mood boards for art direction, environments and materials,
  plus technical libraries of standards and equipment.
- Save comparison views, annotations, scale calibrations, alignment datum
  information, citations and derived source geometry.
- Context-sensitive suggestion/opening of references while editing a wall,
  fixture, ride vehicle, scan alignment or drawing.
- Personal, project and team libraries with clear sharing/permissions and
  portable offline caches. External resources never required for core editing.
- Adjustable focus mode, metadata density and keyboard-friendly navigation
  consistent with neuroinclusive workspace requirements.

## Data and provenance

Each reference has a stable ID, source URI or embedded blob pointer, checksum,
ownership/license, source and attribution, version/date, coordinate/scale
calibration, units, tags and any scene/record relationships.
Derived geometry remains linked to its reference source and transform.
Do not copy proprietary assets into distributable libraries without rights.

References do not silently become authoritative design data; a user can
explicitly trace, calibrate, adopt or model from them. Offline snapshots
record what was actually viewed at a given project revision.

## Architecture links

Scene graph <-> asset and reference IDs <-> typed data store
           <-> image plane / annotations / reference overlay
           <-> materials and equipment catalog
           <-> report/labels and procurement metadata.

Keep large binary assets external or content-addressed rather than embedding
them into every Git revision; repository code stores schemas, docs and
portable sample fixtures. Reference collections support manifest-based
relinking when projects move between computers.

## Initial delivery stages

1. Reference record schema, IDs, source and licensing metadata.
2. File/link import, local thumbnails, collection tags, simple search.
3. Link-to-scene, reference planes and 2D scratchpad attachments.
4. Scale calibration, comparison views and scan alignment overlays.
5. Material/equipment libraries shared with project spreadsheet and reports.
6. Team sharing, versioned references and user-controlled smart indexing.

Prioritize interoperability and simplicity. Elaborate on workflow, UI and
external integrations in a later design session.
