# OrbWeaver — Worldwright's open-source parametric graph

**OrbWeaver** is Worldwright's native Grasshopper-style parametric graph
environment, named for orb-weaving spiders. The current headless Rust library
lives in `crates/orbweaver` (package `orbweaver`, code import
`orbweaver`, node IDs `orbweaver.*`). Its graphic canvas inside Worldwright
and its standalone GUI executable are future presentation layers.

## Shared native functionality (Rust build pending)

- Versioned, serializable typed DAGs with literal and linked ports,
  deterministic evaluation, cycle/type checks and bounded resource usage.
- **15 paired native CAD/OrbWeaver operations** using one shared kernel
  dispatcher: ten point/vector/polyline functions plus five data-tree tools.
- Native data trees with ordered branch paths `{0;2;1}`, preserved empty
  branches, explicit validation and 250,000-item / 4,096-branch bounds.
- Shared tree **Validate, Flatten, Graft, Simplify, Match** operations.
  Matching supports **shortest**, **longest** (repeat last value), and
  **cross-reference** (Cartesian product) via an explicit modifier.
- OrbWeaver nodes can link the output tree of one node to another. Data-tree
  operations are identical through Worldwright CAD/API command IDs.
- **Tree-aware numerical broadcasting:** all ten shared point/vector/polyline
  operations accept exact leaf-typed trees on ordinary input ports. Scalars
  broadcast across branches; multiple trees must have identical branch paths.
  `matching` is an explicit per-CAD-command/per-graph-node modifier with
  `shortest` (default), `longest` (repeat final item), and
  `cross_reference` (Cartesian) behavior. Branches are not reordered, merged
  or silently expanded; an empty branch stays empty.
- `orbweaver` is the one-word Rust package, source directory and Rust import
  name. Public graph nodes retain the existing `orbweaver.*` namespace.
- Local scripts compare 46 kernel-operation DAG entries, 15 paired contracts,
  and 4,356 catalogued source references without invoking GitHub Actions.

## Explicit current limitations

- Strict **identical-branch-path** matching only. Implicit Grasshopper path
  expansion, hierarchical alignment, graft/flatten metadata, per-branch
  remapping and full native Grasshopper list-access parity are not yet implemented.
  The first ten numeric operations now support tree-aware broadcasting by
  explicit native matching policies.
- Structural tree operations are initial native subsets; they do not establish
  exact Grasshopper component/option parity.
- No visual graph canvas, graph persistence inside `.dftba`, preview/bake,
  geometry reference ports, expressions, Kangaroo-type solver or standalone
  native GUI app.
- These Rust modules and tests were authored but **not yet compiled or run**
  against Cargo; do not mark catalog entries as tested or verified.

See [dependency order and paired operations](dependencies/README.md),
[component inventory](components/README.md),
and [delivery roadmap](roadmap/SUITE_ROADMAP.md).
`cargo run -p orbweaver --example paired_tree` demonstrates shared structural
operations; `cargo run -p orbweaver --example paired_broadcast` demonstrates
multi-branch, longest-list numeric broadcasting in CAD and graph. Both examples
require a locally available Rust toolchain.
A Rhino/Grasshopper license is never required for OrbWeaver.

## OrbWeaver and scoped mechanical histories

OrbWeaver graph nodes and Worldwright feature timelines invoke the same
shared Rust kernel operation dispatcher. The timeline owns parameters and
ordered feature state; OrbWeaver owns data-flow evaluation. Live graph bindings,
geometry-handle references and automatic document bake are future steps.
See [feature-history architecture](architecture/FEATURE_HISTORY.md).
