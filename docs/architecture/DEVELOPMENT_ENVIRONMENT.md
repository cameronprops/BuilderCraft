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

### GitHub billing

**No GitHub Actions jobs are required** to use either option above. The
Worldwright workflows are manually dispatched rather than triggered by
normal pushes or pull requests. Opening the local Dev Container does not
run GitHub Actions.

GitHub Codespaces can use the same `.devcontainer` configuration but
Codespaces has **separate compute/storage billing**, even with Actions
disabled. Use a local container to avoid hosted build charges.

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
