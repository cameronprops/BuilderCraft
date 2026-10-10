# Cadrum / OpenCascade: isolated BRep acceptance candidate

Status: **candidate only**, outside root Cargo workspace. Does not change document geometry or the command registry.

- Upstream: https://github.com/lzpel/cadrum, pinned to cadrum 0.8.20, MIT source wrapper.
- Geometry engine: OpenCascade, LGPL-2.1 with exception/redistribution obligations to confirm for bundled and statically linked binaries.
- This proof is intentionally distinct from the Truck candidate (#30) and opencascade-rs probe (#31). Do not merge either backend as authoritative solely because this test passes.
- Candidate claims requiring evidence: exact BRep boundary, boolean success and failure diagnostics, real volume, face/edge enumeration, binary BRep roundtrip, STEP interoperability, memory use, workload performance and no FFI process termination.
- Native prebuilt OCCT may be downloaded during compile. Production releases require provenance, hashes, source availability, deterministic builds, platform review and a relinkable delivery strategy compliant with LGPL.
- CI runs on one free Ubuntu hosted runner. Windows/macOS/Haiku parity must be evaluated separately before production selection. Haiku may require OCCT source porting.
- Acceptance fixtures in src/lib.rs include box/curved primitives, overlapping box exact booleans, identical operands, face tangency and BRep roundtrip; each tests actual constructed geometry.
- Missing: planar/curved trimmed faces with inner holes, intersection curves, complex edge sewing/healing, singularity and periodic seam tests, scale/tolerance sweeps, STEP roundtrip, shell genus, differential OCCT/Truck parity and instrumented benchmarks.
- Internal document types should remain independent of any FFI library. All exact BRep operations must eventually pass a typed, versioned kernel adapter that returns structured errors and explicit tolerance, not raw vendor handles.
- **Not passed until the corresponding CI run succeeds.**

Run: `cargo test --manifest-path evaluations/cadrum/Cargo.toml -- --test-threads=1`
