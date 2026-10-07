# Kernel dependency candidates

Reviewed public repository metadata on 2026-10-07. These are candidates, not newly adopted dependencies. Recheck licenses, component/transitive licenses and target compatibility before adoption. The first kernel increment uses our existing evaluator.

| Repository | Reviewed head | Declared license | Potential role |
|---|---|---|---|
| [Truck](https://github.com/ricosjp/truck) | 88ed005249e5e3a6b07f62425399435905cd3ab6 | Apache-2.0 | Exact topology, NURBS, tessellation and STEP |
| [Parry](https://github.com/dimforge/parry) | b3c3584ce9be9bf6edb821fd7548b817087131ff | Apache-2.0 | Spatial queries and collision |
| [Rapier](https://github.com/dimforge/rapier) | 3406750a38286a0d3c7d0f4be2f9d53153e81d90 | Apache-2.0 | Rigid-body preview physics |
| [rhino3dm](https://github.com/mcneel/rhino3dm) | bc7b030d79636834e8e1ace3fb17b6c5ad4e2687 | MIT | Optional 3DM format adapter, requires Rust/FFI assessment |
| [gltf](https://github.com/gltf-rs/gltf) | 50d65229477fe5f785c2c90df21eb59c93ea2261 | Apache-2.0 | GLB/glTF data tooling; exporter work still needed |
| [petgraph](https://github.com/petgraph/petgraph) | a4d94bd2c39ac198c22682b2dcb1ff21583d0db0 | Apache-2.0 | Graph topology and scheduling support |
| [nalgebra](https://github.com/dimforge/nalgebra) | b2466e6c4070b06240d929895473c9141fe24720 | Apache-2.0 | Registration/constraint numerical foundations |

Acceptance must include geometry accuracy, hostile inputs, cancellation and retained/temporary memory workloads, native/WASM builds and deterministic fixtures. Rapier does not itself provide Grasshopper/Kangaroo component semantics. Exact CAD remains authoritative when preview meshes/physics are derived.

The visualization increment uses `gltf` 1.4.1 as a dev-only independent GLB parser. Registry manifests for it and its gltf-json/gltf-derive components declare MIT OR Apache-2.0; application export code remains original. No proprietary Rhino or engine code was imported.
