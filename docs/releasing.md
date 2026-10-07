# Releasing CADCraft

The canonical recipe is craftrules' [`release/playbook.md`](https://github.com/storytold/craftrules/blob/main/release/playbook.md); this file covers only what's CADCraft-specific.

- **Version:** `[workspace.package] version` in the root `Cargo.toml` is the single source. `cargo xtask version` prints it; `cargo xtask version set X.Y.Z[-pre]` changes it. The app shows it in About, `cadcraft --version` and `cadcraft-cli --version`; release builds also embed the commit (`CADCRAFT_BUILD_SHA`).
- **Cut a release:** bump the version on `main`, then push `main` to `release` (`git push origin main:release`). Every push to `release` runs `.github/workflows/release.yml` and creates or updates the draft GitHub Release `CADCraft v<version>`. A `workflow_dispatch` run with `version: X.Y.Z-rc.N` makes a test build.
- **Artifacts:** macOS universal `.dmg` (Developer ID signed, notarized, stapled) and CLI `.zip`; Windows x64 and x86 `.msi` + portable `.zip` (Azure Trusted Signing); Linux x86_64 `.AppImage`, `.deb`, `.rpm`, `.tar.gz` (+ Flatpak manifest in `packaging/linux/flatpak/`); FreeBSD x86_64 `.tar.gz` (built in a FreeBSD VM); web `.zip` (wasm + `index.html`, see `packaging/web/README.md`); `SHA256SUMS.txt`.
- **Secrets:** the twelve signing secrets live in the `release` environment (deployable only from the `release` branch, which only org admins and release-managers can push). All are optional: a missing one produces unsigned artifacts with a warning.
- **File types:** `.dxf` and `.dwg` (macOS document types, Windows "Open with", Linux MIME types).
- **Local packaging test:** `packaging/macos/package.sh` (ad-hoc signed DMG), `packaging/linux/package.sh`, `packaging/web/package.sh`.
