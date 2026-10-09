# Orb Weaver — Worldwright's open-source parametric graph

**Orb Weaver** is the embedded and independently usable parametric graph engine
for Worldwright. Its name comes from orb-weaving spiders. The core is
host-independent; a visual graph canvas embedded in the CAD workspace and a
standalone Graph app are future presentation layers over the same evaluator.

## Native source implemented (pending local compilation)

- `crates/orb-weaver` — versioned, serializable directed graphs with typed
  literal or linked ports, deterministic dependency-ordered evaluation, cycle
  detection, bounded graphs and typed error propagation.
- `crates/kernel/src/shared_tools.rs` — 10 typed contracts and a shared
  dispatcher to the **same kernel operations** used by Worldwright CAD tools.
- `crates/engine/src/cmd/worldwright_tools.rs` — direct CAD/API command IDs
  for those 10 operations and generic typed execution/discovery.
- Modifier ports `t`, `count` and `spacing` parametrize existing
  algorithms instead of spawning duplicate geometry implementations.
- `docs/dependencies/` — group-level dependency DAG, full reference
  coverage index, classifications and the shared operation pair plan.

This **does not** implement Grasshopper tree matching, list access, graft,
flatten, previews/bake, parameter UI, graph canvas, solver, persistent
world-document graph links, or exact reference component parity. The 817
Grasshopper and 110 Kangaroo reference entries retain their prior status
until native test and reference-conformance gates pass.

Read [the dependency mapping](dependencies/README.md), the
[component inventory](components/README.md) and
[the delivery roadmap](roadmap/SUITE_ROADMAP.md). A native Orb Weaver graph
must not require a licensed Rhino or Grasshopper host. The planned CAD
canvas must call this same graph engine rather than duplicate it.
