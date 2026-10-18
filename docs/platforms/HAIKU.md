# Worldwright on Haiku OS

**Platform commitment:** Haiku x86_64 is a **native experimental release
target for the invited alpha and open beta**, alongside the established
desktop targets. This is a target, **not a claim that native GUI binaries
already exist or that a release date is guaranteed**.

## Architectural boundary

Worldwright's Rust mathematical/scene kernel, geometry commands, history
and file serialization remain operating-system independent. No modeling
algorithms move into Haiku kernel space. Operating-system integration belongs
in replaceable adapters for windows, pointer/keyboard events, file dialogs,
rendering, threading and package metadata.

The shared kernel `buildercraft-kernel` is intentionally independent of
`eframe`. A native Haiku app must call the same kernel/command operations as
CAD, OrbWeaver and headless APIs. No second mathematical implementation.

## Current support, checked against the repository

| Area | Current state | Gate |
|---|---|---|
| Rust Haiku target | Rust documents `x86_64-unknown-haiku` as Tier 3 with standard-library and host-tool entries | Demonstrate a working Haiku-native toolchain for the pinned 1.95 workspace |
| Shared Rust kernel | Portable source exists; **not yet compiled/tested on Haiku** | Kernel regression tests pass natively |
| Native CLI | Source exists; **not yet compiled/tested on Haiku** | Build, run DXF sample/convert/reopen and document command behavior |
| Desktop CAD | Existing `cadcraft` binary depends on eframe/winit/wgpu; **not ported to Haiku** | Native windows, input, viewport, selection, editing and persistence |
| Native packaging | CLI-only tarball builder added, **unexecuted on Haiku**; .hpkg not implemented | Installable, versioned native desktop package, app metadata and smoke test |
| OrbWeaver and other apps | Not independent Haiku applications yet | Shared graph operations and each actual standalone UI validated separately |

Do not put Haiku into the automatic `release.yml` job matrix merely because
the target triple exists: GitHub's hosted runners are not native Haiku hosts.
No unverified binary may be attached to a Worldwright release.

## Development sequence

1. **Native compiler gate:** On Haiku x86_64, establish `rustc`, `cargo`,
   linker, native libraries and the pinned workspace version. Record the
   toolchain/Haiku revision in results. Haiku's Rust target is Tier 3;
   `rustup target add` is not guaranteed to provide prebuilt artifacts.
2. **Shared core:** Pass `cargo test --locked -p buildercraft-kernel`.
   Confirm portable numerical tolerances, serialization, cancellation,
   mesh editing and revision safety behave identically.
3. **Headless executable:** Pass `cargo test --locked -p cadcraft-cli`
   and `bash packaging/haiku/package.sh`. Resolve any dependency
   incompatibilities without adding OS code to the geometry math.
4. **Real Haiku desktop:** Prototype a Haiku App Kit host
   (`BApplication`/`BWindow`/`BView`) or a verified alternative
   Haiku windowing backend. Bridge input, DPI, clipboard, redraw,
   file pickers and application lifecycle into reusable Rust UI logic.
   Start with a CPU/soft-rendered viewport if accelerated rendering
   is unavailable, then profile a GPU path separately. Extract the
   current UI's hard dependency on `egui-wgpu` if necessary; keep
   core CAD/OrbWeaver operations shared.
5. **Packaging:** Make and smoke-test an actual native desktop .hpkg,
   plus checksums, licenses, icons, application signature, file type
   registration and clean uninstallation. The separate CLI preview
   is not a substitute for this step.

## Invited alpha acceptance

A *desktop* Haiku invited-alpha build is eligible only when it:

- Launches as a native Haiku application and opens an editable viewport.
- Creates/edits and selects geometry through shared commands.
- Supports transforms, undo/redo, layers and basic object organization.
- Saves and reopens `.dftba`, and reads compatible legacy `.bcraft`
  projects without silent loss of supported data.
- Imports/exports at least one actually validated exchange format.
- Completes a native regression and manual tester checklist, with a
  published known-issues list for software rendering and drivers.

The headless CLI package can be offered to invited testers **earlier**
as a separately labeled technology preview once its own native tests pass.
It must never be represented as the complete native CAD application.

## Open beta acceptance

The Haiku graphical build passes the invited-alpha gates and adds:
reproducible installation/update/uninstall, crash diagnostics,
memory/performance measurements on representative CAD and mesh scenes,
input/clipboard/file-dialog checks, stable file roundtrips, documented
GPU/software-rendering limitations, and a reproducible native .hpkg
release artifact. Beta coverage is capability- and platform-specific,
not a claim of full Rhino or Grasshopper parity.

## Non-goals

- Moving geometry routines into the operating-system kernel.
- Replacing Linux/Windows/macOS as the primary engineering targets.
- Bundling a Linux or web frontend and calling it Haiku-native.
- Publishing a Haiku build without executing it on Haiku.
