# BuilderCraft Data Workspace: Spreadsheet + Database + Live Scene Bindings

Status: approved design direction; adapter interoperability and real-time
system integrations are planned, NOT implemented or certified.

## Mission

A single native **Data** component spans:
1. familiar spreadsheets (Excel / Google Sheets / Apple Numbers interaction);
2. typed relational records (inventory/database/schedules à la Lightwright);
3. semantic real-time bindings to scene objects, simulations, timelines, and
   approved external show-control systems.

It must share the BuilderCraft kernel's scene identity, units, transactions,
dependency graph, revision system, permissions, metadata and resource budgets.
It must run without a cloud account or internet connection.

An embedded LibreOffice/OpenOffice-compatible spreadsheet engine is an
OPTIONAL implementation/evaluation route. Do not bind the kernel to a desktop
office process or make the editor rely on a proprietary closed file format.
LibreOffice integration must be checked for licensing and deployability.
Engine choice should be backed by prototype benchmarks, compatibility tests,
and maintainable APIs. UI and data contracts remain independent.

## One data model, multiple views

- A typed record with a persistent ID can render as a table row, spreadsheet
  range, asset inspector, CAD property, BIM quantity, paperwork cell or cue row.
- The workbook is a **view/edit surface**; the authoritative typed model is
  not flattened into displayed strings or Excel-style cell addresses.
- Each field has a stable field key, type, units, value, source, permissions,
  validation rules, revision and update origin.
- Supported field types should include text, number, boolean, date/time,
  quantity (value+unit), Vec3, rotation/pose, color, enum, path, reference to
  scene object, reference to asset/fixture/cue, lists and expressions.
- Relationships refer to persistent IDs (e.g. fixture_104), not row number.
  Sorting, filtering or reordering sheets never breaks object references.
- Views: spreadsheets, relational tables, inspector panels, forms, chart
  dashboards, linked 3D selections, timeline/cue views, paperwork schedules.
- Shared filters/selection: select fixtures in a table -> highlight 3D objects;
  select geometry -> focus the corresponding rows and related records.

## Spreadsheet experience

- Multi-sheet workbooks; A1 references, named ranges, formulas, sort/filter,
  freeze panes, validation, formatting, grouping, find/replace, fill, copy/paste.
- Quantity- and unit-aware formulas; geometric functions (volume, bounding
  box, surface area, distance, position, orientation, deformation, clearance).
- Charts and pivots/dynamic aggregations over typed tables.
- Offline import/export: CSV/TSV first; XLSX/ODS where tested; formulas,
  number formats, charts, unsupported workbook features flagged on import.
- Excel/Sheets/Numbers *style* interoperability, not a promise of exact parity.
- Formula engine is sandboxed, deterministic by default, cached by dependency
  graph, and bounded by compute/memory budgets. External requests and scripts
  require explicit user permissions.
- Circular dependencies have explicit diagnostics, not silent evaluation.
- Optional controlled iterative calculation for supported use cases.
- Preserve spreadsheet display formatting separately from true numeric units.

## Database experience

- Built-in embedded transactional store (SQLite is a candidate, not selected
  until dependency and deployment review); normalized tables, secondary indexes,
  typed columns, record linking, query/aggregation, change history.
- Versioned schemas, migrations, import mapping, conflict resolution and
  single-user-first offline operation with later collaboration options.
- Sample tables: Assets, Assemblies, MeshGroups, RockworkPanels, Fixtures,
  Universes, AudioDevices, Cables, NetworkPorts, AutomationAxes, Vehicles,
  ScenicZones, Cues, CueTargets, Materials, Shops, WorkOrders, Deliverables.
- Make source-derived dimensions/volumes read-only computed fields unless a
  user intentionally edits the source geometry or creates an override.
- Asset identity is canonical across spreadsheets, CAD, BIM, documentation.

## Live bindings: bidirectional design data

Example fixture:
- Asset: `LX-104` references one scene-object ID
- Fields: world_pose, pan_deg, tilt_deg, beam_angle_deg, intensity_pct,
  fixture_type, fixture_mode, universe, address, circuit, focus_target_id
- Scene constraint computes pan/tilt from a target or sets target from angles.
- Changing a workbook field creates a validated *design edit transaction*.
- Dragging the focus target in the viewport updates the same typed fields.
- The dependency graph updates visualized fixture beam and affected
  schedules/paperwork with origin tags to avoid feedback loops.

Example motion axis:
- Axis record: travel limits, velocity/acceleration caps, pose constraints,
  rest position, simulation position, hardware binding identity.
- A timeline samples the simulation axis state into the 3D scene.
- Scene manipulation updates authored choreography/timeline only when the
  proper edit mode is active. Observed live telemetry is never implicitly
  written back as commanded motion.

Separate:
1. **Design state** (authored values/revisions),
2. **Simulation state** (evaluated at timestamp/frame),
3. **Observed state** (telemetry/actual device state),
4. **Commanded state** (authorized actions sent to external hardware).

Never create accidental bidirectional loops. Change events include:
object ID, field key, value, unit, time domain, source, revision and authority.
Batch updates transactionally; combine rapid events into frame snapshots.
Scheduling/timebases account for samples, frames, seconds, timecode and
wall-clock, with explicit offset/frame rate and dropped-update behavior.

## Timeline and show cues

- One typed internal cue graph and time domain, independent of vendor files.
- Tracks: 3D transforms, fixture attributes, audio sends/events, media
  playback intentions, AV routes, automation position/kinematic previews,
  marker/events, triggers, look presets, sequence timing and fade curves.
