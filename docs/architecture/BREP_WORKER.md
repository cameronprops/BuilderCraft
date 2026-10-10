# WorldWright exact BRep process boundary

**Status:** experimental worker implementation on `feature/brep-occt-worker`. The validated OCCT/Cadrum candidate on PR #32 stays isolated and unchanged until this worker passes. This is not yet integrated into the CAD document or saved-project schema. WorldWright keeps its own Rust NURBS/scene representation, and the native process returns exact OCCT BRep bytes.

## Why an OS process?

Cadrum is a Rust wrapper around OpenCascade C++. C++ can abort or corrupt memory outside Rust error handling. A separate process lets the calling CAD session reject the operation without committing any altered geometry. The host must enforce timeout and resource limits; the worker is **not** a security sandbox.

## Build, test, invoke

```sh
cargo test --manifest-path evaluations/cadrum/Cargo.toml --all-targets -- --test-threads=1
cargo run --manifest-path evaluations/cadrum/Cargo.toml --bin worldwright-brep-worker <<< '{"op":"box","min":[0,0,0],"max":[2,3,4]}'
```

A one-shot request goes to stdin and one JSON object comes back on stdout. A successful shape response includes `{"ok":true,"solids":[{"brep":"<Base64 of exact binary BRep>","volume":24,"faces":6,"edges":12}]}`; failures include `{"ok":false,"error":"..."}`. The worker is isolated to one request per process; restart on crash. Do not store BRep as a shaded triangulation.

| `op` | Required input | Response |
| --- | --- | --- |
| `box` | `min:[x,y,z], max:[x,y,z]` | One exact closed solid |
| `sphere` | `radius` | One exact closed curved solid |
| `cylinder` | `radius, height:[dx,dy,dz]` | One exact closed curved solid |
| `boolean` | `operation: union\|difference\|intersection, left_brep, right_brep` | Zero or more closed exact solids |
| `inspect` | `brep` | One shape and its volume/topology counts |
| `to_step` | `breps:[...] ` | `step` Base64-encoded STEP bytes |
| `from_step` | `step` Base64-encoded STEP bytes | Zero or more exact solids |

Optional `tolerance` for all operations defaults to 1e-7 document units. It currently controls primitive-input guards **only**, not the internal OCCT Boolean fuzzy tolerance. Worker requests are capped at 8 MiB, encoded geometry at 4 MiB per payload, responses at 16 MiB, output solid count at 64. Treat these as early alpha bounds, not production quotas.

**Known issue:** the native CellsBuilder fails when Boolean operands refer to identical shapes. The Rust adapter resolves proven identical objects/boxes; the worker resolves *byte-identical* persisted BRep requests before invoking C++. Equal volumes/bounds **do not** imply equal exact geometry. Other independently serialized yet geometrically identical BReps remain a regression target.

**Remaining acceptance:** reliable triangle/trimmed face tessellation for shaded view, topology naming, trimmed NURBS conversion, shell validation, healing, large offsets/scale tolerance, general Boolean ambiguity, native + STEP material/units, atomic CAD engine command, save/reopen, GPL/LGPL and bundled OCCT binary distribution audit, Haiku native. A passing kernel test is not a successful whole-CAD alpha.
