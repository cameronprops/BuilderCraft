# Kernel validation, 2026-10-07

Scoped tests passed: `cargo test -p buildercraft-kernel -p cadcraft-doc -p cadcraft-engine -p cadcraft-io`, 183 tests total (11 kernel, 10 document, 137 engine, 25 IO).

New fixtures cover high-precision identity JSON, unit/axis round trips, concurrent retained-byte rejection, final-owner release, invalid mesh/point-cloud rejection, cancellation, foreign resources, atomic transaction failure, hierarchy cycles/orphans, snapshot restore, exact-buffer sharing and copy-on-write, unchanged legacy shape encoding and metadata-only CAD manifest behavior. Existing project/undo/hostile-input tests passed.

Headless example passed and produced a metadata manifest at revision 3 after insert, rename and restore, with 164 retained geometry bytes.

This is contract and integration evidence. Process RSS, long-running workloads, GPU teardown, desktop UX, inter-app synchronization and native Kangaroo functionality are not validated by these tests.

Full workspace gate passed on Rust 1.95.0: `cargo xtask ci`, all six steps (formatting, Clippy with warnings denied, 339 tests, asset attribution, dependency layering across 17 crates and every configured WebAssembly target including the kernel/UI). Build used a clean temporary target directory, disabled incremental compilation, two jobs and unoptimized development code generation. This avoids stale object artifacts seen in the first build directory.

The GitHub alpha workflow now triggers on main and uses Rust 1.95.0, with a complete validation job plus the existing macOS/Windows/Linux build matrix. Local validation is complete; hosted workflow results are separate.

## Tessellation increment

Full `cargo xtask ci` passed on Rust 1.95.0 with 344 tests and all six gates. New fixtures verify rational curve sampling against the exact evaluator, surface extents/index counts/consistent winding, invalid resolution and sample/work/buffer budget rejection, cancellation, retained-budget failure/release and non-mutating CAD preview API with hostile parameters. This does not certify adaptive error tolerance, manifoldness, trims or viewport visual quality.

## Visualization feed and production metadata increment

Full `cargo xtask ci` passed on Rust 1.95.0: 352 tests, formatting, Clippy with warnings denied, asset attribution, dependency layering and all configured WebAssembly checks. Independent MIT/Apache-2.0 glTF 1.4.1 parser tests check the emitted GLB container, accessor counts/bounds, empty/hidden scenes and geometry update fingerprints. Production metadata save/reopen, multiple assignments, hierarchy/reference validation and undo/failed-edit preservation are tested. Local publication tests check monotonic sequences, project mismatch and single-writer rejection.

Native CLI smoke test ran the saved-project watcher through five publications: initial massing, rename, geometry edit, deletion and delete-all. IDs stayed stable on rename, geometry fingerprint changed on edit, the empty snapshot cleared all objects and two GLBs were retained. The original synthetic fixture is reproducible with `cargo run -p cadcraft-io --example massing_feed -- /tmp/massing.bcraft`.

The original Unreal adapter is SOURCE ONLY pending host compilation/editor/runtime acceptance. Neither UnrealEditor nor UnrealBuildTool is available here. Native publication/portable export results do not establish a working Unreal walkthrough, collision fidelity, bidirectional editing, equipment simulation or whole-process/GPU memory performance. See `bridges/unreal/README.md` for the acceptance procedure.

## Viewport selection validation (2026-10-09)

Final source passed all six `cargo xtask ci` gates on Rust 1.95.0: 490 workspace
tests, formatting, Clippy with warnings denied, assets, dependency layers and
all configured WebAssembly library checks. A clean temporary target directory,
two jobs, disabled incremental compilation and
`RUSTFLAGS='-C opt-level=0 -C codegen-units=1 -C debuginfo=0'` avoided corrupt
build artifacts observed in the first target directory. No dependency was added.

Evidence covers pixel/depth math, stable overlap ties, hidden/locked layers,
curve and surface wires, aggregate work rejection, invalid query and selection
preservation, no selection-only geometry revision/undo, real egui pointer
click/toggle/empty/camera-drag/gizmo interactions, and sampling-failure
propagation. The full viewport's selected-wire highlight and controls were
rendered and inspected headlessly. This does not certify native window-system
UX on every OS, exact intersections, surface-interior or subobject picking,
window selection, snapping or aggregate process/GPU memory behavior.

The inherited visualization `Value` import was qualified at its native use to
remove its WebAssembly warning. The documented verify-worldwright-kernel wrapper
is still absent; checked-in `cargo xtask ci` was run directly. Hosted workflows
remain manual-only and were not invoked.
