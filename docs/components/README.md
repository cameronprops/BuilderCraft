# BuilderCraft Grasshopper component inventory

grasshopper1-kangaroo2.json is the machine-readable implementation register.

817 Grasshopper 1 built-in public-index entries; 110 Kangaroo 2 entries (catalog version 2.5.3). Built-in library versions vary between 1.0.0.0 and 1.0.0007 in this reference source; do not interpret those labels as a pinned current Rhino service release.

This inventory covers all selected entries in the retrieved public feed. It is not yet certified complete against a specific installed build. Runtime object GUIDs, input/output contracts, tree matching, platform differences, obsolete/hidden components, and missing public-index entries need a versioned reconciliation. Null contracts mean not researched, never zero ports. No native component is marked working.

For each implementation: verify reference behavior, define typed ports and matching, reuse the shared native CAD service, add original algorithms and UI, test representative/degenerate inputs and cancellation/memory limits, then record paths and fixtures before changing status. A same-name existing CAD command does not establish component parity.

Reference metadata comes from Grasshopper Docs machine-readable feeds. Its repository explicitly permits third-party processing of the live metadata feeds. No reference icons or implementation code are included. The behavioral implementation must remain native, free and open source. Grasshopper 2 and other third-party addons require separate inventories.

Reference descriptions are retained as catalog metadata under the publisher's express feed-processing permission; native implementation code must be original or verified open source.

Known omissions and the required basic typed-parameter review queue are recorded in [GAP_REVIEW.md](GAP_REVIEW.md). The source feed excludes common parameter containers such as Point and Curve. Do not describe the 817 rows as all installed Grasshopper blocks.

## Paired native engine development

The **Orb Weaver** graph engine is now authored in
`crates/orb-weaver`, and the first ten typed, shared CAD/Graph operation
pairs are in `crates/kernel/src/shared_tools.rs` and
`docs/dependencies/tool-groups.json`. Seven of the ten have selected
Grasshopper 1 public-index analogs, but the port lists, matching behavior
and exact GH reference parity remain unverified. The original 817/110
catalog rows are **not** automatically marked working by these wrappers.
A new `docs/dependencies/reference-index.json` indexes all source rows
by provisional dependency groups. Build/update with
`python3 tools/build_dependency_index.py --write`.

