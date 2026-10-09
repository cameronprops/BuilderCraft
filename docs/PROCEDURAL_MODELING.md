# Orb Weaver — Worldwright's open-source parametric graph

**Orb Weaver** is Worldwright's native Grasshopper-style parametric graph
environment, named for orb-weaving spiders. The current headless Rust library
lives in `crates/orb-weaver` (package `orb-weaver`, code import
`orb_weaver`, node IDs `orbweaver.*`). Its graphic canvas inside Worldwright
and its standalone GUI executable are future presentation layers.

## Shared native functionality (Rust build pending)

- Versioned, serializable typed DAGs with literal and linked ports,
  deterministic evaluation, cycle/type checks and bounded resource usage.
- **15 paired native CAD/Orb Weaver operations** using one shared kernel
  dispatcher: ten point/vector/polyline functions plus five data-tree tools.
- Native data trees with ordered branch paths `{0;2;1}`, preserved empty
  branches, explicit validation and 250,000-item / 4,096-branch bounds.
- Shared tree **Validate, Flatten, Graft, Simplify, Match** operations.
  Matching supports **shortest**, **longest** (repeat last value), and
  **cross-reference** (Cartesian product) via an explicit modifier.
- Orb Weaver nodes can link the output tree of one node to another. Data-tree
  operations are identical through Worldwright CAD/API command IDs.
- Local scripts compare 46 kernel-operation DAG entries, 15 paired contracts,
  and 4,356 catalogued source references without invoking GitHub Actions.

## Explicit current limitations

- Strict **identical-branch-path** matching only. Implicit Grasshopper path
  expansion, hierarchical alignment, graft/flatten metadata, and tree-item
  broadcasting into every numeric CAD operation are not yet implemented.
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
`cargo run -p orb-weaver --example paired_tree` demonstrates a headless
CAD/graph tree equivalence check when a local Rust environment is available.
A Rhino/Grasshopper license is never required for Orb Weaver.
