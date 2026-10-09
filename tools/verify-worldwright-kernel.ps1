# Worldwright local source validation for Windows PowerShell.
# Uses only your local Cargo toolchain; does not invoke GitHub Actions.
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
try {
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        throw "Cargo not found. Install the Rust 1.95 toolchain locally."
    }
    $checks = @(
        @("Rust format", @("fmt", "--all", "--", "--check")),
        @("Mesh scene integration", @("test", "--locked", "-p", "buildercraft-kernel", "--test", "mesh_scene")),
        @("Shared kernel", @("test", "--locked", "-p", "buildercraft-kernel")),
        @("CAD document, I/O, engine and UI", @("test", "--locked", "-p", "cadcraft-doc", "-p", "cadcraft-io", "-p", "cadcraft-engine", "-p", "cadcraft-ui-egui")),
        @("Kernel clippy", @("clippy", "--locked", "-p", "buildercraft-kernel", "--all-targets", "--", "-D", "warnings"))
    )
    foreach ($check in $checks) {
        Write-Host ("Checking: " + $check[0])
        & cargo @($check[1])
        if ($LASTEXITCODE -ne 0) {
            throw ("Worldwright validation failed at " + $check[0] + " with exit code " + $LASTEXITCODE)
        }
    }
    Write-Host "Worldwright local validation passed."
}
finally {
    Pop-Location
}
