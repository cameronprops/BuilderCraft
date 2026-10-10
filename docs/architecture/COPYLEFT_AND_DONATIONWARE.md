# Worldwright: open-source donationware and copyleft integration

Status: **project policy**. The development goal is a freely usable and
redistributable, open-source Worldwright, optionally supported by donations.
There is no paywall, license key, or commercial-tier requirement for the alpha.
Charging money and accepting donations are both permitted under GPL licenses.

## What is changing

**Upstream projects no longer need to offer an MIT license** to be considered.
GPLv2/GPLv3, LGPL, MPL, Apache-2.0, BSD and other actual open-source licenses
are eligible, provided their terms permit the *specific combination and
distribution* we plan. Before integrating source, check the license of that
particular file, dependencies, assets, icons, fonts and generated resources.
Verify whether the result is a combined work, plugin, standalone process, or
merely an independently reimplemented UI pattern.

The existing Worldwright/CADCraft Rust crates are marked
`MIT OR Apache-2.0`. **Do not silently change those headers or claim to
relicense third-party material**. Original permissively licensed files can
continue to have their original licenses; a distributed binary or combined
derivative that incorporates GPL code may have to be distributed under GPL,
with corresponding source and notices. This is a packaging/legal decision,
not a conditional build flag that makes obligations disappear.

Source releases and builds that include covered GPL portions must provide
the corresponding source, build/install scripts and applicable license
notices under the relevant GPL terms. This includes patches to upstream
GPL files and any applicable whole-work requirements. GPL and MPL have
different obligations: do not use "copyleft" as a substitute for reviewing
the actual license.

## Source reuse procedure

1. Identify the precise **upstream repository, file, revision and license**
   (not just the product name or a promotional website).
2. Confirm it is redistributable and compatible with the combined artifact;
   record copyright notices, source revisions and modifications.
3. Prefer a stable dependency or an adapter over vendoring an entire
   application. Integrate only verified value with tests, tolerance budgets,
   and one mathematical implementation across interfaces.
4. If copying code, add a manifest entry with destination file paths,
   upstream revision, license and notice locations. Include copied
   notices/LICENSE/COPYING files as required.
5. Run the manifest guard and Rust CI before merge. Maintain a third-party
   source archive or reliable means to obtain Corresponding Source where
   distribution requires it. Review GPL user-interface notice obligations.
6. Keep command IDs, object/document types and serial formats independent
   of external UI implementations when practical, so toolkit components can
   be replaced.

## UI sources being evaluated

- **CADCraft:** existing MIT/Apache-2.0 foundation is already integrated.
  Reuse its menus, command line, selection, drawing, viewport and tool APIs.
- **GIMP:** GIMP application core is GPL and its app framework uses GTK.
  `libgimp` libraries have separate LGPL licensing; icon themes are
  Creative Commons BY-SA and other distributed assets have their own
  licenses. We may use documented concepts such as dockbooks, persistent
  tool options, tabs and contextual commands. Directly copying GTK
  implementation into Worldwright's egui shell is not automatically useful,
  so it requires an explicit reviewed implementation decision.
- **Photon Studio (Tenzen):** free-to-use is **not** proof of open-source
  distribution rights. We have found public information about its editing
  workflow and a website repository, but have **not verified a redistributable
  source license for Tenzen's desktop application**. Use it as a behavioral
  reference only. A similarly named MIT-licensed website repository does not
  license the underlying editor's private source.

These references do not authorize copying proprietary product artwork,
trademarks, icons, screenshots, custom fonts, or other separately licensed
assets. Use original icons and legitimate open assets with verified
attribution.

## A possible future enterprise edition

GPL **does not prohibit commercial sale or enterprise support**. A paid
edition can be open source, and services/support are separate from a closed
source license. If a proprietary combined edition is ever desired, merely
turning off a GPL feature is not enough. Copyleft-covered code would need to
be independently replaced or separated in a way consistent with actual
license terms. The resulting code and all contributors' rights must permit
the proposed distribution; a later rewrite does not retroactively relicense
third-party contributions.

For contributions today, keep an accurate chain of provenance and respect
the upstream's copyright. Do not ask contributors to surrender copyrights
by default; consider any future dual-licensing policy separately and
transparently before soliciting contributions under those terms.

## Build and compliance rules

- The canonical mathematical operation still lives in shared Rust crates.
- Unverified upstream code stays on an experiment branch; do not claim full
  upstream equivalence until cross-platform tests and fixtures pass.
- UI can borrow familiar *behavior* without copying implementation.
- No download/install/telemetry requirement is introduced by the donation
  model. Donations must remain optional.
- Every new source or UI inspiration is tracked in
  [`docs/dependencies/UI_CODE_REUSE.json`](../dependencies/UI_CODE_REUSE.json)
  (or the wider open-source dependency inventory).
- Run `python3 tools/check_ui_reuse_manifest.py` and its unit tests.

**Legal review note:** this is an engineering compliance policy, not a
legal opinion that a particular GPL/LGPL/MPL combination has been cleared.
