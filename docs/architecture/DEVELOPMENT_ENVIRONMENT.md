# Worldwright development environments

## Guarantee boundary

The repository pins **Rust 1.95.0** in `rust-toolchain.toml` (including
`rustfmt`, `clippy`, and `wasm32-unknown-unknown`). This ensures version
selection in a checkout **only when rustup is installed**. It does **not**
install Rust or Cargo into unrelated machines or chat execution sandboxes.

For reproducible development, the supported default is
`.devcontainer/devcontainer.json` and `.devcontainer/Dockerfile`. This
container **installs Rust, Cargo, build dependencies and development tools
into its image**. Each environment that opens the repository through the
Dev Containers specification can rebuild the tools automatically. The image
persists across normal container stops/restarts; rebuilding can recreate it.
First setup requires Docker image and package downloads.

### Option A: local Dev Container (recommended for parity)

1. Install Docker Engine/Desktop and Visual Studio Code with the Dev
   Containers extension on Windows, macOS or Linux.
2. Clone the repository. From VS Code, open the repository folder.
3. Run **Dev Containers: Reopen in Container**. The Dockerfile installs
   the pinned Rust 1.95 toolchain, Cargo, rustfmt, Clippy, WASM target and
   Linux libraries required for the CAD desktop/test compilation.
4. The post-create tool check runs automatically. In a container terminal:
   ```sh
   cargo --version
   rustc --version
   bash tools/verify-worldwright-kernel.sh
   ```

The default environment is designed for **builds and headless tests**.
GUI acceleration/window forwarding from a Linux container is platform-specific;
for desktop interface testing, running Worldwright directly on the host may be
easier. Linux containers do not natively produce Windows/macOS desktop binaries.

### Option B: install natively without Docker

- Install `rustup` from **https://rustup.rs/** on each developer machine.
- On Windows, install the MSVC C++ build/link tools if they are not already
  available. The repository does not install proprietary Windows toolchains.
- Open a terminal in the repository and run:

```sh
rustup toolchain install 1.95.0 --profile minimal --component rustfmt --component clippy --target wasm32-unknown-unknown
bash tools/check-worldwright-rust.sh
bash tools/verify-worldwright-kernel.sh
```

`rust-toolchain.toml` also lets rustup select and provision the matching
toolchain when Rust commands run from within the checkout. Outside the
checkout, other toolchain versions can remain in use.

### Option C: disposable or cloud agent environments

If a coding agent, cloud VM or remote computer supports Docker/Dev Containers,
**open the repository inside its dev container**, rather than installing Rust
ad hoc and losing it when the machine resets. If it supports only plain shell
commands, install rustup/toolchain in that specific environment before running
tests. Keep all environment setup in the repository so it can be repeated.

A ChatGPT chat's temporary Python/container tool is **not** automatically a
Dev Container. No file in GitHub can force that runtime to have or retain
Cargo, and installing tools there once does not guarantee their availability
in other chats, environments or future sessions. Report unexecuted tests
as unverified rather than passing.

### GitHub Actions: free public-repository CI

Worldwright is public. GitHub's **standard GitHub-hosted runners** on public
repositories are free for Actions compute, including standard Ubuntu,
Windows and macOS runners. The owner's explicit authorization allows native
validation to run automatically after pushes and pull requests. Normal
GitHub-hosted runners compile actual Rust code with the pinned toolchain,
without requiring a compiler in ChatGPT's temporary environment.

Use `ubuntu-24.04`, `windows-latest`, and `macos-latest` standard runner
labels. Require formatting, compile, tests and lint checks before treating a
PR as validated. The workflows must not silently modify sources; changes to
fix failing checks should be reviewed and committed by developers.

**Cost boundary:** larger/GPU runners are chargeable even for public projects;
do not configure them. Codespaces also has separate billing. Free standard
runner compute does not guarantee unlimited artifact/cache storage: avoid
unnecessary uploads and monitor storage allowances. If the repository ever
becomes private, reassess usage and disable automatic jobs until billing is
explicitly approved. Keep least-privilege read-only CI permissions, timeouts
and redundant-run cancellation. No automatic release publication or merges.

Public pricing reference:
https://docs.github.com/en/actions/how-tos/write-workflows/choose-where-workflows-run/choose-the-runner-for-a-job

The native CI workflow is `.github/workflows/worldwright-native-ci.yml`.
Its status and run logs, not file existence, are the source of truth for
passed tests. Local Dev Containers remain useful for manual GUI smoke tests.

### Validation gate

Before merging a change to Rust sources:

```sh
bash tools/check-worldwright-rust.sh
bash tools/verify-worldwright-kernel.sh
```

The repository's existing Windows PowerShell validation equivalent is
`tools/verify-worldwright-kernel.ps1`. Native tests may require additional
platform SDKs. The dev container definition has not been built or run in a
real Docker host from this GitHub-only authoring session; verify its first
build and document any platform-specific fixes.
