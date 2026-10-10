# OpenSCAD inside WorldWright: implementation stages

Tracked under [immediate modeling agenda #26](https://github.com/cameronprops/BuilderCraft/issues/26).

## Stage A — implemented in this branch: explicit external rendering bridge

Command:
```sh
cadcraft-cli openscad-render --allow-execute --bin /absolute/path/to/openscad input.scad output.stl
cadcraft-cli openscad-render --allow-execute --bin /absolute/path/to/openscad input.scad output.3mf
```

- Requires an existing local OpenSCAD executable; no bundled or downloaded proprietary binaries.
- Explicit `--allow-execute` and `--bin`; never executes on document open, preview, import or autosave.
- A child process is launched **without a shell** with 120-second timeout and cleanup on failure, for .stl or .3mf only, with a 128-MiB post-render output limit and no overwrite of existing files.
- OpenSCAD remains the source-authoritative compiler for SCAD files; WorldWright can import its mesh output via the existing geometry import pipeline where format support exists.
- Native CLI tests check consent and supported format validation. Integration with actual OpenSCAD must be tested on systems with OpenSCAD installed before user-facing render parity is claimed.

**Security warning:** This is NOT an OS sandbox. `.scad` include/use/import can access other local paths; generated output may grow before postprocess caps. Only render user-trusted files. UI auto-render must wait for an OS-specific sandbox/allowlisted path policy, disk/memory resource budget, and clear user permission.

## Next stages — not yet implemented

1. A dockable syntax-aware `.scad` code pane integrated with saved documents and undo.
2. OpenSCAD Customizer parameter inspector, versioned parameters, source mapping and error line diagnostics.
3. 3MF/STL import conversion to validated mesh geometry with explicit unit conventions; roundtrip tests.
4. Script-to-OrbWeaver evaluated geometry bridge, source protection, cancellation/jobs.
5. Benchmark OpenSCAD Manifold / CGAL, Rust Manifold CSG and Truck BRep backends against current geometry before choosing canonical code paths; adapters are not a second private modeling engine.
6. An in-process engine only after GPL-2.0-or-later, CGAL linkage exception, transitive dependencies and cross-platform packaging notice/source compliance audit. Both external and eventual native paths require explicit trust boundaries.

Upstream: [OpenSCAD repository](https://github.com/openscad/openscad), [CLI manual](https://files.openscad.org/documentation/manual/Using_OpenSCAD_in_a_command_line_environment.html). Copyright/license stays with upstream; no OpenSCAD source copied in this branch.
