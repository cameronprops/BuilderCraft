# Haiku x86_64 native technology preview

This directory provides a native **headless Worldwright/CADCraft CLI** build and
release gate for Haiku. It is **not** the Worldwright CAD desktop interface.

The UI implementation currently uses eframe/winit/wgpu, which is not a validated
Haiku desktop backend. Running the Linux binary under compatibility tooling,
or opening the web build in a Haiku browser, does not count as native support.

## Build on a real Haiku x86_64 installation

Install a working Haiku-native Rust/Cargo toolchain capable of compiling the
workspace's pinned Rust 1.95.0 source, with the required native development
libraries and build utilities. Do not assume `rustup target add` supplies
Tier 3 Haiku artifacts; follow the HaikuPorts or source-build toolchain route
appropriate to the installation.

From the repository root:

```sh
bash packaging/haiku/package.sh
```

The script checks the **host** OS and architecture, runs kernel and CLI tests,
builds the CLI, then checks actual execution of a native sample DXF creation,
inspection and convert/reopen path. Any failure prevents packaging.

After a passing run it creates the unsigned CLI-only archive and a SHA-256
sidecar in `dist/release/`. It does not submit to GitHub Releases or promise
a .hpkg package.

## Delivery stages

- **Invited alpha:** recruit Haiku testers, verify a working x86_64 native
  toolchain and run the headless/kernel gate. A CLI preview can ship with an
  explicit headless label if those native checks pass. A desktop invite requires
  the independent graphical acceptance gates below.
- **Open beta:** target a true Haiku graphical application distributed as a
  native package, with independent install/start, viewport/selection, geometry
  edits, undo/redo, and .dftba/.bcraft reopen and interchange tests.

Nothing is considered shipped until the referenced artifacts have been built,
executed, validated on Haiku, and published. See
`docs/platforms/HAIKU.md` for the graphical port's dependency gates.
