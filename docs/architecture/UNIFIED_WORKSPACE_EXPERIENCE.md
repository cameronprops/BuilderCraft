# Worldwright: one continuous creative workspace

Status: **product direction under design review, not implemented functionality**.
Date: 2026-10-09. This UX direction complements the existing shared-kernel,
independent-app architecture in [SUITE.md](SUITE.md). It must not be mistaken
for a verified alpha feature or a reason to delay kernel debugging.

## Experience principle

**The design remains intact; the workspace changes around it.**

Worldwright should feel like one familiar creative application even when the
user moves among 2D drawing, Rhino-style CAD, OrbWeaver parametrics, mesh/scan
inspection, fabrication, landscape, rendering and show/ride engineering. The
persistent objects, parameters, relationships, revisions and materials are not
silently exported, converted or recreated whenever a user changes toolsets.

The user may choose a blank 2D document without first creating a 3D model.
Workspaces are flexible views and affordances, **not separate document formats
or copies of the project**.

## Project, document, representation, workspace

| Layer | Persistent responsibility | Switching workspace may change |
|---|---|---|
| Project | Stable identity, asset catalog, cross-document references, units, provenance and permissions | Active document, selection context and navigation |
| Document | Typed authoring content and history: 2D sheets/vector art, 3D CAD geometry/mesh, parametric graphs, timelines and equipment records | Which representations are visible/editable |
| Object | Stable identity, source revision, transforms, attributes, materials and relationships; one authoritative representation per owned operation | Display geometry, highlighting, handles, contextual actions |
| Workspace | View layout, tools, menu/palette configuration, inspector, viewports, coordinate systems and shortcuts | Entire presentation and relevant commands, **not document data** |
| User profile | Named layouts, mixed/custom tool palettes, input preferences, accessibility settings | Can override workspace defaults without changing shared files |

A project can contain multiple documents and assets. A 2D page stays a
legitimate page with its own coordinates, text, scales and layout metadata.
Its geometry can be linked or converted into model space explicitly and
reversibly where supported; never pretend typography, a mesh, a Brep and an
equipment cue are identical kinds of geometry. A referenced 3D viewport in a
sheet remains linked to a source object and revision. Derived display or
fabrication representations must retain their source link and fidelity policy.

**Invariant:** changing only the workspace cannot increment a document
revision, silently mutate geometry, change units, drop unsupported fields,
or replace exact geometry with tessellated preview meshes. Editing the
object still goes through the same typed transactions and undo/redo history,
no matter which workspace requested it.

## Familiar, consistent application shell

Stable anchors across all workspaces:

1. One **global command line**, with search, arguments, options,
   command/history feedback, cancellation and API-equivalent execution.
   Workspace-specific aliases may exist but resolve to stable namespaced
   command IDs; shortcut/alias conflicts are visible, never arbitrary.
2. One familiar document switcher, object/asset browser, properties
   inspector, undo/redo and operation/status surface.
3. One canonical selection and identity service, with workspace-specific
   selection filters and handles.
4. Predictable keyboard and pointer behavior, configurable gestures,
   context menus, popup toolbars and repeat-last-command behavior.
5. A discoverable workspace selector that shows the current workspace and
   offers a deterministic return path to the previous one.

Each domain can adapt its *menus* to familiar reference workflows: Rhino-like
CAD command prompts, popup toolbars and contextual menus; Figma-like blank
2D canvases; metrology-specific inspection panels; OrbWeaver nodes; and
fabrication-specific property controls. Reproduce **useful public behavior
and affordances**, not proprietary icons, source code, branding or a
pixel-identical interface.

A workspace change should be a visual/interaction transaction: validate
available capabilities, preserve document selection where meaningful, then
activate the new view. If a capability is unavailable, explain why and
keep the user in a safe state. Do not silently discard unsaved view work.

## Frankenstein palettes without Frankenstein math

User-created palettes are **declarative compositions of existing commands**.
Their entries reference canonical operation IDs and optional view-specific
actions. They do not carry duplicate geometry implementations.

A future palette manifest should describe:

- Stable palette ID, version, title and purpose;
- Groups/ordering, labels, optional icon references and visible controls;
- Command IDs with typed input schema and capability gates;
- Context rules (selected object type, document, mode), optional
  workspace aliases and user shortcut overrides;
- Fallback behavior when an operation is unavailable;
- User/profile persistence, import/export and schema migration.

Examples: a rockwork palette can combine CAD surface creation, Scan mesh
cleanup, fabrication shell thickness and material assignment. A technical
drawing palette can combine 2D curves, snapping, dimensions, layers and
page layout. They must call **the same shared Rust operations** exposed to
CAD, OrbWeaver, CLI and APIs. Mixing unrelated controls does not imply that
all commands are valid for every selection or data type.

User palettes and workspace preferences normally live in a profile or
portable configuration sidecar; they should not unexpectedly modify or
pollute the authoritative `.dftba` design document. A project may explicitly
embed a portable shared workspace preset with its own versioned schema.

## Neurodivergent-friendly, accessible by default

Predictability is a functional design requirement, not a decorative theme.

- **Stable landmarks:** command line, selection indication, escape route,
  feedback and undo remain in known places. Workspaces do not rearrange
  pinned panels or remap shortcuts without explicit consent.
