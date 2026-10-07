# Format support and target matrix

Import, export and fidelity are separate acceptance gates. No extension is considered supported solely because it can be detected. Proprietary application history, constraints, materials and assemblies are separate from geometry compatibility.

| Format | Current alpha | Planned use |
|---|---|---|
| .bcraft | Read/write, versioned JSON envelope with supported DXF data and BuilderCraft exact 3D/organization data | Native project; evolving schema with explicit migrations |
| DXF | Inherited drafting import/export; BuilderCraft 3D export blocked to avoid silent geometry loss | Broad drafting exchange, future 3D mapping |
| DWG | Inherited acadrust adapter; BuilderCraft 3D export blocked | Validate version/entity fidelity with an independent corpus |
| SVG, PNG, PDF | Inherited drawing exports; 3D exports blocked | Drafting deliverables |
| Rhino 3DM | Not implemented | Evaluate openNURBS/rhino3dm bridge, preserve exact geometry and supported attributes |
| STEP | Not implemented | Solids/surfaces and assembly interchange |
| IGES | Not implemented | Curves/surfaces interchange |
| STL | Not implemented | Explicit triangulated manufacturing export/import |
| OBJ, PLY, 3MF | Not implemented | Mesh interchange, scan colors, print metadata where supported |
| glTF/GLB | Not implemented | Display mesh, materials and scene exchange |
| E57, LAS/LAZ, XYZ | Not implemented | Point clouds, scan metadata and coordinate frames |
| SAT/SAB | Not implemented; feasibility/licensing research needed | ACIS exchange where legally and technically supportable |
| Inventor IPT/IAM, Fusion native projects, Revit RVT/RFA, AutoCAD specialized objects | No native support promised | Prefer documented/open interchange or an explicitly licensed external adapter |

.bcraft v1 preserves only what its embedded DXF serializer supports for drafting. It is not an archival guarantee for all unsupported DXF/DWG records. The exact new 3D control data is stored separately and does not pass through DXF. Formats that cannot represent BuilderCraft 3D objects currently return an error rather than silently exporting an empty 3D model. The same preservation policy should extend to assembly metadata and future mesh attributes before broad export is enabled.

Every future adapter needs tests for units, axes, tolerances, geometry, object names, layers, organization, materials and unsupported-data reporting. Import must preserve the original file and return a conversion report. Export must disclose exact versus tessellated geometry and all dropped attributes.
