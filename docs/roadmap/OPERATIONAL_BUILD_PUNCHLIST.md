# Worldwright operational alpha: PR resolution and build punch list

**Snapshot:** 2026-10-17 Pacific / 2026-10-18 UTC. **Repository:** `cameronprops/BuilderCraft`. **Target:** a repeatably buildable, installable, recoverable CAD desktop, not a claim of full Rhino/Fusion parity.

This register explicitly audits **all 39 PRs open at the start of the stabilization sprint**. Five are merged in the first pass (#25, #27, #34, #53, #63); the rest are unresolved at this snapshot. Re-query GitHub before making state claims. A prior green head check is **not** proof that an updated `main` passes. Preserve original branch work until its functionality has landed or has an explained disposition.

## Live acceptance stack (2026-10-19)

This section supersedes historical PR statuses below. The original stabilization
snapshot is retained for traceability and must not be treated as live GitHub state.

| PR | Dependency | Authored success fixtures | Merge gate |
|---|---|---:|---|
| #118 | `main` | 144 / 396 | **MERGED** `8c7c12ac`, required head CI passed |
| #119 | `main` | 149 / 396 | **MERGED** `eb3d3c9`; required native/modeling/exact OCCT CI passed |
| #120 | #119 | 154 / 396 | Covered by consolidated #122; close only after verified merge |
| #121 | #120 | 159 / 396 | Covered by consolidated #122; close only after verified merge |
| #122 | `main` (consolidated #120-#122) | 166 / 396 | 17 new success fixtures; full Linux/macOS/Windows native, focused and exact OCCT CI required |

Reconciliation proof: the #122 fixture table contains all 149 IDs now on
`main` plus 17 distinct new command IDs from #120-#122. Merge #122 only
after fresh native, modeling, and exact BRep checks on the `main`-based
commit; then verify post-merge `main` before closing #120/#121.

**230 / 396** registered commands currently have no independently asserted
positive-path fixture in this proposed stack. New fixtures check real results
and do not constitute full operational certification. Do not merge descendants
before parents, do not bypass failing CI, and verify `main` after every merge.

## Command-certification pipeline (2026-10-19)

**Certification starts with PR #126, independent of fixture-count expansion.**
On merged `main`, 166 / 396 registered commands have a positive-path
acceptance fixture. **0 are yet proven to satisfy all five certification
gates across the required CI platforms.** Three read-only point operations from merged #126 and three stateful 2D
operations from proposed #127 are certification candidates, not certified
releases. The latter must still validate interactive command flows in a
real application before promotion.

- Automated certification suite:
  `cargo test --locked -p cadcraft-engine --test command_certification`
- Machine-readable promotion ledger:
  `docs/commands/certification-registry.json`
- Scope: analytic output, transactional failure, `.dftba` and undo/redo
  preservation for read-only queries, CAD/OrbWeaver API parity, and bounded
  deterministic input repetition
- Promotion only on passing pinned cross-platform PR and post-merge `main`
  checks, plus explicit interface applicability review
- Stateful geometry commands require a second certification tranche with
  actual undo/redo and saved/reopened geometry, not read-only exemptions

**Stateful certification PR #127:** Line, Move and Copy are tested by
`crates/engine/tests/stateful_command_certification.rs`, with exact geometry
and handles, failure-state preservation, native `.dftba` roundtrips,
undo/redo, command parameters/aliases, and bounded repetition. This is
an engineering certification **candidate**, not a desktop interaction
certification or a claim that all five gates passed across platforms.

**Next certification tranche #128:** Rotate, Scale and Mirror extend #127's
stateful Rust test suite and machine-readable ledger to **nine candidates**
(three point-query operations, six stateful 2D modeling commands).
Formal certification stays **zero** until cross-platform integration,
post-merge main and applicable interactive UI evidence is complete.
The workflow is incremental and must not duplicate transform math.

## Definition of "operational alpha" (release-blocking)

- [ ] **Single integration target:** update `main` or a dated stabilization branch to a coherent, non-conflicting head; no hidden dependency on unmerged stacked PRs; record precise release SHA and source provenance.
- [ ] **Compiler and lint:** pinned Rust 1.95 (or declared successor), `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`; run the repository's `tools/verify-worldwright-kernel.sh` and relevant native OCCT acceptance workflows. Test failures cannot be waived without a tracked rationale.
- [ ] **Desktop launch:** `cargo run -p cadcraft -- --sample` works; launcher opens 2D/3D, WorldBuilder/SolidCraft selectors, model browser, viewport and command entry, with no startup panic. Test a **macOS Intel x86_64** target as well as Windows and Linux x64; document build prerequisites and any missing OS test runners.
- [ ] **Representative CAD file:** in one document create/edit a rational curve and surface, polygon mesh and exact OCCT BRep; select/transform, display, hide/restore; perform an exact Boolean and a geometric failure case without corrupting source.
- [ ] **Persistence integrity:** Undo/redo, native `.dftba` save/reopen, units and tolerance preservation, stable object/layer/body IDs, missing external dependency detection, older file compatibility and atomic failure cases.
- [ ] **Operational CAD interaction:** working geometry picking and shaded display for multiple solids; functional basic commands, explicit exact-vs-approximate labeling, cancellable/validated long operations; selection does not act on stale revisions.
- [ ] **Parametric/direct coexistence:** switch WorldBuilder / SolidCraft without cloning models; source-driven edits identify invalid/stale dependent solids; no silent loss of feature history; the constrained-sketch-to-exact-extrude feature is separately tracked and **not yet implemented**.
- [ ] **Mesh basics:** topology inspection, repair preview/commit, nonplanar fill tests, QEM simplification preview + undoable bake on a genuine sample; reject defective output and honor copy/replace rules.
- [ ] **Performance:** startup and 3D interaction stress fixture with bounded memory, scene budgets and cancellation; separate geometry cache from preview cache. Benchmark large scenic mesh + exact ride-vehicle BRep + long ride path in distinct body scopes.
- [ ] **Release and recovery:** a native zipped/bundled executable with version, sample document, reproducible build steps and smoke-test record; independently verify a dated Google Drive checkpoint **by comparing checksum and restoring a file**. A GitHub commit does not imply Drive sync.
- [ ] **Credible scope:** publish alpha known limitations (no full Rhino parity, no ride clearance/safety certification, no guaranteed analytic NURBS sweep, no general parametric feature regeneration).

## Decision policy

**Merge:** target branch mergeable, prerequisites on `main`, license gate clear, relevant CI green, no unreconciled duplicate implementation, and post-merge main smoke validated. **Repair/rebase:** relevant but conflicting or failing PR. **Consolidate:** choose one authoritative successor, compare patches and tests, migrate missing coverage, then close duplicates with linked reason. **Defer:** valuable optional feature, not a blocker to core operational CAD. **Close as superseded:** only after confirming source/test coverage or accepted research conclusion. **Never** merge an experimental competing BRep kernel into production merely to empty the queue.

### Every initially-open PR: disposition and next action

| PR | Workstream | Current stabilization disposition | Next concrete action |
|---|---|---|---|
| [#8](../../pull/8) | Haiku/platform policy | DEFER / old conflicts | Retain Windows/macOS/Linux gates; isolate Haiku CLI preview and parity tests into separate optional milestone. |
| [#14](../../pull/14) | Curvo NURBS experiment | REVIEW / conflicts | Compare Curvo parity evidence to merged #65 OCCT/Truck decision; avoid second canonical runtime; archive experiment if redundant. |
| [#15](../../pull/15) | LAS reader | DEFER / conflicts | Rebase optional read-only scan importer later; no reason to block CAD desktop launch. |
| [#16](../../pull/16) | Kurbo 2D experiment | DEFER / conflicts | Preserve feature-gated benchmark and licensing evidence; only adopt after measured advantage. |
| [#17](../../pull/17) | 2D/3D workspace UI | RECONCILE / conflicts | Compare with merged #64 and preserve missing keyboard/menu tests, then close redundant implementation. |
| [#20](../../pull/20) | Command palette | REBASE / conflicts, depends on #17 | Extract and test Cmd/Ctrl+K UX on current shared workspace shell without restoring duplicate mode implementation. |
| [#21](../../pull/21) | Transactional graph editing | REVALIDATE | CI was green on prior head; check integration with new main, then merge before #22. |
| [#22](../../pull/22) | OrbWeaver graphical node canvas | BLOCKED on #21 / no retrieved CI | Install prerequisite, run native cross-platform GUI tests and correct egui dependency compatibility. |
| [#25](../../pull/25) | Oriented arrays | **MERGED** | Post-merge shared CAD/OrbWeaver command smoke. |
| [#27](../../pull/27) | Rail frames + mesh stage | **MERGED** | Post-merge rail-frame and Pipe/Sweep acceptance; distinguish mesh-stage from exact BRep output. |
| [#28](../../pull/28) | OpenSCAD CLI bridge | DEFER / conflicts | Opt-in executable security acceptance, sandbox limitations and importer tests; optional after core alpha. |
| [#30](../../pull/30) | Truck boolean crash proof | **CLOSED: experimental, deliberately not merged** | Record failure and merged #65 backend choice; never import crashing experimental production topology; archive reproducible tests. |
| [#31](../../pull/31) | Isolated OCCT proof | **CLOSED: research superseded by #42/#65** | Verify equivalent tests in merged #42 native worker; preserve provenance, close superseded experiment if covered. |
| [#34](../../pull/34) | Analytic NURBS derivatives | **MERGED** | Post-merge analytic normal/derivative numerical regression. |
| [#39](../../pull/39) | Topology defect overlay | REBASE / conflicts | Target current viewport with revision/visibility-scoped highlight tests. |
| [#40](../../pull/40) | Quad-strip subdivision | REBASE / conflicts | Check quad winding, stable ID maps, neighbor propagation and undo integration. |
| [#41](../../pull/41) | Mesh Repair workbench | REPAIR / CI failure & conflicts | Rebase on native current UI, correct compile failure, test selection/navigation mode. |
| [#43](../../pull/43) | Hardware profiler | REPAIR / CI failure & conflicts | Extract bounded profile/budget logic; benchmark CPU/GPU routing and check GUI startup. |
| [#44](../../pull/44) | Hole selection/preview | REPAIR / CI failure & conflicts | Rebase on one canonical hole-fill kernel, rerun revision-safe preview/commit tests. |
| [#45](../../pull/45) | OrcaSlicer manufacturing research | DEFER / conflicts | Port license/provenance and optional file-handoff decision to design docs, not slicer binaries. |
| [#46](../../pull/46) | Concave hole fills | RECONCILE / conflicts | Compare ear clipping/validation with #58; port missing adversarial geometry tests. |
| [#49](../../pull/49) | Nonplanar hole modes | RECONCILE / failed CI, conflicts | Compare modes against #58; choose one implementation and port missing best-fit benchmarks. |
| [#50](../../pull/50) | Normal/directional fill | RECONCILE / no retrieved CI, conflicts | Compare with #49/#58; prevent duplicate HoleFillMode enums and hole kernels. |
| [#53](../../pull/53) | Undoable QEM bake | **MERGED** | Post-merge preview parity, copy/replace and undo regression. |
| [#55](../../pull/55) | Simplify Mesh UI v1 | DUPLICATE REVIEW / conflicts | Compare to #56, consolidate best preview controls/tests into one UI PR. |
| [#56](../../pull/56) | Simplify Mesh UI v2 | REPAIR / CI failure & conflicts | Fix failing UI compile/tests; preserve x-ray preview and revision guard; supersede #55 only if coverage retained. |
| [#58](../../pull/58) | Curvature-guided repair | REBASE / formerly green, now conflicts | Target current QEM/BRep main; retain stable rim, safe triangulation, fallback and undo tests. |
| [#59](../../pull/59) | Shaded hole preview | BLOCKED on #58 / CI failure | Merge repaired hole kernel first, then repair overlay and validate stale-cache tests. |
| [#60](../../pull/60) | Multi-solid shaded picking | REBASE / CI previously green, conflicts | High priority: multi-body full-scene pick/render without mistaking display triangles for BReps. |
| [#62](../../pull/62) | FreeCAD LGPL dependency walker | LEGAL/TECH REVIEW | Verify LGPL license/relink obligations, source manifest and measured benefit; not a CAD alpha blocker. |
| [#63](../../pull/63) | Exact BRep move/rotate/scale | **MERGED** | Post-merge OCCT STEP, transform and Inspector smoke. |
| [#66](../../pull/66) | Rational ruled surface | REVALIDATE / merge conflicts fluctuating | Reconcile shared NURBS files with merged #34 and current main; cross-platform CI before merge. |
| [#67](../../pull/67) | Station/tangent/offset | REPAIRED test + retriggered CI; merge pending | Read native/acceptance logs, fix tests, align with merged #27; do not merge into broken main. |
| [#68](../../pull/68) | Nine Rhino-style commands | BLOCKED on #67 | Rebase onto fixed #67, preserve Partial accuracy labels, verify CAD/OrbWeaver operation parity. |
| [#69](../../pull/69) | Exact OCCT sweep + NURBS | BLOCKED on #68 | Validate native worker Pipe/Sweep BReps and rational curve operations, avoid duplicate kernel. |
| [#70](../../pull/70) | Undoable NURBS object edits | BLOCKED on #69 | Check stable document ID, undo/save, source-relative approximated sweep rail qualification. |
| [#71](../../pull/71) | Certified adaptive rail tessellation | BLOCKED on #70, previously green | Preserve one-sided chord certification and explicit limitations; full native CI after rebase. |
| [#72](../../pull/72) | Fillet/Pipe/Sweep feasibility probes | **REPAIRED** lint and test scope; CI pending | Fix Clippy needless borrow at `crates/engine/src/cmd/brep3d.rs` lines ~421/426 and `hostile_constraint_params_never_panic` test; rerun all three workflows. |
| [#73](../../pull/73) | Associative sweep recipes | BLOCKED on #72 and preceding stack | Validate source snapshot, stale tracking, stable-ID opt-in rebuild and failure rollback; no auto rebuild during editing. |

## Critical-path order

1. **Merge baseline integration:** post-merge compile/test from main and correct any breakage introduced by #25/#27/#34/#53/#63. No new release label until green.
2. **Viewport + usable commands:** reconcile #60, #66, #21, and one of #55/#56 with current source; keep UI usable instead of merely headless.
3. **Rhino command stack:** repair #67, then verify/rebase in order #68 -> #69 -> #70 -> #71 -> #72 -> #73. If another PR changes the same shared command files, rerun whole stack CI.
4. **Scenic mesh repair:** consolidate #46/#49/#50 into #58 as appropriate, then #59 and #44; reconcile #39/#40/#41.
5. **Sketch-to-exact-solid first parametric feature:** add a separate tracked task for validated constrained 2D sketch -> OCCT extrusion -> scoped feature revision/recompute/rollback, OrbWeaver parity. Do not claim full SolidCraft until it runs.
6. **Release:** freeze commands/serialization, test representative ride-vehicle / scenic mesh / ride-path document, package alpha, validate fresh-install, then checksum and restore Google Drive backup.

### Triage notes (observed at kickoff)

- There were **39** open PRs when this register was created. Merges completed in this sprint: [#34](../../pull/34), [#53](../../pull/53), [#63](../../pull/63), [#25](../../pull/25), [#27](../../pull/27).
- Pull requests that appear mergeable can temporarily report `mergeable:false` while GitHub recomputes the merge test; re-check before deciding a genuine conflict.
- PR #72 previously showed three failed workflows; errors include strict Clippy in `brep3d.rs` and one constraints regression in the engine.
- CI for #71 was green on its own PR head; that does not validate current main plus its whole stack.
- GitHub is source-of-truth. No demonstrated automatic current backup to Google Drive. Checkpoint must include verified commit SHA, archive digest, destination and restore exercise.

## Sprint progress log

- 2026-10-17 Pacific: Audited 39 initially open PRs, including native CI and dependency descriptions.
- 2026-10-17 Pacific: Merged #34 (analytic derivatives), #53 (undoable mesh bake), #63 (exact-solid transforms), #25 (oriented arrays), and #27 (rail frames). **Combined native post-merge build is not yet verified.**

Update this document as each PR reaches *merged*, *closed with rationale*, *repaired*, or *deferred with owner and gate*. The goal is a reproducible working product, not zero open PRs.

- 2026-10-17 Pacific (continued): Closed #30 (known Truck Boolean crash proof) and #31 (isolated OCCT evaluation) after documenting why these candidate implementations should not be merged alongside the selected OCCT kernel. Branch evidence retained.
- 2026-10-17 Pacific (continued): Fixed `main` compilation regression from #34 on top of #65: the derivative routine still called removed global `basis()`; it now expands the existing span-local `local_basis()` into the coefficient vector. Commit `205feed2`. Native combined-build CI still requires readback.
- 2026-10-17 Pacific (continued): PR #67 removes hardcoded registry count and now tests native path operation registry parity; PR #72 fixes two strict-Clippy borrows and avoids routing exact BRep constraint commands through the legacy 2D interactive fuzz test. Their validations are not yet accepted as green.

## 2026-10-17 PR-zero resolution checkpoint

- **Open PR count: 0** (verified with GitHub after resolution). This is backlog hygiene, **not** completion of the operational alpha build.
- Additional merged PRs in this closure sequence: #8 Haiku/platform foundation, #21 OrbWeaver transactional editor, #66 rational ruled surface. They require post-merge `main` CI and application smoke validation.
- Other previously open PRs were closed **without merging**, with source head SHAs preserved in [PR reconciliation register](PR_RECONCILIATION_2026-10-17.md). Integration tasks, model-space acceptance and tests remain in [GitHub issue #75](https://github.com/cameronprops/BuilderCraft/issues/75).
- Work still required: native build (Rust 1.95 workspace and exact OpenCascade worker), UI and viewport integration, complete command stack, graph UI, mesh repair, file save/restore and alpha packaging. These checks are intentionally left incomplete until independently verified.

