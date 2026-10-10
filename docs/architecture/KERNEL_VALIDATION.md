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

## Latest-main drafting and interchange integration (2026-10-10 UTC)

Reconciled against main `f9ba9088a0ab26190ca73684d5727f2cbfa71aa0`, including
native mesh editing, optional scoped histories, OrbWeaver, LAS/noise adapters
and shared modeling subsets, including persistent undoable mesh Project/Flow. The final source passed all six `cargo xtask ci`
gates on Rust 1.95.0: **646 workspace tests**, formatting, workspace Clippy with
warnings denied, attribution, layering across 18 crates and all 13 configured
WASM libraries. OrbWeaver's headless evaluator is now registered at L2; the
previous full-workspace layering failure is resolved. The native-only JSON
import in visualization is qualified to avoid its WASM warning.

The now-present `tools/verify-worldwright-kernel.sh` passed all eight steps.
`cargo build --locked -p cadcraft -p cadcraft-cli` passed locally. The compiled
CLI created a control curve, saved/reopened `.dftba` and returned the original
control points, degree, weights and knots. A compiled CLI PushPull invocation
returned the expected eight vertices and six quad faces. These are development
builds, not performance measurements or packaged releases.

New fixtures cover off-plane endpoint snaps, layer exclusion, transient drafts,
one-step undo/redo, stale identity/revision/plane rejection, mesh/curve ID
collision rejection, full-viewport drafting consuming mesh-selection clicks,
shared work-budget admission, adaptive orientation cancellation and bounded
STL/OBJ round trips/hostile inputs/precision losses. The exact egui viewport
was captured, rasterized and visually inspected after theme-token integration.

STL reads/writes share one bounded codec path. Command-level triangular OBJ
exchange delegates to the existing named-object parser; it does not replace
native polygon mesh storage or implicitly import geometry into a document.
See `OPEN_SOURCE_INTEGRATION.md` for precise repository reuse and deferred work,
and `../ALPHA_SMOKE_TEST.md` for compiled-program/UI acceptance steps.

Linux/native and portable WASM checks do not establish Windows, macOS or Haiku
window-system behavior, general Rhino parity, full Scan/Terrain functionality,
trimmed BRep/solid support, or aggregate process/GPU memory performance. Hosted
native CI conclusions are separate from this local evidence.
