# Procedural modeling scope

Requested by Cameron: a Houdini-like procedural component including Grasshopper functionality, integrated into BuilderCraft as one shared project.

Status: requested; not implemented in alpha 0.1.

The node editor and graph execution should share the existing geometry, document, command/API, undo, organization and persistence systems. CAD and mesh processing should reuse these services rather than duplicate them.

Planned foundations: typed geometry and numeric ports, parameter nodes, connected operations, lists and Grasshopper-style data trees, reusable subgraphs, incremental evaluation, viewport previews, and baking results into named bodies and layers. Procedural graphs must be accessible through the same API and saved with the native project.

Initial geometry nodes should wrap supported curve and surface operations. Mesh smoothing, offsetting, shelling, hole filling and repair remain requested future tools, exposed through the same graph system when implemented.
