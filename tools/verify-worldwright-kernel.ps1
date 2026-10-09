# Native Windows PowerShell smoke gate. Run inside a VS Developer PowerShell
# with Rust 1.95 installed. --Desktop additionally compiles the graphical app.
param([switch]$Desktop)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

foreach ($tool in @('rustc', 'cargo')) {
    if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
        throw "$tool not found. Install rustup and the Visual Studio C++ build tools."
    }
}

function Invoke-Checked {
    param([string]$Command, [string[]]$Arguments)
    & $Command @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Command $($Arguments -join ' ') failed (exit code $LASTEXITCODE)"
    }
}

Invoke-Checked rustc @('--version')
Invoke-Checked cargo @('--version')
Invoke-Checked cargo @('fmt', '--all', '--', '--check')
Invoke-Checked cargo @('test', '--locked', '-p', 'buildercraft-kernel')
Invoke-Checked cargo @('test', '--locked', '-p', 'cadcraft-cli')
if ($Desktop) {
    Invoke-Checked cargo @('build', '--locked', '-p', 'cadcraft')
}
Write-Host 'Worldwright native smoke gate passed (GUI not exercised).'
