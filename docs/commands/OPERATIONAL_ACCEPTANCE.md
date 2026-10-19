# Worldwright command operational acceptance gate

Baseline: **365** engine command registrations, inspected on 2026-10-18.
Source of truth: `crates/engine/src/cmd/mod.rs` via
`cadcraft_engine::command_specs()`. Do not treat reference command lists,
API names, menu visibility or registration as functionality.

## Automated first pass

Run from the repository root with the pinned Rust toolchain:

```sh
cargo fmt --all -- --check
cargo test -p cadcraft-engine --test operational_commands -- positive_operational_cases
cargo test -p cadcraft-engine --lib failed_registered_command_restores_document_selection_and_redo
```

The ordinary test checks independently specified valid command paths. The
**strict release gate deliberately remains red** until every live command
has a positive acceptance case:

```sh
cargo test -p cadcraft-engine --test operational_commands \
  -- --ignored --exact release_gate_no_command_without_operational_case
```

Do not delete the ignore attribute merely to make ordinary workspace CI green.
Run the explicit strict gate for alpha release and treat omitted commands as
missing coverage rather than passing. The fixture table is in
`crates/engine/tests/operational_commands.rs`. Add real assertions for geometry,
coordinates, returned IDs and state changes, not just `is_ok()`.

**Current tranche (2026-10-18):** 139 independently asserted
positive command fixtures are authored against 396 registered commands;
**257 commands still lack success-path fixtures**. Main contains 134
merged success-path cases including PR #114's native NURBS/Geometry3D
coverage. This branch adds five document-backed model-browser cases:
Create, List, Rename, Select and Visible, with asserted hierarchy, actual
owned geometry, selection, edits and undo. All merged positive fixtures
must pass focused modeling, native Linux/macOS/Windows and exact OCCT
integration before counting as accepted. Positive cases alone do not
constitute five-gate operational release certification.

## Completion criteria for EACH command

1. **Correctness:** invoke with valid geometry and meaningful parameters;
   inspect the resulting geometry or state against independently derived
   expected values at declared units and tolerances.
2. **Failure preservation:** invalid, degenerate, out-of-range, stale-ID and
   cancellation paths must not leave partially mutated document state,
   selection, feature history or untracked undo entries.
3. **Persistence:** when a command changes document data, verify undo, redo,
   native `.dftba` save and reopen preserve the intended result.
4. **Operational interface:** verify availability, command/API arguments,
   interaction or UI path, preview/picking behavior and error diagnostics.
   Commands requiring a BRep worker, platform feature or external bridge
   need a separate required test job with that dependency installed.
5. **Stability:** reasonable resource bounds, no panic, no hidden geometry
   degradation and a representative large or hostile input regression.

Non-mutating query commands should test unchanged revision/document, and
documentless commands should exercise their enabled/disabled states. UI-only
controls do not belong in the engine registry count, but need their own desktop
interaction test matrix.

## Enforcement sequence

- A case in the acceptance table proves only the specifically asserted
  **valid-path behavior**, not the complete criteria above.
- Any affected code requires Rust formatting, workspace clippy/tests and
  native operating system smoke tests. A passing test on a single OS is not
  cross-platform acceptance.
- Preserve evidence of exact command IDs, test names, build SHA and execution
  results. Promote to fully operational only when all relevant criteria are
  verified and an operational build runs successfully.
- Never change an unsupported implementation to a silent no-op or fake
  success result merely to satisfy a test.

## Cross-references

- `docs/roadmap/OPERATIONAL_BUILD_PUNCHLIST.md`
- `docs/commands/rhino8.json`: reference parity, not current engine registry
- `docs/components/grasshopper1-kangaroo2.json`: component reference inventory
- `tools/verify-worldwright-kernel.sh`: kernel integration validation
