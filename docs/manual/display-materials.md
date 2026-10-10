# Wireframe, shaded display and material records

**Status:** Wireframe 3D preview exists in source. **Alpha gate:** usable Wireframe and Shaded display modes with consistent interaction and a minimal material/appearance system. **Beta:** full Rendered/PBR real-time mode, sophisticated lighting and ray/path tracing.

## Three representations, three responsibilities

1. **Authoritative object geometry:** analytic curve, rational NURBS, trimmed BRep or polygon mesh. A shading triangle derived from BRep is NOT its geometric source.
2. **Display mesh/cache:** adaptive view-dependent tessellation with tolerance, face identity and revision provenance; rebuilt/invalidate when the source changes. The renderer can draw this efficiently.
3. **Material/appearance:** named definition and inheritance/overrides determining display color, opacity, sidedness and later PBR data. Materials never replace topology.

## Display modes

| Mode | Alpha | Rules |
| --- | --- | --- |
| **Wireframe** | Required | Curves and all model boundary/feature edges, accurate selection/highlights, layer/object color, optional x-ray and back-edge policy |
| **Shaded** | Required | Filled geometry with simple directional light, face normals, depth visibility, wire-overlay toggle and selection highlight; use tessellated preview for exact surfaces and mark approximation |
| **Rendered** | Beta | Texture, PBR metallic/roughness, image-based/world lighting, shadows, transparent blend sorting and optional ray/path trace |
| X-ray, Ghosted, Technical/Arctic | Beta or opportunistic | Must not delay baseline wireframe/shaded acceptance |

## Minimal alpha material schema (proposed, not implemented)

`Material { id, name, base_color_linear_rgba, opacity, double_sided, metallic?, roughness?, texture_slots?, revision }`. Stable IDs and names; color UI presents sRGB but stores linear values; document persistence has migration defaults and no lossy save. Initial mandatory fields: name, base color, opacity, double-sided, assignment and default layer/object inheritance. Metallic/roughness may be retained as metadata before PBR renderer uses them. Provide Material Browser and color picker as a basic assignment UI, reuse consistent properties across CAD/OrbWeaver/Unreal export.

Precedence: explicit subobject override (where supported) > instance override > object > block definition > layer > document default. Visibility and material inheritance are **separate**: a translucent reference image is not a translucent BRep. Editing a shared material changes all assigned items without geometry mutation; duplicating a material creates a new ID.

## Renderer requirements

A real visible-surface/shaded engine needs depth-buffering, clipping, face identity and consistent selection. Do not mistake CPU painter-sorted polygon fills for fully correct occlusion in interpenetrating geometry. A temporary shaded mesh preview may exist before that requirement passes, but it must be labeled **approximate preview** and cannot count as alpha acceptance. Layer/order toggles, huge coordinates, normals, thin surfaces and mixed triangle/quad+NURBS displays all require fixtures.
