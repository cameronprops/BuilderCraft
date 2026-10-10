# Oriented path array acceptance

## Shared command contract

- Kernel: `kernel.array.path_oriented`
- Native CAD/API: `worldwright.array.path_oriented`
- OrbWeaver: `orbweaver.array.path_oriented`
- Persistent selected object: `mesh3d.array` with `mode=path_oriented`
- Required geometry: ordered, finite 3D source points and an ordered 3D polyline path
- Parameters: positive count, guide up vector, source rotation anchor
- Output: one separate geometry copy per equal-arc-length station, including original at station zero
- Orientation: minimal-rotation transported tangent frame, with explicit convention for 180-degree reversals
- Existing `array.path` is unchanged (translation-only behavior).

## Acceptance tests

- Sharp planar corner rotates the source tangent while preserving model-space lengths.
- 3D path bends and vertical tangent changes maintain a stable frame.
- Reversed path segment is deterministic and finite.
- Parallel initial guide-up, zero-length path, invalid anchor and instance/resource overflow fail atomically.
- Direct CAD shared dispatcher and OrbWeaver node return identical geometry trees.
- Document-backed source mesh remains unchanged, generated copies have distinct persistent IDs, and undo removes only the copies.
- Fixed Rust 1.95: `python3 tools/check_paired_tools.py`, `cargo fmt --all -- --check`, `cargo test --locked -p buildercraft-kernel -p orbweaver -p cadcraft-engine`, `cargo clippy --locked -p buildercraft-kernel -p orbweaver -p cadcraft-engine --all-targets -- -D warnings`.

## Scope limits

This is a polyline tangent-array feature, NOT a full Rhino-style ArrayCrv implementation. Follow-on acceptance should cover exact NURBS arc-length evaluation, user-specified bank/twist profiles, seam correction on closed loops, associative feature-history re-evaluation, and GUI grips and preview. Existing geometry budget is retained: 256 stations maximum and at most 65,536 emitted point instances. No claims of meshed-solid collision clearance or fabrication validity are made.
