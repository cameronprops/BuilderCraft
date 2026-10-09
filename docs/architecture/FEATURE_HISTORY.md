# Worldwright optional parametric feature histories

## Design intent

Worldwright supports **two modeling styles side-by-side**:

- Direct/Rhino-style modeling: geometry remains freely editable with ordinary
  CAD commands; no history requirement is imposed.
- History-driven mechanical and other parametric workspaces: a document,
  component/body, or reusable block definition can own a local ordered
  feature timeline. Dimensions, parameters and dependent operations rebuild
  through a shared Rust operation dispatcher.

This borrows **public behavior patterns** from SolidWorks, Inventor and
Fusion, without copying any proprietary source or requiring those products.

The first increment creates the **history contract, scoped persistence and
headless editing API**. It does not yet create solids from sketches or display
a complete SolidWorks-style drag-and-drop design-history editor. An initial optional 3D workspace panel can already display steps, reorder, suppress, roll back and edit basic local parameters; full feature creation, dependency diagnostics and preview/bake remain later work.

## Scope and ownership

`FeatureScope` identifies a unique recipe domain:

| Scope | Intended use | Example |
|---|---|---|
| `Document` | Whole-project history, when intentionally enabled | Sequential master model |
| `ModelNode(id)` | Assembly/component/body-local timeline | One bracket inside an assembly |
| `BlockDefinition(name)` | Shared reusable definition, independent of instance placements | A repeatable connector or scenic panel |

**Instances share the definition**. Editing a block-definition history updates
the single source recipe, not one detached copy per insertion. Instance-level
configuration and per-instance feature override semantics are later features
that require explicit versioned ownership and geometry-reference rules.

A drawing can contain any number of direct-modeling objects plus optional
scoped histories. History-driven modeling is entirely opt-in. The initial
implementation limits projects to 256 history scopes and at most 512 steps
per scope, with aggregate limits on step/value counts. Block names are
case-insensitive, including during history-scope deduplication.

## Rules required to remain parametric

1. Feature IDs stay stable when names, parameters or positions change.
2. A step may depend only on **earlier steps in its own scope**. The kernel
   rejects reordering that would create forward references or cycles.
3. Inputs are explicit and typed. They may come from a constant, a named
   local parameter, or a previous feature's output.
4. Parameters are local to each timeline. Editing one block's `width`
   cannot silently change another block's `width`.
5. A feature declares exactly one executable shared-kernel operation. Its
   numeric or geometric algorithm is the same one CAD and OrbWeaver use.
6. Parameter changes recompute downstream steps deterministically.
7. Suppression is reversible; downstream consumers of suppressed outputs are
   marked blocked, never given stale cached values.
8. Rollback is non-destructive: steps beyond the history marker remain saved
   but are not evaluated.
9. Edits require the current timeline revision, validate the *candidate*
   history first, and only then replace the old history. CAD document snapshots
   provide undo and redo; failed edits leave the document unchanged.
10. Native `.dftba` stores the exact history metadata as an optional field.
    Version-1 projects without it, including legacy `.bcraft`, still open.
11. A reused block definition with a feature timeline is protected from
    ordinary unused-block purge while its history exists.
12. A future solid feature cannot use raw triangle/face array indices as
    persistent subelement references. Stable typed geometry IDs, topology
    remap diagnostics, units and tolerances are prerequisites.

## Existing executable source (compilation pending)

The native Rust code lives in:

- `crates/kernel/src/feature_history.rs`: scope IDs, typed step inputs,
  validation and canonical operation contracts.
- `crates/kernel/src/history_execution.rs`: revisioned atomic edit,
  suppression, rollback, topological recompute and blocked-dependency states.
- `crates/doc/src/feature_history.rs`: scope validation against CAD block
  definitions and model hierarchy nodes.
- `crates/engine/src/cmd/feature_history.rs`: programmatic history commands.
- `crates/io/src/project.rs`: optional native history persistence.
- `docs/dependencies/feature-history.json`: **60 dependency entries**, of
  which **8 are source-authored but not compiled** and **52 are planned**.

**No new solid/math kernel engine is implemented by this history work.**
Only already registered shared operations (for example
`kernel.point.midpoint`) may become executable feature steps.

## Headless API example

First, create a document-local timeline:

```json
{"command":"worldwright.history.create","params":{"scope":{"kind":"document"}}}
```

Then append a driven operation:

```json
{
  "command":"worldwright.history.edit",
  "params":{
    "scope":{"kind":"document"},
    "expected_revision":0,
    "change":{
      "edit":"append",
      "step":{
        "id":10,
        "name":"Driven midpoint",
        "operation":"kernel.point.midpoint",
        "inputs":{
          "a":{"source":"constant","value":{"kind":"point","value":{"x":0,"y":0,"z":0}}},
          "b":{"source":"constant","value":{"kind":"point","value":{"x":10,"y":0,"z":0}}}
        }
      }
    }
  }
}
```

Run `worldwright.history.evaluate` with `{"scope":{"kind":"document"}}`;
the output is a typed point at `(5,0,0)`, not automatically baked CAD
geometry. `worldwright.history.list` and `worldwright.history.inspect`
report scopes and recipes. The `edit` command also supports
`set_parameter`, `set_input`, `set_suppressed`, `reorder`, and
`set_rollback`. Block scopes are addressed using
`{"kind":"block_definition","id":"Bracket"}`, and model-node scopes by
`{"kind":"model_node","id":42}`.

## Required work before a visual mechanical modeler

Build in dependency order, following
[`feature-history.json`](../dependencies/feature-history.json):

1. Immutable, versioned geometry handles and stable subelement naming;
   preserve exact 3D geometry and remap edge/face references.
2. Dimensional value types, safe expression parsing and a single sketch
   constraint solver; report under/overconstrained sketches.
3. Planes, sketch primitives, sketch profiles and evaluated 2D regions.
4. Shared BRep solids and feature operations: extrusion, revolution, holes,
   loft/sweep, boolean operations, fillet/chamfer, shell, draft and patterns.
5. An optional mechanical workspace sidebar/timeline with parameter editor,
   branch-local feature selection, drag reorder/suppression, rollback marker,
   failure diagnostics, and history/preview/bake controls.
6. Revision-safe preview and bake transactions in the document, with
   OrbWeaver linking to the same feature parameters and geometry handles.
7. Assembly instances, mates and joints, followed by sheet-metal, weldment
   and specialist tools.

The headless recipe evaluator, persistence and tests are source-authored but
have **not** been compiled or executed in the present environment. Preserve
the draft PR until a Rust 1.95/Cargo validation pass completes. Keep GitHub
Actions manual-only.

### Versioned geometry handles, initial shared layer

The shared scene and CAD document now expose typed, revision-bound object
geometry references. They verify project/object identity, expected revision
and geometry representation. Resolving a scene reference reuses the existing
immutable geometry lease. `ToolValue::GeometryReference` makes these
handles representable as typed OrbWeaver/feature-history data.

The current handle is **whole-object only**. There is no persistent edge or
face identity, feature-face mapping after booleans, document-owned project UUID,
automatic rebind on edit or geometry baking. Do not model a sketch support,
fillet target or BRep face reference using an array index and claim it stays
editable. Those depend on the separate topology-naming and provenance layers.
