# First shared kernel increment

Build the complete workspace with Rust 1.95 or newer, as required by the existing egui 0.36 dependencies.

`buildercraft-kernel` is a headless Rust crate beneath document/UI services. It reuses the current rational geometry evaluator. Run `cargo run -p buildercraft-kernel --example scene` for a small scene transaction/snapshot/manifest demonstration.

## Implemented contracts

- Nonzero 128-bit identities serialize as hexadecimal strings, avoiding JSON number precision loss. Callers supply and persist project IDs.
- Immutable exact curves/surfaces, triangle meshes and point clouds; sample/index/finite-value validation; shared leases with concurrent retained-byte admission and release after the last owner drops.
- Scene objects with layers and ID-based parent links, bounded hierarchy validation, revision conflicts, atomic command batches, cooperative cancellation and shared snapshots.
- Point conversion among mm/metres/inches/feet and three declared axis frames. Mesh winding conversion and full scene exchange are future work; exact affine edits are described below.
- Existing CAD exact shapes share buffers across undo snapshots; a control edit copies only the affected exact shape. Existing `.bcraft` v1 shape encoding is retained.

## CAD API

Call `kernel.manifest` with `{"project_id":"00000000000000000000000000000001"}`. Optional `geometry_budget_bytes` defaults to 64 MiB and is capped at 1 GiB. The command returns protocol version, revision, frame, organization and 3D geometry metadata. It does not mutate the drawing.

Legacy CAD handles map to kernel IDs as handle + 1; this is project-scoped. The caller must retain the supplied project identity. This projection covers organization and exact 3D geometry, not inherited 2D drafting, materials or show equipment. Unsupported drawing units return an error. It is not a live bridge or a geometry payload exporter.

## Resource limits and next work

Retained geometry accounting conservatively includes vector capacity. Cloned leases count once; independently retained wrappers may count shared exact buffers twice. Validation/admission happens after input buffers exist, so decoding and temporary allocation need separate upstream budgets. Metadata, process RSS, CAD undo, worker queues, caches and GPU buffers are not covered by this geometry budget. Rust ownership and these contract tests do not establish whole-suite memory performance.

Next vertical workflow: controlled tessellation plus scene export and engine import, then the shared typed graph evaluator and embedded CAD component workspace with native Kangaroo-style goals. Separate apps and proprietary adapters remain optional consumers of the kernel.

## Bounded preview tessellation

`tessellate` samples exact curves into `GeometryData::Polyline` and untrimmed control surfaces into indexed triangle meshes. The authoritative exact shape is never replaced. Segment and sample counts, a per-job output-buffer capacity limit and a conservative complexity score bound admission before sampling. Output buffers use fallible reservation; retained resources use the existing shared budget. Cancellation is checked between samples/faces and before returning the completed lease. Evaluator temporary allocations and whole-process RAM still require separate accounting.

`geometry3d.preview` exposes this headlessly with CAD object ID and source revision. It returns drawing-coordinate geometry, not transformed engine coordinates. Uniform parameter spacing is explicitly not a geometric-error tolerance guarantee. Degenerate/folded surfaces may produce degenerate/folded triangles; topology/normal validation, adaptive error control, trims, seam welding, export and viewport caching are later gates.

## Exact affine edit service

`Transform` and `transform_exact` provide original world-space translation, axis rotation, positive uniform scale and plane reflection for exact rational curves and untrimmed control surfaces. Knots, weights and degree remain unchanged. Copy-on-write preserves existing snapshots; input validation, estimated shape bounds and cooperative cancellation protect each operation.

The CAD `geometry3d.transform` command adds bounded multi-object preflight, fresh copy identities, document undo and all-or-nothing mutation. It currently accepts exact CAD shapes only. Meshes, solid topology, interactive references and parametric history require separate contracts. See [manual rebuild coverage](../commands/MANUAL_REBUILD.md) for supported options and resource limitations.

Directional scaling extends this service with `Scale1d`, `Scale2d`, world-axis
`ScaleNu` and explicit-frame `ScaleByPlane`. Plane frames reject nonperpendicular
axes and clean only normalized dot drift within 1e-9. Numeric API coverage remains
partial; native exact curves/control surfaces retain weights, knots and degrees.
