# BuilderCraft suite instructions

## Worldwright toolchain and environment policy

- The native product is **Worldwright**. Legacy `BuilderCraft` and
  `CADCraft` names remain in repository/crate paths until migration.
- `rust-toolchain.toml` pins **Rust 1.95.0**; use the checked-in
  `.devcontainer/` setup on compatible developer systems, or install rustup
  and the pinned toolchain locally. In a supported workspace run
  `bash tools/check-worldwright-rust.sh` before validating Rust code.
- Run `bash tools/verify-worldwright-kernel.sh` (or the Windows PowerShell
  equivalent) before claiming build success. **Do not label tests passed**
  unless they actually ran with Rust/Cargo and their results were observed.
- **Automatic native CI is authorized for this public repository.** Run Rust
  formatting, linting, unit/integration tests and native build checks on
  ordinary `push`/`pull_request` events using **standard GitHub-hosted
  runners** (e.g. `ubuntu-24.04`, `windows-latest`, `macos-latest`).
  Standard runners are free on public repositories. When CI fails, inspect
  the actual GitHub Actions run and logs, fix the code and re-run by pushing.
  No claims of passing tests without observed successful job conclusions.
- **Do not use chargeable GitHub larger/GPU runners or Codespaces** without
  explicit approval. Avoid uploads/caches that exceed free artifact/storage
  allowances. Keep test workflows least-privileged (`contents: read`),
  concurrency-cancelled, and bounded with timeouts. Preserve a manual
  dispatch option. Avoid auto-publishing releases or merging failed PRs.
  This authorization applies to public BuilderCraft only; re-evaluate
  costs and permissions if the repository becomes private.
- ChatGPT conversation containers may be ephemeral and need not provide
  Docker/Rust. Repository setup is reproducible, not a guarantee that an
  unrelated chat runtime is provisioned.
- See `docs/architecture/DEVELOPMENT_ENVIRONMENT.md`.

## Metrology and reusable geometry dependencies

- Before authoring CAD, Scan, OrbWeaver, inspection, rockwork or fabrication
  operations, consult `docs/dependencies/metrology-reuse.json`. Reuse already
  registered kernel geometry/maths services; planned primitives are **not**
  implemented merely because they appear in this planning registry.
- Put ICP, best-fit, deviation, spatial indexes, thickness, mesh repair and
  scan-to-NURBS mathematical work in shared Rust crates, never in separately
  duplicated app engines. Scan and CAD are independent interface modules.
- Preserve units/frames, source geometry IDs, fit residuals, tolerance policy,
  immutable revision-keyed caches, bounded memory and cancellation.
- Update cross-app dependency metadata alongside implementation/tests. Check
  `python3 tools/check_metrology_reuse.py` and native Rust CI before declaring
  operation parity. GOM/PolyWorks/Design X are behavioral references only.

## Dependency-first native tool policy (Worldwright + OrbWeaver)

The [dependency DAG and pair register](docs/dependencies/README.md) govern
the sequence for **both** native CAD commands and OrbWeaver nodes. Prefer
lower-tier primitives, then add modifiers and document/graph adapters as
thin wrappers. One geometry algorithm must serve both interfaces; never
reimplement the same operation in the OrbWeaver node evaluator.
All 1,072 Rhino commands, 817 Grasshopper components, 110 Kangaroo components
and 2,357 manual topics have preliminary category coverage, **not** verified
per-item dependencies. Review unresolved entries and exact port/tree behavior
before claiming parity. For every new pair update the Rust
`SHARED_TOOLS` contracts and `docs/dependencies/tool-groups.json`,
add CAD and OrbWeaver tests, run the local inventory/pair consistency scripts,
and leave entries unvalidated until compilation/conformance tests pass.
Run free public-repository standard-runner Actions automatically; never initiate paid runners without explicit approval.

OrbWeaver now has native `DataTree<T>` structure operations in the **shared
kernel**: `kernel.tree.validate/flatten/graft/simplify/match`. CAD
`worldwright.tree.*` commands and `orbweaver.tree.*` nodes must call that
same kernel contract. Tree branches use lexicographically ordered unique paths;
matching is explicit and strict by identical path, not automatic Grasshopper
path matching. Keep modifier policies named and typed (shortest, longest,
cross-reference), and preserve empty branches. The next dependency is **immutable geometry handles** and document-scoped
reference resolution. Tree-aware numeric broadcasting is now authored and
must remain a thin shared-kernel adapter, not a second CAD/graph algorithm.
not another copy of tree algorithms in the graph crate. Rust code remains
pending compiled validation.


Native BuilderCraft must be entirely free and open source. Use original implementations or dependencies whose relevant source and redistribution licenses have been verified. Rhino/Grasshopper/Kangaroo are public-behavior references only: never copy proprietary implementation code or require a paid host for native capabilities. Optional third-party adapters must not replace native functionality or become a required runtime dependency.

