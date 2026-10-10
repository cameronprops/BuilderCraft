# Scene browser, layers, groups, blocks and components

**Status:** Existing foundations include 2D block definitions/INSERT commands, layer manager, persisted 2D Group table, 3D named Assembly/Component/Body nodes and basic model browser. **Alpha gate:** fully integrated scene browser and editable 2D/3D groups/blocks.

## The five concepts must stay separate

| Concept | Meaning | Shared geometry? | Selection/visibility model |
| --- | --- | --- | --- |
| **Layer** | Drawing/display classification with optional hierarchy | Not an owner; an object has a layer | Visibility, lock, print and display inheritance |
| **Group** | Selection/organization of existing object IDs | Members are original objects; may belong to multiple groups | Group-select vs select member, group isolation; no copied geometry |
| **Block definition** | A reusable geometry/attribute definition with a base point | **Yes**; shared by multiple placed instances | Edit Definition updates all instances; nested definitions require cycle checks |
| **Block instance** | Reference to definition plus placement transform/overrides | Uses definition, does not clone by default | Select/move instance without redefining source |
| **Assembly / Component / Body** | Named authoring ownership and composition | Persistent node references; Body may own exact or mesh geometry | Expandable object tree, nested ownership, component visibility |

**Never conflate:** a Group is not a Block. A Layer is not a Component. A Scene/Shot is not a copied CAD model. A 3D mesh display proxy is not an original BRep.

## Required alpha scene browser

A single searchable outline should show drawing/project scenes; layers and sublayers; assemblies/components/bodies; block definitions/instances; groups; geometry objects; and optional reference planes. The browser needs a switchable **Layers**, **Components**, **Groups**, **Blocks**, and **All Objects** lens so users do not see a misleading single-parent tree where multiple relationships exist. Include create, rename, move, duplicate, reparent, lock, hide/show, isolate, selection sync, properties, expand/collapse and drag targets only when valid.

Editing a block:
1. Create definition from selected objects and a base point; place two instances.
2. Select one instance and move it. Other instance stays.
3. Enter Edit Definition. Change a component. Both instances update and maintain their transforms.
4. Try recursive nesting A → B → A. Reject with clear cycle error, preserve originals.
5. Exit edit context; save/reopen. Instances, identity, constraints and selection remain coherent.

Editing a group:
1. Select members, **Group**; click member and select whole group by default.
2. **Edit Group** to access members. Add/remove without replacing underlying IDs.
3. **Ungroup** removes relationship only; geometry and layer membership survive.
4. Verify multi-group membership and nested group validation, where explicitly supported.

The 2D CADCraft Blocks dialog and command family are real foundations, but must not be advertised as a complete mixed exact-NURBS/mesh 3D block editor. In the alpha, a user should be able to reuse scenic objects as instances and still edit the reusable definition.

## Data integrity

Stable IDs, ownership and references are checked at save/load. Instance transforms should compose in a documented order and avoid precision loss. Block recursion is rejected. Hide/lock applies through the hierarchy without destroying object-local visibility. All mutations are one shared-engine transaction with undo/redo. Formats unable to preserve groups/block hierarchies need an explicit warning before export.
