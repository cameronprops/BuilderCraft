# Rust memory and job requirements

Review date: 2026-10-07. Inspection of alpha source, not a completed memory/leak benchmark. Applies to every independent app and external worker.

## Current evidence and gaps

| Area | Source evidence | Work remaining |
|---|---|---|
| Unsafe code | Workspace `unsafe_code = "forbid"` in `Cargo.toml` | Check each crate inherits lints; review dependencies/FFI separately |
| Drawing ownership | `Arc<Drawing>` snapshots and chunked `Arc` entity store | Track actual retained bytes, deduplicate immutable large geometry |
| New 3D storage | `Vec<GeometryObject>` with `Arc` exact shapes; edits use copy-on-write | Shared retained geometry budget in kernel; CAD undo aggregate byte limit, decoder/temporary workspace and GPU accounting still pending |
| Undo | Programmatic path caps snapshots at 2,000 | Count is not a byte budget; audit interactive/redo paths, add per-document byte budget and spill/checkpoint policy |
| NURBS bounds | Curves capped at 4,096 controls/degree 5; surface axes capped at 128 | Aggregate scene/job budgets, mesh/point cloud caps and CPU work limits |
| Project input | 128 MiB byte check and post-parse item/shape checks | Deserialization allocates before structural checks; bound input upstream and decoded arrays, nesting, total samples and archive expansion |
| API/control | Existing JSON lines and queued UI requests | Bound messages, queue length, replies, connection rate, payloads and cancellable jobs |
| Rendering | Derived preview geometry | Measure upload/staging/caches and release GPU buffers when document/jobs close |

## Required implementation policy

- Scene owns immutable geometry resources by generational handle or shared immutable allocation; edits create replacements and retain only necessary previous resources. Derivatives link to source revision/content hash. Avoid strong `Arc` cycles: parent links are IDs or `Weak` references.
- Every expensive request estimates retained output, temporary workspace and cache bytes with checked arithmetic before allocation. Budgets cover active scene, undo, workers, CPU/GPU caches and bridge transfer together, across concurrent jobs. Use fallible reservation where meaningful; allocation checks do not guarantee process-wide OOM recovery.
- Per-job limits cover topology count, voxel extent/resolution, sample count, hierarchy/graph depth, decompressed bytes, task queue and iteration/time budget. Reject pathological tessellation and allocations before work starts. A worker process is appropriate for native adapters and crash-prone importers.
- Long operations run off the UI thread with a bounded worker pool. Cancellation is observed within bounded work chunks; progress and byte usage are visible. Publish a new revision only after validation. Cancelled/failed results release buffers, leave previous valid objects intact and report the cause.
- LRU caches have explicit byte budgets; cache keys include geometry/parameter revision, units, tolerance, representation and quality. Coalesce obsolete previews. Low-quality preview and production tessellation do not occupy unbounded parallel caches.
- Imports and saves stream or use bounded staging where practical. Preserve original inputs. Save atomically with migration checks. Shared-memory/FFI resources have explicit lifetime/close semantics; a disconnected adapter cannot retain an unlimited resource lease.
- Use f64 for authoritative geometry; derive relative-origin render buffers at appropriate precision. Huge coordinate magnitudes need explicit transform/frame handling rather than hidden downcasts.

## Release gates

Measure repeated edit/undo/redo, open/close, import/export, cancelled repair, failed bridge transfer, graph reevaluation and GPU teardown. Track peak RSS, retained allocations/resources, frame time and queue length at realistic model sizes. Warm caches may stabilize above baseline; unexplained monotonic retained growth blocks release. Test aggregate budget rejection before large allocation and responsiveness during cancellation.

Run Rust format/lint/unit/integration gates for code changes; exercise sanitizers/Miri where supported and meaningful, especially FFI/unsafe dependencies. Review allocator errors, index arithmetic and validation before deserialization. Do not claim the suite handles memory correctly solely because it compiles or uses Rust. Budget tests and measured workloads must accompany the relevant feature implementation.
