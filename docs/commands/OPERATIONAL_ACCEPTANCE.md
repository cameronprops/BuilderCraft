# Worldwright command operational acceptance gate

Engine registry: **396** registered commands at the 2026-10-19 acceptance snapshot.
Source of truth: `crates/engine/src/cmd/mod.rs` via
`cadcraft_engine::command_specs()`. Do not treat reference command lists,
API names, menu visibility or registration as functionality.

## Five-gate command certification begins now

The positive-case registry covers **166 / 396** live engine command IDs on
merged `main`. It does **not** certify them. The explicit certification ledger
is [`certification-registry.json`](certification-registry.json). It currently
records **0 fully certified commands** and three candidates in PR #126:
`worldwright.point.distance`, `worldwright.point.midpoint`, and
`worldwright.point.interpolate`.

Each candidate is executable in
`crates/engine/tests/command_certification.rs`, with five distinct gates:
analytically correct outputs; atomic input failure; unchanged document plus
undo/redo and `.dftba` roundtrip for read-only queries; CAD/API/OrbWeaver
parity; and bounded deterministic input stress. This establishes a
**headless point-operation scope only**, not GUI or full Rhino parity.

```sh
cargo test --locked -p cadcraft-engine --test command_certification
```

The three entries remain `candidate_ci_pending` until pinned Rust checks,
all required platform CI, post-merge `main` checks, and applicability review
are green. Do not silently promote candidates based on the existence of a
test, or imply that read-only output persists in a drawing. Stateful commands
must add meaningful undo, redo, save/reopen and UI tests before certification.

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

**Current main:** 166 / 396 independently asserted success-path
fixtures after merged PR #122. PR #123 merged the command index and CI
optimizations. The consolidated #125 proposes nine additional geometry
fixtures from #124 and #125, reaching **175 / 396** when merged. These
numbers are not certification counts. PR #126 merged the initial point
certification harness; stateful Line/Move/Copy certification is proposed in
#127. Full five-gate certification is tracked separately above.

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
