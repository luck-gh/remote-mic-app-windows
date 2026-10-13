# Verify current dist inputs, the compiled HTML entry and its exact Brotli asset bytes.
# The raw EXE's Cargo depfile selects its codegen output; unrelated caches cannot pass.
param(
    [string]$Executable = (Join-Path (Split-Path -Parent $PSScriptRoot) 'target/release/sayall-windows-app.exe'),
    [string]$FrontendDirectory = (Join-Path (Split-Path -Parent $PSScriptRoot) 'dist')
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$node = (Get-Command node -ErrorAction Stop).Source
& $node (Join-Path $PSScriptRoot 'verify-frontend-embedded.mjs') verify $Executable $FrontendDirectory
if ($LASTEXITCODE -ne 0) { throw 'Current frontend content is not proven inside the raw EXE.' }