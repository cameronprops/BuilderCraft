# Rhino command coverage

`rhino8.json` is the baseline inventory of documented Rhino 8 Windows/Mac command headings. It records names and official links only, not copied help descriptions. It is not a claim of installed plug-in, hidden, undocumented or all future-version coverage. Later compare with Rhino's runtime `CommandList` and explicitly enumerate optional plug-ins and platform differences.

Status: `working`, `partial`, `unvalidated`, `not_implemented`, or `not_applicable`. Working/partial entries require acceptance evidence. A lowercase match to an inherited command ID is only an unvalidated candidate; it does not establish Rhino option/geometry parity. Own BuilderCraft commands such as `nurbs.curve3d` remain real alpha features while their Rhino equivalence is pending.

Every entry has platforms, references, owner, candidate mapping, options, evidence and notes. Review command options individually; geometry correctness, units/tolerances, undo, source preservation and prompt/API behavior count separately. UI/host-only commands need a deliberate equivalent/applicability decision. Mark external host behavior separately from native implementation.

Refresh from official downloads without copying their text into the repository:

```sh
python3 tools/build_rhino_inventory.py \
  --index windows=/path/to/windows-command-list.html \
  --index mac=/path/to/mac-command-list.html \
  --candidate-root . \
  --output docs/commands/rhino8.json
```

The refresh retains reviewed status/options/evidence and records commands removed from the reference for review. Do not regenerate into a different output path when updating reviewed coverage.

## Priority acceptance definitions

| Command | First native acceptance scope | Dependencies |
|---|---|---|
| Project | Project selected curves/points along an explicit direction onto surface/Brep targets; report multiple/no hits and tolerance policy; preserve source on failure | Surface intersection, trimming/topology awareness, document transaction |
| FlowAlongSrf | Map selected geometry between source/target surface parameterizations with controlled domains, orientation, seams and supported rigid/non-rigid behavior | Surface evaluation, transforms/deformation, representation-specific validation |
| Flow | Deform along source/target curves with deterministic frames and parameterization policy | Rail/frame service, curve evaluation, transform/deformation |
| Loft / Sweep1 / Sweep2 | Explicit supported degree, continuity, frame and profile behavior with native exact outputs | Curve/surface construction and topology |
| OffsetSrf / Shell | Geometry-aware offset/thickness, collision/self-intersection diagnostics and validated output policy | Robust topology, intersections and offset kernel |

These acceptance definitions are BuilderCraft implementation plans. They are not a copied complete specification of Rhino behavior; option-by-option reference review is still required.

## Manual-driven native rebuild

See [MANUAL_REBUILD.md](MANUAL_REBUILD.md) for the stable-version baseline, contracts, implementation sequence and acceptance gates. `manual_inventory.json` registers 2,357 public topic URLs found in the Windows/Mac command indexes. It is a starting register, not a completed traversal or review of the entire manual.

Move, Rotate3D, Scale and Mirror now have partial native acceptance evidence through `geometry3d.transform`. Exact rational curves and control surfaces support world-space numeric transformations, bounded atomic batches, copies and undo. Solids, meshes, interactive reference picking, local-frame gizmos, history and additional Rhino options remain pending.

Scale1D and Scale2D have partial native numeric API coverage through the same
service. Arbitrary explicit directions/planes and zero-factor flattening are
implemented; interactive CPlane/reference input and other options remain pending.

## Dependency-first CAD/Graph pairing

All Rhino command, Grasshopper, Kangaroo and Rhino manual reference entries
have provisional dependency-group coverage in
[`docs/dependencies/reference-index.json`](../dependencies/reference-index.json).
The group DAG and shared CAD/OrbWeaver operation pair plan are in
[`docs/dependencies/tool-groups.json`](../dependencies/tool-groups.json).
These are **not** per-command reviewed dependencies and do not change
Rhino command parity statuses. New paired primitives must share one kernel
algorithm, typed ports, modifier semantics, and independent CAD/Graph
acceptance cases before a reference is marked implemented.

