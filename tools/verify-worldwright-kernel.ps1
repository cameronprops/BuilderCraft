# Worldwright local source validation for Windows PowerShell.
# Uses only your local Cargo toolchain; does not invoke GitHub Actions.
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
try {
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        throw "Cargo not found. Install the Rust 1.95 toolchain locally."
    }
    # These are source-only checks; neither invokes GitHub Actions.
    $python = Get-Command python -ErrorAction SilentlyContinue
    if (-not $python) { $python = Get-Command py -ErrorAction SilentlyContinue }
    if (-not $python) { throw "Python 3 is required for Worldwright dependency checks." }
    $pythonArgs = @()
    if ($python.Name -in @("py", "py.exe")) { $pythonArgs = @("-3") }

    Write-Host "Checking: dependency graph/catalog consistency"
    & $python.Source @pythonArgs tools/build_dependency_index.py --check
    if ($LASTEXITCODE -ne 0) { throw "Dependency index check failed." }

    Write-Host "Checking: shared CAD/Calisoga contract consistency"
    & $python.Source @pythonArgs tools/check_paired_tools.py
    if ($LASTEXITCODE -ne 0) { throw "Paired tool check failed." }

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