BuilderCraft's accepted scope is CAD for themed entertainment professionals, implemented as independently runnable CAD, Scan, Graph and Show apps with shared core services and optional bridges. Read `docs/architecture/SUITE.md`, `docs/roadmap/SUITE_ROADMAP.md`, `docs/architecture/MEMORY_AND_JOBS.md` and `docs/commands/README.md` first. These BuilderCraft product priorities supersede inherited CADCraft parity percentages, app naming and AutoCAD-only command/UI priorities below; inherited engineering/attribution/never-crash rules still apply.

Preserve the independent native core: Rhino-style means functional/UI reference, not a mandatory licensed Rhino dependency. External Rhino/Autodesk/metrology/console adapters are optional and capability-labeled. All apps must run alone. Share geometry/identity/units/revisions/transactions and do not create duplicate modeling or patch engines. Graph nodes use the same command services as direct modeling and APIs. StructureGraph source reuse must follow the reviewed register before extraction; do not treat planned contracts as working tools.

Track documented Rhino command coverage in `docs/commands/rhino8.json`. Never mark a name match as working parity. Implementation, options and acceptance evidence must accompany status changes. Prioritize an early massing-to-engine walkthrough without waiting for advanced VFX. Patch, cue programming and live output have separate validation gates. Rust memory safety does not waive aggregate RAM, undo, cache, IPC, cancellation or GPU teardown budgets.

The inherited local `plan/` and external `craftrules` references may be absent in this fork. Record missing references and proceed with checked-in suite architecture and engineering rules; do not invent their content. No automatic cloud upload of design geometry. Update suite status/roadmap alongside actual feature changes.

## Inherited CADCraft engineering instructions

# CADCraft — instructions for agents

CADCraft is a clean-room, open-source, pure-Rust computer-aided design and drafting application targeting Autodesk AutoCAD parity — and superiority (speed, openness, agent control). It runs natively on macOS, Windows, Linux and FreeBSD, and on the web via WASM. Siblings with the same conventions: `../photocraft` (Photoshop-class), `../vectorcraft` (Illustrator), `../filmcraft` (Premiere), `../lightcraft` (Lightroom), `../pdfcraft` (Acrobat), `../effectcraft` (After Effects), `../designcraft` (InDesign).

Standards and learnings shared across the crafting apps live in `../../craftrules` (or `storytold/craftrules`). Read its `AGENTS.md` at the start of a session, follow its standards, and contribute reusable learnings back there. Never code: repos don't share code.

## Start every session here
1. Read `plan/STATUS.md` (current milestone, next task), then the task in `plan/execution-plan.md` and the relevant `plan/architecture.md` section. Behaviour reference: `plan/autocad/*` (`01-observed-ui.md` holds measured observations of the running reference app; `04-menu-tree.txt` the menu tree by name).
2. Library decisions: `plan/adr/0001-geometry-libraries.md` (what we may depend on, what is rejected for licence reasons).
3. Follow the autonomous operation protocol (`plan/execution-plan.md` §6). Don't stop to ask unless it lists the decision as the owner's.

`plan/` is gitignored (local only).

