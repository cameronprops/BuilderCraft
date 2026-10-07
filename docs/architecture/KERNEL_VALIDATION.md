# Kernel validation, 2026-10-07

Scoped tests passed: `cargo test -p buildercraft-kernel -p cadcraft-doc -p cadcraft-engine -p cadcraft-io`, 183 tests total (11 kernel, 10 document, 137 engine, 25 IO).

New fixtures cover high-precision identity JSON, unit/axis round trips, concurrent retained-byte rejection, final-owner release, invalid mesh/point-cloud rejection, cancellation, foreign resources, atomic transaction failure, hierarchy cycles/orphans, snapshot restore, exact-buffer sharing and copy-on-write, unchanged legacy shape encoding and metadata-only CAD manifest behavior. Existing project/undo/hostile-input tests passed.

Headless example passed and produced a metadata manifest at revision 3 after insert, rename and restore, with 164 retained geometry bytes.

This is contract and integration evidence. Process RSS, long-running workloads, GPU teardown, desktop UX, inter-app synchronization and native Kangaroo functionality are not validated by these tests.

Full workspace gate passed on Rust 1.95.0: `cargo xtask ci`, all six steps (formatting, Clippy with warnings denied, 339 tests, asset attribution, dependency layering across 17 crates and every configured WebAssembly target including the kernel/UI). Build used a clean temporary target directory, disabled incremental compilation, two jobs and unoptimized development code generation. This avoids stale object artifacts seen in the first build directory.

The GitHub alpha workflow now triggers on main and uses Rust 1.95.0, with a complete validation job plus the existing macOS/Windows/Linux build matrix. Local validation is complete; hosted workflow results are separate.

## Tessellation increment

Full `cargo xtask ci` passed on Rust 1.95.0 with 344 tests and all six gates. New fixtures verify rational curve sampling against the exact evaluator, surface extents/index counts/consistent winding, invalid resolution and sample/work/buffer budget rejection, cancellation, retained-budget failure/release and non-mutating CAD preview API with hostile parameters. This does not certify adaptive error tolerance, manifoldness, trims or viewport visual quality.
