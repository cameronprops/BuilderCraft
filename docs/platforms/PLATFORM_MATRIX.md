# Native platform support and localization

**Decision (2026-10-08):** Windows, macOS, Linux and Haiku advance
**concurrently**, against one shared Rust geometry, scene, document, command
and graph implementation. This is a development and validation requirement,
not a claim of release readiness or simultaneous binary delivery.

## Native release matrix

| Platform | Native UI strategy | Rendering strategy | Invited alpha | Open beta |
|---|---|---|---|---|
| Windows (x86_64 first) | Existing eframe desktop; native Win32 integration only at the platform boundary | wgpu/DirectX where validated, software fallback | Required smoke and roundtrip | Installer, scale/IME, low-memory tests |
| macOS (Apple Silicon and Intel) | Existing eframe desktop; native AppKit integration only at the boundary | wgpu/Metal where validated, software fallback | Required smoke and roundtrip | Bundle/notarization, memory pressure tests |
| Linux (x86_64 first; aarch64 as tested) | Existing eframe desktop, X11 and Wayland validated separately | wgpu/Vulkan when supported, software fallback | Required smoke and roundtrip | AppImage/DEB/RPM packaging and Mesa driver tests |
| Haiku (x86_64) | **Native App Kit or proven Haiku-native window host**; existing eframe stack is not validated | CPU viewport first, GPU only after driver/backend proof | Native kernel/CLI preview gate first; separate GUI gate before desktop invite | Installable .hpkg and all shared workflow gates |

**Status:** The repository has desktop Rust source for Windows/macOS/Linux,
but builds, test passes, installer functionality and real hardware behavior
must be verified. Haiku's native CLI packaging gate is *authored but has not
run on Haiku*. Haiku desktop host, rendering and .hpkg are *not implemented*.
See [Haiku gate](HAIKU.md).

A platform is not blocked from shared core development by another platform's
driver or UI gaps. However, any *publicly advertised native build* must pass
its own gates. Platform defects must not silently turn into shared-kernel
forks or duplicated mathematical implementations.

## Shared rules and platform adapters

The shared Rust kernel holds all numerical geometry, exact curves, mesh
operations, feature history, parametric graph evaluation, file schema and
reversible commands. Platform adapters own only:

- **Memory pressure and telemetry:** OS available/remaining memory,
  per-process limits, GPU budgets and pressure notifications.
- **Task scheduling:** priority/QoS mapping for interactive vs background
  work, bounded worker count, wake/suspend/close cleanup.
- **Rendering:** GPU device/driver selection, software path, buffer lifetime,
  relative-origin precision, display scaling and context-loss handling.
- **Filesystem and app integration:** case sensitivity, atomic replacement,
  temp directories, file dialogues, sandbox/permission prompts, native
  packages and clipboard/drag-and-drop.
- **Accessibility and localization:** keyboard layout and IME composition,
  font fallback/Unicode shaping, screen reader descriptions, high DPI,
  locale-specific UI dates/numbers and localized command labels.

No OS-specific API call, display framework or locale formatting belongs
inside mathematical geometry routines. All adapters provide typed values
to shared services.

## Memory adapters and available OS facilities

| OS | Telemetry and pressure adapter to implement | Important nuance |
|---|---|---|
| Linux | `/proc/meminfo` MemAvailable, cgroup v2 `memory.max`/`memory.current`, memory PSI and available renderer-reported VRAM | Container memory limits take precedence over host free RAM; Mesa/iGPU shares physical RAM |
| Windows | `GlobalMemoryStatusEx`, per-process/job-object limits, low-memory notification; DXGI `IDXGIAdapter3::QueryVideoMemoryInfo` | DXGI has budget and current use; integrated/discrete GPUs differ |
| macOS | `os_proc_available_memory`, dispatch memory-pressure events, GPU resource telemetry from the active Metal backend | Unified memory on Apple Silicon means GPU and CPU budgets overlap |
| Haiku | `get_system_info` (`max_pages`, `used_pages`, `cached_pages`, `free_memory`) with native process status and tested fallback | Page accounting and Vulkan support require native experiments; treat missing capacity as unknown |