## Never crash
People trust CADCraft with their drawings; a crash loses their work. **This outranks feature work.** Standard: [`craftrules/standards/never-crash.md`](https://github.com/storytold/craftrules/blob/main/standards/never-crash.md).
- **No panics in non-test code:** no `unwrap()`, `expect()`, `panic!`, `unreachable!`, `todo!`, `unimplemented!`; no `unsafe` (`unsafe_code = "forbid"`).
- **Errors are `Result<T, E>`** through the crate's error type and `?`. An unfinished feature returns an error or reports "not available yet"; it never panics.
- **Input-derived numbers are hostile** (DXF/DWG files, command-line text, MCP/control params): `get()` not `[i]`, checked arithmetic, no NaN casts, cap input-sized allocations and loop counts.
- **Bound recursion** (nested/cyclic block references: `MAX_BLOCK_DEPTH`).
- **Last-resort guard:** `Session::execute` and interactive command input run under `catch_unwind`; an escaped panic restores the drawing and reports an error.
- **Prove it:** every crash fix lands with a regression test (see `hostile_params_never_panic`, `hostile_dxf_does_not_panic`).
- Every production crate root carries `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]`.

## Non-negotiables
- **Clean-room.** AutoCAD is installed on the dev machine and may be *observed* black-box: run it, use its UI with synthetic drawings, take screenshots (by window id) stored only under `plan/autocad/screenshots/` (never committed). **Never** read, disassemble or copy anything inside the AutoCAD bundle (names/listings only); never copy Autodesk fonts (SHX), hatch patterns (`.pat`), linetypes (`.lin`), templates, CUI/PGP files, icons, artwork or help text; never commit files produced by AutoCAD. File formats come from public specifications (Autodesk's published DXF Reference, the Open Design Alliance's public DWG specification) and our own synthetic tests. Never copy or link GPL/LGPL/AGPL code (LibreDWG, LibreCAD, QCAD, FreeCAD's planegcs, SolveSpace, OpenCASCADE) — don't even read their sources.
- **Assets — absolutely essential.** CADCraft contains **no Autodesk, Adobe or Avid iconography, images, fonts, patterns or artwork — ever.** Every icon is drawn in code (`crates/ui-egui/src/icons.rs`), every hatch pattern and linetype is our own definition (`crates/doc/src/library.rs`), the drafting font is our own (`crates/fonts/src/stroke.rs`). Any file asset must be original, public domain/CC0, OSI-licensed, or redistributable Creative Commons (or licensed open source by a contributor who made it), and **must have a row in `ATTRIBUTION.md`** with author, source and licence (`cargo xtask assets` enforces it). The only exception is the ArtCraft brand in `docs/brand/` (trademarks, `docs/brand/LICENSE-brand.txt`). Screenshots of Autodesk software are never committed. Breaking this rule is the most serious mistake you can make in this repo.
- **Fonts live in [`storytold/craft-fonts`](https://github.com/storytold/craft-fonts)**, never in this repo. It is an optional build input (`CRAFT_FONTS_DIR`), never a `Cargo.toml` dependency. Code and tests must work without it.
- **Shared test corpora** live in separate repos (`storytold/<app>-corpus`); never commit large binary fixtures here.
- **Everything is a command.** User-visible behaviour = a command in `crates/engine/src/cmd/*` (`CommandSpec`: id = AutoCAD's command name in lower case, label, menu path, shortcut, aliases, params doc, `enabled`, JSON `run`, optional `interactive` prompt machine) + tests. UI-only commands live in `crates/ui-egui/src/menus.rs` (`UI_COMMANDS`). The command line, menus, toolbar, Tool Sets, scripts, CLI, control channel and MCP all reach the same commands.
- **Programmatic calls never open dialogs.** `engine.execute` runs the JSON form with defaults; only menu/toolbar invocation starts the interactive prompt sequence.
- **Layering** is enforced by `cargo xtask layers`: L0 `geom`, `dxf` → L1 `color`, `doc` → L2 `fonts`, `render`, `constraints` → L3 `io` → L4 `engine` → L5 `ui-egui`, `mcp` → apps. Nothing below L5 depends on egui/eframe/winit/wgpu/rfd. **The UI crate is swappable.**
- **The UI is thin**: panels read engine state and act through `app.run(id, params)` / `app.start(id)` / `app.cmdline(text)`. Colours come from `theme::Tokens`.
- **Rust only** (no handwritten JS/TS). **Never break wasm** (`cargo xtask wasm`).
- **Quality gates** before every commit: `cargo xtask ci` (fmt, clippy -D warnings, tests, assets, layers, wasm). Commit after every feature arc that builds, and push to `main`.

## Running and looking at the app
- `cargo run --release -p cadcraft -- --sample --control PORT` (sample drawing + control channel). Pick a free port; other crafting apps use control ports too.
- Drive it with JSON lines on `127.0.0.1:PORT`:
  - `{"id":1,"method":"cmdline.input","params":{"text":"circle 0,0 5"}}` — type at the command line exactly like a user (prompts, keywords, `@dx,dy`, `@d<a`, direct distance entry).
  - `{"id":2,"method":"engine.execute","params":{"command":"line","params":{"points":[[0,0],[10,0]]}}}` — JSON form.
  - `{"id":3,"method":"ui.screenshot","params":{"path":"/tmp/shot.png"}}` then read the PNG.
  - Methods: `crates/ui-egui/src/control.rs`; protocol doc: `docs/control-protocol.md`.
- **For UI work, look at the result** (screenshot, read the PNG) and compare with `plan/autocad/screenshots/`. If no frame is presented (screen locked) use `ui.render` or `cadcraft-cli`.
- Headless: `cadcraft-cli run --sample --script 'LINE 0,0 10,10\n' --export out.png`; `cadcraft-cli info file.dxf`; `cadcraft-cli mcp`.
- Shell gotcha: `mv`/`cp` may be aliased interactive — use `/bin/mv -f` / `/bin/cp -f`.
- Parallel agents: separate `CARGO_TARGET_DIR` per agent; edit only the crates you own; delete your target dir when done (disk).

## Roadmap
`ROADMAP.md` (committed) tracks status, milestones, parity and estimates. Update it whenever a milestone task lands. `cargo xtask parity` recomputes the command-catalog parity in `docs/parity.md`.

Original native host adapter code may use the host's required language (Unreal C++/UBT C#) under `bridges/`; these adapters are optional, separately validated and must not copy engine implementation code. Core suite services and applications remain Rust.

## Optional feature-history modeling

Read `docs/architecture/FEATURE_HISTORY.md` and
`docs/dependencies/feature-history.json` before implementing mechanical tools.
Timelines are optional per document, model node or block definition. Preserve
direct modeling. Stable step IDs, local typed parameters, chronological
references, reversible suppression/rollback and `.dftba` recipe persistence
are source-authored, not yet compiled. The 52 planned sketch/solid/assembly
features are NOT implemented. Reuse the shared Rust geometry operations for
both CAD feature commands and OrbWeaver nodes. Never duplicate math engines.
