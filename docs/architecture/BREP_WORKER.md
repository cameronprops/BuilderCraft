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


## Document integration branch (2026-10-10)

The stacked `feature/brep-document-native-integration` branch adds `Drawing.exact_breps`, a
bounded Base64-preserved native OCCT archive for each solid, and optional
`brep.*` CAD/API/CLI commands. This is **not signed off** until the native
integration CI passes and an interactive shaded viewport renders the objects.

- `brep.box`, `brep.sphere`, `brep.cylinder`, `brep.boolean`,
  `brep.inspect`, `brep.preview`, `brep.to_step`, `brep.from_step`,
  `brep.list`, `brep.set` execute through one isolated native worker.
- Put the absolute path to the compiled binary in `WORLDWRIGHT_BREP_WORKER`,
  or install the worker alongside the desktop executable. User-supplied
  document or command JSON is never accepted as an executable pathname.
- The host caps requests to 8 MiB and responses to 16 MiB, reads stdout
  concurrently, terminates timed-out workers after 30 seconds, reports
  child process failure and never changes document geometry on worker failure.
- Up to 4 MiB binary data per exact solid, 64 MiB aggregate stored BRep bytes,
  at most 64 returned solids and 4096 CAD 3D objects; data is copy-on-write
  with `Arc` across undo snapshots. Existing `.bcraft` and `.dftba`
  files missing `exact_breps` still load.
- A `brep.boolean` never destructively mutates operands, and successful
  multiple-solid results enter one undoable transaction. An empty Boolean
  intersection produces no geometry and no document mutation.
- Ordinary DXF, PDF, PNG and other unsupported exports reject exact-solid
  documents rather than silently discarding BRep topology. `brep.to_step`
  intentionally returns an exact Base64 STEP payload to the caller.
- Only the OCCT worker can authenticate the meaning of native BRep bytes:
  project load checks decoding, size, statistics and IDs, but does **not**
  pretend to validate arbitrary topological content. Run `brep.inspect`
  to reopen and check its shape through OCCT.

### Native test path

```sh
cargo build --manifest-path evaluations/cadrum/Cargo.toml --bin worldwright-brep-worker
export WORLDWRIGHT_BREP_WORKER="$PWD/evaluations/cadrum/target/debug/worldwright-brep-worker"
cargo test --locked -p cadcraft-io exact_brep
cargo test --locked -p cadcraft-engine exact_worker_document_undo_and_roundtrip_when_worker_is_installed -- --nocapture
```

On Windows, point the variable to the compiled `.exe`. The second fixture only
executes when the variable is set; absence of the worker cannot be interpreted
as an exact-BRep test pass.

### Remaining blockers for a finished Rhino-depth alpha

Full source/third-party binary licensing and deployment; source and destination
tolerance equivalence; topological naming across revisions; edit-in-place and
model browser/selection; native shaded viewport caching and depth; complex
self-intersection/tangency/manifoldness stress cases; standalone Haiku native
worker support. No routine should claim the full alpha closed before those gates.
