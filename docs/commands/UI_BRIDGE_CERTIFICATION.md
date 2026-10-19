# Native UI input-bridge certification

This is an additional, reproducible release gate between the headless CAD
engine's command certification and a physically operated GUI. It does not
replace screenshot review, keyboard/focus tests on a running desktop app,
GPU picking tests, or native platform installers.

## Executable scope

`crates/ui-egui/tests/command_input_bridge.rs` drives the production
`CadApp` and the same `control::handle` dispatcher used by the app's
command line and viewport automation. It exercises:

- Line alias `l`, pointer creation, live rubber-band preview, Enter, undo
  and redo across multiple document positions
- Real idle pointer hit-testing on a geometric line, pick-first Move `m`,
  Escape without mutation, pointer-supplied displacement, undo and redo
- Copy `co`, pointer-supplied displacement, original-object preservation,
  distinct persistent handle, grouped undo and redo

The tests do not set selection or invoke the `Session` editing commands
directly for edit operations. Geometry creation in the Move and Copy
fixtures uses the documented `engine.execute` interface to seed a
real source entity. The subsequent selection and edit traverse the UI
input bridge.

```sh
cargo test --locked -p cadcraft-ui-egui --test command_input_bridge
```

## Promotion rules

These are native **UI-control bridge** acceptance tests, not desktop
certifications on their own. Any command promotion still needs passing
pinned Rustfmt, Clippy, Linux/macOS/Windows validation, the applicable
exact BRep tests, post-merge `main` checks and a live desktop test of
keyboard focus, pointer projection and visual feedback. Store those
results separately from the headless tests. The machine-readable command
registry must not advance a command to desktop-certified on this test alone.

There are no new application or runtime dependencies. Command
implementations and underlying geometric operations remain unchanged.
