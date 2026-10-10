# WorldWright alpha interaction and scene-organization delivery map

**Authoritative tracker:** [GitHub issue #33](https://github.com/cameronprops/BuilderCraft/issues/33). Related geometry gate: [#26](https://github.com/cameronprops/BuilderCraft/issues/26). These tasks do not supersede the BRep selection and robust geometry work. All UI tools must call the same typed engine contracts as scripts/API/OrbWeaver.

| Phase | Deliverables | Dependencies | Current status at planning |
| --- | --- | --- | --- |
| A0 | Manual skeleton, navigation specs, test fixtures, behavior matrix | Existing CAD & UI source audit | Documentation being built |
| A1 | Camera controller, selectable projection and 1/4-view layout, persisted workspace/input profile | Common camera frames, UI state migration, viewport routing | Single orthographic viewport partial |
| A2 | One selection router for 2D+3D, filters, marquee, mesh/BRep/NURBS subobjects and hit stack | Stable object/subobject ID, topology refs, spatial acceleration, camera/depth test | Wire/sample mesh-face pick partial |
| A3 | Gizmo/typed transforms, pivot/local/world/CPlane, snapping and cancel | A1/A2, existing transforms, revisioned undo | World-axis move/rotate/scale partial |
| A4 | Wireframe and shaded renderer with exact-geometry display proxies, normals, clipping/depth | Surface tessellator, scene/material metadata, GPU selection coherence | Wireframe partial; shaded not verified |
| A5 | Minimal material IDs/browser, color/opacity, object/face/layer/block inheritance and native persistence | A4 for shading; document migration, asset IDs | Not integrated |
| A6 | Group editor, definition/instance 2D/3D block editor, all-object tree and multi-lens scene browser | Object/relationship IDs, instance transforms, undo, cycle guards, A2 | 2D blocks/layers + basic model browser partial |
| A7 | Reference image planes/underlays, scale calibration, links, locking and relink | A1, A2, A5 metadata, safe image loaders, document migration | Roadmap only |
| A8 | Native manual/help link, docs completeness, cross-platform interactive acceptance | A1-A7, kernel/OrbWeaver checks | Pending |
| Beta | Real rendered/PBR/shadows, walkthrough/fly/ride-path cameras, enhanced reference library | A1-A8, motion/physics/Show/Unreal bridges | Deferred |

## Architecture rules

1. **Single authoritative object identity**, many independent views. A layer organizes appearance, a group organizes selection, a block owns a reusable definition, an instance places it, and an assembly/component/body organizes authoring.
2. **No automatic geometry downgrades** on shaded display, instancing, group edits, import/export or workspace switches. Tessellation is a bounded cache; backend metadata retains sources and subelement provenance.
3. **Clear alpha MVP material split:** authored material definition vs display override vs shading proxy. Wireframe can work without PBR; shaded needs color/normal/depth correctness, not ray tracing.
4. **Separate view and document state:** per-viewport camera/style/input preferences versus persistent object/material/block/reference metadata. Camera movement must not create model undo steps.
5. **Input priority and safety:** modal command > gizmo > select > navigation; cancel returns to last committed model state. Hidden/locked/reference-only objects cannot accidentally be edited.
6. **No forged completeness:** describe the current source foundations as partial; require native interactive and file-roundtrip signoff to update status to implemented.
7. **Platforms and release:** Windows, macOS, Linux concurrent checks; Haiku native as planned, separately validated. No proprietary code copied from Rhino/Adobe/Autodesk; open-source compatible donors considered with GPL/copyleft attribution audits.
8. **Documentation ships with code:** merge tests and manual changes together, self-contained offline Markdown plus optional MkDocs site, no paid hosting needed.

## Earliest low-risk development slice
- Introduce a serialized UI-only `DisplayMode` enum (Wireframe/Shaded) with default Wireframe, per viewport migration, user-visible controls and command registration. **Do not expose Shaded as working until it draws actual geometry.**
- Implement **bounded approximate shaded display** for native triangle/quad meshes with reliable depth, selection, face highlights and fallbacks; add NURBS/BRep proxy path with provenance. If a temporary painter-algorithm fill is used, label it approximate and never count it as fully correct shaded mode.
- Start camera/layout management and a unified selection router without waiting on all exact Boolean operations. Keep typed result/error contracts central and interoperable.
- Build and publish the manual side-by-side with development, with each status tagged.
