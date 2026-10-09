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
        [pscustomobject]@{Name = "Rust format"; Args = @("fmt", "--all", "--", "--check")}
        [pscustomobject]@{Name = "Mesh scene integration"; Args = @("test", "--locked", "-p", "buildercraft-kernel", "--test", "mesh_scene")}
        [pscustomobject]@{Name = "Shared kernel"; Args = @("test", "--locked", "-p", "buildercraft-kernel")}
        [pscustomobject]@{Name = "CAD document, I/O, engine and UI"; Args = @("test", "--locked", "-p", "cadcraft-doc", "-p", "cadcraft-io", "-p", "cadcraft-engine", "-p", "cadcraft-ui-egui")}
        [pscustomobject]@{Name = "Calisoga shared graph"; Args = @("test", "--locked", "-p", "calisoga")}
        [pscustomobject]@{Name = "Kernel clippy"; Args = @("clippy", "--locked", "-p", "buildercraft-kernel", "--all-targets", "--", "-D", "warnings")}
    )
    foreach ($check in $checks) {
        Write-Host ("Checking: " + $check.Name)
        $arguments = [string[]]$check.Args
        & cargo @arguments
        if ($LASTEXITCODE -ne 0) {
            throw ("Worldwright validation failed at " + $check.Name + " with exit code " + $LASTEXITCODE)
        }
    }
    Write-Host "Worldwright local validation passed."
}
finally {
    Pop-Location
}
