# Third-party UI and geometry reuse for commercial and open-source editions

Status: engineering/licensing decision record. This is **not legal advice** and
does not imply that unexamined code is approved for copying. Before shipping,
verify the exact revision and file-level SPDX licenses, all dependencies,
patents/trademarks, redistribution notices and platform runtime terms.

## Product policy

**Prefer proven source over reinvention, while maintaining one authoritative
operation.** Worldwright should ship a free open-source edition and leave
room for a paid enterprise edition. Commercial distribution is allowed by
open-source licenses including GPL; **proprietary enterprise distribution**
is the separate question. The owner has authorized GPL/LGPL/MPL/Apache/MIT
evaluation, not automatic embedding of every copyleft module in the same
proprietary binary.

Consider runtime/host requirements separately from permission to use an SDK.
Evaluate upstream code with performance benchmarks, accuracy fixtures,
security review and maintenance health. Never claim license compatibility
based solely on the repository root license: individual files and assets may
differ.

## UI references and source reuse

| Project | Verified repository or source | Code/reuse approach | Enterprise consequence |
|---|---|---|---|
| **CADCraft** | [Upstream](https://github.com/storytold/cadcraft) and inherited Rust `crates/ui-egui` | **Already reused as app foundation**: egui menus, command line, layers, properties and canvas; improve same shell | Preserve upstream provenance/notice and dependency obligations |
| **GIMP** | [GNOME mirror](https://github.com/GNOME/gimp) | Borrow documented interaction patterns: dockable panels, toolbox presets, canvas behavior, undo, layer inspector. Do not lift GPL program core into permissive app by default | Core is GPL; direct combined proprietary distribution may be incompatible |
| **Inkscape** | [Official GitLab](https://gitlab.com/inkscape/inkscape) | Reference snapping, vector editing, contextual tool controls, SVG workflow, selectors and document/page controls. Reuse permissively licensed sub-libraries only after file-level check | Main editor GPL; packaged binaries may contain GPLv3+ dependencies |
| **Photon Studio** | [PhotonStudio website repo](https://github.com/arsnexc/PhotonStudio), **identity pending** | The identified repo contains a marketing website, not a confirmed image-editor implementation. Can reference public layout/accessibility patterns; **no CAD/editor code copied** | Verify the intended project identity and actual source before adopting |
| **RhinoCommon SDK** | [McNeel developer MIT license](https://developer.rhino3d.com/en/license/) | Optional C#/.NET bridge to **user-provided licensed Rhino**; not a Rust kernel substitute | MIT SDK license permits commerce; full Rhino binary/runtime governed by its separate EULA |
| **rhino3dm / openNURBS** | [rhino3dm](https://github.com/mcneel/rhino3dm), [openNURBS](https://github.com/mcneel/opennurbs) | Preferred standalone `.3dm` compatibility and compatible NURBS geometry evaluation support, tested roundtrip before marketing support | Free SDK/toolkit, retain notices and test supported operation surface |
| **Rhino.Compute** | [McNeel compute licensing](https://developer.rhino3d.com/guides/compute/compute-faq/) | Optional local/server adapter with explicit licensed Rhino or billable compute provider; not needed for core | Server core-hour licensing may apply |

## License review matrix

- **MIT/BSD/ISC/Apache-2.0:** generally amenable to permissive standalone
  and proprietary enterprise distributions with preservation of notices,
  and where applicable patent grants and trademark restrictions.
- **MPL-2.0:** file-level copyleft, potentially appropriate for a separate
  library if modified covered files are disclosed under their terms.
- **LGPL:** library-level copyleft with linking, relinking, notice and
  modification rights to review for each binary/platform.
- **GPL:** not excluded from the product ecosystem; linking or copying core
  GPL code into a distributed combined work can impose source/redistribution
  obligations incompatible with a closed-source enterprise build. Commercial
  sale of a GPL-covered enterprise edition is possible, but recipients
  retain GPL freedoms. A separate process over an interface **is not an
  automatic legal exemption**.
- **AGPL:** network deployment may invoke additional source-availability
  obligations. Review separately before offering hosted enterprise services.
- **Commercial/SDK-only runtimes:** license of wrapper bindings does not
  automatically grant rights to bundle closed host programs or use protected
  services without fees.

## Distribution architecture

1. Keep shared Rust geometry, document model, command registry and OrbWeaver
   execution independently executable on ordinary OSes under a reviewed
   compatible license set.
2. Preserve licenses upstream and register any modified files and any source
   that must be published. Separate UI reference **behavior** from source reuse.
3. Use out-of-process/service/format bridges for independently supplied
   proprietary hosts such as Rhino, with explicit capability detection and
   optional licensing. Never hide a paid-host dependency in an alpha feature.
4. For GPL/LGPL/MPL components, record a **per-component legal integration
   decision**: interface, combined-work risk, distribution obligations,
   source offer, and enterprise packaging. Get counsel review for an actual
   proprietary release rather than claiming a universal safe plugin boundary.
5. Run reproducible software-bill-of-materials and dependency license scans
   in release engineering. Do not allow trademarks, exclusive branding or
   upstream icons to migrate accidentally into Worldwright assets.

## Immediate interface implementation

`crates/ui-egui/src/workspace.rs` owns two reversible presentation modes;
they reuse CADCraft's existing command/viewport shell and do not copy GIMP
or Inkscape source. Menus, keyboard shortcuts and typed commands all call
one UI-only transition. Future palette composition should reference stable
command IDs, not fork mathematical implementations.