- Cue records link to stable target IDs and record snapshots or time-varying
  parameters. Use deterministic evaluation for repeatable previs.
- Cue and patch editors can be grids, timelines, node graphs or inspectors.
- Support grouping, override priority, look sharing, alternate sequences,
  cue comparison and structured conflict reporting.
- Adapter compatibility is per product, version, mode and licensed interface.
  Do NOT claim a universal interchange format covers all show-control systems.

## Integration adapters: concrete target matrix

| Target | Planned functions | Verify before claiming support |
| --- | --- | --- |
| Lightwright | Fixture database, patch/position/attributes, unit numbers, focus paperwork, bidirectional exchange when possible | Available export/import files and vendor-specific field mappings |
| Q-SYS | AV asset schedule, device/control states, cue/scene snapshot mappings, optional emulator/previs, selected control integrations | QRC JSON-RPC API and permissions, version-specific availability |
| QLab | Cue lists, targets, timing and trigger reference mapping, optional read/edit/monitor integration | QLab 5 OSC and scripting interfaces, workspace authorization |
| ETC Eos | Fixture patch, channel/type, focus and cue/value export where supported | Exact target version's supported patch/show export/import APIs and formats |
| Hog | Fixtures, patch, pallettes/groups, cue/list workflows where permitted | Console family/version and documented show/CSV import/export capabilities |
| grandMA3 | Fixtures, groups, patch and sequences where supported | Official grandMA3 export/import/API format and version compatibility |
| Previs / Unreal | Cue sampling into camera, light/audio/video/automation visualization | Versioned runtime transport and coordinate/timebase conversion |

Do not conflate grandMA2 and grandMA3, or Hog family formats.
For each adapter maintain a versioned mapping with sample real vendor exports,
roundtrip fixtures, warnings for unsupported columns, and a loss report.
A fixture channel patch is not a complete show file or executable cue stack.

## Safety and live systems boundary

- Simulation/editor is not a certified ride/automation control system.
- Design sheets NEVER directly issue real motion, laser, rigging, pyrotechnic,
  safety interlock, or dangerous device commands. Do not infer permission from
  possession of a project file.
- Exported desired settings/patches require explicit review and validation
  before deployment. Vendor targets enforce their own checks.
- Live connections require a separate opt-in, authenticated adapter,
  target allowlist, operator role, state/health checks, and explicit arming.
- Separate readonly monitoring, show-data editing, rehearsal control and
  live actuation capabilities. Live actuation requires purpose-built safety
  review and external certified control interfaces.
- Provide disconnection handling, audit trails and source/target state
  reconciliation. No hidden automatic device writeback from cell updates.
- Device state and show cues can always be modeled and simulated offline.

## Public kernel-facing abstractions (design contracts)

`DataRecordId`, `FieldId`, `SceneObjectRef`, `DataValue`,
`DataUnit`, `Expression`, `Dependency`, `Transaction`,
`DataBinding`, `EventStream`, `Timeline`, `Cue`, `PatchMap`.

The existing simple `OperationDescriptor` catalog is not yet an executable
command router. Extend it with schemas/typed output and version preconditions
without breaking existing IDs. Keep display components separate from the
authoritative data engine.

Internal data reference example (conceptual):
`record("LX-104").field("pan_deg")` -> `scene(id).fixture.pan`
`scene(id).bounding_box.height` -> `table("Panels").row(id).height`

Transaction policy: typed validation, field-level authority,
source+revision provenance, cycle detection, unit conversion, preview/dry
run, undo/redo, redraw invalidation and batch event emission.

## Delivery sequence

1. Headless typed data/record schema and unit-aware bindings, with tests.
2. Offline table store + CSV import/export; model object ID lookup.
3. Spreadsheet grid viewer/editor with query/filter/sort and selection sync.
4. Formula and dependency engine + scene geometry accessors.
5. Editable constraints/bidirectional bindings with origin+revision checks.
6. Cue/timeline schema and deterministic offline simulation data sampling.
7. Excel/ODS compatibility, workbook styles and pivot-like views.
8. Lightwright-style fixtures/patch, audio equipment, cable schedules.
9. Vendor adapter exchange; QLab/Q-SYS optional read APIs after verification.
10. Controlled operator integration, versioned exports and round-trip tests.

## Required test scenes

- Fixture in workbook, table, CAD viewport and cue view shares one object ID.
- Changing focus aim updates preview direction and a linked report; a viewport
  adjustment edits the correct table row without update recursion.
- Sorting grid does not change fixture identity or patch references.
- Automation timeline moves a simulated vehicle while actual telemetry is
  displayed independently and cannot accidentally overwrite the cues.
- External QLab/Q-SYS snapshots can be imported/mapped without authorizing
  device actuation.
- GrandMA3 / Eos / Hog patch export detects unavailable or unsupported modes.
- XLSX/ODS/CSV crosscheck preserves numeric values, units and formulas within
  explicitly documented compatibility limits.
- Offline operation works when no vendor devices or SaaS accounts are present.

## References to validate during implementation

QLab OSC documentation: https://qlab.app/docs/v5/networking/using-osc/
QLab OSC dictionary: https://reference.qlab.app/docs/v5/scripting/osc-dictionary-v5/
Q-SYS QRC documentation:
https://help.qsys.com/content/external_control_apis/qrc/QRC_Commands.htm

Other vendor exchange format details must be rechecked from original vendor
documentation and representative test files before coding any adapter.
