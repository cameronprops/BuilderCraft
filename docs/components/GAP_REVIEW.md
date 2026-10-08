# Parameter and version reconciliation

The 817-entry Grasshopper public feed is incomplete as a runtime inventory. Its
Params library does not list basic typed parameter containers such as Point or
Curve. These must be implemented too. The entries below are a required review
queue, not additional verified catalog counts. Runtime GUIDs, exact names,
platform/build availability and detailed conversions remain to be checked.

| Type to review | Native behavior to build | Status |
|---|---|---|
| Generic data | Pass arbitrary typed values through the graph | Catalog verification pending |
| Geometry | Store and pass geometry references | Catalog verification pending |
| Point | Store three-dimensional locations | Catalog verification pending |
| Vector | Store three-dimensional directions and magnitudes | Catalog verification pending |
| Plane | Store an origin and coordinate frame | Catalog verification pending |
| Curve | Store curve geometry | Catalog verification pending |
| Surface | Store surface geometry | Catalog verification pending |
| Brep | Store boundary representations | Catalog verification pending |
| Mesh | Store vertices, connectivity and attributes | Catalog verification pending |
| Line | Store linear segments | Catalog verification pending |
| Arc | Store circular arcs | Catalog verification pending |
| Circle | Store circles in a plane | Catalog verification pending |
| Rectangle | Store planar rectangular boundaries | Catalog verification pending |
| Box | Store oriented three-dimensional bounds | Catalog verification pending |
| Boolean | Store true/false values | Catalog verification pending |
| Integer | Store whole numbers | Catalog verification pending |
| Number | Store real-valued numbers | Catalog verification pending |
| Text | Store strings | Catalog verification pending |
| Colour | Store color values | Catalog verification pending |
| Domain | Store one-dimensional intervals | Catalog verification pending |
| Domain² | Store paired parameter intervals | Catalog verification pending |
| Transform | Store geometric transformations | Catalog verification pending |

Also reconcile hidden/obsolete components, scripting components, persistent
parameter editing, expressions, clusters, previews, bake semantics, data matching,
platform-specific items and Rhino-host integration. Native geometry references
must use BuilderCraft identities rather than require a Rhino document.

Public API documentation itself can drift: the namespace page at
https://developer.rhino3d.com/api/grasshopper/html/N_Grasshopper_Kernel_Parameters.htm
identified its build as Rhino 9.0.26272.12300 on retrieval. It cannot silently
certify the Rhino 8 / Grasshopper 1 baseline. Grasshopper 2 remains a separate
catalog and must not be substituted for missing Grasshopper 1 entries.

The next catalog pass must reconcile a versioned component/parameter registry
with public documentation, include typed ports and matching behavior, and
record unresolved entries explicitly. No native working coverage is inferred
from any discovery source.
