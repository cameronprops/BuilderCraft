# Candidate open-source libraries for Worldwright

Reviewed: 2026-10-08. **Research only**. These repositories have not been
integrated as new Worldwright dependencies. Verify the exact source revision,
license files, third-party code and transitive dependencies before adoption.

Worldwright owns one canonical shared Rust operation per mathematical
function. Reuse mature algorithms *behind the shared geometry/kernel API* so
the CAD interface, OrbWeaver nodes and external API invoke the same operation.
Do not fork the mathematical engine by operating system or application.

| Repository | License verified | Good fit | Caveat |
|---|---|---|---|
| [mattatz/curvo](https://github.com/mattatz/curvo) | MIT (repository) | Independent NURBS algorithms and construction tests for loft, sweep, intersections | Audit precision, tolerances, data shape and dependencies before replacing any existing exact geometry code |
| [georust/robust](https://github.com/georust/robust) | MIT OR Apache-2.0 | Adaptive predicates for robust orientation and degeneracies | Evaluate predicate semantics, boundary cases and f64 compatibility |
| [Stoeoef/spade](https://github.com/Stoeoef/spade) | MIT OR Apache-2.0 | Delaunay triangulation for planar meshing and terrain | Not a complete 3D BREP triangulator |
| [georust/rstar](https://github.com/georust/rstar) | MIT OR Apache-2.0 | Spatial index for scene, mesh, nearest-neighbor and selection queries | Benchmark update costs and memory vs simple scans on small drawings |
| [zeux/meshoptimizer](https://github.com/zeux/meshoptimizer) | MIT | Mesh optimization, LOD and GPU-friendly derivatives for preview / Unreal exchange | C++ core and binding complexity; preserve native quads/exact source |
| [Auburn/FastNoiseLite](https://github.com/Auburn/FastNoiseLite) | MIT | Deterministic rock, terrain, volume and procedural textures, including Rust implementation | Validate seed/output determinism and scale/units |
| [koide3/small_gicp](https://github.com/koide3/small_gicp) | MIT | Metrology: efficient point-cloud rigid registration and ICP/GICP | C++ native dependency; may need worker isolation; Haiku compatibility unproven |
| [isl-org/Open3D](https://github.com/isl-org/Open3D) | MIT core LICENSE | Full-featured point clouds, alignment, reconstruction and inspection reference/backend | Large C++ library and third-party dependencies require independent license/deployment checks |
| [gltf-rs/gltf](https://github.com/gltf-rs/gltf) | MIT OR Apache-2.0 | Standard glTF parsing/validation | Avoid redundant loader if existing exchange adapter already covers acceptance fixtures |
| [Auburn/FastNoise2](https://github.com/Auburn/FastNoise2) | MIT | SIMD node-based procedural noise | Optional C++ backend only after benchmarks and cross-platform proof |
| [microsoft/mimalloc](https://github.com/microsoft/mimalloc) | MIT | Potential memory allocator/fragmentation improvements | Never switch global allocator without controlled cross-platform benchmarks |
| [fogleman/sdf](https://github.com/fogleman/sdf) | MIT | Reference for signed-distance procedural solids | Python, not a production native-Rust engine |

## Explicit distinctions

- MIT, and MIT OR Apache-2.0 **when used under its MIT option**, fit a
  strict MIT dependency requirement, subject to notices and audits.
- GitHub's license badge is not sufficient evidence of package license
  terms or legal clearance for all bundled assets.
- [dimforge/nalgebra](https://github.com/dimforge/nalgebra),
  [dimforge/parry](https://github.com/dimforge/parry),
  [dimforge/rapier](https://github.com/dimforge/rapier) and
  [ricosjp/truck](https://github.com/ricosjp/truck) currently advertise
  Apache-2.0 in the repository/package metadata, **not MIT**.
- [elalish/manifold](https://github.com/elalish/manifold) is Apache-2.0;
  [mkeeter/fidget](https://github.com/mkeeter/fidget) is MPL-2.0.
  They deserve separate technical consideration only under an explicitly
  broader licensing policy.

## Adoption priority and gates

1. Robust geometric predicates, then spatial indexing, so one shared
   implementation accelerates CAD, OrbWeaver, picking and metrology.
2. Mesh optimization for derived viewport/engine geometry, without
   replacing exact or editable source structures.
3. Noise algorithms behind one deterministic shared procedural operation.
4. Compare NURBS behavior against `cadcraft-geom` using hard numerical
   tolerance and roundtrip fixtures, before replacing implementations.
5. Evaluate small_gicp/Open3D as opt-in metrology adapters; keep native
   Worldwright functionality independent of the host.
6. Measure mimalloc after baseline RSS and fragmentation profiling.
   Never use an allocator as a substitute for resource budget admission.

For every candidate: pin a commit; confirm the source and transitive license;
isolate behind a shared API with typed units/tolerances; test invalid,
degenerate, huge, and ordinary geometry; benchmark peak RSS and speed; test
Windows, macOS, Linux and Haiku separately; publish only proven support.
