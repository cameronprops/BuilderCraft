# WorldBuilder + SolidCraft: one Worldwright CAD document

Decision: 2026-10-17. **WorldBuilder** is the Rhino-style freeform/direct-modeling perspective. **SolidCraft** is the Fusion-style mechanical/feature-history perspective. They are not separate geometry engines, duplicated documents, or third-party repositories named similarly.

## Implemented integration slice (pending native CI)

- The single Worldwright 3D desktop exposes `WorldBuilder` and `SolidCraft` mode commands (`ui.workspace.worldbuilder`, `ui.workspace.solidcraft`) beside the existing 3D/drafting selector.
- Both expose the same viewport, object IDs, named body/component organization, direct curve/surface/mesh controls, BRep solids and Boolean commands, selection, undo and `.dftba` storage. Switching perspectives does not copy or convert geometry and does not advance document revision.
- SolidCraft opens the **existing optional scoped feature-history inspector** by default. WorldBuilder leaves it collapsed by default but available, enabling mixed modeling without disabling histories.
- Workspace preference serialization retains the selected modeling mode; older profiles missing the new field default to WorldBuilder. The mode lives only in UI preferences, not authoritative CAD geometry or the native file.
- No new geometry engine, document format, permanent geometry cache or external host runtime is introduced. The existing exact OpenCascade/BRep worker remains the sole exact-solid authority, and the existing Rust shared-tool dispatcher remains the feature-history and OrbWeaver operation authority.

This slice does **not** yet implement Fusion-equivalent constrained sketch-to-extrude/revolve, parametric exact solid features, automatic direct-edit-to-feature conversion or topological naming.

## Representation and edit ownership

| Type | Authoritative representation | Allowed workflow |
|---|---|---|
| Rhino-style curve/surface | Native editable NURBS/control geometry | Direct editing in either modeling mode |
| Mesh/sculpt/scan | Native mesh or linked scan | Direct editing/repair without pretending it is a BRep |
| Exact BRep solid | OCCT binary shape in the document | Exact creation/Boolean/STEP with derived disposable shaded preview |
| Parametric feature recipe | Optional timeline scoped to document, body/model node or block | Revisioned edits, deterministic evaluation, suppression and rollback |
| OrbWeaver procedural definition | Reusable shared-tool graph | Evaluate via the existing shared dispatcher; explicit bake is future work |

Do not silently discard a SolidCraft recipe when editing its derived geometry. A direct edit to a derived parametric BRep needs an explicit **edit source / make direct / add feature** policy and topology provenance. Existing BRep commands currently create exact solids but are not history-evaluated features. An edit cannot be advertised as parametric until dependency graph, authoritative output handles, recompute and non-destructive failure handling all work.

## Efficiency and dependency rules

1. **One operation, several inputs:** WorldBuilder commands, SolidCraft features, OrbWeaver nodes and the headless API must call the same exact geometric primitive. Never translate one mode's display mesh into the other's authoritative CAD source.
2. **No wholesale Fusion/Rhino engine copying:** Those products supply workflow requirements, not proprietary code. Reuse legally compatible open-source kernels and modules where measured improvements justify added dependency cost.
3. **Per-body optional history:** Keep history local to the bodies that benefit from it. Freeform organic scenery and concept geometry need not carry a no-op feature tree.
4. **Selective recompute:** Invalidate only affected steps/objects and their derived previews, keyed by stable ID, source revision, units, tolerance and operation version. Use bounded caches and cancellation; do not eagerly rebuild untouched scenes.
5. **No mode data duplication:** A workspace switch must not regenerate a BRep, clone document data, change units, bump revision or consume a new undo record.
6. **Explicit conversions:** Mesh-to-BRep reconstruction and driven-solid-to-direct geometry require verifiable fidelity, provenance and opt-in. A rendered or triangulated preview is not a manufacturing solid.

## Next implementation gate

Implement a constrained sketch profile and a single history-driven exact `extrude` feature through the existing OCCT worker, with stable body ownership and published output IDs. Test parameter edits, rebuild/rollback/suppression, undo/redo, save/reopen and failed-Boolean preservation; expose the same shared operation in OrbWeaver and headless API. This depends on exact sketch/profile validation, stable topology references and worker IPC admission, not a second separate SolidCraft kernel.

Related: [feature history](FEATURE_HISTORY.md), [unified workspace](UNIFIED_WORKSPACE_EXPERIENCE.md), [world-building pipeline](WORLD_BUILDING_PIPELINE.md), [memory and job budgets](MEMORY_AND_JOBS.md).