- **Low cognitive load:** sensible defaults; progressive disclosure of
  advanced options; consistent labels and descriptions across toolsets.
- **Visible mode and state:** current workspace, active command, snapping
  mode, selection filter, document units, unsaved edits and background work
  are clearly indicated. No invisible modal traps.
- **Reversible exploration:** preview before expensive/destructive
  operations, explicit confirmation only for consequential actions,
  dependable undo/redo, and an obvious Cancel/Back action.
- **Customizable sensory presentation:** scalable text/UI, keyboard-only
  navigation and focus indicators, high contrast, reduced motion,
  no mandatory animation, and individually adjustable density.
- **Remember user intent:** pin/favorite tools, save layouts, expose
  command search and recent actions, and preserve calm return paths.
- **Accessibility testing:** perform actual keyboard, focus, zoom,
  contrast, screen-reader/speech where supported, and usability tests.
  Documentation alone does not establish accessibility conformance.

Avoid surprising automatic toolset switching on selection. Offer a
user-controlled *suggested contextual tools* mode instead. When switching
to a different workflow, show which tools changed while preserving anchors.

## Module boundaries and one app

The existing CAD/Scan/Graph/Show boundaries remain useful for dependency
isolation, licensing, optional installation and standalone headless use.
They need **not** force separate visible applications.

Recommended composition:

```text
Worldwright Shell (desktop + optional specialized shell)
  Global command line / searchable command registry / history
  Document & project navigator / inspector / menus / context menus
  Workspaces (2D, CAD 3D, OrbWeaver, Scan, Fabrication, Terrain,
              Previs, Show)
      -> typed command adapters, optional UI modules
          -> shared document, identity, revisions, undo, job services
              -> shared Rust geometry, graph, simulation and I/O kernels
```

A stand-alone mode or lightweight app can host any subset of these modules.
The unified shell is the preferred continuous-design experience; the
independent modules remain testable without any GUI.

Avoid a single gigantic frontend crate: lazily activate optional
view/panel services, load heavy datasets on demand, cap memory/work queues,
and use stable capability manifests. GUI components own **presentation,
not mathematical engines**. One geometric operation, multiple interfaces.

## Future operating-system distribution

An optional **Worldwright Workstation** may eventually boot directly into
this shell as a dedicated creative environment. It is a **packaging and
desktop-session project**, not a reason to put CAD math into a kernel.

Phased evaluation:

1. Make the identical shell and project files work on supported desktop
   Windows, macOS, Linux and experimental Haiku targets.
2. Offer a portable Linux app bundle and optional full-screen/kiosk
   session, while keeping a normal desktop exit and recovery path.
3. Explore a specialized Linux-based image with a Wayland compositor,
   graphics/tablet drivers, secure updates, encrypted files/backups,
   user-configured peripherals and sandboxed plugins.
4. Consider maintaining a full distribution only after native application
   usability, hardware support, security update responsibility, rollback
   and upgrade testing can be sustained.

Linux/BSD/Haiku differences belong in **platform adapters** for input,
graphics, files, permissions and resources. Haiku is not a Linux/Unix
distribution; treat it as a distinct target. Do not change the Linux or
BSD kernel to host CAD operations. GPU acceleration, worker scheduling,
memory policies and scene caching should be implemented in portable
application/user-space services with platform-specific optimizations.

Every user must remain able to use Worldwright without the custom OS.
A specialized distribution must not become the only place the app runs.

## Release and verification gates

Do not divert the active Rust alpha effort into a GUI redesign or OS build
until shared operations have passed required tests.

- **Design gate:** define a typed, versioned workspace/palette schema and
  command capability resolver with tests; keep it separate from document
  persistence.
- **First vertical slice:** one 2D document, two workspace presets,
  the same editable object IDs, one command line and working undo across
  workspace switches. Test that switching does not increment revisions.
- **Cross-domain slice:** CAD geometry viewed and modified in OrbWeaver
  through the same kernel operation, with stable selection and provenance.
- **Customization gate:** save/reopen a mixed palette and test missing
  capabilities, shortcut conflicts, keyboard focus and migration.
- **Accessibility gate:** test calm/no-motion/pinned-panel scenarios and
  keyboard-only command execution. Do not claim compliance without evidence.
- **OS gate:** separate future initiative; keep all app and kernel tests
  running on ordinary host operating systems.

### Open questions (not blockers for current kernel work)

Should workspace layout and user palettes default to per-user only or sync
per-project on explicit invitation? How should collaborative edit ownership
work for a project containing 2D and 3D documents? Which Figma-like
2D illustration capabilities beyond existing drafting are required?
These require design experiments and user testing; the invariants above
do not depend on resolving them yet.

## Implemented alpha workbench slice (2026-10-10)

The native CAD desktop now has registered Modeling/Drafting/Focus workspace
commands, one custom saved layout, bounded versioned desktop preferences,
resizable side panels, a bottom command line, compact modeling toolbar and
keyboard command search over the engine/UI registries. Switching layouts does
not create a document transaction. Preferences exclude geometry, selection,
camera and transient drafting state. This does not implement the broader
cross-domain shell, mixed custom palettes or free docking proposed above.
See `docs/ALPHA_SMOKE_TEST.md` for acceptance and remaining work.