Do not confuse total RAM, currently available RAM, process allocatable
headroom, and GPU allocation budget. Values are live **advisory snapshots**,
not permissions to allocate without bounds. Check OS/SDK availability and
units, and validate each adapter on the OS where it runs. Platform probes
that cannot determine a value return `None` rather than assuming it.

## Portable policy implemented in shared kernel

`crates/kernel/src/resource_policy.rs` provides:

- `ResourceLimits::from_sample`: conservative quotas based on physical
  total, available, process remaining, pressure and CPU count.
- Partitions for retained geometry, undo, derived previews, transient jobs,
  and transfer buffers. GPU gets a separate capped reported budget.
- `ResourceLedger::reserve` returns a unique RAII lease; all checks and
  counters are synchronized so two workers cannot admit the same quota.
- `update_limits` lowers new-admission capacity without invalidating
  existing resources. Client code must trim real caches and undo histories
  where possible and defer background work under pressure.

The ledger is **process-local reservation infrastructure**, not proof of
real allocation tracking or integration with current scene budgets.
`GeometryBudget` still separately accounts for geometry leases. Consumers
must avoid double-accounting, tie one admission to one allocation lifetime,
and arrange aggregate budgets across documents and multiple processes.
OS probes, live re-planning, UI consumption and process-wide reconciliation
are still integration tasks.

## Consistency and localization acceptance

All platform builds must use identical `.dftba` semantics and migration
rules for legacy `.bcraft`. File schemas, IDs, geometry coordinates,
units and saved numeric values are **locale-invariant**. A user can choose
millimetres or inches; switching UI language or decimal punctuation cannot
change saved geometry.

Locale is for display and user input interpretation only: a decimal comma
can be accepted in a locale-aware numeric field, but persisted quantities
must round-trip across English/French/German/Japanese system locales.
Test localized decimal input against tuple separators, command parsing and
reopening on a differently localized system. Retain Unicode names and
metadata without lossy ASCII conversion.

Key mappings use physical keys/keyboard layout appropriately: Ctrl vs Cmd,
dead keys, modifier selection, right/middle pointer buttons and IME text
composition are all platform tests. Native file dialogs, drag/drop,
screen-reader accessibility and high-DPI coordinate-to-pixel transforms
must be tested separately on each OS.

## Release gate for each platform

1. Native host/compiler and required SDKs verified; document OS, CPU
   architecture, compiler commit and graphics driver.
2. Shared geometry/scene/unit/regression test corpus passes without
   changing expected numeric behavior merely for an OS.
3. Native interactive viewport, selection, transforms, layers, undo/redo,
   graph preview/bake, save/reopen and a supported exchange roundtrip.
4. Memory test: sustained edit/undo, multiple documents, scan preview,
   cancellation, GPU loss, pressure increase and process close.
5. Numeric/localization test across locale and keyboard variants,
   plus DPI/file path/sandbox tests.
6. Signed or unsigned artifact honestly labeled; native package
   install/start/upgrade/uninstall tested with exact version and checksum.

Run headless tests on all four OSes as soon as toolchains are available;
native end-to-end GUI gates remain per-platform. Do not equate cross-compile
success with execution, or CLI with an interactive CAD desktop.

## Immediate dependency chain

1. Finish consuming portable reservation tokens in document, undo, render,
   work queues and import paths, keeping live ownership and cancellation.
2. Implement four isolated memory telemetry adapters and pressure callbacks;
   test samples and transitions using injected fake adapters.
3. Separate platform host/window input and render backends from common CAD
   and OrbWeaver UI logic. Haiku can start with a software canvas.
4. Add equivalent per-platform native smoke and scripted corpus runners.
5. Publish alpha/beta artifacts only after real hardware/OS validation.

No effort should move NURBS, Boolean calculations or graph solvers into an
operating-system kernel; that would increase crash risk without improving
their CPU arithmetic.
